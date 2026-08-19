//! Vault service - manages accounts and vault lifecycle.
//! Stores accounts in ~/.solosoul/ with per-account config and vault.db

#[cfg(test)]
use crate::biometric::legacy::FileBiometricStorage;
use crate::biometric::BiometricManager;
use crate::vault_file_system::{LocalVaultFileSystem, VaultFileSystem};
use serde::{Deserialize, Serialize};
use solosoul_crypto::kdf::KdfConfig;
use solosoul_vault::VaultStore;
use std::collections::HashMap;

/// P032：主密码失败计数读-改-写的原子化互斥锁（镜像 pin.rs 的 PIN_OP_LOCK 模式，
/// 保证并发解锁时失败计数不丢更新）。
static PASSWORD_ATTEMPT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// P032：主密码锁定错误文案。前端 `rustErrors.ts` 精确匹配后映射为
/// `common:password_locked` 双语文案（镜像 PIN 的 `__PIN_ERR__:locked` 约定）。
pub(crate) const MASTER_PASSWORD_LOCKED_ERR: &str = "Too many failed attempts; try again later";

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
#[cfg(any(unix, test))]
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
    // P022: 移除 salt/verify_hash——仅供前端零消费，暴露扩大 WebView 攻击面
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
}

impl Default for VaultService {
    fn default() -> Self {
        Self::new()
    }
}

// P025: impl VaultService 按域拆分——账户 CRUD / 解锁会话 / SAF 同步
mod account;
mod saf;
#[cfg(test)]
mod tests;
mod unlock;
