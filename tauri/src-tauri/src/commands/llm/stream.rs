use crate::services::llm_context::{build_automatic_system_prompt, ChatContextSelection};
use crate::state::AppState;
use serde::Serialize;
use solosoul_core::{VaultService, VaultSession};
use solosoul_vault::VaultStore;
use std::sync::{Arc, RwLock};
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
    pub account_id: String,
    pub session_generation: u64,
    pub conversation_id: String,
    pub request_id: String,
    pub chunk: String,
    pub is_done: bool,
    pub error: Option<String>,
}

/// RF-002：一次流请求的固定身份。网络等待不持有服务锁或会话门闩。
struct StreamContext {
    service: Arc<RwLock<VaultService>>,
    session: VaultSession,
    conversation_id: String,
    request_id: String,
    emit_event: Box<dyn Fn(LlmStreamPayload) -> Result<(), String> + Send + Sync>,
    #[cfg(test)]
    before_send: Option<Arc<dyn Fn() -> futures::future::BoxFuture<'static, ()> + Send + Sync>>,
}

impl StreamContext {
    fn capture(
        service: &Arc<RwLock<VaultService>>,
        account_id: &str,
        conversation_id: String,
        request_id: Option<String>,
        emit_event: impl Fn(LlmStreamPayload) -> Result<(), String> + Send + Sync + 'static,
    ) -> Result<Self, String> {
        let session = service
            .read()
            .map_err(|_| "Vault service lock poisoned")?
            .capture_session(account_id)?;
        Ok(Self {
            service: service.clone(),
            session,
            conversation_id,
            request_id: request_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            emit_event: Box::new(emit_event),
            #[cfg(test)]
            before_send: None,
        })
    }

    fn with_vault<T>(
        &self,
        commit: impl FnOnce(&VaultStore) -> Result<T, String>,
    ) -> Result<T, String> {
        let owner = self.session.vault().root_owner();
        let _activity = solosoul_core::import_activity::begin_owned_root_activity(owner)?;
        self.service
            .read()
            .map_err(|_| "Vault service lock poisoned")?
            .with_session(&self.session, commit)
    }

    fn emit(&self, chunk: String, is_done: bool, error: Option<String>) -> Result<(), String> {
        // 发布和锁定串行；在前端队列中迟到的事件仍携带完整旧身份供 RF-104 过滤。
        self.with_vault(|_| {
            if let Err(error) = (self.emit_event)(LlmStreamPayload {
                account_id: self.session.account_id().to_owned(),
                session_generation: self.session.generation(),
                conversation_id: self.conversation_id.clone(),
                request_id: self.request_id.clone(),
                chunk,
                is_done,
                error,
            }) {
                // 保留原事件发送的 best-effort 行为；交付失败不能丢失后台回复保存。
                tracing::warn!("Failed to emit LLM stream event: {}", error);
            }
            Ok(())
        })
    }
}

/// 打字机效果：将完整文本逐块推送到前端（降级用）
/// P111: 改为按 CHUNK_SIZE 个字符批量发送，减少 IPC 事件数量。
async fn emit_typing_effect(context: &StreamContext, full_text: &str) -> Result<(), String> {
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
            context.emit(remaining, true, None)?;
            return Ok(());
        }
        let end = std::cmp::min(pos + CHUNK_SIZE, total);
        let chunk: String = chars[pos..end].concat();
        context.emit(chunk, false, None)?;
        pos = end;
        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
    }

    emit_stream_done(context)
}

/// 发送聊天请求并流式推送结果（Phase 2.3：SSE 流式 + 打字机降级）
/// 返回 (完整文本, 可选的真实 TokenUsage)
/// 从 SSE JSON chunk 提取 delta 文本（P026: 转发 core 共享纯函数，
/// 兼容 Anthropic delta.text 与 OpenAI choices[0].delta.content）。
fn extract_delta_text<'a>(json: &'a serde_json::Value, api_type: &ApiType) -> Option<&'a str> {
    solosoul_core::llm::protocol::extract_delta_text(json, api_type)
}

