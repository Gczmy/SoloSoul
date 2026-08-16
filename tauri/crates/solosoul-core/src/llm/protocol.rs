//! LLM 协议纯函数（请求构造 + SSE/JSON 解析）。
//!
//! blocking（`solosoul-core::llm::client`）与 async（`src-tauri::commands::llm`）
//! 两套 HTTP 客户端共用的纯函数：无 IO、无 reqwest 依赖，行为由测试锁定。
//! 两个 client 只保留 IO 绑定（发请求 / 读流），协议层统一收敛于此。

use super::config::ApiType;

/// 构建请求 URL：OpenAI → `{base}/chat/completions`，Anthropic → `{base}/messages`。
pub fn build_api_url(base_url: &str, api_type: &ApiType) -> String {
    let base = base_url.trim_end_matches('/');
    match api_type {
        ApiType::Anthropic => format!("{base}/messages"),
        ApiType::OpenAI => format!("{base}/chat/completions"),
    }
}

/// 认证与内容头（Anthropic x-api-key / OpenAI Bearer）。
///
/// 返回 `Vec<(String, String)>` 以便 blocking 与 async 两侧以各自方式套用到请求上。
pub fn auth_headers(api_key: &str, api_type: &ApiType) -> Vec<(String, String)> {
    match api_type {
        ApiType::Anthropic => vec![
            ("x-api-key".to_string(), api_key.to_string()),
            ("anthropic-version".to_string(), "2023-06-01".to_string()),
            ("content-type".to_string(), "application/json".to_string()),
        ],
        ApiType::OpenAI => vec![
            ("Authorization".to_string(), format!("Bearer {api_key}")),
            ("content-type".to_string(), "application/json".to_string()),
        ],
    }
}

/// 从消息列表中分离 system 消息（Anthropic 需要 system 单列）。
///
/// 返回 `(system 文本块数组, 其余消息)`；仅保留 content 为字符串的 system 消息。
pub fn split_system_messages(
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

/// 从 SSE data JSON 提取 delta 文本：
/// Anthropic `delta.text`；OpenAI `choices[0].delta.content`。
pub fn extract_delta_text<'a>(json: &'a serde_json::Value, api_type: &ApiType) -> Option<&'a str> {
    match api_type {
        ApiType::Anthropic => json
            .get("delta")
            .and_then(|d| d.get("text"))
            .and_then(|t| t.as_str()),
        ApiType::OpenAI => json
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|choice| choice.get("delta"))
            .and_then(|delta| delta.get("content"))
            .and_then(|c| c.as_str()),
    }
}

/// 从 OpenAI SSE chunk 提取 usage（usage 可能在 choices 为空的 chunk 中）。
/// 返回 `(prompt, completion)`，均为 `Option`——缺失的字段由调用方决定
/// 是覆盖（blocking 旧语义）还是保留先前累积值（async N008 语义）。
pub fn extract_openai_usage_from_chunk(
    json: &serde_json::Value,
) -> Option<(Option<u64>, Option<u64>)> {
    let usage = json.get("usage")?;
    let prompt = usage.get("prompt_tokens").and_then(|v| v.as_u64());
    let completion = usage.get("completion_tokens").and_then(|v| v.as_u64());
    Some((prompt, completion))
}

/// 从 Anthropic SSE chunk 提取输入 token（`message.usage.input_tokens`）。
/// 由调用方决定何时使用（async 侧仅在 message_start 事件；blocking 侧直接覆盖）。
pub fn extract_anthropic_input_tokens(json: &serde_json::Value) -> Option<u64> {
    json.get("message")
        .and_then(|m| m.get("usage"))
        .and_then(|u| u.get("input_tokens"))
        .and_then(|v| v.as_u64())
}

/// 从 Anthropic SSE chunk 提取输出 token（`usage.output_tokens`）。
/// 由调用方决定何时使用（async 侧仅在 message_delta 事件；blocking 侧直接覆盖）。
pub fn extract_anthropic_output_tokens(json: &serde_json::Value) -> Option<u64> {
    json.get("usage")
        .and_then(|u| u.get("output_tokens"))
        .and_then(|v| v.as_u64())
}

