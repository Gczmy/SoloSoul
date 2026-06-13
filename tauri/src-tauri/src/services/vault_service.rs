//! Vault service - manages accounts and vault lifecycle.
//! Stores accounts in ~/.solosoul/ with per-account config and vault.db

use serde::{Deserialize, Serialize};
use solosoul_crypto::kdf::{derive_key, generate_salt, KdfConfig};
use solosoul_vault::{VaultConfig, VaultStore};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::RwLock;
use zeroize::{Zeroize, Zeroizing};

#[cfg(unix)]
fn set_private_dir(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let meta = fs::metadata(path).map_err(|e| e.to_string())?;
    let mut perms = meta.permissions();
    perms.set_mode(0o700);
    fs::set_permissions(path, perms).map_err(|e| e.to_string())
}

#[cfg(unix)]
fn set_private_file(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let meta = fs::metadata(path).map_err(|e| e.to_string())?;
    let mut perms = meta.permissions();
    perms.set_mode(0o600);
    fs::set_permissions(path, perms).map_err(|e| e.to_string())
}

#[cfg(not(unix))]
fn set_private_dir(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(not(unix))]
fn set_private_file(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountConfig {
    pub account_id: String,
    pub name: String,
    pub salt: String,        // base64
    pub verify_hash: String, // hex
    pub created_at: String,
    pub crypto_version: u32,
    pub password_hint: Option<String>,
    pub last_login_at: Option<String>,
    pub last_operation_at: Option<String>,
    pub last_operation_desc: Option<String>,
    #[serde(default)]
    pub biometric_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AccountEntry {
    id: String,
    name: String,
    created_at: String,
    last_accessed: Option<String>,
}

pub struct VaultService {
    base_path: PathBuf,
    accounts_cache: RwLock<HashMap<String, AccountEntry>>,
    session_key: RwLock<Option<Zeroizing<[u8; 32]>>>,
    unlocked_account: RwLock<Option<String>>,
    vault_store: RwLock<Option<VaultStore>>,
}

impl VaultService {
    pub fn new() -> Self {
        let base_path = Self::default_base_path();
        let svc = Self {
            base_path,
            accounts_cache: RwLock::new(HashMap::new()),
            session_key: RwLock::new(None),
            unlocked_account: RwLock::new(None),
            vault_store: RwLock::new(None),
        };
        svc.load_accounts();
        svc
    }

    fn default_base_path() -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            if let Ok(profile) = std::env::var("USERPROFILE") {
                return PathBuf::from(profile).join(".solosoul");
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".solosoul")
        } else {
            PathBuf::from("/tmp/solosoul")
        }
    }

    pub fn base_path(&self) -> &PathBuf {
        &self.base_path
    }

    fn accounts_file(&self) -> PathBuf {
        self.base_path.join("accounts.json")
    }

    fn account_dir(&self, id: &str) -> PathBuf {
        self.base_path.join(id)
    }

    fn config_path(&self, id: &str) -> PathBuf {
        self.account_dir(id).join("config.json")
    }

    fn load_accounts(&self) {
        let file = self.accounts_file();
        if !file.exists() {
            return;
        }
        if let Ok(content) = fs::read_to_string(&file) {
            if let Ok(accounts) = serde_json::from_str::<Vec<AccountEntry>>(&content) {
                if let Ok(mut cache) = self.accounts_cache.write() {
                    for a in accounts {
                        cache.insert(a.id.clone(), a);
                    }
                }
            }
        }
    }

    fn save_accounts(&self) -> Result<(), String> {
        let cache = self.accounts_cache.read().map_err(|e| e.to_string())?;
        let list: Vec<&AccountEntry> = cache.values().collect();
        let content = serde_json::to_string_pretty(&list).map_err(|e| e.to_string())?;
        fs::create_dir_all(&self.base_path).map_err(|e| e.to_string())?;
        set_private_dir(&self.base_path)?;
        let file = self.accounts_file();
        fs::write(&file, content).map_err(|e| e.to_string())?;
        set_private_file(&file)?;
        Ok(())
    }

    pub fn has_any_account(&self) -> bool {
        self.accounts_cache
            .read()
            .map(|c| !c.is_empty())
            .unwrap_or(false)
    }

    pub fn list_accounts(&self) -> Vec<serde_json::Value> {
        let cache = self.accounts_cache.read().ok();
        let accounts = match cache {
            Some(ref c) => c.values().cloned().collect::<Vec<_>>(),
            None => return vec![],
        };
        let mut result = Vec::new();
        for entry in &accounts {
            let config_path = self.config_path(&entry.id);
            let (salt, verify_hash, password_hint, created_at) = if config_path.exists() {
                if let Ok(content) = fs::read_to_string(&config_path) {
                    if let Ok(cfg) = serde_json::from_str::<AccountConfig>(&content) {
                        (
                            Some(cfg.salt),
                            Some(cfg.verify_hash),
                            cfg.password_hint,
                            Some(cfg.created_at),
                        )
                    } else {
                        (None, None, None, None)
                    }
                } else {
                    (None, None, None, None)
                }
            } else {
                (None, None, None, None)
            };

            result.push(serde_json::json!({
                "id": entry.id, "name": entry.name,
                "salt": salt, "verifyHash": verify_hash,
                "passwordHint": password_hint, "createdAt": created_at,
                "lastAccessed": entry.last_accessed,
            }));
        }
        result
    }

    pub fn create_account(
        &self,
        name: &str,
        password: &str,
        password_hint: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        if name.trim().is_empty() {
            return Err("Account name is required".to_string());
        }
        if password.len() < 8 {
            return Err("Password must be at least 8 characters".to_string());
        }

        let cache = self.accounts_cache.read().map_err(|e| e.to_string())?;
        if cache
            .values()
            .any(|a| a.name.to_lowercase() == name.to_lowercase())
        {
            return Err("Account name already taken".to_string());
        }
        drop(cache);

        let account_id = format!(
            "acc_{}",
            &uuid::Uuid::new_v4().to_string().replace("-", "")[..16]
        );
        let salt = generate_salt();
        let config = KdfConfig::balanced();
        let master_key = derive_key(password, &salt, &config)
            .map_err(|e| format!("Key derivation failed: {}", e))?;

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
        .map_err(|e| format!("Verify key derivation failed: {}", e))?;
        let verify_hash = hex::encode(verify_key.as_slice());

        let dir = self.account_dir(&account_id);
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        set_private_dir(&dir)?;

        let now = chrono::Utc::now().to_rfc3339();
        let config_data = AccountConfig {
            account_id: account_id.clone(),
            name: name.to_string(),
            salt: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                salt.as_slice(),
            ),
            verify_hash,
            created_at: now.clone(),
            crypto_version: 2,
            biometric_enabled: false,
            password_hint: password_hint.map(|s| s.to_string()),
            last_login_at: Some(now.clone()),
            last_operation_at: None,
            last_operation_desc: None,
        };
        let config_path = self.config_path(&account_id);
        let config_json = serde_json::to_string_pretty(&config_data).map_err(|e| e.to_string())?;
        fs::write(&config_path, config_json).map_err(|e| e.to_string())?;
        set_private_file(&config_path)?;

        // Add to cache
        let entry = AccountEntry {
            id: account_id.clone(),
            name: name.to_string(),
            created_at: now.clone(),
            last_accessed: Some(now),
        };
        if let Ok(mut cache) = self.accounts_cache.write() {
            cache.insert(account_id.clone(), entry);
        }
        self.save_accounts()?;

        // Open vault
        let vault_config = VaultConfig::new(&account_id, self.account_dir(&account_id));
        let vault =
            VaultStore::open(vault_config).map_err(|e| format!("Failed to open vault: {}", e))?;
        if let Ok(mut store) = self.vault_store.write() {
            *store = Some(vault);
        }
        if let Ok(mut key) = self.session_key.write() {
            *key = Some(Zeroizing::new(
                master_key
                    .as_slice()
                    .try_into()
                    .expect("HKDF output must be 32 bytes"),
            ));
        }
        if let Ok(mut ua) = self.unlocked_account.write() {
            *ua = Some(account_id.clone());
        }

        Ok(serde_json::json!({
            "id": account_id, "name": name,
            "salt": config_data.salt, "verifyHash": config_data.verify_hash,
            "passwordHint": config_data.password_hint,
        }))
    }

    pub fn unlock(&self, account_id: &str, password: &str) -> Result<(), String> {
        let config_path = self.config_path(account_id);
        let content =
            fs::read_to_string(&config_path).map_err(|_| "Account not found".to_string())?;
        let config: AccountConfig =
            serde_json::from_str(&content).map_err(|_| "Config parse error".to_string())?;

        let salt_bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &config.salt)
                .map_err(|_| "Invalid salt".to_string())?;
        let salt_arr: [u8; 16] = salt_bytes
            .as_slice()
            .try_into()
            .map_err(|_| "Invalid salt length".to_string())?;

        let kdf_config = KdfConfig::balanced();
        let master_key = derive_key(password, &salt_arr, &kdf_config)
            .map_err(|_| "Key derivation failed".to_string())?;

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
        let computed_hash = hex::encode(verify_key.as_slice());

        if computed_hash != config.verify_hash {
            return Err("Invalid password".to_string());
        }

        // Update last accessed
        let _now = chrono::Utc::now().to_rfc3339();
        if let Ok(mut cache) = self.accounts_cache.write() {
            if let Some(entry) = cache.get_mut(account_id) {
                entry.last_accessed = Some(chrono::Utc::now().to_rfc3339());
            }
        }
        self.save_accounts().ok();

        // Store session key
        if let Ok(mut key) = self.session_key.write() {
            *key = Some(Zeroizing::new(
                master_key
                    .as_slice()
                    .try_into()
                    .expect("Argon2id output must be 32 bytes"),
            ));
        }
        if let Ok(mut ua) = self.unlocked_account.write() {
            *ua = Some(account_id.to_string());
        }

        // Open vault
        let vault_config = VaultConfig::new(account_id, self.account_dir(account_id));
        let vault =
            VaultStore::open(vault_config).map_err(|e| format!("Failed to open vault: {}", e))?;
        if let Ok(mut store) = self.vault_store.write() {
            *store = Some(vault);
        }

        Ok(())
    }

    pub fn lock(&self) {
        if let Ok(mut store) = self.vault_store.write() {
            if let Some(ref mut v) = *store {
                v.lock();
            }
            store.take();
        }
        if let Ok(mut key) = self.session_key.write() {
            if let Some(mut k) = key.take() {
                k.zeroize();
            }
        }
        if let Ok(mut ua) = self.unlocked_account.write() {
            ua.take();
        }
    }

    pub fn is_unlocked(&self) -> bool {
        let key = self.session_key.read().ok();
        let ua = self.unlocked_account.read().ok();
        key.map(|k| k.is_some()).unwrap_or(false) && ua.map(|u| u.is_some()).unwrap_or(false)
    }

    /// Verify whether the given password matches the account's master password.
    /// Does NOT modify any state (no unlocking, no session key storage).
    pub fn verify_password(&self, account_id: &str, password: &str) -> Result<bool, String> {
        let config_path = self.config_path(account_id);
        let content =
            fs::read_to_string(&config_path).map_err(|_| "Account not found".to_string())?;
        let config: AccountConfig =
            serde_json::from_str(&content).map_err(|_| "Config parse error".to_string())?;

        let salt_bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &config.salt)
                .map_err(|_| "Invalid salt".to_string())?;
        let salt_arr: [u8; 16] = salt_bytes
            .as_slice()
            .try_into()
            .map_err(|_| "Invalid salt length".to_string())?;

        let kdf_config = KdfConfig::balanced();
        let master_key = derive_key(password, &salt_arr, &kdf_config)
            .map_err(|_| "Key derivation failed".to_string())?;

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
        let computed_hash = hex::encode(verify_key.as_slice());

        Ok(computed_hash == config.verify_hash)
    }

    /// Unlock vault with a pre-derived session key (used by biometric unlock).
    /// The session key must match the account's encryption key.
    pub fn unlock_with_session_key(
        &self,
        account_id: &str,
        session_key: &[u8; 32],
    ) -> Result<(), String> {
        // Set session key
        if let Ok(mut key) = self.session_key.write() {
            *key = Some(Zeroizing::new(*session_key));
        }
        if let Ok(mut ua) = self.unlocked_account.write() {
            *ua = Some(account_id.to_string());
        }

        // Open vault
        let vault_config = VaultConfig::new(account_id, self.account_dir(account_id));
        let vault =
            VaultStore::open(vault_config).map_err(|e| format!("Failed to open vault: {}", e))?;
        if let Ok(mut store) = self.vault_store.write() {
            *store = Some(vault);
        }

        Ok(())
    }

    pub fn change_password(
        &self,
        account_id: &str,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), String> {
        // Verify old password first
        self.unlock(account_id, old_password)?;

        // Generate new salt
        let salt = generate_salt();
        let kdf_config = KdfConfig::balanced();
        let new_key = derive_key(new_password, &salt, &kdf_config)
            .map_err(|e| format!("New key derivation failed: {}", e))?;

        let verify_data = b"SOLOSOUL_VAULT_VERIFY_v1";
        let verify_key = derive_key(
            &hex::encode(new_key.as_slice()),
            verify_data,
            &KdfConfig {
                memory_kb: 8192,
                iterations: 1,
                parallelism: 1,
            },
        )
        .map_err(|e| e.to_string())?;
        let verify_hash = hex::encode(verify_key.as_slice());

        // Update config
        let config_path = self.config_path(account_id);
        let content =
            fs::read_to_string(&config_path).map_err(|_| "Account not found".to_string())?;
        let mut config: AccountConfig =
            serde_json::from_str(&content).map_err(|_| "Config parse error".to_string())?;
        config.salt =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, salt.as_slice());
        config.verify_hash = verify_hash;
        let config_json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
        fs::write(&config_path, config_json).map_err(|e| e.to_string())?;

        // Update session key
        let new_key_arr: [u8; 32] = new_key
            .as_slice()
            .try_into()
            .expect("Key derivation output must be 32 bytes");
        {
            if let Ok(mut key) = self.session_key.write() {
                *key = Some(Zeroizing::new(new_key_arr));
            }
        }

        // Re-open vault store with new session key context
        if let Ok(mut store) = self.vault_store.write() {
            *store = None; // Drop old vault connection
        }
        let vault_config = VaultConfig::new(account_id, self.account_dir(account_id));
        match VaultStore::open(vault_config) {
            Ok(vault) => {
                if let Ok(mut store) = self.vault_store.write() {
                    *store = Some(vault);
                }
            }
            Err(e) => {
                return Err(format!("Password updated but vault reopen failed: {}", e));
            }
        }

        // TODO: Re-encrypt existing profiles with new session key
        // Currently stored blobs use the old key; they will fail decryption
        // via encrypt/decrypt_bytes until the application re-encrypts them.
        // This matches Flutter behavior where re-encryption happens at the Dart layer.

        Ok(())
    }

    pub fn delete_account(&self, account_id: &str) -> Result<(), String> {
        self.lock();
        if let Ok(mut cache) = self.accounts_cache.write() {
            cache.remove(account_id);
        }
        self.save_accounts()?;
        let dir = self.account_dir(account_id);
        if dir.exists() {
            fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn get_vault_state(&self) -> String {
        if self.is_unlocked() {
            "unlocked".to_string()
        } else {
            "locked".to_string()
        }
    }

    pub fn get_session_key(&self) -> Option<zeroize::Zeroizing<[u8; 32]>> {
        self.session_key.read().ok()?.clone()
    }

    pub fn get_vault_store(
        &self,
    ) -> Option<std::sync::RwLockReadGuard<'_, Option<solosoul_vault::VaultStore>>> {
        if !self.is_unlocked() {
            return None;
        }
        self.vault_store.read().ok()
    }

    pub fn get_current_account(&self) -> Option<String> {
        self.unlocked_account.read().ok()?.clone()
    }

    pub fn update_password_hint(&self, account_id: &str, hint: &str) -> Result<(), String> {
        let config_path = self.config_path(account_id);
        let content =
            std::fs::read_to_string(&config_path).map_err(|_| "Account not found".to_string())?;
        let mut config: AccountConfig =
            serde_json::from_str(&content).map_err(|_| "Config parse error".to_string())?;
        config.password_hint = Some(hint.to_string());
        let config_json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
        std::fs::write(&config_path, config_json).map_err(|e| e.to_string())?;
        Ok(())
    }
}

