//! LLM HTTP client for OpenAI-compatible and Anthropic APIs with SSE streaming.
//!
//! P026: 请求构造与 SSE/JSON 解析纯函数收敛到 `super::protocol`（与
//! src-tauri 的 async client 共享），本文件只保留 IO 绑定（blocking reqwest）。

use super::protocol::{self, split_system_messages};
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
    let url = protocol::build_api_url(base_url, api_type);
    let headers = protocol::auth_headers(api_key, api_type);
    let body = match api_type {
        ApiType::Anthropic => {
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
            body
        }
        ApiType::OpenAI => serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": true,
            "stream_options": { "include_usage": true },
        }),
    };
    Ok((url, body, headers))
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
    // P004: 不再 `resp.bytes()` 整包读入——blocking Response 实现 `std::io::Read`，
    // 由独立读线程逐行消费网络流，经 mpsc 转发给解析侧；首个 chunk 到达即触发
    // on_event（CLI 打字机真正流式）。空闲超时用 `recv_timeout` 实现。
    let rx = spawn_sse_reader(resp);

    let mut counters = SseCounters::default();

    loop {
        // 通道值语义：Ok(Some(line)) 正常行；Ok(None) 真正 EOF；
        // Err(e) 网络读错误（P004-R1：必须中断并传播，不得当作完整回复）。
        let line_opt = rx.recv_timeout(SSE_IDLE_TIMEOUT).map_err(|e| {
            format!(
                "SSE stream read error (idle timeout {}s or disconnected): {}",
                SSE_IDLE_TIMEOUT.as_secs(),
                e
            )
        })?;
        // Err(e) = 网络读错误（P004-R1：中断并传播，不得当作完整回复）；
        // Ok(None) = 真正的 EOF。
        let raw_line = line_opt?;
        let Some(raw_line) = raw_line else {
            break; // 真正的 EOF
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

            handle_sse_payload(&json, api_type, on_event, &mut counters);
        }
    }

    if api_type == &ApiType::Anthropic {
        counters.prompt_tokens = counters.anthropic_prompt_tokens;
        counters.completion_tokens = counters.anthropic_completion_tokens;
    }

    on_event(LlmStreamEvent::Done {
        prompt_tokens: counters.prompt_tokens,
        completion_tokens: counters.completion_tokens,
    });

    Ok(())
}
/// P004: 独立读线程逐行消费网络流（blocking Response 实现 `std::io::Read`），
/// 经 mpsc 转发给解析侧。
///
/// 通道语义（P004-R1 修正）：`Err(String)` 表示读线程遇到**网络读错误**，
/// 必须传播给解析侧（进程外由 send_chat_stream 返回 Err 通知 CLI/引擎），
/// **不得**伪装成 EOF——否则网络中途断流的半截回复会被当作完整回复
/// emit Done 并持久化。`Ok(None)` 仅表示真正的 EOF。
///
/// 不 join：空闲超时退出时读线程可能仍阻塞在 read_line，detach 语义让其随
/// 连接关闭/进程退出自然清理（解析侧丢 rx 后其 send 会失败并退出）。
fn spawn_sse_reader(
    resp: reqwest::blocking::Response,
) -> std::sync::mpsc::Receiver<Result<Option<String>, String>> {
    let (tx, rx) = std::sync::mpsc::channel::<Result<Option<String>, String>>();
    let _reader = std::thread::spawn(move || {
        sse_reader_loop(std::io::BufReader::new(resp), tx);
    });
    rx
}

