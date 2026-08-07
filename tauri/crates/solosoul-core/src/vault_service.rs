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

/// P032：主密码失败计数读-改-写的原子化互斥锁（镜像 pin.rs 的 PIN_OP_LOCK 模式，
/// 保证并发解锁时失败计数不丢更新）。
static PASSWORD_ATTEMPT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// P032：主密码锁定错误文案。前端 `rustErrors.ts` 精确匹配后映射为
/// `common:password_locked` 双语文案（镜像 PIN 的 `__PIN_ERR__:locked` 约定）。
const MASTER_PASSWORD_LOCKED_ERR: &str = "Too many failed attempts; try again later";

/// P032：主密码失败限流阶梯（与 PIN 同款：0-4 次不锁，5-9 次锁 30s，
/// 第 10 次锁 5 分钟，之后每次递增 5 分钟）。
fn password_lockout_seconds(failed_attempts: u32) -> u64 {
    match failed_attempts {
        0..=4 => 0,
        5..=9 => 30,
        10 => 300,                        // 5 分钟
        n => 300 + (n - 10) as u64 * 300, // 之后每次递增 5 分钟
    }
}
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
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
        .arg(path_str.as_ref())
        .arg("/inheritance:r")
        .arg("/grant")
        .arg(format!("{username}:(OI)(CI)F"))
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
        .arg(path_str.as_ref())
        .arg("/inheritance:r")
        .arg("/grant")
        .arg(format!("{username}:F"))
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

    // ── 主密码限流相关字段（P032，镜像 PIN） ──
    /// 连续主密码错误次数。
    #[serde(default, rename = "passwordFailedAttempts")]
    pub password_failed_attempts: u32,
    /// 主密码锁定截止时间（ISO 8601），None 表示未锁定。
    #[serde(default, rename = "passwordLockedUntil")]
    pub password_locked_until: Option<String>,
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
    /// 该账户是否曾在卸载前启用过生物识别（指纹/人脸）。
    /// 用于重装后检测到已有账户时，引导用户重新设置。
    #[serde(default)]
    pub has_biometric_history: bool,
    /// 该账户是否曾在卸载前启用过 PIN 码解锁。
    #[serde(default)]
    pub has_pin_history: bool,
}

pub struct VaultService {
    base_path: PathBuf,
    fs: Arc<dyn VaultFileSystem>,
    accounts_cache: RwLock<HashMap<String, AccountEntry>>,
    session_key: RwLock<Option<Zeroizing<[u8; 32]>>>,
    unlocked_account: RwLock<Option<String>>,
    vault_store: RwLock<Option<Arc<VaultStore>>>,
    /// 设备级「同步设置偏好」开关（默认 true=偏好照常同步）。
    /// 跨 unlock 生命周期持久：每次 unlock 新建 VaultStore（其内部开关重置为
    /// 默认 true）时，把本期望值应用上去——用户锁屏再解锁后偏好开关不丢失。
    ui_prefs_sync_enabled: AtomicBool,
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

/// P225: unlock / verify_password 共享派生结果的元组别名（(config, salt_arr, mk, master_key)）。
type DerivedMasterKey = (AccountConfig, [u8; 16], [u8; 32], Zeroizing<Vec<u8>>);

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
            ui_prefs_sync_enabled: AtomicBool::new(true),
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

    /// 设置设备级「同步设置偏好」开关（由 src-tauri 层调用）。
    /// 立即应用当前已解锁的 VaultStore；未解锁时仅记录期望值，
    /// 下次 unlock 新建 VaultStore 时自动应用。
    pub fn set_ui_prefs_sync_enabled(&self, enabled: bool) {
        self.ui_prefs_sync_enabled.store(enabled, Ordering::SeqCst);
        if let Some(v) = self.get_vault_store() {
            v.set_ui_prefs_sync_enabled(enabled);
        }
    }

