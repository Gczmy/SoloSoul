//! LLM service for configuration, conversation, and usage statistics.
//!
//! All methods take a `&VaultStore` reference so callers control vault lifecycle.

use solosoul_vault::{Profile, VaultStore};

use crate::llm::config::{
    AiFeatures, Conversation, ConversationSummary, LlmConfig, LlmUsageStats, ProviderConfig,
    ProviderWithKey,
};

/// Result type for LlmService operations.
pub type LlmResult<T> = Result<T, String>;

/// Service for managing LLM configuration, conversations, and usage statistics.
pub struct LlmService;

impl Default for LlmService {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmService {
    /// Create a new LlmService.
    pub fn new() -> Self {
        Self
    }

    // ── helpers ────────────────────────────────────────────────────

    /// Load the root profile data JSON for an account.
    fn load_profile_data(vault: &VaultStore, account_id: &str) -> LlmResult<serde_json::Value> {
        let profile = vault
            .load_profile(account_id)
            .map_err(|e| format!("Failed to load profile: {}", e))?
            .unwrap_or_else(|| Profile::new_with_id(account_id, account_id, Vec::new()));
        if profile.data.is_empty() {
            Ok(serde_json::Value::Object(serde_json::Map::new()))
        } else {
            serde_json::from_slice(&profile.data).map_err(|e| format!("Parse profile: {}", e))
        }
    }

    /// Save JSON back to profile data.
    fn save_profile_data(
        vault: &VaultStore,
        account_id: &str,
        data: &serde_json::Value,
    ) -> LlmResult<()> {
        let mut profile = vault
            .load_profile(account_id)
            .map_err(|e| format!("Failed to load profile: {}", e))?
            .unwrap_or_else(|| Profile::new_with_id(account_id, account_id, Vec::new()));
        profile.data = serde_json::to_vec(data).map_err(|e| format!("Serialize profile: {}", e))?;
        profile.updated_at = chrono::Utc::now();
        profile.version += 1;
        vault
            .save_profile(&profile)
            .map_err(|e| format!("Save profile: {}", e))
    }

