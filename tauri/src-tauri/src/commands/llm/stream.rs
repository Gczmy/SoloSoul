use crate::state::AppState;
use serde::Serialize;
use tauri::State;

// =============================================================================
// Streaming Response (§5.3)
// =============================================================================

use tauri::Emitter;

use super::request;
use super::*;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmStreamPayload {
    pub conversation_id: String,
    pub chunk: String,
    pub is_done: bool,
    pub error: Option<String>,
}

/// 打字机效果：将完整文本逐块推送到前端（降级用）
/// P111: 改为按 CHUNK_SIZE 个字符批量发送，减少 IPC 事件数量。
async fn emit_typing_effect(app: &tauri::AppHandle, conversation_id: &str, full_text: &str) {
    const CHUNK_SIZE: usize = 20;
    let chars: Vec<String> = full_text.chars().map(|c| c.to_string()).collect();
    let total = chars.len();
    let max_typing_ms = 3000u64;
    let delay_ms = if total <= 50 { 10u64 } else { 30u64 };

    let mut pos = 0;
    while pos < total {
        let elapsed = (pos as u64 / CHUNK_SIZE as u64) * delay_ms;
        if elapsed >= max_typing_ms {
            let remaining: String = chars[pos..].concat();
            let _ = app.emit(
                "llm-stream-chunk",
                LlmStreamPayload {
                    conversation_id: conversation_id.to_string(),
                    chunk: remaining,
                    is_done: true,
                    error: None,
                },
            );
            return;
        }
        let end = std::cmp::min(pos + CHUNK_SIZE, total);
        let chunk: String = chars[pos..end].concat();
        let _ = app.emit(
            "llm-stream-chunk",
            LlmStreamPayload {
                conversation_id: conversation_id.to_string(),
                chunk,
                is_done: false,
                error: None,
            },
        );
        pos = end;
        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
    }

    let _ = app.emit(
        "llm-stream-chunk",
        LlmStreamPayload {
            conversation_id: conversation_id.to_string(),
            chunk: String::new(),
            is_done: true,
            error: None,
        },
    );
}

/// 发送聊天请求并流式推送结果（Phase 2.3：SSE 流式 + 打字机降级）
/// 返回 (完整文本, 可选的真实 TokenUsage)
/// 从 SSE JSON chunk 提取 delta 文本（兼容 Anthropic delta.text 与 OpenAI choices[0].delta.content）。
fn extract_delta_text<'a>(json: &'a serde_json::Value, api_type: &ApiType) -> Option<&'a str> {
    if is_anthropic(api_type) {
        json.get("delta")
            .and_then(|d| d.get("text"))
            .and_then(|t| t.as_str())
    } else {
        json.get("choices")
            .and_then(|c| c.get(0))
            .and_then(|choice| choice.get("delta"))
            .and_then(|delta| delta.get("content"))
            .and_then(|c| c.as_str())
    }
}

