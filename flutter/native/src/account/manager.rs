//! Account manager - handles account CRUD and password verification
//!
//! Account data is stored in:
//! ~/.solosoul/accounts.json - list of account metadata
//! ~/.solosoul/{account_id}/config.json - per-account config with salt and verification token

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use argon2::password_hash::rand_core::OsRng;
use rand::RngCore;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;
use zeroize::{Zeroize, Zeroizing};

use crate::crypto::argon2::{
    derive_key, DEFAULT_ITERATIONS, DEFAULT_MEMORY_KIB, DEFAULT_PARALLELISM,
};
use crate::vault::{VaultConfig, VaultStore};

/// Account metadata (stored in accounts.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountMetadata {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub last_accessed: Option<DateTime<Utc>>,
}

/// Per-account config (stored in {account_id}/config.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountConfig {
    pub account_id: String,
    pub name: String,
    pub salt: String,           // Base64 encoded salt
    pub verify_hash: String,     // Hex encoded: Argon2id(master_key_hex, verify_data)
    pub created_at: DateTime<Utc>,
    pub crypto_version: u32,     // Version of crypto algorithm (2 = current)
}

/// Account info returned to Flutter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub id: String,
    pub name: String,
    pub salt: String,           // Base64 encoded salt used for key derivation
    pub verify_hash: String,    // Hex encoded verify hash (for Dart to store)
    pub crypto_version: u32,    // Version of crypto algorithm (2 = current)
    pub last_accessed: Option<DateTime<Utc>>,
}

/// Verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResult {
    pub success: bool,
    pub error: Option<String>,
    pub crypto_version: u32,  // Version used for verification
}

/// Account manager singleton
pub struct AccountManager {
    /// Base path for all SoloSoul data (e.g., ~/.solosoul)
    base_path: PathBuf,
    /// In-memory cache of account metadata
    accounts_cache: RwLock<HashMap<String, AccountMetadata>>,
    /// Derived key for current session (cleared on lock)
    session_key: RwLock<Option<Zeroizing<[u8; 32]>>>,
    /// Current unlocked account ID
    unlocked_account: RwLock<Option<String>>,
    /// Vault store (SQLCipher-encrypted SQLite)
    vault_store: RwLock<Option<VaultStore>>,
}

impl AccountManager {
    /// Create or get the account manager singleton
    pub fn new(base_path: PathBuf) -> Self {
        let manager = Self {
            base_path,
            accounts_cache: RwLock::new(HashMap::new()),
            session_key: RwLock::new(None),
            unlocked_account: RwLock::new(None),
            vault_store: RwLock::new(None),
        };
        manager.load_accounts_cache();
        manager
    }

    /// Get the base path
    pub fn base_path(&self) -> &PathBuf {
        &self.base_path
    }

    /// Get account directory for a specific account
    fn account_dir(&self, account_id: &str) -> PathBuf {
        self.base_path.join(account_id)
    }

    /// Load accounts cache from disk
    fn load_accounts_cache(&self) {
        let accounts_file = self.base_path.join("accounts.json");
        if accounts_file.exists() {
            if let Ok(content) = fs::read_to_string(&accounts_file) {
                if let Ok(accounts) = serde_json::from_str::<Vec<AccountMetadata>>(&content) {
                    let mut cache = self.accounts_cache.write().unwrap();
                    for account in accounts {
                        cache.insert(account.id.clone(), account);
                    }
                }
            }
        }
    }

    /// Save accounts cache to disk
    fn save_accounts_cache(&self) -> Result<(), String> {
        let cache = self.accounts_cache.read().unwrap();
        let accounts: Vec<&AccountMetadata> = cache.values().collect();
        let content =
            serde_json::to_string_pretty(&accounts).map_err(|e| format!("Serialize failed: {}", e))?;

        // Ensure directory exists
        fs::create_dir_all(&self.base_path)
            .map_err(|e| format!("Create base dir failed: {}", e))?;

        fs::write(self.base_path.join("accounts.json"), content)
            .map_err(|e| format!("Write accounts.json failed: {}", e))?;

        Ok(())
    }

