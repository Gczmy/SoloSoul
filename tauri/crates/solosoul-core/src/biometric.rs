//! Biometric (Touch ID / Face ID / Windows Hello) primitives.
//!
//! This module is host-agnostic: it knows how to store an obfuscated master key,
//! trigger the OS biometric dialog, and unlock a `VaultService`. The Tauri
//! command wrappers live in `src-tauri/src/commands/biometric.rs` and only
//! forward parameters plus emit events if needed.

use crate::auth::verify_password_core;
use crate::vault_service::{AccountConfig, VaultService};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiometricAvailability {
    pub available: bool,
    pub configured: bool,
    pub biometry_type: Option<String>,
    pub error: Option<String>,
}

const BIO_OBF: &[u8; 32] = b"Solosoul_biometric_obfuscate_v1!";

/// Host-agnostic manager for biometric credentials.
#[derive(Debug, Clone)]
pub struct BiometricManager {
    base_path: PathBuf,
}

impl BiometricManager {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    fn account_dir(&self, account_id: &str) -> PathBuf {
        self.base_path.join(account_id)
    }

    fn bio_key_path(&self, account_id: &str) -> PathBuf {
        self.account_dir(account_id).join("biometric_key")
    }

    fn config_path(&self, account_id: &str) -> PathBuf {
        self.account_dir(account_id).join("config.json")
    }

    /// Check whether biometric authentication is available and configured for
    /// the given account. `available` refers to the device/platform; `configured`
    /// refers to whether this account has a stored credential.
    pub fn availability(&self, account_id: &str) -> BiometricAvailability {
        let bt = if is_macos() {
            Some("touchId".into())
        } else {
            None
        };
        let configured = self.is_configured(account_id);
        let available = bt.is_some();
        BiometricAvailability {
            available,
            configured,
            biometry_type: bt,
            error: if is_macos() {
                None
            } else {
                Some("platform not supported".into())
            },
        }
    }

    /// Save a biometric credential for the account after verifying the password
    /// and asking the user to authenticate biometrically (unless `silent`).
    pub fn save_credential(
        &self,
        account_id: &str,
        password: &str,
        silent: bool,
        reason: &str,
    ) -> Result<(), String> {
        if !is_macos() {
            return Err("platform not supported".into());
        }
        self.verify_password(password, account_id)?;
        if !silent {
            trigger_system_biometric(reason)?;
        }
        let key_hex = derive_master_key(password, account_id, &self.base_path)?;
        save_master_key(&self.bio_key_path(account_id), account_id, &key_hex)?;
        set_config_flag(&self.config_path(account_id), true)?;
        Ok(())
    }

    /// Unlock the vault using the stored biometric key. Returns the biometry
    /// type that was used (e.g. `"touchId"`).
    pub fn unlock(
        &self,
        account_id: &str,
        vault_service: &VaultService,
        reason: &str,
    ) -> Result<String, String> {
        if !is_macos() {
            return Err("platform not supported".into());
        }
        trigger_system_biometric(reason)?;
        let key_hex = read_master_key(&self.bio_key_path(account_id), account_id)?;
        let key_bytes = hex::decode(&key_hex).map_err(|_| "Bad key format")?;
        let key: [u8; 32] = key_bytes.as_slice().try_into().map_err(|_| "Bad key len")?;
        vault_service.unlock_with_session_key(account_id, &key)?;
        Ok(self
            .availability(account_id)
            .biometry_type
            .unwrap_or_else(|| "unknown".into()))
    }

    /// Delete the stored biometric credential after password verification.
    pub fn delete_credential(&self, account_id: &str, password: &str) -> Result<(), String> {
        if !is_macos() {
            return Err("platform not supported".into());
        }
        self.verify_password(password, account_id)?;
        delete_master_key(&self.bio_key_path(account_id), account_id);
        set_config_flag(&self.config_path(account_id), false)?;
        Ok(())
    }

    /// Trigger the system biometric dialog as a self-test.
    pub fn test(&self, reason: &str) -> Result<bool, String> {
        if !is_macos() {
            return Ok(false);
        }
        trigger_system_biometric(reason)?;
        Ok(true)
    }

    /// Verify the master password for the account. Wrapper exposed so hosts can
    /// reuse the same logic.
    pub fn verify_password(&self, password: &str, account_id: &str) -> Result<(), String> {
        let cfg = read_account_config(account_id, &self.base_path)?;
        if verify_password_core(password, &cfg)? {
            Ok(())
        } else {
            Err("Invalid password".into())
        }
    }

