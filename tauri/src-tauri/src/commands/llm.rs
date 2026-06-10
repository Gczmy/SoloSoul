//! LLM configuration commands (§26)
//! Multi-provider model with encrypted API key storage.
//! `llm_test_provider` and `llm_send_message` use reqwest for HTTP calls.

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use solosoul_vault::VaultStore;
use std::collections::HashMap;
use tauri::{Manager, State};

// ── Data models ─────────────────────────────────────────────

/// API protocol type for the provider
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ApiType {
    #[default]
    OpenAI,
    Anthropic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub is_enabled: bool,
    pub is_built_in: bool,
    #[serde(default)]
    pub api_type: ApiType,
    #[serde(default)]
    pub embedding_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderWithKey {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub is_enabled: bool,
    pub is_built_in: bool,
    pub api_key: String,
    #[serde(default)]
    pub api_type: ApiType,
    #[serde(default)]
    pub embedding_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AiFeatures {
    pub chat: bool,
    pub smart_fill: bool,
    pub command_gen: bool,
    pub natural_language_search: bool,
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
    #[serde(default)]
    pub use_local_embedding: bool,
    #[serde(default)]
    pub local_embed_model_id: Option<String>,
}

fn default_true() -> bool {
    true
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
        ProviderWithKey {
            id: "builtin_openai".into(),
            name: "OpenAI".into(),
            base_url: "https://api.openai.com/v1".into(),
            model: "gpt-4o".into(),
            is_enabled: false,
            is_built_in: true,
            api_key: String::new(),
            api_type: ApiType::OpenAI,
            embedding_model: Some("text-embedding-3-small".into()),
        },
        ProviderWithKey {
            id: "builtin_anthropic".into(),
            name: "Anthropic".into(),
            base_url: "https://api.anthropic.com/v1".into(),
            model: "claude-sonnet-4-20250514".into(),
            is_enabled: false,
            is_built_in: true,
            api_key: String::new(),
            api_type: ApiType::Anthropic,
            embedding_model: None,
        },
        ProviderWithKey {
            id: "builtin_ollama".into(),
            name: "Ollama (Local)".into(),
            base_url: "http://localhost:11434/v1".into(),
            model: "llama3.1".into(),
            is_enabled: false,
            is_built_in: true,
            api_key: String::new(),
            api_type: ApiType::OpenAI,
            embedding_model: Some("nomic-embed-text".into()),
        },
        ProviderWithKey {
            id: "builtin_deepseek".into(),
            name: "DeepSeek".into(),
            base_url: "https://api.deepseek.com/v1".into(),
            model: "deepseek-chat".into(),
            is_enabled: false,
            is_built_in: true,
            api_key: String::new(),
            api_type: ApiType::OpenAI,
            embedding_model: Some("text-embedding".into()),
        },
        ProviderWithKey {
            id: "builtin_alibaba".into(),
            name: "Alibaba Cloud".into(),
            base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1".into(),
            model: "qwen-max".into(),
            is_enabled: false,
            is_built_in: true,
            api_key: String::new(),
            api_type: ApiType::OpenAI,
            embedding_model: Some("text-embedding-v3".into()),
        },
    ]
}

fn load_config(vault: &VaultStore, account_id: &str) -> Result<LlmConfig, String> {
    match vault.load_profile(account_id) {
        Ok(Some(profile)) => {
            let data: serde_json::Value =
                serde_json::from_slice(&profile.data).map_err(|e| format!("Parse: {}", e))?;
            if let Some(llm) = data.get("preferences").and_then(|p| p.get("llmConfig")) {
                serde_json::from_value(llm.clone()).map_err(|e| format!("Parse: {}", e))
            } else {
                Ok(LlmConfig {
                    providers: vec![],
                    active_provider_id: None,
                    ai_features_enabled: AiFeatures::default(),
                    has_accepted_risk: false,
                    include_system_prompt: true,
                    use_local_embedding: false,
                    local_embed_model_id: None,
                })
            }
        }
        _ => Ok(LlmConfig {
            providers: vec![],
            active_provider_id: None,
            ai_features_enabled: AiFeatures::default(),
            has_accepted_risk: false,
            include_system_prompt: true,
            use_local_embedding: false,
            local_embed_model_id: None,
        }),
    }
}

fn save_config(vault: &VaultStore, account_id: &str, config: &LlmConfig) -> Result<(), String> {
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
    let prefs = data
        .as_object_mut()
        .ok_or("Invalid")?
        .entry("preferences".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    prefs["llmConfig"] = serde_json::to_value(config).map_err(|e| e.to_string())?;
    profile.data = serde_json::to_vec(&data).map_err(|e| e.to_string())?;
    profile.updated_at = chrono::Utc::now();
    profile.version += 1;
    vault.save_profile(&profile)
}

fn load_api_keys(vault: &VaultStore, account_id: &str) -> Result<HashMap<String, String>, String> {
    match vault.load_profile(account_id) {
        Ok(Some(profile)) => {
            let data: serde_json::Value =
                serde_json::from_slice(&profile.data).map_err(|e| format!("Parse: {}", e))?;
            Ok(data
                .get("preferences")
                .and_then(|p| p.get("llmApiKeys"))
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default())
        }
        _ => Ok(HashMap::new()),
    }
}

fn save_api_key(
    vault: &VaultStore,
    account_id: &str,
    provider_id: &str,
    api_key: &str,
) -> Result<(), String> {
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
    let prefs = data
        .as_object_mut()
        .ok_or("Invalid")?
        .entry("preferences".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    let mut keys: HashMap<String, String> = prefs
        .get("llmApiKeys")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    keys.insert(provider_id.to_string(), api_key.to_string());
    prefs["llmApiKeys"] = serde_json::to_value(&keys).map_err(|e| e.to_string())?;
    profile.data = serde_json::to_vec(&data).map_err(|e| e.to_string())?;
    profile.updated_at = chrono::Utc::now();
    profile.version += 1;
    vault.save_profile(&profile)
}

// ── Commands ───────────────────────────────────────────────

#[tauri::command]
pub async fn llm_get_config(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<LlmConfig, String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    load_config(vault, &account_id)
}

#[tauri::command]
pub async fn llm_get_providers(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<Vec<ProviderWithKey>, String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    let config = load_config(vault, &account_id)?;
    let keys = load_api_keys(vault, &account_id)?;
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
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    let mut config = load_config(vault, &account_id)?;
    let api_key = if provider.is_built_in && provider.api_key == "••••••••" {
        String::new()
    } else {
        provider.api_key.clone()
    };
    if !api_key.is_empty() {
        save_api_key(vault, &account_id, &provider.id, &api_key)?;
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
    save_config(vault, &account_id, &config)
}

#[tauri::command]
pub async fn llm_set_active_provider(
    state: State<'_, AppState>,
    account_id: String,
    provider_id: String,
) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    let mut config = load_config(vault, &account_id)?;
    config.active_provider_id = Some(provider_id);
    save_config(vault, &account_id, &config)
}

#[tauri::command]
pub async fn llm_set_ai_features(
    state: State<'_, AppState>,
    account_id: String,
    features: AiFeatures,
) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    let mut config = load_config(vault, &account_id)?;
    config.ai_features_enabled = features;
    save_config(vault, &account_id, &config)
}

#[tauri::command]
pub async fn llm_set_system_prompt_switch(
    state: State<'_, AppState>,
    account_id: String,
    enabled: bool,
) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    let mut config = load_config(vault, &account_id)?;
    config.include_system_prompt = enabled;
    save_config(vault, &account_id, &config)
}

/// Toggle local embedding and set the active model ID.
#[tauri::command]
pub async fn llm_set_local_embedding(
    state: State<'_, AppState>,
    account_id: String,
    enabled: bool,
    model_id: Option<String>,
) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    let mut config = load_config(vault, &account_id)?;
    config.use_local_embedding = enabled;
    config.local_embed_model_id = model_id;
    save_config(vault, &account_id, &config)?;
    crate::local_embed::clear_embedder_cache();
    Ok(())
}

