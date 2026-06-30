//! Biometric (Touch ID/Face ID/Windows Hello) commands.
//!
//! Business logic lives in `solosoul_core::biometric::BiometricManager`; this
//! file only contains the thin `#[tauri::command]` wrappers and audit logging
//! that depends on the unlocked vault store.

use crate::state::AppState;
use solosoul_core::biometric::{
    trigger_system_biometric, BiometricAvailability, BiometricError, BiometricManager,
};
use tauri::State;

const BIO_ERR_PREFIX: &str = "__BIO_ERR__:";

fn bio_err(code: &str) -> String {
    format!("{}{}", BIO_ERR_PREFIX, code)
}

/// 将 BiometricError 映射为前端可国际化的短代码。
fn map_bio_error(e: BiometricError, operation: &str) -> String {
    let code = match &e {
        BiometricError::PlatformNotSupported => "platform_not_supported",
        BiometricError::UserPresenceCancelled => "cancelled",
        BiometricError::UserPresenceUnavailable => "user_presence_unavailable",
        BiometricError::KeychainItemNotFound => "not_configured",
        BiometricError::MissingKeychainEntitlement => "keychain_write_failed",
        BiometricError::InvalidKeyFormat => "invalid_key_format",
        BiometricError::LegacyMigrationFailed(_) => "stale_credential",
        BiometricError::Other(msg) => {
            let lower = msg.to_lowercase();
            if lower.contains("invalid password") {
                return bio_err("invalid_password");
            }
            match operation {
                "save" => "save_failed",
                "unlock" => "unlock_failed",
                "delete" => "delete_failed",
                _ => "unknown",
            }
        }
        BiometricError::KeychainWriteFailed(_) => match operation {
            "save" => "keychain_write_failed",
            "delete" => "delete_failed",
            _ => "keychain_write_failed",
        },
        BiometricError::KeychainReadFailed(_) => match operation {
            "unlock" => "keychain_read_failed",
            _ => "keychain_read_failed",
        },
    };
    bio_err(code)
}

#[tauri::command]
pub async fn biometric_check_availability(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<BiometricAvailability, String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let manager = BiometricManager::new(svc.base_path().clone());
    // 旧版本地文件凭证在查询时自动迁移到 Keychain；失败也不影响显示。
    if !account_id.is_empty() {
        let _ = manager.migrate_legacy_if_needed(&account_id);
    }
    let result = manager.availability(&account_id);
    if !account_id.is_empty() {
        tracing::debug!(
            "biometric_check_availability account_id={} available={} configured={} biometry_type={:?}",
            account_id,
            result.available,
            result.configured,
            result.biometry_type
        );
    }
    Ok(result)
}

#[tauri::command]
pub async fn biometric_save_credential(
    state: State<'_, AppState>,
    account_id: String,
    password: String,
    location: Option<String>,
    action: Option<String>,
    biometry_type: Option<String>,
) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let manager = BiometricManager::new(svc.base_path().clone());

    // 保存前校验即将写入的密钥与当前 Vault 会话密钥一致，避免钥匙串/文件访问问题导致回读失败。
    if let Some(session_key) = svc.get_session_key() {
        let expected = hex::encode(session_key.as_slice());
        let derived = manager
            .derive_key_hex(&password, &account_id)
            .map_err(|e| map_bio_error(e, "save"))?;
        if derived != expected {
            return Err(bio_err("credential_mismatch"));
        }
    }

    manager
        .save_credential(
            &account_id,
            &password,
            "verify your identity to enable Touch ID for SoloSoul",
        )
        .map_err(|e| map_bio_error(e, "save"))?;

    if let Some(vg) = svc.get_vault_store() {
        let vault = vg.as_ref();
        let loc = location.unwrap_or_else(|| "unknown".to_string());
        let act = action.unwrap_or_else(|| "enable".to_string());
        let bio_type = biometry_type.as_deref().unwrap_or("unknown");
        let _ = vault.log_structured(
            "biometric_saved",
            "biometric",
            Some(&account_id),
            None,
            "user",
            Some(&format!(
                "location={} action={} type={}",
                loc, act, bio_type
            )),
        );
    }
    Ok(())
}