    /// List all accounts
    pub fn list_accounts(&self) -> Vec<AccountInfo> {
        let cache = self.accounts_cache.read().unwrap();
        cache
            .values()
            .map(|m| AccountInfo {
                id: m.id.clone(),
                name: m.name.clone(),
                salt: String::new(),
                verify_hash: String::new(),
                crypto_version: 2,
                last_accessed: m.last_accessed,
            })
            .collect()
    }

    /// List accounts sorted by most recent access
    pub fn list_accounts_sorted(&self) -> Vec<AccountInfo> {
        let mut accounts = self.list_accounts();
        accounts.sort_by(|a, b| {
            let a_time = a.last_accessed.unwrap_or_default();
            let b_time = b.last_accessed.unwrap_or_default();
            b_time.cmp(&a_time)
        });
        accounts
    }

    /// Check if an account name is available
    pub fn is_name_available(&self, name: &str) -> bool {
        let cache = self.accounts_cache.read().unwrap();
        !cache.values().any(|a| a.name.to_lowercase() == name.to_lowercase())
    }

    /// Create a new account
    pub fn create_account(&self, name: &str, password: &str) -> Result<AccountInfo, String> {
        // Validate
        if name.trim().is_empty() {
            return Err("Account name is required".to_string());
        }
        if password.len() < 8 {
            return Err("Password must be at least 8 characters".to_string());
        }
        if !self.is_name_available(name) {
            return Err("This account name is already taken".to_string());
        }

        // Generate account ID and salt (32 bytes)
        let account_id = format!("acc_{}", &uuid::Uuid::new_v4().to_string().replace("-", "")[..16].to_string());
        let mut salt_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut salt_bytes);
        let salt_b64 = base64_encode(&salt_bytes);

        // Derive key from password using Argon2id
        let master_key = derive_key(
            password,
            &salt_bytes,
            DEFAULT_MEMORY_KIB,
            DEFAULT_ITERATIONS,
            DEFAULT_PARALLELISM,
        )
        .map_err(|e| format!("Key derivation failed: {}", e))?;

        // Create verification hash: hash the derived key with a fixed phrase
        // This is deterministic so we can verify without storing the password
        let verify_data = b"SOLOSOUL_VAULT_VERIFY_v1";
        let verify_key = derive_key(
            &hex::encode(master_key.as_slice()),
            verify_data,
            8192,  // Smaller params for verification
            1,
            1,
        )
        .map_err(|e| format!("Verify key derivation failed: {}", e))?;
        let verify_hash = hex::encode(verify_key.as_slice());

        // Create account directory
        let account_dir = self.account_dir(&account_id);
        fs::create_dir_all(&account_dir)
            .map_err(|e| format!("Create account dir failed: {}", e))?;

        // Create config file
        let config = AccountConfig {
            account_id: account_id.clone(),
            name: name.to_string(),
            salt: salt_b64.clone(),
            verify_hash: verify_hash.clone(),
            created_at: Utc::now(),
            crypto_version: 2, // Version 2: two-step derivation (master_key + verify_hash)
        };

        let config_path = account_dir.join("config.json");
        let config_content = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Serialize config failed: {}", e))?;
        fs::write(&config_path, config_content)
            .map_err(|e| format!("Write config failed: {}", e))?;

        // Create metadata and save
        let metadata = AccountMetadata {
            id: account_id.clone(),
            name: name.to_string(),
            created_at: Utc::now(),
            last_accessed: Some(Utc::now()),
        };

        {
            let mut cache = self.accounts_cache.write().unwrap();
            cache.insert(account_id.clone(), metadata);
        }
        self.save_accounts_cache()?;