/// SSE 流式解析：逐行解析 data: 行，提取 delta 文本与 usage，事件经 IPC 推送。
async fn handle_sse_stream(
    app: &tauri::AppHandle,
    resp: reqwest::Response,
    conversation_id: &str,
    api_type: &ApiType,
) -> Result<(String, Option<TokenUsage>), String> {
    use futures::StreamExt;

    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();
    let mut full_text = String::new();
    let mut token_usage = TokenUsage::default();

    // Anthropic 跨事件累积
    let mut anthropic_prompt_tokens: u64 = 0;
    let mut anthropic_completion_tokens: u64 = 0;
    let mut current_event: String = String::new();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| format!("Stream error: {e}"))?;
        let text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&text);

        // 按行处理缓冲区
        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim().to_string();
            buffer = buffer[pos + 1..].to_string();

            if line.is_empty() {
                continue;
            }

            // 处理 event: 行（Anthropic 使用）
            if let Some(event) = line.strip_prefix("event: ") {
                current_event = event.to_string();
                continue;
            }

            // 只处理 data: 行
            if !line.starts_with("data: ") {
                continue;
            }
            let data = &line[6..];

            // OpenAI 风格结束标记
            if data == "[DONE]" {
                let _ = app.emit(
                    "llm-stream-chunk",
                    LlmStreamPayload {
                        conversation_id: conversation_id.to_string(),
                        chunk: String::new(),
                        is_done: true,
                        error: None,
                    },
                );
                let usage = if token_usage.prompt_tokens > 0 || token_usage.completion_tokens > 0 {
                    Some(token_usage)
                } else {
                    None
                };
                return Ok((full_text, usage));
            }

            // 尝试解析 JSON
            let Ok(json) = serde_json::from_str::<serde_json::Value>(data) else {
                continue;
            };

            // ── 提取 delta content ──
            let delta_text = extract_delta_text(&json, api_type);

            if let Some(text) = delta_text {
                if !text.is_empty() {
                    full_text.push_str(text);
                    let _ = app.emit(
                        "llm-stream-chunk",
                        LlmStreamPayload {
                            conversation_id: conversation_id.to_string(),
                            chunk: text.to_string(),
                            is_done: false,
                            error: None,
                        },
                    );
                }
            }

            // ── 提取 usage（P034: 按 Anthropic/OpenAI 各抽函数，消除 5 层嵌套）──
            if is_anthropic(api_type) {
                if let Some((input, output)) = extract_anthropic_usage(&json, &current_event) {
                    if let Some(i) = input {
                        anthropic_prompt_tokens = i;
                    }
                    if let Some(o) = output {
                        anthropic_completion_tokens = o;
                    }
                }
                token_usage.prompt_tokens = anthropic_prompt_tokens;
                token_usage.completion_tokens = anthropic_completion_tokens;
            } else {
                // N008/R005: 逐字段更新——缺失字段保留先前累积值，避免整体清零。
                apply_openai_usage_chunk(&mut token_usage, &json);
            }
        }
    }

    // 处理缓冲区中剩余的内容
    if let Some(data) = buffer.trim().strip_prefix("data: ") {
        handle_remaining_data(
            data,
            app,
            conversation_id,
            api_type,
            &mut full_text,
            &mut token_usage,
        );
    }

    // 流正常结束
    let _ = app.emit(
        "llm-stream-chunk",
        LlmStreamPayload {
            conversation_id: conversation_id.to_string(),
            chunk: String::new(),
            is_done: true,
            error: None,
        },
    );
    let usage = if token_usage.prompt_tokens > 0 || token_usage.completion_tokens > 0 {
        Some(token_usage)
    } else {
        None
    };
    Ok((full_text, usage))
}

/// 处理流结束前缓冲区内最后一行（未换行）的 data 内容。
fn handle_remaining_data(
    data: &str,
    app: &tauri::AppHandle,
    conversation_id: &str,
    api_type: &ApiType,
    full_text: &mut String,
    token_usage: &mut TokenUsage,
) {
    if data == "[DONE]" {
        return;
    }
    let Ok(json) = serde_json::from_str::<serde_json::Value>(data) else {
        return;
    };
    if let Some(text) = extract_delta_text(&json, api_type) {
        if !text.is_empty() {
            full_text.push_str(text);
            let _ = app.emit(
                "llm-stream-chunk",
                LlmStreamPayload {
                    conversation_id: conversation_id.to_string(),
                    chunk: text.to_string(),
                    is_done: false,
                    error: None,
                },
            );
        }
    }
    // 剩余内容也可能含 usage（P034: 复用 OpenAI chunk 解析）
    if !is_anthropic(api_type) {
        // N008/R005: 逐字段更新——缺失字段保留先前累积值。
        apply_openai_usage_chunk(token_usage, &json);
    }
}

/// N008/R005: 把 OpenAI SSE chunk 的 usage 逐字段应用到累积值——缺失字段保留
/// 先前累积值（旧实现缺字段用 0 兜底，会把前一 chunk 的累积值整体清零）。
/// 无 usage 字段时整体不动。
fn apply_openai_usage_chunk(token_usage: &mut TokenUsage, json: &serde_json::Value) {
    if let Some((prompt, completion)) = extract_openai_usage_from_chunk(json) {
        if let Some(p) = prompt {
            token_usage.prompt_tokens = p;
        }
        if let Some(c) = completion {
            token_usage.completion_tokens = c;
        }
    }
}

