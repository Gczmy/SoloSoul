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
    // P030: 此前 timeout(None)——慢速滴流可永久挂起阻塞线程；P004: 请求级 120s
    // 总超时会把「长回复持续出 token」的流式响应直接截断。现改为：连接阶段由
    // connect_timeout(15s) 兜底；流式路径（process_sse）用空闲超时
    // SSE_IDLE_TIMEOUT（每收到完整一行重置，长回复不截断，死连接不挂起）覆盖；
    // 非流式路径（process_non_streaming）由 read_body_with_timeout 总超时覆盖。
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(15))
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

/// P004: SSE 空闲超时——每收到完整一行重置计时（替代旧的请求级 120s 总超时：
/// 长回复持续出 token 不再被截断；连接死而不发数据也不会永久挂起）。
const SSE_IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

fn process_sse(
    resp: reqwest::blocking::Response,
    api_type: &ApiType,
    on_event: &dyn Fn(LlmStreamEvent),
) -> Result<(), String> {
    use std::io::BufRead;

    // P004: 不再 `resp.bytes()` 整包读入——blocking Response 实现 `std::io::Read`，
    // 由独立读线程逐行消费网络流，经 mpsc 转发给解析侧；首个 chunk 到达即触发
    // on_event（CLI 打字机真正流式）。空闲超时用 `recv_timeout` 实现。
    let (tx, rx) = std::sync::mpsc::channel::<Option<String>>();
    // 不 join：空闲超时退出时读线程可能仍阻塞在 read_line，detach 语义让其随
    // 连接关闭/进程退出自然清理（解析侧丢 rx 后其 send 会失败并退出）。
    let _reader = std::thread::spawn(move || {
        let mut reader = std::io::BufReader::new(resp);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    if tx.send(Some(std::mem::take(&mut line))).is_err() {
                        break; // 解析侧已退出（如空闲超时），停止发送
                    }
                }
                Err(_) => break,
            }
        }
        let _ = tx.send(None); // 通知解析侧流结束
    });

    let mut anthropic_prompt_tokens: u64 = 0;
    let mut anthropic_completion_tokens: u64 = 0;
    let mut prompt_tokens: u64 = 0;
    let mut completion_tokens: u64 = 0;

    loop {
        let line_opt = rx.recv_timeout(SSE_IDLE_TIMEOUT).map_err(|e| {
            format!(
                "SSE stream read error (idle timeout {}s or disconnected): {}",
                SSE_IDLE_TIMEOUT.as_secs(),
                e
            )
        })?;
        let Some(raw_line) = line_opt else {
            break; // EOF
        };
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

/// 带总超时的阻塞读 body（线程 + recv_timeout，仅非流式路径使用）。
/// P004: 非流式响应无「空闲重置」语义，保留总超时兜底防死连接永久挂起。
fn read_body_with_timeout(
    resp: reqwest::blocking::Response,
    timeout: std::time::Duration,
) -> Result<String, String> {
    let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();
    std::thread::spawn(move || {
        let _ = tx.send(resp.text().map_err(|e| format!("Read response: {}", e)));
    });
    rx.recv_timeout(timeout)
        .map_err(|_| format!("Response body read timed out after {}s", timeout.as_secs()))?
}

fn process_non_streaming(
    resp: reqwest::blocking::Response,
    api_type: &ApiType,
    on_event: &dyn Fn(LlmStreamEvent),
) -> Result<(), String> {
    let body = read_body_with_timeout(resp, SSE_IDLE_TIMEOUT)?;
    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("Parse response: {}", e))?;

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