    /// Get or create the preferences sub-object within profile data.
    fn prefs_mut(
        data: &mut serde_json::Value,
    ) -> LlmResult<&mut serde_json::Map<String, serde_json::Value>> {
        let obj = data
            .as_object_mut()
            .ok_or_else(|| "profile data must be object".to_string())?;
        obj.entry("preferences".to_string())
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))
            .as_object_mut()
            .ok_or_else(|| "preferences must be object".to_string())
    }

    // ── LLM Configuration ─────────────────────────────────────────

    /// Load the LlmConfig for an account.
    pub fn load_config(&self, vault: &VaultStore, account_id: &str) -> LlmResult<LlmConfig> {
        let data = Self::load_profile_data(vault, account_id)?;
        let config = data
            .get("preferences")
            .and_then(|p| p.get("llmConfig"))
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_else(|| LlmConfig {
                providers: Self::default_providers(),
                active_provider_id: None,
                ai_features_enabled: AiFeatures::default(),
                has_accepted_risk: false,
                include_system_prompt: true,
                use_local_embedding: false,
                local_embed_model_id: None,
            });
        Ok(config)
    }

    /// Save the LlmConfig for an account.
    pub fn save_config(
        &self,
        vault: &VaultStore,
        account_id: &str,
        config: &LlmConfig,
    ) -> LlmResult<()> {
        let mut data = Self::load_profile_data(vault, account_id)?;
        let prefs = Self::prefs_mut(&mut data)?;
        prefs.insert(
            "llmConfig".to_string(),
            serde_json::to_value(config).map_err(|e| e.to_string())?,
        );
        Self::save_profile_data(vault, account_id, &data)
    }

    /// List providers (API keys masked).
    pub fn get_providers(
        &self,
        vault: &VaultStore,
        account_id: &str,
    ) -> LlmResult<Vec<ProviderConfig>> {
        let config = self.load_config(vault, account_id)?;
        Ok(config.providers)
    }

    /// Get provider with its API key (decrypted).
    pub fn get_provider_with_key(
        &self,
        vault: &VaultStore,
        account_id: &str,
        provider_id: &str,
    ) -> LlmResult<Option<ProviderWithKey>> {
        let data = Self::load_profile_data(vault, account_id)?;
        let keys: std::collections::HashMap<String, String> = data
            .get("preferences")
            .and_then(|p| p.get("llmApiKeys"))
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        let config = self.load_config(vault, account_id)?;
        Ok(config
            .providers
            .iter()
            .find(|p| p.id == provider_id)
            .map(|p| ProviderWithKey {
                id: p.id.clone(),
                name: p.name.clone(),
                base_url: p.base_url.clone(),
                model: p.model.clone(),
                is_enabled: p.is_enabled,
                is_built_in: p.is_built_in,
                api_key: keys.get(&p.id).cloned().unwrap_or_default(),
                api_type: p.api_type.clone(),
                embedding_model: p.embedding_model.clone(),
            }))
    }

    /// Save a provider (with API key stored separately).
    pub fn save_provider(
        &self,
        vault: &VaultStore,
        account_id: &str,
        provider: &ProviderWithKey,
    ) -> LlmResult<()> {
        let mut config = self.load_config(vault, account_id)?;
        let provider_config = ProviderConfig {
            id: provider.id.clone(),
            name: provider.name.clone(),
            base_url: provider.base_url.clone(),
            model: provider.model.clone(),
            is_enabled: provider.is_enabled,
            is_built_in: provider.is_built_in,
            api_type: provider.api_type.clone(),
            embedding_model: provider.embedding_model.clone(),
        };

        // Upsert provider in list
        if let Some(existing) = config.providers.iter_mut().find(|p| p.id == provider.id) {
            *existing = provider_config;
        } else {
            config.providers.push(provider_config);
        }

        // Save API key separately
        let mut data = Self::load_profile_data(vault, account_id)?;
        let prefs = Self::prefs_mut(&mut data)?;
        let mut keys: std::collections::HashMap<String, String> = prefs
            .get("llmApiKeys")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        keys.insert(provider.id.clone(), provider.api_key.clone());
        prefs.insert(
            "llmApiKeys".to_string(),
            serde_json::to_value(&keys).map_err(|e| e.to_string())?,
        );
        prefs.insert(
            "llmConfig".to_string(),
            serde_json::to_value(&config).map_err(|e| e.to_string())?,
        );
        Self::save_profile_data(vault, account_id, &data)
    }

    /// Set the active provider by ID.
    pub fn set_active_provider(
        &self,
        vault: &VaultStore,
        account_id: &str,
        provider_id: Option<&str>,
    ) -> LlmResult<()> {
        let mut config = self.load_config(vault, account_id)?;
        config.active_provider_id = provider_id.map(|s| s.to_string());
        self.save_config(vault, account_id, &config)
    }

    /// Set AI feature toggles.
    pub fn set_ai_features(
        &self,
        vault: &VaultStore,
        account_id: &str,
        features: &AiFeatures,
    ) -> LlmResult<()> {
        let mut config = self.load_config(vault, account_id)?;
        config.ai_features_enabled = features.clone();
        self.save_config(vault, account_id, &config)
    }

    /// Toggle system prompt inclusion.
    pub fn set_system_prompt_switch(
        &self,
        vault: &VaultStore,
        account_id: &str,
        include: bool,
    ) -> LlmResult<()> {
        let mut config = self.load_config(vault, account_id)?;
        config.include_system_prompt = include;
        self.save_config(vault, account_id, &config)
    }

    /// Set local embedding preferences.
    pub fn set_local_embedding(
        &self,
        vault: &VaultStore,
        account_id: &str,
        use_local: bool,
        model_id: Option<&str>,
    ) -> LlmResult<()> {
        let mut config = self.load_config(vault, account_id)?;
        config.use_local_embedding = use_local;
        config.local_embed_model_id = model_id.map(|s| s.to_string());
        self.save_config(vault, account_id, &config)
    }

    /// Mark risk as accepted.
    pub fn accept_risk(&self, vault: &VaultStore, account_id: &str) -> LlmResult<()> {
        let mut config = self.load_config(vault, account_id)?;
        config.has_accepted_risk = true;
        self.save_config(vault, account_id, &config)
    }

    /// Delete a provider.
    pub fn delete_provider(
        &self,
        vault: &VaultStore,
        account_id: &str,
        provider_id: &str,
    ) -> LlmResult<()> {
        let mut config = self.load_config(vault, account_id)?;
        config.providers.retain(|p| p.id != provider_id);
        if config.active_provider_id.as_deref() == Some(provider_id) {
            config.active_provider_id = None;
        }
        // Also clean up API key
        let mut data = Self::load_profile_data(vault, account_id)?;
        let prefs = Self::prefs_mut(&mut data)?;
        if let Some(keys_val) = prefs.get_mut("llmApiKeys") {
            if let Some(keys_obj) = keys_val.as_object_mut() {
                keys_obj.remove(provider_id);
            }
        }
        prefs.insert(
            "llmConfig".to_string(),
            serde_json::to_value(&config).map_err(|e| e.to_string())?,
        );
        Self::save_profile_data(vault, account_id, &data)
    }

    // ── Conversations ──────────────────────────────────────────────

    /// Load all conversations for an account.
    pub fn load_conversations(
        &self,
        vault: &VaultStore,
        account_id: &str,
    ) -> LlmResult<Vec<Conversation>> {
        let data = Self::load_profile_data(vault, account_id)?;
        Ok(data
            .get("preferences")
            .and_then(|p| p.get("llmConversations"))
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default())
    }

    /// Save conversations.
    pub fn save_conversations(
        &self,
        vault: &VaultStore,
        account_id: &str,
        conversations: &[Conversation],
    ) -> LlmResult<()> {
        let mut data = Self::load_profile_data(vault, account_id)?;
        let prefs = Self::prefs_mut(&mut data)?;
        prefs.insert(
            "llmConversations".to_string(),
            serde_json::to_value(conversations).map_err(|e| e.to_string())?,
        );
        Self::save_profile_data(vault, account_id, &data)
    }

    /// List conversation summaries (non-temporary, non-deleted).
    pub fn list_conversations(
        &self,
        vault: &VaultStore,
        account_id: &str,
    ) -> LlmResult<Vec<ConversationSummary>> {
        let conversations = self.load_conversations(vault, account_id)?;
        let mut summaries: Vec<ConversationSummary> = conversations
            .iter()
            .filter(|c| !c.is_temporary && c.deleted_at.is_none())
            .map(|c| ConversationSummary {
                id: c.id.clone(),
                name: c.name.clone(),
                updated_at: c.updated_at.clone(),
                message_count: c.messages.len(),
                deleted_at: c.deleted_at.clone(),
            })
            .collect();
        summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(summaries)
    }

    /// Get a single conversation by ID.
    pub fn get_conversation(
        &self,
        vault: &VaultStore,
        account_id: &str,
        conversation_id: &str,
    ) -> LlmResult<Option<Conversation>> {
        let conversations = self.load_conversations(vault, account_id)?;
        Ok(conversations.into_iter().find(|c| c.id == conversation_id))
    }

    /// Save a single conversation (upsert).
    pub fn save_conversation(
        &self,
        vault: &VaultStore,
        account_id: &str,
        conversation: &Conversation,
    ) -> LlmResult<()> {
        let mut conversations = self.load_conversations(vault, account_id)?;
        if let Some(existing) = conversations.iter_mut().find(|c| c.id == conversation.id) {
            *existing = conversation.clone();
        } else {
            conversations.push(conversation.clone());
        }
        self.save_conversations(vault, account_id, &conversations)
    }

    /// Soft-delete a conversation.
    pub fn soft_delete_conversation(
        &self,
        vault: &VaultStore,
        account_id: &str,
        conversation_id: &str,
    ) -> LlmResult<()> {
        let mut conversations = self.load_conversations(vault, account_id)?;
        if let Some(conv) = conversations.iter_mut().find(|c| c.id == conversation_id) {
            conv.deleted_at = Some(chrono::Utc::now().to_rfc3339());
        }
        self.save_conversations(vault, account_id, &conversations)
    }

    /// Restore a soft-deleted conversation.
    pub fn restore_conversation(
        &self,
        vault: &VaultStore,
        account_id: &str,
        conversation_id: &str,
    ) -> LlmResult<()> {
        let mut conversations = self.load_conversations(vault, account_id)?;
        if let Some(conv) = conversations.iter_mut().find(|c| c.id == conversation_id) {
            conv.deleted_at = None;
        }
        self.save_conversations(vault, account_id, &conversations)
    }

    /// Permanently delete a conversation.
    pub fn delete_conversation(
        &self,
        vault: &VaultStore,
        account_id: &str,
        conversation_id: &str,
    ) -> LlmResult<()> {
        let mut conversations = self.load_conversations(vault, account_id)?;
        conversations.retain(|c| c.id != conversation_id);
        self.save_conversations(vault, account_id, &conversations)
    }

    /// Rename a conversation.
    pub fn rename_conversation(
        &self,
        vault: &VaultStore,
        account_id: &str,
        conversation_id: &str,
        name: &str,
    ) -> LlmResult<()> {
        let mut conversations = self.load_conversations(vault, account_id)?;
        if let Some(conv) = conversations.iter_mut().find(|c| c.id == conversation_id) {
            conv.name = name.to_string();
        }
        self.save_conversations(vault, account_id, &conversations)
    }

    // ── Stats ──────────────────────────────────────────────────────

    /// Load usage stats for an account.
    pub fn load_stats(&self, vault: &VaultStore, account_id: &str) -> LlmResult<LlmUsageStats> {
        let data = Self::load_profile_data(vault, account_id)?;
        Ok(data
            .get("preferences")
            .and_then(|p| p.get("llmStats"))
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default())
    }

    /// Save usage stats for an account.
    pub fn save_stats(
        &self,
        vault: &VaultStore,
        account_id: &str,
        stats: &LlmUsageStats,
    ) -> LlmResult<()> {
        let mut data = Self::load_profile_data(vault, account_id)?;
        let prefs = Self::prefs_mut(&mut data)?;
        prefs.insert(
            "llmStats".to_string(),
            serde_json::to_value(stats).map_err(|e| e.to_string())?,
        );
        Self::save_profile_data(vault, account_id, &data)
    }

    /// Reset usage stats for an account.
    pub fn reset_stats(&self, vault: &VaultStore, account_id: &str) -> LlmResult<()> {
        self.save_stats(vault, account_id, &LlmUsageStats::default())
    }

    /// Truncate a message to a short name (first 30 chars).
    fn truncate_for_name(message: &str) -> String {
        let cleaned: String = message.chars().take(30).collect();
        let trimmed = cleaned.trim();
        if trimmed.len() < message.len() {
            format!("{}…", trimmed)
        } else {
            trimmed.to_string()
        }
    }

    fn default_providers() -> Vec<ProviderConfig> {
        use crate::llm::config::ApiType;
        vec![
            ProviderConfig {
                id: "openai".into(),
                name: "OpenAI".into(),
                base_url: "https://api.openai.com/v1".into(),
                model: "gpt-4o".into(),
                is_enabled: false,
                is_built_in: true,
                api_type: ApiType::OpenAI,
                embedding_model: Some("text-embedding-3-small".into()),
            },
            ProviderConfig {
                id: "anthropic".into(),
                name: "Anthropic".into(),
                base_url: "https://api.anthropic.com/v1".into(),
                model: "claude-sonnet-4-20250514".into(),
                is_enabled: false,
                is_built_in: true,
                api_type: ApiType::Anthropic,
                embedding_model: None,
            },
            ProviderConfig {
                id: "ollama".into(),
                name: "Ollama".into(),
                base_url: "http://localhost:11434/v1".into(),
                model: "llama3.2".into(),
                is_enabled: false,
                is_built_in: true,
                api_type: ApiType::OpenAI,
                embedding_model: Some("nomic-embed-text".into()),
            },
            ProviderConfig {
                id: "deepseek".into(),
                name: "DeepSeek".into(),
                base_url: "https://api.deepseek.com/v1".into(),
                model: "deepseek-chat".into(),
                is_enabled: false,
                is_built_in: true,
                api_type: ApiType::OpenAI,
                embedding_model: None,
            },
            ProviderConfig {
                id: "dashscope".into(),
                name: "Alibaba Cloud".into(),
                base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1".into(),
                model: "qwen-plus".into(),
                is_enabled: false,
                is_built_in: true,
                api_type: ApiType::OpenAI,
                embedding_model: None,
            },
        ]
    }

    // ── Chat / Send ───────────────────────────────────────────────

    /// Send a chat message (non-streaming) and return the full response text.
    pub fn send_message(
        &self,
        vault: &VaultStore,
        account_id: &str,
        conversation_id: Option<&str>,
        message: &str,
    ) -> LlmResult<String> {
        let config = self.load_config(vault, account_id)?;
        let provider_id = config
            .active_provider_id
            .as_ref()
            .ok_or("No active provider configured".to_string())?;
        let provider = self
            .get_provider_with_key(vault, account_id, provider_id)?
            .ok_or("Active provider not found".to_string())?;

        let mut messages: Vec<serde_json::Value> = Vec::new();

        // System prompt
        if config.include_system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": "You are a helpful assistant integrated into SoloSoul, a local-first personal data vault. Answer concisely and accurately."
            }));
        }

        // Load conversation history
        if let Some(conv_id) = conversation_id {
            if let Some(conv) = self.get_conversation(vault, account_id, conv_id)? {
                for msg in &conv.messages {
                    messages.push(serde_json::json!({
                        "role": msg.role,
                        "content": msg.content
                    }));
                }
            }
        }

        // Current user message
        messages.push(serde_json::json!({
            "role": "user",
            "content": message
        }));

        let response = crate::llm::client::send_chat(
            &provider.base_url,
            &provider.api_key,
            &provider.model,
            &provider.api_type,
            &messages,
        )?;

        // Save to conversation
        self.save_to_conversation(vault, account_id, conversation_id, message, &response)?;

        // Record usage
        let prompt_chars: u64 = messages.iter().map(|m| m.to_string().len() as u64).sum();
        let completion_chars: u64 = response.len() as u64;
        self.record_usage_stats(
            vault,
            account_id,
            &provider.name,
            &provider.model,
            prompt_chars,
            completion_chars,
        )?;

        Ok(response)
    }

    /// Send a streaming chat message, calling on_event for each stream event.
    /// This method blocks until the stream completes.
    pub fn send_message_stream(
        &self,
        vault: &VaultStore,
        account_id: &str,
        conversation_id: Option<&str>,
        message: &str,
        on_event: &dyn Fn(crate::llm::client::LlmStreamEvent),
    ) -> LlmResult<()> {
        use crate::llm::client::LlmStreamEvent;

        let config = self.load_config(vault, account_id)?;
        let provider_id = config
            .active_provider_id
            .as_ref()
            .ok_or("No active provider configured".to_string())?;
        let provider = self
            .get_provider_with_key(vault, account_id, provider_id)?
            .ok_or("Active provider not found".to_string())?;

        let mut messages: Vec<serde_json::Value> = Vec::new();

        if config.include_system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": "You are a helpful assistant integrated into SoloSoul, a local-first personal data vault. Answer concisely and accurately."
            }));
        }

        if let Some(conv_id) = conversation_id {
            if let Some(conv) = self.get_conversation(vault, account_id, conv_id)? {
                for msg in &conv.messages {
                    messages.push(serde_json::json!({
                        "role": msg.role,
                        "content": msg.content
                    }));
                }
            }
        }

        messages.push(serde_json::json!({
            "role": "user",
            "content": message
        }));

        let prompt_chars: u64 = messages.iter().map(|m| m.to_string().len() as u64).sum();

        // Collect the full response from streaming events
        let full_response = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let final_tokens = std::sync::Arc::new(std::sync::Mutex::new((0u64, 0u64)));
        let response_clone = full_response.clone();
        let tokens_clone = final_tokens.clone();

        let provider_name = provider.name.clone();
        let provider_model = provider.model.clone();
        let account_id_owned = account_id.to_string();
        crate::llm::client::send_chat_stream(
            &provider.base_url,
            &provider.api_key,
            &provider.model,
            &provider.api_type,
            &messages,
            &|event| match event {
                LlmStreamEvent::Chunk { content } => {
                    if let Ok(mut resp) = response_clone.lock() {
                        resp.push_str(&content);
                    }
                    on_event(LlmStreamEvent::Chunk { content });
                }
                LlmStreamEvent::Done {
                    prompt_tokens,
                    completion_tokens,
                } => {
                    if let Ok(mut t) = tokens_clone.lock() {
                        *t = (prompt_tokens, completion_tokens);
                    }
                }
                LlmStreamEvent::Error { .. } => {}
            },
        )?;

        let response_text = full_response.lock().map_err(|e| e.to_string())?.clone();
        let (pt, ct) = *final_tokens.lock().map_err(|e| e.to_string())?;

        // Save conversation
        self.save_to_conversation(vault, account_id, conversation_id, message, &response_text)?;

        // Record usage
        let p_tokens = if pt > 0 { pt } else { prompt_chars };
        let c_tokens = if ct > 0 {
            ct
        } else {
            response_text.len() as u64
        };
        self.record_usage_stats(
            vault,
            &account_id_owned,
            &provider_name,
            &provider_model,
            p_tokens,
            c_tokens,
        )?;

        on_event(LlmStreamEvent::Done {
            prompt_tokens: p_tokens,
            completion_tokens: c_tokens,
        });

        Ok(())
    }

    /// Save a message exchange to a conversation.
    fn save_to_conversation(
        &self,
        vault: &VaultStore,
        account_id: &str,
        conversation_id: Option<&str>,
        user_message: &str,
        assistant_response: &str,
    ) -> LlmResult<()> {
        use crate::llm::config::{ChatMessage, Conversation};

        let now = chrono::Utc::now().to_rfc3339();
        let conv_id = conversation_id
            .map(|s| s.to_string())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let mut conversation = self
            .get_conversation(vault, account_id, &conv_id)?
            .unwrap_or_else(|| Conversation {
                id: conv_id.clone(),
                name: Self::truncate_for_name(user_message),
                is_temporary: false,
                messages: vec![],
                updated_at: now.clone(),
                deleted_at: None,
            });

        conversation.messages.push(ChatMessage {
            role: "user".to_string(),
            content: user_message.to_string(),
            created_at: now,
        });
        conversation.messages.push(ChatMessage {
            role: "assistant".to_string(),
            content: assistant_response.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        });
        conversation.updated_at = chrono::Utc::now().to_rfc3339();

        self.save_conversation(vault, account_id, &conversation)
    }

    /// Record usage statistics.
    fn record_usage_stats(
        &self,
        vault: &VaultStore,
        account_id: &str,
        provider_name: &str,
        model: &str,
        prompt_tokens: u64,
        completion_tokens: u64,
    ) -> LlmResult<()> {
        use crate::llm::config::{DailyUsage, ModelUsage};

        let mut stats = self.load_stats(vault, account_id)?;
        let total = prompt_tokens + completion_tokens;
        stats.usage_count += 1;
        stats.prompt_tokens += prompt_tokens;
        stats.completion_tokens += completion_tokens;
        stats.total_tokens += total;

        // Per-model stats
        let now_iso = chrono::Utc::now().to_rfc3339();
        if let Some(existing) = stats
            .per_model_stats
            .iter_mut()
            .find(|m| m.model == model && m.provider == provider_name)
        {
            existing.count += 1;
            existing.tokens += total;
            existing.prompt_tokens += prompt_tokens;
            existing.completion_tokens += completion_tokens;
            existing.last_used_time = Some(now_iso);
        } else {
            stats.per_model_stats.push(ModelUsage {
                model: model.to_string(),
                provider: provider_name.to_string(),
                count: 1,
                tokens: total,
                prompt_tokens,
                completion_tokens,
                last_used_time: Some(now_iso),
            });
        }

        // Daily stats
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let model_key = format!("{}/{}", provider_name, model);
        if let Some(daily) = stats.daily_stats.iter_mut().find(|d| d.date == today) {
            daily.count += 1;
            daily.tokens += total;
            *daily.per_model_tokens.entry(model_key).or_insert(0) += total;
        } else {
            let mut daily = DailyUsage {
                date: today,
                count: 1,
                tokens: total,
                per_model_tokens: std::collections::HashMap::new(),
            };
            daily.per_model_tokens.insert(model_key, total);
            stats.daily_stats.push(daily);
        }

        self.save_stats(vault, account_id, &stats)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::config::ApiType;

    fn setup_vault() -> (tempfile::TempDir, VaultStore, String) {
        let dir = tempfile::TempDir::new().unwrap();
        let config = solosoul_vault::VaultConfig::new("test", dir.path().to_path_buf())
            .with_data_key([0x42u8; 32]);
        let vault = VaultStore::open(config).unwrap();
        let account_id = "test_account";
        // Initialize profile
        vault
            .save_profile(&Profile::new_with_id(account_id, account_id, Vec::new()))
            .unwrap();
        (dir, vault, account_id.to_string())
    }

    #[test]
    fn test_load_default_config() {
        let (_dir, vault, account_id) = setup_vault();
        let service = LlmService::new();
        let config = service.load_config(&vault, &account_id).unwrap();
        assert_eq!(config.providers.len(), 5); // default providers
        assert!(config.active_provider_id.is_none());
        assert!(!config.has_accepted_risk);
        assert!(config.include_system_prompt);
    }

    #[test]
    fn test_save_and_load_config() {
        let (_dir, vault, account_id) = setup_vault();
        let service = LlmService::new();
        let mut config = service.load_config(&vault, &account_id).unwrap();
        config.has_accepted_risk = true;
        config.active_provider_id = Some("openai".into());
        service.save_config(&vault, &account_id, &config).unwrap();

        let reloaded = service.load_config(&vault, &account_id).unwrap();
        assert!(reloaded.has_accepted_risk);
        assert_eq!(reloaded.active_provider_id, Some("openai".into()));
    }

    #[test]
    fn test_save_and_get_provider() {
        let (_dir, vault, account_id) = setup_vault();
        let service = LlmService::new();
        let provider = ProviderWithKey {
            id: "custom".into(),
            name: "Custom".into(),
            base_url: "https://custom.api/v1".into(),
            model: "custom-model".into(),
            is_enabled: true,
            is_built_in: false,
            api_key: "sk-test".into(),
            api_type: ApiType::OpenAI,
            embedding_model: None,
        };
        service
            .save_provider(&vault, &account_id, &provider)
            .unwrap();

        let with_key = service
            .get_provider_with_key(&vault, &account_id, "custom")
            .unwrap()
            .unwrap();
        assert_eq!(with_key.api_key, "sk-test");
    }

    #[test]
    fn test_conversations_crud() {
        let (_dir, vault, account_id) = setup_vault();
        let service = LlmService::new();

        let conv = Conversation {
            id: "conv1".into(),
            name: "First chat".into(),
            is_temporary: false,
            messages: vec![],
            updated_at: "2024-01-01T00:00:00Z".into(),
            deleted_at: None,
        };
        service
            .save_conversation(&vault, &account_id, &conv)
            .unwrap();

        let summaries = service.list_conversations(&vault, &account_id).unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, "conv1");

        let loaded = service
            .get_conversation(&vault, &account_id, "conv1")
            .unwrap()
            .unwrap();
        assert_eq!(loaded.name, "First chat");

        service
            .soft_delete_conversation(&vault, &account_id, "conv1")
            .unwrap();
        let after_delete = service.list_conversations(&vault, &account_id).unwrap();
        assert!(after_delete.is_empty());
    }

    #[test]
    fn test_stats() {
        let (_dir, vault, account_id) = setup_vault();
        let service = LlmService::new();

        let stats = service.load_stats(&vault, &account_id).unwrap();
        assert_eq!(stats.total_tokens, 0);

        let mut new_stats = LlmUsageStats {
            total_tokens: 1000,
            ..Default::default()
        };
        service.save_stats(&vault, &account_id, &new_stats).unwrap();
        let loaded = service.load_stats(&vault, &account_id).unwrap();
        assert_eq!(loaded.total_tokens, 1000);

        new_stats.total_tokens = 0;
        service.reset_stats(&vault, &account_id).unwrap();
        let reset = service.load_stats(&vault, &account_id).unwrap();
        assert_eq!(reset.total_tokens, 0);
    }
}
