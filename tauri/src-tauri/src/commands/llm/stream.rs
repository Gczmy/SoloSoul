use crate::state::AppState;
use serde::Serialize;
use tauri::State;

// =============================================================================
// Streaming Response (§5.3)
// =============================================================================

use tauri::Emitter;

use super::*;
use super::request;

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
        .map_err(|e| format!("Client: {}", e))?;

    // 使用共享 helper 构建 URL、请求体和认证头
    let url = request::build_api_url(&base_url, &api_type);
    let body = request::build_request_body(&model, messages, &api_type, DEFAULT_MAX_TOKENS, true);
    let req = request::add_auth_headers(client.post(&url).json(&body), &api_key, &api_type);

    let resp = req
        .send()
        .await
        .map_err(|e| format!("Request to {} failed: {}", url, e))?;

    let status = resp.status();
    if !status.is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        let _ = app.emit(
            "llm-stream-chunk",
            LlmStreamPayload {
                conversation_id: conversation_id.clone(),
                chunk: String::new(),
                is_done: false,
                error: Some(format!("HTTP {}: {}", status, err_text)),
            },
        );
        return Err(format!("HTTP {}: {}", status, err_text));
    }

    // 检查 Content-Type，判断是否为 SSE
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let is_sse = content_type.contains("text/event-stream");

    if is_sse {
        // ===================== SSE 流式解析 =====================
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
            let chunk = chunk_result.map_err(|e| format!("Stream error: {}", e))?;
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
                            conversation_id: conversation_id.clone(),
                            chunk: String::new(),
                            is_done: true,
                            error: None,
                        },
                    );
                    let usage =
                        if token_usage.prompt_tokens > 0 || token_usage.completion_tokens > 0 {
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
                let delta_text = if is_anthropic(&api_type) {
                    json.get("delta")
                        .and_then(|d| d.get("text"))
                        .and_then(|t| t.as_str())
                } else {
                    json.get("choices")
                        .and_then(|c| c.get(0))
                        .and_then(|choice| choice.get("delta"))
                        .and_then(|delta| delta.get("content"))
                        .and_then(|c| c.as_str())
                };

                if let Some(text) = delta_text {
                    if !text.is_empty() {
                        full_text.push_str(text);
                        let _ = app.emit(
                            "llm-stream-chunk",
                            LlmStreamPayload {
                                conversation_id: conversation_id.clone(),
                                chunk: text.to_string(),
                                is_done: false,
                                error: None,
                            },
                        );
                    }
                }

                // ── 提取 usage ──
                if is_anthropic(&api_type) {
                    if current_event == "message_start" {
                        if let Some(input_tokens) = json
                            .get("message")
                            .and_then(|m| m.get("usage"))
                            .and_then(|u| u.get("input_tokens"))
                            .and_then(|v| v.as_u64())
                        {
                            anthropic_prompt_tokens = input_tokens;
                        }
                    } else if current_event == "message_delta" {
                        if let Some(output_tokens) = json
                            .get("usage")
                            .and_then(|u| u.get("output_tokens"))
                            .and_then(|v| v.as_u64())
                        {
                            anthropic_completion_tokens = output_tokens;
                        }
                    }
                    token_usage.prompt_tokens = anthropic_prompt_tokens;
                    token_usage.completion_tokens = anthropic_completion_tokens;
                } else {
                    // OpenAI: usage 可能在 choices 为空的 chunk 中
                    if let Some(usage) = json.get("usage") {
                        if let Some(prompt) = usage.get("prompt_tokens").and_then(|v| v.as_u64()) {
                            token_usage.prompt_tokens = prompt;
                        }
                        if let Some(completion) =
                            usage.get("completion_tokens").and_then(|v| v.as_u64())
                        {
                            token_usage.completion_tokens = completion;
                        }
                    }
                }
            }
        }

        // 处理缓冲区中剩余的内容
        let remaining = buffer.trim();
        if let Some(data) = remaining.strip_prefix("data: ") {
            if data != "[DONE]" {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                    let delta_text = if is_anthropic(&api_type) {
                        json.get("delta")
                            .and_then(|d| d.get("text"))
                            .and_then(|t| t.as_str())
                    } else {
                        json.get("choices")
                            .and_then(|c| c.get(0))
                            .and_then(|choice| choice.get("delta"))
                            .and_then(|delta| delta.get("content"))
                            .and_then(|c| c.as_str())
                    };
                    if let Some(text) = delta_text {
                        if !text.is_empty() {
                            full_text.push_str(text);
                            let _ = app.emit(
                                "llm-stream-chunk",
                                LlmStreamPayload {
                                    conversation_id: conversation_id.clone(),
                                    chunk: text.to_string(),
                                    is_done: false,
                                    error: None,
                                },
                            );
                        }
                    }
                    // 剩余内容也可能含 usage
                    if !is_anthropic(&api_type) {
                        if let Some(usage) = json.get("usage") {
                            if let Some(prompt) =
                                usage.get("prompt_tokens").and_then(|v| v.as_u64())
                            {
                                token_usage.prompt_tokens = prompt;
                            }
                            if let Some(completion) =
                                usage.get("completion_tokens").and_then(|v| v.as_u64())
                            {
                                token_usage.completion_tokens = completion;
                            }
                        }
                    }
                }
            }
        }

        // 流正常结束
        let _ = app.emit(
            "llm-stream-chunk",
            LlmStreamPayload {
                conversation_id: conversation_id.clone(),
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
    } else {
        // ===================== 非 SSE：完整获取 + 打字机效果 =====================
        let result: serde_json::Value = resp.json().await.map_err(|e| format!("Parse: {}", e))?;

        // 使用共享 helper 提取响应文本
        let full_text = request::extract_response_text(&result, &api_type).unwrap_or_default();

        // 提取非 SSE 的真实 usage（仅 OpenAI 有 usage 字段）
        let mut token_usage = TokenUsage::default();
        if !is_anthropic(&api_type) {
            let (prompt, completion) = request::extract_openai_usage(&result);
            token_usage.prompt_tokens = prompt;
            token_usage.completion_tokens = completion;
        }

        emit_typing_effect(&app, &conversation_id, &full_text).await;
        let usage = if token_usage.prompt_tokens > 0 || token_usage.completion_tokens > 0 {
            Some(token_usage)
        } else {
            None
        };
        Ok((full_text, usage))
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
        app,
        conversation_id.clone(),
        base_url,
        api_key,
        model.clone(),
        api_type.clone(),
        messages.clone(),
    )
    .await?;

    // Auto-save conversation with AI reply after stream completes
    // (ensures data persists even if frontend component is unmounted)
    {
        let svc = state
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
        let vault = vg.as_ref();
        let mut convs = load_conversations(vault, &account_id)?;
        if let Some(conv) = convs.iter_mut().find(|c| c.id == conversation_id) {
            conv.messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: full_text.clone(),
                created_at: now_iso(),
            });
            conv.updated_at = now_iso();
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
            convs.push(Conversation {
                id: conversation_id,
                name,
                is_temporary: false,
                messages: vec![ChatMessage {
                    role: "assistant".to_string(),
                    content: full_text.clone(),
                    created_at: now_iso(),
                }],
                updated_at: now_iso(),
                deleted_at: None,
            });
        }
        let _ = save_conversations(vault, &account_id, &convs);
    }

    let provider_name = format!("{:?}", api_type);
    if let Some(usage) = token_usage {
        let _ = record_usage(
            &account_id,
            &model,
            &provider_name,
            usage.prompt_tokens,
            usage.completion_tokens,
        )
        .await;
    } else {
        let _ = record_usage_fallback(
            &account_id,
            &model,
            &provider_name,
            &prompt_text,
            &full_text,
        )
        .await;
    }
    // Persist usage stats to vault immediately after recording
    {
        let stats = {
            let map = STATS_MAP.read().await;
            map.get(&account_id).cloned().unwrap_or_default()
        };
        let svc = state
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        if let Some(vg) = svc.get_vault_store() {
            let vault = vg.as_ref();
            {
                let _ = save_stats_to_vault(vault, &account_id, &stats);
            }
        };
    }
    Ok(())
}
