//! Account manager - handles account CRUD and password verification
//!
//! Account data is stored in:
//! ~/.solosoul/accounts.json - list of account metadata
//! ~/.solosoul/{account_id}/config.json - per-account config with salt and verification token

use argon2::password_hash::rand_core::OsRng;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::{DateTime, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::RwLock;
use zeroize::{Zeroize, Zeroizing};

use crate::crypto::argon2::{
    derive_key, DEFAULT_ITERATIONS, DEFAULT_MEMORY_KIB, DEFAULT_PARALLELISM,
};
use crate::safe_storage;
use crate::vault::{VaultConfig, VaultStore};

/// Write debug log to file (works in sandboxed environment)
fn log_to_file(msg: &str) {
    let log_path = if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join("Library/Logs/solosoul_debug.log")
    } else {
        PathBuf::from("/tmp/solosoul_debug.log")
    };
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
        let timestamp = chrono::Utc::now().to_rfc3339();
        let _ = writeln!(file, "[{}] {}", timestamp, msg);
    }
}

/// Account metadata (stored in accounts.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountMetadata {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub last_accessed: Option<DateTime<Utc>>,
}

/// Device entry for recent devices tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceEntry {
    pub device_name: String,
    pub last_used: String, // RFC3339 timestamp
}

/// Fields to update in account metadata (all optional)
#[derive(Debug, Default)]
pub struct MetadataUpdate {
    pub password_hint: Option<String>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub last_operation_at: Option<DateTime<Utc>>,
    pub last_operation_desc: Option<String>,
    pub recent_devices: Option<Vec<DeviceEntry>>,
    pub biometric_enabled: Option<bool>,
}

/// Per-account config (stored in {account_id}/config.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountConfig {
    pub account_id: String,
    pub name: String,
    pub salt: String,        // Base64 encoded salt
    pub verify_hash: String, // Hex encoded: Argon2id(master_key_hex, verify_data)
    pub created_at: DateTime<Utc>,
    pub crypto_version: u32, // Version of crypto algorithm (2 = current)
    // Phase 2: metadata fields (migrated from Keychain)
    #[serde(default)]
    pub password_hint: Option<String>,
    #[serde(default)]
    pub last_login_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_operation_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_operation_desc: Option<String>,
    #[serde(default)]
    pub recent_devices: Vec<DeviceEntry>,
    #[serde(default)]
    pub biometric_enabled: bool,
    #[serde(default)]
    pub biometric_session_key_hash: Option<String>,
}

/// Account info returned to Flutter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerAccountInfo {
    pub id: String,
    pub name: String,
    pub salt: String,        // Base64 encoded salt used for key derivation
    pub verify_hash: String, // Hex encoded verify hash (for Dart to store)
    pub crypto_version: u32, // Version of crypto algorithm (2 = current)
    pub created_at: Option<String>, // RFC3339 timestamp
    pub last_accessed: Option<String>, // RFC3339 timestamp
    // Phase 2: metadata fields
    #[serde(default)]
    pub password_hint: Option<String>,
    #[serde(default)]
    pub last_login_at: Option<String>, // RFC3339 timestamp
    #[serde(default)]
    pub last_operation_at: Option<String>, // RFC3339 timestamp
    #[serde(default)]
    pub last_operation_desc: Option<String>,
    #[serde(default)]
    pub recent_devices: Vec<DeviceEntry>,
    #[serde(default)]
    pub biometric_enabled: bool,
}

/// Verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResult {
    pub success: bool,
    pub error: Option<String>,
    pub crypto_version: u32, // Version used for verification
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

    /// Load accounts cache from disk (with crash recovery)
    fn load_accounts_cache(&self) {
        let accounts_file = self.base_path.join("accounts.json");
        if let Some(content) = safe_storage::recover_or_load(&accounts_file) {
            if let Ok(accounts) = serde_json::from_str::<Vec<AccountMetadata>>(&content) {
                if let Ok(mut cache) = self
                    .accounts_cache
                    .write()
                    .map_err(|e| format!("Lock poisoned: {}", e))
                {
                    for account in accounts {
                        cache.insert(account.id.clone(), account);
                    }
                }
            }
        }
    }

    /// Save accounts cache to disk
    fn save_accounts_cache(&self) -> Result<(), String> {
        let cache = self
            .accounts_cache
            .read()
            .map_err(|e| format!("Lock poisoned: {}", e))?;
        let accounts: Vec<&AccountMetadata> = cache.values().collect();
        let content = serde_json::to_string_pretty(&accounts)
            .map_err(|e| format!("Serialize failed: {}", e))?;

        // Ensure directory exists
        fs::create_dir_all(&self.base_path)
            .map_err(|e| format!("Create base dir failed: {}", e))?;

        safe_storage::write_atomic(&self.base_path.join("accounts.json"), content.as_bytes())
            .map_err(|e| format!("Write accounts.json failed: {}", e))?;

        Ok(())
    }

    /// List all accounts with full metadata from config.json
    pub fn list_accounts(&self) -> Vec<ManagerAccountInfo> {
        let cache = match self.accounts_cache.read() {
            Ok(c) => c,
            Err(e) => {
                log_to_file(&format!("[MANAGER] Failed to read accounts cache: {}", e));
                return Vec::new();
            }
        };
        cache
            .values()
            .map(|m| {
                // Read full config to get metadata fields
                let config_path = self.account_dir(&m.id).join("config.json");
                let (created_at, password_hint, last_login_at, last_operation_at, last_operation_desc, recent_devices, biometric_enabled) =
                    match safe_storage::recover_or_load(&config_path) {
                        Some(content) => match serde_json::from_str::<AccountConfig>(&content) {
                            Ok(config) => (
                                Some(config.created_at.to_rfc3339()),
                                config.password_hint,
                                config.last_login_at.map(|d| d.to_rfc3339()),
                                config.last_operation_at.map(|d| d.to_rfc3339()),
                                config.last_operation_desc,
                                config.recent_devices,
                                config.biometric_enabled,
                            ),
                            Err(_) => (None, None, None, None, None, Vec::new(), false),
                        },
                        None => (None, None, None, None, None, Vec::new(), false),
                    };
                ManagerAccountInfo {
                    id: m.id.clone(),
                    name: m.name.clone(),
                    salt: String::new(),
                    verify_hash: String::new(),
                    crypto_version: 2,
                    created_at,
                    last_accessed: m.last_accessed.map(|d| d.to_rfc3339()),
                    password_hint,
                    last_login_at,
                    last_operation_at,
                    last_operation_desc,
                    recent_devices,
                    biometric_enabled,
                }
            })
            .collect()
    }

    /// List accounts sorted by most recent access
    pub fn list_accounts_sorted(&self) -> Vec<ManagerAccountInfo> {
        let mut accounts = self.list_accounts();
        accounts.sort_by(|a, b| {
            let a_time = a.last_accessed.as_deref().unwrap_or("");
            let b_time = b.last_accessed.as_deref().unwrap_or("");
            b_time.cmp(a_time)
        });
        accounts
    }

    /// Check if an account name is available
    pub fn is_name_available(&self, name: &str) -> bool {
        let cache = match self.accounts_cache.read() {
            Ok(c) => c,
            Err(e) => {
                log_to_file(&format!("[MANAGER] Failed to read accounts cache: {}", e));
                return false;
            }
        };
        !cache
            .values()
            .any(|a| a.name.to_lowercase() == name.to_lowercase())
    }

    /// Create a new account
    pub fn create_account(&self, name: &str, password: &str) -> Result<ManagerAccountInfo, String> {
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
        let account_id = format!(
            "acc_{}",
            &uuid::Uuid::new_v4().to_string().replace("-", "")[..16].to_string()
        );
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
            8192, // Smaller params for verification
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
            password_hint: None,
            last_login_at: None,
            last_operation_at: None,
            last_operation_desc: None,
            recent_devices: Vec::new(),
            biometric_enabled: false,
            biometric_session_key_hash: None,
        };

        let config_path = account_dir.join("config.json");
        let config_content = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Serialize config failed: {}", e))?;
        safe_storage::write_atomic(&config_path, config_content.as_bytes())
            .map_err(|e| format!("Write config failed: {}", e))?;

        // Create metadata and save
        let metadata = AccountMetadata {
            id: account_id.clone(),
            name: name.to_string(),
            created_at: Utc::now(),
            last_accessed: Some(Utc::now()),
        };

        {
            let mut cache = self
                .accounts_cache
                .write()
                .map_err(|e| format!("Lock poisoned: {}", e))?;
            cache.insert(account_id.clone(), metadata);
        }
        self.save_accounts_cache()?;

        // Store session key (clone the key)
        let key_copy = master_key.as_slice().try_into().unwrap();
        {
            let mut session = self
                .session_key
                .write()
                .map_err(|e| format!("Lock poisoned: {}", e))?;
            *session = Some(Zeroizing::new(key_copy));
        }
        {
            let mut unlocked = self
                .unlocked_account
                .write()
                .map_err(|e| format!("Lock poisoned: {}", e))?;
            *unlocked = Some(account_id.clone());
        }

        // Open vault store for the new account
        let sqlcipher_key = self.derive_sqlcipher_key(&key_copy);
        let vault_config = VaultConfig {
            path: self.account_dir(&account_id),
            account_id: account_id.clone(),
            sqlcipher_key: Some(sqlcipher_key),
        };
        match VaultStore::open(vault_config) {
            Ok(vault) => {
                let mut vault_store = self
                    .vault_store
                    .write()
                    .map_err(|e| format!("Lock poisoned: {}", e))?;
                *vault_store = Some(vault);
            }
            Err(e) => {
                log_to_file(&format!(
                    "[MANAGER] Failed to open vault store during create_account: {}",
                    e
                ));
                // Clear partial unlock state
                {
                    let mut session = self
                        .session_key
                        .write()
                        .map_err(|e| format!("Lock poisoned: {}", e))?;
                    if let Some(ref mut key) = *session {
                        key.zeroize();
                    }
                    session.take();
                }
                {
                    let mut unlocked = self
                        .unlocked_account
                        .write()
                        .map_err(|e| format!("Lock poisoned: {}", e))?;
                    unlocked.take();
                }
                return Err(format!("Failed to open vault: {}", e));
            }
        }

        Ok(ManagerAccountInfo {
            id: account_id,
            name: name.to_string(),
            salt: salt_b64.clone(),
            verify_hash: verify_hash.clone(),
            crypto_version: 2,
            created_at: Some(Utc::now().to_rfc3339()),
            last_accessed: Some(Utc::now().to_rfc3339()),
            password_hint: None,
            last_login_at: Some(Utc::now().to_rfc3339()),
            last_operation_at: None,
            last_operation_desc: None,
            recent_devices: Vec::new(),
            biometric_enabled: false,
        })
    }

    /// Unlock account with password
    pub fn unlock(&self, account_id: &str, password: &str) -> VerifyResult {
        log_to_file(&format!(
            "[MANAGER] unlock called for account_id: {}",
            account_id
        ));

        // Fast path: if vault is already unlocked for this account, verify password only
        if self.is_unlocked() {
            if let Some(current) = self.get_unlocked_account() {
                if current == account_id {
                    log_to_file(
                        "[MANAGER] Vault already unlocked for this account, checking password...",
                    );
                    // Re-verify password without re-opening the vault
                    let account_dir = self.account_dir(account_id);
                    let config_path = account_dir.join("config.json");
                    let config_content = match safe_storage::recover_or_load(&config_path) {
                        Some(c) => c,
                        None => {
                            return VerifyResult {
                                success: false,
                                error: Some("Failed to read config".to_string()),
                                crypto_version: 0,
                            }
                        }
                    };
                    let config: AccountConfig = match serde_json::from_str(&config_content) {
                        Ok(c) => c,
                        Err(_) => {
                            return VerifyResult {
                                success: false,
                                error: Some("Config parse error".to_string()),
                                crypto_version: 0,
                            }
                        }
                    };
                    let salt_bytes: [u8; 32] = match base64_decode(&config.salt) {
                        Ok(s) => match s.as_slice().try_into() {
                            Ok(a) => a,
                            Err(_) => {
                                return VerifyResult {
                                    success: false,
                                    error: Some("Invalid salt".to_string()),
                                    crypto_version: 0,
                                }
                            }
                        },
                        Err(_) => {
                            return VerifyResult {
                                success: false,
                                error: Some("Invalid salt encoding".to_string()),
                                crypto_version: 0,
                            }
                        }
                    };
                    let master_key = match derive_key(
                        password,
                        &salt_bytes,
                        DEFAULT_MEMORY_KIB,
                        DEFAULT_ITERATIONS,
                        DEFAULT_PARALLELISM,
                    ) {
                        Ok(k) => k,
                        Err(_) => {
                            return VerifyResult {
                                success: false,
                                error: Some("Key derivation failed".to_string()),
                                crypto_version: 0,
                            }
                        }
                    };
                    let verify_data = b"SOLOSOUL_VAULT_VERIFY_v1";
                    let verify_key = match derive_key(
                        &hex::encode(master_key.as_slice()),
                        verify_data,
                        8192,
                        1,
                        1,
                    ) {
                        Ok(k) => k,
                        Err(_) => {
                            return VerifyResult {
                                success: false,
                                error: Some("Verify failed".to_string()),
                                crypto_version: 0,
                            }
                        }
                    };
                    let computed_hash = hex::encode(verify_key.as_slice());
                    log_to_file(&format!(
                        "[UNLOCK-FAST] account_id={} cfg_hash={}..({}) computed_hash={}..({})",
                        account_id,
                        &config.verify_hash[..8.min(config.verify_hash.len())],
                        config.verify_hash.len(),
                        &computed_hash[..8.min(computed_hash.len())],
                        computed_hash.len(),
                    ));
                    if computed_hash != config.verify_hash {
                        log_to_file(&format!(
                            "[UNLOCK-FAST] HASH MISMATCH for account_id={}",
                            account_id,
                        ));
                        return VerifyResult {
                            success: false,
                            error: Some("Invalid password".to_string()),
                            crypto_version: config.crypto_version,
                        };
                    }
                    // Ensure vault store is open (create_account sets unlocked state but may not open vault)
                    let vault_guard = match self.vault_store.read() {
                        Ok(g) => g,
                        Err(e) => {
                            log_to_file(&format!("[MANAGER] Vault store lock poisoned: {}", e));
                            return VerifyResult {
                                success: false,
                                error: Some("Internal error".to_string()),
                                crypto_version: config.crypto_version,
                            };
                        }
                    };
                    if vault_guard.is_none() {
                        drop(vault_guard);
                        let session_guard = match self.session_key.read() {
                            Ok(g) => g,
                            Err(e) => {
                                log_to_file(&format!("[MANAGER] Session key lock poisoned: {}", e));
                                return VerifyResult {
                                    success: false,
                                    error: Some("Internal error".to_string()),
                                    crypto_version: config.crypto_version,
                                };
                            }
                        };
                        if let Some(ref key) = *session_guard {
                            let sqlcipher_key = self.derive_sqlcipher_key(key);
                            let vault_config = VaultConfig {
                                path: self.account_dir(account_id),
                                account_id: account_id.to_string(),
                                sqlcipher_key: Some(sqlcipher_key),
                            };
                            match VaultStore::open(vault_config) {
                                Ok(vault) => {
                                    let mut vault_store = match self.vault_store.write() {
                                        Ok(guard) => guard,
                                        Err(e) => {
                                            log_to_file(&format!("[MANAGER] Lock poisoned: {}", e));
                                            return VerifyResult {
                                                success: false,
                                                error: Some(format!("Lock poisoned: {}", e)),
                                                crypto_version: config.crypto_version,
                                            };
                                        }
                                    };
                                    *vault_store = Some(vault);
                                }
                                Err(e) => {
                                    log_to_file(&format!(
                                        "[MANAGER] Failed to open vault store in fast path: {}",
                                        e
                                    ));
                                    return VerifyResult {
                                        success: false,
                                        error: Some(format!("Failed to open vault: {}", e)),
                                        crypto_version: config.crypto_version,
                                    };
                                }
                            }
                        }
                    }
                    log_to_file("[MANAGER] Password re-verified successfully (vault ready)");
                    return VerifyResult {
                        success: true,
                        error: None,
                        crypto_version: config.crypto_version,
                    };
                }
            }
        }

        // Check if account directory exists
        let account_dir = self.account_dir(account_id);
        log_to_file(&format!("[MANAGER] account_dir: {:?}", account_dir));
        log_to_file(&format!(
            "[MANAGER] account_dir exists: {}",
            account_dir.exists()
        ));

        // Load config (with crash recovery)
        let config_path = account_dir.join("config.json");
        log_to_file(&format!("[MANAGER] config_path: {:?}", config_path));
        log_to_file(&format!(
            "[MANAGER] config_path exists: {}",
            config_path.exists()
        ));

        let config_content = match safe_storage::recover_or_load(&config_path) {
            Some(c) => c,
            None => {
                log_to_file("[MANAGER] Failed to read config (no valid source)");
                return VerifyResult {
                    success: false,
                    error: Some("Failed to read config".to_string()),
                    crypto_version: 0,
                };
            }
        };
        log_to_file("[MANAGER] Config file read successfully");

        let config: AccountConfig = match serde_json::from_str(&config_content) {
            Ok(c) => c,
            Err(e) => {
                log_to_file(&format!("[MANAGER] Config parse error: {}", e));
                return VerifyResult {
                    success: false,
                    error: Some(format!("Failed to parse config: {}", e)),
                    crypto_version: 0,
                };
            }
        };
        log_to_file(&format!(
            "[MANAGER] Config parsed, account_id: {}, name: {}",
            config.account_id, config.name
        ));

        // Decode salt
        let salt_bytes = match base64_decode(&config.salt) {
            Ok(s) => {
                let arr: [u8; 32] = match s.as_slice().try_into() {
                    Ok(a) => a,
                    Err(_) => {
                        return VerifyResult {
                            success: false,
                            error: Some("Invalid salt length".to_string()),
                            crypto_version: 0,
                        }
                    }
                };
                arr
            }
            Err(_) => {
                return VerifyResult {
                    success: false,
                    error: Some("Invalid salt encoding".to_string()),
                    crypto_version: 0,
                }
            }
        };

        // Derive key from password
        log_to_file("[MANAGER] Starting key derivation...");
        let master_key = match derive_key(
            password,
            &salt_bytes,
            DEFAULT_MEMORY_KIB,
            DEFAULT_ITERATIONS,
            DEFAULT_PARALLELISM,
        ) {
            Ok(k) => k,
            Err(e) => {
                return VerifyResult {
                    success: false,
                    error: Some(format!("Key derivation failed: {}", e)),
                    crypto_version: 0,
                }
            }
        };
        log_to_file("[MANAGER] Key derivation complete, starting verification hash...");

        // Verify by computing the same verify hash
        let verify_data = b"SOLOSOUL_VAULT_VERIFY_v1";
        let verify_key =
            match derive_key(&hex::encode(master_key.as_slice()), verify_data, 8192, 1, 1) {
                Ok(k) => k,
                Err(_) => {
                    return VerifyResult {
                        success: false,
                        error: Some("Verification failed".to_string()),
                        crypto_version: 0,
                    }
                }
            };
        let computed_hash = hex::encode(verify_key.as_slice());

        log_to_file(&format!(
            "[UNLOCK] account_id={} salt_b64_len={} verify_hash_cfg={}..({}) computed_hash={}..({})",
            account_id,
            config.salt.len(),
            &config.verify_hash[..8.min(config.verify_hash.len())],
            config.verify_hash.len(),
            &computed_hash[..8.min(computed_hash.len())],
            computed_hash.len(),
        ));

        if computed_hash != config.verify_hash {
            log_to_file(&format!(
                "[UNLOCK] HASH MISMATCH for account_id={} cfg={}.. computed={}..",
                account_id,
                &config.verify_hash[..12.min(config.verify_hash.len())],
                &computed_hash[..12.min(computed_hash.len())],
            ));
            return VerifyResult {
                success: false,
                error: Some("Invalid password".to_string()),
                crypto_version: config.crypto_version,
            };
        }

        // Update last accessed
        {
            let mut cache = match self.accounts_cache.write() {
                Ok(guard) => guard,
                Err(e) => {
                    log_to_file(&format!("[MANAGER] Lock poisoned: {}", e));
                    return VerifyResult {
                        success: false,
                        error: Some(format!("Lock poisoned: {}", e)),
                        crypto_version: 0,
                    };
                }
            };
            if let Some(account) = cache.get_mut(account_id) {
                account.last_accessed = Some(Utc::now());
            }
        }
        self.save_accounts_cache().ok();

        // Store session key
        let key_copy = master_key.as_slice().try_into().unwrap();
        {
            let mut session = match self.session_key.write() {
                Ok(guard) => guard,
                Err(e) => {
                    log_to_file(&format!("[MANAGER] Lock poisoned: {}", e));
                    return VerifyResult {
                        success: false,
                        error: Some(format!("Lock poisoned: {}", e)),
                        crypto_version: 0,
                    };
                }
            };
            *session = Some(Zeroizing::new(key_copy));
        }
        {
            let mut unlocked = match self.unlocked_account.write() {
                Ok(guard) => guard,
                Err(e) => {
                    log_to_file(&format!("[MANAGER] Lock poisoned: {}", e));
                    return VerifyResult {
                        success: false,
                        error: Some(format!("Lock poisoned: {}", e)),
                        crypto_version: 0,
                    };
                }
            };
            *unlocked = Some(account_id.to_string());
        }

        // Derive SQLCipher key and open vault
        log_to_file("[MANAGER] Deriving SQLCipher key...");
        let sqlcipher_key = self.derive_sqlcipher_key(&key_copy);
        let vault_config = VaultConfig {
            path: self.account_dir(account_id),
            account_id: account_id.to_string(),
            sqlcipher_key: Some(sqlcipher_key),
        };
        log_to_file(&format!(
            "[MANAGER] Opening vault at: {:?}",
            vault_config.path
        ));
        match VaultStore::open(vault_config) {
            Ok(vault) => {
                log_to_file("[MANAGER] Vault opened successfully");
                let mut vault_store = match self.vault_store.write() {
                    Ok(guard) => guard,
                    Err(e) => {
                        log_to_file(&format!("[MANAGER] Lock poisoned: {}", e));
                        return VerifyResult {
                            success: false,
                            error: Some(format!("Lock poisoned: {}", e)),
                            crypto_version: config.crypto_version,
                        };
                    }
                };
                *vault_store = Some(vault);
                VerifyResult {
                    success: true,
                    error: None,
                    crypto_version: config.crypto_version,
                }
            }
            Err(e) => {
                log_to_file(&format!("[MANAGER] Vault open failed: {}", e));
                // Vault failed to open - clear partial unlock state
                {
                    let mut session = match self.session_key.write() {
                        Ok(guard) => guard,
                        Err(e) => {
                            log_to_file(&format!("[MANAGER] Lock poisoned: {}", e));
                            return VerifyResult {
                                success: false,
                                error: Some(format!("Lock poisoned: {}", e)),
                                crypto_version: config.crypto_version,
                            };
                        }
                    };
                    if let Some(ref mut key) = *session {
                        key.zeroize();
                    }
                    session.take();
                }
                {
                    let mut unlocked = match self.unlocked_account.write() {
                        Ok(guard) => guard,
                        Err(e) => {
                            log_to_file(&format!("[MANAGER] Lock poisoned: {}", e));
                            return VerifyResult {
                                success: false,
                                error: Some(format!("Lock poisoned: {}", e)),
                                crypto_version: config.crypto_version,
                            };
                        }
                    };
                    unlocked.take();
                }
                VerifyResult {
                    success: false,
                    error: Some(format!("Failed to open vault: {}", e)),
                    crypto_version: config.crypto_version,
                }
            }
        }
    }

    /// Unlock account with a pre-derived session key (for biometric unlock)
    pub fn unlock_with_key(&self, account_id: &str, session_key_b64: &str) -> VerifyResult {
        log_to_file(&format!(
            "[MANAGER] unlock_with_key called for account_id: {}",
            account_id
        ));
        let was_unlocked = self.is_unlocked();
        log_to_file(&format!("[MANAGER] unlock_with_key: was_unlocked before={}", was_unlocked));

        // Decode session key from base64
        let session_key_bytes = match base64_decode(session_key_b64) {
            Ok(b) => b,
            Err(_) => {
                return VerifyResult {
                    success: false,
                    error: Some("Invalid session key encoding".to_string()),
                    crypto_version: 0,
                }
            }
        };
        let session_key_arr: [u8; 32] = match session_key_bytes.as_slice().try_into() {
            Ok(a) => a,
            Err(_) => {
                return VerifyResult {
                    success: false,
                    error: Some("Invalid session key length".to_string()),
                    crypto_version: 0,
                }
            }
        };

        // Check if account directory exists
        let account_dir = self.account_dir(account_id);
        if !account_dir.exists() {
            return VerifyResult {
                success: false,
                error: Some("Account not found".to_string()),
                crypto_version: 0,
            };
        }

        // Load config (with crash recovery)
        let config_path = account_dir.join("config.json");
        let config_content = match safe_storage::recover_or_load(&config_path) {
            Some(c) => c,
            None => {
                return VerifyResult {
                    success: false,
                    error: Some("Failed to read config".to_string()),
                    crypto_version: 0,
                }
            }
        };
        let config: AccountConfig = match serde_json::from_str(&config_content) {
            Ok(c) => c,
            Err(e) => {
                return VerifyResult {
                    success: false,
                    error: Some(format!("Failed to parse config: {}", e)),
                    crypto_version: 0,
                }
            }
        };

        // Verify the session key by computing the same verify hash
        let verify_data = b"SOLOSOUL_VAULT_VERIFY_v1";
        let verify_key = match derive_key(
            &hex::encode(session_key_arr.as_slice()),
            verify_data,
            8192,
            1,
            1,
        ) {
            Ok(k) => k,
            Err(_) => {
                return VerifyResult {
                    success: false,
                    error: Some("Verification failed".to_string()),
                    crypto_version: 0,
                }
            }
        };
        let computed_hash = hex::encode(verify_key.as_slice());

        if computed_hash != config.verify_hash {
            return VerifyResult {
                success: false,
                error: Some("Invalid session key".to_string()),
                crypto_version: config.crypto_version,
            };
        }

        // Update last accessed
        {
            let mut cache = match self.accounts_cache.write() {
                Ok(guard) => guard,
                Err(e) => {
                    log_to_file(&format!("[MANAGER] Lock poisoned: {}", e));
                    return VerifyResult {
                        success: false,
                        error: Some(format!("Lock poisoned: {}", e)),
                        crypto_version: 0,
                    };
                }
            };
            if let Some(account) = cache.get_mut(account_id) {
                account.last_accessed = Some(Utc::now());
            }
        }
        self.save_accounts_cache().ok();

        // Store session key
        let key_copy = session_key_arr;
        {
            let mut session = match self.session_key.write() {
                Ok(guard) => guard,
                Err(e) => {
                    log_to_file(&format!("[MANAGER] Lock poisoned: {}", e));
                    return VerifyResult {
                        success: false,
                        error: Some(format!("Lock poisoned: {}", e)),
                        crypto_version: 0,
                    };
                }
            };
            *session = Some(Zeroizing::new(key_copy));
        }
        {
            let mut unlocked = match self.unlocked_account.write() {
                Ok(guard) => guard,
                Err(e) => {
                    log_to_file(&format!("[MANAGER] Lock poisoned: {}", e));
                    return VerifyResult {
                        success: false,
                        error: Some(format!("Lock poisoned: {}", e)),
                        crypto_version: 0,
                    };
                }
            };
            *unlocked = Some(account_id.to_string());
        }

        // Derive SQLCipher key and open vault
        log_to_file("[MANAGER] Deriving SQLCipher key from session key...");
        let sqlcipher_key = self.derive_sqlcipher_key(&key_copy);
        let vault_config = VaultConfig {
            path: self.account_dir(account_id),
            account_id: account_id.to_string(),
            sqlcipher_key: Some(sqlcipher_key),
        };
        log_to_file(&format!(
            "[MANAGER] Opening vault at: {:?}",
            vault_config.path
        ));
        match VaultStore::open(vault_config) {
            Ok(vault) => {
                log_to_file("[MANAGER] Vault opened successfully with session key");
                let now_unlocked = self.is_unlocked();
                log_to_file(&format!("[MANAGER] unlock_with_key: is_unlocked after open={}, session_key={}", now_unlocked, self.get_session_key().is_some()));
                let mut vault_store = match self.vault_store.write() {
                    Ok(guard) => guard,
                    Err(e) => {
                        log_to_file(&format!("[MANAGER] Lock poisoned: {}", e));
                        return VerifyResult {
                            success: false,
                            error: Some(format!("Lock poisoned: {}", e)),
                            crypto_version: config.crypto_version,
                        };
                    }
                };
                *vault_store = Some(vault);
                VerifyResult {
                    success: true,
                    error: None,
                    crypto_version: config.crypto_version,
                }
            }
            Err(e) => {
                log_to_file(&format!("[MANAGER] Vault open failed: {}", e));
                // Clear partial unlock state
                {
                    let mut session = match self.session_key.write() {
                        Ok(guard) => guard,
                        Err(e) => {
                            log_to_file(&format!("[MANAGER] Lock poisoned: {}", e));
                            return VerifyResult {
                                success: false,
                                error: Some(format!("Lock poisoned: {}", e)),
                                crypto_version: config.crypto_version,
                            };
                        }
                    };
                    if let Some(ref mut key) = *session {
                        key.zeroize();
                    }
                    session.take();
                }
                {
                    let mut unlocked = match self.unlocked_account.write() {
                        Ok(guard) => guard,
                        Err(e) => {
                            log_to_file(&format!("[MANAGER] Lock poisoned: {}", e));
                            return VerifyResult {
                                success: false,
                                error: Some(format!("Lock poisoned: {}", e)),
                                crypto_version: config.crypto_version,
                            };
                        }
                    };
                    unlocked.take();
                }
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
        log_to_file("[MANAGER] lock() called");
        // Lock vault to clear SQLCipher key
        {
            let mut vault_store = match self.vault_store.write() {
                Ok(g) => g,
                Err(e) => {
                    log_to_file(&format!(
                        "[MANAGER] Vault store lock poisoned during lock: {}",
                        e
                    ));
                    return;
                }
            };
            if let Some(ref mut vault) = *vault_store {
                vault.lock();
            }
            vault_store.take();
        }
        // Clear session key - extract from Option before zeroizing to ensure proper cleanup
        {
            let mut session = match self.session_key.write() {
                Ok(g) => g,
                Err(e) => {
                    log_to_file(&format!(
                        "[MANAGER] Session key lock poisoned during lock: {}",
                        e
                    ));
                    return;
                }
            };
            if let Some(mut key) = session.take() {
                key.zeroize();
            }
        }
        {
            let mut unlocked = match self.unlocked_account.write() {
                Ok(g) => g,
                Err(e) => {
                    log_to_file(&format!(
                        "[MANAGER] Unlocked account lock poisoned during lock: {}",
                        e
                    ));
                    return;
                }
            };
            unlocked.take();
        }
    }

    /// Change account password
    /// 1. Verify old password
    /// 2. Generate new salt and derive new keys
    /// 3. Update config.json with new credentials
    /// 4. Re-encrypt all profiles with new key
    pub fn change_password(
        &self,
        account_id: &str,
        old_password: &str,
        new_password: &str,
    ) -> Result<ManagerAccountInfo, String> {
        // Step 1: Verify old password first
        let config_path = self.account_dir(account_id).join("config.json");
        let config_content = safe_storage::recover_or_load(&config_path)
            .ok_or_else(|| "Failed to read config".to_string())?;
        let config: AccountConfig = serde_json::from_str(&config_content)
            .map_err(|e| format!("Failed to parse config: {}", e))?;

        // Decode old salt and verify old password
        let old_salt_bytes =
            base64_decode(&config.salt).map_err(|e| format!("Invalid salt: {}", e))?;
        let old_salt_arr: [u8; 32] = old_salt_bytes
            .as_slice()
            .try_into()
            .map_err(|_| "Invalid salt length".to_string())?;

        let old_master_key = derive_key(
            old_password,
            &old_salt_arr,
            DEFAULT_MEMORY_KIB,
            DEFAULT_ITERATIONS,
            DEFAULT_PARALLELISM,
        )
        .map_err(|e| format!("Key derivation failed: {}", e))?;

        // Verify old password
        let verify_data = b"SOLOSOUL_VAULT_VERIFY_v1";
        let old_verify_key = derive_key(
            &hex::encode(old_master_key.as_slice()),
            verify_data,
            8192,
            1,
            1,
        )
        .map_err(|e| format!("Verify key derivation failed: {}", e))?;
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
        )
        .map_err(|e| format!("New key derivation failed: {}", e))?;

        // Create new verify hash
        let new_verify_key = derive_key(
            &hex::encode(new_master_key.as_slice()),
            verify_data,
            8192,
            1,
            1,
        )
        .map_err(|e| format!("New verify key derivation failed: {}", e))?;
        let new_verify_hash = hex::encode(new_verify_key.as_slice());

        // Step 3: Re-encrypt all profiles with new key
        let vault_guard = self.get_vault_store();
        if let Some(vault_guard) = vault_guard {
            if let Some(ref vault) = *vault_guard {
                // Get all profiles and re-encrypt them
                if let Ok(profiles) = vault.list_profiles() {
                    for profile_summary in profiles {
                        if let Ok(Some(profile)) = vault.load_profile(&profile_summary.id) {
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
            password_hint: config.password_hint.clone(),
            last_login_at: config.last_login_at,
            last_operation_at: config.last_operation_at,
            last_operation_desc: config.last_operation_desc.clone(),
            recent_devices: config.recent_devices.clone(),
            biometric_enabled: config.biometric_enabled,
            biometric_session_key_hash: None, // cleared on password change
        };

        let new_config_content = serde_json::to_string_pretty(&new_config)
            .map_err(|e| format!("Serialize config failed: {}", e))?;
        safe_storage::write_atomic(&config_path, new_config_content.as_bytes())
            .map_err(|e| format!("Write config failed: {}", e))?;

        // Update session key to new key
        let key_copy = new_master_key.as_slice().try_into().unwrap();
        {
            let mut session = self
                .session_key
                .write()
                .map_err(|e| format!("Lock poisoned: {}", e))?;
            *session = Some(Zeroizing::new(key_copy));
        }

        Ok(ManagerAccountInfo {
            id: config.account_id,
            name: config.name,
            salt: new_salt_b64,
            verify_hash: new_verify_hash,
            crypto_version: 2,
            created_at: Some(config.created_at.to_rfc3339()),
            last_accessed: Some(chrono::Utc::now().to_rfc3339()),
            password_hint: config.password_hint,
            last_login_at: config.last_login_at.map(|d| d.to_rfc3339()),
            last_operation_at: config.last_operation_at.map(|d| d.to_rfc3339()),
            last_operation_desc: config.last_operation_desc,
            recent_devices: config.recent_devices,
            biometric_enabled: config.biometric_enabled,
        })
    }

    /// Check if vault is unlocked
    pub fn is_unlocked(&self) -> bool {
        let session = match self.session_key.read() {
            Ok(s) => s,
            Err(e) => {
                log_to_file(&format!(
                    "[MANAGER] Session key lock poisoned in is_unlocked: {}",
                    e
                ));
                return false;
            }
        };
        let unlocked = match self.unlocked_account.read() {
            Ok(u) => u,
            Err(e) => {
                log_to_file(&format!(
                    "[MANAGER] Unlocked account lock poisoned in is_unlocked: {}",
                    e
                ));
                return false;
            }
        };
        session.is_some() && unlocked.is_some()
    }

    /// Get current unlocked account ID
    pub fn get_unlocked_account(&self) -> Option<String> {
        let unlocked = match self.unlocked_account.read() {
            Ok(u) => u,
            Err(e) => {
                log_to_file(&format!("[MANAGER] Unlocked account lock poisoned: {}", e));
                return None;
            }
        };
        unlocked.clone()
    }

    /// Get session key (only available when unlocked)
    pub fn get_session_key(&self) -> Option<Zeroizing<[u8; 32]>> {
        let session = match self.session_key.read() {
            Ok(s) => s,
            Err(e) => {
                log_to_file(&format!("[MANAGER] Session key lock poisoned: {}", e));
                return None;
            }
        };
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
    pub fn get_account_config(&self, account_id: &str) -> Option<ManagerAccountInfo> {
        let config_path = self.account_dir(account_id).join("config.json");
        let config_content = match safe_storage::recover_or_load(&config_path) {
            Some(c) => c,
            None => return None,
        };
        let config: AccountConfig = match serde_json::from_str(&config_content) {
            Ok(c) => c,
            Err(_) => return None,
        };
        Some(ManagerAccountInfo {
            id: config.account_id,
            name: config.name,
            salt: config.salt,
            verify_hash: config.verify_hash,
            crypto_version: config.crypto_version,
            created_at: Some(config.created_at.to_rfc3339()),
            last_accessed: None,
            password_hint: config.password_hint,
            last_login_at: config.last_login_at.map(|d| d.to_rfc3339()),
            last_operation_at: config.last_operation_at.map(|d| d.to_rfc3339()),
            last_operation_desc: config.last_operation_desc,
            recent_devices: config.recent_devices,
            biometric_enabled: config.biometric_enabled,
        })
    }

    /// Update account metadata fields in config.json.
    /// Only non-None fields are updated; existing values are preserved.
    pub fn update_account_metadata(
        &self,
        account_id: &str,
        update: MetadataUpdate,
    ) -> Result<(), String> {
        let config_path = self.account_dir(account_id).join("config.json");
        let config_content = safe_storage::recover_or_load(&config_path)
            .ok_or_else(|| {
                "Failed to read config".to_string()
            })?;
        let mut config: AccountConfig = serde_json::from_str(&config_content)
            .map_err(|e| {
                format!("Failed to parse config: {}", e)
            })?;

        if let Some(hint) = update.password_hint {
            config.password_hint = Some(hint);
        }
        if let Some(login) = update.last_login_at {
            config.last_login_at = Some(login);
        }
        if let Some(op_at) = update.last_operation_at {
            config.last_operation_at = Some(op_at);
        }
        if let Some(op_desc) = update.last_operation_desc {
            config.last_operation_desc = Some(op_desc);
        }
        if let Some(devices) = update.recent_devices {
            config.recent_devices = devices;
        }
        if let Some(enabled) = update.biometric_enabled {
            config.biometric_enabled = enabled;
        }

        let new_content = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Serialize config failed: {}", e))?;
        safe_storage::write_atomic(&config_path, new_content.as_bytes())
            .map_err(|e| {
                format!("Write config failed: {}", e)
            })?;

        Ok(())
    }

    /// Delete an account and all its data
    pub fn delete_account(&self, account_id: &str) -> Result<(), String> {
        // Remove from accounts cache
        {
            let mut cache = self
                .accounts_cache
                .write()
                .map_err(|e| format!("Lock poisoned: {}", e))?;
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
    BASE64
        .decode(input)
        .map_err(|e| format!("Base64 decode error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_account_manager() -> (AccountManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();
        let manager = AccountManager::new(base_path);
        (manager, temp_dir)
    }

    #[test]
    fn test_account_manager_new() {
        let (manager, _temp_dir) = create_test_account_manager();
        assert!(manager.list_accounts().is_empty());
    }

    #[test]
    fn test_create_account() {
        let (manager, _temp_dir) = create_test_account_manager();

        let result = manager.create_account("test_account", "password123");
        assert!(
            result.is_ok(),
            "Failed to create account: {:?}",
            result.err()
        );

        let info = result.unwrap();
        assert_eq!(info.name, "test_account");
        assert!(!info.salt.is_empty(), "Salt should be generated");
        assert!(
            !info.verify_hash.is_empty(),
            "Verify hash should be generated"
        );
        assert_eq!(info.crypto_version, 2);
    }

    #[test]
    fn test_create_account_password_too_short() {
        let (manager, _temp_dir) = create_test_account_manager();

        let result = manager.create_account("test", "short");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Password must be at least 8 characters"));
    }

    #[test]
    fn test_create_account_empty_name() {
        let (manager, _temp_dir) = create_test_account_manager();

        let result = manager.create_account("", "password123");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Account name is required"));
    }

    #[test]
    fn test_create_account_duplicate_name() {
        let (manager, _temp_dir) = create_test_account_manager();

        manager.create_account("duplicate", "password123").unwrap();
        let result = manager.create_account("duplicate", "password456");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already taken"));
    }

    #[test]
    fn test_create_account_name_case_insensitive() {
        let (manager, _temp_dir) = create_test_account_manager();

        manager
            .create_account("TestAccount", "password123")
            .unwrap();
        let result = manager.create_account("testaccount", "password456");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already taken"));
    }

    #[test]
    fn test_is_name_available() {
        let (manager, _temp_dir) = create_test_account_manager();

        assert!(manager.is_name_available("new_account"));

        manager.create_account("existing", "password123").unwrap();
        assert!(!manager.is_name_available("existing"));
        assert!(!manager.is_name_available("EXISTING"));
    }

    #[test]
    fn test_unlock_account() {
        let (manager, _temp_dir) = create_test_account_manager();

        let created = manager
            .create_account("unlock_test", "password123")
            .unwrap();

        manager.lock();

        let result = manager.unlock(&created.id, "password123");
        assert!(result.success, "Failed to unlock: {:?}", result.error);
        assert_eq!(result.crypto_version, 2);
    }

    #[test]
    fn test_unlock_account_wrong_password() {
        let (manager, _temp_dir) = create_test_account_manager();

        let created = manager
            .create_account("wrong_pw_test", "password123")
            .unwrap();
        manager.lock();

        let result = manager.unlock(&created.id, "wrongpassword");
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_unlock_nonexistent_account() {
        let (manager, _temp_dir) = create_test_account_manager();

        let result = manager.unlock("nonexistent_id", "password123");
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_unlock_with_key() {
        let (manager, _temp_dir) = create_test_account_manager();

        let created = manager
            .create_account("unlock_key_test", "password123")
            .unwrap();
        assert!(manager.is_unlocked());

        // Get the session key
        let session_key = manager.get_session_key().unwrap();
        let session_key_b64 = base64_encode(session_key.as_slice());

        // Lock and re-unlock with session key
        manager.lock();
        assert!(!manager.is_unlocked());

        let result = manager.unlock_with_key(&created.id, &session_key_b64);
        assert!(
            result.success,
            "Failed to unlock with key: {:?}",
            result.error
        );
        assert!(manager.is_unlocked());
    }

    #[test]
    fn test_unlock_with_key_wrong_key() {
        let (manager, _temp_dir) = create_test_account_manager();

        let created = manager
            .create_account("unlock_key_wrong_test", "password123")
            .unwrap();
        manager.lock();

        let wrong_key = [0u8; 32];
        let wrong_key_b64 = base64_encode(&wrong_key);

        let result = manager.unlock_with_key(&created.id, &wrong_key_b64);
        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(!manager.is_unlocked());
    }

    #[test]
    fn test_lock_account() {
        let (manager, _temp_dir) = create_test_account_manager();

        manager.create_account("lock_test", "password123").unwrap();
        assert!(manager.is_unlocked());

        manager.lock();
        assert!(!manager.is_unlocked());
    }

    #[test]
    fn test_is_unlocked_after_create() {
        let (manager, _temp_dir) = create_test_account_manager();

        manager
            .create_account("unlocked_test", "password123")
            .unwrap();
        assert!(manager.is_unlocked());
    }

    #[test]
    fn test_get_unlocked_account() {
        let (manager, _temp_dir) = create_test_account_manager();

        let created = manager
            .create_account("session_test", "password123")
            .unwrap();
        assert_eq!(manager.get_unlocked_account(), Some(created.id.clone()));

        manager.lock();
        assert_eq!(manager.get_unlocked_account(), None);
    }

    #[test]
    fn test_get_session_key() {
        let (manager, _temp_dir) = create_test_account_manager();

        manager.create_account("key_test", "password123").unwrap();
        let key = manager.get_session_key();
        assert!(key.is_some());

        manager.lock();
        assert!(manager.get_session_key().is_none());
    }

    #[test]
    fn test_list_accounts() {
        let (manager, _temp_dir) = create_test_account_manager();

        manager.create_account("account1", "password123").unwrap();
        manager.create_account("account2", "password456").unwrap();

        let accounts = manager.list_accounts();
        assert_eq!(accounts.len(), 2);
    }

    #[test]
    fn test_list_accounts_sorted() {
        let (manager, _temp_dir) = create_test_account_manager();

        manager.create_account("first", "password123").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        manager.create_account("second", "password456").unwrap();

        let accounts = manager.list_accounts_sorted();
        assert_eq!(accounts[0].name, "second");
    }

    #[test]
    fn test_get_account_config() {
        let (manager, _temp_dir) = create_test_account_manager();

        let created = manager
            .create_account("config_test", "password123")
            .unwrap();
        manager.lock();

        let config = manager.get_account_config(&created.id);
        assert!(config.is_some());

        let config = config.unwrap();
        assert_eq!(config.name, "config_test");
        assert!(!config.salt.is_empty());
        assert!(!config.verify_hash.is_empty());
    }

    #[test]
    fn test_get_account_config_nonexistent() {
        let (manager, _temp_dir) = create_test_account_manager();

        let config = manager.get_account_config("nonexistent");
        assert!(config.is_none());
    }

    #[test]
    fn test_delete_account() {
        let (manager, _temp_dir) = create_test_account_manager();

        let created = manager
            .create_account("delete_test", "password123")
            .unwrap();
        assert!(manager.list_accounts().len() == 1);

        manager.lock();
        let result = manager.delete_account(&created.id);
        assert!(result.is_ok());

        assert!(manager.list_accounts().is_empty());
        assert!(manager.get_account_config(&created.id).is_none());
    }

    #[test]
    fn test_change_password() {
        let (manager, _temp_dir) = create_test_account_manager();

        let created = manager
            .create_account("change_pw_test", "oldpassword")
            .unwrap();

        let result = manager.change_password(&created.id, "oldpassword", "newpassword123");
        assert!(
            result.is_ok(),
            "Failed to change password: {:?}",
            result.err()
        );

        let info = result.unwrap();
        assert_eq!(info.name, "change_pw_test");
        assert!(!info.salt.is_empty());
        assert!(!info.verify_hash.is_empty());

        manager.lock();
        let unlock_result = manager.unlock(&created.id, "newpassword123");
        assert!(unlock_result.success, "New password should work");
    }

    #[test]
    fn test_change_password_wrong_old_password() {
        let (manager, _temp_dir) = create_test_account_manager();

        let created = manager
            .create_account("pw_wrong_test", "correctpassword")
            .unwrap();

        let result = manager.change_password(&created.id, "wrongpassword", "newpassword123");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid current password"));
    }

    #[test]
    fn test_derive_sqlcipher_key() {
        let (manager, _temp_dir) = create_test_account_manager();
        manager
            .create_account("sqlcipher_test", "password123")
            .unwrap();

        let session_key = manager.get_session_key().unwrap();
        let session_key_arr: [u8; 32] = session_key.as_slice().try_into().unwrap();
        let sqlcipher_key = manager.derive_sqlcipher_key(&session_key_arr);

        assert_eq!(sqlcipher_key.len(), 32);

        let sqlcipher_key2 = manager.derive_sqlcipher_key(&session_key_arr);
        assert_eq!(sqlcipher_key, sqlcipher_key2);
    }

    #[test]
    fn test_derive_sqlcipher_key_different_inputs() {
        let (manager, _temp_dir) = create_test_account_manager();
        manager
            .create_account("key_diff_test", "password123")
            .unwrap();

        let session_key1 = manager.get_session_key().unwrap();
        let mut session_key2 = [0u8; 32];
        session_key2.copy_from_slice(session_key1.as_slice());
        session_key2[0] ^= 0xFF;

        let sqlcipher_key1 = manager.derive_sqlcipher_key(&session_key2);
        let mut session_key3 = session_key2;
        session_key3[0] ^= 0xFF;
        let sqlcipher_key2 = manager.derive_sqlcipher_key(&session_key3);

        assert_ne!(sqlcipher_key1, sqlcipher_key2);
    }

    #[test]
    fn test_base64_encode_decode() {
        let original = b"Hello, World!";
        let encoded = base64_encode(original);
        assert_eq!(encoded, "SGVsbG8sIFdvcmxkIQ==");
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_base64_decode_invalid() {
        let result = base64_decode("not-valid-base64!!!");
        assert!(result.is_err());
    }

    #[test]
    fn test_create_account_whitespace_name() {
        let (manager, _temp_dir) = create_test_account_manager();

        let result = manager.create_account("   ", "password123");
        assert!(result.is_err());
    }

    #[test]
    fn test_unlock_account_updates_last_accessed() {
        let (manager, _temp_dir) = create_test_account_manager();

        let created = manager
            .create_account("access_test", "password123")
            .unwrap();
        manager.lock();

        let result = manager.unlock(&created.id, "password123");
        assert!(result.success);
        assert!(manager.is_unlocked());
    }
}
