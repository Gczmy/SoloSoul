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
}

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