/// 从非流式响应 JSON 提取文本内容：
/// Anthropic `content[?(@.type=="text")].text`（兼容无 type 的文本块）；
/// OpenAI `choices[0].message.content`。
pub fn extract_response_text(result: &serde_json::Value, api_type: &ApiType) -> Option<String> {
    match api_type {
        ApiType::Anthropic => result["content"]
            .as_array()
            .and_then(|arr| {
                arr.iter()
                    .find(|c| {
                        c.get("type").and_then(|t| t.as_str()) == Some("text")
                            || c.get("type").is_none()
                    })
                    .and_then(|c| c.get("text").and_then(|v| v.as_str()))
            })
            .map(|s| s.to_string()),
        ApiType::OpenAI => result["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string()),
    }
}

/// 从非流式响应提取 token usage（OpenAI 格式）。
pub fn extract_openai_usage(result: &serde_json::Value) -> (u64, u64) {
    let prompt = result["usage"]["prompt_tokens"].as_u64().unwrap_or(0);
    let completion = result["usage"]["completion_tokens"].as_u64().unwrap_or(0);
    (prompt, completion)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_api_url() {
        assert_eq!(
            build_api_url("https://api.openai.com/v1/", &ApiType::OpenAI),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            build_api_url("https://api.anthropic.com/v1", &ApiType::Anthropic),
            "https://api.anthropic.com/v1/messages"
        );
    }

    #[test]
    fn test_auth_headers() {
        let openai = auth_headers("sk-test", &ApiType::OpenAI);
        assert!(openai
            .iter()
            .any(|(k, v)| k == "Authorization" && v == "Bearer sk-test"));
        let anthropic = auth_headers("sk-test", &ApiType::Anthropic);
        assert!(anthropic.iter().any(|(k, _)| k == "x-api-key"));
        assert!(anthropic
            .iter()
            .any(|(k, v)| k == "anthropic-version" && v == "2023-06-01"));
    }

    #[test]
    fn test_split_system_messages() {
        let messages = vec![
            serde_json::json!({"role": "system", "content": "You are helpful."}),
            serde_json::json!({"role": "user", "content": "Hello"}),
            serde_json::json!({"role": "system", "content": "Be concise."}),
        ];
        let (system, other) = split_system_messages(&messages);
        assert_eq!(system.len(), 2);
        assert_eq!(system[0]["text"], "You are helpful.");
        assert_eq!(system[1]["text"], "Be concise.");
        assert_eq!(other.len(), 1);
        assert_eq!(other[0]["role"], "user");
    }

    #[test]
    fn test_extract_delta_text() {
        let openai = serde_json::json!({"choices": [{"delta": {"content": "Hi"}}]});
        assert_eq!(extract_delta_text(&openai, &ApiType::OpenAI), Some("Hi"));
        let anthropic = serde_json::json!({"delta": {"text": "Hi"}});
        assert_eq!(
            extract_delta_text(&anthropic, &ApiType::Anthropic),
            Some("Hi")
        );
        assert_eq!(
            extract_delta_text(&serde_json::json!({}), &ApiType::OpenAI),
            None
        );
    }

    #[test]
    fn test_extract_openai_usage_from_chunk() {
        assert_eq!(
            extract_openai_usage_from_chunk(&serde_json::json!({"choices": []})),
            None
        );
        let full = serde_json::json!({"usage": {"prompt_tokens": 10, "completion_tokens": 20}});
        assert_eq!(
            extract_openai_usage_from_chunk(&full),
            Some((Some(10), Some(20)))
        );
        let partial = serde_json::json!({"usage": {"prompt_tokens": 5}});
        assert_eq!(
            extract_openai_usage_from_chunk(&partial),
            Some((Some(5), None))
        );
    }

    #[test]
    fn test_extract_anthropic_tokens() {
        let start = serde_json::json!({"message": {"usage": {"input_tokens": 7}}});
        assert_eq!(extract_anthropic_input_tokens(&start), Some(7));
        assert_eq!(extract_anthropic_output_tokens(&start), None);
        let delta = serde_json::json!({"usage": {"output_tokens": 9}});
        assert_eq!(extract_anthropic_output_tokens(&delta), Some(9));
        assert_eq!(extract_anthropic_input_tokens(&delta), None);
    }

    #[test]
    fn test_extract_response_text() {
        let openai = serde_json::json!({"choices": [{"message": {"content": "Reply"}}]});
        assert_eq!(
            extract_response_text(&openai, &ApiType::OpenAI),
            Some("Reply".to_string())
        );
        let anthropic = serde_json::json!({"content": [{"type": "text", "text": "Reply"}]});
        assert_eq!(
            extract_response_text(&anthropic, &ApiType::Anthropic),
            Some("Reply".to_string())
        );
        let legacy = serde_json::json!({"content": [{"text": "Legacy"}]});
        assert_eq!(
            extract_response_text(&legacy, &ApiType::Anthropic),
            Some("Legacy".to_string())
        );
    }

    #[test]
    fn test_extract_openai_usage() {
        let result = serde_json::json!({"usage": {"prompt_tokens": 3, "completion_tokens": 4}});
        assert_eq!(extract_openai_usage(&result), (3, 4));
        assert_eq!(extract_openai_usage(&serde_json::json!({})), (0, 0));
    }
}