    /// Return true if the account has enabled biometric AND a stored key exists.
    pub fn is_configured(&self, account_id: &str) -> bool {
        let config_path = self.config_path(account_id);
        let has_flag = std::fs::read_to_string(&config_path)
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| v.get("biometricEnabled").and_then(|v| v.as_bool()))
            .unwrap_or(false);
        let has_key = has_stored_master_key(&self.bio_key_path(account_id), account_id);
        tracing::debug!(
            "biometric is_configured for {}: flag={}, key={}, config_path={}",
            account_id,
            has_flag,
            has_key,
            config_path.display()
        );
        has_flag && has_key
    }
}

pub fn is_macos() -> bool {
    std::env::consts::OS == "macos"
}

fn use_keyring() -> bool {
    !cfg!(test)
}

fn keyring_entry(account_id: &str) -> Result<keyring::Entry, String> {
    let service = format!("solosoul_biometric_{}", account_id);
    keyring::Entry::new(&service, account_id).map_err(|e| e.to_string())
}

fn write_obfuscated_key_file(path: &Path, key_hex: &str) -> Result<(), String> {
    let key_bytes = hex::decode(key_hex).map_err(|e| e.to_string())?;
    let obf: Vec<u8> = key_bytes
        .iter()
        .enumerate()
        .map(|(i, b)| b ^ BIO_OBF[i % 32])
        .collect();
    std::fs::create_dir_all(path.parent().unwrap())
        .map_err(|e| format!("Failed to create {}: {}", path.display(), e))?;
    std::fs::write(path, hex::encode(&obf))
        .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)
            .map_err(|e| format!("Failed to stat {}: {}", path.display(), e))?
            .permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(path, perms)
            .map_err(|e| format!("Failed to chmod {}: {}", path.display(), e))?;
    }
    Ok(())
}

