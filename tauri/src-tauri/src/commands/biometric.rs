//! Biometric (Touch ID/Face ID/Windows Hello) commands (27)
//! macOS: uses objc2 FFI to call LocalAuthentication directly (no Swift compiler needed).
//! Master key stored obfuscated on disk at ~/.solosoul/{account_id}/biometric_key.

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiometricAvailability {
    pub available: bool,
    pub configured: bool,
    pub biometry_type: Option<String>,
    pub error: Option<String>,
}

fn is_macos() -> bool {
    std::env::consts::OS == "macos"
}
const BIO_OBF: &[u8; 32] = b"Solosoul_biometric_obfuscate_v1!";

fn solosoul_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()))
        .join(".solosoul")
}
fn acct_dir(id: &str) -> std::path::PathBuf {
    solosoul_dir().join(id)
}
fn bio_key_path(id: &str) -> std::path::PathBuf {
    acct_dir(id).join("biometric_key")
}

fn save_master_key(account_id: &str, key_hex: &str) -> Result<(), String> {
    let key_bytes = hex::decode(key_hex).map_err(|e| e.to_string())?;
    let obf: Vec<u8> = key_bytes
        .iter()
        .enumerate()
        .map(|(i, b)| b ^ BIO_OBF[i % 32])
        .collect();
    let path = bio_key_path(account_id);
    std::fs::write(&path, hex::encode(&obf))
        .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path)
            .map_err(|e| format!("Failed to stat {}: {}", path.display(), e))?
            .permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&path, perms)
            .map_err(|e| format!("Failed to chmod {}: {}", path.display(), e))?;
    }
    Ok(())
}

fn read_master_key(account_id: &str) -> Result<String, String> {
    let path = bio_key_path(account_id);
    let hex_str = std::fs::read_to_string(&path)
        .map_err(|e| format!("No key file at {}: {}", path.display(), e))?;
    let obf = hex::decode(hex_str.trim()).map_err(|e| e.to_string())?;
    let key: Vec<u8> = obf
        .iter()
        .enumerate()
        .map(|(i, b)| b ^ BIO_OBF[i % 32])
        .collect();
    Ok(hex::encode(&key))
}

fn delete_master_key(account_id: &str) {
    let p = bio_key_path(account_id);
    if p.exists() {
        let _ = std::fs::remove_file(&p);
    }
}

