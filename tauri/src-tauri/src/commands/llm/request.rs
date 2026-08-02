//! 共享的 LLM 请求构造与响应解析工具。
//!
//! 把 `chat_http.rs` 和 `stream.rs` 中重复的 Anthropic / OpenAI 请求构造、
//! 响应解析、错误转换提取为公共函数。

use super::*;

/// 根据 API 类型构建请求 URL。
pub fn build_api_url(base_url: &str, api_type: &ApiType) -> String {
    let base = base_url.trim_end_matches('/');
    if is_anthropic(api_type) {
        format!("{}/messages", base)
    } else {
        format!("{}/chat/completions", base)
    }
}

/// 为请求添加认证与 Content-Type 头。
pub fn add_auth_headers(
    req: reqwest::RequestBuilder,
    api_key: &str,
    api_type: &ApiType,
) -> reqwest::RequestBuilder {
    if is_anthropic(api_type) {
        req.header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
    } else {
        req.header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
    }
}

/// 构建请求体（合并 Anthropic 与 OpenAI 的消息格式）。
///
/// - Anthropic：从 messages 中分离 system 消息单独设置。
/// - OpenAI：直接使用 messages 数组。
pub fn build_request_body(
    model: &str,
    messages: Vec<serde_json::Value>,
    api_type: &ApiType,
    max_tokens: u32,
    stream: bool,
) -> serde_json::Value {
    if is_anthropic(api_type) {
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
            "max_tokens": max_tokens,
            "messages": chat_msgs,
        });
        if stream {
            body["stream"] = serde_json::Value::Bool(true);
        }
        if let Some(sys) = system {
            body["system"] = serde_json::Value::String(sys);
        }
        body
    } else {
        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
            "max_tokens": max_tokens,
        });
        if stream {
            body["stream"] = serde_json::Value::Bool(true);
            body["stream_options"] = serde_json::json!({"include_usage": true});
        }
        body
    }
}

/// 从 API 响应 JSON 中提取文本内容。
///
/// - Anthropic：`content[?(@.type=="text")].text`
/// - OpenAI：`choices[0].message.content`
pub fn extract_response_text(result: &serde_json::Value, api_type: &ApiType) -> Option<String> {
    if is_anthropic(api_type) {
        result["content"]
            .as_array()
            .and_then(|arr| {
                arr.iter()
                    .find(|c| {
                        c.get("type").and_then(|t| t.as_str()) == Some("text")
                            || c.get("type").is_none()
                    })
                    .and_then(|c| c.get("text").and_then(|v| v.as_str()))
            })
            .map(|s| s.to_string())
    } else {
        result["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
    }
}

/// 检查 HTTP 响应状态，失败时返回格式化错误。
pub async fn check_response(resp: reqwest::Response) -> Result<reqwest::Response, String> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    let url = resp.url().to_string();
    let body = resp.text().await.unwrap_or_default();
    let snippet = if body.is_empty() {
        "(empty body)".to_string()
    } else {
        body.chars().take(MAX_PREVIEW_CHARS).collect()
    };
    Err(format!("HTTP {} {} — {}", status.as_u16(), url, snippet))
}

/// 发送 HTTP POST 请求并检查响应状态。
pub async fn send_json_request(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
    api_key: &str,
    api_type: &ApiType,
) -> Result<serde_json::Value, String> {
    let req = client.post(url).json(body);
    let req = add_auth_headers(req, api_key, api_type);
    let resp = req
        .send()
        .await
        .map_err(|e| format!("Request to {} failed: {}", url, e))?;
    let resp = check_response(resp).await?;
    resp.json()
        .await
        .map_err(|e| format!("Parse response from {}: {}", url, e))
}

/// 从非流式响应中提取 token usage（OpenAI 格式）。
pub fn extract_openai_usage(result: &serde_json::Value) -> (u64, u64) {
    let prompt = result["usage"]["prompt_tokens"].as_u64().unwrap_or(0);
    let completion = result["usage"]["completion_tokens"].as_u64().unwrap_or(0);
    (prompt, completion)
}

/// P102：校验 LLM 请求目标 `base_url`，收窄后端网络出口。
///
/// 仅允许 http/https scheme 且含非空 host，拒绝 userinfo 与其它 scheme
/// （`javascript:`、`data:`、`file:` 等）。调用方在发起任何外连前必须先通过本校验，
/// 防止被 XSS 当作任意 URL 数据外传通道（CSP 已禁 webview 直连，后端是唯一出口）。
pub(crate) fn validate_llm_base_url(base_url: &str) -> Result<(), String> {
    let url = url::Url::parse(base_url).map_err(|e| format!("Invalid base_url: {e}"))?;
    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(format!(
            "base_url scheme must be http or https, got: {scheme}"
        ));
    }
    let host = url.host_str().unwrap_or("");
    if host.is_empty() {
        return Err("base_url must contain a host".to_string());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("base_url must not contain userinfo".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_base_url_accepts_https() {
        assert!(validate_llm_base_url("https://api.openai.com/v1").is_ok());
    }

    #[test]
    fn test_validate_base_url_accepts_localhost_http() {
        // Ollama 本地端点允许 http + localhost。
        assert!(validate_llm_base_url("http://localhost:11434/v1").is_ok());
        assert!(validate_llm_base_url("http://127.0.0.1:11434/v1").is_ok());
    }

    #[test]
    fn test_validate_base_url_rejects_non_http_schemes() {
        for bad in [
            "javascript:alert(1)",
            "data:text/html,<script>alert(1)</script>",
            "file:///etc/passwd",
            "ftp://example.com/v1",
            "gopher://example.com/x",
        ] {
            assert!(
                validate_llm_base_url(bad).is_err(),
                "should reject scheme: {bad}"
            );
        }
    }

    #[test]
    fn test_validate_base_url_rejects_missing_host() {
        // `https:///path` 会被 WHATWG URL 规范化为主机 "path"（非空），不算缺失 host。
        for bad in ["https://", "http://", "https:// ", "http:// "] {
            assert!(
                validate_llm_base_url(bad).is_err(),
                "should reject missing host: {bad:?}"
            );
        }
    }

    #[test]
    fn test_validate_base_url_rejects_userinfo() {
        assert!(validate_llm_base_url("https://user:pass@evil.com/v1").is_err());
        assert!(validate_llm_base_url("https://user@evil.com/v1").is_err());
    }

    #[test]
    fn test_validate_base_url_rejects_garbage() {
        for bad in ["", "   ", "not-a-url", "https://exa mple.com"] {
            assert!(
                validate_llm_base_url(bad).is_err(),
                "should reject garbage: {bad:?}"
            );
        }
    }
}
