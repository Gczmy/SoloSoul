use crate::commands::vault_handle;
use crate::state::AppState;
use tauri::State;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};

// ── Commands ───────────────────────────────────────────────

use super::*;
#[tauri::command]
pub async fn llm_get_config(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<LlmConfig, String> {
    let vault = vault_handle(&state)?;
    load_config(&vault, &account_id)
}

#[tauri::command]
pub async fn llm_get_providers(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<Vec<ProviderWithKey>, String> {
    let vault = vault_handle(&state)?;
    // P019: 合并逻辑收敛到 llm::merge_providers_with_keys（含 embedding_model 同步，
    // 原 if 分支漏同步该字段）；此处仅做密钥掩码展示。
    let mut defaults = super::merge_providers_with_keys(&vault, &account_id)?;
    for p in &mut defaults {
        if !p.api_key.is_empty() {
            p.api_key = "••••••••".to_string();
        }
    }
    Ok(defaults)
}

#[tauri::command]
pub async fn llm_save_provider(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    account_id: String,
    provider: ProviderWithKey,
) -> Result<(), String> {
    // P102：保存前校验 base_url 的 scheme/host，拒绝向非法地址登记 provider。
    super::request::validate_llm_base_url(&provider.base_url)?;
    let vault = vault_handle(&state)?;
    let mut config = load_config(&vault, &account_id)?;

    // N-4：登记门禁闭环——**未登记**的外部 URL（非内置默认 ∪ 非已保存 config）
    // 必须经原生确认对话框（`app.dialog()` 为系统级原生对话框，webview 内 XSS
    // 无法程序化点击，彻底杜绝「先登记恶意 provider 再外传」两步绕过）。
    // 已登记地址的再次保存（编辑）与内置默认直接放行。
    if !super::is_registered_provider_url(&config, &provider.base_url) {
        let (tx, rx) = tokio::sync::oneshot::channel::<bool>();
        let url = provider.base_url.clone();
        app.dialog()
            .message(format!(
                "确认将此地址登记为 AI Provider？\n\n{}\n\n仅登记后，该地址才会被用于发送聊天 / 嵌入请求。",
                url
            ))
            .title("AI Provider 登记确认")
            .buttons(MessageDialogButtons::OkCancelCustom("确认登记".to_string(), "取消".to_string()))
            .show(move |confirmed| {
                let _ = tx.send(confirmed);
            });
        // 超时兜底：对话框异常未回调时不得永久挂起命令（等待用户输入通常秒级）。
        let confirmed = tokio::time::timeout(std::time::Duration::from_secs(120), rx)
            .await
            .map_err(|_| "登记确认等待超时".to_string())?
            .map_err(|_| "登记确认对话框未响应".to_string())?;
        if !confirmed {
            return Err("已取消 AI Provider 登记".to_string());
        }
    }
    let api_key = if provider.is_built_in && provider.api_key == "••••••••" {
        String::new()
    } else {
        provider.api_key.clone()
    };
    if !api_key.is_empty() {
        save_api_key(&vault, &account_id, &provider.id, &api_key)?;
    }
    let pc = ProviderConfig {
        id: provider.id.clone(),
        name: provider.name,
        base_url: provider.base_url,
        model: provider.model,
        is_enabled: provider.is_enabled,
        is_built_in: provider.is_built_in,
        api_type: provider.api_type,
        embedding_model: provider.embedding_model,
    };
    if let Some(e) = config.providers.iter_mut().find(|p| p.id == pc.id) {
        *e = pc;
    } else {
        config.providers.push(pc);
    }
    save_config(&vault, &account_id, &config)
}

#[tauri::command]
pub async fn llm_set_active_provider(
    state: State<'_, AppState>,
    account_id: String,
    provider_id: String,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let mut config = load_config(&vault, &account_id)?;
    config.active_provider_id = Some(provider_id);
    save_config(&vault, &account_id, &config)
}

#[tauri::command]
pub async fn llm_set_ai_features(
    state: State<'_, AppState>,
    account_id: String,
    features: AiFeatures,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let mut config = load_config(&vault, &account_id)?;
    config.ai_features_enabled = features;
    save_config(&vault, &account_id, &config)
}

#[tauri::command]
pub async fn llm_set_system_prompt_switch(
    state: State<'_, AppState>,
    account_id: String,
    enabled: bool,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let mut config = load_config(&vault, &account_id)?;
    config.include_system_prompt = enabled;
    save_config(&vault, &account_id, &config)
}

/// Toggle local embedding and set the active model ID.
#[tauri::command]
pub async fn llm_set_local_embedding(
    state: State<'_, AppState>,
    account_id: String,
    enabled: bool,
    model_id: Option<String>,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let mut config = load_config(&vault, &account_id)?;
    config.use_local_embedding = enabled;
    config.local_embed_model_id = model_id;
    save_config(&vault, &account_id, &config)?;
    crate::local_embed::clear_embedder_cache();
    Ok(())
}

#[tauri::command]
pub async fn llm_accept_risk(state: State<'_, AppState>, account_id: String) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let mut config = load_config(&vault, &account_id)?;
    config.has_accepted_risk = true;
    save_config(&vault, &account_id, &config)?;
    crate::commands::log_audit_best_effort(
        &vault,
        "llm_risk_accepted",
        "preference",
        Some(&account_id),
        None,
        "user",
        None,
    );
    Ok(())
}

#[tauri::command]
pub async fn llm_get_api_key(
    state: State<'_, AppState>,
    account_id: String,
    provider_id: String,
) -> Result<String, String> {
    let vault = vault_handle(&state)?;
    load_api_keys(&vault, &account_id).map(|k| k.get(&provider_id).cloned().unwrap_or_default())
}

#[tauri::command]
pub async fn llm_delete_provider(
    state: State<'_, AppState>,
    account_id: String,
    provider_id: String,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let mut config = load_config(&vault, &account_id)?;
    config.providers.retain(|p| p.id != provider_id);
    if config.active_provider_id.as_deref() == Some(&provider_id) {
        config.active_provider_id = config.providers.first().map(|p| p.id.clone());
    }
    save_config(&vault, &account_id, &config)
}

pub(crate) fn is_anthropic(api_type: &ApiType) -> bool {
    matches!(api_type, ApiType::Anthropic)
}
