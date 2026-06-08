//! LLM configuration commands (§26)
//! Multi-provider model with encrypted API key storage.
//! `llm_test_provider` and `llm_send_message` use reqwest for HTTP calls.

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use solosoul_vault::VaultStore;
use tauri::State;

// ── Data models ─────────────────────────────────────────────

/// API protocol type for the provider
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ApiType {
    OpenAI,
    Anthropic,
}

impl Default for ApiType {
    fn default() -> Self { Self::OpenAI }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub id: String, pub name: String, pub base_url: String, pub model: String,
    pub is_enabled: bool, pub is_built_in: bool,
    #[serde(default)]
    pub api_type: ApiType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderWithKey {
    pub id: String, pub name: String, pub base_url: String, pub model: String,
    pub is_enabled: bool, pub is_built_in: bool, pub api_key: String,
    #[serde(default)]
    pub api_type: ApiType,
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
    #[serde(default = "default_true")]
    pub include_system_prompt: bool,
}

fn default_true() -> bool { true }

// ── Conversation data models ────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: String,
    pub name: String,
    pub is_temporary: bool,
    pub messages: Vec<ChatMessage>,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationSummary {
    pub id: String,
    pub name: String,
    pub updated_at: String,
    pub message_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<String>,
}

fn default_providers() -> Vec<ProviderWithKey> {
    vec![
        ProviderWithKey { id: "builtin_openai".into(), name: "OpenAI".into(), base_url: "https://api.openai.com/v1".into(), model: "gpt-4o".into(), is_enabled: false, is_built_in: true, api_key: String::new(), api_type: ApiType::OpenAI },
        ProviderWithKey { id: "builtin_anthropic".into(), name: "Anthropic".into(), base_url: "https://api.anthropic.com/v1".into(), model: "claude-sonnet-4-20250514".into(), is_enabled: false, is_built_in: true, api_key: String::new(), api_type: ApiType::Anthropic },
        ProviderWithKey { id: "builtin_ollama".into(), name: "Ollama (Local)".into(), base_url: "http://localhost:11434/v1".into(), model: "llama3.1".into(), is_enabled: false, is_built_in: true, api_key: String::new(), api_type: ApiType::OpenAI },
        ProviderWithKey { id: "builtin_deepseek".into(), name: "DeepSeek".into(), base_url: "https://api.deepseek.com/v1".into(), model: "deepseek-chat".into(), is_enabled: false, is_built_in: true, api_key: String::new(), api_type: ApiType::OpenAI },
        ProviderWithKey { id: "builtin_alibaba".into(), name: "Alibaba Cloud".into(), base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1".into(), model: "qwen-max".into(), is_enabled: false, is_built_in: true, api_key: String::new(), api_type: ApiType::OpenAI },
    ]
}

fn load_config(vault: &VaultStore, account_id: &str) -> Result<LlmConfig, String> {
    match vault.load_profile(account_id) {
        Ok(Some(profile)) => {
            let data: serde_json::Value = serde_json::from_slice(&profile.data).map_err(|e| format!("Parse: {}", e))?;
            if let Some(llm) = data.get("preferences").and_then(|p| p.get("llmConfig")) {
                serde_json::from_value(llm.clone()).map_err(|e| format!("Parse: {}", e))
            } else {
                Ok(LlmConfig { providers: vec![], active_provider_id: None, ai_features_enabled: AiFeatures::default(), has_accepted_risk: false, include_system_prompt: true })
            }
        }
        _ => Ok(LlmConfig { providers: vec![], active_provider_id: None, ai_features_enabled: AiFeatures::default(), has_accepted_risk: false, include_system_prompt: true }),
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
            d.api_key = keys.get(&saved.id).cloned().unwrap_or_default();
            d.api_type = saved.api_type.clone();
        } else {
            defaults.push(ProviderWithKey {
                id: saved.id.clone(), name: saved.name.clone(), base_url: saved.base_url.clone(),
                model: saved.model.clone(), is_enabled: saved.is_enabled, is_built_in: false,
                api_key: keys.get(&saved.id).cloned().unwrap_or_default(),
                api_type: saved.api_type.clone(),
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
    let pc = ProviderConfig { id: provider.id.clone(), name: provider.name, base_url: provider.base_url, model: provider.model, is_enabled: provider.is_enabled, is_built_in: provider.is_built_in, api_type: provider.api_type };
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
pub async fn llm_set_system_prompt_switch(state: State<'_, AppState>, account_id: String, enabled: bool) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    let mut config = load_config(vault, &account_id)?;
    config.include_system_prompt = enabled;
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
    let _ = vault.log_structured("llm_risk_accepted", "preference", Some(&account_id), None, "user", None);
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

fn is_anthropic(api_type: &ApiType) -> bool {
    matches!(api_type, ApiType::Anthropic)
}

// ── Conversation storage ──────────────────────────────────

fn load_conversations(vault: &VaultStore, account_id: &str) -> Result<Vec<Conversation>, String> {
    match vault.load_profile(account_id) {
        Ok(Some(profile)) => {
            let data: serde_json::Value = serde_json::from_slice(&profile.data).map_err(|e| format!("Parse: {}", e))?;
            Ok(data.get("preferences").and_then(|p| p.get("llmConversations")).and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default())
        }
        _ => Ok(vec![]),
    }
}

fn save_conversations(vault: &VaultStore, account_id: &str, conversations: &[Conversation]) -> Result<(), String> {
    let mut profile = match vault.load_profile(account_id) {
        Ok(Some(p)) => p,
        Ok(None) => solosoul_vault::Profile::new_with_id(account_id, account_id, Vec::new()),
        Err(e) => return Err(format!("Load: {}", e)),
    };
    let mut data: serde_json::Value = if profile.data.is_empty() { serde_json::Value::Object(serde_json::Map::new()) }
        else { serde_json::from_slice(&profile.data).map_err(|e| format!("Parse: {}", e))? };
    let prefs = data.as_object_mut().ok_or("Invalid")?.entry("preferences".to_string()).or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    prefs["llmConversations"] = serde_json::to_value(conversations).map_err(|e| e.to_string())?;
    profile.data = serde_json::to_vec(&data).map_err(|e| e.to_string())?;
    profile.updated_at = chrono::Utc::now();
    profile.version += 1;
    vault.save_profile(&profile)
}

fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

// ── Conversation IPC commands ─────────────────────────────

#[tauri::command]
pub async fn llm_list_conversations(state: State<'_, AppState>, account_id: String) -> Result<Vec<ConversationSummary>, String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    let convs = load_conversations(vault, &account_id)?;
    let mut summaries: Vec<ConversationSummary> = convs.into_iter()
        .filter(|c| !c.is_temporary && c.deleted_at.is_none())
        .map(|c| ConversationSummary { id: c.id, name: c.name, updated_at: c.updated_at, message_count: c.messages.len(), deleted_at: None })
        .collect();
    summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(summaries)
}

#[tauri::command]
pub async fn llm_list_trash(state: State<'_, AppState>, account_id: String) -> Result<Vec<ConversationSummary>, String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    let convs = load_conversations(vault, &account_id)?;
    let mut summaries: Vec<ConversationSummary> = convs.into_iter()
        .filter(|c| c.deleted_at.is_some())
        .map(|c| ConversationSummary {
            id: c.id, name: c.name, updated_at: c.updated_at, message_count: c.messages.len(),
            deleted_at: c.deleted_at.clone(),
        })
        .collect();
    summaries.sort_by(|a, b| b.deleted_at.as_deref().unwrap_or(&a.updated_at).cmp(&a.deleted_at.as_deref().unwrap_or(&b.updated_at)));
    Ok(summaries)
}

#[tauri::command]
pub async fn llm_get_conversation(state: State<'_, AppState>, account_id: String, conversation_id: String) -> Result<Conversation, String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    let convs = load_conversations(vault, &account_id)?;
    convs.into_iter().find(|c| c.id == conversation_id).ok_or_else(|| "Not found".to_string())
}

#[tauri::command]
pub async fn llm_save_conversation(state: State<'_, AppState>, account_id: String, conversation: Conversation) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    let mut convs = load_conversations(vault, &account_id)?;
    let mut c = conversation;
    c.is_temporary = false;
    if let Some(existing) = convs.iter_mut().find(|e| e.id == c.id) {
        *existing = c;
    } else {
        convs.push(c);
    }
    save_conversations(vault, &account_id, &convs)
}

#[tauri::command]
pub async fn llm_delete_conversation(state: State<'_, AppState>, account_id: String, conversation_id: String) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    let mut convs = load_conversations(vault, &account_id)?;
    convs.retain(|c| c.id != conversation_id);
    save_conversations(vault, &account_id, &convs)
}

#[tauri::command]
pub async fn llm_soft_delete_conversation(state: State<'_, AppState>, account_id: String, conversation_id: String) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    let mut convs = load_conversations(vault, &account_id)?;
    if let Some(c) = convs.iter_mut().find(|c| c.id == conversation_id) {
        c.deleted_at = Some(now_iso());
    }
    save_conversations(vault, &account_id, &convs)
}

#[tauri::command]
pub async fn llm_restore_conversation(state: State<'_, AppState>, account_id: String, conversation_id: String) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    let mut convs = load_conversations(vault, &account_id)?;
    if let Some(c) = convs.iter_mut().find(|c| c.id == conversation_id) {
        c.deleted_at = None;
    }
    save_conversations(vault, &account_id, &convs)
}

#[tauri::command]
pub async fn llm_permanent_delete(state: State<'_, AppState>, account_id: String, conversation_id: String) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    let mut convs = load_conversations(vault, &account_id)?;
    convs.retain(|c| c.id != conversation_id);
    save_conversations(vault, &account_id, &convs)
}

#[tauri::command]
pub async fn llm_rename_conversation(state: State<'_, AppState>, account_id: String, conversation_id: String, name: String) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    let mut convs = load_conversations(vault, &account_id)?;
    if let Some(c) = convs.iter_mut().find(|c| c.id == conversation_id) {
        c.name = name;
        c.updated_at = now_iso();
    }
    save_conversations(vault, &account_id, &convs)
}

#[tauri::command]
pub async fn llm_check_connection(base_url: String, api_key: String, model: String, api_type: ApiType) -> Result<bool, String> {
    // Lightweight health check using test-provider pattern with very short timeout
    let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(5)).build().map_err(|_| "Client error")?;
    let (url, body) = if is_anthropic(&api_type) {
        (format!("{}/messages", base_url.trim_end_matches('/')),
         serde_json::json!({"model": model, "max_tokens": 1, "messages": [{"role": "user", "content": "Hi"}]}))
    } else {
        (format!("{}/chat/completions", base_url.trim_end_matches('/')),
         serde_json::json!({"model": model, "messages": [{"role": "user", "content": "Hi"}], "max_tokens": 1}))
    };
    let mut req = client.post(&url).header("Content-Type", "application/json").json(&body);
    if is_anthropic(&api_type) {
        req = req.header("x-api-key", &api_key).header("anthropic-version", "2023-06-01");
    } else {
        req = req.header("Authorization", format!("Bearer {}", api_key));
    }
    match req.send().await {
        Ok(r) => Ok(r.status().is_success()),
        Err(_) => Ok(false),
    }
}

#[tauri::command]
pub async fn llm_test_provider(base_url: String, api_key: String, model: String, api_type: ApiType) -> Result<String, String> {
    let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(10)).build().map_err(|e| format!("Client: {}", e))?;

    let (url, body, auth_header, auth_value): (String, serde_json::Value, &str, String) = if is_anthropic(&api_type) {
        let u = format!("{}/messages", base_url.trim_end_matches('/'));
        let b = serde_json::json!({"model": model, "max_tokens": 10, "messages": [{"role": "user", "content": "Hello"}]});
        (u, b, "x-api-key", api_key)
    } else {
        let u = format!("{}/chat/completions", base_url.trim_end_matches('/'));
        let b = serde_json::json!({"model": model, "messages": [{"role": "user", "content": "Hello"}], "max_tokens": 5, "stream": false});
        (u, b, "Authorization", format!("Bearer {}", api_key))
    };

    let mut req = client.post(&url)
        .header("Content-Type", "application/json")
        .json(&body);
    if is_anthropic(&api_type) {
        req = req.header(auth_header, &auth_value).header("anthropic-version", "2023-06-01");
    } else {
        req = req.header(auth_header, &auth_value);
    }

    let resp = req.send().await.map_err(|e| format!("Request to {} failed: {}", url, e))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let snippet = if text.is_empty() { "(empty body)".to_string() } else { text.chars().take(300).collect() };
        return Err(format!("HTTP {} {} — {}", status.as_u16(), url, snippet));
    }
    let result: serde_json::Value = resp.json().await.map_err(|e| format!("Parse response from {}: {}", url, e))?;

    if is_anthropic(&api_type) {
        let text = result["content"].as_array().and_then(|arr| {
            arr.iter().find(|c| c.get("type").and_then(|t| t.as_str()) == Some("text") || c.get("type").is_none())
                .and_then(|c| c.get("text").and_then(|v| v.as_str()))
        }).unwrap_or("ok");
        Ok(text.to_string())
    } else {
        Ok(result["choices"][0]["message"]["content"].as_str().unwrap_or("ok").to_string())
    }
}

#[tauri::command]
pub async fn llm_send_message(base_url: String, api_key: String, model: String, api_type: ApiType, messages: Vec<serde_json::Value>) -> Result<String, String> {
    let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(30)).build().map_err(|e| format!("Client: {}", e))?;

    if is_anthropic(&api_type) {
        // Anthropic Messages API format
        // Separate system message from chat messages
        let system = messages.iter().find(|m| m.get("role").and_then(|r| r.as_str()) == Some("system"))
            .and_then(|m| m.get("content").and_then(|c| c.as_str())).map(|s| s.to_string());
        let chat_msgs: Vec<serde_json::Value> = messages.into_iter().filter(|m| m.get("role").and_then(|r| r.as_str()) != Some("system")).collect();

        let mut body = serde_json::json!({
            "model": model,
            "max_tokens": 4096,
            "messages": chat_msgs,
        });
        if let Some(sys) = &system {
            body["system"] = serde_json::Value::String(sys.clone());
        }

        let url = format!("{}/messages", base_url.trim_end_matches('/'));
        let resp = client.post(&url)
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body).send().await.map_err(|e| format!("Request: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("HTTP {}: {}", resp.status(), resp.text().await.unwrap_or_default()));
        }
        let result: serde_json::Value = resp.json().await.map_err(|e| format!("Parse: {}", e))?;
        // Anthropic thinking models return content blocks with types:
        // [{"type":"thinking","thinking":"..."}, {"type":"text","text":"..."}]
        let text = result["content"].as_array().and_then(|arr| {
            arr.iter().find(|c| c.get("type").and_then(|t| t.as_str()) == Some("text") || c.get("type").is_none())
                .and_then(|c| c.get("text").and_then(|v| v.as_str()))
        });
        text.map(|s| s.to_string()).ok_or_else(|| {
            let raw = result.to_string();
            format!("No response — raw: {}", &raw[..300.min(raw.len())])
        })
    } else {
        // OpenAI-compatible API format
        let body = serde_json::json!({"model": model, "messages": messages, "stream": false});
        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
        let resp = client.post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json").json(&body).send().await.map_err(|e| format!("Request: {}", e))?;
        if !resp.status().is_success() { return Err(format!("HTTP {}: {}", resp.status(), resp.text().await.unwrap_or_default())); }
        let result: serde_json::Value = resp.json().await.map_err(|e| format!("Parse: {}", e))?;
        let text = result["choices"][0]["message"]["content"].as_str();
        text.map(|s| s.to_string()).ok_or_else(|| {
            let raw = result.to_string();
            format!("No response — raw: {}", &raw[..300.min(raw.len())])
        })
    }
}

// =============================================================================
// Help Guide Retrieval (§7)
// =============================================================================

use std::sync::Mutex;
use std::path::PathBuf;
use once_cell::sync::Lazy;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GuideIndexEntry {
    id: String,
    title: String,
    keywords: Vec<String>,
    files: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GuideIndex {
    guides: Vec<GuideIndexEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuideContent {
    pub id: String,
    pub title: String,
    pub content: String,
}

/// 资源文件路径解析：开发模式从 src-tauri/resources/ 读取，生产模式从 app bundle 读取
fn resource_path(rel: &str) -> PathBuf {
    if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources").join(rel)
    } else {
        std::env::current_exe()
            .ok()
            .and_then(|exe| {
                let parent = exe.parent()?;
                // macOS app bundle: SoloSoul.app/Contents/MacOS/SoloSoul → ../Resources
                let resources = parent.join("../Resources");
                Some(resources.join(rel))
            })
            .unwrap_or_else(|| PathBuf::from(rel))
    }
}

/// 缓存的指南索引
static GUIDE_INDEX_CACHE: Lazy<Mutex<Option<GuideIndex>>> = Lazy::new(|| Mutex::new(None));

fn load_guide_index() -> Result<GuideIndex, String> {
    {
        let cache: std::sync::MutexGuard<Option<GuideIndex>> = GUIDE_INDEX_CACHE.lock().map_err(|e: std::sync::PoisonError<std::sync::MutexGuard<Option<GuideIndex>>>| e.to_string())?;
        if let Some(ref idx) = *cache {
            return Ok(idx.clone());
        }
    }
    let path = resource_path("docs/guides/index.json");
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read guide index at {:?}: {}", path, e))?;
    let index: GuideIndex = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse guide index: {}", e))?;
    {
        let mut cache: std::sync::MutexGuard<Option<GuideIndex>> = GUIDE_INDEX_CACHE.lock().map_err(|e: std::sync::PoisonError<std::sync::MutexGuard<Option<GuideIndex>>>| e.to_string())?;
        *cache = Some(index.clone());
    }
    Ok(index)
}

/// 分词 + 停用词过滤（简化版）
fn tokenize_query(query: &str) -> Vec<String> {
    let lowered = query.to_lowercase();
    // 中文按字符分，英文按空格分
    let tokens: Vec<String> = lowered
        .split_whitespace()
        .flat_map(|s| {
            if s.chars().any(|c| c as u32 >= 0x4E00 && c as u32 <= 0x9FFF) {
                // 含中文字符：每个中文字符单独作为一个 token，英文部分整体保留
                s.chars()
                    .filter(|c| !is_stop_char(*c))
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
            } else {
                vec![s.to_string()]
            }
        })
        .filter(|t| !t.is_empty() && !is_stop_word(t))
        .collect();
    tokens
}

fn is_stop_char(c: char) -> bool {
    matches!(c, '。' | '，' | '！' | '？' | '、' | '；' | ':' | ';' | ',' | '.' | '!' | '?')
}

fn is_stop_word(word: &str) -> bool {
    let stops: &[&str] = &[
        "的", "了", "是", "在", "我", "有", "和", "就", "不", "人", "都", "一", "一个", "上", "也", "很", "到", "说", "要", "去", "你", "会", "着", "没有", "看", "好", "自己", "这",
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had", "do", "does", "did", "will", "would", "could", "should", "may", "might", "must", "shall", "can", "need", "dare", "ought", "used", "to", "of", "in", "for", "on", "with", "at", "by", "from", "as", "into", "through", "during", "before", "after", "above", "below", "between", "under", "and", "but", "or", "yet", "so", "if", "because", "although", "though", "while", "where", "when", "that", "which", "who", "whom", "whose", "what", "whatever", "whoever", "whomever", "this", "these", "those", "i", "me", "my", "myself", "we", "our", "ours", "ourselves", "you", "your", "yours", "yourself", "yourselves", "he", "him", "his", "himself", "she", "her", "hers", "herself", "it", "its", "itself", "they", "them", "their", "theirs", "themselves",
    ];
    stops.contains(&word)
}

fn resolve_language(files: &HashMap<String, String>, requested: &str) -> String {
    if files.contains_key(requested) {
        return requested.to_string();
    }
    // 简化为 'zh' 或 'en'
    let short = if requested.starts_with("zh") { "zh" } else { "en" };
    if files.contains_key(short) {
        return short.to_string();
    }
    if files.contains_key("en") {
        return "en".to_string();
    }
    files.keys().next().cloned().unwrap_or_else(|| "en".to_string())
}

fn load_guide_content(entry: &GuideIndexEntry, language: &str) -> Result<GuideContent, String> {
    let lang = resolve_language(&entry.files, language);
    let rel_path = format!("docs/guides/{}", entry.files.get(&lang).ok_or("No file")?);
    let path = resource_path(&rel_path);
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read guide {:?}: {}", path, e))?;

    // 截断至 800 字符，按段落边界
    let truncated = if content.len() > 800 {
        let mut cut = &content[..800];
        if let Some(pos) = cut.rfind("\n\n") {
            cut = &content[..pos];
        } else if let Some(pos) = cut.rfind('\n') {
            cut = &content[..pos];
        }
        format!("{}\n\n（文档内容过长，已截断）", cut)
    } else {
        content
    };

    Ok(GuideContent {
        id: entry.id.clone(),
        title: entry.title.clone(),
        content: truncated,
    })
}

fn find_relevant_guides_internal(query: &str, language: &str) -> Result<Vec<GuideContent>, String> {
    let index = load_guide_index()?;
    let tokens = tokenize_query(query);
    if tokens.is_empty() {
        return Ok(vec![]);
    }

    let threshold = if tokens.len() >= 2 { 2 } else { 1 };

    let mut scored: Vec<(GuideIndexEntry, i32)> = vec![];
    for guide in &index.guides {
        let mut score = 0;
        for token in &tokens {
            if guide.keywords.iter().any(|k| k.to_lowercase().contains(token)) {
                score += 1;
            }
            if guide.title.to_lowercase().contains(token) {
                score += 3;
            }
        }
        if score >= threshold {
            scored.push((guide.clone(), score));
        }
    }

    scored.sort_by(|a, b| b.1.cmp(&a.1));

    let mut results = vec![];
    for (entry, _) in scored.into_iter().take(1) {
        match load_guide_content(&entry, language) {
            Ok(g) => results.push(g),
            Err(e) => eprintln!("Guide load error: {}", e),
        }
    }

    Ok(results)
}

#[tauri::command]
pub async fn llm_find_guides(query: String, language: String) -> Result<Vec<GuideContent>, String> {
    find_relevant_guides_internal(&query, &language)
}

// =============================================================================
// Usage Statistics (§10)
// =============================================================================

use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmUsageStats {
    pub usage_count: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub per_model_stats: Vec<ModelUsage>,
    pub daily_stats: Vec<DailyUsage>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsage {
    pub model: String,
    pub count: u64,
    pub tokens: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyUsage {
    pub date: String, // YYYY-MM-DD
    pub count: u64,
    pub tokens: u64,
}

/// 内存中的使用统计（按账户隔离）
static STATS_MAP: Lazy<Arc<RwLock<HashMap<String, LlmUsageStats>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// 估算 Token 数（保守策略：所有字符按 1 token）
pub fn estimate_tokens(text: &str) -> u64 {
    text.chars().count() as u64
}

/// 记录一次 AI 调用
pub async fn record_usage(account_id: &str, model: &str, prompt: &str, completion: &str) {
    let prompt_tokens = estimate_tokens(prompt);
    let completion_tokens = estimate_tokens(completion);
    let total = prompt_tokens + completion_tokens;

    let mut map: tokio::sync::RwLockWriteGuard<'_, HashMap<String, LlmUsageStats>> = STATS_MAP.write().await;
    let stats: &mut LlmUsageStats = map.entry(account_id.to_string()).or_default();

    stats.usage_count += 1;
    stats.prompt_tokens += prompt_tokens;
    stats.completion_tokens += completion_tokens;
    stats.total_tokens += total;

    // 更新按模型统计
    if let Some(m) = stats.per_model_stats.iter_mut().find(|m| m.model == model) {
        m.count += 1;
        m.tokens += total;
    } else {
        stats.per_model_stats.push(ModelUsage {
            model: model.to_string(),
            count: 1,
            tokens: total,
        });
    }

    // 更新每日统计
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    if let Some(d) = stats.daily_stats.iter_mut().find(|d| d.date == today) {
        d.count += 1;
        d.tokens += total;
    } else {
        stats.daily_stats.push(DailyUsage {
            date: today,
            count: 1,
            tokens: total,
        });
    }
}

fn save_stats_to_vault(vault: &VaultStore, account_id: &str, stats: &LlmUsageStats) -> Result<(), String> {
    let mut profile = match vault.load_profile(account_id) {
        Ok(Some(p)) => p,
        Ok(None) => solosoul_vault::Profile::new_with_id(account_id, account_id, Vec::new()),
        Err(e) => return Err(format!("Load: {}", e)),
    };
    let mut data: serde_json::Value = if profile.data.is_empty() {
        serde_json::Value::Object(serde_json::Map::new())
    } else {
        serde_json::from_slice(&profile.data).map_err(|e| format!("Parse: {}", e))?
    };
    let prefs = data.as_object_mut()
        .ok_or("Invalid")?
        .entry("preferences".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    prefs["llmUsageStats"] = serde_json::to_value(stats).map_err(|e| e.to_string())?;
    profile.data = serde_json::to_vec(&data).map_err(|e| e.to_string())?;
    profile.updated_at = chrono::Utc::now();
    profile.version += 1;
    vault.save_profile(&profile)
}

fn load_stats_from_vault(vault: &VaultStore, account_id: &str) -> Result<LlmUsageStats, String> {
    match vault.load_profile(account_id) {
        Ok(Some(profile)) => {
            let data: serde_json::Value = serde_json::from_slice(&profile.data)
                .map_err(|e| format!("Parse: {}", e))?;
            Ok(data.get("preferences")
                .and_then(|p| p.get("llmUsageStats"))
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default())
        }
        _ => Ok(LlmUsageStats::default()),
    }
}

#[tauri::command]
pub async fn llm_get_stats(state: State<'_, AppState>, account_id: String) -> Result<LlmUsageStats, String> {
    // 1. 尝试从内存读取
    {
        let map: tokio::sync::RwLockReadGuard<'_, HashMap<String, LlmUsageStats>> = STATS_MAP.read().await;
        if let Some(stats) = map.get(&account_id) {
            return Ok(stats.clone());
        }
    }
    // 2. 内存未命中，从 Vault 加载（严格限定作用域，确保 RwLockGuard 在 await 前 drop）
    let stats: LlmUsageStats = {
        let svc = state.vault_service.read().await;
        let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
        let vault = vg.as_ref().ok_or("Vault not unlocked")?;
        load_stats_from_vault(vault, &account_id)?
    };
    // 3. 加载到内存
    {
        let mut map: tokio::sync::RwLockWriteGuard<'_, HashMap<String, LlmUsageStats>> = STATS_MAP.write().await;
        map.insert(account_id.clone(), stats.clone());
    }
    Ok(stats)
}

#[tauri::command]
pub async fn llm_reset_stats(state: State<'_, AppState>, account_id: String) -> Result<(), String> {
    {
        let mut map: tokio::sync::RwLockWriteGuard<'_, HashMap<String, LlmUsageStats>> = STATS_MAP.write().await;
        map.remove(&account_id);
    }
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    save_stats_to_vault(vault, &account_id, &LlmUsageStats::default())
}

/// 将指定账户的统计持久化到 Vault（debounce 保存由调用方管理）
pub async fn persist_stats(account_id: &str, vault: &VaultStore) -> Result<(), String> {
    let stats: LlmUsageStats = {
        let map: tokio::sync::RwLockReadGuard<'_, HashMap<String, LlmUsageStats>> = STATS_MAP.read().await;
        map.get(account_id).cloned().unwrap_or_default()
    };
    save_stats_to_vault(vault, account_id, &stats)
}

// =============================================================================
// Streaming Response (§5.3)
// =============================================================================

use tauri::Emitter;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmStreamPayload {
    pub conversation_id: String,
    pub chunk: String,
    pub is_done: bool,
    pub error: Option<String>,
}

/// 打字机效果：将完整文本逐字推送到前端（降级用）
async fn emit_typing_effect(
    app: &tauri::AppHandle,
    conversation_id: &str,
    full_text: &str,
) {
    let graphemes: Vec<String> = full_text.graphemes(true).map(|g| g.to_string()).collect();
    let total = graphemes.len();
    let max_typing_ms = 3000u64;
    let delay_ms = if total <= 50 { 2u64 } else { 4u64 };

    for (i, g) in graphemes.iter().enumerate() {
        let elapsed = (i as u64) * delay_ms;
        if elapsed >= max_typing_ms {
            let remaining: String = graphemes[i..].concat();
            let _ = app.emit("llm-stream-chunk", LlmStreamPayload {
                conversation_id: conversation_id.to_string(),
                chunk: remaining,
                is_done: true,
                error: None,
            });
            return;
        }
        let _ = app.emit("llm-stream-chunk", LlmStreamPayload {
            conversation_id: conversation_id.to_string(),
            chunk: g.clone(),
            is_done: false,
            error: None,
        });
        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
    }

    if (total as u64) * delay_ms < max_typing_ms {
        let _ = app.emit("llm-stream-chunk", LlmStreamPayload {
            conversation_id: conversation_id.to_string(),
            chunk: String::new(),
            is_done: true,
            error: None,
        });
    }
}

/// 发送聊天请求并流式推送结果（Phase 2.3：SSE 流式 + 打字机降级）
async fn send_chat_stream(
    app: tauri::AppHandle,
    conversation_id: String,
    base_url: String,
    api_key: String,
    model: String,
    api_type: ApiType,
    messages: Vec<serde_json::Value>,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Client: {}", e))?;

    let (url, body, auth_header, auth_value): (String, serde_json::Value, &str, String) = if is_anthropic(&api_type) {
        let system = messages.iter()
            .find(|m| m.get("role").and_then(|r| r.as_str()) == Some("system"))
            .and_then(|m| m.get("content").and_then(|c| c.as_str()))
            .map(|s| s.to_string());
        let chat_msgs: Vec<serde_json::Value> = messages.into_iter()
            .filter(|m| m.get("role").and_then(|r| r.as_str()) != Some("system"))
            .collect();
        let mut b = serde_json::json!({
            "model": model,
            "max_tokens": 4096,
            "messages": chat_msgs,
            "stream": true,
        });
        if let Some(sys) = &system {
            b["system"] = serde_json::Value::String(sys.clone());
        }
        (format!("{}/messages", base_url.trim_end_matches('/')), b, "x-api-key", api_key)
    } else {
        let b = serde_json::json!({"model": model, "messages": messages, "stream": true});
        (format!("{}/chat/completions", base_url.trim_end_matches('/')), b, "Authorization", format!("Bearer {}", api_key))
    };

    let resp = client.post(&url)
        .header(auth_header, &auth_value)
        .header("Content-Type", "application/json")
        .json(&body)
        .send().await
        .map_err(|e| format!("Request: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        let _ = app.emit("llm-stream-chunk", LlmStreamPayload {
            conversation_id: conversation_id.clone(),
            chunk: String::new(),
            is_done: false,
            error: Some(format!("HTTP {}: {}", status, err_text)),
        });
        return Err(format!("HTTP {}: {}", status, err_text));
    }

    // 检查 Content-Type，判断是否为 SSE
    let content_type = resp.headers()
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

                // 只处理 data: 行（忽略 event: 等）
                if line.starts_with("data: ") {
                    let data = &line[6..];

                    // OpenAI 风格结束标记
                    if data == "[DONE]" {
                        let _ = app.emit("llm-stream-chunk", LlmStreamPayload {
                            conversation_id: conversation_id.clone(),
                            chunk: String::new(),
                            is_done: true,
                            error: None,
                        });
                        return Ok(full_text);
                    }

                    // 尝试解析 JSON
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                        let delta_text = if is_anthropic(&api_type) {
                            // Anthropic: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"..."}}
                            json.get("delta")
                                .and_then(|d| d.get("text"))
                                .and_then(|t| t.as_str())
                        } else {
                            // OpenAI: {"choices":[{"delta":{"content":"..."},"index":0}]}
                            json.get("choices")
                                .and_then(|c| c.get(0))
                                .and_then(|choice| choice.get("delta"))
                                .and_then(|delta| delta.get("content"))
                                .and_then(|c| c.as_str())
                        };

                        if let Some(text) = delta_text {
                            if !text.is_empty() {
                                full_text.push_str(text);
                                let _ = app.emit("llm-stream-chunk", LlmStreamPayload {
                                    conversation_id: conversation_id.clone(),
                                    chunk: text.to_string(),
                                    is_done: false,
                                    error: None,
                                });
                            }
                        }
                    }
                }
            }
        }

        // 处理缓冲区中剩余的内容
        let remaining = buffer.trim();
        if remaining.starts_with("data: ") {
            let data = &remaining[6..];
            if data != "[DONE]" {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                    let delta_text = if is_anthropic(&api_type) {
                        json.get("delta").and_then(|d| d.get("text")).and_then(|t| t.as_str())
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
                            let _ = app.emit("llm-stream-chunk", LlmStreamPayload {
                                conversation_id: conversation_id.clone(),
                                chunk: text.to_string(),
                                is_done: false,
                                error: None,
                            });
                        }
                    }
                }
            }
        }

        // 流正常结束
        let _ = app.emit("llm-stream-chunk", LlmStreamPayload {
            conversation_id: conversation_id.clone(),
            chunk: String::new(),
            is_done: true,
            error: None,
        });
        Ok(full_text)
    } else {
        // ===================== 非 SSE：完整获取 + 打字机效果 =====================
        let result: serde_json::Value = resp.json().await.map_err(|e| format!("Parse: {}", e))?;

        let full_text = if is_anthropic(&api_type) {
            result["content"].as_array().and_then(|arr| {
                arr.iter()
                    .find(|c| c.get("type").and_then(|t| t.as_str()) == Some("text") || c.get("type").is_none())
                    .and_then(|c| c.get("text").and_then(|v| v.as_str()))
            }).unwrap_or("").to_string()
        } else {
            result["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string()
        };

        emit_typing_effect(&app, &conversation_id, &full_text).await;
        Ok(full_text)
    }
}

#[tauri::command]
pub async fn llm_send_message_stream(
    app: tauri::AppHandle,
    _state: State<'_, AppState>,
    account_id: String,
    conversation_id: String,
    base_url: String,
    api_key: String,
    model: String,
    api_type: ApiType,
    messages: Vec<serde_json::Value>,
) -> Result<(), String> {
    let prompt_text: String = messages.iter()
        .filter_map(|m| m.get("content").and_then(|c| c.as_str()).map(|s| s.to_string()))
        .collect::<Vec<_>>()
        .join("\n");

    let full_text = send_chat_stream(
        app, conversation_id.clone(), base_url, api_key, model.clone(), api_type, messages,
    ).await?;

    let _ = record_usage(&account_id, &model, &prompt_text, &full_text).await;
    Ok(())
}

// =============================================================================
// Unified Chat Command (§28 Phase 2.2)
// =============================================================================

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
        let svc = state.vault_service.read().await;
        let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
        let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

