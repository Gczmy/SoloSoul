//! RF005：真实 Vault 上的普通聊天服务商授权与单快照凭据解析。

use super::*;
use crate::llm::config::ApiType;
use serde_json::{json, Value};

const ACCOUNT_A: &str = "rf005-account-a";
const ACCOUNT_B: &str = "rf005-account-b";
const SYNTHETIC_KEY: &str = "rf005-synthetic-key-never-report";

fn vault_fixture() -> (tempfile::TempDir, VaultStore) {
    let dir = tempfile::TempDir::new().unwrap();
    let config = solosoul_vault::VaultConfig::new(ACCOUNT_A, dir.path().to_path_buf())
        .with_data_key([0x53; 32]);
    let vault = VaultStore::open(config).unwrap();
    (dir, vault)
}

fn provider(id: &str, enabled: bool) -> ProviderConfig {
    ProviderConfig {
        id: id.to_string(),
        name: "RF005 saved provider".to_string(),
        base_url: "http://127.0.0.1:11434/v1".to_string(),
        model: "rf005-saved-model".to_string(),
        is_enabled: enabled,
        is_built_in: false,
        api_type: ApiType::OpenAI,
        embedding_model: None,
    }
}

fn profile_data(providers: Vec<ProviderConfig>, keys: Option<Value>) -> Value {
    let config = LlmConfig {
        providers,
        active_provider_id: Some("another-active-provider".to_string()),
        ai_features_enabled: AiFeatures::default(),
        has_accepted_risk: true,
        include_system_prompt: true,
        use_local_embedding: false,
        local_embed_model_id: None,
    };
    let mut data = json!({"preferences": {"llmConfig": config}});
    if let Some(keys) = keys {
        data["preferences"]["llmApiKeys"] = keys;
    }
    data
}

fn save_data(vault: &VaultStore, account_id: &str, data: &Value) {
    save_bytes(vault, account_id, serde_json::to_vec(data).unwrap());
}

fn save_bytes(vault: &VaultStore, account_id: &str, data: Vec<u8>) {
    vault
        .save_profile(&Profile::new_with_id(account_id, account_id, data))
        .unwrap();
}

fn resolver_error(vault: &VaultStore, account_id: &str, provider_id: &str) -> String {
    match LlmService::new().resolve_chat_provider(vault, account_id, provider_id) {
        Ok(_) => panic!("invalid provider unexpectedly resolved"),
        Err(error) => {
            assert!(!error.contains(SYNTHETIC_KEY));
            error
        }
    }
}

#[test]
fn rf005_saved_config_and_key_override_defaults_and_active_selection() {
    let (_dir, vault) = vault_fixture();
    let mut saved = provider("builtin_alibaba", true);
    saved.name = "Saved override".to_string();
    saved.base_url = "https://rf005.example.invalid/custom/v1".to_string();
    saved.model = "saved-chat-model".to_string();
    saved.api_type = ApiType::Anthropic;
    saved.is_built_in = true;
    saved.embedding_model = Some("saved-embedding-model".to_string());
    let data = profile_data(
        vec![saved.clone(), provider("another-active-provider", true)],
        Some(
            json!({"builtin_alibaba": SYNTHETIC_KEY, "another-active-provider": "rf005-other-key"}),
        ),
    );
    save_data(&vault, ACCOUNT_A, &data);

    let resolved = LlmService::new()
        .resolve_chat_provider(&vault, ACCOUNT_A, "builtin_alibaba")
        .unwrap();
    assert_eq!(resolved.id, saved.id);
    assert_eq!(resolved.name, saved.name);
    assert_eq!(resolved.base_url, saved.base_url);
    assert_eq!(resolved.model, saved.model);
    assert_eq!(resolved.api_type, saved.api_type);
    assert_eq!(resolved.embedding_model, saved.embedding_model);
    assert!(resolved.is_enabled && resolved.is_built_in);
    assert!(resolved.api_key == SYNTHETIC_KEY);
    assert_eq!(
        vault.load_profile(ACCOUNT_A).unwrap().unwrap().data,
        serde_json::to_vec(&data).unwrap()
    );
}

#[test]
fn rf005_historical_builtin_ids_remain_exact_saved_ids() {
    let (_dir, vault) = vault_fixture();
    for id in [
        "builtin_openai",
        "builtin_anthropic",
        "builtin_ollama",
        "builtin_deepseek",
        "builtin_alibaba",
        "dashscope",
    ] {
        let mut keys = serde_json::Map::new();
        keys.insert(id.to_string(), json!(SYNTHETIC_KEY));
        save_data(
            &vault,
            ACCOUNT_A,
            &profile_data(vec![provider(id, true)], Some(Value::Object(keys))),
        );
        let resolved = LlmService::new()
            .resolve_chat_provider(&vault, ACCOUNT_A, id)
            .unwrap();
        assert_eq!(resolved.id, id);
        assert!(resolved.api_key == SYNTHETIC_KEY);
    }
}

