use crate::commands::object::trash::run_expired_trash_cleanup;
use crate::state::AppState;
use solosoul_core::auth::verify_password_core;
use solosoul_core::template_service::seed_default_templates;
use solosoul_core::AccountConfig;
use solosoul_core::AccountSummary;
use tauri::State;
use zeroize::Zeroizing;

#[tauri::command]
pub async fn check_has_account(state: State<'_, AppState>) -> Result<bool, String> {
    let vault_service = state.vault_service.clone();
    tokio::task::spawn_blocking(move || {
        let svc = vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        Ok::<_, String>(svc.has_any_account())
    })
    .await
    .map_err(|e| format!("check_has_account task failed: {}", e))?
}

#[tauri::command]
pub async fn bootstrap(
    state: State<'_, AppState>,
    account_name: String,
    password: String,
    locale: String,
    password_hint: Option<String>,
) -> Result<AccountSummary, String> {
    // P031: 密码以 Zeroizing<String> 接收，使用完毕后立即安全清零。
    let password = Zeroizing::new(password);
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let result = svc.create_account(&account_name, password.as_ref(), password_hint.as_deref())?;
    let account_id = result["id"].as_str().unwrap_or("").to_string();

    // 首个账户创建成功后，立即触发一次 SAF 同步，消除首次数据丢失窗口。
    // AutoSyncManager 会自行判断是否处于 SAF 模式，非 SAF 下为 no-op。
    state.auto_sync.trigger_immediate();

    // Seed default templates from embedded resources (one-time import)
    {
        let vault_guard = svc
            .get_vault_store()
            .ok_or("Vault not available after creation")?;
        let vault = vault_guard.as_ref();
        {
            if let Err(e) = seed_default_templates(vault, &account_id, &locale) {
                tracing::warn!("Failed to seed default templates: {}", e);
            }
        }
    }

    // P023: 收敛 core 单一 AccountSummary——新账户尚无生物识别/PIN 历史，标志位为 false。
    Ok(AccountSummary {
        id: account_id,
        name: result["name"].as_str().unwrap_or("").to_string(),
        password_hint: result["passwordHint"].as_str().map(|s| s.to_string()),
        created_at: Some(chrono::Utc::now().to_rfc3339()),
        has_biometric_history: false,
        has_pin_history: false,
    })
}

