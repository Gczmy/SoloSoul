//! LLM configuration commands (§26)
//! Multi-provider model with encrypted API key storage.
//! `llm_test_provider` and `llm_send_message` use reqwest for HTTP calls.

use serde::{Deserialize, Serialize};
use solosoul_vault::VaultStore;
use std::collections::HashMap;

/// 错误响应/正文预览的最大字符数。
pub const MAX_PREVIEW_CHARS: usize = 300;
/// 指南摘要的最大字节数。
pub const MAX_GUIDE_SUMMARY_BYTES: usize = 200;
/// 默认 LLM 输出 token 上限。
pub const DEFAULT_MAX_TOKENS: u32 = 4096;

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

pub fn default_true() -> bool {
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

pub fn default_providers() -> Vec<ProviderWithKey> {
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

pub fn load_config(vault: &VaultStore, account_id: &str) -> Result<LlmConfig, String> {
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

pub fn save_config(vault: &VaultStore, account_id: &str, config: &LlmConfig) -> Result<(), String> {
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

pub fn load_api_keys(
    vault: &VaultStore,
    account_id: &str,
) -> Result<HashMap<String, String>, String> {
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

pub fn save_api_key(
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

// ── Sub-modules ─────────────────────────────────────────────

pub mod chat_http;
pub mod conversation;
pub mod guide;
pub mod provider;
pub mod rag;
pub mod request;
pub mod stats;
pub mod stream;
#[cfg(test)]
mod tests;
pub mod unified_chat;

// Re-export all command functions so that `commands::llm::xxx` paths remain valid.
pub use chat_http::*;
pub use conversation::*;
pub use guide::*;
pub use provider::*;
pub use rag::*;
pub use stats::*;
pub use stream::*;
pub use unified_chat::*;