/// P034: 从 Anthropic SSE chunk 提取 usage（跨事件：message_start 提供 input_tokens，
/// message_delta 提供 output_tokens）。返回 (input 更新, output 更新)，`None` 表示该
/// 事件不携带该字段（保持累积值）。
fn extract_anthropic_usage(
    json: &serde_json::Value,
    current_event: &str,
) -> Option<(Option<u64>, Option<u64>)> {
    if current_event == "message_start" {
        let input = json
            .get("message")
            .and_then(|m| m.get("usage"))
            .and_then(|u| u.get("input_tokens"))
            .and_then(|v| v.as_u64())?;
        Some((Some(input), None))
    } else if current_event == "message_delta" {
        let output = json
            .get("usage")
            .and_then(|u| u.get("output_tokens"))
            .and_then(|v| v.as_u64())?;
        Some((None, Some(output)))
    } else {
        None
    }
}

/// P034: 从 OpenAI SSE chunk 提取 usage（usage 可能在 choices 为空的 chunk 中）。
/// 返回 (prompt, completion)，均为 `Option`——缺失的字段由调用方保留先前累积值
/// （N008：旧实现缺字段用 0 兜底，会把前一 chunk 的累积值整体清零）。
fn extract_openai_usage_from_chunk(json: &serde_json::Value) -> Option<(Option<u64>, Option<u64>)> {
    let usage = json.get("usage")?;
    let prompt = usage.get("prompt_tokens").and_then(|v| v.as_u64());
    let completion = usage.get("completion_tokens").and_then(|v| v.as_u64());
    Some((prompt, completion))
}

/// 非 SSE 响应：完整获取文本 + 打字机效果降级推送。
async fn handle_json_response(
    app: &tauri::AppHandle,
    resp: reqwest::Response,
    conversation_id: &str,
    api_type: &ApiType,
) -> Result<(String, Option<TokenUsage>), String> {
    let result: serde_json::Value = resp.json().await.map_err(|e| format!("Parse: {e}"))?;

    // 使用共享 helper 提取响应文本
    let full_text = request::extract_response_text(&result, api_type).unwrap_or_default();

    // 提取非 SSE 的真实 usage（仅 OpenAI 有 usage 字段）
    let mut token_usage = TokenUsage::default();
    if !is_anthropic(api_type) {
        let (prompt, completion) = request::extract_openai_usage(&result);
        token_usage.prompt_tokens = prompt;
        token_usage.completion_tokens = completion;
    }

    emit_typing_effect(app, conversation_id, &full_text).await;
    let usage = if token_usage.prompt_tokens > 0 || token_usage.completion_tokens > 0 {
        Some(token_usage)
    } else {
        None
    };
    Ok((full_text, usage))
}