#[tauri::command]
pub async fn llm_accept_risk(state: State<'_, AppState>, account_id: String) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    let mut config = load_config(vault, &account_id)?;
    config.has_accepted_risk = true;
    save_config(vault, &account_id, &config)?;
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
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    load_api_keys(vault, &account_id).map(|k| k.get(&provider_id).cloned().unwrap_or_default())
}

#[tauri::command]
pub async fn llm_delete_provider(
    state: State<'_, AppState>,
    account_id: String,
    provider_id: String,
) -> Result<(), String> {
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
            let data: serde_json::Value =
                serde_json::from_slice(&profile.data).map_err(|e| format!("Parse: {}", e))?;
            Ok(data
                .get("preferences")
                .and_then(|p| p.get("llmConversations"))
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default())
        }
        _ => Ok(vec![]),
    }
}

fn save_conversations(
    vault: &VaultStore,
    account_id: &str,
    conversations: &[Conversation],
) -> Result<(), String> {
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
    let prefs = data
        .as_object_mut()
        .ok_or("Invalid")?
        .entry("preferences".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
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
pub async fn llm_list_conversations(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<Vec<ConversationSummary>, String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    let convs = load_conversations(vault, &account_id)?;
    let mut summaries: Vec<ConversationSummary> = convs
        .into_iter()
        .filter(|c| !c.is_temporary && c.deleted_at.is_none())
        .map(|c| ConversationSummary {
            id: c.id,
            name: c.name,
            updated_at: c.updated_at,
            message_count: c.messages.len(),
            deleted_at: None,
        })
        .collect();
    summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(summaries)
}

#[tauri::command]
pub async fn llm_list_trash(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<Vec<ConversationSummary>, String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    let convs = load_conversations(vault, &account_id)?;
    let mut summaries: Vec<ConversationSummary> = convs
        .into_iter()
        .filter(|c| c.deleted_at.is_some())
        .map(|c| ConversationSummary {
            id: c.id,
            name: c.name,
            updated_at: c.updated_at,
            message_count: c.messages.len(),
            deleted_at: c.deleted_at.clone(),
        })
        .collect();
    summaries.sort_by(|a, b| {
        b.deleted_at
            .as_deref()
            .unwrap_or(&a.updated_at)
            .cmp(a.deleted_at.as_deref().unwrap_or(&b.updated_at))
    });
    Ok(summaries)
}

#[tauri::command]
pub async fn llm_get_conversation(
    state: State<'_, AppState>,
    account_id: String,
    conversation_id: String,
) -> Result<Conversation, String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    let convs = load_conversations(vault, &account_id)?;
    convs
        .into_iter()
        .find(|c| c.id == conversation_id)
        .ok_or_else(|| "Not found".to_string())
}

#[tauri::command]
pub async fn llm_save_conversation(
    state: State<'_, AppState>,
    account_id: String,
    conversation: Conversation,
) -> Result<(), String> {
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
pub async fn llm_delete_conversation(
    state: State<'_, AppState>,
    account_id: String,
    conversation_id: String,
) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    let mut convs = load_conversations(vault, &account_id)?;
    convs.retain(|c| c.id != conversation_id);
    save_conversations(vault, &account_id, &convs)
}

#[tauri::command]
pub async fn llm_soft_delete_conversation(
    state: State<'_, AppState>,
    account_id: String,
    conversation_id: String,
) -> Result<(), String> {
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
pub async fn llm_restore_conversation(
    state: State<'_, AppState>,
    account_id: String,
    conversation_id: String,
) -> Result<(), String> {
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
pub async fn llm_permanent_delete(
    state: State<'_, AppState>,
    account_id: String,
    conversation_id: String,
) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    let mut convs = load_conversations(vault, &account_id)?;
    convs.retain(|c| c.id != conversation_id);
    save_conversations(vault, &account_id, &convs)
}

#[tauri::command]
pub async fn llm_rename_conversation(
    state: State<'_, AppState>,
    account_id: String,
    conversation_id: String,
    name: String,
) -> Result<(), String> {
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
pub async fn llm_check_connection(
    base_url: String,
    api_key: String,
    model: String,
    api_type: ApiType,
) -> Result<bool, String> {
    // Lightweight health check using test-provider pattern with very short timeout
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|_| "Client error")?;
    let (url, body) = if is_anthropic(&api_type) {
        (
            format!("{}/messages", base_url.trim_end_matches('/')),
            serde_json::json!({"model": model, "max_tokens": 1, "messages": [{"role": "user", "content": "Hi"}]}),
        )
    } else {
        (
            format!("{}/chat/completions", base_url.trim_end_matches('/')),
            serde_json::json!({"model": model, "messages": [{"role": "user", "content": "Hi"}], "max_tokens": 1}),
        )
    };
    let mut req = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body);
    if is_anthropic(&api_type) {
        req = req
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01");
    } else {
        req = req.header("Authorization", format!("Bearer {}", api_key));
    }
    match req.send().await {
        Ok(r) => Ok(r.status().is_success()),
        Err(_) => Ok(false),
    }
}

#[tauri::command]
pub async fn llm_test_provider(
    base_url: String,
    api_key: String,
    model: String,
    api_type: ApiType,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client: {}", e))?;

    let (url, body, auth_header, auth_value): (String, serde_json::Value, &str, String) =
        if is_anthropic(&api_type) {
            let u = format!("{}/messages", base_url.trim_end_matches('/'));
            let b = serde_json::json!({"model": model, "max_tokens": 10, "messages": [{"role": "user", "content": "Hello"}]});
            (u, b, "x-api-key", api_key)
        } else {
            let u = format!("{}/chat/completions", base_url.trim_end_matches('/'));
            let b = serde_json::json!({"model": model, "messages": [{"role": "user", "content": "Hello"}], "max_tokens": 5, "stream": false});
            (u, b, "Authorization", format!("Bearer {}", api_key))
        };

    let mut req = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body);
    if is_anthropic(&api_type) {
        req = req
            .header(auth_header, &auth_value)
            .header("anthropic-version", "2023-06-01");
    } else {
        req = req.header(auth_header, &auth_value);
    }

    let resp = req
        .send()
        .await
        .map_err(|e| format!("Request to {} failed: {}", url, e))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let snippet = if text.is_empty() {
            "(empty body)".to_string()
        } else {
            text.chars().take(300).collect()
        };
        return Err(format!("HTTP {} {} — {}", status.as_u16(), url, snippet));
    }
    let result: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse response from {}: {}", url, e))?;

    if is_anthropic(&api_type) {
        let text = result["content"]
            .as_array()
            .and_then(|arr| {
                arr.iter()
                    .find(|c| {
                        c.get("type").and_then(|t| t.as_str()) == Some("text")
                            || c.get("type").is_none()
                    })
                    .and_then(|c| c.get("text").and_then(|v| v.as_str()))
            })
            .unwrap_or("ok");
        Ok(text.to_string())
    } else {
        Ok(result["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("ok")
            .to_string())
    }
}

