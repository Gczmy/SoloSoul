use crate::state::AppState;
use serde::{Deserialize, Serialize};
use solosoul_vault::VaultStore;
use tauri::State;

// =============================================================================
// Unified Chat Command (§28 Phase 2.2)
// =============================================================================

use super::*;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatRequest {
    pub account_id: String,
    pub conversation_id: String,
    pub prompt: String,
    pub history: Vec<ChatMessage>,
    pub include_system_prompt: bool,
    pub include_help_doc: bool,
    pub language: String,
}

#[tauri::command]
pub async fn llm_chat(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    request: ChatRequest,
) -> Result<(), String> {
    // 1. 读取 Provider 配置 + 解密 API Key（同步块，vault_guard 在此释放）
    let (base_url, api_key, model, api_type) = {
        let svc = state
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
        let vault = vault_guard.as_ref();

        let config = load_config(vault, &request.account_id)?;
        let active_id = config.active_provider_id.ok_or("No active provider")?;

        let providers = load_providers_with_keys(vault, &request.account_id)?;
        let active = providers
            .into_iter()
            .find(|p| p.id == active_id)
            .ok_or("Active provider not found")?;

        if !active.is_enabled {
            return Err("Provider is disabled".to_string());
        }

        let key = if active.api_key == "••••••••" {
            load_api_keys(vault, &request.account_id)?
                .get(&active.id)
                .cloned()
                .unwrap_or_default()
        } else {
            active.api_key
        };

        (active.base_url, key, active.model, active.api_type)
    };

    // 2. 获取使用统计（.await，此时 vault_guard 已释放）
    let stats = {
        let map = STATS_MAP.read().await;
        map.get(&request.account_id).cloned().unwrap_or_default()
    };

    // 3. 构建 messages 数组
    let mut messages: Vec<serde_json::Value> = Vec::new();

    // 3a. 系统提示词（重新获取 vault + 查询已安装插件）
    if request.include_system_prompt {
        let system_prompt = {
            let svc = state
                .vault_service
                .read()
                .map_err(|_| "Vault service lock poisoned".to_string())?;
            let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
            let vault = vault_guard.as_ref();

            // 获取已安装插件列表（失败时不阻塞对话，降至空列表）
            let plugins = state.plugin_manager.list_installed().unwrap_or_default();

            crate::services::llm_context::build_context(
                &request.account_id,
                vault,
                stats.usage_count,
                stats.prompt_tokens,
                stats.completion_tokens,
                stats.total_tokens,
                &request.language,
                &plugins,
            )?
        };
        if !system_prompt.is_empty() {
            messages.push(serde_json::json!({"role": "system", "content": system_prompt}));
        }
    }

    // 3b. 帮助文档（不需要 vault）
    if request.include_help_doc {
        let guides = find_relevant_guides_internal(&request.prompt, &request.language)?;
        if !guides.is_empty() {
            let mut doc_parts = vec!["---".to_string()];
            doc_parts.push(
                "以下是与用户问题相关的功能使用文档，请参考这些信息回答用户问题。".to_string(),
            );
            for (i, guide) in guides.iter().enumerate() {
                doc_parts.push(format!(
                    "\n【文档 {}：{}】\n{}",
                    i + 1,
                    guide.title,
                    guide.content
                ));
            }
            doc_parts.push("\n【文档结束】\n---".to_string());
            messages.push(serde_json::json!({"role": "system", "content": doc_parts.join("\n")}));
        }
    }

    // 3c. 历史对话
    for msg in &request.history {
        messages.push(serde_json::json!({
            "role": msg.role,
            "content": msg.content,
        }));
    }

    // 3d. 当前用户输入
    messages.push(serde_json::json!({"role": "user", "content": request.prompt}));

    // 4. 构建 prompt_text 用于统计
    let prompt_text: String = messages
        .iter()
        .filter_map(|m| {
            m.get("content")
                .and_then(|c| c.as_str())
                .map(|s| s.to_string())
        })
        .collect::<Vec<_>>()
        .join("\n");

    // 5. 发送请求（复用 send_chat_stream，Phase 2.3 将替换为 SSE）
    let (full_text, token_usage) = send_chat_stream(
        app,
        request.conversation_id.clone(),
        base_url,
        api_key,
        model.clone(),
        api_type.clone(),
        messages,
    )
    .await?;

    // 6. 记录统计
    let provider_name = format!("{:?}", api_type);
    if let Some(usage) = token_usage {
        let _ = record_usage(
            &request.account_id,
            &model,
            &provider_name,
            usage.prompt_tokens,
            usage.completion_tokens,
        )
        .await;
    } else {
        let _ = record_usage_fallback(
            &request.account_id,
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
            map.get(&request.account_id).cloned().unwrap_or_default()
        };
        let svc = state
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        if let Some(vg) = svc.get_vault_store() {
            let vault = vg.as_ref();
            {
                let _ = save_stats_to_vault(vault, &request.account_id, &stats);
            }
        };
    }

    Ok(())
}

// Helper: load providers with decrypted keys (internal reuse)
pub fn load_providers_with_keys(
    vault: &VaultStore,
    account_id: &str,
) -> Result<Vec<ProviderWithKey>, String> {
    let config = load_config(vault, account_id)?;
    let keys = load_api_keys(vault, account_id)?;
    let mut defaults = default_providers();
    for saved in &config.providers {
        if let Some(d) = defaults.iter_mut().find(|d| d.id == saved.id) {
            d.name = saved.name.clone();
            d.base_url = saved.base_url.clone();
            d.model = saved.model.clone();
            d.is_enabled = saved.is_enabled;
            d.api_type = saved.api_type.clone();
            d.embedding_model = saved.embedding_model.clone();
            d.api_key = keys.get(&saved.id).cloned().unwrap_or_default();
        } else {
            defaults.push(ProviderWithKey {
                id: saved.id.clone(),
                name: saved.name.clone(),
                base_url: saved.base_url.clone(),
                model: saved.model.clone(),
                is_enabled: saved.is_enabled,
                is_built_in: false,
                api_key: keys.get(&saved.id).cloned().unwrap_or_default(),
                api_type: saved.api_type.clone(),
                embedding_model: saved.embedding_model.clone(),
            });
        }
    }
    Ok(defaults)
}