/// SSE 流式解析：逐行解析 data: 行，提取 delta 文本与 usage，事件经 IPC 推送。
async fn handle_sse_stream(
    context: &StreamContext,
    resp: reqwest::Response,
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

            // 处理单行：event/data 前缀、[DONE] 结束标记、delta emit、usage 提取
            if process_sse_line(
                &line,
                context,
                api_type,
                &mut current_event,
                &mut full_text,
                &mut token_usage,
                &mut anthropic_prompt_tokens,
                &mut anthropic_completion_tokens,
            )? {
                emit_stream_done(context)?;
                return Ok((full_text, build_usage_result(&token_usage)));
            }
        }
    }

    // 处理缓冲区中剩余的内容
    if let Some(data) = buffer.trim().strip_prefix("data: ") {
        handle_remaining_data(data, context, api_type, &mut full_text, &mut token_usage)?;
    }

    // 流正常结束
    emit_stream_done(context)?;
    Ok((full_text, build_usage_result(&token_usage)))
}

/// 处理单行 SSE 数据（event:/data: 前缀、[DONE] 结束标记、delta 提取与 emit、usage 提取）。
/// 返回 true 表示命中 [DONE]，调用方应立即结束并发送完成信号。
#[allow(clippy::too_many_arguments)]
fn process_sse_line(
    line: &str,
    context: &StreamContext,
    api_type: &ApiType,
    current_event: &mut String,
    full_text: &mut String,
    token_usage: &mut TokenUsage,
    anthropic_prompt_tokens: &mut u64,
    anthropic_completion_tokens: &mut u64,
) -> Result<bool, String> {
    context.with_vault(|_| Ok(()))?;
    // 处理 event: 行（Anthropic 使用）
    if let Some(event) = line.strip_prefix("event: ") {
        *current_event = event.to_string();
        return Ok(false);
    }

    // 只处理 data: 行
    if !line.starts_with("data: ") {
        return Ok(false);
    }
    let data = &line[6..];

    // OpenAI 风格结束标记
    if data == "[DONE]" {
        return Ok(true);
    }

    // 尝试解析 JSON
    let Ok(json) = serde_json::from_str::<serde_json::Value>(data) else {
        return Ok(false);
    };

    // ── 提取 delta content ──
    let delta_text = extract_delta_text(&json, api_type);

    if let Some(text) = delta_text {
        if !text.is_empty() {
            full_text.push_str(text);
            context.emit(text.to_string(), false, None)?;
        }
    }

    // ── 提取 usage（P034: 按 Anthropic/OpenAI 各抽函数，消除 5 层嵌套）──
    if is_anthropic(api_type) {
        if let Some((input, output)) = extract_anthropic_usage(&json, current_event) {
            if let Some(i) = input {
                *anthropic_prompt_tokens = i;
            }
            if let Some(o) = output {
                *anthropic_completion_tokens = o;
            }
        }
        token_usage.prompt_tokens = *anthropic_prompt_tokens;
        token_usage.completion_tokens = *anthropic_completion_tokens;
    } else {
        // N008/R005: 逐字段更新——缺失字段保留先前累积值，避免整体清零。
        apply_openai_usage_chunk(token_usage, &json);
    }
    Ok(false)
}

/// 发送流结束信号（is_done: true）。
fn emit_stream_done(context: &StreamContext) -> Result<(), String> {
    context.emit(String::new(), true, None)
}