/// 发送聊天请求并流式推送结果（Phase 2.3：SSE 流式 + 打字机降级）
/// 返回 (完整文本, 可选的真实 TokenUsage)
pub async fn send_chat_stream(
    app: tauri::AppHandle,
    conversation_id: String,
    base_url: String,
    api_key: String,
    model: String,
    api_type: ApiType,
    messages: Vec<serde_json::Value>,
) -> Result<(String, Option<TokenUsage>), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Client: {e}"))?;

    // 使用共享 helper 构建 URL、请求体和认证头
    let url = request::build_api_url(&base_url, &api_type);
    let body = request::build_request_body(&model, messages, &api_type, DEFAULT_MAX_TOKENS, true);
    let req = request::add_auth_headers(client.post(&url).json(&body), &api_key, &api_type);

    let resp = req
        .send()
        .await
        .map_err(|e| format!("Request to {url} failed: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        let _ = app.emit(
            "llm-stream-chunk",
            LlmStreamPayload {
                conversation_id: conversation_id.clone(),
                chunk: String::new(),
                is_done: false,
                error: Some(format!("HTTP {status}: {err_text}")),
            },
        );
        return Err(format!("HTTP {status}: {err_text}"));
    }

    // 检查 Content-Type，判断是否为 SSE
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if content_type.contains("text/event-stream") {
        handle_sse_stream(&app, resp, &conversation_id, &api_type).await
    } else {
        handle_json_response(&app, resp, &conversation_id, &api_type).await
    }
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn llm_send_message_stream(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    account_id: String,
    conversation_id: String,
    base_url: String,
    api_key: String,
    model: String,
    api_type: ApiType,
    messages: Vec<serde_json::Value>,
) -> Result<(), String> {
    // P102：网络出口收窄——base_url 必须通过 scheme/host 校验，且必须属于
    // 当前账户已登记的 provider（内置默认 ∪ 设置中保存过的地址）。
    // 防止聊天内容（可能含敏感数据）被 XSS 借 LLM 通道外传到任意地址。
    request::validate_llm_base_url(&base_url)?;
    // P016：SSRF 内网段防护——字面内网 IP 已被 validate 拦截，此处对主机名再做
    // 异步解析复核（防 `http://nas.local` 这类解析到内网地址的绕过），与 chat_http 一致。
    request::ensure_public_llm_host(&base_url).await?;
    ensure_registered_provider(&state, &account_id, &base_url)?;
    let prompt_text: String = messages
        .iter()
        .filter_map(|m| {
            m.get("content")
                .and_then(|c| c.as_str())
                .map(|s| s.to_string())
        })
        .collect::<Vec<_>>()
        .join("\n");

    let (full_text, token_usage) = send_chat_stream(
        // P002: clone 保留 app 所有权，流结束后仍需 emit 持久化失败事件。
        app.clone(),
        conversation_id.clone(),
        base_url,
        api_key,
        model.clone(),
        api_type.clone(),
        messages.clone(),
    )
    .await?;

    persist_conversation_reply(
        &app,
        &state,
        &account_id,
        &conversation_id,
        &full_text,
        &messages,
    )?;

    record_and_persist_usage(
        &state,
        &account_id,
        &model,
        &api_type,
        token_usage,
        &prompt_text,
        &full_text,
    )
    .await?;
    Ok(())
}
/// P102/P016：校验 base_url 属于当前账户已登记的 provider（网络出口收窄）。
fn ensure_registered_provider(
    state: &State<'_, AppState>,
    account_id: &str,
    base_url: &str,
) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let config = super::load_config(vg.as_ref(), account_id)?;
    if !super::is_registered_provider_url(&config, base_url) {
        return Err(format!(
            "base_url 未在当前账户登记，已拒绝请求: {}",
            base_url
        ));
    }
    Ok(())
}

/// Auto-save conversation with AI reply after stream completes
/// (ensures data persists even if frontend component is unmounted)。
/// 热路径行级读写（P004）；保存失败落 warn 并 emit 持久化失败事件（P002），不整命令判失败。
fn persist_conversation_reply(
    app: &tauri::AppHandle,
    state: &State<'_, AppState>,
    account_id: &str,
    conversation_id: &str,
    full_text: &str,
    messages: &[serde_json::Value],
) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref();
    // P004: 热路径行级读写——单行加载目标会话、追加助手回复、行级保存，
    // 不再整 blob 解密/深克隆/重写全部会话。
    let mut conv: Option<Conversation> = vault
        .load_conversation(account_id, conversation_id)?
        .and_then(|data| serde_json::from_slice::<Conversation>(&data).ok());
    if let Some(conv_mut) = conv.as_mut() {
        conv_mut.messages.push(ChatMessage {
            role: "assistant".to_string(),
            content: full_text.to_string(),
            created_at: now_iso(),
        });
        conv_mut.updated_at = now_iso();
    } else {
        // Fallback: create new conversation if not found
        let name = messages
            .iter()
            .filter_map(|m| m.get("role").and_then(|r| r.as_str()))
            .zip(
                messages
                    .iter()
                    .filter_map(|m| m.get("content").and_then(|c| c.as_str())),
            )
            .find(|(role, _)| *role == "user")
            .map(|(_, content)| content.chars().take(30).collect::<String>())
            .unwrap_or_default();
        conv = Some(Conversation {
            id: conversation_id.to_string(),
            name,
            is_temporary: false,
            messages: vec![ChatMessage {
                role: "assistant".to_string(),
                content: full_text.to_string(),
                created_at: now_iso(),
            }],
            updated_at: now_iso(),
            deleted_at: None,
        });
    }
    if let Some(conv) = conv {
        if let Err(e) = save_conversation(vault, account_id, &conv) {
            // P002: 保存失败不再静默吞错——落 warn 日志（不含消息内容）并向前端
            // emit 持久化失败事件，用户可见可重试，不再无感知丢失整段对话。
            // （回复已完整流式展示，此处不把整个命令判失败，避免前端误判为
            // 生成中断。）
            tracing::warn!(
                "Failed to persist conversation {} after stream: {}",
                conv.id,
                e
            );
            let _ = app.emit(
                "llm-stream-chunk",
                LlmStreamPayload {
                    conversation_id: conv.id.clone(),
                    chunk: String::new(),
                    is_done: true,
                    // P002-R1: 结构化标记——前端据此区分「持久化失败」（回复已完整展示，
                    // 只 toast 提示，保留内容）与「生成中断」（流错误，替换为错误文案）。
                    error: Some(format!("__LLM_PERSIST_FAILED__: {e}")),
                },
            );
        }
    }
    Ok(())
}