        let config = load_config(vault, &request.account_id)?;
        let active_id = config.active_provider_id.ok_or("No active provider")?;

        let providers = load_providers_with_keys(vault, &request.account_id)?;
        let active = providers.into_iter()
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

    // 3a. 系统提示词（重新获取 vault）
    if request.include_system_prompt {
        let system_prompt = {
            let svc = state.vault_service.read().await;
            let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
            let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
            crate::services::llm_context::build_context(
                &request.account_id,
                vault,
                stats.usage_count,
                stats.prompt_tokens,
                stats.completion_tokens,
                stats.total_tokens,
                &request.language,
            )?
        };
        if !system_prompt.is_empty() {
            messages.push(serde_json::json!({"role": "system", "content": system_prompt}));
        }
    }

    // 3b. 帮助文档（不需要 vault）
    if request.include_help_doc {
        let guides = find_relevant_guides_internal(&request.prompt, &request.language)?;
        if let Some(guide) = guides.first() {
            let doc_content = format!(
                "---\n以下是与用户问题相关的功能使用文档，请参考这些信息回答用户问题。\n\n【文档：{}】\n{}\n【文档结束】\n---",
                guide.title, guide.content
            );
            messages.push(serde_json::json!({"role": "system", "content": doc_content}));
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
    let prompt_text: String = messages.iter()
        .filter_map(|m| m.get("content").and_then(|c| c.as_str()).map(|s| s.to_string()))
        .collect::<Vec<_>>()
        .join("\n");

    // 5. 发送请求（复用 send_chat_stream，Phase 2.3 将替换为 SSE）
    let full_text = send_chat_stream(
        app,
        request.conversation_id.clone(),
        base_url,
        api_key,
        model.clone(),
        api_type,
        messages,
    ).await?;

    // 6. 记录统计
    let _ = record_usage(&request.account_id, &model, &prompt_text, &full_text).await;

    Ok(())
}

// Helper: load providers with decrypted keys (internal reuse)
fn load_providers_with_keys(vault: &VaultStore, account_id: &str) -> Result<Vec<ProviderWithKey>, String> {
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
            });
        }
    }
    Ok(defaults)
}
