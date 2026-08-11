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

    // ── Conversations ──────────────────────────────────────────────

    /// P004 懒迁移：旧版本会话存于 profile preferences 的 `llmConversations` blob。
    /// 首次访问时把 blob 中的全部会话写入行级表并清除 blob 键（幂等：迁移后
    /// 键已删，再次调用直接跳过）。GUI 与 CLI 共用同一实现。
    ///
    /// LWW 保护：对每条会话按 `updated_at` 比较，仅当 blob 数据比行级表现有
    /// 数据更新（或行级表无此 id）时才写入，避免无条件 upsert 覆盖 CLI/其他
    /// 端已写入的较新行（N005）。
    pub fn migrate_legacy_conversations(
        &self,
        vault: &VaultStore,
        account_id: &str,
    ) -> LlmResult<()> {
        let data = Self::load_profile_data(vault, account_id)?;
        let legacy: Option<Vec<Conversation>> = data
            .get("preferences")
            .and_then(|p| p.get("llmConversations"))
            .and_then(|v| serde_json::from_value(v.clone()).ok());
        let Some(convs) = legacy else { return Ok(()) };
        if convs.is_empty() {
            // 空数组也视为已迁移：清掉键避免每次重复空写。
            return self.clear_legacy_conversations(vault, account_id);
        }

        for mut c in convs {
            trim_conversation_messages(&mut c);
            // LWW：行级表已有更新或相同的数据则跳过，防止覆盖较新行。
            if let Some(raw) = vault
                .load_conversation(account_id, &c.id)
                .map_err(|e| e.to_string())?
            {
                if let Ok(existing) = serde_json::from_slice::<Conversation>(&raw) {
                    if compare_updated_at(&existing.updated_at, &c.updated_at)
                        != std::cmp::Ordering::Less
                    {
                        continue;
                    }
                }
            }
            let data = serde_json::to_vec(&c).map_err(|e| format!("Serialize: {e}"))?;
            vault
                .save_conversation(account_id, &c.id, &c.updated_at, &data)
                .map_err(|e| e.to_string())?;
        }
        self.clear_legacy_conversations(vault, account_id)
    }

    /// 清掉 profile preferences 中的旧 `llmConversations` blob 键。
    fn clear_legacy_conversations(&self, vault: &VaultStore, account_id: &str) -> LlmResult<()> {
        let mut data = Self::load_profile_data(vault, account_id)?;
        if let Some(prefs) = data.get_mut("preferences").and_then(|p| p.as_object_mut()) {
            prefs.remove("llmConversations");
            Self::save_profile_data(vault, account_id, &data)?;
        }
        Ok(())
    }

    /// Load all conversations for an account.
    ///
    /// P004: 会话改存 `llm_conversations` 行级表（不再存 profile preferences blob），
    /// 本方法委托 vault 行级读取，与 src-tauri 命令实现保持单一数据源。
    pub fn load_conversations(
        &self,
        vault: &VaultStore,
        account_id: &str,
    ) -> LlmResult<Vec<Conversation>> {
        self.migrate_legacy_conversations(vault, account_id)?;
        let rows = vault
            .list_conversations(account_id)
            .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .filter_map(|(_id, _updated, data)| serde_json::from_slice(&data).ok())
            .collect())
    }

    /// Save conversations（整批 upsert 到行级表）。
    pub fn save_conversations(
        &self,
        vault: &VaultStore,
        account_id: &str,
        conversations: &[Conversation],
    ) -> LlmResult<()> {
        for c in conversations {
            let data = serde_json::to_vec(c).map_err(|e| format!("Serialize: {e}"))?;
            vault
                .save_conversation(account_id, &c.id, &c.updated_at, &data)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
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

    /// Get a single conversation by ID（P004：单行读取，仅解密目标行）。
    pub fn get_conversation(
        &self,
        vault: &VaultStore,
        account_id: &str,
        conversation_id: &str,
    ) -> LlmResult<Option<Conversation>> {
        self.migrate_legacy_conversations(vault, account_id)?;
        let data = vault
            .load_conversation(account_id, conversation_id)
            .map_err(|e| e.to_string())?;
        Ok(data.and_then(|d| serde_json::from_slice(&d).ok()))
    }

    /// Save a single conversation (行级 upsert)。
    pub fn save_conversation(
        &self,
        vault: &VaultStore,
        account_id: &str,
        conversation: &Conversation,
    ) -> LlmResult<()> {
        let data = serde_json::to_vec(conversation).map_err(|e| format!("Serialize: {e}"))?;
        vault
            .save_conversation(
                account_id,
                &conversation.id,
                &conversation.updated_at,
                &data,
            )
            .map_err(|e| e.to_string())?;
        Ok(())
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

        let loaded = service
            .get_conversation(&vault, &account_id, "conv1")
            .unwrap()
            .unwrap();
        assert_eq!(loaded.name, "First chat");
        assert_eq!(loaded.messages.len(), 0);
    }

    #[test]
    fn test_stats() {
        let (_dir, vault, account_id) = setup_vault();
        let service = LlmService::new();

        let stats = service.load_stats(&vault, &account_id).unwrap();
        assert_eq!(stats.total_tokens, 0);

        let new_stats = LlmUsageStats {
            total_tokens: 1000,
            ..Default::default()
        };
        service.save_stats(&vault, &account_id, &new_stats).unwrap();
        let loaded = service.load_stats(&vault, &account_id).unwrap();
        assert_eq!(loaded.total_tokens, 1000);
    }

    #[test]
    fn test_migrate_legacy_conversations_lww() {
        let (_dir, vault, account_id) = setup_vault();
        let service = LlmService::new();

        // 构造旧 blob：preferences.llmConversations 含两条会话
        let legacy_blob = serde_json::json!([
            {
                "id": "c1",
                "name": "old c1",
                "isTemporary": false,
                "messages": [],
                "updatedAt": "2024-01-01T00:00:00Z",
                "deletedAt": null
            },
            {
                "id": "c2",
                "name": "old c2",
                "isTemporary": false,
                "messages": [],
                "updatedAt": "2024-01-02T00:00:00Z",
                "deletedAt": null
            }
        ]);
        let data = serde_json::json!({
            "preferences": {
                "llmConversations": legacy_blob
            }
        });
        LlmService::save_profile_data(&vault, &account_id, &data).unwrap();

        // 模拟 CLI/新端已写入更新的 c1（updated_at 比 blob 新）
        let newer = Conversation {
            id: "c1".into(),
            name: "newer c1".to_string(),
            is_temporary: false,
            messages: vec![],
            updated_at: "2024-03-01T00:00:00Z".to_string(),
            deleted_at: None,
        };
        service
            .save_conversation(&vault, &account_id, &newer)
            .unwrap();

        // 迁移：c1 已有更新数据应保留，c2 无行应写入
        service
            .migrate_legacy_conversations(&vault, &account_id)
            .unwrap();

        let c1 = service
            .get_conversation(&vault, &account_id, "c1")
            .unwrap()
            .unwrap();
        assert_eq!(c1.name, "newer c1", "LWW: 较新的已存在行不应被 blob 覆盖");
        assert_eq!(c1.updated_at, "2024-03-01T00:00:00Z");

        let c2 = service
            .get_conversation(&vault, &account_id, "c2")
            .unwrap()
            .unwrap();
        assert_eq!(c2.name, "old c2");

        // blob 键已清除：再次迁移幂等且无残留
        service
            .migrate_legacy_conversations(&vault, &account_id)
            .unwrap();
        let data = LlmService::load_profile_data(&vault, &account_id).unwrap();
        assert!(
            data.get("preferences")
                .and_then(|p| p.get("llmConversations"))
                .is_none(),
            "迁移后 blob 键应清除"
        );
    }
}

/// 单条对话的最大消息数量，超过此限时自动裁剪最早的消息（与 GUI 侧常量一致）。
const MAX_CONVERSATION_MESSAGES: usize = 500;

/// 裁剪单条对话的消息数量，防止数据无限增长。
fn trim_conversation_messages(conv: &mut Conversation) {
    if conv.messages.len() > MAX_CONVERSATION_MESSAGES {
        let excess = conv.messages.len() - MAX_CONVERSATION_MESSAGES;
        conv.messages.drain(..excess);
    }
}

/// 比较两个 RFC3339 时间字符串；无法解析时按字典序兜底（同格式下等价）。
fn compare_updated_at(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |s: &str| {
        chrono::DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.timestamp_millis())
            .ok()
    };
    match (parse(a), parse(b)) {
        (Some(x), Some(y)) => x.cmp(&y),
        _ => a.cmp(b),
    }
}