#[tauri::command]
pub async fn llm_send_message(
    base_url: String,
    api_key: String,
    model: String,
    api_type: ApiType,
    messages: Vec<serde_json::Value>,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Client: {}", e))?;

    if is_anthropic(&api_type) {
        // Anthropic Messages API format
        // Separate system message from chat messages
        let system = messages
            .iter()
            .find(|m| m.get("role").and_then(|r| r.as_str()) == Some("system"))
            .and_then(|m| m.get("content").and_then(|c| c.as_str()))
            .map(|s| s.to_string());
        let chat_msgs: Vec<serde_json::Value> = messages
            .into_iter()
            .filter(|m| m.get("role").and_then(|r| r.as_str()) != Some("system"))
            .collect();

        let mut body = serde_json::json!({
            "model": model,
            "max_tokens": 4096,
            "messages": chat_msgs,
        });
        if let Some(sys) = &system {
            body["system"] = serde_json::Value::String(sys.clone());
        }

        let url = format!("{}/messages", base_url.trim_end_matches('/'));
        let resp = client
            .post(&url)
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Request: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!(
                "HTTP {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        let result: serde_json::Value = resp.json().await.map_err(|e| format!("Parse: {}", e))?;
        // Anthropic thinking models return content blocks with types:
        // [{"type":"thinking","thinking":"..."}, {"type":"text","text":"..."}]
        let text = result["content"].as_array().and_then(|arr| {
            arr.iter()
                .find(|c| {
                    c.get("type").and_then(|t| t.as_str()) == Some("text")
                        || c.get("type").is_none()
                })
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
        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Request: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!(
                "HTTP {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
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

use once_cell::sync::Lazy;
use std::path::PathBuf;
use std::sync::Mutex;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuideTitle {
    pub zh: String,
    pub en: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuideCategoryMeta {
    pub id: String,
    pub title: GuideTitle,
    pub order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuideIndexEntry {
    pub id: String,
    pub title: GuideTitle,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub order: u32,
    pub keywords: Vec<String>,
    pub files: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuideIndex {
    pub guides: Vec<GuideIndexEntry>,
    #[serde(default)]
    pub categories: Vec<GuideCategoryMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuideContent {
    pub id: String,
    pub title: String,
    pub content: String,
}

/// 资源文件路径解析：开发模式从 src-tauri/resources/ 读取，生产模式从 app bundle 读取
pub fn resource_path(rel: &str) -> PathBuf {
    if cfg!(debug_assertions) {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join(rel);
        eprintln!("[resource_path] debug mode: {:?}", path);
        path
    } else {
        std::env::current_exe()
            .ok()
            .and_then(|exe| {
                let parent = exe.parent()?;
                // macOS app bundle: SoloSoul.app/Contents/MacOS/SoloSoul → ../Resources
                let resources = parent.join("../Resources");
                eprintln!(
                    "[resource_path] release mode: exe={:?}, resources={:?}",
                    exe, resources
                );
                Some(resources.join(rel))
            })
            .unwrap_or_else(|| {
                eprintln!("[resource_path] release mode fallback: {:?}", rel);
                PathBuf::from(rel)
            })
    }
}

/// 缓存的指南索引
static GUIDE_INDEX_CACHE: Lazy<Mutex<Option<GuideIndex>>> = Lazy::new(|| Mutex::new(None));

/// 指南摘要缓存：guideId -> 前 200 字摘要（用于 AI 快速匹配）
static GUIDE_SUMMARY_CACHE: Lazy<Mutex<HashMap<String, String>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// 获取缓存内容，容忍毒化锁（poisoned lock recovery）
fn get_index_cache() -> Option<GuideIndex> {
    let guard = GUIDE_INDEX_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    guard.clone()
}

fn set_index_cache(index: GuideIndex) {
    let mut guard = GUIDE_INDEX_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    *guard = Some(index);
}

fn get_summary_cache() -> HashMap<String, String> {
    let guard = GUIDE_SUMMARY_CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    guard.clone()
}

fn set_summary_cache(summaries: HashMap<String, String>) {
    let mut guard = GUIDE_SUMMARY_CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    *guard = summaries;
}

pub fn load_guide_index() -> Result<GuideIndex, String> {
    if let Some(idx) = get_index_cache() {
        return Ok(idx);
    }

    let path = resource_path("docs/guides/index.json");
    eprintln!("[load_guide_index] reading from {:?}", path);
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read guide index at {:?}: {}", path, e))?;
    let index: GuideIndex = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse guide index: {}", e))?;

    // 预加载每篇指南的摘要到缓存
    let mut summaries = HashMap::new();
    for guide in &index.guides {
        let lang = guide
            .files
            .keys()
            .next()
            .cloned()
            .unwrap_or_else(|| "en".to_string());
        if let Some(file) = guide.files.get(&lang) {
            let file_path = resource_path(&format!("docs/guides/{}", file));
            if let Ok(text) = std::fs::read_to_string(&file_path) {
                let summary = if text.len() > 200 {
                    // 找到不超过 200 字节的最近合法字符边界（避免在中文字符中间切片 panic）
                    let mut end = 200;
                    while !text.is_char_boundary(end) {
                        end -= 1;
                    }
                    let cut = &text[..end];
                    match cut.rfind('\n') {
                        Some(pos) => text[..pos].to_string(),
                        None => cut.to_string(),
                    }
                } else {
                    text
                };
                summaries.insert(guide.id.clone(), summary);
            }
        }
    }
    set_summary_cache(summaries);
    set_index_cache(index.clone());
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
    matches!(
        c,
        '。' | '，' | '！' | '？' | '、' | '；' | ':' | ';' | ',' | '.' | '!' | '?'
    )
}

fn is_stop_word(word: &str) -> bool {
    let stops: &[&str] = &[
        "的",
        "了",
        "是",
        "在",
        "我",
        "有",
        "和",
        "就",
        "不",
        "人",
        "都",
        "一",
        "一个",
        "上",
        "也",
        "很",
        "到",
        "说",
        "要",
        "去",
        "你",
        "会",
        "着",
        "没有",
        "看",
        "好",
        "自己",
        "这",
        "the",
        "a",
        "an",
        "is",
        "are",
        "was",
        "were",
        "be",
        "been",
        "being",
        "have",
        "has",
        "had",
        "do",
        "does",
        "did",
        "will",
        "would",
        "could",
        "should",
        "may",
        "might",
        "must",
        "shall",
        "can",
        "need",
        "dare",
        "ought",
        "used",
        "to",
        "of",
        "in",
        "for",
        "on",
        "with",
        "at",
        "by",
        "from",
        "as",
        "into",
        "through",
        "during",
        "before",
        "after",
        "above",
        "below",
        "between",
        "under",
        "and",
        "but",
        "or",
        "yet",
        "so",
        "if",
        "because",
        "although",
        "though",
        "while",
        "where",
        "when",
        "that",
        "which",
        "who",
        "whom",
        "whose",
        "what",
        "whatever",
        "whoever",
        "whomever",
        "this",
        "these",
        "those",
        "i",
        "me",
        "my",
        "myself",
        "we",
        "our",
        "ours",
        "ourselves",
        "you",
        "your",
        "yours",
        "yourself",
        "yourselves",
        "he",
        "him",
        "his",
        "himself",
        "she",
        "her",
        "hers",
        "herself",
        "it",
        "its",
        "itself",
        "they",
        "them",
        "their",
        "theirs",
        "themselves",
    ];
    stops.contains(&word)
}

pub fn resolve_language(files: &HashMap<String, String>, requested: &str) -> String {
    if files.contains_key(requested) {
        return requested.to_string();
    }
    // 简化为 'zh' 或 'en'
    let short = if requested.starts_with("zh") {
        "zh"
    } else {
        "en"
    };
    if files.contains_key(short) {
        return short.to_string();
    }
    if files.contains_key("en") {
        return "en".to_string();
    }
    files
        .keys()
        .next()
        .cloned()
        .unwrap_or_else(|| "en".to_string())
}

pub fn resolve_title(title: &GuideTitle, language: &str) -> String {
    if language.starts_with("zh") {
        title.zh.clone()
    } else {
        title.en.clone()
    }
}

fn load_guide_content(entry: &GuideIndexEntry, language: &str) -> Result<GuideContent, String> {
    let lang = resolve_language(&entry.files, language);
    let rel_path = format!("docs/guides/{}", entry.files.get(&lang).ok_or("No file")?);
    let path = resource_path(&rel_path);
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read guide {:?}: {}", path, e))?;

    // 文档较短时不截断；超长时截断至 4000 字节（覆盖所有现有帮助文档）
    const MAX_GUIDE_LEN: usize = 4000;
    let truncated = if content.len() > MAX_GUIDE_LEN {
        let mut end = MAX_GUIDE_LEN;
        while !content.is_char_boundary(end) {
            end -= 1;
        }
        let mut cut = &content[..end];
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
        title: resolve_title(&entry.title, language),
        content: truncated,
    })
}

fn find_relevant_guides_internal(query: &str, language: &str) -> Result<Vec<GuideContent>, String> {
    let index = load_guide_index()?;
    let tokens = tokenize_query(query);
    if tokens.is_empty() {
        return Ok(vec![]);
    }

    // 意图分类：简单规则加权
    let is_howto = tokens
        .iter()
        .any(|t| ["怎么", "如何", "怎样", "how", "步骤", "step"].contains(&t.as_str()));
    let is_concept = tokens
        .iter()
        .any(|t| ["什么是", "为什么", "what", "why", "explain"].contains(&t.as_str()));

    let threshold = if tokens.len() >= 2 { 2 } else { 1 };

    let summary_cache = get_summary_cache();

    let mut scored: Vec<(GuideIndexEntry, i32)> = vec![];
    for guide in &index.guides {
        let mut score = 0;
        let title_text = resolve_title(&guide.title, language).to_lowercase();
        let summary_text = summary_cache
            .get(&guide.id)
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        for token in &tokens {
            if guide
                .keywords
                .iter()
                .any(|k| k.to_lowercase().contains(token))
            {
                score += 1;
            }
            if title_text.contains(token) {
                score += 3;
            }
            if summary_text.contains(token) {
                score += 2; // 摘要命中权重介于关键词和标题之间
            }
        }
        // 意图加权
        if is_howto && guide.category == "objects" {
            score += 2;
        }
        if is_concept && guide.category == "security" {
            score += 2;
        }
        if score >= threshold {
            scored.push((guide.clone(), score));
        }
    }

    scored.sort_by_key(|b| std::cmp::Reverse(b.1));

    // Top-3（v2.0 从 Top-1 扩展为 3 篇互补）
    let mut results = vec![];
    for (entry, _) in scored.into_iter().take(3) {
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
// Guide System Commands (§18)
// =============================================================================

#[tauri::command]
pub async fn guide_load_index() -> Result<GuideIndex, String> {
    load_guide_index()
}

#[tauri::command]
pub async fn guide_load_content(
    guide_id: String,
    language: String,
) -> Result<GuideContent, String> {
    let index = load_guide_index()?;
    let entry = index
        .guides
        .into_iter()
        .find(|g| g.id == guide_id)
        .ok_or_else(|| format!("Guide not found: {}", guide_id))?;
    let lang = resolve_language(&entry.files, &language);
    let rel_path = format!("docs/guides/{}", entry.files.get(&lang).ok_or("No file")?);
    let path = resource_path(&rel_path);
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read guide {:?}: {}", path, e))?;
    Ok(GuideContent {
        id: entry.id,
        title: resolve_title(&entry.title, &language),
        content,
    })
}

#[tauri::command]
pub async fn guide_search(query: String, language: String) -> Result<Vec<GuideContent>, String> {
    let index = load_guide_index()?;
    let tokens: Vec<String> = query
        .to_lowercase()
        .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    if tokens.is_empty() {
        return Ok(vec![]);
    }

    let mut scored: Vec<(GuideIndexEntry, i32)> = vec![];
    for guide in index.guides {
        let mut score = 0;
        let title_text = resolve_title(&guide.title, &language).to_lowercase();
        for token in &tokens {
            if guide
                .keywords
                .iter()
                .any(|k| k.to_lowercase().contains(token))
            {
                score += 1;
            }
            if title_text.contains(token) {
                score += 3;
            }
        }
        if score >= 1 {
            scored.push((guide, score));
        }
    }

    scored.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut results = vec![];
    for (entry, _) in scored.into_iter().take(10) {
        match guide_load_content(entry.id.clone(), language.clone()).await {
            Ok(g) => results.push(g),
            Err(e) => eprintln!("Guide load error: {}", e),
        }
    }
    Ok(results)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchIndex {
    pub words: std::collections::HashMap<String, Vec<String>>,
    pub titles: std::collections::HashMap<String, GuideTitle>,
}

#[tauri::command]
pub async fn guide_load_search_index() -> Result<SearchIndex, String> {
    let path = resource_path("docs/guides/search-index.json");
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read search index at {:?}: {}", path, e))?;
    let index: SearchIndex = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse search index: {}", e))?;
    Ok(index)
}

// =============================================================================
// Usage Statistics (§10)
// =============================================================================

use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

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
    pub provider: String,
    pub count: u64,
    pub tokens: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub last_used_time: Option<String>, // ISO8601
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyUsage {
    pub date: String, // YYYY-MM-DD
    pub count: u64,
    pub tokens: u64,
    pub per_model_tokens: HashMap<String, u64>, // Key: "provider/model"
}

/// 内存中的使用统计（按账户隔离）
static STATS_MAP: Lazy<Arc<RwLock<HashMap<String, LlmUsageStats>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// 估算 Token 数（保守策略：所有字符按 1 token）
pub fn estimate_tokens(text: &str) -> u64 {
    text.chars().count() as u64
}

/// 记录一次 AI 调用（使用真实 token 数）
/// 四层聚合：account totals → per-model → daily (with per-model breakdown)
pub async fn record_usage(
    account_id: &str,
    model: &str,
    provider: &str,
    prompt_tokens: u64,
    completion_tokens: u64,
) {
    let total = prompt_tokens + completion_tokens;
    let now_iso = chrono::Utc::now().to_rfc3339();
    let model_key = format!("{}/{}", provider, model);
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    let mut map: tokio::sync::RwLockWriteGuard<'_, HashMap<String, LlmUsageStats>> =
        STATS_MAP.write().await;
    let stats: &mut LlmUsageStats = map.entry(account_id.to_string()).or_default();

    // 1. Account-level totals
    stats.usage_count += 1;
    stats.prompt_tokens += prompt_tokens;
    stats.completion_tokens += completion_tokens;
    stats.total_tokens += total;

    // 2. Per-model stats
    if let Some(m) = stats
        .per_model_stats
        .iter_mut()
        .find(|m| m.model == model && m.provider == provider)
    {
        m.count += 1;
        m.tokens += total;
        m.prompt_tokens += prompt_tokens;
        m.completion_tokens += completion_tokens;
        m.last_used_time = Some(now_iso.clone());
    } else {
        stats.per_model_stats.push(ModelUsage {
            model: model.to_string(),
            provider: provider.to_string(),
            count: 1,
            tokens: total,
            prompt_tokens,
            completion_tokens,
            last_used_time: Some(now_iso.clone()),
        });
    }

    // 3. Daily stats (with per-model breakdown)
    if let Some(d) = stats.daily_stats.iter_mut().find(|d| d.date == today) {
        d.count += 1;
        d.tokens += total;
        let prev = d.per_model_tokens.get(&model_key).copied().unwrap_or(0);
        d.per_model_tokens.insert(model_key, prev + total);
    } else {
        let mut per_model = HashMap::new();
        per_model.insert(model_key, total);
        stats.daily_stats.push(DailyUsage {
            date: today,
            count: 1,
            tokens: total,
            per_model_tokens: per_model,
        });
    }
}

/// 回退：当 API 未返回真实 token 时，使用估算值
pub async fn record_usage_fallback(
    account_id: &str,
    model: &str,
    provider: &str,
    prompt: &str,
    completion: &str,
) {
    let prompt_tokens = estimate_tokens(prompt);
    let completion_tokens = estimate_tokens(completion);
    record_usage(
        account_id,
        model,
        provider,
        prompt_tokens,
        completion_tokens,
    )
    .await;
}

fn save_stats_to_vault(
    vault: &VaultStore,
    account_id: &str,
    stats: &LlmUsageStats,
) -> Result<(), String> {
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
    let prefs = data
        .as_object_mut()
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
            let data: serde_json::Value =
                serde_json::from_slice(&profile.data).map_err(|e| format!("Parse: {}", e))?;
            Ok(data
                .get("preferences")
                .and_then(|p| p.get("llmUsageStats"))
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default())
        }
        _ => Ok(LlmUsageStats::default()),
    }
}

#[tauri::command]
pub async fn llm_get_stats(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<LlmUsageStats, String> {
    // 1. 尝试从内存读取
    {
        let map: tokio::sync::RwLockReadGuard<'_, HashMap<String, LlmUsageStats>> =
            STATS_MAP.read().await;
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
        let mut map: tokio::sync::RwLockWriteGuard<'_, HashMap<String, LlmUsageStats>> =
            STATS_MAP.write().await;
        map.insert(account_id.clone(), stats.clone());
    }
    Ok(stats)
}

#[tauri::command]
pub async fn llm_reset_stats(state: State<'_, AppState>, account_id: String) -> Result<(), String> {
    {
        let mut map: tokio::sync::RwLockWriteGuard<'_, HashMap<String, LlmUsageStats>> =
            STATS_MAP.write().await;
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
        let map: tokio::sync::RwLockReadGuard<'_, HashMap<String, LlmUsageStats>> =
            STATS_MAP.read().await;
        map.get(account_id).cloned().unwrap_or_default()
    };
    save_stats_to_vault(vault, account_id, &stats)
}

#[tauri::command]
pub async fn llm_persist_stats(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<(), String> {
    let stats: LlmUsageStats = {
        let map: tokio::sync::RwLockReadGuard<'_, HashMap<String, LlmUsageStats>> =
            STATS_MAP.read().await;
        map.get(&account_id).cloned().unwrap_or_default()
    };
    let svc = state.vault_service.read().await;
    let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vg.as_ref().ok_or("Vault not unlocked")?;
    save_stats_to_vault(vault, &account_id, &stats)
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
async fn emit_typing_effect(app: &tauri::AppHandle, conversation_id: &str, full_text: &str) {
    let graphemes: Vec<String> = full_text.graphemes(true).map(|g| g.to_string()).collect();
    let total = graphemes.len();
    let max_typing_ms = 3000u64;
    let delay_ms = if total <= 50 { 2u64 } else { 4u64 };

    for (i, g) in graphemes.iter().enumerate() {
        let elapsed = (i as u64) * delay_ms;
        if elapsed >= max_typing_ms {
            let remaining: String = graphemes[i..].concat();
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
        let _ = app.emit(
            "llm-stream-chunk",
            LlmStreamPayload {
                conversation_id: conversation_id.to_string(),
                chunk: g.clone(),
                is_done: false,
                error: None,
            },
        );
        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
    }

    if (total as u64) * delay_ms < max_typing_ms {
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
}

/// 发送聊天请求并流式推送结果（Phase 2.3：SSE 流式 + 打字机降级）
/// 返回 (完整文本, 可选的真实 TokenUsage)
async fn send_chat_stream(
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

    let (url, body, auth_header, auth_value): (String, serde_json::Value, &str, String) =
        if is_anthropic(&api_type) {
            let system = messages
                .iter()
                .find(|m| m.get("role").and_then(|r| r.as_str()) == Some("system"))
                .and_then(|m| m.get("content").and_then(|c| c.as_str()))
                .map(|s| s.to_string());
            let chat_msgs: Vec<serde_json::Value> = messages
                .into_iter()
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
            (
                format!("{}/messages", base_url.trim_end_matches('/')),
                b,
                "x-api-key",
                api_key,
            )
        } else {
            let mut b = serde_json::json!({"model": model, "messages": messages, "stream": true});
            b["stream_options"] = serde_json::json!({"include_usage": true});
            (
                format!("{}/chat/completions", base_url.trim_end_matches('/')),
                b,
                "Authorization",
                format!("Bearer {}", api_key),
            )
        };

    let resp = client
        .post(&url)
        .header(auth_header, &auth_value)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request: {}", e))?;

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

        let full_text = if is_anthropic(&api_type) {
            result["content"]
                .as_array()
                .and_then(|arr| {
                    arr.iter()
                        .find(|c| {
                            c.get("type").and_then(|t| t.as_str()) == Some("text")
                                || c.get("type").is_none()
                        })
                        .and_then(|c| c.get("text").and_then(|v| v.as_str()))
                })
                .unwrap_or("")
                .to_string()
        } else {
            result["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string()
        };

        // 提取非 SSE 的真实 usage
        let mut token_usage = TokenUsage::default();
        if !is_anthropic(&api_type) {
            if let Some(usage) = result.get("usage") {
                if let Some(prompt) = usage.get("prompt_tokens").and_then(|v| v.as_u64()) {
                    token_usage.prompt_tokens = prompt;
                }
                if let Some(completion) = usage.get("completion_tokens").and_then(|v| v.as_u64()) {
                    token_usage.completion_tokens = completion;
                }
            }
        }
        // Anthropic 非流式响应通常也有 usage（如果需要可以后续补充）

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
        let svc = state.vault_service.read().await;
        let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
        let vault = vg.as_ref().ok_or("Vault not unlocked")?;
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

    Ok(())
}

// Helper: load providers with decrypted keys (internal reuse)
fn load_providers_with_keys(
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

// =============================================================================
// RAG Embedding API (§RAG-3)
// =============================================================================

/// Normalize a vector to unit length.
fn normalize_vector(vec: &mut [f32]) {
    let norm: f32 = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in vec.iter_mut() {
            *v /= norm;
        }
    }
}

/// Compute dot product of two vectors (assumes both are normalized).
fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Source for embedding: cloud API or local ONNX model.
#[derive(Clone)]
enum EmbeddingSource {
    Cloud {
        base_url: String,
        api_key: String,
        model: String,
    },
    Local {
        model_id: String,
    },
}

/// Get the embedding source for the active account.
/// Checks local embedding preference first, then falls back to cloud provider.
fn get_embedding_source(
    vault: &VaultStore,
    account_id: &str,
    models_dir: &std::path::Path,
) -> Result<EmbeddingSource, String> {
    let config = load_config(vault, account_id)?;

    // 1. Check if local embedding is preferred and model is installed
    if config.use_local_embedding {
        if let Some(ref model_id) = config.local_embed_model_id {
            if crate::local_embed::is_model_installed(models_dir, model_id) {
                return Ok(EmbeddingSource::Local {
                    model_id: model_id.clone(),
                });
            }
        }
    }

    // 2. Fall back to cloud provider
    let active_id = config.active_provider_id.ok_or("No active provider")?;
    let providers = load_providers_with_keys(vault, account_id)?;
    let active = providers
        .into_iter()
        .find(|p| p.id == active_id)
        .ok_or("Active provider not found")?;

    if !active.is_enabled {
        return Err("Provider is disabled".to_string());
    }

    if matches!(active.api_type, ApiType::Anthropic) {
        return Err("Anthropic does not support embedding API".to_string());
    }

    let api_key = if active.api_key == "••••••••" {
        load_api_keys(vault, account_id)?
            .get(&active.id)
            .cloned()
            .unwrap_or_default()
    } else {
        active.api_key
    };

    let embedding_model = active
        .embedding_model
        .or_else(|| match active.name.to_lowercase().as_str() {
            n if n.contains("openai") => Some("text-embedding-3-small".into()),
            n if n.contains("ollama") => Some("nomic-embed-text".into()),
            n if n.contains("deepseek") => Some("text-embedding".into()),
            n if n.contains("alibaba") => Some("text-embedding-v3".into()),
            _ => None,
        })
        .ok_or("No embedding model configured for this provider")?;

    Ok(EmbeddingSource::Cloud {
        base_url: active.base_url,
        api_key,
        model: embedding_model,
    })
}

/// Call the embedding API (cloud or local) for a single text.
/// Returns normalized embedding vector.
async fn embed_text(
    source: EmbeddingSource,
    models_dir: std::path::PathBuf,
    text: String,
) -> Result<Vec<f32>, String> {
    match source {
        EmbeddingSource::Cloud {
            base_url,
            api_key,
            model,
        } => {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| format!("Client: {}", e))?;

            let url = format!("{}/embeddings", base_url.trim_end_matches('/'));
            let body = serde_json::json!({
                "input": text,
                "model": model,
                "encoding_format": "float"
            });

            let resp = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("Request to {} failed: {}", url, e))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body_text = resp.text().await.unwrap_or_default();
                return Err(format!(
                    "Embedding API HTTP {}: {}",
                    status,
                    body_text.chars().take(300).collect::<String>()
                ));
            }

            let result: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("Parse embedding response: {}", e))?;

            let embedding = result["data"]
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|obj| obj["embedding"].as_array())
                .ok_or("Invalid embedding response format")?;

            let mut vec: Vec<f32> = embedding
                .iter()
                .filter_map(|v| v.as_f64().map(|f| f as f32))
                .collect();

            if vec.is_empty() {
                return Err("Empty embedding vector".to_string());
            }

            normalize_vector(&mut vec);
            Ok(vec)
        }
        EmbeddingSource::Local { model_id } => {
            let embedder = crate::local_embed::get_embedder(&models_dir, &model_id)?;
            tokio::task::spawn_blocking(move || embedder.embed(&text))
                .await
                .map_err(|e| format!("Embedding task: {}", e))?
        }
    }
}

/// Batch embed multiple texts. Stops on first error.
async fn embed_texts(
    source: EmbeddingSource,
    models_dir: std::path::PathBuf,
    texts: Vec<String>,
) -> Result<Vec<Vec<f32>>, String> {
    match &source {
        EmbeddingSource::Local { model_id } => {
            let model_id = model_id.clone();
            let embedder = crate::local_embed::get_embedder(&models_dir, &model_id)?;
            tokio::task::spawn_blocking(move || embedder.embed_batch(&texts))
                .await
                .map_err(|e| format!("Embedding batch task: {}", e))?
        }
        EmbeddingSource::Cloud { .. } => {
            let mut results = Vec::with_capacity(texts.len());
            for text in texts {
                let vec = embed_text(source.clone(), models_dir.clone(), text).await?;
                results.push(vec);
            }
            Ok(results)
        }
    }
}

/// Search guide chunks by vector similarity.
/// Falls back to keyword search if embedding is unavailable.
#[tauri::command]
pub async fn llm_search_guide_chunks(
    state: State<'_, AppState>,
    account_id: String,
    query: String,
    language: String,
    top_k: Option<usize>,
) -> Result<Vec<super::rag::GuideChunk>, String> {
    let top_k = top_k.unwrap_or(3);

    // 1. Load embedding source and existing chunks (sync block)
    let models_dir = state
        .handle
        .path()
        .resolve("models", tauri::path::BaseDirectory::LocalData)
        .map_err(|e| format!("Resolve models dir: {}", e))?;

    let (source, chunks) = {
        let svc = state.vault_service.read().await;
        let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
        let vault = vg.as_ref().ok_or("Vault not unlocked")?;

        let source = match get_embedding_source(vault, &account_id, &models_dir) {
            Ok(s) => s,
            Err(e) => {
                eprintln!(
                    "[RAG] Embedding source error: {}, falling back to keyword search",
                    e
                );
                return fallback_keyword_search(&query, &language, top_k);
            }
        };

        let chunks = match vault.list_guide_embeddings() {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "[RAG] Load embeddings failed: {}, falling back to keyword search",
                    e
                );
                return fallback_keyword_search(&query, &language, top_k);
            }
        };

        if chunks.is_empty() {
            eprintln!("[RAG] No embeddings found, falling back to keyword search");
            return fallback_keyword_search(&query, &language, top_k);
        }

        (source, chunks)
    };

    // 2. Embed query (async)
    let mut query_vec = match embed_text(source, models_dir, query.clone()).await {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "[RAG] Embed query failed: {}, falling back to keyword search",
                e
            );
            return fallback_keyword_search(&query, &language, top_k);
        }
    };
    normalize_vector(&mut query_vec);

    // 3. Compute similarities and return top-k
    let mut scored: Vec<(f32, solosoul_vault::GuideEmbeddingChunk)> = chunks
        .into_iter()
        .map(|chunk| {
            let sim = dot_product(&query_vec, &chunk.embedding);
            (sim, chunk)
        })
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut results = Vec::new();
    for (sim, chunk) in scored.into_iter().take(top_k) {
        let guide_title = super::rag::chunk_all_guides(&language)
            .ok()
            .and_then(|raws| raws.into_iter().find(|r| r.guide_id == chunk.guide_id))
            .map(|r| r.guide_title)
            .unwrap_or_else(|| chunk.guide_id.clone());

        results.push(super::rag::GuideChunk {
            guide_id: chunk.guide_id,
            guide_title,
            chunk_text: chunk.chunk_text,
            similarity: sim,
        });
    }

    if results.is_empty() {
        fallback_keyword_search(&query, &language, top_k)
    } else {
        Ok(results)
    }
}

/// Fallback to keyword-based guide search.
fn fallback_keyword_search(
    query: &str,
    language: &str,
    top_k: usize,
) -> Result<Vec<super::rag::GuideChunk>, String> {
    let guides = find_relevant_guides_internal(query, language)?;
    let mut results = Vec::new();
    for guide in guides.into_iter().take(top_k) {
        results.push(super::rag::GuideChunk {
            guide_id: guide.id.clone(),
            guide_title: guide.title.clone(),
            chunk_text: guide.content,
            similarity: 0.5, // placeholder similarity for fallback
        });
    }
    Ok(results)
}

/// Rebuild all guide embeddings. Clears existing and re-creates.
#[tauri::command]
pub async fn llm_rebuild_guide_embeddings(
    state: State<'_, AppState>,
    account_id: String,
    language: String,
) -> Result<usize, String> {
    let models_dir = state
        .handle
        .path()
        .resolve("models", tauri::path::BaseDirectory::LocalData)
        .map_err(|e| format!("Resolve models dir: {}", e))?;

    // 1. Extract embedding source and chunk guides (sync, vault guard released after this block)
    let (source, raw_chunks) = {
        let svc = state.vault_service.read().await;
        let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
        let vault = vg.as_ref().ok_or("Vault not unlocked")?;

        let source = get_embedding_source(vault, &account_id, &models_dir)?;
        let raw_chunks = super::rag::chunk_all_guides(&language)?;
        if raw_chunks.is_empty() {
            return Ok(0);
        }
        vault
            .clear_guide_embeddings()
            .map_err(|e| format!("Clear embeddings: {}", e))?;
        (source, raw_chunks)
    };

    let model_name = match &source {
        EmbeddingSource::Cloud { model, .. } => model.clone(),
        EmbeddingSource::Local { model_id } => model_id.clone(),
    };

    // 2. Batch embed all chunks (async)
    let texts: Vec<String> = raw_chunks.iter().map(|c| c.text.clone()).collect();
    let embeddings = embed_texts(source, models_dir, texts).await?;

    // 3. Store in vault (sync)
    let count = {
        let svc = state.vault_service.read().await;
        let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
        let vault = vg.as_ref().ok_or("Vault not unlocked")?;

        let now = chrono::Utc::now().to_rfc3339();
        for (i, (raw, mut vec)) in raw_chunks.into_iter().zip(embeddings).enumerate() {
            normalize_vector(&mut vec);
            let chunk = solosoul_vault::GuideEmbeddingChunk {
                id: format!("{}_{}", raw.guide_id, raw.chunk_index),
                guide_id: raw.guide_id,
                chunk_index: raw.chunk_index as i32,
                chunk_text: raw.text,
                embedding: vec,
                model: model_name.clone(),
                created_at: now.clone(),
            };
            vault
                .save_guide_embedding(&chunk)
                .map_err(|e| format!("Save embedding {}: {}", i, e))?;
        }

        super::rag::mark_rebuilt(vault, &language)?;
        vault.count_guide_embeddings()?
    };

    Ok(count)
}

/// Check if embedding is available (cloud or local).
#[tauri::command]
pub async fn llm_check_embedding_available(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<bool, String> {
    let models_dir = state
        .handle
        .path()
        .resolve("models", tauri::path::BaseDirectory::LocalData)
        .map_err(|e| format!("Resolve models dir: {}", e))?;

    let source = {
        let svc = state.vault_service.read().await;
        let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
        let vault = vg.as_ref().ok_or("Vault not unlocked")?;
        get_embedding_source(vault, &account_id, &models_dir)
    };

    match source {
        Ok(EmbeddingSource::Local { model_id }) => {
            // Local model: just check if it's installed and can load
            match crate::local_embed::get_embedder(&models_dir, &model_id) {
                Ok(_) => Ok(true),
                Err(e) => {
                    eprintln!("[RAG] Local embedding not available: {}", e);
                    Ok(false)
                }
            }
        }
        Ok(EmbeddingSource::Cloud {
            base_url,
            api_key,
            model,
        }) => {
            // Try a test embedding call with a dummy text
            match embed_text(
                EmbeddingSource::Cloud {
                    base_url,
                    api_key,
                    model,
                },
                models_dir,
                "test".into(),
            )
            .await
            {
                Ok(_) => Ok(true),
                Err(e) => {
                    eprintln!("[RAG] Embedding availability check failed: {}", e);
                    Ok(false)
                }
            }
        }
        Err(e) => {
            eprintln!("[RAG] No embedding source: {}", e);
            Ok(false)
        }
    }
}

/// Ensure guide embeddings are built on app startup.
/// Called from app setup. Non-blocking, errors are logged only.
#[allow(clippy::await_holding_lock)]
pub async fn ensure_guide_embeddings_built(state: &AppState, account_id: &str, language: &str) {
    let result = async {
        let models_dir = state
            .handle
            .path()
            .resolve("models", tauri::path::BaseDirectory::LocalData)
            .map_err(|e| format!("Resolve models dir: {}", e))?;

        let svc = state.vault_service.read().await;
        let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
        let vault = vg.as_ref().ok_or("Vault not unlocked")?;

        if !super::rag::needs_rebuild(vault, language)? {
            return Ok::<(), String>(());
        }

        let source = match get_embedding_source(vault, account_id, &models_dir) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[RAG] Cannot build embeddings: {}", e);
                return Ok(());
            }
        };

        let model_name = match &source {
            EmbeddingSource::Cloud { model, .. } => model.clone(),
            EmbeddingSource::Local { model_id } => model_id.clone(),
        };

        let raw_chunks = super::rag::chunk_all_guides(language)?;
        if raw_chunks.is_empty() {
            return Ok(());
        }

        vault.clear_guide_embeddings().map_err(|e| e.to_string())?;
        drop(vg);
        drop(svc);

        let texts: Vec<String> = raw_chunks.iter().map(|c| c.text.clone()).collect();
        let embeddings = embed_texts(source, models_dir, texts).await?;

        let svc = state.vault_service.read().await;
        let vg = svc.get_vault_store().ok_or("Vault not unlocked")?;
        let vault = vg.as_ref().ok_or("Vault not unlocked")?;

        let now = chrono::Utc::now().to_rfc3339();
        for (raw, mut vec) in raw_chunks.into_iter().zip(embeddings) {
            normalize_vector(&mut vec);
            let chunk = solosoul_vault::GuideEmbeddingChunk {
                id: format!("{}_{}", raw.guide_id, raw.chunk_index),
                guide_id: raw.guide_id,
                chunk_index: raw.chunk_index as i32,
                chunk_text: raw.text,
                embedding: vec,
                model: model_name.clone(),
                created_at: now.clone(),
            };
            vault
                .save_guide_embedding(&chunk)
                .map_err(|e| e.to_string())?;
        }

        super::rag::mark_rebuilt(vault, language)?;
        let count = vault.count_guide_embeddings()?;
        eprintln!("[RAG] Built {} guide embeddings", count);
        Ok(())
    }
    .await;

    if let Err(e) = result {
        eprintln!("[RAG] Failed to build embeddings: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solosoul_vault::{VaultConfig, VaultStore};
    use tempfile::TempDir;

    fn setup_vault() -> (VaultStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let config = VaultConfig::new("test_account", dir.path().to_path_buf());
        let vault = VaultStore::open(config).unwrap();
        (vault, dir)
    }

    // ── Trivial helpers ─────────────────────────────────────────

    #[test]
    fn test_default_true() {
        assert!(default_true());
    }

    #[test]
    fn test_default_providers() {
        let providers = default_providers();
        assert!(!providers.is_empty());
        assert_eq!(providers.len(), 5);

        let ids: Vec<&str> = providers.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"builtin_openai"));
        assert!(ids.contains(&"builtin_anthropic"));
        assert!(ids.contains(&"builtin_ollama"));
        assert!(ids.contains(&"builtin_deepseek"));
        assert!(ids.contains(&"builtin_alibaba"));

        // All defaults are built-in and disabled
        for p in &providers {
            assert!(p.is_built_in);
            assert!(!p.is_enabled);
            assert!(p.api_key.is_empty());
        }
    }

    #[test]
    fn test_is_anthropic() {
        assert!(!is_anthropic(&ApiType::OpenAI));
        assert!(is_anthropic(&ApiType::Anthropic));
    }

    #[test]
    fn test_now_iso_format() {
        let ts = now_iso();
        // Ends with Z and can be parsed as RFC 3339
        assert!(ts.ends_with('Z'));
        assert!(chrono::DateTime::parse_from_rfc3339(&ts).is_ok());
    }

    // ── Serde roundtrips ────────────────────────────────────────

    #[test]
    fn test_api_type_serde_roundtrip() {
        for original in [ApiType::OpenAI, ApiType::Anthropic] {
            let json = serde_json::to_string(&original).unwrap();
            let restored: ApiType = serde_json::from_str(&json).unwrap();
            assert_eq!(original, restored);
        }
        // Verify camelCase serialization
        assert_eq!(serde_json::to_string(&ApiType::OpenAI).unwrap(), "\"openAI\"");
        assert_eq!(
            serde_json::to_string(&ApiType::Anthropic).unwrap(),
            "\"anthropic\""
        );
    }

    #[test]
    fn test_provider_config_serde_roundtrip() {
        let original = ProviderConfig {
            id: "custom_1".into(),
            name: "Custom".into(),
            base_url: "https://example.com/v1".into(),
            model: "gpt-4".into(),
            is_enabled: true,
            is_built_in: false,
            api_type: ApiType::Anthropic,
            embedding_model: Some("embed-model".into()),
        };
        let json = serde_json::to_string(&original).unwrap();
        let restored: ProviderConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, original.id);
        assert_eq!(restored.name, original.name);
        assert_eq!(restored.base_url, original.base_url);
        assert_eq!(restored.model, original.model);
        assert_eq!(restored.is_enabled, original.is_enabled);
        assert_eq!(restored.is_built_in, original.is_built_in);
        assert_eq!(restored.api_type, original.api_type);
        assert_eq!(restored.embedding_model, original.embedding_model);
    }

    #[test]
    fn test_provider_with_key_serde_roundtrip() {
        let original = ProviderWithKey {
            id: "pwk_1".into(),
            name: "Provider".into(),
            base_url: "https://api.example.com".into(),
            model: "model-x".into(),
            is_enabled: true,
            is_built_in: true,
            api_key: "sk-secret".into(),
            api_type: ApiType::OpenAI,
            embedding_model: None,
        };
        let json = serde_json::to_string(&original).unwrap();
        let restored: ProviderWithKey = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, original.id);
        assert_eq!(restored.name, original.name);
        assert_eq!(restored.base_url, original.base_url);
        assert_eq!(restored.model, original.model);
        assert_eq!(restored.is_enabled, original.is_enabled);
        assert_eq!(restored.is_built_in, original.is_built_in);
        assert_eq!(restored.api_key, original.api_key);
        assert_eq!(restored.api_type, original.api_type);
        assert_eq!(restored.embedding_model, original.embedding_model);
    }

    #[test]
    fn test_llm_config_serde_roundtrip_with_defaults() {
        let original = LlmConfig {
            providers: vec![ProviderConfig {
                id: "p1".into(),
                name: "P1".into(),
                base_url: "http://localhost".into(),
                model: "m1".into(),
                is_enabled: false,
                is_built_in: false,
                api_type: ApiType::OpenAI,
                embedding_model: None,
            }],
            active_provider_id: Some("p1".into()),
            ai_features_enabled: AiFeatures {
                chat: true,
                smart_fill: false,
                command_gen: true,
                natural_language_search: false,
            },
            has_accepted_risk: true,
            include_system_prompt: false,
            use_local_embedding: true,
            local_embed_model_id: Some("model-id".into()),
        };
        let json = serde_json::to_string(&original).unwrap();
        let restored: LlmConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.providers.len(), original.providers.len());
        assert_eq!(restored.active_provider_id, original.active_provider_id);
        assert_eq!(restored.has_accepted_risk, original.has_accepted_risk);
        assert_eq!(restored.include_system_prompt, original.include_system_prompt);
        assert_eq!(restored.use_local_embedding, original.use_local_embedding);
        assert_eq!(restored.local_embed_model_id, original.local_embed_model_id);
        assert_eq!(
            restored.ai_features_enabled.chat,
            original.ai_features_enabled.chat
        );
    }

    #[test]
    fn test_llm_config_default_fields() {
        // When deserializing a partial object, defaults should apply for omitted fields
        let json = r#"{"providers":[],"activeProviderId":null,"aiFeaturesEnabled":{"chat":false,"smartFill":false,"commandGen":false,"naturalLanguageSearch":false},"hasAcceptedRisk":false}"#;
        let config: LlmConfig = serde_json::from_str(json).unwrap();
        assert!(config.providers.is_empty());
        assert_eq!(config.active_provider_id, None);
        assert!(config.include_system_prompt); // default_true
        assert!(!config.use_local_embedding);
        assert_eq!(config.local_embed_model_id, None);
    }

    #[test]
    fn test_ai_features_serde_roundtrip() {
        let original = AiFeatures {
            chat: true,
            smart_fill: false,
            command_gen: true,
            natural_language_search: true,
        };
        let json = serde_json::to_string(&original).unwrap();
        let restored: AiFeatures = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.chat, original.chat);
        assert_eq!(restored.smart_fill, original.smart_fill);
        assert_eq!(restored.command_gen, original.command_gen);
        assert_eq!(
            restored.natural_language_search,
            original.natural_language_search
        );
    }

    #[test]
    fn test_chat_message_serde_roundtrip() {
        let original = ChatMessage {
            role: "user".into(),
            content: "Hello".into(),
            created_at: "2024-01-01T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let restored: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.role, original.role);
        assert_eq!(restored.content, original.content);
        assert_eq!(restored.created_at, original.created_at);
    }

    #[test]
    fn test_conversation_serde_roundtrip_with_deleted_at() {
        let original = Conversation {
            id: "conv-1".into(),
            name: "Test Conv".into(),
            is_temporary: false,
            messages: vec![ChatMessage {
                role: "assistant".into(),
                content: "Hi".into(),
                created_at: "2024-01-01T00:00:00Z".into(),
            }],
            updated_at: "2024-06-01T12:00:00Z".into(),
            deleted_at: Some("2024-06-02T12:00:00Z".into()),
        };
        let json = serde_json::to_string(&original).unwrap();
        assert!(json.contains("deletedAt"));
        let restored: Conversation = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, original.id);
        assert_eq!(restored.name, original.name);
        assert_eq!(restored.is_temporary, original.is_temporary);
        assert_eq!(restored.messages.len(), original.messages.len());
        assert_eq!(restored.updated_at, original.updated_at);
        assert_eq!(restored.deleted_at, original.deleted_at);
    }

    #[test]
    fn test_conversation_serde_without_deleted_at() {
        let original = Conversation {
            id: "conv-2".into(),
            name: "Active Conv".into(),
            is_temporary: true,
            messages: vec![],
            updated_at: "2024-06-01T12:00:00Z".into(),
            deleted_at: None,
        };
        let json = serde_json::to_string(&original).unwrap();
        assert!(!json.contains("deletedAt"));
        let restored: Conversation = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.deleted_at, None);
    }

    // ── VaultStore-backed helpers ───────────────────────────────

    #[test]
    fn test_load_save_config() {
        let (vault, _dir) = setup_vault();
        let account_id = "test_account";

        // Loading before saving returns defaults
        let loaded = load_config(&vault, account_id).unwrap();
        assert!(loaded.providers.is_empty());
        assert_eq!(loaded.active_provider_id, None);
        assert!(!loaded.has_accepted_risk);
        assert!(loaded.include_system_prompt);

        // Save a custom config
        let config = LlmConfig {
            providers: vec![ProviderConfig {
                id: "p1".into(),
                name: "P1".into(),
                base_url: "http://localhost".into(),
                model: "m1".into(),
                is_enabled: true,
                is_built_in: false,
                api_type: ApiType::Anthropic,
                embedding_model: None,
            }],
            active_provider_id: Some("p1".into()),
            ai_features_enabled: AiFeatures::default(),
            has_accepted_risk: true,
            include_system_prompt: false,
            use_local_embedding: true,
            local_embed_model_id: Some("lid".into()),
        };
        save_config(&vault, account_id, &config).unwrap();

        // Load back and verify
        let loaded = load_config(&vault, account_id).unwrap();
        assert_eq!(loaded.providers.len(), 1);
        assert_eq!(loaded.providers[0].id, "p1");
        assert_eq!(loaded.providers[0].api_type, ApiType::Anthropic);
        assert_eq!(loaded.active_provider_id, Some("p1".into()));
        assert!(loaded.has_accepted_risk);
        assert!(!loaded.include_system_prompt);
        assert!(loaded.use_local_embedding);
        assert_eq!(loaded.local_embed_model_id, Some("lid".into()));
    }

    #[test]
    fn test_load_save_api_keys() {
        let (vault, _dir) = setup_vault();
        let account_id = "test_account";

        // Empty initially
        let keys = load_api_keys(&vault, account_id).unwrap();
        assert!(keys.is_empty());

        // Save a key
        save_api_key(&vault, account_id, "openai", "sk-abc123").unwrap();
        let keys = load_api_keys(&vault, account_id).unwrap();
        assert_eq!(keys.get("openai"), Some(&"sk-abc123".to_string()));

        // Save another key
        save_api_key(&vault, account_id, "anthropic", "sk-ant-xyz").unwrap();
        let keys = load_api_keys(&vault, account_id).unwrap();
        assert_eq!(keys.len(), 2);
        assert_eq!(keys.get("anthropic"), Some(&"sk-ant-xyz".to_string()));
    }

    #[test]
    fn test_load_save_conversations() {
        let (vault, _dir) = setup_vault();
        let account_id = "test_account";

        // Empty initially
        let convs = load_conversations(&vault, account_id).unwrap();
        assert!(convs.is_empty());

        let conversations = vec![
            Conversation {
                id: "conv-1".into(),
                name: "First".into(),
                is_temporary: false,
                messages: vec![ChatMessage {
                    role: "user".into(),
                    content: "Hi".into(),
                    created_at: "2024-01-01T00:00:00Z".into(),
                }],
                updated_at: "2024-01-01T00:00:00Z".into(),
                deleted_at: None,
            },
            Conversation {
                id: "conv-2".into(),
                name: "Second".into(),
                is_temporary: false,
                messages: vec![],
                updated_at: "2024-02-01T00:00:00Z".into(),
                deleted_at: Some("2024-03-01T00:00:00Z".into()),
            },
        ];

        save_conversations(&vault, account_id, &conversations).unwrap();

        let loaded = load_conversations(&vault, account_id).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].id, "conv-1");
        assert_eq!(loaded[0].deleted_at, None);
        assert_eq!(loaded[1].id, "conv-2");
        assert_eq!(
            loaded[1].deleted_at,
            Some("2024-03-01T00:00:00Z".into())
        );
    }
}
