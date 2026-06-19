use crate::commands::vault_handle;
use crate::state::AppState;
use tauri::State;

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
    let config = load_config(&vault, &account_id)?;
    let keys = load_api_keys(&vault, &account_id)?;
    let mut defaults = default_providers();
    for saved in &config.providers {
        if let Some(d) = defaults.iter_mut().find(|d| d.id == saved.id) {
            d.name = saved.name.clone();
            d.base_url = saved.base_url.clone();
            d.model = saved.model.clone();
            d.is_enabled = saved.is_enabled;
            d.api_key = keys.get(&saved.id).cloned().unwrap_or_default();
            d.api_type = saved.api_type.clone();
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
    for p in &mut defaults {
        if !p.api_key.is_empty() {
            p.api_key = "••••••••".to_string();
        }
    }
    Ok(defaults)
}

#[tauri::command]
pub async fn llm_save_provider(
    state: State<'_, AppState>,
    account_id: String,
    provider: ProviderWithKey,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let mut config = load_config(&vault, &account_id)?;
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
    let _ = vault.log_structured(
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
