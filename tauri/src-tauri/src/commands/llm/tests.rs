use super::*;

use solosoul_vault::{VaultConfig, VaultStore};
use tempfile::TempDir;

fn setup_vault() -> (VaultStore, TempDir) {
    let dir = TempDir::new().unwrap();
    let config =
        VaultConfig::new("test_account", dir.path().to_path_buf()).with_data_key([0x42u8; 32]);
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
    assert_eq!(
        serde_json::to_string(&ApiType::OpenAI).unwrap(),
        "\"openAI\""
    );
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
    assert_eq!(
        restored.include_system_prompt,
        original.include_system_prompt
    );
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
    assert_eq!(loaded[1].deleted_at, Some("2024-03-01T00:00:00Z".into()));
}

#[test]
fn guide_index_loads_successfully() {
    let index = load_guide_index().expect("load_guide_index should succeed");
    assert!(
        !index.guides.is_empty(),
        "guide index should contain guides"
    );
    assert!(
        !index.categories.is_empty(),
        "guide index should contain categories"
    );
}
