//! Biometric (Touch ID/Face ID/Windows Hello) commands.
//!
//! Business logic lives in `solosoul_core::biometric::BiometricManager`; this
//! file only contains the thin `#[tauri::command]` wrappers and audit logging
//! that depends on the unlocked vault store.

use crate::state::AppState;
use solosoul_core::biometric::{BiometricAvailability, BiometricError, BiometricManager};
use tauri::State;

#[cfg(desktop)]
use solosoul_core::biometric::trigger_system_biometric;

const BIO_ERR_PREFIX: &str = "__BIO_ERR__:";

fn bio_err(code: &str) -> String {
    format!("{}{}", BIO_ERR_PREFIX, code)
}

/// 将 Keystore 插件返回的错误字符串映射为 BiometricError。
#[cfg(target_os = "android")]
fn map_keystore_error(e: String, operation: &str) -> String {
    if e == "BIOMETRIC_KEY_INVALIDATED" || e == "BIOMETRIC_KEY_NOT_FOUND" {
        map_bio_error(BiometricError::KeychainItemNotFound, operation)
    } else if e == "BIOMETRIC_CANCELLED" {
        map_bio_error(BiometricError::UserPresenceCancelled, operation)
    } else if e == "BIOMETRIC_NOT_ENROLLED" {
        map_bio_error(BiometricError::UserPresenceUnavailable, operation)
    } else if e == "BIOMETRIC_LOCKOUT" || e == "BIOMETRIC_UNAVAILABLE" {
        map_bio_error(BiometricError::UserPresenceUnavailable, operation)
    } else if e.starts_with("BIOMETRIC_ERROR:") {
        map_bio_error(BiometricError::Other(e), operation)
    } else if operation == "save" {
        map_bio_error(BiometricError::KeychainWriteFailed(e), operation)
    } else {
        map_bio_error(BiometricError::KeychainReadFailed(e), operation)
    }
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

#[cfg(desktop)]
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

#[cfg(all(mobile, any(target_os = "android", target_os = "ios")))]
#[tauri::command]
pub async fn biometric_check_availability(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    account_id: String,
) -> Result<BiometricAvailability, String> {
    use tauri_plugin_biometric::{BiometricExt, BiometryType};

    let status = app.biometric().status().map_err(|e| e.to_string())?;
    let biometry_type = match status.biometry_type {
        BiometryType::TouchID => Some("touchId".to_string()),
        BiometryType::FaceID => Some("faceId".to_string()),
        BiometryType::None => None,
    };

    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;

    // Android 使用 Keystore 存储，iOS 沿用 FileBiometricStorage
    #[cfg(target_os = "android")]
    let configured = {
        let path = svc.base_path().join(&account_id).join("keystore_data.json");
        path.exists()
    };

    #[cfg(target_os = "ios")]
    let configured = {
        let manager = BiometricManager::new(svc.base_path().clone());
        manager.is_configured(&account_id)
    };

    Ok(BiometricAvailability {
        available: status.is_available,
        configured,
        biometry_type,
        error: status.error,
    })
}

#[cfg(desktop)]
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

    // reason 根据 biometry_type 动态生成，避免在 Windows Hello 设备上显示 Touch ID
    let save_reason = match biometry_type.as_deref() {
        Some("windowsHello") => "verify your identity to enable Windows Hello for SoloSoul",
        _ => "verify your identity to enable Touch ID / Face ID for SoloSoul",
    };
    manager
        .save_credential(&account_id, &password, save_reason)
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

#[cfg(all(mobile, any(target_os = "android", target_os = "ios")))]
#[tauri::command]
pub async fn biometric_save_credential(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    account_id: String,
    password: String,
    location: Option<String>,
    action: Option<String>,
    biometry_type: Option<String>,
) -> Result<(), String> {
    use solosoul_core::biometric::legacy::FileBiometricStorage;
    use solosoul_core::biometric::BiometricStorage;

    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let manager = BiometricManager::new(svc.base_path().clone());

    // 1. 验证主密码
    manager
        .verify_password(&password, &account_id)
        .map_err(|e| map_bio_error(e, "save"))?;

    // 2. 派生主密钥并使用平台安全存储保存
    let key_hex = manager
        .derive_key_hex(&password, &account_id)
        .map_err(|e| map_bio_error(e, "save"))?;

    #[cfg(target_os = "android")]
    {
        use crate::keystore_plugin::{BiometricPromptInfo, KeystorePluginHandle};
        use tauri::Manager;

        let keystore = app.state::<KeystorePluginHandle<tauri::Wry>>();
        let cipher = keystore
            .authenticate_and_save(
                &account_id,
                &key_hex,
                BiometricPromptInfo {
                    title: "SoloSoul",
                    subtitle: "Enable biometric authentication",
                    cancel_title: "Cancel",
                },
            )
            .map_err(|e| map_keystore_error(e, "save"))?;

        let path = svc.base_path().join(&account_id).join("keystore_data.json");
        let json = serde_json::to_string(&cipher)
            .map_err(|e| map_bio_error(BiometricError::Other(format!("serialize: {e}")), "save"))?;
        std::fs::write(&path, json).map_err(|e| {
            map_bio_error(BiometricError::KeychainWriteFailed(e.to_string()), "save")
        })?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&path)
                .map_err(|e| map_bio_error(BiometricError::Other(format!("stat: {e}")), "save"))?
                .permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(&path, perms)
                .map_err(|e| map_bio_error(BiometricError::Other(format!("chmod: {e}")), "save"))?;
        }

        // 删除旧版 FileBiometricStorage 凭证，避免弱加密文件残留
        let legacy_path = svc.base_path().join(&account_id).join("biometric_key");
        let _ = std::fs::remove_file(&legacy_path);
    }

    #[cfg(target_os = "ios")]
    {
        let reason = "verify your identity to enable biometric authentication for SoloSoul";
        let storage = FileBiometricStorage::new(svc.base_path().clone());
        storage
            .save(&account_id, &key_hex, reason)
            .map_err(|e| map_bio_error(e, "save"))?;
    }

    // 3. 更新配置标记
    manager
        .set_config_flag(&account_id, true)
        .map_err(|e| map_bio_error(e, "save"))?;

    // 4. 审计日志
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

