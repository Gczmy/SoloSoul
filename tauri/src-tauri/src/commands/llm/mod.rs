//! LLM configuration commands (§26)
//! Multi-provider model with encrypted API key storage.
//! `llm_test_provider` and `llm_send_message` use reqwest for HTTP calls.

use crate::services::profile_prefs::update_profile_prefs;
use solosoul_vault::VaultStore;
use std::collections::HashMap;

/// 错误响应/正文预览的最大字符数。
pub const MAX_PREVIEW_CHARS: usize = 300;
/// 指南摘要的最大字节数。
pub const MAX_GUIDE_SUMMARY_BYTES: usize = 200;
/// 默认 LLM 输出 token 上限。
pub const DEFAULT_MAX_TOKENS: u32 = 4096;

// ── Data models ─────────────────────────────────────────────
// P137: 类型定义统一复用 `solosoul_core::llm::config`（唯一真理来源），
// 消除跨 crate 重复（原两份 8 结构体定义易漂移）。
pub use solosoul_core::llm::config::{
    AiFeatures, ApiType, ChatMessage, Conversation, ConversationSummary, LlmConfig, ProviderConfig,
    ProviderWithKey,
};

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
    update_profile_prefs(vault, account_id, |prefs| {
        prefs.insert(
            "llmConfig".to_string(),
            serde_json::to_value(config).map_err(|e| e.to_string())?,
        );
        Ok(())
    })
}

/// P019: 合并默认 provider 与已保存配置（含解密密钥注入）——provider 列表命令与
/// 聊天内部链路共用同一合并语义，消除跨文件复制漂移。
/// 返回**真实密钥**；需掩码的调用方（如 `llm_get_providers`）自行替换。
pub fn merge_providers_with_keys(
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
    update_profile_prefs(vault, account_id, |prefs| {
        let mut keys: HashMap<String, String> = prefs
            .get("llmApiKeys")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        keys.insert(provider_id.to_string(), api_key.to_string());
        prefs.insert(
            "llmApiKeys".to_string(),
            serde_json::to_value(&keys).map_err(|e| e.to_string())?,
        );
        Ok(())
    })
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
