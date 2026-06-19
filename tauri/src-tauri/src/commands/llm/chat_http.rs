use super::*;
#[tauri::command]
pub async fn llm_check_connection(
    base_url: String,
    api_key: String,
    model: String,
    api_type: ApiType,
) -> Result<bool, String> {
    // Lightweight health check using test-provider pattern with very short timeout
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|_| "Client error")?;
    let (url, body) = if is_anthropic(&api_type) {
        (
            format!("{}/messages", base_url.trim_end_matches('/')),
            serde_json::json!({"model": model, "max_tokens": 1, "messages": [{"role": "user", "content": "Hi"}]}),
        )
    } else {
        (
            format!("{}/chat/completions", base_url.trim_end_matches('/')),
            serde_json::json!({"model": model, "messages": [{"role": "user", "content": "Hi"}], "max_tokens": 1}),
        )
    };
    let mut req = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body);
    if is_anthropic(&api_type) {
        req = req
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01");
    } else {
        req = req.header("Authorization", format!("Bearer {}", api_key));
    }
    match req.send().await {
        Ok(r) => Ok(r.status().is_success()),
        Err(_) => Ok(false),
    }
}

#[tauri::command]
pub async fn llm_test_provider(
    base_url: String,
    api_key: String,
    model: String,
    api_type: ApiType,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client: {}", e))?;

    let (url, body, auth_header, auth_value): (String, serde_json::Value, &str, String) =
        if is_anthropic(&api_type) {
            let u = format!("{}/messages", base_url.trim_end_matches('/'));
            let b = serde_json::json!({"model": model, "max_tokens": 10, "messages": [{"role": "user", "content": "Hello"}]});
            (u, b, "x-api-key", api_key)
        } else {
            let u = format!("{}/chat/completions", base_url.trim_end_matches('/'));
            let b = serde_json::json!({"model": model, "messages": [{"role": "user", "content": "Hello"}], "max_tokens": 5, "stream": false});
            (u, b, "Authorization", format!("Bearer {}", api_key))
        };

    let mut req = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body);
    if is_anthropic(&api_type) {
        req = req
            .header(auth_header, &auth_value)
            .header("anthropic-version", "2023-06-01");
    } else {
        req = req.header(auth_header, &auth_value);
    }

    let resp = req
        .send()
        .await
        .map_err(|e| format!("Request to {} failed: {}", url, e))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let snippet = if text.is_empty() {
            "(empty body)".to_string()
        } else {
            text.chars().take(MAX_PREVIEW_CHARS).collect()
        };
        return Err(format!("HTTP {} {} — {}", status.as_u16(), url, snippet));
    }
    let result: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse response from {}: {}", url, e))?;

    if is_anthropic(&api_type) {
        let text = result["content"]
            .as_array()
            .and_then(|arr| {
                arr.iter()
                    .find(|c| {
                        c.get("type").and_then(|t| t.as_str()) == Some("text")
                            || c.get("type").is_none()
                    })
                    .and_then(|c| c.get("text").and_then(|v| v.as_str()))
            })
            .unwrap_or("ok");
        Ok(text.to_string())
    } else {
        Ok(result["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("ok")
            .to_string())
    }
}

#[tauri::command]
pub async fn llm_send_message(
    base_url: String,
    api_key: String,
    model: String,
    api_type: ApiType,
    messages: Vec<serde_json::Value>,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Client: {}", e))?;

    if is_anthropic(&api_type) {
        // Anthropic Messages API format
        // Separate system message from chat messages
        let system = messages
            .iter()
            .find(|m| m.get("role").and_then(|r| r.as_str()) == Some("system"))
            .and_then(|m| m.get("content").and_then(|c| c.as_str()))
            .map(|s| s.to_string());
        let chat_msgs: Vec<serde_json::Value> = messages
            .into_iter()
            .filter(|m| m.get("role").and_then(|r| r.as_str()) != Some("system"))
            .collect();

        let mut body = serde_json::json!({
            "model": model,
            "max_tokens": DEFAULT_MAX_TOKENS,
            "messages": chat_msgs,
        });
        if let Some(sys) = &system {
            body["system"] = serde_json::Value::String(sys.clone());
        }

        let url = format!("{}/messages", base_url.trim_end_matches('/'));
        let resp = client
            .post(&url)
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Request: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!(
                "HTTP {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        let result: serde_json::Value = resp.json().await.map_err(|e| format!("Parse: {}", e))?;
        // Anthropic thinking models return content blocks with types:
        // [{"type":"thinking","thinking":"..."}, {"type":"text","text":"..."}]
        let text = result["content"].as_array().and_then(|arr| {
            arr.iter()
                .find(|c| {
                    c.get("type").and_then(|t| t.as_str()) == Some("text")
                        || c.get("type").is_none()
                })
                .and_then(|c| c.get("text").and_then(|v| v.as_str()))
        });
        text.map(|s| s.to_string()).ok_or_else(|| {
            let raw = result.to_string();
            format!("No response — raw: {}", &raw[..300.min(raw.len())])
        })
    } else {
        // OpenAI-compatible API format
        let body = serde_json::json!({"model": model, "messages": messages, "stream": false});
        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Request: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!(
                "HTTP {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        let result: serde_json::Value = resp.json().await.map_err(|e| format!("Parse: {}", e))?;
        let text = result["choices"][0]["message"]["content"].as_str();
        text.map(|s| s.to_string()).ok_or_else(|| {
            let raw = result.to_string();
            format!("No response — raw: {}", &raw[..300.min(raw.len())])
        })
    }
}
