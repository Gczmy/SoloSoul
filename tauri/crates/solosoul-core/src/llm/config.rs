//! LLM configuration and conversation data models.

use serde::{Deserialize, Serialize};

/// Supported LLM API types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ApiType {
    #[default]
    OpenAI,
    Anthropic,
}

/// A configured LLM provider (without sensitive API key).
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

/// A provider configuration that includes the API key (for internal use).
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

/// AI feature toggles.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AiFeatures {
    pub chat: bool,
    pub smart_fill: bool,
    pub command_gen: bool,
    pub natural_language_search: bool,
}

fn default_true() -> bool {
    true
}

/// Full LLM configuration stored in profile preferences.
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

/// A single chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub created_at: String,
}

/// A full conversation with all messages.
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

/// Lightweight conversation summary for listing.
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

/// Aggregated LLM usage statistics.
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

/// Per-model usage statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsage {
    pub model: String,
    pub provider: String,
    pub count: u64,
    pub tokens: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub last_used_time: Option<String>,
}

/// Daily usage statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyUsage {
    pub date: String,
    pub count: u64,
    pub tokens: u64,
    pub per_model_tokens: std::collections::HashMap<String, u64>,
}

impl LlmConfig {
    /// Get the active provider config, if any.
    pub fn active_provider(&self) -> Option<&ProviderConfig> {
        self.active_provider_id
            .as_ref()
            .and_then(|id| self.providers.iter().find(|p| p.id == *id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_config_serde_roundtrip() {
        let config = LlmConfig {
            providers: vec![],
            active_provider_id: None,
            ai_features_enabled: AiFeatures::default(),
            has_accepted_risk: false,
            include_system_prompt: true,
            use_local_embedding: false,
            local_embed_model_id: None,
        };
        let json = serde_json::to_string(&config).unwrap();
        let restored: LlmConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.providers.len(), 0);
        assert!(restored.include_system_prompt);
    }

    #[test]
    fn test_api_type_default() {
        let api_type: ApiType = serde_json::from_str("\"openAI\"").unwrap();
        assert_eq!(api_type, ApiType::OpenAI);
    }

    #[test]
    fn test_conversation_serde_roundtrip() {
        let conv = Conversation {
            id: "c1".into(),
            name: "Test".into(),
            is_temporary: false,
            messages: vec![],
            updated_at: "2024-01-01T00:00:00Z".into(),
            deleted_at: None,
        };
        let json = serde_json::to_string(&conv).unwrap();
        let restored: Conversation = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "c1");
        assert!(restored.deleted_at.is_none());
    }
}