#[test]
fn rf005_unknown_ids_never_fall_back_to_active_provider_or_builtin_alias() {
    let (_dir, vault) = vault_fixture();
    save_data(
        &vault,
        ACCOUNT_A,
        &profile_data(
            vec![provider("builtin_alibaba", true)],
            Some(json!({"builtin_alibaba": SYNTHETIC_KEY})),
        ),
    );
    for id in [
        "unknown",
        "dashscope",
        "BUILTIN_ALIBABA",
        " builtin_alibaba",
        "",
    ] {
        assert_eq!(
            resolver_error(&vault, ACCOUNT_A, id),
            "Chat provider is not saved"
        );
    }
}

#[test]
fn rf005_disabled_provider_is_rejected_without_changing_legacy_getter() {
    let (_dir, vault) = vault_fixture();
    save_data(
        &vault,
        ACCOUNT_A,
        &profile_data(
            vec![
                provider("disabled", false),
                provider("another-active-provider", true),
            ],
            Some(json!({"disabled": SYNTHETIC_KEY})),
        ),
    );
    assert_eq!(
        resolver_error(&vault, ACCOUNT_A, "disabled"),
        "Chat provider is disabled"
    );
    let legacy = LlmService::new()
        .get_provider_with_key(&vault, ACCOUNT_A, "disabled")
        .unwrap()
        .unwrap();
    assert!(!legacy.is_enabled);
    assert!(legacy.api_key == SYNTHETIC_KEY);
}

#[test]
fn rf005_missing_profile_or_config_does_not_create_default_provider() {
    let (_dir, vault) = vault_fixture();
    assert_eq!(
        resolver_error(&vault, ACCOUNT_A, "openai"),
        "Chat provider configuration is missing"
    );
    assert!(vault.load_profile(ACCOUNT_A).unwrap().is_none());
    save_data(&vault, ACCOUNT_A, &json!({"preferences": {}}));
    assert_eq!(
        resolver_error(&vault, ACCOUNT_A, "openai"),
        "Chat provider configuration is missing"
    );
}

#[test]
fn rf005_provider_from_other_profile_is_not_borrowed() {
    let (_dir, vault) = vault_fixture();
    save_data(&vault, ACCOUNT_A, &profile_data(Vec::new(), None));
    save_data(
        &vault,
        ACCOUNT_B,
        &profile_data(
            vec![provider("foreign-provider", true)],
            Some(json!({"foreign-provider": SYNTHETIC_KEY})),
        ),
    );
    assert_eq!(
        resolver_error(&vault, ACCOUNT_A, "foreign-provider"),
        "Chat provider is not saved"
    );
    assert_eq!(
        resolver_error(&vault, "missing-account", "foreign-provider"),
        "Chat provider configuration is missing"
    );
}

#[test]
fn rf005_same_id_uses_requested_accounts_own_config_and_key() {
    let (_dir, vault) = vault_fixture();
    let mut first = provider("shared-id", true);
    first.base_url = "https://account-a.example.invalid/v1".to_string();
    let mut second = provider("shared-id", true);
    second.base_url = "https://account-b.example.invalid/v1".to_string();
    save_data(
        &vault,
        ACCOUNT_A,
        &profile_data(
            vec![first.clone()],
            Some(json!({"shared-id": "rf005-key-a"})),
        ),
    );
    save_data(
        &vault,
        ACCOUNT_B,
        &profile_data(
            vec![second.clone()],
            Some(json!({"shared-id": "rf005-key-b"})),
        ),
    );
    let service = LlmService::new();
    let resolved_a = service
        .resolve_chat_provider(&vault, ACCOUNT_A, "shared-id")
        .unwrap();
    let resolved_b = service
        .resolve_chat_provider(&vault, ACCOUNT_B, "shared-id")
        .unwrap();
    assert_eq!(resolved_a.base_url, first.base_url);
    assert_eq!(resolved_b.base_url, second.base_url);
    assert!(resolved_a.api_key == "rf005-key-a");
    assert!(resolved_b.api_key == "rf005-key-b");
}