#[cfg(desktop)]
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
                "windowsHello" => "windows_hello_unlock",
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

#[cfg(all(mobile, any(target_os = "android", target_os = "ios")))]
#[tauri::command]
pub async fn biometric_unlock(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    account_id: String,
    location: Option<String>,
    action: Option<String>,
    biometry_type: Option<String>,
) -> Result<(), String> {
    use solosoul_core::biometric::legacy::FileBiometricStorage;
    use solosoul_core::biometric::BiometricStorage;

    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let manager = BiometricManager::new(svc.base_path().clone());

    // 1. 读取已保存的主密钥（Android 通过 CryptoObject 绑定生物识别提示）
    let key_hex = {
        #[cfg(target_os = "android")]
        {
            use crate::keystore_plugin::{BiometricPromptInfo, KeystorePluginHandle};
            use tauri::Manager;

            let path = svc.base_path().join(&account_id).join("keystore_data.json");
            let json = std::fs::read_to_string(&path)
                .map_err(|_| map_bio_error(BiometricError::KeychainItemNotFound, "unlock"))?;
            let cipher: crate::keystore_plugin::KeystoreCiphertext = serde_json::from_str(&json)
                .map_err(|_| map_bio_error(BiometricError::InvalidKeyFormat, "unlock"))?;

            let keystore = app.state::<KeystorePluginHandle<tauri::Wry>>();
            keystore
                .authenticate_and_read(
                    &account_id,
                    &cipher.iv,
                    &cipher.ciphertext,
                    BiometricPromptInfo {
                        title: "SoloSoul",
                        subtitle: "Unlock with biometric authentication",
                        cancel_title: "Cancel",
                    },
                )
                .map_err(|e| map_keystore_error(e, "unlock"))?
        }

        #[cfg(target_os = "ios")]
        {
            let reason = "unlock SoloSoul";
            let storage = FileBiometricStorage::new(svc.base_path().clone());
            storage
                .read(&account_id, reason)
                .map_err(|e| map_bio_error(e, "unlock"))?
        }
    };

    let key_bytes = hex::decode(&key_hex)
        .map_err(|_| map_bio_error(BiometricError::InvalidKeyFormat, "unlock"))?;
    let key: [u8; 32] = key_bytes
        .as_slice()
        .try_into()
        .map_err(|_| map_bio_error(BiometricError::InvalidKeyFormat, "unlock"))?;

    // 2. 解锁 Vault
    svc.unlock_with_session_key(&account_id, &key)
        .map_err(|e| map_bio_error(BiometricError::Other(format!("{:#}", e)), "unlock"))?;

    // 3. 审计日志
    if let Some(vg) = svc.get_vault_store() {
        let vault = vg.as_ref();
        let loc = location.unwrap_or_else(|| "unknown".to_string());
        let act = action.unwrap_or_else(|| "unlock".to_string());
        let bio_type = biometry_type.as_deref().unwrap_or("unknown");
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

#[cfg(desktop)]
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

#[cfg(all(mobile, any(target_os = "android", target_os = "ios")))]
#[tauri::command]
pub async fn biometric_delete_credential(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    account_id: String,
    password: String,
    location: Option<String>,
    action: Option<String>,
    biometry_type: Option<String>,
) -> Result<(), String> {
    use solosoul_core::biometric::legacy::FileBiometricStorage;
    use solosoul_core::biometric::BiometricStorage;

    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let manager = BiometricManager::new(svc.base_path().clone());

    // 1. 验证主密码
    manager
        .verify_password(&password, &account_id)
        .map_err(|e| map_bio_error(e, "delete"))?;

    // 2. 删除移动端安全存储中的凭证
    #[cfg(target_os = "android")]
    {
        use crate::keystore_plugin::KeystorePluginHandle;
        use tauri::Manager;

        let keystore = app.state::<KeystorePluginHandle<tauri::Wry>>();
        let _ = keystore.delete(&account_id);

        let path = svc.base_path().join(&account_id).join("keystore_data.json");
        let _ = std::fs::remove_file(&path);

        // 同时清理可能存在的旧版 FileBiometricStorage 凭证
        let legacy_path = svc.base_path().join(&account_id).join("biometric_key");
        let _ = std::fs::remove_file(&legacy_path);
    }

    #[cfg(target_os = "ios")]
    {
        let storage = FileBiometricStorage::new(svc.base_path().clone());
        match storage.delete(&account_id) {
            Ok(()) | Err(BiometricError::KeychainItemNotFound) => {}
            Err(e) => return Err(map_bio_error(e, "delete")),
        }
    }

    // 3. 更新配置标记
    manager
        .set_config_flag(&account_id, false)
        .map_err(|e| map_bio_error(e, "delete"))?;

    // 4. 审计日志
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

#[cfg(desktop)]
#[tauri::command]
pub async fn biometric_test(_account_id: String) -> Result<bool, String> {
    if !solosoul_core::biometric::is_macos() && std::env::consts::OS != "windows" {
        return Ok(false);
    }
    // 使用严格策略确保实际触发生物识别，不回落到设备密码。
    let reason = if solosoul_core::biometric::is_macos() {
        "test biometric authentication for SoloSoul"
    } else {
        "Test Windows Hello for SoloSoul"
    };
    trigger_system_biometric(reason, true).map_err(|e| map_bio_error(e, "test"))?;
    Ok(true)
}

#[cfg(all(mobile, any(target_os = "android", target_os = "ios")))]
#[tauri::command]
pub async fn biometric_test(app: tauri::AppHandle, _account_id: String) -> Result<bool, String> {
    use tauri_plugin_biometric::{AuthOptions, BiometricExt};

    app.biometric()
        .authenticate(
            "Test biometric authentication for SoloSoul".to_string(),
            AuthOptions {
                allow_device_credential: false,
                cancel_title: Some("Cancel".to_string()),
                fallback_title: None,
                title: Some("SoloSoul".to_string()),
                subtitle: Some("Test biometric authentication".to_string()),
                confirmation_required: Some(false),
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(true)
}

#[cfg(all(test, desktop))]
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
