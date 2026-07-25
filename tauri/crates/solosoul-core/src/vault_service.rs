//! Vault service - manages accounts and vault lifecycle.
//! Stores accounts in ~/.solosoul/ with per-account config and vault.db

#[cfg(test)]
use crate::biometric::legacy::FileBiometricStorage;
use crate::biometric::BiometricManager;
use crate::pin::PinManager;
use crate::vault_file_system::{LocalVaultFileSystem, VaultFileSystem};
use serde::{Deserialize, Serialize};
use solosoul_crypto::kdf::{derive_key, generate_salt, KdfConfig};
use solosoul_crypto::secure::secure_compare;
use solosoul_vault::{VaultConfig, VaultStore};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use zeroize::{Zeroize, Zeroizing};

// ── Platform‑specific private file/directory permissions ──────────
// Unix: chmod 0700/0600
// Windows: icacls — remove inheritance, grant Full‑Control to current user only

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

#[cfg(windows)]
/// Validate Windows username to prevent icacls command injection.
/// Only allow characters valid in Windows usernames.
fn sanitize_windows_username(username: &str) -> Result<String, String> {
    if username.is_empty() {
        return Err("Windows username is empty".to_string());
    }
    if !username
        .chars()
        .all(|c| c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' || c == '.')
    {
        return Err(format!(
            "Windows username contains invalid characters: {}",
            username
        ));
    }
    Ok(username.to_string())
}

#[cfg(windows)]
fn set_private_dir(path: &Path) -> Result<(), String> {
    let path_str = path.to_string_lossy();
    let username = std::env::var("USERNAME")
        .map_err(|_| "USERNAME environment variable not found".to_string())?;
    let username = sanitize_windows_username(&username)?;
    let status = std::process::Command::new("icacls")
        .args([
            path_str.as_ref(),
            "/inheritance:r",
            "/grant",
            &format!("{username}:(OI)(CI)F"),
        ])
        .status()
        .map_err(|e| format!("icacls failed to start: {e}"))?;
    if !status.success() {
        return Err(format!(
            "icacls returned exit code {:?} when setting permissions on {path_str}",
            status.code()
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn set_private_file(path: &Path) -> Result<(), String> {
    let path_str = path.to_string_lossy();
    let username = std::env::var("USERNAME")
        .map_err(|_| "USERNAME environment variable not found".to_string())?;
    let username = sanitize_windows_username(&username)?;
    let status = std::process::Command::new("icacls")
        .args([
            path_str.as_ref(),
            "/inheritance:r",
            "/grant",
            &format!("{username}:F"),
        ])
        .status()
        .map_err(|e| format!("icacls failed to start: {e}"))?;
    if !status.success() {
        return Err(format!(
            "icacls returned exit code {:?} when setting permissions on {path_str}",
            status.code()
        ));
    }
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
    #[serde(rename = "biometricEnabled")]
    pub biometric_enabled: bool,
    /// KDF 参数：存储的 memory_kb，None 表示使用 KdfConfig::balanced() 向后兼容
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kdf_memory_kb: Option<u32>,
    /// KDF 参数：存储的 iterations
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kdf_iterations: Option<u32>,
    /// KDF 参数：存储的 parallelism
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kdf_parallelism: Option<u32>,

    // ── PIN 相关字段 ──
    /// PIN 解锁是否已启用。
    #[serde(default, rename = "pinEnabled")]
    pub pin_enabled: bool,
    /// PIN 长度（4~8）。
    #[serde(default, rename = "pinLength")]
    pub pin_length: u32,
    /// 连续 PIN 错误次数。
    #[serde(default, rename = "pinFailedAttempts")]
    pub pin_failed_attempts: u32,
    /// PIN 锁定截止时间（ISO 8601），None 表示未锁定。
    #[serde(default, rename = "pinLockedUntil")]
    pub pin_locked_until: Option<String>,
}

impl AccountConfig {
    /// 读取账户配置中存储的 KDF 参数。
    /// 对于旧账户（无存储字段），回退到 `KdfConfig::balanced()` 保证向后兼容。
    pub fn kdf_config(&self) -> KdfConfig {
        KdfConfig {
            memory_kb: self.kdf_memory_kb.unwrap_or(16 * 1024),
            iterations: self.kdf_iterations.unwrap_or(3),
            parallelism: self.kdf_parallelism.unwrap_or(4),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AccountEntry {
    id: String,
    name: String,
    created_at: String,
    last_accessed: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSummary {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub salt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verify_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

pub struct VaultService {
    base_path: PathBuf,
    fs: Arc<dyn VaultFileSystem>,
    accounts_cache: RwLock<HashMap<String, AccountEntry>>,
    session_key: RwLock<Option<Zeroizing<[u8; 32]>>>,
    unlocked_account: RwLock<Option<String>>,
    vault_store: RwLock<Option<Arc<VaultStore>>>,
    /// Serializes `create_account` to eliminate the check-then-act race on
    /// account name uniqueness (R024).
    create_lock: std::sync::Mutex<()>,
}

#[cfg(not(test))]
fn make_biometric_manager(base_path: PathBuf) -> BiometricManager {
    BiometricManager::new(base_path)
}

#[cfg(test)]
fn make_biometric_manager(base_path: PathBuf) -> BiometricManager {
    BiometricManager::with_storage(
        base_path.clone(),
        Box::new(FileBiometricStorage::new(base_path)),
    )
}

impl VaultService {
    pub fn new() -> Self {
        let base_path = Self::default_base_path();
        let svc = Self::with_base_path(base_path);
        svc.load_accounts();
        svc
    }

    /// 使用指定的基础路径创建 VaultService（P120: 避免测试中 set_var 污染）。
    /// 不自动从 env var 读取路径，也不调用 load_accounts（由调用者按需初始化）。
    pub fn with_base_path(base_path: PathBuf) -> Self {
        let fs: Arc<dyn VaultFileSystem> = Arc::new(LocalVaultFileSystem::new(base_path.clone()));
        Self::with_file_system(base_path, fs)
    }

    /// 使用自定义文件系统创建 VaultService。
    ///
    /// 调用者应自行调用 `load_accounts()` 初始化账户缓存。
    pub fn with_file_system(base_path: PathBuf, fs: Arc<dyn VaultFileSystem>) -> Self {
        Self {
            base_path,
            fs,
            accounts_cache: RwLock::new(HashMap::new()),
            session_key: RwLock::new(None),
            unlocked_account: RwLock::new(None),
            vault_store: RwLock::new(None),
            create_lock: std::sync::Mutex::new(()),
        }
    }

    fn default_base_path() -> PathBuf {
        if let Ok(dir) = std::env::var("SOLOSOUL_DATA_DIR") {
            return PathBuf::from(dir);
        }
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

    fn accounts_file_rel(&self) -> &str {
        "accounts.json"
    }

    fn account_dir_rel(&self, id: &str) -> String {
        id.to_string()
    }

    fn config_path_rel(&self, id: &str) -> String {
        format!("{id}/config.json")
    }

    fn ensure_private_dir(&self, rel: &str) -> Result<(), String> {
        if let Some(path) = self.fs.local_path(rel) {
            set_private_dir(&path)?;
        }
        Ok(())
    }

    fn ensure_private_file(&self, rel: &str) -> Result<(), String> {
        if let Some(path) = self.fs.local_path(rel) {
            set_private_file(&path)?;
        }
        Ok(())
    }

    pub fn load_accounts(&self) {
        let rel = self.accounts_file_rel();
        match self.fs.exists(rel) {
            Ok(false) | Err(_) => {
                tracing::warn!("Accounts file does not exist: {}", rel);
                return;
            }
            Ok(true) => {}
        }
        match self.fs.read_file(rel) {
            Ok(content) => match serde_json::from_slice::<Vec<AccountEntry>>(&content) {
                Ok(accounts) => {
                    if let Ok(mut cache) = self.accounts_cache.write() {
                        for a in accounts {
                            cache.insert(a.id.clone(), a);
                        }
                        tracing::debug!("Loaded {} account(s) from {}", cache.len(), rel);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to parse accounts file: {}", e);
                }
            },
            Err(e) => {
                tracing::error!("Failed to read accounts file: {}", e);
            }
        }
    }

    fn save_accounts(&self) -> Result<(), String> {
        let cache = self.accounts_cache.read().map_err(|e| e.to_string())?;
        let list: Vec<&AccountEntry> = cache.values().collect();
        let content = serde_json::to_string_pretty(&list).map_err(|e| e.to_string())?;
        self.fs.create_dir_all("")?;
        self.ensure_private_dir("")?;
        let rel = self.accounts_file_rel();
        self.fs.write_file(rel, content.as_bytes())?;
        self.ensure_private_file(rel)?;
        Ok(())
    }

    pub fn has_any_account(&self) -> bool {
        self.accounts_cache
            .read()
            .map(|c| !c.is_empty())
            .unwrap_or(false)
    }

    pub fn list_accounts(&self) -> Vec<AccountSummary> {
        let cache = self.accounts_cache.read().ok();
        let accounts = match cache {
            Some(ref c) => c.values().cloned().collect::<Vec<_>>(),
            None => return vec![],
        };
        let mut result = Vec::new();
        for entry in &accounts {
            let config_rel = self.config_path_rel(&entry.id);
            let (salt, verify_hash, password_hint, created_at) =
                match self.fs.read_file(&config_rel) {
                    Ok(content) => match serde_json::from_slice::<AccountConfig>(&content) {
                        Ok(cfg) => (
                            Some(cfg.salt),
                            Some(cfg.verify_hash),
                            cfg.password_hint,
                            Some(cfg.created_at),
                        ),
                        Err(_) => (None, None, None, None),
                    },
                    Err(_) => (None, None, None, None),
                };

            result.push(AccountSummary {
                id: entry.id.clone(),
                name: entry.name.clone(),
                salt,
                verify_hash,
                password_hint,
                created_at,
            });
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

        // R024: serialize account creation to eliminate the race between the
        // name-uniqueness check and writing the new account to disk/cache.
        let _create_guard = self.create_lock.lock().map_err(|e| e.to_string())?;

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
        let kdf_config = KdfConfig::from_env();
        let master_key = derive_key(password, &salt, &kdf_config)
            .map_err(|e| format!("Key derivation failed: {}", e))?;

        let mk: [u8; 32] = master_key
            .as_slice()
            .try_into()
            .map_err(|_| "Master key must be 32 bytes".to_string())?;
        let verify_hash = hex::encode(
            solosoul_crypto::hkdf_ext::derive_hkdf_key(&mk, &salt, b"SOLOSOUL_VAULT_VERIFY_v1")
                .map_err(|e| format!("Verify HKDF failed: {}", e))?,
        );

        let dir_rel = self.account_dir_rel(&account_id);
        self.fs.create_dir_all(&dir_rel)?;
        self.ensure_private_dir(&dir_rel)?;

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
            crypto_version: 3, // P2-010: HKDF-based verify hash
            biometric_enabled: false,
            pin_enabled: false,
            pin_length: 0,
            pin_failed_attempts: 0,
            pin_locked_until: None,
            kdf_memory_kb: Some(kdf_config.memory_kb),
            kdf_iterations: Some(kdf_config.iterations),
            kdf_parallelism: Some(kdf_config.parallelism),
            password_hint: password_hint.map(|s| s.to_string()),
            last_login_at: Some(now.clone()),
            last_operation_at: None,
            last_operation_desc: None,
        };
        let config_rel = self.config_path_rel(&account_id);
        let config_json = serde_json::to_string_pretty(&config_data).map_err(|e| e.to_string())?;
        self.fs.write_file(&config_rel, config_json.as_bytes())?;
        self.ensure_private_file(&config_rel)?;

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

        // Open vault with data key
        let master_key_arr: [u8; 32] = master_key
            .as_slice()
            .try_into()
            .map_err(|_| "HKDF output must be 32 bytes".to_string())?;
        let account_dir_path = self.fs.local_path(&dir_rel).ok_or("无法解析账户本地目录")?;
        let vault_config =
            VaultConfig::new(&account_id, account_dir_path).with_data_key(master_key_arr);
        let vault =
            VaultStore::open(vault_config).map_err(|e| format!("Failed to open vault: {}", e))?;
        let vault_arc = Arc::new(vault);
        if let Ok(mut store) = self.vault_store.write() {
            *store = Some(vault_arc);
        }
        if let Ok(mut key) = self.session_key.write() {
            *key = Some(Zeroizing::new(master_key_arr));
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

    /// 安全解锁：接受 Zeroizing<String> 主密码，避免调用侧额外明文拷贝。
    pub fn unlock_secure(
        &self,
        account_id: &str,
        password: &Zeroizing<String>,
    ) -> Result<(), String> {
        self.unlock(account_id, password.as_ref())
    }

    pub fn unlock(&self, account_id: &str, password: &str) -> Result<(), String> {
        let config_rel = self.config_path_rel(account_id);
        let content = self
            .fs
            .read_file(&config_rel)
            .map_err(|_| "Account not found".to_string())?;
        let content =
            String::from_utf8(content).map_err(|_| "Config encoding error".to_string())?;
        let config: AccountConfig =
            serde_json::from_str(&content).map_err(|_| "Config parse error".to_string())?;

        let salt_bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &config.salt)
                .map_err(|_| "Invalid salt".to_string())?;
        let salt_arr: [u8; 16] = salt_bytes
            .as_slice()
            .try_into()
            .map_err(|_| "Invalid salt length".to_string())?;

        let kdf_config = config.kdf_config();
        let master_key = derive_key(password, &salt_arr, &kdf_config)
            .map_err(|_| "Key derivation failed".to_string())?;

        let mk: [u8; 32] = master_key
            .as_slice()
            .try_into()
            .map_err(|_| "Master key must be 32 bytes".to_string())?;
        // Backward compat: crypto_version < 3 uses old Argon2id verify hash
        let computed_hash = if config.crypto_version < 3 {
            let verify_key = derive_key(
                &hex::encode(master_key.as_slice()),
                b"SOLOSOUL_VAULT_VERIFY_v1",
                &KdfConfig {
                    memory_kb: 8192,
                    iterations: 1,
                    parallelism: 1,
                },
            )
            .map_err(|_| "Verify failed".to_string())?;
            hex::encode(verify_key.as_slice())
        } else {
            hex::encode(
                solosoul_crypto::hkdf_ext::derive_hkdf_key(
                    &mk,
                    &salt_arr,
                    b"SOLOSOUL_VAULT_VERIFY_v1",
                )
                .map_err(|_| "Verify HKDF failed".to_string())?,
            )
        };

        if !secure_compare(computed_hash.as_bytes(), config.verify_hash.as_bytes()) {
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
        let master_key_arr: [u8; 32] = master_key
            .as_slice()
            .try_into()
            .map_err(|_| "Argon2id output must be 32 bytes".to_string())?;
        if let Ok(mut key) = self.session_key.write() {
            *key = Some(Zeroizing::new(master_key_arr));
        }
        if let Ok(mut ua) = self.unlocked_account.write() {
            *ua = Some(account_id.to_string());
        }

        // Open vault with data key
        let account_dir_path = self
            .fs
            .local_path(&self.account_dir_rel(account_id))
            .ok_or("无法解析账户本地目录")?;
        let vault_config =
            VaultConfig::new(account_id, account_dir_path).with_data_key(master_key_arr);
        let vault =
            VaultStore::open(vault_config).map_err(|e| format!("Failed to open vault: {}", e))?;
        let vault_arc = Arc::new(vault);
        if let Ok(mut store) = self.vault_store.write() {
            *store = Some(vault_arc);
        }

        // 用户已通过主密码验证身份，重置 PIN 锁定状态。
        let pin_manager = PinManager::new(self.base_path().clone());
        if let Err(e) = pin_manager.reset_attempts(account_id) {
            tracing::warn!("Failed to reset PIN attempts after unlock: {}", e);
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
    /// Verify hash is derived from the Argon2id master key using HKDF-SHA256 (P2-010).
    pub fn verify_password(&self, account_id: &str, password: &str) -> Result<bool, String> {
        let config_rel = self.config_path_rel(account_id);
        let content = self
            .fs
            .read_file(&config_rel)
            .map_err(|_| "Account not found".to_string())?;
        let content =
            String::from_utf8(content).map_err(|_| "Config encoding error".to_string())?;
        let config: AccountConfig =
            serde_json::from_str(&content).map_err(|_| "Config parse error".to_string())?;

        let salt_bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &config.salt)
                .map_err(|_| "Invalid salt".to_string())?;
        let salt_arr: [u8; 16] = salt_bytes
            .as_slice()
            .try_into()
            .map_err(|_| "Invalid salt length".to_string())?;

        let kdf_config = config.kdf_config();
        let master_key = derive_key(password, &salt_arr, &kdf_config)
            .map_err(|_| "Key derivation failed".to_string())?;

        let mk: [u8; 32] = master_key
            .as_slice()
            .try_into()
            .map_err(|_| "Master key must be 32 bytes".to_string())?;
        // Backward compat: crypto_version < 3 uses old Argon2id verify hash
        let computed_hash = if config.crypto_version < 3 {
            let verify_key = derive_key(
                &hex::encode(master_key.as_slice()),
                b"SOLOSOUL_VAULT_VERIFY_v1",
                &KdfConfig {
                    memory_kb: 8192,
                    iterations: 1,
                    parallelism: 1,
                },
            )
            .map_err(|_| "Verify failed".to_string())?;
            hex::encode(verify_key.as_slice())
        } else {
            hex::encode(
                solosoul_crypto::hkdf_ext::derive_hkdf_key(
                    &mk,
                    &salt_arr,
                    b"SOLOSOUL_VAULT_VERIFY_v1",
                )
                .map_err(|_| "Verify HKDF failed".to_string())?,
            )
        };

        Ok(secure_compare(
            computed_hash.as_bytes(),
            config.verify_hash.as_bytes(),
        ))
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

        // Open vault with data key
        let account_dir_path = self
            .fs
            .local_path(&self.account_dir_rel(account_id))
            .ok_or("无法解析账户本地目录")?;
        let vault_config =
            VaultConfig::new(account_id, account_dir_path).with_data_key(*session_key);
        let vault =
            VaultStore::open(vault_config).map_err(|e| format!("Failed to open vault: {}", e))?;
        let vault_arc = Arc::new(vault);
        if let Ok(mut store) = self.vault_store.write() {
            *store = Some(vault_arc);
        }

        // 用户已通过更强因子（生物识别 / PIN 本身）验证身份，重置 PIN 锁定状态。
        let pin_manager = PinManager::new(self.base_path().clone());
        if let Err(e) = pin_manager.reset_attempts(account_id) {
            tracing::warn!("Failed to reset PIN attempts after unlock: {}", e);
        }

        Ok(())
    }

    pub fn change_password(
        &self,
        account_id: &str,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), String> {
        // Verify old password first; this opens the vault with the old data key.
        self.unlock(account_id, old_password)?;

        // Capture old data key.
        let old_key_arr = self
            .get_session_key()
            .ok_or("Session key not available after unlock")?;
        let old_key = solosoul_vault::DataEncryptionKey::new(*old_key_arr);

        // Generate new salt and derive new key.
        let salt = generate_salt();
        let new_kdf_config = KdfConfig::from_env();
        let new_key = derive_key(new_password, &salt, &new_kdf_config)
            .map_err(|e| format!("New key derivation failed: {}", e))?;
        let new_key_arr: [u8; 32] = new_key
            .as_slice()
            .try_into()
            .map_err(|_| "Key derivation output must be 32 bytes".to_string())?;
        let new_key_enc = solosoul_vault::DataEncryptionKey::new(new_key_arr);

        // Re-encrypt all sensitive data with the new key.
        {
            let vault_guard = self
                .get_vault_store()
                .ok_or("Vault not available for re-encryption")?;
            let vault = vault_guard.as_ref();
            vault.reencrypt_all(&old_key, &new_key_enc)?;
        }

        // Derive new verify hash via HKDF (P2-010).
        let mk: [u8; 32] = new_key_arr; // already 32 bytes from try_into above
        let verify_hash = hex::encode(
            solosoul_crypto::hkdf_ext::derive_hkdf_key(&mk, &salt, b"SOLOSOUL_VAULT_VERIFY_v1")
                .map_err(|e| format!("Verify HKDF failed: {}", e))?,
        );

        // Update config
        let config_rel = self.config_path_rel(account_id);
        let content = self
            .fs
            .read_file(&config_rel)
            .map_err(|_| "Account not found".to_string())?;
        let content =
            String::from_utf8(content).map_err(|_| "Config encoding error".to_string())?;
        let mut config: AccountConfig =
            serde_json::from_str(&content).map_err(|_| "Config parse error".to_string())?;
        config.crypto_version = 3; // P2-010: HKDF-based verify hash
        config.salt =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, salt.as_slice());
        config.verify_hash = verify_hash;
        config.kdf_memory_kb = Some(new_kdf_config.memory_kb);
        config.kdf_iterations = Some(new_kdf_config.iterations);
        config.kdf_parallelism = Some(new_kdf_config.parallelism);
        let config_json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
        self.fs.write_file(&config_rel, config_json.as_bytes())?;

        // Update session key and reopen vault with new data key.
        {
            if let Ok(mut key) = self.session_key.write() {
                *key = Some(Zeroizing::new(new_key_arr));
            }
        }
        if let Ok(mut store) = self.vault_store.write() {
            *store = None;
        }
        let account_dir_path = self
            .fs
            .local_path(&self.account_dir_rel(account_id))
            .ok_or("无法解析账户本地目录")?;
        let vault_config =
            VaultConfig::new(account_id, account_dir_path).with_data_key(new_key_arr);
        match VaultStore::open(vault_config) {
            Ok(vault) => {
                if let Ok(mut store) = self.vault_store.write() {
                    *store = Some(Arc::new(vault));
                }
            }
            Err(e) => {
                return Err(format!("Password updated but vault reopen failed: {}", e));
            }
        }

        // 如果用户已启用生物识别，同步更新其中保存的主密钥，使改密后 Touch ID 仍可用。
        {
            let bio_manager = make_biometric_manager(self.base_path().clone());
            let new_key_hex = hex::encode(new_key_arr.as_slice());
            if let Err(e) = bio_manager.update_credential(account_id, &new_key_hex) {
                tracing::warn!(
                    "Failed to update biometric credential after password change for {}: {}",
                    account_id,
                    e
                );
            }
        }

        // 如果用户已启用 PIN 解锁，同步更新 PIN 凭证。
        // 由于 PIN 派生 KEK 时需要 PIN 输入（不可用），此处清除凭证并标记为未配置，
        // 用户需要重新设置 PIN。
        {
            let pin_manager = PinManager::new(self.base_path().clone());
            if let Err(e) = pin_manager.clear_credential(account_id) {
                tracing::warn!(
                    "Failed to update PIN credential after password change for {}: {}",
                    account_id,
                    e
                );
            }
        }

        Ok(())
    }

    pub fn delete_account(&self, account_id: &str) -> Result<(), String> {
        self.lock();
        if let Ok(mut cache) = self.accounts_cache.write() {
            cache.remove(account_id);
        }
        self.save_accounts()?;
        let dir_rel = self.account_dir_rel(account_id);
        if self.fs.exists(&dir_rel).unwrap_or(false) {
            self.fs.remove_dir_all(&dir_rel)?;
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

    pub fn get_vault_store(&self) -> Option<Arc<solosoul_vault::VaultStore>> {
        if !self.is_unlocked() {
            return None;
        }
        self.vault_store.read().ok().and_then(|g| g.clone())
    }

    pub fn get_current_account(&self) -> Option<String> {
        self.unlocked_account.read().ok()?.clone()
    }

    /// 将 Vault 数据同步到远端存储（如 SAF）。
    /// 若当前文件系统为本地文件系统，则为空操作。
    pub fn sync_to_remote(&self) -> Result<(), String> {
        self.fs.sync_to_remote()
    }

    /// 从远端存储（如 SAF）同步 Vault 数据到本地。
    /// 若当前文件系统为本地文件系统，则为空操作。
    pub fn sync_from_remote(&self) -> Result<(), String> {
        self.fs.sync_from_remote()
    }

    /// 如果底层文件系统支持脏标记，同步尚未同步到远端的脏数据。
    /// 适用于定期后台自动同步的调用场景。
    pub fn sync_if_dirty(&self) -> Result<(), String> {
        self.fs.sync_if_dirty()
    }

    /// 当前 Vault 是否有尚未同步到远端的脏数据。
    pub fn is_dirty(&self) -> bool {
        self.fs.is_dirty()
    }

    /// 当前 Vault 是否使用远端（SAF）存储。
    pub fn is_remote_storage(&self) -> bool {
        self.fs.is_remote()
    }

    pub fn update_password_hint(&self, account_id: &str, hint: &str) -> Result<(), String> {
        let config_rel = self.config_path_rel(account_id);
        let content = self
            .fs
            .read_file(&config_rel)
            .map_err(|_| "Account not found".to_string())?;
        let content =
            String::from_utf8(content).map_err(|_| "Config encoding error".to_string())?;
        let mut config: AccountConfig =
            serde_json::from_str(&content).map_err(|_| "Config parse error".to_string())?;
        config.password_hint = Some(hint.to_string());
        let config_json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
        self.fs.write_file(&config_rel, config_json.as_bytes())?;
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
        std::fs::create_dir_all(&base).unwrap();
        let svc = VaultService::with_base_path(base);
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

        // Simulate biometric unlock having been enabled
        let config_path = svc.base_path().join(account_id).join("config.json");
        let mut raw: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
        raw["biometricEnabled"] = serde_json::Value::Bool(true);
        fs::write(&config_path, serde_json::to_string_pretty(&raw).unwrap()).unwrap();
        // 用旧版文件模拟已启用生物识别，验证 change_password 不会误删标记。
        let legacy_key_path = svc.base_path().join(account_id).join("biometric_key");
        let legacy_key_hex = "deadbeef1234567890abcdef1234567890abcdef1234567890abcdef12345678";
        fs::write(&legacy_key_path, legacy_key_hex).unwrap();

        svc.unlock(account_id, "oldpassword").unwrap();
        svc.change_password(account_id, "oldpassword", "newpassword")
            .unwrap();

        // Old password should fail
        assert!(!svc.verify_password(account_id, "oldpassword").unwrap());
        // New password should succeed
        assert!(svc.verify_password(account_id, "newpassword").unwrap());

        let content = fs::read_to_string(&config_path).unwrap();
        let config: AccountConfig = serde_json::from_str(&content).unwrap();
        // 修改密码后生物识别启用标记应保持为 true；测试使用文件存储后端，实际密钥应同步更新。
        assert!(config.biometric_enabled);

        let bio_manager = make_biometric_manager(svc.base_path().clone());
        assert!(bio_manager.is_configured(account_id));
        let expected_hex = hex::encode(svc.get_session_key().unwrap().as_slice());
        let stored_hex = bio_manager
            .read_stored_key_hex(account_id, "verify after change")
            .unwrap();
        assert_eq!(stored_hex, expected_hex);
    }

    #[test]
    fn test_update_password_hint() {
        let (svc, _dir) = setup_service();
        let account = svc.create_account("Frank", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap();

        // Simulate biometric unlock having been enabled
        let config_path = svc.base_path().join(account_id).join("config.json");
        let mut raw: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
        raw["biometricEnabled"] = serde_json::Value::Bool(true);
        fs::write(&config_path, serde_json::to_string_pretty(&raw).unwrap()).unwrap();

        svc.update_password_hint(account_id, "My favorite color")
            .unwrap();

        let content = fs::read_to_string(&config_path).unwrap();
        let config: AccountConfig = serde_json::from_str(&content).unwrap();
        assert_eq!(config.password_hint, Some("My favorite color".to_string()));
        assert!(config.biometric_enabled);
    }

    #[test]
    fn test_delete_account() {
        let (svc, _dir) = setup_service();
        let account = svc.create_account("Grace", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap();

        svc.delete_account(account_id).unwrap();
        assert!(!svc.has_any_account());
        assert!(!svc.base_path().join(account_id).exists());
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