fn save_master_key(path: &Path, account_id: &str, key_hex: &str) -> Result<(), String> {
    // Always keep an obfuscated file backup. The OS keychain is the primary
    // store, but keychain reads can fail after app restart/lock on some macOS
    // configurations; the backup lets us still report biometric as configured.
    write_obfuscated_key_file(path, key_hex)?;

    if use_keyring() {
        match keyring_entry(account_id) {
            Ok(entry) => {
                if let Err(e) = entry.set_password(key_hex) {
                    tracing::warn!(
                        "Failed to write biometric key to keychain for {}: {}; file backup is available",
                        account_id,
                        e
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to create keychain entry for {}: {}; relying on file backup",
                    account_id,
                    e
                );
            }
        }
    }
    Ok(())
}

fn read_master_key_from_file(path: &Path) -> Result<String, String> {
    let hex_str = std::fs::read_to_string(path)
        .map_err(|e| format!("No key file at {}: {}", path.display(), e))?;
    let obf = hex::decode(hex_str.trim()).map_err(|e| e.to_string())?;
    let key: Vec<u8> = obf
        .iter()
        .enumerate()
        .map(|(i, b)| b ^ BIO_OBF[i % 32])
        .collect();
    Ok(hex::encode(&key))
}

fn read_master_key(path: &Path, account_id: &str) -> Result<String, String> {
    if use_keyring() {
        let entry = keyring_entry(account_id)?;
        match entry.get_password() {
            Ok(key) => return Ok(key),
            Err(_) => {
                // Backwards compatibility: migrate a legacy file-stored key into
                // the OS keychain on first read.
                let key = read_master_key_from_file(path)?;
                let _ = entry.set_password(&key);
                return Ok(key);
            }
        }
    }

    read_master_key_from_file(path)
}

fn delete_master_key(path: &Path, account_id: &str) {
    if use_keyring() {
        let _ = keyring_entry(account_id)
            .and_then(|e| e.delete_credential().map_err(|e| e.to_string()));
    }
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
}

fn has_stored_master_key(path: &Path, account_id: &str) -> bool {
    if use_keyring() {
        if let Ok(entry) = keyring_entry(account_id) {
            if entry.get_password().is_ok() {
                return true;
            }
        }
        // Legacy file-based key still counts as configured and will be migrated on unlock.
        return path.exists();
    }
    path.exists()
}

fn set_config_flag(config_path: &Path, enabled: bool) -> Result<(), String> {
    let s = std::fs::read_to_string(config_path).map_err(|_| "Account not found")?;
    let mut v: serde_json::Value = serde_json::from_str(&s).map_err(|_| "Parse error")?;
    v["biometricEnabled"] = serde_json::Value::Bool(enabled);
    std::fs::write(
        config_path,
        serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

fn read_account_config(account_id: &str, base_path: &Path) -> Result<AccountConfig, String> {
    let p = base_path.join(account_id).join("config.json");
    let s = std::fs::read_to_string(&p).map_err(|_| "Account not found")?;
    serde_json::from_str(&s).map_err(|_| "Parse error".into())
}

fn derive_master_key(password: &str, account_id: &str, base_path: &Path) -> Result<String, String> {
    let cfg = read_account_config(account_id, base_path)?;
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

pub fn trigger_system_biometric(reason: &str) -> Result<(), String> {
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
    use std::ffi::{c_void, CString};
    use std::sync::mpsc;

    use block2::RcBlock;
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, NSObject};

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

    fn manager_from_home() -> BiometricManager {
        let home = std::env::var("HOME").unwrap();
        BiometricManager::new(std::path::PathBuf::from(home).join(".solosoul"))
    }

    fn create_test_account_config(password: &str) -> (crate::vault_service::AccountConfig, String) {
        let salt = solosoul_crypto::kdf::generate_salt();
        let salt_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, salt);
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

        let cfg = crate::vault_service::AccountConfig {
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
            let manager = manager_from_home();
            let account_id = "acc-1";
            std::fs::create_dir_all(manager.account_dir(account_id)).unwrap();
            let key_hex = "deadbeef1234567890abcdef1234567890abcdef1234567890abcdef12345678";
            save_master_key(&manager.bio_key_path(account_id), account_id, key_hex).unwrap();
            let read_back = read_master_key(&manager.bio_key_path(account_id), account_id).unwrap();
            assert_eq!(read_back, key_hex);

            delete_master_key(&manager.bio_key_path(account_id), account_id);
            assert!(read_master_key(&manager.bio_key_path(account_id), account_id).is_err());
        });
    }

    #[test]
    fn test_is_configured_and_set_config_flag() {
        with_temp_home(|_path| {
            let manager = manager_from_home();
            let account_id = "acc-2";
            let acct_path = manager.account_dir(account_id);
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

            assert!(!manager.is_configured(account_id));

            // Enable flag and create key file
            set_config_flag(&manager.config_path(account_id), true).unwrap();
            save_master_key(&manager.bio_key_path(account_id), account_id, "aabbccdd").unwrap();

            assert!(manager.is_configured(account_id));

            // Disable flag
            set_config_flag(&manager.config_path(account_id), false).unwrap();
            assert!(!manager.is_configured(account_id));
        });
    }

    #[test]
    fn test_set_config_flag_missing_account() {
        with_temp_home(|_path| {
            let manager = manager_from_home();
            let result = set_config_flag(&manager.config_path("nonexistent"), true);
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_derive_master_key() {
        with_temp_home(|path| {
            let password = "testpassword123";
            let (cfg, expected_hex) = create_test_account_config(password);
            let account_id = "acc-derive";
            let acct_path = path.join(".solosoul").join(account_id);
            std::fs::create_dir_all(&acct_path).unwrap();
            std::fs::write(
                acct_path.join("config.json"),
                serde_json::to_string_pretty(&cfg).unwrap(),
            )
            .unwrap();

            let derived = derive_master_key(password, account_id, &path.join(".solosoul")).unwrap();
            assert_eq!(derived, expected_hex);
        });
    }

    #[test]
    fn test_verify_password_success() {
        with_temp_home(|path| {
            let password = "mypassword456";
            let (cfg, _expected_hex) = create_test_account_config(password);
            let account_id = "acc-verify";
            let acct_path = path.join(".solosoul").join(account_id);
            std::fs::create_dir_all(&acct_path).unwrap();
            std::fs::write(
                acct_path.join("config.json"),
                serde_json::to_string_pretty(&cfg).unwrap(),
            )
            .unwrap();

            let manager = BiometricManager::new(path.join(".solosoul"));
            assert!(manager.verify_password(password, account_id).is_ok());
        });
    }

    #[test]
    fn test_verify_password_failure() {
        with_temp_home(|path| {
            let password = "correctpassword";
            let (cfg, _expected_hex) = create_test_account_config(password);
            let account_id = "acc-verify-fail";
            let acct_path = path.join(".solosoul").join(account_id);
            std::fs::create_dir_all(&acct_path).unwrap();
            std::fs::write(
                acct_path.join("config.json"),
                serde_json::to_string_pretty(&cfg).unwrap(),
            )
            .unwrap();

            let manager = BiometricManager::new(path.join(".solosoul"));
            assert!(manager
                .verify_password("wrongpassword", account_id)
                .is_err());
        });
    }
}
