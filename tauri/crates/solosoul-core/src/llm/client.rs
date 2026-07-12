//! LLM HTTP client for OpenAI-compatible and Anthropic APIs with SSE streaming.

use crate::llm::config::ApiType;

/// LLM 请求构建结果：URL、JSON body、HTTP headers。
type LlmRequestParts = (String, serde_json::Value, Vec<(String, String)>);

/// Events emitted during streaming.
#[derive(Debug, Clone)]
pub enum LlmStreamEvent {
    /// A text chunk from the LLM.
    Chunk { content: String },
    /// Streaming completed with token usage.
    Done {
        prompt_tokens: u64,
        completion_tokens: u64,
    },
    /// An error occurred.
    Error { message: String },
}

/// Send a chat completion request to an LLM provider, calling `on_event` for each streaming event.
///
/// This function blocks until the response is complete or an error occurs.
pub fn send_chat_stream(
    base_url: &str,
    api_key: &str,
    model: &str,
    api_type: &ApiType,
    messages: &[serde_json::Value],
    on_event: &dyn Fn(LlmStreamEvent),
) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(None)
        .build()
        .map_err(|e| format!("Client build error: {}", e))?;

    let (url, body, headers_map) = build_request(base_url, api_key, model, api_type, messages)?;

    let mut req = client.post(&url).json(&body);
    for (k, v) in &headers_map {
        req = req.header(k.as_str(), v.as_str());
    }

    let resp = req.send().map_err(|e| format!("Request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        on_event(LlmStreamEvent::Error {
            message: format!("HTTP {}: {}", status, body),
        });
        return Err(format!("HTTP {}: {}", status, body));
    }

    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v: &reqwest::header::HeaderValue| v.to_str().ok())
        .unwrap_or("");

    if content_type.contains("text/event-stream") {
        process_sse(resp, api_type, on_event)
    } else {
        process_non_streaming(resp, api_type, on_event)
    }
}

/// Send a non-streaming chat completion request and return the full response text.
pub fn send_chat(
    base_url: &str,
    api_key: &str,
    model: &str,
    api_type: &ApiType,
    messages: &[serde_json::Value],
) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Client build error: {}", e))?;

    let (url, mut body, headers_map) = build_request(base_url, api_key, model, api_type, messages)?;

    if let serde_json::Value::Object(ref mut map) = body {
        map.remove("stream");
        map.remove("stream_options");
    }

    let mut req = client.post(&url).json(&body);
    for (k, v) in &headers_map {
        req = req.header(k.as_str(), v.as_str());
    }

    let resp = req.send().map_err(|e| format!("Request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, body));
    }

    let json: serde_json::Value = resp
        .json()
        .map_err(|e: reqwest::Error| format!("Parse response: {}", e))?;

    match api_type {
        ApiType::Anthropic => json["content"]
            .as_array()
            .and_then(|arr: &Vec<serde_json::Value>| arr.first())
            .and_then(|b: &serde_json::Value| b["text"].as_str())
            .map(|s: &str| s.to_string())
            .ok_or_else(|| "No text in Anthropic response".to_string()),
        ApiType::OpenAI => json["choices"][0]["message"]["content"]
            .as_str()
            .map(|s: &str| s.to_string())
            .ok_or_else(|| "No content in OpenAI response".to_string()),
    }
}

// ── helpers ──────────────────────────────────────────────────────

fn build_request(
    base_url: &str,
    api_key: &str,
    model: &str,
    api_type: &ApiType,
    messages: &[serde_json::Value],
) -> Result<LlmRequestParts, String> {
    match api_type {
        ApiType::Anthropic => {
            let url = format!("{}/messages", base_url.trim_end_matches('/'));
            let (system_prompts, other_messages) = split_system_messages(messages);

            let mut body = serde_json::json!({
                "model": model,
                "max_tokens": 4096,
                "stream": true,
                "messages": other_messages,
            });
            if !system_prompts.is_empty() {
                body["system"] = serde_json::Value::Array(system_prompts);
            }

            let headers = vec![
                ("x-api-key".to_string(), api_key.to_string()),
                ("anthropic-version".to_string(), "2023-06-01".to_string()),
                ("content-type".to_string(), "application/json".to_string()),
            ];
            Ok((url, body, headers))
        }
        ApiType::OpenAI => {
            let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
            let body = serde_json::json!({
                "model": model,
                "messages": messages,
                "stream": true,
                "stream_options": { "include_usage": true },
            });
            let headers = vec![
                ("Authorization".to_string(), format!("Bearer {}", api_key)),
                ("content-type".to_string(), "application/json".to_string()),
            ];
            Ok((url, body, headers))
        }
    }
}

fn split_system_messages(
    messages: &[serde_json::Value],
) -> (Vec<serde_json::Value>, Vec<serde_json::Value>) {
    let mut system = Vec::new();
    let mut other = Vec::new();
    for msg in messages {
        if msg.get("role").and_then(|r| r.as_str()) == Some("system") {
            if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
                system.push(serde_json::json!({
                    "type": "text",
                    "text": content
                }));
            }
        } else {
            other.push(msg.clone());
        }
    }
    (system, other)
}