/// P004-R1: 读线程逐行消费循环（抽函数便于单测）。
/// 语义：`Ok(Some(line))` 正常行；`Err(e)` 网络读错误（必须传播，不得伪装 EOF）；
/// 循环结束后 `Ok(None)` 通知真正的流结束（解析侧已因 Err 提前返回时被忽略）。
fn sse_reader_loop<R: std::io::BufRead>(
    mut reader: R,
    tx: std::sync::mpsc::Sender<Result<Option<String>, String>>,
) {
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break, // 真正的 EOF
            Ok(_) => {
                if tx.send(Ok(Some(std::mem::take(&mut line)))).is_err() {
                    break; // 解析侧已退出（如空闲超时），停止发送
                }
            }
            Err(e) => {
                // P004-R1: 读错误必须传播，不能伪装成 EOF
                let _ = tx.send(Err(format!("SSE stream read error: {e}")));
                break;
            }
        }
    }
    let _ = tx.send(Ok(None)); // 通知解析侧真正的流结束
}

/// process_sse 的 token 计数聚合。
#[derive(Default)]
struct SseCounters {
    anthropic_prompt_tokens: u64,
    anthropic_completion_tokens: u64,
    prompt_tokens: u64,
    completion_tokens: u64,
}

/// 解析单条 SSE data JSON，按 api_type 提取 token 计数并派发 Chunk 事件。
/// 从 process_sse 主循环抽出以消除 8 层嵌套（line → data → match → choices → delta → content）。
fn handle_sse_payload(
    json: &serde_json::Value,
    api_type: &ApiType,
    on_event: &dyn Fn(LlmStreamEvent),
    counters: &mut SseCounters,
) {
    match api_type {
        ApiType::Anthropic => {
            if let Some(input) = protocol::extract_anthropic_input_tokens(json) {
                counters.anthropic_prompt_tokens = input;
            }
            if let Some(output) = protocol::extract_anthropic_output_tokens(json) {
                counters.anthropic_completion_tokens = output;
            }
            if let Some(text) = protocol::extract_delta_text(json, api_type) {
                on_event(LlmStreamEvent::Chunk {
                    content: text.to_string(),
                });
            }
        }
        ApiType::OpenAI => {
            if let Some((prompt, completion)) = protocol::extract_openai_usage_from_chunk(json) {
                counters.prompt_tokens = prompt.unwrap_or(0);
                counters.completion_tokens = completion.unwrap_or(0);
            }
            if let Some(text) = protocol::extract_delta_text(json, api_type) {
                on_event(LlmStreamEvent::Chunk {
                    content: text.to_string(),
                });
            }
        }
    }
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

    let text = protocol::extract_response_text(&json, api_type).unwrap_or_default();

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

    /// P004-R1 回归测试：读线程在「读出一行后网络中断」时，必须发送
    /// `Err`（读错误）而不是 `Ok(None)`（伪装 EOF）——否则解析侧会把
    /// 半截回复当完整回复 emit Done 并持久化。
    struct ReadLineThenBroken {
        emitted: bool,
    }
    impl std::io::Read for ReadLineThenBroken {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if !self.emitted {
                self.emitted = true;
                let line = b"data: {\"role\": \"assistant\"}\n";
                let n = line.len().min(buf.len());
                buf[..n].copy_from_slice(&line[..n]);
                Ok(n)
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "connection reset by peer",
                ))
            }
        }
    }

    #[test]
    fn sse_reader_propagates_read_error_not_fake_eof() {
        let (tx, rx) = std::sync::mpsc::channel::<Result<Option<String>, String>>();
        sse_reader_loop(
            std::io::BufReader::new(ReadLineThenBroken { emitted: false }),
            tx,
        );

        // 第一行正常送达
        let first = rx.recv().expect("first line");
        assert!(
            matches!(&first, Ok(Some(l)) if l.starts_with("data: ")),
            "第一行应为正常行，got {first:?}"
        );

        // 读错误必须作为 Err 传播，而不是 Ok(None)（伪装 EOF）
        let second = rx.recv().expect("read error");
        assert!(
            matches!(&second, Err(e) if e.contains("reset")),
            "读错误应传播为 Err，got {second:?}"
        );

        // 循环结束仍发送 Ok(None)（detach 通知，解析侧已提前返回则忽略）
        let third = rx.recv().expect("eof marker");
        assert!(
            matches!(third, Ok(None)),
            "EOF 标记应为 Ok(None)，got {third:?}"
        );
    }
}