    /// 读取设备级「同步设置偏好」开关（默认 true）。
    pub fn ui_prefs_sync_enabled(&self) -> bool {
        self.ui_prefs_sync_enabled.load(Ordering::SeqCst)
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

    /// R-4① 方案 2：两阶段 config 交换的 pending 载体路径。
    /// reencrypt 前先把新 config 内容原子写到这里，交换完成后删除；
    /// 崩溃后残留的 pending 文件由 `recover_pending_reencrypt` 消费。
    fn config_pending_path_rel(&self, id: &str) -> String {
        format!("{id}/config.json.pending")
    }

    /// R-4① 方案 2：原子写 pending config（意图记录）。与 `write_config_atomic`
    /// 同款 .tmp+rename 原子语义（复用 fs 层的 write_file_atomic）。
    fn write_config_pending(&self, account_id: &str, content: &[u8]) -> Result<(), String> {
        let pending_rel = self.config_pending_path_rel(account_id);
        self.fs.write_file_atomic(&pending_rel, content)?;
        self.ensure_private_file(&pending_rel)?;
        Ok(())
    }

    /// R-4① 方案 2：删除 pending config（best-effort，日志兜底）。
    fn remove_config_pending(&self, account_id: &str) {
        let pending_rel = self.config_pending_path_rel(account_id);
        if self.fs.exists(&pending_rel).unwrap_or(false) {
            if let Err(e) = self.fs.remove_file(&pending_rel) {
                tracing::warn!("Failed to remove pending config for {}: {}", account_id, e);
            }
        }
    }

    /// P135：原子写账户 config（.tmp + rename）。
    ///
    /// 与 R-4① 协同：reencrypt→config 两阶段中 config 写入是最后一步，
    /// 原子写保证「写一半/进程崩溃」时目标文件要么是旧内容要么是新内容，
    /// 不会出现截断/损坏的 config（崩溃后残留孤儿 .tmp 由读取侧
    /// `recover_config_or_load` 或下次原子写覆盖）。
    fn write_config_atomic(&self, account_id: &str, content: &[u8]) -> Result<(), String> {
        let config_rel = self.config_path_rel(account_id);
        self.fs.write_file_atomic(&config_rel, content)?;
        self.ensure_private_file(&config_rel)?;
        // 评审补强：write_atomic 的 fs::copy 生成的 .bak 权限为 umask 默认（0644），
        // 与主文件 0600 不一致——.bak 含 salt/verify_hash/hint 同敏感级，
        // 需同等收紧（目录 0700 仅是纵深兜底）。
        if let Some(path) = self.fs.local_path(&config_rel) {
            let bak_path = path.with_extension("bak");
            if bak_path.exists() {
                set_private_file(&bak_path).map_err(|e| format!("收紧 .bak 权限失败: {e}"))?;
            }
        }
        Ok(())
    }

    /// P135：读取 config，优先恢复孤儿 .tmp/.bak（配合原子写）。
    ///
    /// 正常路径 = 直接读主文件；主文件缺失/非法时回退到
    /// `safe_storage::recover_or_load`（提升孤儿 .tmp、回退 .bak）。
    /// 读取 config，优先恢复孤儿 .tmp/.bak（配合原子写）。
    ///
    /// 原子写保证主文件「要么旧要么新」不会截断，但外部因素（磁盘损坏/
    /// 旧版本残留/手工改动）仍可能留下非法 JSON——此时也回退到
    /// `safe_storage::recover_or_load`（提升孤儿 .tmp、回退 .bak）。
    fn read_config_with_recovery(&self, account_id: &str) -> Result<Vec<u8>, String> {
        let config_rel = self.config_path_rel(account_id);
        let recover = |path: &std::path::Path| -> Option<Vec<u8>> {
            if let Some(content) = solosoul_vault::safe_storage::recover_or_load(path) {
                // 评审补强：提升/回退直接改写本地文件，SAF 场景需标脏以便同步到远端。
                let _ = self.fs.sync_if_dirty();
                return Some(content.into_bytes());
            }
            None
        };
        match self.fs.read_file(&config_rel) {
            Ok(content) => {
                // 主文件可读且为合法 JSON → 直接使用。
                if serde_json::from_slice::<serde_json::Value>(&content).is_ok() {
                    return Ok(content);
                }
                // 主文件损坏 → 尝试恢复（孤儿 .tmp 或 .bak）。
                if let Some(path) = self.fs.local_path(&config_rel) {
                    if let Some(recovered) = recover(&path) {
                        return Ok(recovered);
                    }
                }
                // 恢复失败仍返回原内容，调用方按 parse error 处理（语义不变）。
                Ok(content)
            }
            Err(_) => {
                // 主文件缺失（可能是崩溃时 rename 前的孤儿 .tmp 未被提升）。
                if let Some(path) = self.fs.local_path(&config_rel) {
                    if let Some(recovered) = recover(&path) {
                        return Ok(recovered);
                    }
                }
                Err("Account not found".to_string())
            }
        }
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
        // P135: 账户清单同样关键——原子写避免截断后 load_accounts 静默清空。
        self.fs.write_file_atomic(rel, content.as_bytes())?;
        self.ensure_private_file(rel)?;
        Ok(())
    }

    pub fn has_any_account(&self) -> bool {
        self.accounts_cache
            .read()
            .map(|c| !c.is_empty())
            .unwrap_or(false)
    }

    /// 判断指定 `account_id` 的账户是否存在。
    /// 仅查内存缓存（accounts_cache），不做任何文件 IO，
    /// 适合需要高频判断但无需账户详情（如恢复覆盖前的存在性检查）的场景。
    pub fn has_account(&self, account_id: &str) -> bool {
        self.accounts_cache
            .read()
            .map(|c| c.contains_key(account_id))
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
            let (
                salt,
                verify_hash,
                password_hint,
                created_at,
                has_biometric_history,
                has_pin_history,
            ) = match self.fs.read_file(&config_rel) {
                Ok(content) => match serde_json::from_slice::<AccountConfig>(&content) {
                    Ok(cfg) => (
                        Some(cfg.salt),
                        Some(cfg.verify_hash),
                        cfg.password_hint,
                        Some(cfg.created_at),
                        cfg.biometric_enabled,
                        cfg.pin_enabled,
                    ),
                    Err(_) => (None, None, None, None, false, false),
                },
                Err(_) => (None, None, None, None, false, false),
            };

            result.push(AccountSummary {
                id: entry.id.clone(),
                name: entry.name.clone(),
                salt,
                verify_hash,
                password_hint,
                created_at,
                has_biometric_history,
                has_pin_history,
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
            password_failed_attempts: 0,
            password_locked_until: None,
            kdf_memory_kb: Some(kdf_config.memory_kb),
            kdf_iterations: Some(kdf_config.iterations),
            kdf_parallelism: Some(kdf_config.parallelism),
            password_hint: password_hint.map(|s| s.to_string()),
            last_login_at: Some(now.clone()),
            last_operation_at: None,
            last_operation_desc: None,
        };
        let config_json = serde_json::to_string_pretty(&config_data).map_err(|e| e.to_string())?;
        // P135: 原子写（.tmp + rename）——create_account 为关键写入路径。
        self.write_config_atomic(&account_id, config_json.as_bytes())?;

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
        // 设备级偏好同步开关：unlock 新建 VaultStore 后应用期望值（默认 true）。
        vault_arc.set_ui_prefs_sync_enabled(self.ui_prefs_sync_enabled.load(Ordering::SeqCst));
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

    /// 使用指定的 account_id 创建账户（用于跨设备恢复等场景）。
    /// 账户身份以 `account_id` 为准：恢复场景允许同名账户（如大小写不同）共存，
    /// 因此不做账户名唯一性检查；仅当 `account_id` 在本机已存在时返回错误
    /// （"Account ID already exists"），由调用方决定是否覆盖恢复。
    pub fn create_account_with_id(
        &self,
        account_id: &str,
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

        let _create_guard = self.create_lock.lock().map_err(|e| e.to_string())?;

        // 如果该 account_id 已经存在，直接拒绝，避免覆盖已有数据
        if self
            .accounts_cache
            .read()
            .map_err(|e| e.to_string())?
            .contains_key(account_id)
        {
            return Err("Account ID already exists".to_string());
        }

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

        let dir_rel = self.account_dir_rel(account_id);
        self.fs.create_dir_all(&dir_rel)?;
        self.ensure_private_dir(&dir_rel)?;

        let now = chrono::Utc::now().to_rfc3339();
        let config_data = AccountConfig {
            account_id: account_id.to_string(),
            name: name.to_string(),
            salt: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                salt.as_slice(),
            ),
            verify_hash,
            created_at: now.clone(),
            crypto_version: 3,
            biometric_enabled: false,
            pin_enabled: false,
            pin_length: 0,
            pin_failed_attempts: 0,
            pin_locked_until: None,
            password_failed_attempts: 0,
            password_locked_until: None,
            kdf_memory_kb: Some(kdf_config.memory_kb),
            kdf_iterations: Some(kdf_config.iterations),
            kdf_parallelism: Some(kdf_config.parallelism),
            password_hint: password_hint.map(|s| s.to_string()),
            last_login_at: Some(now.clone()),
            last_operation_at: None,
            last_operation_desc: None,
        };
        let config_json = serde_json::to_string_pretty(&config_data).map_err(|e| e.to_string())?;
        // P135: 原子写（.tmp + rename）——create_account_with_id 为关键写入路径。
        self.write_config_atomic(account_id, config_json.as_bytes())?;

        let entry = AccountEntry {
            id: account_id.to_string(),
            name: name.to_string(),
            created_at: now.clone(),
            last_accessed: Some(now),
        };
        if let Ok(mut cache) = self.accounts_cache.write() {
            cache.insert(account_id.to_string(), entry);
        }
        self.save_accounts()?;

        let master_key_arr: [u8; 32] = master_key
            .as_slice()
            .try_into()
            .map_err(|_| "HKDF output must be 32 bytes".to_string())?;
        let account_dir_path = self.fs.local_path(&dir_rel).ok_or("无法解析账户本地目录")?;
        let vault_config =
            VaultConfig::new(account_id, account_dir_path).with_data_key(master_key_arr);
        let vault =
            VaultStore::open(vault_config).map_err(|e| format!("Failed to open vault: {}", e))?;
        let vault_arc = Arc::new(vault);
        // 设备级偏好同步开关：unlock 新建 VaultStore 后应用期望值（默认 true）。
        vault_arc.set_ui_prefs_sync_enabled(self.ui_prefs_sync_enabled.load(Ordering::SeqCst));
        if let Ok(mut store) = self.vault_store.write() {
            *store = Some(vault_arc);
        }
        if let Ok(mut key) = self.session_key.write() {
            *key = Some(Zeroizing::new(master_key_arr));
        }
        if let Ok(mut ua) = self.unlocked_account.write() {
            *ua = Some(account_id.to_string());
        }

        Ok(serde_json::json!({
            "id": account_id, "name": name,
            "salt": config_data.salt, "verifyHash": config_data.verify_hash,
            "passwordHint": config_data.password_hint,
        }))
    }

    /// 安全解锁：接受 Zeroizing<String> 主密码，避免调用侧额外明文拷贝。
    /// P225: 加载账户配置并派生主密钥（unlock / verify_password 共享前缀收敛）。
    /// 返回 (config, salt_arr, mk, master_key)。
    fn load_config_and_derive_master_key(
        &self,
        account_id: &str,
        password: &str,
    ) -> Result<DerivedMasterKey, String> {
        // P135: 带孤儿 .tmp/.bak 恢复的读取（配合原子写）。
        let content = self
            .read_config_with_recovery(account_id)
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
        Ok((config, salt_arr, mk, master_key))
    }

    /// P032：读取并解析账户 config（供锁定预检与失败计数 RMW 使用）。
    fn read_account_config(&self, account_id: &str) -> Result<AccountConfig, String> {
        let content = self
            .read_config_with_recovery(account_id)
            .map_err(|_| "Account not found".to_string())?;
        let content =
            String::from_utf8(content).map_err(|_| "Config encoding error".to_string())?;
        serde_json::from_str(&content).map_err(|_| "Config parse error".to_string())
    }

    /// P032：主密码验证失败——递增失败计数，命中阶梯档位时写入锁定截止时间。
    /// 读-改-写经 `PASSWORD_ATTEMPT_LOCK` 原子化（镜像 PIN_OP_LOCK 模式），
    /// 防止并发解锁时计数丢更新。
    fn record_password_failure(&self, account_id: &str) -> Result<(), String> {
        let _guard = PASSWORD_ATTEMPT_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let mut config = self.read_account_config(account_id)?;
        let attempts = config.password_failed_attempts + 1;
        config.password_failed_attempts = attempts;
        let lockout_secs = password_lockout_seconds(attempts);
        config.password_locked_until = if lockout_secs > 0 {
            Some((chrono::Utc::now() + chrono::Duration::seconds(lockout_secs as i64)).to_rfc3339())
        } else {
            None
        };
        let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
        self.write_config_atomic(account_id, json.as_bytes())
    }

    /// P032：主密码验证成功——归零失败计数与锁定。仅当有残留时才写 config，
    /// 避免每次成功登录都多一次磁盘写入。
    fn clear_password_failures(&self, account_id: &str, config: &AccountConfig) {
        if config.password_failed_attempts == 0 && config.password_locked_until.is_none() {
            return;
        }
        let _guard = PASSWORD_ATTEMPT_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let mut config = match self.read_account_config(account_id) {
            Ok(c) => c,
            Err(_) => return,
        };
        if config.password_failed_attempts == 0 && config.password_locked_until.is_none() {
            return;
        }
        config.password_failed_attempts = 0;
        config.password_locked_until = None;
        if let Ok(json) = serde_json::to_string_pretty(&config) {
            let _ = self.write_config_atomic(account_id, json.as_bytes());
        }
    }

    /// P225: 计算 verify hash（crypto_version<3 旧 Argon2id 路径，否则 HKDF）。
    fn compute_verify_hash(
        config: &AccountConfig,
        mk: &[u8; 32],
        salt_arr: &[u8; 16],
        master_key: &[u8],
    ) -> Result<String, String> {
        if config.crypto_version < 3 {
            let verify_key = derive_key(
                &hex::encode(master_key),
                b"SOLOSOUL_VAULT_VERIFY_v1",
                &KdfConfig {
                    memory_kb: 8192,
                    iterations: 1,
                    parallelism: 1,
                },
            )
            .map_err(|_| "Verify failed".to_string())?;
            Ok(hex::encode(verify_key.as_slice()))
        } else {
            Ok(hex::encode(
                solosoul_crypto::hkdf_ext::derive_hkdf_key(
                    mk,
                    salt_arr,
                    b"SOLOSOUL_VAULT_VERIFY_v1",
                )
                .map_err(|_| "Verify HKDF failed".to_string())?,
            ))
        }
    }

    /// R-4① 方案 2：恢复未完成的 reencrypt→config 两阶段交换。
    ///
    /// 崩溃窗口 = reencrypt 事务已提交、active config 未更新。此时
    /// `config.json.pending`（reencrypt 前写入的新 config 内容）残留：
    /// 用密码派生旧钥（active config）与新钥（pending config）各探测一次数据
    /// （`VaultStore::probe_data_key`），判定 reencrypt 是否已提交：
    /// - 新钥可解密 → reencrypt 已提交 → **promote**：pending 内容原子写为
    ///   active config，删除 pending，账户数据密钥切换到新钥；
    /// - 旧钥可解密 → reencrypt 未提交（事务回滚或从未执行）→ **discard**：
    ///   删除 pending，数据保持旧钥；
    /// - 两者都不可解 → 密码错误（或数据损坏）→ 不删 pending，上抛。
    ///
    /// 常态（无 pending 文件）零开销。probe 为只读，不触发迁移类副作用。
    fn recover_pending_reencrypt(&self, account_id: &str, password: &str) -> Result<(), String> {
        let pending_rel = self.config_pending_path_rel(account_id);
        if !self.fs.exists(&pending_rel).map_err(|e| e.to_string())? {
            return Ok(()); // 常态零开销
        }
        tracing::warn!(
            "R-4①: pending config found for {}, recovering interrupted reencrypt",
            account_id
        );

        // 读取并解析 pending（新）config。
        let pending_bytes = self
            .fs
            .read_file(&pending_rel)
            .map_err(|_| "Pending config read failed".to_string())?;
        let pending_content = String::from_utf8(pending_bytes.clone())
            .map_err(|_| "Pending config encoding error".to_string())?;
        let pending_config: AccountConfig = serde_json::from_str(&pending_content)
            .map_err(|_| "Pending config parse error".to_string())?;

        // 新钥探测：用密码 + pending config 派生新钥，尝试解密数据。
        if let Ok(new_key_arr) = self.derive_key_from_config(&pending_config, password) {
            if self.probe_data_key(account_id, &new_key_arr)? {
                // reencrypt 已提交 → promote：pending 内容写为 active config。
                tracing::info!("R-4①: promote pending config for {}", account_id);
                self.write_config_atomic(account_id, &pending_bytes)?;
                self.remove_config_pending(account_id);
                // 评审补强：同步更新生物识别凭证（promote 后数据=新钥，旧凭证陈旧会
                // 导致下一次生物识别解锁“打开成功但解密全失败”）与清除 PIN 凭证，
                // 镜像 change_password/unlock_with_kdf_upgrade 成功路径的尾部。
                self.refresh_credentials_after_promote(account_id, &new_key_arr);
                return Ok(());
            }
        }

        // 旧钥探测：用密码 + active config 派生旧钥，尝试解密数据。
        let (config, salt_arr, mk, master_key) =
            self.load_config_and_derive_master_key(account_id, password)?;
        let _ = (config, salt_arr, master_key);
        if self.probe_data_key(account_id, &mk)? {
            // reencrypt 未提交 → discard：删除 pending，数据保持旧钥。
            tracing::info!("R-4①: discard pending config for {}", account_id);
            self.remove_config_pending(account_id);
            return Ok(());
        }

        // 密码错误（或数据损坏）：保留 pending 供下次重试/人工恢复。
        // 附带 pending 存在提示，帮助用户在「改密后崩溃、新旧密码不确定」场景下判断
        // 应尝试哪一侧密码（数据侧密钥为准）。
        Err(
            "Invalid password (interrupted key rotation pending; try the other password)"
                .to_string(),
        )
    }

    /// R-4① 方案 2：用给定密钥探测 vault.db 数据可解性。
    /// 纯只读独立连接（solosoul_vault::probe_data_key），不触发 open 的迁移/
    /// 回填副作用，可安全用错误密钥调用。
    fn probe_data_key(&self, account_id: &str, key: &[u8; 32]) -> Result<bool, String> {
        let account_dir_path = self
            .fs
            .local_path(&self.account_dir_rel(account_id))
            .ok_or("无法解析账户本地目录")?;
        let db_path = account_dir_path.join("vault.db");
        solosoul_vault::probe_data_key(&db_path, &solosoul_vault::DataEncryptionKey::new(*key))
    }

    /// R-4① 方案 2：promote 后同步凭证（best-effort，失败仅记日志）。
    /// 镜像 change_password / unlock_with_kdf_upgrade 成功路径尾部：
    /// 生物识别凭证更新为新钥；PIN 凭证（旧钥）清除后由用户重新设置。
    fn refresh_credentials_after_promote(&self, account_id: &str, new_key: &[u8; 32]) {
        {
            let bio_manager = make_biometric_manager(self.base_path().clone());
            let new_key_hex = hex::encode(new_key.as_slice());
            if let Err(e) = bio_manager.update_credential(account_id, &new_key_hex) {
                tracing::warn!(
                    "Failed to update biometric credential after R-4① promote for {}: {}",
                    account_id,
                    e
                );
            }
        }
        {
            let pin_manager = PinManager::new(self.base_path().clone());
            if let Err(e) = pin_manager.clear_credential(account_id) {
                tracing::warn!(
                    "Failed to clear PIN credential after R-4① promote for {}: {}",
                    account_id,
                    e
                );
            }
        }
    }

    /// R-4① 方案 2：从给定的 AccountConfig 派生主密钥（[u8; 32]）。
    fn derive_key_from_config(
        &self,
        config: &AccountConfig,
        password: &str,
    ) -> Result<[u8; 32], String> {
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
        master_key
            .as_slice()
            .try_into()
            .map_err(|_| "Master key must be 32 bytes".to_string())
    }

    pub fn unlock_secure(
        &self,
        account_id: &str,
        password: &Zeroizing<String>,
    ) -> Result<(), String> {
        self.unlock(account_id, password.as_ref())
    }

    pub fn unlock(&self, account_id: &str, password: &str) -> Result<(), String> {
        // R-4① 方案 2：解锁入口先恢复未完成的 reencrypt→config 交换。
        // 常态（无 pending 文件）零开销；有 pending 时 promote/discard 后
        // 再走正常解锁路径（config 已恢复一致，verify 与数据密钥对齐）。
        self.recover_pending_reencrypt(account_id, password)?;

        // P032：主密码失败限流——锁定预检放在昂贵 KDF 之前。
        let pre_config = self.read_account_config(account_id)?;
        if let Some(ref until) = pre_config.password_locked_until {
            if let Ok(until_time) = chrono::DateTime::parse_from_rfc3339(until) {
                if chrono::Utc::now() < until_time {
                    return Err(MASTER_PASSWORD_LOCKED_ERR.to_string());
                }
            }
        }

        let (config, salt_arr, mk, master_key) =
            self.load_config_and_derive_master_key(account_id, password)?;
        // Backward compat: crypto_version < 3 uses old Argon2id verify hash
        let computed_hash =
            Self::compute_verify_hash(&config, &mk, &salt_arr, master_key.as_slice())?;

        if !secure_compare(computed_hash.as_bytes(), config.verify_hash.as_bytes()) {
            // P032：失败递增计数并持久化（阶梯锁定），随后返回原错误文案。
            self.record_password_failure(account_id)?;
            return Err("Invalid password".to_string());
        }

        // P032：验证成功归零失败计数/锁定（有残留时才写，避免每次登录多余 IO）。
        self.clear_password_failures(account_id, &config);

        // Update last accessed
        let _now = chrono::Utc::now().to_rfc3339();
        if let Ok(mut cache) = self.accounts_cache.write() {
            if let Some(entry) = cache.get_mut(account_id) {
                entry.last_accessed = Some(chrono::Utc::now().to_rfc3339());
            }
        }
        self.save_accounts().ok();

        // P003: 已有账户若使用低于生产档的 KDF 参数（开发档 8MiB/2iter 或平衡档
        // 16MiB/3iter），在 release 构建下解锁成功后透明升级到生产参数并重加密
        // 整个 Vault。debug 构建保持开发档以加速本地开发/测试。
        if !cfg!(debug_assertions) && config.kdf_config() != KdfConfig::production() {
            return self.unlock_with_kdf_upgrade(account_id, password, &master_key);
        }

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
        // 设备级偏好同步开关：unlock 新建 VaultStore 后应用期望值（默认 true）。
        vault_arc.set_ui_prefs_sync_enabled(self.ui_prefs_sync_enabled.load(Ordering::SeqCst));
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

    /// N-2：reencrypt→config 两阶段失败回滚。reencrypt_all 成功后若 config 写入失败，
    /// 账户会处于“数据已换新钥、config 仍记旧参数”的不可用态。本方法恢复旧 config，
    /// 并在调用方持有的同一 VaultStore 上切换内存密钥为新钥读回数据、再重加密回旧钥，
    /// 保持账户一致可用。
    ///
    /// R-4：返回 `Result<(), String>`——回滚自身失败（磁盘满等共同根因）必须上抛，
    /// 由调用方并入错误文案，不再以「已尝试自动回滚」掩盖回滚未生效的事实。
    fn rollback_reencrypt_and_config(
        &self,
        account_id: &str,
        vault: &VaultStore,
        old_config_content: &[u8],
        old_key: &solosoul_vault::DataEncryptionKey,
        new_key: &solosoul_vault::DataEncryptionKey,
    ) -> Result<(), String> {
        // 1) 恢复旧 config（盐/参数/verify_hash 与旧密钥一致）——原子写，
        //    避免回滚自身再次写坏 config。
        self.fs
            .write_file_atomic(&self.config_path_rel(account_id), old_config_content)
            .map_err(|e| format!("rollback: failed to restore config: {}", e))?;
        // 2) 数据重加密回旧密钥：同一 store 先切内存密钥为新钥读回，再以旧钥写回
        vault.set_data_key(new_key.clone());
        if let Err(e) = vault.reencrypt_all(new_key, old_key) {
            // 先恢复内存密钥再上抛，避免调用方在回滚失败后仍持有错误的内存密钥
            vault.set_data_key(old_key.clone());
            return Err(format!(
                "rollback: re-encrypt back to old key failed: {}",
                e
            ));
        }
        vault.set_data_key(old_key.clone());
        Ok(())
    }

    /// P003：将账户 KDF 参数透明升级到生产档并重加密整个 Vault。
    ///
    /// 仅在 `unlock` 成功验证密码后调用（release 构建、存储参数低于生产档时）。
    /// 流程与 `change_password` 一致：用旧密钥打开 Vault → `reencrypt_all` 重加密
    /// 全部数据 → 更新 config（新 salt / 新 verify hash / 生产参数）→ 用新密钥重开
    /// Vault → 同步更新生物识别凭证、清除 PIN 凭证（其保存的旧密钥已失效）。
    fn unlock_with_kdf_upgrade(
        &self,
        account_id: &str,
        password: &str,
        old_master_key: &Zeroizing<Vec<u8>>,
    ) -> Result<(), String> {
        // 旧密钥（已验证通过）。
        let old_key_arr: [u8; 32] = old_master_key
            .as_slice()
            .try_into()
            .map_err(|_| "Master key must be 32 bytes".to_string())?;
        let old_key = solosoul_vault::DataEncryptionKey::new(old_key_arr);

        // 新 salt + 生产参数派生新主密钥。
        let salt = generate_salt();
        let new_kdf_config = KdfConfig::production();
        let new_key = derive_key(password, &salt, &new_kdf_config)
            .map_err(|e| format!("New key derivation failed: {}", e))?;
        let new_key_arr: [u8; 32] = new_key
            .as_slice()
            .try_into()
            .map_err(|_| "Key derivation output must be 32 bytes".to_string())?;
        let new_key_enc = solosoul_vault::DataEncryptionKey::new(new_key_arr);

        // N-2：备份旧 config——reencrypt 成功后若 config 写入失败，恢复旧 config 并
        // 把数据重加密回旧密钥，避免“数据已换新钥、config 仍记旧参数”的账户不可用态。
        let config_rel = self.config_path_rel(account_id);
        let old_config_content = self
            .fs
            .read_file(&config_rel)
            .map_err(|_| "Account not found".to_string())?;

        // 更新 config：新 salt、新 verify hash、生产参数。
        let content = String::from_utf8(old_config_content.clone())
            .map_err(|_| "Config encoding error".to_string())?;
        let mut config: AccountConfig =
            serde_json::from_str(&content).map_err(|_| "Config parse error".to_string())?;
        config.crypto_version = 3; // P2-010: HKDF-based verify hash
        config.salt =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, salt.as_slice());
        let mk: [u8; 32] = new_key_arr;
        config.verify_hash = hex::encode(
            solosoul_crypto::hkdf_ext::derive_hkdf_key(&mk, &salt, b"SOLOSOUL_VAULT_VERIFY_v1")
                .map_err(|e| format!("Verify HKDF failed: {}", e))?,
        );
        config.kdf_memory_kb = Some(new_kdf_config.memory_kb);
        config.kdf_iterations = Some(new_kdf_config.iterations);
        config.kdf_parallelism = Some(new_kdf_config.parallelism);
        let config_json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;

        // R-4① 方案 2：reencrypt 前先把新 config 原子写入 pending 载体。
        self.write_config_pending(account_id, config_json.as_bytes())?;

        // 用旧密钥打开 Vault 并重加密全部数据。
        // N-2：reencrypt_all 事务内全有或全无（任一行失败整体回滚，数据保持旧密钥）。
        let account_dir_path = self
            .fs
            .local_path(&self.account_dir_rel(account_id))
            .ok_or("无法解析账户本地目录")?;
        let vault_config =
            VaultConfig::new(account_id, account_dir_path).with_data_key(old_key_arr);
        let vault = VaultStore::open(vault_config)
            .map_err(|e| format!("Failed to open vault for KDF upgrade: {}", e))?;
        if let Err(e) = vault.reencrypt_all(&old_key, &new_key_enc) {
            // reencrypt 失败（事务回滚，数据仍为旧钥）：pending 无意义，清除后上抛。
            self.remove_config_pending(account_id);
            return Err(format!("KDF upgrade re-encryption failed: {}", e));
        }

        // P135: 原子写——config 更新为关键写入路径。
        if let Err(e) = self.write_config_atomic(account_id, config_json.as_bytes()) {
            // N-2：config 写入失败 → 回滚（恢复旧 config + 数据重加密回旧密钥），
            // 保持账户一致可用，并把失败原因上抛。
            // R-4：回滚自身失败（磁盘满等共同根因）必须并入上抛文案。
            let rollback_note = match self.rollback_reencrypt_and_config(
                account_id,
                &vault,
                &old_config_content,
                &old_key,
                &new_key_enc,
            ) {
                Ok(_) => {
                    // 回滚成功：数据已重加密回旧钥、config 已恢复 → pending 同步清除。
                    self.remove_config_pending(account_id);
                    "an automatic rollback to the previous key was attempted.".to_string()
                }
                Err(rb) => {
                    // 回滚失败：保留 pending——数据可能仍为新钥，下次解锁经
                    // recover_pending_reencrypt promote 可完成交换（恢复线索）。
                    format!("automatic rollback FAILED: {}", rb)
                }
            };
            return Err(format!(
                "KDF upgrade failed to update config: {}; {}",
                e, rollback_note
            ));
        }
        // R-4① 方案 2：config 写入成功 → 交换完成，清除 pending。
        self.remove_config_pending(account_id);
        // 释放临时 Vault 连接，随后以新密钥重开（避免同一 DB 双连接）
        drop(vault);

        // 更新会话密钥并重开 Vault（新密钥）。
        {
            if let Ok(mut key) = self.session_key.write() {
                *key = Some(Zeroizing::new(new_key_arr));
            }
        }
        if let Ok(mut ua) = self.unlocked_account.write() {
            *ua = Some(account_id.to_string());
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
                let vault_arc = Arc::new(vault);
                // 设备级偏好同步开关：新建 VaultStore 后应用期望值（默认 true）。
                vault_arc
                    .set_ui_prefs_sync_enabled(self.ui_prefs_sync_enabled.load(Ordering::SeqCst));
                if let Ok(mut store) = self.vault_store.write() {
                    *store = Some(vault_arc);
                }
            }
            Err(e) => {
                return Err(format!(
                    "KDF upgrade succeeded but vault reopen failed: {}",
                    e
                ));
            }
        }

        // 此路径不再单独调用 pin_manager.reset_attempts：clear_credential 已
        // 将 pin_failed_attempts 归零并清除 pin_locked_until，与 unlock 尾部的
        // reset_attempts 效果等价。

        // 生物识别凭证保存的是旧主密钥，需同步更新。
        {
            let bio_manager = make_biometric_manager(self.base_path().clone());
            let new_key_hex = hex::encode(new_key_arr.as_slice());
            if let Err(e) = bio_manager.update_credential(account_id, &new_key_hex) {
                tracing::warn!(
                    "Failed to update biometric credential after KDF upgrade for {}: {}",
                    account_id,
                    e
                );
            }
        }

        // PIN 凭证保存的是旧主密钥，且重加密需要 PIN 输入（不可用），清除后由用户重新设置。
        {
            let pin_manager = PinManager::new(self.base_path().clone());
            if let Err(e) = pin_manager.clear_credential(account_id) {
                tracing::warn!(
                    "Failed to clear PIN credential after KDF upgrade for {}: {}",
                    account_id,
                    e
                );
            }
        }

        // SAF 远端存储：本地 vault.db 已用新密钥重加密，同步到远端避免
        // 下次 sync_from_remote 用旧副本覆盖。
        if self.is_remote_storage() {
            if let Err(e) = self.sync_to_remote() {
                tracing::error!(
                    "Failed to sync re-encrypted vault.db to SAF after KDF upgrade for {}: {}",
                    account_id,
                    e
                );
                return Err(format!(
                    "KDF upgrade failed to sync encrypted data to remote storage: {}",
                    e
                ));
            }
        }

        tracing::info!("KDF params upgraded to production for {}", account_id);
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
    /// Does NOT unlock (no session key storage).
    /// R-4① 方案 2：入口会先恢复未完成的 reencrypt→config 交换（promote 时原子改写
    /// config.json / 删除 pending）——这是崩溃恢复所需的修复性副作用，非解锁状态变更。
    /// Verify hash is derived from the Argon2id master key using HKDF-SHA256 (P2-010).
    pub fn verify_password(&self, account_id: &str, password: &str) -> Result<bool, String> {
        // R-4① 方案 2：verify 入口同样先恢复 pending（reencrypt→config 交换），
        // 保证密码校验基于一致的 config；常态零开销。
        self.recover_pending_reencrypt(account_id, password)?;
        let (config, salt_arr, mk, master_key) =
            self.load_config_and_derive_master_key(account_id, password)?;
        // Backward compat: crypto_version < 3 uses old Argon2id verify hash
        let computed_hash =
            Self::compute_verify_hash(&config, &mk, &salt_arr, master_key.as_slice())?;

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
        // R-4① 方案 2：存在未完成的 reencrypt→config 交换时，会话密钥（生物识别/
        // PIN）可能是旧钥而数据已是新钥——需密码派生密钥才能恢复，这里显式拒绝
        // 并引导走密码解锁（recover_pending_reencrypt 会完成交换）。
        let pending_rel = self.config_pending_path_rel(account_id);
        if self.fs.exists(&pending_rel).map_err(|e| e.to_string())? {
            return Err(
                "Pending key rotation detected; please unlock with your password".to_string(),
            );
        }

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
        // 设备级偏好同步开关：unlock 新建 VaultStore 后应用期望值（默认 true）。
        vault_arc.set_ui_prefs_sync_enabled(self.ui_prefs_sync_enabled.load(Ordering::SeqCst));
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

        // N-2：在 reencrypt 之前读取并解析旧 config（备份 + 校验）。任何读取/解析失败
        // 都发生在数据改动之前——若失败直接返回，杜绝"数据已换新钥、config 仍记旧参数"
        // 的混态（旧实现把读取放在 reencrypt 之后，读取失败会留下混态）。
        let config_rel = self.config_path_rel(account_id);
        let old_config_content = self
            .fs
            .read_file(&config_rel)
            .map_err(|_| "Account not found".to_string())?;
        let content = String::from_utf8(old_config_content.clone())
            .map_err(|_| "Config encoding error".to_string())?;
        let old_config: AccountConfig =
            serde_json::from_str(&content).map_err(|_| "Config parse error".to_string())?;

        // Derive new verify hash via HKDF (P2-010).
        let mk: [u8; 32] = new_key_arr; // already 32 bytes from try_into above
        let verify_hash = hex::encode(
            solosoul_crypto::hkdf_ext::derive_hkdf_key(&mk, &salt, b"SOLOSOUL_VAULT_VERIFY_v1")
                .map_err(|e| format!("Verify HKDF failed: {}", e))?,
        );

        // 基于 reencrypt 前解析的旧 config 派生新 config。
        let mut config = old_config;
        config.crypto_version = 3; // P2-010: HKDF-based verify hash
        config.salt =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, salt.as_slice());
        config.verify_hash = verify_hash;
        config.kdf_memory_kb = Some(new_kdf_config.memory_kb);
        config.kdf_iterations = Some(new_kdf_config.iterations);
        config.kdf_parallelism = Some(new_kdf_config.parallelism);
        let config_json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;

        // R-4① 方案 2：reencrypt 前先把新 config 原子写入 pending 载体——
        // 崩溃后残留的 pending 由 unlock/verify 入口 recover_pending_reencrypt
        // 用 probe 判定 reencrypt 是否已提交：已提交 → promote（pending 升为
        // active），未提交 → discard（删 pending）。
        self.write_config_pending(account_id, config_json.as_bytes())?;

        // Re-encrypt all sensitive data with the new key（reencrypt_all 事务内全有或全无）。
        {
            let vault_guard = self
                .get_vault_store()
                .ok_or("Vault not available for re-encryption")?;
            let vault = vault_guard.as_ref();
            if let Err(e) = vault.reencrypt_all(&old_key, &new_key_enc) {
                // reencrypt 失败（事务回滚，数据仍为旧钥）：pending 无意义，清除后上抛。
                self.remove_config_pending(account_id);
                return Err(format!("Re-encryption failed: {}", e));
            }
        }

        // P135: 原子写——config 更新为关键写入路径。
        if let Err(e) = self.write_config_atomic(account_id, config_json.as_bytes()) {
            // N-2：config 写入失败 → 回滚（恢复旧 config + 数据重加密回旧密钥），
            // 避免账户不可用；会话密钥尚未切换，回滚后当前会话仍以旧密钥工作。
            // R-4：回滚自身失败（磁盘满等共同根因）必须并入上抛文案。
            let rollback_note = if let Some(vault_guard) = self.get_vault_store() {
                match self.rollback_reencrypt_and_config(
                    account_id,
                    vault_guard.as_ref(),
                    &old_config_content,
                    &old_key,
                    &new_key_enc,
                ) {
                    Ok(_) => {
                        // 回滚成功：数据已重加密回旧钥、config 已恢复 → pending 同步清除。
                        self.remove_config_pending(account_id);
                        "an automatic rollback to the previous key was attempted.".to_string()
                    }
                    Err(rb) => {
                        // 回滚失败：保留 pending——数据可能仍为新钥，下次解锁经
                        // recover_pending_reencrypt promote 可完成交换（恢复线索）。
                        format!("automatic rollback FAILED: {}", rb)
                    }
                }
            } else {
                "automatic rollback skipped (vault unavailable)".to_string()
            };
            return Err(format!(
                "Password updated but config write failed: {}; {}",
                e, rollback_note
            ));
        }

        // R-4① 方案 2：config 写入成功 → 交换完成，清除 pending。
        self.remove_config_pending(account_id);

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
                let vault_arc = Arc::new(vault);
                // 设备级偏好同步开关：新建 VaultStore 后应用期望值（默认 true）。
                vault_arc
                    .set_ui_prefs_sync_enabled(self.ui_prefs_sync_enabled.load(Ordering::SeqCst));
                if let Ok(mut store) = self.vault_store.write() {
                    *store = Some(vault_arc);
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

        // 关键修复：reencrypt_all 已将 vault.db 用新密钥重新加密到本地临时目录，
        // 但在 Android SAF 模式下，本地临时目录与远端 SAF 存储是分离的。
        // 若不主动 sync_to_remote，重新登录时 sync_from_remote 会用旧的 SAF 副本
        // （仍用旧密钥加密）覆盖本地 vault.db，导致所有解密失败（object not found /
        // audit details decryption failed）。
        if self.is_remote_storage() {
            if let Err(e) = self.sync_to_remote() {
                tracing::error!(
                    "Failed to sync re-encrypted vault.db to SAF after password change for {}: {}. \
                     The local DB has been re-encrypted but the remote copy is stale. \
                     Do NOT restart the app — that would overwrite the good local copy via sync_from_remote.",
                    account_id,
                    e
                );
                return Err(format!(
                    "Password updated but failed to sync encrypted data to remote storage: {}. \
                     The local database is correct but the remote copy is stale. \
                     Please retry syncing from Settings — do NOT restart the app before syncing, \
                     as that would overwrite the local data with the stale remote copy.",
                    e
                ));
            }
            tracing::info!(
                "Successfully synced re-encrypted vault.db to SAF after password change for {}",
                account_id
            );
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

    /// 重置账户的安全标志（生物识别/PIN 已启用状态）到关闭状态。
    ///
    /// 用于重装后选择已有外部目录并登录的场景：旧 config.json 中可能残留了前一次安装
    /// 的 biometric_enabled/pin_enabled=true，但实际 KeyStore 凭证与 PIN 文件已被卸载清除。
    /// 此方法将这些标志复位，避免用户进入安全设置后看到"已启用"但实际无法使用的状态。
    ///
    /// 同时清理 `keystore_data.json`、`biometric_key`、`pin_credential` 等凭证残留文件。
    /// 注意：PIN 凭证实际文件名为 `pin_credential`（无扩展名，见 PinManager），
    /// 旧的 `pin_*.cred` 后缀模式不匹配该文件，会残留凭证导致重装后误显示 PIN 解锁。
    pub fn reset_security_flags(&self, account_id: &str) -> Result<(), String> {
        let config_rel = self.config_path_rel(account_id);
        let content = self
            .fs
            .read_file(&config_rel)
            .map_err(|_| "Account not found".to_string())?;
        let content =
            String::from_utf8(content).map_err(|_| "Config encoding error".to_string())?;
        let mut config: AccountConfig =
            serde_json::from_str(&content).map_err(|_| "Config parse error".to_string())?;

        config.biometric_enabled = false;
        config.pin_enabled = false;
        config.pin_length = 0;
        config.pin_failed_attempts = 0;
        config.pin_locked_until = None;

        let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
        // P135: 原子写——安全标志复位同样避免写坏 config。
        self.write_config_atomic(account_id, json.as_bytes())?;

        // 清理可能残留的凭证文件
        let dir_rel = self.account_dir_rel(account_id);
        // keystore_data.json（Android 双槽凭证）
        let keystore_path_rel = format!("{dir_rel}/keystore_data.json");
        if self.fs.exists(&keystore_path_rel).unwrap_or(false) {
            let _ = self.fs.remove_file(&keystore_path_rel);
        }
        // legacy biometric_key 文件
        let bio_key_rel = format!("{dir_rel}/biometric_key");
        if self.fs.exists(&bio_key_rel).unwrap_or(false) {
            let _ = self.fs.remove_file(&bio_key_rel);
        }
        // PIN 凭证文件：精确删除 pin_credential（无扩展名，PinManager 实际文件名），
        // 同时兼容历史遗留的 pin_*.cred 命名，通过本地路径枚举删除。
        if let Some(local_dir) = self.fs.local_path(&dir_rel) {
            if let Ok(entries) = std::fs::read_dir(&local_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if name_str == "pin_credential"
                        || (name_str.starts_with("pin_") && name_str.ends_with(".cred"))
                    {
                        let _ = std::fs::remove_file(entry.path());
                    }
                }
            }
        }
        // SAF/外部目录下 VaultFileSystem::remove_file 可能静默失败，
        // 再用本地 std::fs 路径兜底删除一次关键凭证文件。
        if let Some(local_dir) = self.fs.local_path(&dir_rel) {
            let keystore_local = local_dir.join("keystore_data.json");
            if keystore_local.exists() {
                if let Err(e) = std::fs::remove_file(&keystore_local) {
                    tracing::warn!(
                        "reset_security_flags: failed to remove keystore_data.json at {:?}: {}",
                        keystore_local,
                        e
                    );
                }
            }
            let bio_key_local = local_dir.join("biometric_key");
            if bio_key_local.exists() {
                if let Err(e) = std::fs::remove_file(&bio_key_local) {
                    tracing::warn!(
                        "reset_security_flags: failed to remove biometric_key at {:?}: {}",
                        bio_key_local,
                        e
                    );
                }
            }
        }

        tracing::info!(
            "已重置账户 {} 的安全标志（biometric/pin 已关闭，残留凭证已清理）",
            account_id
        );
        Ok(())
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
        // P135: 原子写——密码提示更新同样避免写坏 config。
        self.write_config_atomic(account_id, config_json.as_bytes())?;
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
    use std::sync::atomic::{AtomicBool, Ordering};
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
    fn test_create_account_with_id_same_name_different_id_succeeds() {
        let (svc, _dir) = setup_service();
        // 正常创建账户 A（account_id 随机生成）
        let account_a = svc.create_account("Zzc", "password123", None).unwrap();
        let account_a_id = account_a["id"].as_str().unwrap().to_string();

        // 恢复场景：同名（大小写不同）但 account_id 不同 → 允许（身份是 account_id）
        let result =
            svc.create_account_with_id("acc_restore_same_name", "zzc", "password456", None);
        assert!(
            result.is_ok(),
            "同名校验不应对恢复场景生效: {:?}",
            result.err()
        );
        let account_b = result.unwrap();
        assert_eq!(account_b["name"], "zzc");
        assert_eq!(account_b["id"], "acc_restore_same_name");

        // 两个同名账户均可列出（登录页按 account_id 区分）
        let accounts = svc.list_accounts();
        assert_eq!(accounts.len(), 2);
        assert!(accounts.iter().any(|a| a.id == account_a_id));
        assert!(accounts.iter().any(|a| a.id == "acc_restore_same_name"));
    }

    #[test]
    fn test_create_account_with_id_duplicate_id_fails() {
        let (svc, _dir) = setup_service();
        let account = svc.create_account("Zzc", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap().to_string();

        // 相同 account_id → 拒绝，错误字符串稳定供前端识别冲突
        let result = svc.create_account_with_id(&account_id, "Zzc", "password456", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Account ID already exists"));
    }

    #[test]
    fn test_unlock_and_lock() {
        let (svc, _dir) = setup_service();
        let account = svc.create_account("Bob", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap();

        // create_account leaves vault unlocked
        assert!(svc.is_unlocked());
        svc.lock();
        assert!(!svc.is_unlocked());

        svc.unlock(account_id, "password123").unwrap();
        assert!(svc.is_unlocked());

        svc.lock();
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
    fn test_password_lockout_seconds() {
        assert_eq!(password_lockout_seconds(0), 0);
        assert_eq!(password_lockout_seconds(4), 0);
        assert_eq!(password_lockout_seconds(5), 30);
        assert_eq!(password_lockout_seconds(9), 30);
        assert_eq!(password_lockout_seconds(10), 300);
        assert_eq!(password_lockout_seconds(11), 600);
        assert_eq!(password_lockout_seconds(15), 1800);
    }

    #[test]
    fn test_unlock_rate_limit_locks_after_5_failures() {
        let (svc, _dir) = setup_service();
        let account = svc.create_account("Dave", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap().to_string();

        // 前 4 次失败：不锁定，返回 Invalid password
        for _ in 0..4 {
            let err = svc.unlock(&account_id, "wrong").unwrap_err();
            assert!(err.contains("Invalid password"));
        }
        // 第 5 次失败：触发 30s 阶梯锁定
        let err = svc.unlock(&account_id, "wrong").unwrap_err();
        assert!(err.contains("Invalid password"));
        // 锁定期间即使密码正确也被拒绝，且文案稳定（不递增计数）
        let err = svc.unlock(&account_id, "password123").unwrap_err();
        assert!(err.contains("Too many failed attempts"));
        let err = svc.unlock(&account_id, "wrong").unwrap_err();
        assert!(err.contains("Too many failed attempts"));
    }

    #[test]
    fn test_unlock_rate_limit_resets_on_success() {
        let (svc, _dir) = setup_service();
        let account = svc.create_account("Eve", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap().to_string();

        // 4 次失败（未触发锁定）
        for _ in 0..4 {
            assert!(svc.unlock(&account_id, "wrong").is_err());
        }
        // 成功解锁 → 计数归零
        assert!(svc.unlock(&account_id, "password123").is_ok());
        // 计数已从 0 重新累计：再失败 5 次 → 第 5 次触发锁定
        for _ in 0..5 {
            let err = svc.unlock(&account_id, "wrong").unwrap_err();
            assert!(err.contains("Invalid password"));
        }
        let err = svc.unlock(&account_id, "password123").unwrap_err();
        assert!(err.contains("Too many failed attempts"));
    }

    #[test]
    fn test_unlock_rate_limit_expires_and_clears() {
        let (svc, _dir) = setup_service();
        let account = svc.create_account("Frank", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap().to_string();

        // 触发锁定
        for _ in 0..5 {
            assert!(svc.unlock(&account_id, "wrong").is_err());
        }
        assert!(svc.unlock(&account_id, "password123").is_err());

        // 手动把锁定截止时间改为过去（模拟锁定到期）
        let config_path = svc.base_path().join(&account_id).join("config.json");
        let mut value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        value["passwordLockedUntil"] = serde_json::Value::String(
            (chrono::Utc::now() - chrono::Duration::seconds(1)).to_rfc3339(),
        );
        std::fs::write(&config_path, serde_json::to_string_pretty(&value).unwrap()).unwrap();

        // 到期后正确密码解锁成功，且失败计数/锁定被归零
        assert!(svc.unlock(&account_id, "password123").is_ok());
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(value["passwordFailedAttempts"], 0);
        assert!(value["passwordLockedUntil"].as_str().is_none());
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

    // ── R-4: 回滚失败必须并入上抛文案（而非「已尝试自动回滚」掩盖）──────────
    //
    // N-2 残余：rollback_reencrypt_and_config 失败时仅记日志，调用方文案仍写
    // 「已尝试自动回滚」——磁盘满等共同根因下 config 可能被截断且用户无感知。
    // 本测试用 toggleable mock fs：仅对 config.json 写入注入失败 → 改密的 config
    // 写入失败触发回滚 → 回滚的 config 恢复同样失败 → 错误文案必须明示
    // 「automatic rollback FAILED」并带底层原因（修复前失败仅记日志，文案无此信息）。
    struct FailConfigWriteFs {
        inner: LocalVaultFileSystem,
        fail_config_writes: Arc<AtomicBool>,
    }

    impl VaultFileSystem for FailConfigWriteFs {
        fn read_file(&self, relative_path: &str) -> Result<Vec<u8>, String> {
            self.inner.read_file(relative_path)
        }
        fn write_file(&self, relative_path: &str, data: &[u8]) -> Result<(), String> {
            if self.fail_config_writes.load(Ordering::SeqCst)
                && relative_path.ends_with("config.json")
            {
                return Err("mock config write failure (injected)".to_string());
            }
            self.inner.write_file(relative_path, data)
        }
        // P135: config 写入已切换为原子写路径——mock 需同样注入才能触发回滚。
        fn write_file_atomic(&self, relative_path: &str, data: &[u8]) -> Result<(), String> {
            if self.fail_config_writes.load(Ordering::SeqCst)
                && relative_path.ends_with("config.json")
            {
                return Err("mock config write failure (injected)".to_string());
            }
            self.inner.write_file_atomic(relative_path, data)
        }
        fn remove_file(&self, relative_path: &str) -> Result<(), String> {
            self.inner.remove_file(relative_path)
        }
        fn exists(&self, relative_path: &str) -> Result<bool, String> {
            self.inner.exists(relative_path)
        }
        fn create_dir_all(&self, relative_path: &str) -> Result<(), String> {
            self.inner.create_dir_all(relative_path)
        }
        fn remove_dir_all(&self, relative_path: &str) -> Result<(), String> {
            self.inner.remove_dir_all(relative_path)
        }
        fn list_dir(&self, relative_path: &str) -> Result<Vec<String>, String> {
            self.inner.list_dir(relative_path)
        }
        fn local_path(&self, relative_path: &str) -> Option<PathBuf> {
            self.inner.local_path(relative_path)
        }
    }

    #[test]
    fn test_change_password_rollback_failure_surfaced() {
        let dir = TempDir::new().unwrap();
        let base = dir.path().join(".solosoul");
        std::fs::create_dir_all(&base).unwrap();
        let fail_flag = Arc::new(AtomicBool::new(false));
        let mock = Arc::new(FailConfigWriteFs {
            inner: LocalVaultFileSystem::new(base.clone()),
            fail_config_writes: fail_flag.clone(),
        });
        let svc = VaultService::with_file_system(base, mock);

        let account = svc.create_account("R4", "oldpassword", None).unwrap();
        let account_id = account["id"].as_str().unwrap();
        svc.unlock(account_id, "oldpassword").unwrap();

        // 注入失败：此后对 config.json 的写（改密写入 + 回滚恢复）都失败。
        // change_password 内部 unlock 只写 accounts.json，不受影响。
        fail_flag.store(true, Ordering::SeqCst);

        let err = svc
            .change_password(account_id, "oldpassword", "newpassword")
            .unwrap_err();
        // R-4：错误文案必须明示回滚失败，而非「已尝试自动回滚」
        assert!(
            err.contains("automatic rollback FAILED"),
            "错误文案必须明示回滚失败，实际: {}",
            err
        );
        assert!(
            err.contains("mock config write failure"),
            "错误文案须带底层原因，实际: {}",
            err
        );
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
    fn test_has_account() {
        let (svc, _dir) = setup_service();
        // 无账户时返回 false
        assert!(!svc.has_account("acc_nonexistent"));

        // 创建账户后可按 account_id 命中（纯内存查询，无文件 IO）
        let account = svc.create_account("Helen", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap().to_string();
        assert!(svc.has_account(&account_id));
        assert!(!svc.has_account("acc_other"));

        // 删除后不再命中
        svc.delete_account(&account_id).unwrap();
        assert!(!svc.has_account(&account_id));
    }

    #[test]
    fn test_unlock_with_session_key() {
        let (svc, _dir) = setup_service();
        let account = svc.create_account("Hank", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap();

        let session_key = [0u8; 32];
        svc.unlock_with_session_key(account_id, &session_key)
            .unwrap();
        assert!(svc.is_unlocked());
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

    /// P003：`unlock_with_kdf_upgrade` 应将存储的开发档 KDF 参数透明升级到生产档，
    /// 重加密全部数据（写入一条审计日志后升级，仍可解密读出），并更新 config 中的
    /// verify hash 与参数。
    #[test]
    fn test_unlock_with_kdf_upgrade_reencrypts_and_upgrades_params() {
        let (svc, _dir) = setup_service();
        let account = svc
            .create_account("KdfUpgrade", "password123", None)
            .unwrap();
        let account_id = account["id"].as_str().unwrap();

        // 写入一条审计日志（加密数据），用于验证升级后仍可解密。
        {
            let vault = svc.get_vault_store().expect("vault open after create");
            vault
                .log_structured(
                    "test_kdf_upgrade",
                    "test",
                    Some(account_id),
                    None,
                    "user",
                    Some("before-upgrade"),
                )
                .unwrap();
        }

        // 模拟旧账户：将 config 的 KDF 参数改为开发档（8 MiB / 2 iter）。
        let config_path = svc.base_path().join(account_id).join("config.json");
        let mut raw: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
        raw["kdfMemoryKb"] = serde_json::Value::from(8 * 1024u32);
        raw["kdfIterations"] = serde_json::Value::from(2u32);
        raw["kdfParallelism"] = serde_json::Value::from(4u32);
        fs::write(&config_path, serde_json::to_string_pretty(&raw).unwrap()).unwrap();

        // 用旧（开发档）参数派生旧密钥并执行透明升级。
        let old_kdf = KdfConfig::development();
        let salt_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            raw["salt"].as_str().unwrap(),
        )
        .unwrap();
        let salt_arr: [u8; 16] = salt_bytes.as_slice().try_into().unwrap();
        let old_key = derive_key("password123", &salt_arr, &old_kdf).unwrap();
        svc.unlock_with_kdf_upgrade(account_id, "password123", &old_key)
            .unwrap();

        // config 应已升级为生产参数。
        let content = fs::read_to_string(&config_path).unwrap();
        let config: AccountConfig = serde_json::from_str(&content).unwrap();
        assert_eq!(config.kdf_config(), KdfConfig::production());

        // 旧参数派生的密钥应不再匹配 verify hash（新密钥来自生产参数）。
        let old_verify = derive_key("password123", &salt_arr, &old_kdf).unwrap();
        assert_ne!(config.verify_hash, hex::encode(old_verify.as_slice()));

        // 用新会话密钥（生产参数）打开 Vault 后，升级前的加密数据仍可解密。
        let vault = svc.get_vault_store().expect("vault open after upgrade");
        let logs = vault.list_audit_log(10).unwrap();
        assert!(!logs.is_empty());
        assert!(logs
            .iter()
            .any(|l| l.details.as_deref() == Some("before-upgrade")));
    }

    // ── P135: 原子写 + 崩溃恢复端到端 ──────────────────────────

    #[test]
    fn test_unlock_recovers_orphan_config_tmp() {
        // R-4① 崩溃故事：reencrypt 已提交、config 写入中进程崩溃——
        // 孤儿 .tmp（新 config）存在、主 config.json 缺失。
        // 下次 unlock 经 read_config_with_recovery → recover_or_load 提升 .tmp。
        let (svc, _dir) = setup_service();
        let account = svc.create_account("Orphan", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap();

        // 模拟崩溃：删主 config.json，仅留孤儿 .tmp（内容=原 config）。
        let config_path = svc.base_path().join(account_id).join("config.json");
        let content = fs::read(&config_path).unwrap();
        fs::write(config_path.with_extension("tmp"), &content).unwrap();
        fs::remove_file(&config_path).unwrap();

        // unlock 应经恢复路径成功（而非 "Account not found"）。
        svc.unlock(account_id, "password123").unwrap();
        assert!(config_path.exists(), "orphan .tmp 应被提升回主 config.json");
        assert!(
            !config_path.with_extension("tmp").exists(),
            "提升后孤儿 .tmp 应被清除"
        );
        assert!(svc.verify_password(account_id, "password123").unwrap());
    }

    #[test]
    fn test_unlock_recovers_backup_bak() {
        // 主文件损坏（非 JSON）但 .bak 完好 → recover_or_load 回退 .bak。
        let (svc, _dir) = setup_service();
        let account = svc.create_account("BakUser", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap();

        let config_path = svc.base_path().join(account_id).join("config.json");
        let content = fs::read(&config_path).unwrap();
        fs::write(config_path.with_extension("bak"), &content).unwrap();
        fs::write(&config_path, b"{corrupted json").unwrap();

        svc.unlock(account_id, "password123").unwrap();
        assert!(svc.verify_password(account_id, "password123").unwrap());
    }

    // ── R-4① 方案 2：两阶段 config 交换（config.json.pending）崩溃恢复 ──────────
    //
    // 模拟工具：构造「reencrypt 已提交、active config 未更新」的崩溃后状态——
    // 数据用新钥重加密 + pending 文件残留 + active config 仍为旧。

    /// 派生并返回（new_salt, new_key_arr）。
    fn make_new_key(password: &str) -> ([u8; 16], [u8; 32]) {
        let salt = generate_salt();
        let kdf = KdfConfig::from_env();
        let new_key = derive_key(password, &salt, &kdf).unwrap();
        let arr: [u8; 32] = new_key.as_slice().try_into().unwrap();
        (salt, arr)
    }

    /// 构造 pending config 内容：基于当前 active config，仅替换 salt/verify_hash 为新钥对应值。
    fn build_pending_config_json(
        svc: &VaultService,
        account_id: &str,
        new_salt: &[u8; 16],
        new_key: &[u8; 32],
    ) -> String {
        let config_path = svc.base_path().join(account_id).join("config.json");
        let mut raw: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
        raw["salt"] = serde_json::Value::String(base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            new_salt.as_slice(),
        ));
        let verify_hash = hex::encode(
            solosoul_crypto::hkdf_ext::derive_hkdf_key(
                new_key,
                new_salt,
                b"SOLOSOUL_VAULT_VERIFY_v1",
            )
            .unwrap(),
        );
        raw["verify_hash"] = serde_json::Value::String(verify_hash);
        serde_json::to_string_pretty(&raw).unwrap()
    }

    #[test]
    fn test_recover_pending_promotes_when_reencrypt_committed() {
        // 崩溃点：reencrypt 已提交（数据=新钥），active config 未更新（仍旧），
        // pending 文件残留。下次用新密码 unlock 应 promote：pending 升为 active。
        let (svc, _dir) = setup_service();
        let account = svc.create_account("Promote", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap();

        // 写一条非空 profile，保证 probe 能区分新旧钥。
        {
            let vault = svc.get_vault_store().unwrap();
            vault
                .save_profile(&solosoul_vault::Profile::new_with_id(
                    account_id,
                    "p",
                    b"sensitive".to_vec(),
                ))
                .unwrap();
        }

        // 模拟：旧钥解锁 → 数据重加密到新钥（reencrypt 提交）→ 写 pending（新 config）→
        // 不更新 active config（模拟崩溃残留）。
        svc.unlock(account_id, "password123").unwrap();
        let old_key_arr = *svc.get_session_key().unwrap();
        let old_key = solosoul_vault::DataEncryptionKey::new(old_key_arr);
        let (new_salt, new_key_arr) = make_new_key("newpassword123");
        let new_key = solosoul_vault::DataEncryptionKey::new(new_key_arr);
        let pending_json = build_pending_config_json(&svc, account_id, &new_salt, &new_key_arr);
        svc.write_config_pending(account_id, pending_json.as_bytes())
            .unwrap();
        svc.get_vault_store()
            .unwrap()
            .reencrypt_all(&old_key, &new_key)
            .unwrap();
        let pending_path = svc.base_path().join(account_id).join("config.json.pending");
        assert!(pending_path.exists(), "pending 应存在");
        svc.lock();

        // 用新密码解锁 → 恢复应 promote（pending 升为 active），解锁成功。
        svc.unlock(account_id, "newpassword123").unwrap();
        assert!(!pending_path.exists(), "promote 后 pending 应被删除");
        assert!(svc.verify_password(account_id, "newpassword123").unwrap());
        assert!(!svc.verify_password(account_id, "password123").unwrap());
        // 数据以新钥可解密。
        let logs_vault = svc.get_vault_store().unwrap();
        let profile = logs_vault.load_profile(account_id).unwrap();
        assert!(profile.is_some(), "promote 后数据应以新钥可读");
    }

    #[test]
    fn test_recover_pending_discards_when_reencrypt_not_committed() {
        // 崩溃点：pending 已写但 reencrypt 未提交（数据仍=旧钥）。
        // 下次用旧密码 unlock 应 discard：删 pending，保持旧钥。
        let (svc, _dir) = setup_service();
        let account = svc.create_account("Discard", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap();
        {
            let vault = svc.get_vault_store().unwrap();
            vault
                .save_profile(&solosoul_vault::Profile::new_with_id(
                    account_id,
                    "p",
                    b"sensitive".to_vec(),
                ))
                .unwrap();
        }

        svc.unlock(account_id, "password123").unwrap();
        let (new_salt, new_key_arr) = make_new_key("newpassword123");
        let pending_json = build_pending_config_json(&svc, account_id, &new_salt, &new_key_arr);
        svc.write_config_pending(account_id, pending_json.as_bytes())
            .unwrap();
        // 注意：不执行 reencrypt_all——数据保持旧钥。
        let pending_path = svc.base_path().join(account_id).join("config.json.pending");
        assert!(pending_path.exists());
        svc.lock();

        // 用旧密码解锁 → 恢复应 discard（pending 删除），解锁成功、数据仍旧钥可读。
        svc.unlock(account_id, "password123").unwrap();
        assert!(!pending_path.exists(), "discard 后 pending 应被删除");
        assert!(svc.verify_password(account_id, "password123").unwrap());
        assert!(!svc.verify_password(account_id, "newpassword123").unwrap());
        let vault = svc.get_vault_store().unwrap();
        let profile = vault.load_profile(account_id).unwrap();
        assert!(profile.is_some(), "discard 后数据仍以旧钥可读");
    }

    #[test]
    fn test_recover_pending_wrong_password_preserves_pending() {
        // 密码错误时：两钥探测都失败 → 上抛且保留 pending（供下次重试/人工恢复）。
        let (svc, _dir) = setup_service();
        let account = svc.create_account("WrongPw", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap();
        {
            let vault = svc.get_vault_store().unwrap();
            vault
                .save_profile(&solosoul_vault::Profile::new_with_id(
                    account_id,
                    "p",
                    b"sensitive".to_vec(),
                ))
                .unwrap();
        }

        svc.unlock(account_id, "password123").unwrap();
        let (new_salt, new_key_arr) = make_new_key("newpassword123");
        let pending_json = build_pending_config_json(&svc, account_id, &new_salt, &new_key_arr);
        svc.write_config_pending(account_id, pending_json.as_bytes())
            .unwrap();
        svc.lock();

        let err = svc.unlock(account_id, "totally_wrong").unwrap_err();
        assert!(err.contains("Invalid password"), "实际: {}", err);
        let pending_path = svc.base_path().join(account_id).join("config.json.pending");
        assert!(pending_path.exists(), "密码错误时 pending 必须保留");

        // 用正确密码重试 → 应恢复成功。
        svc.unlock(account_id, "password123").unwrap();
        assert!(!pending_path.exists());
    }

    #[test]
    fn test_change_password_success_removes_pending() {
        // 成功路径：change_password 完成后 pending 应被清除，无残留。
        let (svc, _dir) = setup_service();
        let account = svc.create_account("Clean", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap();
        svc.unlock(account_id, "password123").unwrap();
        svc.change_password(account_id, "password123", "newpassword123")
            .unwrap();
        let pending_path = svc.base_path().join(account_id).join("config.json.pending");
        assert!(!pending_path.exists(), "成功改密后 pending 不应残留");
        assert!(svc.verify_password(account_id, "newpassword123").unwrap());
    }

    #[test]
    fn test_unlock_with_session_key_rejects_pending() {
        // 生物识别/PIN 会话密钥解锁遇 pending 应显式拒绝，引导密码解锁。
        let (svc, _dir) = setup_service();
        let account = svc.create_account("BioGuard", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap();
        svc.unlock(account_id, "password123").unwrap();
        let session_key = *svc.get_session_key().unwrap();
        let (new_salt, new_key_arr) = make_new_key("newpassword123");
        let pending_json = build_pending_config_json(&svc, account_id, &new_salt, &new_key_arr);
        svc.write_config_pending(account_id, pending_json.as_bytes())
            .unwrap();
        svc.lock();

        let err = svc
            .unlock_with_session_key(account_id, &session_key)
            .unwrap_err();
        assert!(
            err.contains("Pending key rotation detected"),
            "实际: {}",
            err
        );
        let pending_path = svc.base_path().join(account_id).join("config.json.pending");
        assert!(pending_path.exists(), "拒绝解锁不应误删 pending");
    }

    #[test]
    fn test_write_config_atomic_tightens_bak_permissions() {
        // 评审补强：write_atomic 的 .bak 经 fs::copy 生成，权限为 umask 默认——
        // write_config_atomic 应对 .bak 变体收紧到 0600（与主 config 一致）。
        let (svc, _dir) = setup_service();
        let account = svc.create_account("BakPerm", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap();

        // 触发第二次 config 写（产生 .bak）。
        svc.update_password_hint(account_id, "hint").unwrap();

        let config_path = svc.base_path().join(account_id).join("config.json");
        let bak_path = config_path.with_extension("bak");
        if bak_path.exists() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = fs::metadata(&bak_path).unwrap().permissions().mode() & 0o777;
                assert_eq!(mode, 0o600, ".bak 权限应收紧为 0600，实际 {mode:o}");
            }
        }
    }
}