// ── SSE processing ───────────────────────────────────────────────

fn process_sse(
    resp: reqwest::blocking::Response,
    api_type: &ApiType,
    on_event: &dyn Fn(LlmStreamEvent),
) -> Result<(), String> {
    use std::io::BufRead;

    // Read the full body as bytes, then use Cursor for BufRead
    let bytes = resp.bytes().map_err(|e| format!("Read response: {}", e))?;
    let cursor = std::io::Cursor::new(bytes);
    let reader = std::io::BufReader::new(cursor);

    let mut anthropic_prompt_tokens: u64 = 0;
    let mut anthropic_completion_tokens: u64 = 0;
    let mut prompt_tokens: u64 = 0;
    let mut completion_tokens: u64 = 0;

    for raw_line_result in reader.lines() {
        let raw_line: String =
            raw_line_result.map_err(|e: std::io::Error| format!("Read error: {}", e))?;
        let line: &str = raw_line.trim();

        if line.is_empty() {
            continue;
        }

        // Anthropic event lines
        if matches!(api_type, ApiType::Anthropic) && line.starts_with("event:") {
            continue;
        }

        if let Some(data) = line.strip_prefix("data: ") {
            if data == "[DONE]" {
                break;
            }

            let json: serde_json::Value =
                serde_json::from_str(data).map_err(|e| format!("SSE parse: {}", e))?;

            match api_type {
                ApiType::Anthropic => {
                    if let Some(usage) = json.get("message").and_then(|m| m.get("usage")) {
                        anthropic_prompt_tokens = usage
                            .get("input_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                    }
                    if let Some(usage) = json.get("usage") {
                        anthropic_completion_tokens = usage
                            .get("output_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                    }

                    if let Some(delta) = json.get("delta") {
                        if let Some(text) = delta.get("text").and_then(|v| v.as_str()) {
                            on_event(LlmStreamEvent::Chunk {
                                content: text.to_string(),
                            });
                        }
                    }
                }
                ApiType::OpenAI => {
                    if let Some(usage) = json.get("usage") {
                        prompt_tokens = usage
                            .get("prompt_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        completion_tokens = usage
                            .get("completion_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                    }

                    if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
                        for choice in choices {
                            if let Some(delta) = choice.get("delta") {
                                if let Some(content) = delta.get("content").and_then(|v| v.as_str())
                                {
                                    on_event(LlmStreamEvent::Chunk {
                                        content: content.to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if api_type == &ApiType::Anthropic {
        prompt_tokens = anthropic_prompt_tokens;
        completion_tokens = anthropic_completion_tokens;
    }

    on_event(LlmStreamEvent::Done {
        prompt_tokens,
        completion_tokens,
    });

    Ok(())
}

fn process_non_streaming(
    resp: reqwest::blocking::Response,
    api_type: &ApiType,
    on_event: &dyn Fn(LlmStreamEvent),
) -> Result<(), String> {
    let json: serde_json::Value = resp
        .json()
        .map_err(|e: reqwest::Error| format!("Parse response: {}", e))?;

    let text = match api_type {
        ApiType::Anthropic => json["content"]
            .as_array()
            .and_then(|arr: &Vec<serde_json::Value>| arr.first())
            .and_then(|b: &serde_json::Value| b["text"].as_str())
            .unwrap_or("")
            .to_string(),
        ApiType::OpenAI => json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string(),
    };

    if !text.is_empty() {
        on_event(LlmStreamEvent::Chunk { content: text });
    }

    on_event(LlmStreamEvent::Done {
        prompt_tokens: 0,
        completion_tokens: 0,
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_openai_request() {
        let messages = vec![serde_json::json!({
            "role": "user",
            "content": "Hello"
        })];
        let (url, body, headers) = build_request(
            "https://api.openai.com/v1",
            "sk-test",
            "gpt-4o",
            &ApiType::OpenAI,
            &messages,
        )
        .unwrap();
        assert_eq!(url, "https://api.openai.com/v1/chat/completions");
        assert_eq!(body["model"], "gpt-4o");
        assert_eq!(body["stream"], true);
        assert!(headers.iter().any(|(k, _)| k == "Authorization"));
    }

    #[test]
    fn test_build_anthropic_request() {
        let messages = vec![
            serde_json::json!({
                "role": "system",
                "content": "You are helpful."
            }),
            serde_json::json!({
                "role": "user",
                "content": "Hello"
            }),
        ];
        let (url, body, headers) = build_request(
            "https://api.anthropic.com/v1",
            "sk-test",
            "claude-sonnet-4-20250514",
            &ApiType::Anthropic,
            &messages,
        )
        .unwrap();
        assert_eq!(url, "https://api.anthropic.com/v1/messages");
        assert_eq!(body["model"], "claude-sonnet-4-20250514");
        assert!(body.get("system").is_some());
        assert!(headers.iter().any(|(k, _)| k == "x-api-key"));
    }
}
