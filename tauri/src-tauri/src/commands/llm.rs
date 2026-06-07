//! LLM configuration commands (§26)
//! Multi-provider model with encrypted API key storage.
//! `llm_test_provider` and `llm_send_message` use reqwest for HTTP calls.

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use solosoul_vault::VaultStore;
use tauri::State;

// ── Data models ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub id: String, pub name: String, pub base_url: String, pub model: String,
    pub is_enabled: bool, pub is_built_in: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderWithKey {
    pub id: String, pub name: String, pub base_url: String, pub model: String,
    pub is_enabled: bool, pub is_built_in: bool, pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiFeatures {
    pub chat: bool, pub smart_fill: bool, pub command_gen: bool, pub natural_language_search: bool,
}
impl Default for AiFeatures {
    fn default() -> Self { Self { chat: false, smart_fill: false, command_gen: false, natural_language_search: false } }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmConfig {
    pub providers: Vec<ProviderConfig>,
    pub active_provider_id: Option<String>,
    pub ai_features_enabled: AiFeatures,
    pub has_accepted_risk: bool,
}

fn default_providers() -> Vec<ProviderWithKey> {
    vec![
        ProviderWithKey { id: "builtin_openai".into(), name: "OpenAI".into(), base_url: "https://api.openai.com/v1".into(), model: "gpt-4o".into(), is_enabled: false, is_built_in: true, api_key: String::new() },
        ProviderWithKey { id: "builtin_anthropic".into(), name: "Anthropic".into(), base_url: "https://api.anthropic.com/v1".into(), model: "claude-3-sonnet-20241022".into(), is_enabled: false, is_built_in: true, api_key: String::new() },
        ProviderWithKey { id: "builtin_ollama".into(), name: "Ollama (Local)".into(), base_url: "http://localhost:11434/v1".into(), model: "llama3.1".into(), is_enabled: false, is_built_in: true, api_key: String::new() },
        ProviderWithKey { id: "builtin_deepseek".into(), name: "DeepSeek".into(), base_url: "https://api.deepseek.com/v1".into(), model: "deepseek-chat".into(), is_enabled: false, is_built_in: true, api_key: String::new() },
        ProviderWithKey { id: "builtin_alibaba".into(), name: "Alibaba Cloud".into(), base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1".into(), model: "qwen-max".into(), is_enabled: false, is_built_in: true, api_key: String::new() },
    ]
}

fn load_config(vault: &VaultStore, account_id: &str) -> Result<LlmConfig, String> {
    match vault.load_profile(account_id) {
        Ok(Some(profile)) => {
            let data: serde_json::Value = serde_json::from_slice(&profile.data).map_err(|e| format!("Parse: {}", e))?;
            if let Some(llm) = data.get("preferences").and_then(|p| p.get("llmConfig")) {
                serde_json::from_value(llm.clone()).map_err(|e| format!("Parse: {}", e))
            } else {
                Ok(LlmConfig { providers: vec![], active_provider_id: None, ai_features_enabled: AiFeatures::default(), has_accepted_risk: false })
            }
        }
        _ => Ok(LlmConfig { providers: vec![], active_provider_id: None, ai_features_enabled: AiFeatures::default(), has_accepted_risk: false }),
    }
}

fn save_config(vault: &VaultStore, account_id: &str, config: &LlmConfig) -> Result<(), String> {
    let mut profile = match vault.load_profile(account_id) {
        Ok(Some(p)) => p, Ok(None) => solosoul_vault::Profile::new_with_id(account_id, account_id, Vec::new()),
        Err(e) => return Err(format!("Load: {}", e)),
    };
    let mut data: serde_json::Value = if profile.data.is_empty() { serde_json::Value::Object(serde_json::Map::new()) }
        else { serde_json::from_slice(&profile.data).map_err(|e| format!("Parse: {}", e))? };
    let prefs = data.as_object_mut().ok_or("Invalid")?.entry("preferences".to_string()).or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    prefs["llmConfig"] = serde_json::to_value(config).map_err(|e| e.to_string())?;
    profile.data = serde_json::to_vec(&data).map_err(|e| e.to_string())?;
    profile.updated_at = chrono::Utc::now(); profile.version += 1; vault.save_profile(&profile)
}

fn load_api_keys(vault: &VaultStore, account_id: &str) -> Result<HashMap<String, String>, String> {
    match vault.load_profile(account_id) {
        Ok(Some(profile)) => {
            let data: serde_json::Value = serde_json::from_slice(&profile.data).map_err(|e| format!("Parse: {}", e))?;
            Ok(data.get("preferences").and_then(|p| p.get("llmApiKeys")).and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default())
        }
        _ => Ok(HashMap::new()),
    }
}

fn save_api_key(vault: &VaultStore, account_id: &str, provider_id: &str, api_key: &str) -> Result<(), String> {
    let mut profile = match vault.load_profile(account_id) {
        Ok(Some(p)) => p, Ok(None) => solosoul_vault::Profile::new_with_id(account_id, account_id, Vec::new()),
        Err(e) => return Err(format!("Load: {}", e)),
    };
    let mut data: serde_json::Value = if profile.data.is_empty() { serde_json::Value::Object(serde_json::Map::new()) }
        else { serde_json::from_slice(&profile.data).map_err(|e| format!("Parse: {}", e))? };
    let prefs = data.as_object_mut().ok_or("Invalid")?.entry("preferences".to_string()).or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    let mut keys: HashMap<String, String> = prefs.get("llmApiKeys").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
    keys.insert(provider_id.to_string(), api_key.to_string());
    prefs["llmApiKeys"] = serde_json::to_value(&keys).map_err(|e| e.to_string())?;
    profile.data = serde_json::to_vec(&data).map_err(|e| e.to_string())?;
    profile.updated_at = chrono::Utc::now(); profile.version += 1; vault.save_profile(&profile)
}


// ── Commands ───────────────────────────────────────────────

#[tauri::command]
pub async fn llm_get_config(state: State<'_, AppState>, account_id: String) -> Result<LlmConfig, String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    load_config(vault, &account_id)
}

#[tauri::command]
pub async fn llm_get_providers(state: State<'_, AppState>, account_id: String) -> Result<Vec<ProviderWithKey>, String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    let config = load_config(vault, &account_id)?;
    let keys = load_api_keys(vault, &account_id)?;
    let mut defaults = default_providers();
    for saved in &config.providers {
        if let Some(d) = defaults.iter_mut().find(|d| d.id == saved.id) {
            d.name = saved.name.clone(); d.base_url = saved.base_url.clone(); d.model = saved.model.clone(); d.is_enabled = saved.is_enabled;
        } else {
            defaults.push(ProviderWithKey {
                id: saved.id.clone(), name: saved.name.clone(), base_url: saved.base_url.clone(),
                model: saved.model.clone(), is_enabled: saved.is_enabled, is_built_in: false,
                api_key: keys.get(&saved.id).cloned().unwrap_or_default(),
            });
        }
    }
    for p in &mut defaults { if !p.api_key.is_empty() { p.api_key = "••••••••".to_string(); } }
    Ok(defaults)
}

#[tauri::command]
pub async fn llm_save_provider(state: State<'_, AppState>, account_id: String, provider: ProviderWithKey) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    let mut config = load_config(vault, &account_id)?;
    let api_key = if provider.is_built_in && provider.api_key == "••••••••" { String::new() } else { provider.api_key.clone() };
    if !api_key.is_empty() { save_api_key(vault, &account_id, &provider.id, &api_key)?; }
    let pc = ProviderConfig { id: provider.id.clone(), name: provider.name, base_url: provider.base_url, model: provider.model, is_enabled: provider.is_enabled, is_built_in: provider.is_built_in };
    if let Some(e) = config.providers.iter_mut().find(|p| p.id == pc.id) { *e = pc; } else { config.providers.push(pc); }
    save_config(vault, &account_id, &config)
}

#[tauri::command]
pub async fn llm_set_active_provider(state: State<'_, AppState>, account_id: String, provider_id: String) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    let mut config = load_config(vault, &account_id)?;
    config.active_provider_id = Some(provider_id);
    save_config(vault, &account_id, &config)
}

#[tauri::command]
pub async fn llm_set_ai_features(state: State<'_, AppState>, account_id: String, features: AiFeatures) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    let mut config = load_config(vault, &account_id)?;
    config.ai_features_enabled = features;
    save_config(vault, &account_id, &config)
}

#[tauri::command]
pub async fn llm_accept_risk(state: State<'_, AppState>, account_id: String) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    let mut config = load_config(vault, &account_id)?;
    config.has_accepted_risk = true;
    save_config(vault, &account_id, &config)?;
    let _ = vault.log_action("llm_risk_accepted", &format!("account={}", account_id));
    Ok(())
}

#[tauri::command]
pub async fn llm_get_api_key(state: State<'_, AppState>, account_id: String, provider_id: String) -> Result<String, String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    load_api_keys(vault, &account_id).map(|k| k.get(&provider_id).cloned().unwrap_or_default())
}

#[tauri::command]
pub async fn llm_delete_provider(state: State<'_, AppState>, account_id: String, provider_id: String) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    let mut config = load_config(vault, &account_id)?;
    config.providers.retain(|p| p.id != provider_id);
    if config.active_provider_id.as_deref() == Some(&provider_id) {
        config.active_provider_id = config.providers.first().map(|p| p.id.clone());
    }
    save_config(vault, &account_id, &config)
}

#[tauri::command]
pub async fn llm_test_provider(base_url: String, api_key: String, model: String) -> Result<String, String> {
    let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(10)).build().map_err(|e| format!("Client: {}", e))?;
    let body = serde_json::json!({"model": model, "messages": [{"role": "user", "content": "Hello"}], "max_tokens": 5, "stream": false});
    let resp = client.post(&format!("{}/chat/completions", base_url.trim_end_matches('/')))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json").json(&body).send().await.map_err(|e| format!("Request: {}", e))?;
    if !resp.status().is_success() { return Err(format!("HTTP {}: {}", resp.status(), resp.text().await.unwrap_or_default())); }
    let result: serde_json::Value = resp.json().await.map_err(|e| format!("Parse: {}", e))?;
    Ok(result["choices"][0]["message"]["content"].as_str().unwrap_or("ok").to_string())
}

#[tauri::command]
pub async fn llm_send_message(base_url: String, api_key: String, model: String, messages: Vec<serde_json::Value>) -> Result<String, String> {
    let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(60)).build().map_err(|e| format!("Client: {}", e))?;
    let body = serde_json::json!({"model": model, "messages": messages, "stream": false});
    let resp = client.post(&format!("{}/chat/completions", base_url.trim_end_matches('/')))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json").json(&body).send().await.map_err(|e| format!("Request: {}", e))?;
    if !resp.status().is_success() { return Err(format!("HTTP {}: {}", resp.status(), resp.text().await.unwrap_or_default())); }
    let result: serde_json::Value = resp.json().await.map_err(|e| format!("Parse: {}", e))?;
    result["choices"][0]["message"]["content"].as_str().map(|s| s.to_string()).ok_or_else(|| "No response".to_string())
}