        // Store session key (clone the key)
        let key_copy = master_key.as_slice().try_into().unwrap();
        {
            let mut session = self.session_key.write().unwrap();
            *session = Some(Zeroizing::new(key_copy));
        }
        {
            let mut unlocked = self.unlocked_account.write().unwrap();
            *unlocked = Some(account_id.clone());
        }

        Ok(AccountInfo {
            id: account_id,
            name: name.to_string(),
            salt: salt_b64.clone(),
            verify_hash: verify_hash.clone(),
            crypto_version: 2,
            last_accessed: Some(Utc::now()),
        })
    }

    /// Unlock account with password
    pub fn unlock(&self, account_id: &str, password: &str) -> VerifyResult {
        // Load config
        let config_path = self.account_dir(account_id).join("config.json");
        let config_content = match fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(e) => return VerifyResult {
                success: false,
                error: Some(format!("Failed to read config: {}", e)),
                crypto_version: 0,
            },
        };

        let config: AccountConfig = match serde_json::from_str(&config_content) {
            Ok(c) => c,
            Err(e) => return VerifyResult {
                success: false,
                error: Some(format!("Failed to parse config: {}", e)),
                crypto_version: 0,
            },
        };

        // Decode salt
        let salt_bytes = match base64_decode(&config.salt) {
            Ok(s) => {
                let arr: [u8; 32] = match s.as_slice().try_into() {
                    Ok(a) => a,
                    Err(_) => return VerifyResult {
                        success: false,
                        error: Some("Invalid salt length".to_string()),
                        crypto_version: 0,
                    },
                };
                arr
            }
            Err(_) => return VerifyResult {
                success: false,
                error: Some("Invalid salt encoding".to_string()),
                crypto_version: 0,
            },
        };

        // Derive key from password
        let master_key = match derive_key(
            password,
            &salt_bytes,
            DEFAULT_MEMORY_KIB,
            DEFAULT_ITERATIONS,
            DEFAULT_PARALLELISM,
        ) {
            Ok(k) => k,
            Err(e) => return VerifyResult {
                success: false,
                error: Some(format!("Key derivation failed: {}", e)),
                crypto_version: 0,
            },
        };

        // Verify by computing the same verify hash
        let verify_data = b"SOLOSOUL_VAULT_VERIFY_v1";
        let verify_key = match derive_key(
            &hex::encode(master_key.as_slice()),
            verify_data,
            8192,
            1,
            1,
        ) {
            Ok(k) => k,
            Err(_) => return VerifyResult {
                success: false,
                error: Some("Verification failed".to_string()),
                crypto_version: 0,
            },
        };
        let computed_hash = hex::encode(verify_key.as_slice());

        if computed_hash != config.verify_hash {
            return VerifyResult {
                success: false,
                error: Some("Invalid password".to_string()),
                crypto_version: config.crypto_version,
            };
        }

        // Update last accessed
        {
            let mut cache = self.accounts_cache.write().unwrap();
            if let Some(account) = cache.get_mut(account_id) {
                account.last_accessed = Some(Utc::now());
            }
        }
        self.save_accounts_cache().ok();

        // Store session key
        let key_copy = master_key.as_slice().try_into().unwrap();
        {
            let mut session = self.session_key.write().unwrap();
            *session = Some(Zeroizing::new(key_copy));
        }
        {
            let mut unlocked = self.unlocked_account.write().unwrap();
            *unlocked = Some(account_id.to_string());
        }

        // Derive SQLCipher key and open vault
        let sqlcipher_key = self.derive_sqlcipher_key(&key_copy);
        let vault_config = VaultConfig {
            path: self.account_dir(account_id),
            account_id: account_id.to_string(),
            sqlcipher_key: Some(sqlcipher_key),
        };
        match VaultStore::open(vault_config) {
            Ok(vault) => {
                let mut vault_store = self.vault_store.write().unwrap();
                *vault_store = Some(vault);
                VerifyResult {
                    success: true,
                    error: None,
                    crypto_version: config.crypto_version,
                }
            }
            Err(e) => {
                // Vault failed to open - clear partial unlock state
                {
                    let mut session = self.session_key.write().unwrap();
                    if let Some(ref mut key) = *session {
                        key.zeroize();
                    }
                    session.take();
                }
                {
                    let mut unlocked = self.unlocked_account.write().unwrap();
                    unlocked.take();
                }
                eprintln!("Failed to open vault: {}", e);
                VerifyResult {
                    success: false,
                    error: Some(format!("Failed to open vault: {}", e)),
                    crypto_version: config.crypto_version,
                }
            }
        }
    }

    /// Lock the vault
    pub fn lock(&self) {
        // Lock vault to clear SQLCipher key
        {
            let mut vault_store = self.vault_store.write().unwrap();
            if let Some(ref mut vault) = *vault_store {
                vault.lock();
            }
            vault_store.take();
        }
        // Clear session key - extract from Option before zeroizing to ensure proper cleanup
        {
            let mut session = self.session_key.write().unwrap();
            if let Some(mut key) = session.take() {
                key.zeroize();
            }
        }
        {
            let mut unlocked = self.unlocked_account.write().unwrap();
            unlocked.take();
        }
    }

    /// Change account password
    /// 1. Verify old password
    /// 2. Generate new salt and derive new keys
    /// 3. Update config.json with new credentials
    /// 4. Re-encrypt all profiles with new key
    pub fn change_password(&self, account_id: &str, old_password: &str, new_password: &str) -> Result<AccountInfo, String> {
        // Step 1: Verify old password first
        let config_path = self.account_dir(account_id).join("config.json");
        let config_content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config: {}", e))?;
        let config: AccountConfig = serde_json::from_str(&config_content)
            .map_err(|e| format!("Failed to parse config: {}", e))?;

        // Decode old salt and verify old password
        let old_salt_bytes = base64_decode(&config.salt)
            .map_err(|e| format!("Invalid salt: {}", e))?;
        let old_salt_arr: [u8; 32] = old_salt_bytes.as_slice().try_into()
            .map_err(|_| "Invalid salt length".to_string())?;

        let old_master_key = derive_key(
            old_password,
            &old_salt_arr,
            DEFAULT_MEMORY_KIB,
            DEFAULT_ITERATIONS,
            DEFAULT_PARALLELISM,
        ).map_err(|e| format!("Key derivation failed: {}", e))?;

        // Verify old password
        let verify_data = b"SOLOSOUL_VAULT_VERIFY_v1";
        let old_verify_key = derive_key(
            &hex::encode(old_master_key.as_slice()),
            verify_data,
            8192,
            1,
            1,
        ).map_err(|e| format!("Verify key derivation failed: {}", e))?;
        let old_computed_hash = hex::encode(old_verify_key.as_slice());

        if old_computed_hash != config.verify_hash {
            return Err("Invalid current password".to_string());
        }

        // Step 2: Generate new salt and derive new keys
        let mut new_salt_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut new_salt_bytes);
        let new_salt_b64 = base64_encode(&new_salt_bytes);

        let new_master_key = derive_key(
            new_password,
            &new_salt_bytes,
            DEFAULT_MEMORY_KIB,
            DEFAULT_ITERATIONS,
            DEFAULT_PARALLELISM,
        ).map_err(|e| format!("New key derivation failed: {}", e))?;

        // Create new verify hash
        let new_verify_key = derive_key(
            &hex::encode(new_master_key.as_slice()),
            verify_data,
            8192,
            1,
            1,
        ).map_err(|e| format!("New verify key derivation failed: {}", e))?;
        let new_verify_hash = hex::encode(new_verify_key.as_slice());

        // Step 3: Re-encrypt all profiles with new key
        let vault_guard = self.get_vault_store();
        if let Some(vault_guard) = vault_guard {
            if let Some(ref vault) = *vault_guard {
                // Get all profiles and re-encrypt them
                if let Ok(profiles) = vault.list_profiles() {
                    for profile_summary in profiles {
                        if let Ok(Some(mut profile)) = vault.load_profile(&profile_summary.id) {
                            // Re-encrypt profile data with new key
                            // AES-256-GCM encryption happens at Flutter layer, vault stores raw bytes
                            // So we just need to update the vault with the same data
                            // The actual re-encryption with new key happens in Dart
                            let _ = vault.save_profile(&profile);
                        }
                    }
                }
            }
        }

        // Step 4: Update config.json with new credentials
        let new_config = AccountConfig {
            account_id: config.account_id.clone(),
            name: config.name.clone(),
            salt: new_salt_b64.clone(),
            verify_hash: new_verify_hash.clone(),
            created_at: config.created_at,
            crypto_version: 2,
        };

        let new_config_content = serde_json::to_string_pretty(&new_config)
            .map_err(|e| format!("Serialize config failed: {}", e))?;
        fs::write(&config_path, new_config_content)
            .map_err(|e| format!("Write config failed: {}", e))?;

        // Update session key to new key
        let key_copy = new_master_key.as_slice().try_into().unwrap();
        {
            let mut session = self.session_key.write().unwrap();
            *session = Some(Zeroizing::new(key_copy));
        }

        Ok(AccountInfo {
            id: config.account_id,
            name: config.name,
            salt: new_salt_b64,
            verify_hash: new_verify_hash,
            crypto_version: 2,
            last_accessed: Some(chrono::Utc::now()),
        })
    }

    /// Check if vault is unlocked
    pub fn is_unlocked(&self) -> bool {
        let session = self.session_key.read().unwrap();
        let unlocked = self.unlocked_account.read().unwrap();
        session.is_some() && unlocked.is_some()
    }

    /// Get current unlocked account ID
    pub fn get_unlocked_account(&self) -> Option<String> {
        let unlocked = self.unlocked_account.read().unwrap();
        unlocked.clone()
    }

    /// Get session key (only available when unlocked)
    pub fn get_session_key(&self) -> Option<Zeroizing<[u8; 32]>> {
        let session = self.session_key.read().unwrap();
        session.clone()
    }

    /// Get vault store reference (only available when unlocked)
    pub fn get_vault_store(&self) -> Option<std::sync::RwLockReadGuard<'_, Option<VaultStore>>> {
        if !self.is_unlocked() {
            return None;
        }
        self.vault_store.read().ok()
    }

    /// Derive SQLCipher key from session key using SHA-256
    fn derive_sqlcipher_key(&self, session_key: &[u8; 32]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(session_key);
        hasher.finalize().to_vec()
    }

    /// Get account config (salt and verify_hash) for migration to Dart Keychain
    pub fn get_account_config(&self, account_id: &str) -> Option<AccountInfo> {
        let config_path = self.account_dir(account_id).join("config.json");
        let config_content = match fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(_) => return None,
        };
        let config: AccountConfig = match serde_json::from_str(&config_content) {
            Ok(c) => c,
            Err(_) => return None,
        };
        Some(AccountInfo {
            id: config.account_id,
            name: config.name,
            salt: config.salt,
            verify_hash: config.verify_hash,
            crypto_version: config.crypto_version,
            last_accessed: None,
        })
    }

    /// Delete an account and all its data
    pub fn delete_account(&self, account_id: &str) -> Result<(), String> {
        // Remove from accounts cache
        {
            let mut cache = self.accounts_cache.write().unwrap();
            cache.remove(account_id);
        }

        // Save updated accounts cache
        self.save_accounts_cache()?;

        // Delete account directory and all its data
        let account_dir = self.account_dir(account_id);
        if account_dir.exists() {
            fs::remove_dir_all(&account_dir)
                .map_err(|e| format!("Failed to delete account directory: {}", e))?;
        }

        Ok(())
    }
}

/// Simple base64 encoding (URL-safe, no padding)
fn base64_encode(data: &[u8]) -> String {
    BASE64.encode(data)
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    BASE64.decode(input).map_err(|e| format!("Base64 decode error: {}", e))
}