/// 记录 token 用量（真实/兜底）并立即持久化统计到 vault。
async fn record_and_persist_usage(
    state: &State<'_, AppState>,
    account_id: &str,
    model: &str,
    api_type: &ApiType,
    token_usage: Option<TokenUsage>,
    prompt_text: &str,
    full_text: &str,
) -> Result<(), String> {
    let provider_name = format!("{:?}", api_type);
    if let Some(usage) = token_usage {
        let _ = record_usage(
            account_id,
            model,
            &provider_name,
            usage.prompt_tokens,
            usage.completion_tokens,
        )
        .await;
    } else {
        let _ =
            record_usage_fallback(account_id, model, &provider_name, prompt_text, full_text).await;
    }
    // Persist usage stats to vault immediately after recording
    {
        let stats = {
            let map = STATS_MAP.read().await;
            map.get(account_id).cloned().unwrap_or_default()
        };
        let svc = state
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        if let Some(vg) = svc.get_vault_store() {
            let vault = vg.as_ref();
            {
                let _ = save_stats_to_vault(vault, account_id, &stats);
            }
        };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::llm::stats::TokenUsage;

    /// R005-②: usage 缺失 → None（调用方整体不动）。
    #[test]
    fn test_extract_openai_usage_absent() {
        assert_eq!(
            extract_openai_usage_from_chunk(&serde_json::json!({"choices": []})),
            None
        );
        assert_eq!(
            extract_openai_usage_from_chunk(&serde_json::json!({})),
            None
        );
    }

    /// R005-②: 双字段齐备 → 全部提取。
    #[test]
    fn test_extract_openai_usage_both_fields() {
        let json = serde_json::json!({"usage": {"prompt_tokens": 10, "completion_tokens": 20}});
        assert_eq!(
            extract_openai_usage_from_chunk(&json),
            Some((Some(10), Some(20)))
        );
    }

    /// R005-②: 缺字段必须返回 None（而非 0），调用方才可能保留先前累积值。
    #[test]
    fn test_extract_openai_usage_missing_field_yields_none_not_zero() {
        let only_prompt = serde_json::json!({"usage": {"prompt_tokens": 5}});
        assert_eq!(
            extract_openai_usage_from_chunk(&only_prompt),
            Some((Some(5), None))
        );
        let only_completion = serde_json::json!({"usage": {"completion_tokens": 7}});
        assert_eq!(
            extract_openai_usage_from_chunk(&only_completion),
            Some((None, Some(7)))
        );
        // usage 存在但字段非数字 → None 字段
        let bad = serde_json::json!({"usage": {"prompt_tokens": "abc"}});
        assert_eq!(extract_openai_usage_from_chunk(&bad), Some((None, None)));
    }

    /// R005-②: 逐字段更新语义——缺失字段保留先前累积值（N008 修复目标）。
    #[test]
    fn test_apply_openai_usage_chunk_retains_missing_fields() {
        let mut usage = TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 200,
        };
        // 仅 prompt 的 chunk → completion 保留先前值
        apply_openai_usage_chunk(
            &mut usage,
            &serde_json::json!({"usage": {"prompt_tokens": 300}}),
        );
        assert_eq!(usage.prompt_tokens, 300);
        assert_eq!(usage.completion_tokens, 200);
        // 仅 completion 的 chunk → prompt 保留先前值
        apply_openai_usage_chunk(
            &mut usage,
            &serde_json::json!({"usage": {"completion_tokens": 400}}),
        );
        assert_eq!(usage.prompt_tokens, 300);
        assert_eq!(usage.completion_tokens, 400);
        // 无 usage → 完全不动
        apply_openai_usage_chunk(&mut usage, &serde_json::json!({"choices": []}));
        assert_eq!(usage.prompt_tokens, 300);
        assert_eq!(usage.completion_tokens, 400);
    }
}
