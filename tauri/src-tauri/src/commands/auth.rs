use crate::state::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct AccountInfo {
    pub id: String,
    pub name: String,
    pub salt: String,
    pub verify_hash: String,
    pub password_hint: Option<String>,
    pub created_at: Option<String>,
}

#[tauri::command]
pub async fn check_has_account(state: State<'_, AppState>) -> Result<bool, String> {
    let svc = state.vault_service.read().await;
    Ok(svc.has_any_account())
}

#[tauri::command]
pub async fn bootstrap(
    state: State<'_, AppState>,
    account_name: String,
    password: String,
    locale: String,
    password_hint: Option<String>,
) -> Result<AccountInfo, String> {
    let svc = state.vault_service.read().await;
    let result = svc.create_account(&account_name, &password, password_hint.as_deref())?;
    let account_id = result["id"].as_str().unwrap_or("").to_string();

    // Seed default templates from embedded resources (one-time import)
    {
        let vault_guard = svc
            .get_vault_store()
            .ok_or("Vault not available after creation")?;
        if let Some(vault) = vault_guard.as_ref() {
            if let Err(e) = crate::services::template_service::seed_default_templates(
                vault,
                &account_id,
                &locale,
            ) {
                tracing::warn!("Failed to seed default templates: {}", e);
            }
        }
    }

    Ok(AccountInfo {
        id: account_id,
        name: result["name"].as_str().unwrap_or("").to_string(),
        salt: result["salt"].as_str().unwrap_or("").to_string(),
        verify_hash: result["verifyHash"].as_str().unwrap_or("").to_string(),
        password_hint: result["passwordHint"].as_str().map(|s| s.to_string()),
        created_at: Some(chrono::Utc::now().to_rfc3339()),
    })
}

#[tauri::command]
pub async fn login(
    state: State<'_, AppState>,
    account_id: String,
    password: String,
) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    svc.unlock(&account_id, &password)?;
    if let Some(vg) = svc.get_vault_store() {
        if let Some(vault) = vg.as_ref() {
            let _ = vault.log_structured(
                "login",
                "auth",
                Some(&account_id),
                None,
                "user",
                Some("method=password location=login_page action=unlock"),
            );
        }
    }
    Ok(())
}

pub(crate) fn verify_password_core(
    password: &str,
    config: &crate::services::vault_service::AccountConfig,
) -> Result<bool, String> {
    use solosoul_crypto::kdf::{derive_key, KdfConfig};
    let salt_bytes =
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &config.salt)
            .map_err(|_| "Invalid salt".to_string())?;
    let salt_arr: [u8; 16] = salt_bytes
        .as_slice()
        .try_into()
        .map_err(|_| "Bad salt".to_string())?;

    let kdf_config = KdfConfig::balanced();
    let master_key =
        derive_key(password, &salt_arr, &kdf_config).map_err(|_| "KDF failed".to_string())?;

    let verify_data = b"SOLOSOUL_VAULT_VERIFY_v1";
    let verify_key = derive_key(
        &hex::encode(master_key.as_slice()),
        verify_data,
        &KdfConfig {
            memory_kb: 8192,
            iterations: 1,
            parallelism: 1,
        },
    )
    .map_err(|_| "Verify failed".to_string())?;

    let expected_hash =
        hex::decode(&config.verify_hash).map_err(|_| "Invalid verify hash".to_string())?;
    Ok(solosoul_crypto::secure::secure_compare(
        verify_key.as_slice(),
        &expected_hash,
    ))
}

#[tauri::command]
pub async fn verify_password(
    state: State<'_, AppState>,
    account_id: String,
    password: String,
) -> Result<bool, String> {
    let svc = state.vault_service.read().await;
    let config_path = svc.base_path().join(&account_id).join("config.json");
    let content =
        std::fs::read_to_string(&config_path).map_err(|_| "Account not found".to_string())?;
    let config: crate::services::vault_service::AccountConfig =
        serde_json::from_str(&content).map_err(|_| "Parse error".to_string())?;

    verify_password_core(&password, &config)
}

#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    svc.lock();
    Ok(())
}

#[tauri::command]
pub async fn get_current_account(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let svc = state.vault_service.read().await;
    Ok(svc.get_current_account())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::vault_service::AccountConfig;
    use base64::Engine;
    use solosoul_crypto::kdf::{derive_key, KdfConfig};

    fn sample_account_config() -> AccountConfig {
        AccountConfig {
            account_id: "acc-1".to_string(),
            name: "Test".to_string(),
            salt: base64::engine::general_purpose::STANDARD.encode(b"1234567890123456"),
            verify_hash: String::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            crypto_version: 1,
            password_hint: None,
            last_login_at: None,
            last_operation_at: None,
            last_operation_desc: None,
            biometric_enabled: false,
        }
    }

    fn compute_verify_hash(password: &str, salt: &[u8; 16]) -> String {
        let kdf_config = KdfConfig::balanced();
        let master_key = derive_key(password, salt, &kdf_config).unwrap();
        let verify_data = b"SOLOSOUL_VAULT_VERIFY_v1";
        let verify_key = derive_key(
            &hex::encode(master_key.as_slice()),
            verify_data,
            &KdfConfig {
                memory_kb: 8192,
                iterations: 1,
                parallelism: 1,
            },
        )
        .unwrap();
        hex::encode(verify_key.as_slice())
    }

    #[test]
    fn test_account_info_serialization() {
        let info = AccountInfo {
            id: "acc-1".to_string(),
            name: "Alice".to_string(),
            salt: "salty".to_string(),
            verify_hash: "hashy".to_string(),
            password_hint: Some("hint".to_string()),
            created_at: Some("2024-01-01T00:00:00Z".to_string()),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"id\":\"acc-1\""));
        assert!(json.contains("\"name\":\"Alice\""));
        assert!(json.contains("\"password_hint\":\"hint\""));
        assert!(json.contains("\"created_at\":\"2024-01-01T00:00:00Z\""));
    }

    #[test]
    fn test_account_config_serde_roundtrip() {
        let original = sample_account_config();
        let json = serde_json::to_string(&original).unwrap();
        let restored: AccountConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.account_id, original.account_id);
        assert_eq!(restored.name, original.name);
        assert_eq!(restored.salt, original.salt);
        assert_eq!(restored.verify_hash, original.verify_hash);
        assert_eq!(restored.crypto_version, original.crypto_version);
    }

    #[test]
    fn test_verify_password_core_correct_password() {
        let salt = b"1234567890123456";
        let mut config = sample_account_config();
        config.salt = base64::engine::general_purpose::STANDARD.encode(salt);
        config.verify_hash = compute_verify_hash("secret123", salt);

        assert!(verify_password_core("secret123", &config).unwrap());
    }

    #[test]
    fn test_verify_password_core_wrong_password() {
        let salt = b"1234567890123456";
        let mut config = sample_account_config();
        config.salt = base64::engine::general_purpose::STANDARD.encode(salt);
        config.verify_hash = compute_verify_hash("secret123", salt);

        assert!(!verify_password_core("wrongpassword", &config).unwrap());
    }

    #[test]
    fn test_verify_password_core_invalid_salt() {
        let mut config = sample_account_config();
        config.salt = "not-valid-base64!!!".to_string();
        assert!(verify_password_core("secret123", &config).is_err());
    }

    #[test]
    fn test_verify_password_core_bad_salt_length() {
        let mut config = sample_account_config();
        config.salt = base64::engine::general_purpose::STANDARD.encode(b"short");
        assert!(verify_password_core("secret123", &config).is_err());
    }
}