impl Default for VaultService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_service() -> (VaultService, TempDir) {
        let dir = TempDir::new().unwrap();
        let base = dir.path().join(".solosoul");
        fs::create_dir_all(&base).unwrap();
        let svc = VaultService {
            base_path: base,
            accounts_cache: RwLock::new(HashMap::new()),
            session_key: RwLock::new(None),
            unlocked_account: RwLock::new(None),
            vault_store: RwLock::new(None),
        };
        (svc, dir)
    }

    #[test]
    fn test_create_account_success() {
        let (svc, _dir) = setup_service();
        let result = svc.create_account("Alice", "password123", None);
        assert!(result.is_ok());
        let account = result.unwrap();
        assert_eq!(account["name"], "Alice");
        assert!(svc.has_any_account());
    }

    #[test]
    fn test_create_account_empty_name_fails() {
        let (svc, _dir) = setup_service();
        let result = svc.create_account("", "password123", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("required"));
    }

    #[test]
    fn test_create_account_short_password_fails() {
        let (svc, _dir) = setup_service();
        let result = svc.create_account("Alice", "short", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("8 characters"));
    }

    #[test]
    fn test_create_account_duplicate_name_fails() {
        let (svc, _dir) = setup_service();
        svc.create_account("Alice", "password123", None).unwrap();
        let result = svc.create_account("alice", "password456", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already taken"));
    }

    #[test]
    fn test_unlock_and_lock() {
        let (svc, _dir) = setup_service();
        let account = svc.create_account("Bob", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap();

        // create_account leaves vault unlocked
        assert_eq!(svc.get_vault_state(), "unlocked");
        svc.lock();
        assert_eq!(svc.get_vault_state(), "locked");
        assert!(!svc.is_unlocked());

        svc.unlock(account_id, "password123").unwrap();
        assert_eq!(svc.get_vault_state(), "unlocked");

        svc.lock();
        assert_eq!(svc.get_vault_state(), "locked");
        assert!(!svc.is_unlocked());
    }

    #[test]
    fn test_unlock_wrong_password_fails() {
        let (svc, _dir) = setup_service();
        let account = svc.create_account("Carol", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap();

        let result = svc.unlock(account_id, "wrongpassword");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid password"));
    }

    #[test]
    fn test_list_accounts() {
        let (svc, _dir) = setup_service();
        svc.create_account("Alice", "password123", None).unwrap();
        svc.create_account("Bob", "password123", None).unwrap();
        let accounts = svc.list_accounts();
        assert_eq!(accounts.len(), 2);
    }

    #[test]
    fn test_verify_password() {
        let (svc, _dir) = setup_service();
        let account = svc.create_account("Dave", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap();

        assert!(svc.verify_password(account_id, "password123").unwrap());
        assert!(!svc.verify_password(account_id, "wrong").unwrap());
    }

    #[test]
    fn test_change_password() {
        let (svc, _dir) = setup_service();
        let account = svc.create_account("Eve", "oldpassword", None).unwrap();
        let account_id = account["id"].as_str().unwrap();

        svc.unlock(account_id, "oldpassword").unwrap();
        svc.change_password(account_id, "oldpassword", "newpassword")
            .unwrap();

        // Old password should fail
        assert!(!svc.verify_password(account_id, "oldpassword").unwrap());
        // New password should succeed
        assert!(svc.verify_password(account_id, "newpassword").unwrap());
    }

    #[test]
    fn test_update_password_hint() {
        let (svc, _dir) = setup_service();
        let account = svc.create_account("Frank", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap();

        svc.update_password_hint(account_id, "My favorite color")
            .unwrap();

        let config_path = svc.config_path(account_id);
        let content = fs::read_to_string(&config_path).unwrap();
        let config: AccountConfig = serde_json::from_str(&content).unwrap();
        assert_eq!(config.password_hint, Some("My favorite color".to_string()));
    }

    #[test]
    fn test_delete_account() {
        let (svc, _dir) = setup_service();
        let account = svc.create_account("Grace", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap();

        svc.delete_account(account_id).unwrap();
        assert!(!svc.has_any_account());
        assert!(!svc.account_dir(account_id).exists());
    }

    #[test]
    fn test_unlock_with_session_key() {
        let (svc, _dir) = setup_service();
        let account = svc.create_account("Hank", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap();

        let session_key = [0u8; 32];
        svc.unlock_with_session_key(account_id, &session_key)
            .unwrap();
        assert_eq!(svc.get_vault_state(), "unlocked");
        assert!(svc.get_session_key().is_some());
    }

    #[test]
    fn test_get_vault_store_when_locked() {
        let (svc, _dir) = setup_service();
        svc.create_account("Ivy", "password123", None).unwrap();
        // create_account leaves vault unlocked; lock first
        svc.lock();
        assert!(svc.get_vault_store().is_none());
    }

    #[test]
    fn test_get_vault_store_when_unlocked() {
        let (svc, _dir) = setup_service();
        let account = svc.create_account("Jack", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap();
        svc.unlock(account_id, "password123").unwrap();
        assert!(svc.get_vault_store().is_some());
    }
}