#[test]
fn rf005_invalid_profile_json_returns_sanitized_error() {
    let (_dir, vault) = vault_fixture();
    save_bytes(
        &vault,
        ACCOUNT_A,
        format!("{{\"preferences\":\"{SYNTHETIC_KEY}\"").into_bytes(),
    );
    assert_eq!(
        resolver_error(&vault, ACCOUNT_A, "saved"),
        "Invalid chat provider configuration"
    );
}

#[test]
fn rf005_invalid_root_or_preferences_returns_sanitized_error() {
    let (_dir, vault) = vault_fixture();
    for data in [
        Value::Null,
        json!([]),
        json!({}),
        json!({"preferences": null}),
        json!({"preferences": [SYNTHETIC_KEY]}),
        json!({"preferences": SYNTHETIC_KEY}),
    ] {
        save_data(&vault, ACCOUNT_A, &data);
        assert_eq!(
            resolver_error(&vault, ACCOUNT_A, "saved"),
            "Invalid chat provider configuration"
        );
    }
}

#[test]
fn rf005_invalid_saved_config_does_not_fall_back_or_report_values() {
    let (_dir, vault) = vault_fixture();
    let valid = profile_data(
        vec![provider("saved", true)],
        Some(json!({"saved": SYNTHETIC_KEY})),
    );
    let mut invalid_api = valid.clone();
    invalid_api["preferences"]["llmConfig"]["providers"][0]["apiType"] = json!(SYNTHETIC_KEY);
    let mut invalid_enabled = valid.clone();
    invalid_enabled["preferences"]["llmConfig"]["providers"][0]["isEnabled"] = json!(SYNTHETIC_KEY);
    let mut missing_url = valid.clone();
    missing_url["preferences"]["llmConfig"]["providers"][0]
        .as_object_mut()
        .unwrap()
        .remove("baseUrl");
    let mut invalid_config = valid;
    invalid_config["preferences"]["llmConfig"] = Value::Null;
    for data in [invalid_api, invalid_enabled, missing_url, invalid_config] {
        save_data(&vault, ACCOUNT_A, &data);
        assert_eq!(
            resolver_error(&vault, ACCOUNT_A, "saved"),
            "Invalid chat provider configuration"
        );
    }
}

#[test]
fn rf005_invalid_key_map_is_rejected_instead_of_becoming_empty_key() {
    let (_dir, vault) = vault_fixture();
    for keys in [
        Value::Null,
        json!([SYNTHETIC_KEY]),
        json!(SYNTHETIC_KEY),
        json!({"saved": {"unexpected": SYNTHETIC_KEY}}),
        json!({"saved": SYNTHETIC_KEY, "other": 7}),
    ] {
        save_data(
            &vault,
            ACCOUNT_A,
            &profile_data(vec![provider("saved", true)], Some(keys)),
        );
        assert_eq!(
            resolver_error(&vault, ACCOUNT_A, "saved"),
            "Invalid chat provider credentials"
        );
    }
}

#[test]
fn rf005_missing_and_empty_keys_allow_saved_local_provider() {
    let (_dir, vault) = vault_fixture();
    for keys in [
        None,
        Some(json!({})),
        Some(json!({"other": SYNTHETIC_KEY})),
        Some(json!({"local": ""})),
    ] {
        save_data(
            &vault,
            ACCOUNT_A,
            &profile_data(vec![provider("local", true)], keys),
        );
        let resolved = LlmService::new()
            .resolve_chat_provider(&vault, ACCOUNT_A, "local")
            .unwrap();
        assert!(resolved.api_key.is_empty());
        assert_eq!(resolved.base_url, "http://127.0.0.1:11434/v1");
    }
}

#[test]
fn rf005_vault_read_failure_is_rejected_with_sanitized_error() {
    let (_dir, vault) = vault_fixture();
    save_data(
        &vault,
        ACCOUNT_A,
        &profile_data(
            vec![provider("saved", true)],
            Some(json!({"saved": SYNTHETIC_KEY})),
        ),
    );
    vault.lock();
    assert_eq!(
        resolver_error(&vault, ACCOUNT_A, "saved"),
        "Failed to load chat provider configuration"
    );
}

#[test]
fn rf005_duplicate_selected_ids_are_rejected_in_any_enabled_order() {
    let (_dir, vault) = vault_fixture();
    for enabled in [[true, false], [false, true], [true, true]] {
        save_data(
            &vault,
            ACCOUNT_A,
            &profile_data(
                vec![provider("saved", enabled[0]), provider("saved", enabled[1])],
                Some(json!({"saved": SYNTHETIC_KEY})),
            ),
        );
        assert_eq!(
            resolver_error(&vault, ACCOUNT_A, "saved"),
            "Invalid chat provider configuration"
        );
    }
}
