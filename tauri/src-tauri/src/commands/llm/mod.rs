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

/// P011: 内置 provider 默认值唯一来源为 `solosoul_core::llm::service::LlmService::default_providers`。
/// 本层仅做 GUI 适配：追加 `builtin_` id 前缀（前端持久化的 `active_provider_id` 依赖该
/// 前缀，不可变更）与空的 `api_key` 字段。模型名/地址/embedding 等值全部来自 core 单一
/// 来源，杜绝两处默认值漂移。
/// 例外：Alibaba 的 core id 为 `dashscope`，GUI 历史持久化 id 为 `builtin_alibaba`
/// （`merge_providers_with_keys` 按 id 匹配已保存配置，改名会使存量配置失配），故显式映射。
pub fn default_providers() -> Vec<ProviderWithKey> {
    use solosoul_core::llm::service::LlmService;
    LlmService::default_providers()
        .into_iter()
        .map(|p| ProviderWithKey {
            id: match p.id.as_str() {
                "dashscope" => "builtin_alibaba".to_string(),
                other => format!("builtin_{}", other),
            },
            name: p.name,
            base_url: p.base_url,
            model: p.model,
            is_enabled: p.is_enabled,
            is_built_in: true,
            api_key: String::new(),
            api_type: p.api_type,
            embedding_model: p.embedding_model,
        })
        .collect()
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
pub mod contracts;
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
pub use chat_http::{llm_check_connection, llm_test_provider};
pub use conversation::{
    llm_get_conversation, llm_list_conversations, llm_list_trash, llm_permanent_delete,
    llm_rename_conversation, llm_restore_conversation, llm_save_conversation,
    llm_soft_delete_conversation,
};
#[cfg(test)]
pub(crate) use conversation::{load_conversations, now_iso, save_conversation};
pub use guide::{
    find_relevant_guides_internal, guide_load_content, guide_load_index, guide_search,
    load_guide_index, load_search_index_impl, resolve_language, resolve_title, resource_path,
    GuideCategoryMeta, GuideContent, GuideIndex, GuideIndexEntry, GuideTitle, SearchIndex,
    RESOURCE_DIR,
};
pub(crate) use provider::is_anthropic;
pub use provider::{
    llm_accept_risk, llm_delete_provider, llm_get_api_key, llm_get_config, llm_get_providers,
    llm_save_provider, llm_set_active_provider, llm_set_ai_features, llm_set_local_embedding,
    llm_set_system_prompt_switch,
};
pub use rag::{
    chunk_all_guides, compute_content_hash, guide_title_map, llm_check_embedding_available,
    llm_rebuild_guide_embeddings, llm_search_guide_chunks, mark_rebuilt, GuideChunk, RawChunk,
};
pub use stats::{
    estimate_tokens, llm_get_stats, llm_reset_stats, load_stats_from_vault, record_usage,
    record_usage_fallback, save_stats_to_vault, DailyUsage, LlmUsageStats, ModelUsage, TokenUsage,
    STATS_MAP,
};
pub use stream::{llm_send_message_stream, LlmStreamPayload};
pub(crate) use unified_chat::is_registered_provider_url;
pub use unified_chat::load_providers_with_keys;

// chat_http 的 Tauri 生成宏与函数一并保留。
pub use chat_http::{
    __cmd__llm_check_connection, __cmd__llm_test_provider,
    __tauri_command_name_llm_check_connection, __tauri_command_name_llm_test_provider,
};

// conversation 的 Tauri 生成宏与函数一并保留。
pub use conversation::{
    __cmd__llm_get_conversation, __cmd__llm_list_conversations, __cmd__llm_list_trash,
    __cmd__llm_permanent_delete, __cmd__llm_rename_conversation, __cmd__llm_restore_conversation,
    __cmd__llm_save_conversation, __cmd__llm_soft_delete_conversation,
    __tauri_command_name_llm_get_conversation, __tauri_command_name_llm_list_conversations,
    __tauri_command_name_llm_list_trash, __tauri_command_name_llm_permanent_delete,
    __tauri_command_name_llm_rename_conversation, __tauri_command_name_llm_restore_conversation,
    __tauri_command_name_llm_save_conversation, __tauri_command_name_llm_soft_delete_conversation,
};

// guide 的 Tauri 生成宏与函数一并保留。
pub use guide::{
    __cmd__guide_load_content, __cmd__guide_load_index, __cmd__guide_search,
    __tauri_command_name_guide_load_content, __tauri_command_name_guide_load_index,
    __tauri_command_name_guide_search,
};

// provider 的 Tauri 生成宏与函数一并保留。
pub use provider::{
    __cmd__llm_accept_risk, __cmd__llm_delete_provider, __cmd__llm_get_api_key,
    __cmd__llm_get_config, __cmd__llm_get_providers, __cmd__llm_save_provider,
    __cmd__llm_set_active_provider, __cmd__llm_set_ai_features, __cmd__llm_set_local_embedding,
    __cmd__llm_set_system_prompt_switch, __tauri_command_name_llm_accept_risk,
    __tauri_command_name_llm_delete_provider, __tauri_command_name_llm_get_api_key,
    __tauri_command_name_llm_get_config, __tauri_command_name_llm_get_providers,
    __tauri_command_name_llm_save_provider, __tauri_command_name_llm_set_active_provider,
    __tauri_command_name_llm_set_ai_features, __tauri_command_name_llm_set_local_embedding,
    __tauri_command_name_llm_set_system_prompt_switch,
};

// rag 的 Tauri 生成宏与函数一并保留。
pub use rag::{
    __cmd__llm_check_embedding_available, __cmd__llm_rebuild_guide_embeddings,
    __cmd__llm_search_guide_chunks, __tauri_command_name_llm_check_embedding_available,
    __tauri_command_name_llm_rebuild_guide_embeddings,
    __tauri_command_name_llm_search_guide_chunks,
};

// stats 的 Tauri 生成宏与函数一并保留。
pub use stats::{
    __cmd__llm_get_stats, __cmd__llm_reset_stats, __tauri_command_name_llm_get_stats,
    __tauri_command_name_llm_reset_stats,
};

// stream 的 Tauri 生成宏与函数一并保留。
pub use stream::{__cmd__llm_send_message_stream, __tauri_command_name_llm_send_message_stream};