#[tauri::command]
pub async fn login(
    state: State<'_, AppState>,
    account_id: String,
    password: String,
) -> Result<(), String> {
    // P031: 密码以 Zeroizing<String> 接收，使用完毕后立即安全清零。
    let password = Zeroizing::new(password);
    // Run the CPU-intensive KDF and synchronous vault IO on the blocking pool
    // so the async runtime worker threads are not starved (R018 follow-up).
    let vault_service = state.vault_service.clone();
    tokio::task::spawn_blocking(move || {
        let svc = vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        svc.unlock_secure(&account_id, &password)?;
        if let Some(vg) = svc.get_vault_store() {
            let vault = vg.as_ref();
            {
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
        Ok::<_, String>(())
    })
    .await
    .map_err(|e| format!("Login task failed: {}", e))??;

    // 登录成功后自动清理过期回收站项目（失败不影响登录结果）
    run_expired_trash_cleanup(&state);

    Ok(())
}

/// 重置账户的安全标志（生物识别、PIN 等）到初始关闭状态。
///
/// 用于重装后从已有外部目录登录的场景：config.json 中的旧标志会在卸载后残留，
/// 但实际凭证（KeyStore 条目、PIN 文件）已被清除。此命令将标志复位，
/// 避免用户在安全设置中看到「已启用」但实际无法使用的状态。
#[tauri::command]
pub async fn reset_security_flags(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<(), String> {
    let vault_service = state.vault_service.clone();
    tokio::task::spawn_blocking(move || {
        let svc = vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        svc.reset_security_flags(&account_id)
    })
    .await
    .map_err(|e| format!("reset_security_flags task failed: {}", e))?
}

/// Verify the master password and unlock the vault without writing an audit log.
/// Used when re-authenticating to reveal critical fields, so the resulting
/// critical-field audit entry is the only log produced.
#[tauri::command]
pub async fn unlock_with_password(
    state: State<'_, AppState>,
    account_id: String,
    password: String,
) -> Result<(), String> {
    // P031: 密码以 Zeroizing<String> 接收，使用完毕后立即安全清零。
    let password = Zeroizing::new(password);
    let vault_service = state.vault_service.clone();
    tokio::task::spawn_blocking(move || {
        let svc = vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        svc.unlock_secure(&account_id, &password)?;
        Ok::<_, String>(())
    })
    .await
    .map_err(|e| format!("Unlock task failed: {}", e))??;

    // 解锁成功后自动清理过期回收站项目（失败不影响解锁结果）
    run_expired_trash_cleanup(&state);

    Ok(())
}

#[tauri::command]
pub async fn verify_password(
    state: State<'_, AppState>,
    account_id: String,
    password: String,
) -> Result<bool, String> {
    // P031: 密码以 Zeroizing<String> 接收，使用完毕后立即安全清零。
    let password = Zeroizing::new(password);
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let config_path = svc.base_path().join(&account_id).join("config.json");
    let content =
        std::fs::read_to_string(&config_path).map_err(|_| "Account not found".to_string())?;
    let config: AccountConfig =
        serde_json::from_str(&content).map_err(|_| "Parse error".to_string())?;

    verify_password_core(password.as_ref(), &config)
}

#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    svc.lock();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use solosoul_crypto::kdf::{derive_key, KdfConfig};

    fn sample_account_config() -> AccountConfig {
        AccountConfig {
            account_id: "acc-1".to_string(),
            name: "Test".to_string(),
            salt: base64::engine::general_purpose::STANDARD.encode(b"1234567890123456"),
            verify_hash: String::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            crypto_version: 3,
            password_hint: None,
            last_login_at: None,
            last_operation_at: None,
            last_operation_desc: None,
            biometric_enabled: false,
            pin_enabled: false,
            pin_length: 0,
            pin_failed_attempts: 0,
            pin_locked_until: None,
            password_failed_attempts: 0,
            password_locked_until: None,
            kdf_memory_kb: None,
            kdf_iterations: None,
            kdf_parallelism: None,
        }
    }

    fn compute_verify_hash(password: &str, salt: &[u8; 16]) -> String {
        let kdf_config = KdfConfig::balanced();
        let master_key = derive_key(password, salt, &kdf_config).unwrap();
        let mk: [u8; 32] = master_key.as_slice().try_into().unwrap();
        let vk = solosoul_crypto::hkdf_ext::derive_hkdf_key(&mk, salt, b"SOLOSOUL_VAULT_VERIFY_v1")
            .unwrap();
        hex::encode(vk)
    }

    #[test]
    fn test_account_summary_serialization() {
        let info = AccountSummary {
            id: "acc-1".to_string(),
            name: "Alice".to_string(),
            password_hint: Some("hint".to_string()),
            created_at: Some("2024-01-01T00:00:00Z".to_string()),
            has_biometric_history: false,
            has_pin_history: false,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"id\":\"acc-1\""));
        assert!(json.contains("\"name\":\"Alice\""));
        assert!(json.contains("\"passwordHint\":\"hint\""));
        assert!(json.contains("\"createdAt\":\"2024-01-01T00:00:00Z\""));
        assert!(json.contains("\"hasBiometricHistory\":false"));
        assert!(json.contains("\"hasPinHistory\":false"));
    }

    #[test]
    fn test_account_config_serde_roundtrip() {
        let original = sample_account_config();
        let json = serde_json::to_string(&original).unwrap();
        assert!(json.contains("\"biometricEnabled\":false"));
        let restored: AccountConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.account_id, original.account_id);
        assert_eq!(restored.name, original.name);
        assert_eq!(restored.crypto_version, original.crypto_version);
        assert_eq!(restored.biometric_enabled, original.biometric_enabled);
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