/// 有 usage 时返回 Some，否则 None。
fn build_usage_result(token_usage: &TokenUsage) -> Option<TokenUsage> {
    if token_usage.prompt_tokens > 0 || token_usage.completion_tokens > 0 {
        Some(token_usage.clone())
    } else {
        None
    }
}
/// 处理流结束前缓冲区内最后一行（未换行）的 data 内容。
fn handle_remaining_data(
    data: &str,
    context: &StreamContext,
    api_type: &ApiType,
    full_text: &mut String,
    token_usage: &mut TokenUsage,
) -> Result<(), String> {
    if data == "[DONE]" {
        return Ok(());
    }
    let Ok(json) = serde_json::from_str::<serde_json::Value>(data) else {
        return Ok(());
    };
    if let Some(text) = extract_delta_text(&json, api_type) {
        if !text.is_empty() {
            full_text.push_str(text);
            context.emit(text.to_string(), false, None)?;
        }
    }
    // 剩余内容也可能含 usage（P034: 复用 OpenAI chunk 解析）
    if !is_anthropic(api_type) {
        // N008/R005: 逐字段更新——缺失字段保留先前累积值。
        apply_openai_usage_chunk(token_usage, &json);
    }
    Ok(())
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
/// 事件不携带该字段（保持累积值）。P026: 字段提取转发 core 共享纯函数。
fn extract_anthropic_usage(
    json: &serde_json::Value,
    current_event: &str,
) -> Option<(Option<u64>, Option<u64>)> {
    if current_event == "message_start" {
        solosoul_core::llm::protocol::extract_anthropic_input_tokens(json)
            .map(|input| (Some(input), None))
    } else if current_event == "message_delta" {
        solosoul_core::llm::protocol::extract_anthropic_output_tokens(json)
            .map(|output| (None, Some(output)))
    } else {
        None
    }
}

/// P034: 从 OpenAI SSE chunk 提取 usage（usage 可能在 choices 为空的 chunk 中）。
/// 返回 (prompt, completion)，均为 `Option`——缺失的字段由调用方保留先前累积值
/// （N008：旧实现缺字段用 0 兜底，会把前一 chunk 的累积值整体清零）。
/// P026: 转发 core 共享纯函数。
fn extract_openai_usage_from_chunk(json: &serde_json::Value) -> Option<(Option<u64>, Option<u64>)> {
    solosoul_core::llm::protocol::extract_openai_usage_from_chunk(json)
}

/// 非 SSE 响应：完整获取文本 + 打字机效果降级推送。
async fn handle_json_response(
    context: &StreamContext,
    resp: reqwest::Response,
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

    emit_typing_effect(context, &full_text).await?;
    let usage = if token_usage.prompt_tokens > 0 || token_usage.completion_tokens > 0 {
        Some(token_usage)
    } else {
        None
    };
    Ok((full_text, usage))
}

/// 发送聊天请求并流式推送结果（Phase 2.3：SSE 流式 + 打字机降级）
/// 返回 (完整文本, 可选的真实 TokenUsage)
async fn send_chat_stream(
    context: &StreamContext,
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

    context.with_vault(|_| Ok(()))?;
    let resp = req
        .send()
        .await
        .map_err(|e| format!("Request to {url} failed: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        context.emit(
            String::new(),
            false,
            Some(format!("HTTP {status}: {err_text}")),
        )?;
        return Err(format!("HTTP {status}: {err_text}"));
    }

    // 检查 Content-Type，判断是否为 SSE
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if content_type.contains("text/event-stream") {
        handle_sse_stream(context, resp, &api_type).await
    } else {
        handle_json_response(context, resp, &api_type).await
    }
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn llm_send_message_stream(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    account_id: String,
    conversation_id: String,
    provider_id: String,
    messages: Vec<serde_json::Value>,
    request_id: Option<String>,
    context_selection: Option<ChatContextSelection>,
) -> Result<(), String> {
    // 在任何异步等待前捕获；旧调用方可不传 requestId，由后端生成。
    let context = StreamContext::capture(
        &state.vault_service,
        &account_id,
        conversation_id,
        request_id,
        move |payload| {
            app.emit("llm-stream-chunk", payload)
                .map_err(|e| e.to_string())
        },
    )?;
    run_chat_stream(&context, provider_id, messages, context_selection).await
}

/// RF-005：普通发送只接受 provider ID，配置与凭证来自原会话的同份 Profile。
async fn run_chat_stream(
    context: &StreamContext,
    provider_id: String,
    messages: Vec<serde_json::Value>,
    context_selection: Option<ChatContextSelection>,
) -> Result<(), String> {
    let provider = context.with_vault(|vault| {
        solosoul_core::llm::service::LlmService::new().resolve_chat_provider(
            vault,
            context.session.account_id(),
            &provider_id,
        )
    })?;
    // 保留 P102/P015/P016：已保存的配置同样必须通过 URL、DNS 与登记校验。
    request::validate_llm_base_url(&provider.base_url)?;
    request::ensure_public_llm_host(&provider.base_url).await?;
    ensure_registered_provider(context, &provider.base_url)?;
    run_resolved_chat_stream(
        context,
        provider.base_url,
        provider.api_key,
        provider.model,
        provider.api_type,
        messages,
        context_selection,
    )
    .await
}

/// RF-004：用户消息与自动附加资料分离；旧存储模型保持 string role 兼容。
fn prepare_chat_messages(
    context: &StreamContext,
    selection: &ChatContextSelection,
    messages: Vec<serde_json::Value>,
) -> Result<Vec<serde_json::Value>, String> {
    context.with_vault(|_| Ok(()))?;
    let mut messages = messages
        .into_iter()
        .map(|message| {
            let role = message.get("role").and_then(serde_json::Value::as_str);
            let content = message.get("content").and_then(serde_json::Value::as_str);
            match (role, content) {
                (Some(role @ ("user" | "assistant")), Some(content)) => {
                    Ok(serde_json::json!({"role": role, "content": content}))
                }
                _ => Err(
                    "Chat messages must contain a user/assistant role and text content".to_string(),
                ),
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(prompt) = build_automatic_system_prompt(&context.session, selection)? {
        messages.insert(0, serde_json::json!({"role": "system", "content": prompt}));
    }
    context.with_vault(|_| Ok(()))?;
    Ok(messages)
}

/// 已解析配置后的唯一完成路径；RF-002/RF-004 回归覆盖发送、保存和统计，RF-005 覆盖前置解析。
async fn run_resolved_chat_stream(
    context: &StreamContext,
    base_url: String,
    api_key: String,
    model: String,
    api_type: ApiType,
    messages: Vec<serde_json::Value>,
    context_selection: Option<ChatContextSelection>,
) -> Result<(), String> {
    let messages =
        prepare_chat_messages(context, &context_selection.unwrap_or_default(), messages)?;
    #[cfg(test)]
    if let Some(before_send) = &context.before_send {
        before_send().await;
    }
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
        context,
        base_url,
        api_key,
        model.clone(),
        api_type.clone(),
        messages.clone(),
    )
    .await?;

    persist_conversation_reply(context, &full_text, &messages)?;

    record_and_persist_usage(
        context,
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
fn ensure_registered_provider(context: &StreamContext, base_url: &str) -> Result<(), String> {
    context.with_vault(|vault| {
        let config = super::load_config(vault, context.session.account_id())?;
        if !super::is_registered_provider_url(&config, base_url) {
            return Err(format!(
                "base_url 未在当前账户登记，已拒绝请求: {}",
                base_url
            ));
        }
        Ok(())
    })
}

/// Auto-save conversation with AI reply after stream completes
/// (ensures data persists even if frontend component is unmounted)。
/// 热路径行级读写（P004）；保存失败落 warn 并 emit 持久化失败事件（P002），不整命令判失败。
fn persist_conversation_reply(
    context: &StreamContext,
    full_text: &str,
    messages: &[serde_json::Value],
) -> Result<(), String> {
    let account_id = context.session.account_id();
    let conversation_id = &context.conversation_id;
    let save_result = context.with_vault(|vault| {
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
            save_conversation(vault, account_id, &conv)?;
        }
        Ok(())
    });
    if let Err(e) = save_result {
        // P002: 保存失败不再静默吞错——落 warn 日志（不含消息内容）并向前端
        // emit 持久化失败事件，用户可见可重试，不再无感知丢失整段对话。
        // （回复已完整流式展示，此处不把整个命令判失败，避免前端误判为
        // 生成中断。）
        tracing::warn!(
            "Failed to persist conversation {} after stream: {}",
            conversation_id,
            e
        );
        // 重新保护发布；若会话已失效，直接返回失效错误，不向新会话发通知。
        context.emit(
            String::new(),
            true,
            Some(format!("__LLM_PERSIST_FAILED__: {e}")),
        )?;
    }
    Ok(())
}

/// 记录 token 用量（真实/兜底）并立即持久化统计到 vault。
async fn record_and_persist_usage(
    context: &StreamContext,
    model: &str,
    api_type: &ApiType,
    token_usage: Option<TokenUsage>,
    prompt_text: &str,
    full_text: &str,
) -> Result<(), String> {
    let provider_name = format!("{:?}", api_type);
    let usage = token_usage.unwrap_or_else(|| TokenUsage {
        prompt_tokens: estimate_tokens(prompt_text),
        completion_tokens: estimate_tokens(full_text),
    });
    // 先等待统计锁，再进入同步会话临界区。不得持会话门闩跨 await。
    let mut map = STATS_MAP.write().await;
    context.with_vault(|vault| {
        let account_id = context.session.account_id();
        let update: Result<LlmUsageStats, String> = (|| {
            let mut stats = match map.get(account_id) {
                Some(stats) => stats.clone(),
                None => load_stats_from_vault(vault, account_id)?,
            };
            super::stats::accumulate_usage(
                &mut stats,
                model,
                &provider_name,
                usage.prompt_tokens,
                usage.completion_tokens,
            );
            save_stats_to_vault(vault, account_id, &stats)?;
            Ok(stats)
        })();
        match update {
            Ok(stats) => {
                map.insert(account_id.to_owned(), stats);
            }
            // 保留统计的 best-effort 语义：已完成的回复不能因统计 IO 失败变成生成失败。
            // 会话失效仍由外层 with_vault 拒绝，不能将失效错误一并吞掉。
            Err(error) => tracing::warn!("Failed to persist LLM usage statistics: {}", error),
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    mod rf004;
    mod rf005;

    use super::*;
    use crate::commands::llm::stats::TokenUsage;
    use std::sync::Mutex;
    use tempfile::TempDir;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    struct StreamFixture {
        context: StreamContext,
        events: Arc<Mutex<Vec<LlmStreamPayload>>>,
        _dir: TempDir,
    }

    impl StreamFixture {
        fn new() -> Self {
            let dir = TempDir::new().unwrap();
            let service = Arc::new(RwLock::new(VaultService::with_base_path(
                dir.path().join("vault"),
            )));
            let account = format!("acc_{}", uuid::Uuid::new_v4().simple());
            service
                .read()
                .unwrap()
                .create_account_with_id(&account, "A", "password123", None)
                .unwrap();
            let events = Arc::new(Mutex::new(Vec::new()));
            let sink = events.clone();
            let context = StreamContext::capture(
                &service,
                &account,
                "conversation".into(),
                None,
                move |payload| {
                    sink.lock().unwrap().push(payload);
                    Ok(())
                },
            )
            .unwrap();
            context
                .with_vault(|vault| {
                    save_conversation(
                        vault,
                        &account,
                        &Conversation {
                            id: context.conversation_id.clone(),
                            name: "test".into(),
                            is_temporary: false,
                            messages: vec![ChatMessage {
                                role: "user".into(),
                                content: "hello".into(),
                                created_at: now_iso(),
                            }],
                            updated_at: now_iso(),
                            deleted_at: None,
                        },
                    )?;
                    save_stats_to_vault(
                        vault,
                        &account,
                        &LlmUsageStats {
                            usage_count: 5,
                            ..Default::default()
                        },
                    )
                })
                .unwrap();
            Self {
                context,
                events,
                _dir: dir,
            }
        }

        fn conversation(&self) -> Conversation {
            self.context
                .with_vault(|vault| {
                    let data = vault
                        .load_conversation(
                            self.context.session.account_id(),
                            &self.context.conversation_id,
                        )?
                        .unwrap();
                    serde_json::from_slice(&data).map_err(|e| e.to_string())
                })
                .unwrap()
        }
    }

    /// 真实 HTTP 响应，头部立即发送，正文受通道屏障控制；不访问外部服务。
    async fn paused_server(
        content_type: &str,
        body: &str,
    ) -> (
        String,
        tokio::sync::oneshot::Sender<()>,
        tokio::task::JoinHandle<()>,
    ) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (release, wait) = tokio::sync::oneshot::channel();
        let headers = format!("HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
        let body = body.to_owned();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                socket.read_exact(&mut byte).await.unwrap();
                request.push(byte[0]);
            }
            let length = String::from_utf8_lossy(&request)
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .and_then(|v| v.trim().parse::<usize>().ok())
                })
                .unwrap_or(0);
            socket.read_exact(&mut vec![0; length]).await.unwrap();
            socket.write_all(headers.as_bytes()).await.unwrap();
            tokio::time::timeout(std::time::Duration::from_secs(10), wait)
                .await
                .unwrap()
                .unwrap();
            socket.write_all(body.as_bytes()).await.unwrap();
        });
        (format!("http://{address}"), release, server)
    }

    async fn paused_response(
        content_type: &str,
        body: &str,
    ) -> (
        reqwest::Response,
        tokio::sync::oneshot::Sender<()>,
        tokio::task::JoinHandle<()>,
    ) {
        let (url, release, server) = paused_server(content_type, body).await;
        let response = reqwest::Client::new().get(url).send().await.unwrap();
        (response, release, server)
    }

    const SSE_REPLY: &str = "data: {\"choices\":[{\"delta\":{\"content\":\"reply\"}}]}\n\ndata: {\"usage\":{\"prompt_tokens\":3,\"completion_tokens\":2}}\n\ndata: [DONE]\n\n";

    #[tokio::test]
    async fn rf002_normal_stream_has_fixed_identity_and_one_saved_reply() {
        let fixture = StreamFixture::new();
        let context = &fixture.context;
        let (url, release, server) = paused_server("text/event-stream", SSE_REPLY).await;
        release.send(()).unwrap();
        run_resolved_chat_stream(
            context,
            url,
            "test-key".into(),
            "model".into(),
            ApiType::OpenAI,
            vec![serde_json::json!({"role": "user", "content": "hello"})],
            None,
        )
        .await
        .unwrap();
        server.await.unwrap();
        let conversation = fixture.conversation();
        assert_eq!(conversation.messages.len(), 2);
        assert_eq!(conversation.messages[1].content, "reply");
        let stats = context
            .with_vault(|vault| load_stats_from_vault(vault, context.session.account_id()))
            .unwrap();
        assert_eq!(
            (
                stats.usage_count,
                stats.prompt_tokens,
                stats.completion_tokens
            ),
            (6, 3, 2)
        );
        let events = fixture.events.lock().unwrap().clone();
        assert_eq!(events.iter().filter(|e| e.is_done).count(), 1);
        assert_eq!(
            events.iter().map(|e| e.chunk.as_str()).collect::<String>(),
            "reply"
        );
        for event in events.iter() {
            assert_eq!(event.account_id, context.session.account_id());
            assert_eq!(event.session_generation, context.session.generation());
            assert_eq!(event.conversation_id, context.conversation_id);
            assert_eq!(event.request_id, context.request_id);
            assert!(event.error.is_none());
            let json = serde_json::to_value(event).unwrap();
            assert!(json.get("sessionGeneration").is_some());
            assert!(json.get("requestId").is_some());
        }
        STATS_MAP.write().await.remove(context.session.account_id());
    }

    #[tokio::test]
    async fn rf002_paused_stream_rejects_switch_reunlock_and_lock() {
        for mode in ["switch", "reunlock", "lock"] {
            let fixture = StreamFixture::new();
            let context = &fixture.context;
            let (response, release, server) = paused_response("text/event-stream", SSE_REPLY).await;
            let account = context.session.account_id();
            let other = format!("acc_{}", uuid::Uuid::new_v4().simple());
            // HTTP 正文仍在等待，锁定立即完成；随后释放响应不能产生事件或写入。
            {
                let service = context.service.read().unwrap();
                service.lock();
                if mode == "switch" {
                    service
                        .create_account_with_id(&other, "B", "password456", None)
                        .unwrap();
                } else if mode == "reunlock" {
                    service.unlock(account, "password123").unwrap();
                }
            }
            release.send(()).unwrap();
            assert!(handle_sse_stream(context, response, &ApiType::OpenAI)
                .await
                .is_err());
            server.await.unwrap();
            assert!(persist_conversation_reply(context, "late", &[]).is_err());
            assert!(record_and_persist_usage(
                context,
                "model",
                &ApiType::OpenAI,
                None,
                "prompt",
                "late"
            )
            .await
            .is_err());
            assert!(fixture.events.lock().unwrap().is_empty());
            assert!(!STATS_MAP.read().await.contains_key(account));
            let service = context.service.read().unwrap();
            if mode == "switch" {
                let vault = service.get_vault_store().unwrap();
                assert!(vault.list_profiles().unwrap().is_empty());
                assert!(vault.list_conversations(&other).unwrap().is_empty());
            }
            service.unlock(account, "password123").unwrap();
            let vault = service.get_vault_store().unwrap();
            let data = vault
                .load_conversation(account, &context.conversation_id)
                .unwrap()
                .unwrap();
            assert_eq!(
                serde_json::from_slice::<Conversation>(&data)
                    .unwrap()
                    .messages
                    .len(),
                1
            );
            assert_eq!(
                load_stats_from_vault(&vault, account).unwrap().usage_count,
                5
            );
        }
    }

    #[tokio::test]
    async fn rf002_json_fallback_and_late_completion_use_same_guard() {
        let fixture = StreamFixture::new();
        let context = &fixture.context;
        let (response, release, server) = paused_response(
            "application/json",
            r#"{"choices":[{"message":{"content":"json reply"}}]}"#,
        )
        .await;
        release.send(()).unwrap();
        let (text, usage) = handle_json_response(context, response, &ApiType::OpenAI)
            .await
            .unwrap();
        server.await.unwrap();
        assert_eq!(text, "json reply");
        assert!(usage.is_none());
        let event_count = fixture.events.lock().unwrap().len();
        context.service.read().unwrap().lock();
        assert!(emit_typing_effect(context, "late").await.is_err());
        assert!(persist_conversation_reply(context, &text, &[]).is_err());
        assert!(record_and_persist_usage(
            context,
            "model",
            &ApiType::OpenAI,
            usage,
            "prompt",
            &text
        )
        .await
        .is_err());
        assert_eq!(fixture.events.lock().unwrap().len(), event_count);
    }

    #[tokio::test]
    async fn rf002_stats_io_failure_does_not_fail_completed_reply() {
        for cached in [false, true] {
            let fixture = StreamFixture::new();
            let context = &fixture.context;
            let account = context.session.account_id();
            if cached {
                STATS_MAP.write().await.insert(
                    account.to_owned(),
                    LlmUsageStats {
                        usage_count: 5,
                        ..Default::default()
                    },
                );
            }
            // 无缓存时触发真实读取解析错误；有缓存时触发真实偏好写入错误。
            let corrupt_data = if cached {
                b"[]".to_vec()
            } else {
                b"not-json".to_vec()
            };
            context
                .with_vault(|vault| {
                    vault.save_profile(&solosoul_vault::Profile::new_with_id(
                        account,
                        account,
                        corrupt_data.clone(),
                    ))
                })
                .unwrap();
            let (url, release, server) = paused_server("text/event-stream", SSE_REPLY).await;
            release.send(()).unwrap();
            run_resolved_chat_stream(
                context,
                url,
                "test-key".into(),
                "model".into(),
                ApiType::OpenAI,
                vec![serde_json::json!({"role": "user", "content": "hello"})],
                None,
            )
            .await
            .unwrap();
            server.await.unwrap();
            let conversation = fixture.conversation();
            assert_eq!(conversation.messages.len(), 2);
            assert_eq!(conversation.messages[1].content, "reply");
            let events = fixture.events.lock().unwrap().clone();
            assert_eq!(events.iter().filter(|e| e.is_done).count(), 1);
            assert!(events.iter().all(|e| e.error.is_none()));
            let stored = context
                .with_vault(|vault| vault.load_profile(account))
                .unwrap()
                .unwrap();
            assert_eq!(stored.data, corrupt_data);
            let mut map = STATS_MAP.write().await;
            if cached {
                assert_eq!(map.get(account).unwrap().usage_count, 5);
            } else {
                assert!(!map.contains_key(account));
            }
            map.remove(account);
        }
    }

    #[tokio::test]
    async fn rf002_event_delivery_failure_still_persists_reply() {
        let mut fixture = StreamFixture::new();
        fixture.context.emit_event = Box::new(|_| Err("injected delivery failure".into()));
        let context = &fixture.context;
        let (url, release, server) = paused_server("text/event-stream", SSE_REPLY).await;
        release.send(()).unwrap();
        run_resolved_chat_stream(
            context,
            url,
            "test-key".into(),
            "model".into(),
            ApiType::OpenAI,
            vec![serde_json::json!({"role": "user", "content": "hello"})],
            None,
        )
        .await
        .unwrap();
        server.await.unwrap();
        let conversation = fixture.conversation();
        assert_eq!(conversation.messages.len(), 2);
        assert_eq!(conversation.messages[1].content, "reply");
        let stats = context
            .with_vault(|vault| load_stats_from_vault(vault, context.session.account_id()))
            .unwrap();
        assert_eq!(stats.usage_count, 6);
        STATS_MAP.write().await.remove(context.session.account_id());
    }

    #[tokio::test]
    async fn rf002_stats_lock_wait_rechecks_session_before_mutation() {
        let fixture = StreamFixture::new();
        let context = &fixture.context;
        let stats_guard = STATS_MAP.write().await;
        let pending =
            record_and_persist_usage(context, "model", &ApiType::OpenAI, None, "prompt", "reply");
        tokio::pin!(pending);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(20), &mut pending)
                .await
                .is_err()
        );
        context.service.read().unwrap().lock();
        drop(stats_guard);
        assert!(pending.await.is_err());
        assert!(!STATS_MAP
            .read()
            .await
            .contains_key(context.session.account_id()));
    }

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