fn trigger_system_biometric(reason: &str) -> Result<(), String> {
    let _ = reason;
    if !is_macos() {
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    return trigger_macos_biometric(reason);
    #[cfg(not(target_os = "macos"))]
    Ok(())
}

#[cfg(target_os = "macos")]
fn trigger_macos_biometric(reason: &str) -> Result<(), String> {
    use std::ffi::{c_void, CStr, CString};
    use std::sync::mpsc;

    use block2::RcBlock;
    use objc2::runtime::{AnyClass, NSObject};
    use objc2::{msg_send, sel};

    let la_name = c"LAContext";
    let la_cls = AnyClass::get(la_name).ok_or_else(|| "Touch ID not available".to_string())?;

    let ctx: *mut NSObject = unsafe {
        let alloc: *mut NSObject = msg_send![la_cls, alloc];
        msg_send![alloc, init]
    };
    if ctx.is_null() {
        return Err("failed to initialise LAContext".to_string());
    }

    let c_reason = CString::new(reason).map_err(|_| "invalid reason string".to_string())?;
    let ns_name = c"NSString";
    let ns_cls = AnyClass::get(ns_name).ok_or_else(|| "NSString class not found".to_string())?;
    let ns_reason: *mut NSObject = unsafe {
        let alloc: *mut NSObject = msg_send![ns_cls, alloc];
        msg_send![alloc, initWithUTF8String: c_reason.as_ptr()]
    };
    if ns_reason.is_null() {
        return Err("failed to create NSString".to_string());
    }

    let (tx, rx) = mpsc::channel::<bool>();

    let block = RcBlock::new(move |success: i8, _error: *mut c_void| {
        let _ = tx.send(success != 0);
    });

    // LAPolicyDeviceOwnerAuthenticationWithBiometrics = 1 (NSInteger)
    unsafe {
        let _: () = msg_send![
            ctx,
            evaluatePolicy: 1i64,
            localizedReason: ns_reason,
            reply: &*block,
        ];
    }

    let success = rx
        .recv()
        .map_err(|_| "Touch ID dialog interrupted".to_string())?;

    // Release manually-owned ObjC objects (MRC)
    unsafe {
        let _: () = msg_send![ctx, release];
        let _: () = msg_send![ns_reason, release];
    }

    if success {
        Ok(())
    } else {
        Err("User cancelled or biometric not available".to_string())
    }
}

fn is_configured(account_id: &str) -> bool {
    let has_flag = std::fs::read_to_string(acct_dir(account_id).join("config.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("biometricEnabled").and_then(|v| v.as_bool()))
        .unwrap_or(false);
    has_flag && bio_key_path(account_id).exists()
}

fn set_config_flag(account_id: &str, enabled: bool) -> Result<(), String> {
    let p = acct_dir(account_id).join("config.json");
    let s = std::fs::read_to_string(&p).map_err(|_| "Account not found")?;
    let mut v: serde_json::Value = serde_json::from_str(&s).map_err(|_| "Parse error")?;
    v["biometricEnabled"] = serde_json::Value::Bool(enabled);
    std::fs::write(
        &p,
        serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

fn derive_master_key(password: &str, account_id: &str) -> Result<String, String> {
    let s = std::fs::read_to_string(acct_dir(account_id).join("config.json"))
        .map_err(|_| "Account not found")?;
    let cfg: crate::services::vault_service::AccountConfig =
        serde_json::from_str(&s).map_err(|_| "Parse error")?;
    let salt_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &cfg.salt)
        .map_err(|_| "Invalid salt")?;
    let salt: [u8; 16] = salt_bytes
        .as_slice()
        .try_into()
        .map_err(|_| "Bad salt len")?;
    let k = solosoul_crypto::kdf::KdfConfig::balanced();
    let mk = solosoul_crypto::kdf::derive_key(password, &salt, &k).map_err(|_| "KDF failed")?;
    Ok(hex::encode(mk.as_slice()))
}

fn verify_password(password: &str, account_id: &str) -> Result<(), String> {
    let s = std::fs::read_to_string(acct_dir(account_id).join("config.json"))
        .map_err(|_| "Account not found")?;
    let cfg: crate::services::vault_service::AccountConfig =
        serde_json::from_str(&s).map_err(|_| "Parse error")?;
    if crate::commands::auth::verify_password_core(password, &cfg)? {
        Ok(())
    } else {
        Err("Invalid password".into())
    }
}

// IPC Commands

#[tauri::command]
pub async fn biometric_check_availability(
    account_id: String,
) -> Result<BiometricAvailability, String> {
    let bt = if is_macos() {
        Some("touchId".into())
    } else {
        None
    };
    Ok(BiometricAvailability {
        available: bt.is_some(),
        configured: is_configured(&account_id),
        biometry_type: bt,
        error: if is_macos() {
            None
        } else {
            Some("platform not supported".into())
        },
    })
}

#[tauri::command]
pub async fn biometric_save_credential(
    state: State<'_, AppState>,
    account_id: String,
    password: String,
    silent: Option<bool>,
    location: Option<String>,
    action: Option<String>,
    biometry_type: Option<String>,
) -> Result<(), String> {
    if !is_macos() {
        return Err("platform not supported".into());
    }
    verify_password(&password, &account_id)?;
    if !silent.unwrap_or(false) {
        trigger_system_biometric("verify your identity to enable Touch ID for SoloSoul")?;
    }
    let key_hex = derive_master_key(&password, &account_id)?;
    save_master_key(&account_id, &key_hex)?;
    set_config_flag(&account_id, true)?;
    let svc = state.vault_service.read().unwrap();
    if let Some(vg) = svc.get_vault_store() {
        let vault = vg.as_ref();
        {
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
    if !is_macos() {
        return Err("platform not supported".into());
    }
    trigger_system_biometric("unlock SoloSoul")?;
    let key_hex = read_master_key(&account_id)?;
    let key_bytes = hex::decode(&key_hex).map_err(|_| "Bad key format")?;
    let key: [u8; 32] = key_bytes.as_slice().try_into().map_err(|_| "Bad key len")?;
    let svc = state.vault_service.read().unwrap();
    svc.unlock_with_session_key(&account_id, &key)?;
    if let Some(vg) = svc.get_vault_store() {
        let vault = vg.as_ref();
        {
            let loc = location.unwrap_or_else(|| "unknown".to_string());
            let act = action.unwrap_or_else(|| "unlock".to_string());
            let bio_type = biometry_type.as_deref().unwrap_or("unknown");
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
    if !is_macos() {
        return Err("platform not supported".into());
    }
    verify_password(&password, &account_id)?;
    delete_master_key(&account_id);
    set_config_flag(&account_id, false)?;
    let svc = state.vault_service.read().unwrap();
    if let Some(vg) = svc.get_vault_store() {
        let vault = vg.as_ref();
        {
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
    }
    Ok(())
}

#[tauri::command]
pub async fn biometric_test(_account_id: String) -> Result<bool, String> {
    if !is_macos() {
        return Ok(false);
    }
    trigger_system_biometric("test biometric authentication for SoloSoul")?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    static HOME_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn with_temp_home<F>(f: F)
    where
        F: FnOnce(&std::path::Path),
    {
        let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = TempDir::new().unwrap();
        let solosoul = dir.path().join(".solosoul");
        std::fs::create_dir_all(&solosoul).unwrap();
        let original = std::env::var("HOME").ok();
        std::env::set_var("HOME", dir.path());
        f(dir.path());
        match original {
            Some(h) => std::env::set_var("HOME", h),
            None => std::env::remove_var("HOME"),
        }
    }

    fn create_test_account_config(
        password: &str,
    ) -> (crate::services::vault_service::AccountConfig, String) {
        let salt = solosoul_crypto::kdf::generate_salt();
        let salt_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &salt);
        let config = solosoul_crypto::kdf::KdfConfig::balanced();
        let master_key = solosoul_crypto::kdf::derive_key(password, &salt, &config).unwrap();
        let master_key_hex = hex::encode(master_key.as_slice());

        let verify_config = solosoul_crypto::kdf::KdfConfig {
            memory_kb: 8192,
            iterations: 1,
            parallelism: 1,
        };
        let verify_key = solosoul_crypto::kdf::derive_key(
            &master_key_hex,
            b"SOLOSOUL_VAULT_VERIFY_v1",
            &verify_config,
        )
        .unwrap();
        let verify_hash = hex::encode(verify_key.as_slice());

        let cfg = crate::services::vault_service::AccountConfig {
            account_id: "test_acc".to_string(),
            name: "Test".to_string(),
            salt: salt_b64,
            verify_hash,
            created_at: chrono::Utc::now().to_rfc3339(),
            crypto_version: 2,
            password_hint: None,
            last_login_at: None,
            last_operation_at: None,
            last_operation_desc: None,
            biometric_enabled: false,
        };
        (cfg, master_key_hex)
    }

    #[test]
    fn test_biometric_availability_serde_roundtrip() {
        let original = BiometricAvailability {
            available: true,
            configured: false,
            biometry_type: Some("touchId".to_string()),
            error: None,
        };
        let json = serde_json::to_string(&original).unwrap();
        assert!(json.contains("touchId"));
        let restored: BiometricAvailability = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.available, original.available);
        assert_eq!(restored.configured, original.configured);
        assert_eq!(restored.biometry_type, original.biometry_type);
    }

    #[test]
    fn test_is_macos() {
        let expected = std::env::consts::OS == "macos";
        assert_eq!(is_macos(), expected);
    }

    #[test]
    fn test_master_key_obfuscation_roundtrip() {
        with_temp_home(|_path| {
            let account_id = "acc-1";
            std::fs::create_dir_all(acct_dir(account_id)).unwrap();
            let key_hex = "deadbeef1234567890abcdef1234567890abcdef1234567890abcdef12345678";
            save_master_key(account_id, key_hex).unwrap();
            let read_back = read_master_key(account_id).unwrap();
            assert_eq!(read_back, key_hex);

            delete_master_key(account_id);
            assert!(read_master_key(account_id).is_err());
        });
    }

    #[test]
    fn test_is_configured_and_set_config_flag() {
        with_temp_home(|_path| {
            let account_id = "acc-2";
            let acct_path = acct_dir(account_id);
            std::fs::create_dir_all(&acct_path).unwrap();
            let config = serde_json::json!({
                "accountId": account_id,
                "name": "Test",
                "salt": "c2FsdDEyMzQ1Njc=",
                "verifyHash": "abcd",
                "createdAt": "2024-01-01T00:00:00Z",
                "cryptoVersion": 2,
                "biometricEnabled": false,
            });
            std::fs::write(
                acct_path.join("config.json"),
                serde_json::to_string_pretty(&config).unwrap(),
            )
            .unwrap();

            assert!(!is_configured(account_id));

            // Enable flag and create key file
            set_config_flag(account_id, true).unwrap();
            save_master_key(account_id, "aabbccdd").unwrap();

            assert!(is_configured(account_id));

            // Disable flag
            set_config_flag(account_id, false).unwrap();
            assert!(!is_configured(account_id));
        });
    }

    #[test]
    fn test_set_config_flag_missing_account() {
        with_temp_home(|_path| {
            let result = set_config_flag("nonexistent", true);
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_derive_master_key() {
        with_temp_home(|_path| {
            let password = "testpassword123";
            let (cfg, expected_hex) = create_test_account_config(password);
            let account_id = "acc-derive";
            let acct_path = acct_dir(account_id);
            std::fs::create_dir_all(&acct_path).unwrap();
            std::fs::write(
                acct_path.join("config.json"),
                serde_json::to_string_pretty(&cfg).unwrap(),
            )
            .unwrap();

            let derived = derive_master_key(password, account_id).unwrap();
            assert_eq!(derived, expected_hex);
        });
    }

    #[test]
    fn test_verify_password_success() {
        with_temp_home(|_path| {
            let password = "mypassword456";
            let (cfg, _expected_hex) = create_test_account_config(password);
            let account_id = "acc-verify";
            let acct_path = acct_dir(account_id);
            std::fs::create_dir_all(&acct_path).unwrap();
            std::fs::write(
                acct_path.join("config.json"),
                serde_json::to_string_pretty(&cfg).unwrap(),
            )
            .unwrap();

            assert!(verify_password(password, account_id).is_ok());
        });
    }

    #[test]
    fn test_verify_password_failure() {
        with_temp_home(|_path| {
            let password = "correctpassword";
            let (cfg, _expected_hex) = create_test_account_config(password);
            let account_id = "acc-verify-fail";
            let acct_path = acct_dir(account_id);
            std::fs::create_dir_all(&acct_path).unwrap();
            std::fs::write(
                acct_path.join("config.json"),
                serde_json::to_string_pretty(&cfg).unwrap(),
            )
            .unwrap();

            assert!(verify_password("wrongpassword", account_id).is_err());
        });
    }
}