#[tauri::command]
pub async fn biometric_unlock(
    state: State<'_, AppState>,
    account_id: String,
    location: Option<String>,
    action: Option<String>,
    biometry_type: Option<String>,
) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let manager = BiometricManager::new(svc.base_path().clone());
    let used_bio_type = manager
        .unlock(&account_id, &svc, "unlock SoloSoul")
        .map_err(|e| map_bio_error(e, "unlock"))?;

    if let Some(vg) = svc.get_vault_store() {
        let vault = vg.as_ref();
        let loc = location.unwrap_or_else(|| "unknown".to_string());
        let act = action.unwrap_or_else(|| "unlock".to_string());
        let bio_type = biometry_type.as_deref().unwrap_or(&used_bio_type);
        // Critical field access produces a more detailed frontend-side audit
        // entry (critical_field_touch_id / critical_field_face_id), so skip
        // the generic biometric unlock entry to avoid duplicates.
        if loc != "critical_data_access" {
            let action_type = match bio_type {
                "touchId" => "touch_id_unlock",
                "faceId" => "face_id_unlock",
                _ => "biometric_unlock",
            };
            let _ = vault.log_structured(
                action_type,
                "biometric",
                Some(&account_id),
                None,
                "user",
                Some(&format!(
                    "location={} action={} type={}",
                    loc, act, bio_type
                )),
            );
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn biometric_delete_credential(
    state: State<'_, AppState>,
    account_id: String,
    password: String,
    location: Option<String>,
    action: Option<String>,
    biometry_type: Option<String>,
) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let manager = BiometricManager::new(svc.base_path().clone());
    manager
        .delete_credential(&account_id, &password)
        .map_err(|e| map_bio_error(e, "delete"))?;

    if let Some(vg) = svc.get_vault_store() {
        let vault = vg.as_ref();
        let loc = location.unwrap_or_else(|| "unknown".to_string());
        let act = action.unwrap_or_else(|| "disable".to_string());
        let bio_type = biometry_type.as_deref().unwrap_or("unknown");
        let _ = vault.log_structured(
            "biometric_deleted",
            "biometric",
            Some(&account_id),
            None,
            "user",
            Some(&format!(
                "location={} action={} type={}",
                loc, act, bio_type
            )),
        );
    }
    Ok(())
}

#[tauri::command]
pub async fn biometric_test(_account_id: String) -> Result<bool, String> {
    if !solosoul_core::biometric::is_macos() {
        return Ok(false);
    }
    // 使用严格策略确保实际触发生物识别，不回落到设备密码。
    trigger_system_biometric("test biometric authentication for SoloSoul", true)
        .map_err(|e| map_bio_error(e, "test"))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bio_err_format() {
        let err = bio_err("not_configured");
        assert_eq!(err, "__BIO_ERR__:not_configured");
    }

    #[test]
    fn test_map_bio_error_platform_not_supported() {
        let err = map_bio_error(BiometricError::PlatformNotSupported, "save");
        assert_eq!(err, "__BIO_ERR__:platform_not_supported");
    }

    #[test]
    fn test_map_bio_error_cancelled() {
        let err = map_bio_error(BiometricError::UserPresenceCancelled, "unlock");
        assert_eq!(err, "__BIO_ERR__:cancelled");
    }

    #[test]
    fn test_map_bio_error_unavailable() {
        let err = map_bio_error(BiometricError::UserPresenceUnavailable, "save");
        assert_eq!(err, "__BIO_ERR__:user_presence_unavailable");
    }

    #[test]
    fn test_map_bio_error_not_configured() {
        let err = map_bio_error(BiometricError::KeychainItemNotFound, "unlock");
        assert_eq!(err, "__BIO_ERR__:not_configured");
    }

    #[test]
    fn test_map_bio_error_other_invalid_password() {
        let err = map_bio_error(
            BiometricError::Other("Invalid password".to_string()),
            "unlock",
        );
        assert_eq!(err, "__BIO_ERR__:invalid_password");
    }

    #[test]
    fn test_map_bio_error_other_save_operation() {
        let err = map_bio_error(BiometricError::Other("disk full".to_string()), "save");
        assert_eq!(err, "__BIO_ERR__:save_failed");
    }

    #[test]
    fn test_map_bio_error_other_unlock_operation() {
        let err = map_bio_error(BiometricError::Other("timeout".to_string()), "unlock");
        assert_eq!(err, "__BIO_ERR__:unlock_failed");
    }

    #[test]
    fn test_map_bio_error_other_delete_operation() {
        let err = map_bio_error(
            BiometricError::Other("permission denied".to_string()),
            "delete",
        );
        assert_eq!(err, "__BIO_ERR__:delete_failed");
    }

    #[test]
    fn test_map_bio_error_other_unknown_operation() {
        let err = map_bio_error(
            BiometricError::Other("something else".to_string()),
            "unknown_op",
        );
        assert_eq!(err, "__BIO_ERR__:unknown");
    }

    #[test]
    fn test_map_bio_error_keychain_write_failed_save() {
        let err = map_bio_error(
            BiometricError::KeychainWriteFailed("write error".to_string()),
            "save",
        );
        assert_eq!(err, "__BIO_ERR__:keychain_write_failed");
    }

    #[test]
    fn test_map_bio_error_keychain_write_failed_delete() {
        let err = map_bio_error(
            BiometricError::KeychainWriteFailed("write error".to_string()),
            "delete",
        );
        assert_eq!(err, "__BIO_ERR__:delete_failed");
    }

    #[test]
    fn test_map_bio_error_keychain_read_failed() {
        let err = map_bio_error(
            BiometricError::KeychainReadFailed("read error".to_string()),
            "unlock",
        );
        assert_eq!(err, "__BIO_ERR__:keychain_read_failed");
    }

    #[test]
    fn test_map_bio_error_legacy_migration_failed() {
        let err = map_bio_error(
            BiometricError::LegacyMigrationFailed("mig error".to_string()),
            "save",
        );
        assert_eq!(err, "__BIO_ERR__:stale_credential");
    }

    #[test]
    fn test_map_bio_error_invalid_key_format() {
        let err = map_bio_error(BiometricError::InvalidKeyFormat, "save");
        assert_eq!(err, "__BIO_ERR__:invalid_key_format");
    }

    #[test]
    fn test_map_bio_error_missing_keychain_entitlement() {
        let err = map_bio_error(BiometricError::MissingKeychainEntitlement, "save");
        assert_eq!(err, "__BIO_ERR__:keychain_write_failed");
    }
}
