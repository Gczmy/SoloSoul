//! PIN 码便捷解锁功能。
//!
//! PIN 不参与 Vault 主密钥派生。PIN 仅用于解密一个已保存的会话密钥副本
//!（与生物识别凭证逻辑类似）。会话密钥副本使用 PIN + salt 派生的 KEK 加密，
//! 存储在账户目录的本地文件中。
//!
//! 安全策略：
//! - 强 KDF (Argon2id, ~100ms) 从 PIN+salt 派生 KEK
//! - 连续 5 次错误 → 锁定 30s；10 次 → 5 分钟；之后每次 +5 分钟
//! - 锁定期间拒绝 PIN 解锁，提示使用主密码

use crate::vault_service::{AccountConfig, VaultService};
use serde::{Deserialize, Serialize};
use solosoul_crypto::cipher::{decrypt_from_bytes, encrypt_to_bytes};
use solosoul_crypto::kdf::{derive_key, KdfConfig};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// 进程级互斥锁，保护所有 PIN 操作的失败计数器不受并发干扰。
/// PinManager 在 Tauri 命令中每次都被重新创建，因此锁不能挂在实例上。
static PIN_OP_LOCK: Mutex<()> = Mutex::new(());

/// PIN 凭证文件的内容。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PinCredential {
    version: u32,
    /// 16 字节随机盐，base64 编码
    salt: String,
    /// 使用 KEK 经 AES-256-GCM 加密后的 32 字节 Vault 会话密钥，hex 编码
    ciphertext: String,
}

/// PIN 状态：返回给前端用于判断 UI 展示。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PinStatus {
    pub configured: bool,
    pub locked: bool,
    pub remaining_attempts: u32,
    pub locked_until: Option<String>, // ISO 8601
}

/// PIN 相关错误。
#[derive(Debug, Clone)]
pub enum PinError {
    Incorrect,
    Locked,
    TooShort,
    TooLong,
    NotConfigured,
    InvalidPassword,
    SetupFailed(String),
    UnlockFailed(String),
    DisableFailed(String),
    AccountNotFound,
    ParseError,
    Internal(String),
}

impl std::fmt::Display for PinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PinError::Incorrect => write!(f, "incorrect PIN"),
            PinError::Locked => write!(f, "PIN locked"),
            PinError::TooShort => write!(f, "PIN too short"),
            PinError::TooLong => write!(f, "PIN too long"),
            PinError::NotConfigured => write!(f, "PIN not configured"),
            PinError::InvalidPassword => write!(f, "invalid password"),
            PinError::SetupFailed(s) => write!(f, "setup failed: {s}"),
            PinError::UnlockFailed(s) => write!(f, "unlock failed: {s}"),
            PinError::DisableFailed(s) => write!(f, "disable failed: {s}"),
            PinError::AccountNotFound => write!(f, "account not found"),
            PinError::ParseError => write!(f, "parse error"),
            PinError::Internal(s) => write!(f, "internal: {s}"),
        }
    }
}

impl PinError {
    /// 返回给前端国际化的短代码。前端通过 `__PIN_ERR__:<code>` 识别。
    pub fn code(&self) -> &'static str {
        match self {
            PinError::Incorrect => "incorrect",
            PinError::Locked => "locked",
            PinError::TooShort => "too_short",
            PinError::TooLong => "too_long",
            PinError::NotConfigured => "not_configured",
            PinError::InvalidPassword => "invalid_password",
            PinError::SetupFailed(_) => "setup_failed",
            PinError::UnlockFailed(_) => "unlock_failed",
            PinError::DisableFailed(_) => "disable_failed",
            PinError::AccountNotFound => "account_not_found",
            PinError::ParseError => "parse_error",
            PinError::Internal(_) => "internal",
        }
    }
}

/// PIN 的 KDF 配置：**强制生产级参数**（P005）。
///
/// PIN 空间仅 10^4~10^6，且 `pin_credential` 连同 salt 落在数据目录中，
/// 攻击者拿到数据目录副本后可离线爆破。若沿用开发模式低参数（8 MiB / 2 iter），
/// 数小时即可穷举全部 6 位 PIN 解开 Vault；生产参数（64 MiB / 3 iter）将
/// 每次派生成本提升到 ~1s，使离线爆破不可行。因此 PIN 凭证**不随
/// `SOLOSOUL_SECURE` 降级**，始终使用 `KdfConfig::production()`。
fn pin_kdf_config() -> KdfConfig {
    KdfConfig::production()
}

/// 管理 PIN 凭证的创建、验证、锁定状态等。
pub struct PinManager {
    base_path: PathBuf,
}

impl PinManager {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    fn pin_credential_path(&self, account_id: &str) -> PathBuf {
        self.base_path.join(account_id).join("pin_credential")
    }

    fn config_path(&self, account_id: &str) -> PathBuf {
        self.base_path.join(account_id).join("config.json")
    }

    // ── 锁定逻辑 ──────────────────────────────────────────────

    /// 根据失败次数计算锁定时长（秒）。
    fn compute_lockout_seconds(failed_attempts: u32) -> u64 {
        match failed_attempts {
            0..=4 => 0,
            5..=9 => 30,
            10 => 300,                        // 5 分钟
            n => 300 + (n - 10) as u64 * 300, // 之后每次递增 5 分钟
        }
    }

    // ── 配置文件读写 ──────────────────────────────────────────

    fn read_config(&self, account_id: &str) -> Result<AccountConfig, PinError> {
        let path = self.config_path(account_id);
        let s = std::fs::read_to_string(&path).map_err(|_| PinError::AccountNotFound)?;
        serde_json::from_str(&s).map_err(|_| PinError::ParseError)
    }

    fn write_config(&self, account_id: &str, config: AccountConfig) -> Result<(), PinError> {
        let path = self.config_path(account_id);
        let json =
            serde_json::to_string_pretty(&config).map_err(|e| PinError::Internal(e.to_string()))?;
        std::fs::write(&path, json).map_err(|e| PinError::Internal(e.to_string()))?;
        Ok(())
    }

    fn update_config_field(
        &self,
        account_id: &str,
        f: impl FnOnce(&mut AccountConfig),
    ) -> Result<(), PinError> {
        let mut config = self.read_config(account_id)?;
        f(&mut config);
        self.write_config(account_id, config)
    }

    // ── 公有 API ──────────────────────────────────────────────

    /// 设置 PIN：验证主密码后，用 PIN 派生 KEK 加密会话密钥。
    pub fn setup_pin(
        &self,
        account_id: &str,
        password: &str,
        pin: &str,
        vault_service: &VaultService,
    ) -> Result<(), PinError> {
        // 校验 PIN 格式
        validate_pin(pin)?;

        // 先解锁以验证主密码并获取会话密钥
        vault_service
            .unlock(account_id, password)
            .map_err(|_| PinError::InvalidPassword)?;

        let session_key = vault_service
            .get_session_key()
            .ok_or_else(|| PinError::SetupFailed("no session key".into()))?;

        // 生成随机盐
        let salt = solosoul_crypto::kdf::generate_salt();
        let kdf_cfg = pin_kdf_config();

        // 从 PIN + salt 派生 KEK
        let kek = derive_key(pin, &salt, &kdf_cfg)
            .map_err(|e| PinError::SetupFailed(format!("kdf: {e}")))?;
        let kek_arr: [u8; 32] = kek
            .as_slice()
            .try_into()
            .map_err(|_| PinError::SetupFailed("kek must be 32 bytes".into()))?;

        // 用 KEK 加密会话密钥
        let ciphertext_bytes = encrypt_to_bytes(&kek_arr, session_key.as_slice(), None)
            .map_err(|e| PinError::SetupFailed(format!("encrypt: {e}")))?;

        // 写入凭证文件
        let credential = PinCredential {
            version: 1,
            salt: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                salt.as_slice(),
            ),
            ciphertext: hex::encode(ciphertext_bytes),
        };
        let cred_path = self.pin_credential_path(account_id);
        if let Some(parent) = cred_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| PinError::SetupFailed(format!("mkdir: {e}")))?;
        }
        let cred_json = serde_json::to_string_pretty(&credential)
            .map_err(|e| PinError::SetupFailed(format!("serialize: {e}")))?;
        std::fs::write(&cred_path, cred_json)
            .map_err(|e| PinError::SetupFailed(format!("write: {e}")))?;
        // 设置文件权限为私有（Unix 0600）
        if let Err(e) = set_private_file(&cred_path) {
            tracing::warn!("Failed to set private permissions on PIN credential: {}", e);
        }

        // 更新 config
        self.update_config_field(account_id, |c| {
            c.pin_enabled = true;
            c.pin_length = pin.len() as u32;
            c.pin_failed_attempts = 0;
            c.pin_locked_until = None;
        })?;

        // 写审计日志
        if let Some(vg) = vault_service.get_vault_store() {
            let _ = vg.as_ref().log_structured(
                "pin_setup",
                "auth",
                Some(account_id),
                None,
                "user",
                Some(&format!("pin_length={}", pin.len())),
            );
        }

        Ok(())
    }

    /// 使用 PIN 解锁 Vault。
    pub fn unlock_with_pin(
        &self,
        account_id: &str,
        pin: &str,
        vault_service: &VaultService,
        location: Option<&str>,
        action: Option<&str>,
    ) -> Result<(), PinError> {
        // 获取进程级互斥锁，确保失败计数器的读-改-写操作的原子性
        let _guard = PIN_OP_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let config = self.read_config(account_id)?;

        // 检查锁定状态
        if let Some(ref until) = config.pin_locked_until {
            if let Ok(until_time) = chrono::DateTime::parse_from_rfc3339(until) {
                if chrono::Utc::now() < until_time {
                    return Err(PinError::Locked);
                }
            }
        }

        // 读取凭证文件
        let cred_path = self.pin_credential_path(account_id);
        let cred_json = std::fs::read_to_string(&cred_path).map_err(|_| PinError::NotConfigured)?;
        let credential: PinCredential =
            serde_json::from_str(&cred_json).map_err(|_| PinError::ParseError)?;

        // 解码 salt
        let salt_bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &credential.salt)
                .map_err(|_| PinError::ParseError)?;
        let salt_arr: [u8; 16] = salt_bytes
            .as_slice()
            .try_into()
            .map_err(|_| PinError::ParseError)?;

        // 尝试 AES-GCM 解密。P005：优先使用生产级参数；若解密失败则回退到
        // 旧参数（`from_env()` 开发参数）重试一次——兼容**存量** PIN 凭证
        // （旧凭证由开发参数加密，直接切换生产参数会导致无法解锁并递增失败计数
        // 直至锁死账户）。旧参数解锁成功后会用生产参数重新加密凭证完成就地升级。
        let ciphertext_bytes =
            hex::decode(&credential.ciphertext).map_err(|_| PinError::ParseError)?;
        let (session_key_bytes, upgraded) =
            match decrypt_session_key_with_fallback(pin, &salt_arr, &ciphertext_bytes) {
                Ok(pair) => pair,
                Err(_) => {
                    // PIN 错误：递增失败计数
                    let attempts = config.pin_failed_attempts + 1;
                    let lockout_secs = Self::compute_lockout_seconds(attempts);
                    let locked_until = if lockout_secs > 0 {
                        Some(
                            (chrono::Utc::now() + chrono::Duration::seconds(lockout_secs as i64))
                                .to_rfc3339(),
                        )
                    } else {
                        None
                    };

                    self.update_config_field(account_id, |c| {
                        c.pin_failed_attempts = attempts;
                        c.pin_locked_until = locked_until;
                    })?;

                    return Err(PinError::Incorrect);
                }
            };

        self.update_config_field(account_id, |c| {
            c.pin_failed_attempts = 0;
            c.pin_locked_until = None;
        })?;

        // 旧参数解锁成功 → 用生产级参数重新加密凭证（就地升级，P005）
        if upgraded {
            if let Err(e) = self.upgrade_credential(account_id, pin, &salt_arr, &session_key_bytes)
            {
                tracing::warn!("Failed to upgrade PIN credential KDF params: {}", e);
            }
        }

        let key: [u8; 32] = session_key_bytes
            .as_slice()
            .try_into()
            .map_err(|_| PinError::UnlockFailed("key must be 32 bytes".into()))?;

        vault_service
            .unlock_with_session_key(account_id, &key)
            .map_err(PinError::UnlockFailed)?;

        // 关键数据访问场景由前端写 critical_field_login 审计日志，此处跳过通用日志避免重复。
        // 与 biometric.rs 中 `location != "critical_data_access"` 的跳过逻辑一致。
        if location != Some("critical_data_access") {
            let details = format!(
                "method=pin location={} action={}",
                location.unwrap_or("unknown"),
                action.unwrap_or("unlock")
            );
            if let Some(vg) = vault_service.get_vault_store() {
                let _ = vg.as_ref().log_structured(
                    "pin_unlock",
                    "auth",
                    Some(account_id),
                    None,
                    "user",
                    Some(&details),
                );
            }
        }

        Ok(())
    }

    /// 用生产级 KDF 参数重新加密 PIN 凭证（就地升级，P005）。
    fn upgrade_credential(
        &self,
        account_id: &str,
        pin: &str,
        salt: &[u8; 16],
        session_key: &[u8],
    ) -> Result<(), PinError> {
        let kek = derive_key(pin, salt, &KdfConfig::production())
            .map_err(|e| PinError::SetupFailed(format!("kdf: {e}")))?;
        let kek_arr: [u8; 32] = kek
            .as_slice()
            .try_into()
            .map_err(|_| PinError::SetupFailed("kek must be 32 bytes".into()))?;
        let ciphertext_bytes = encrypt_to_bytes(&kek_arr, session_key, None)
            .map_err(|e| PinError::SetupFailed(format!("encrypt: {e}")))?;
        let credential = PinCredential {
            version: 1,
            salt: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                salt.as_slice(),
            ),
            ciphertext: hex::encode(ciphertext_bytes),
        };
        let cred_path = self.pin_credential_path(account_id);
        if let Some(parent) = cred_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| PinError::SetupFailed(format!("mkdir: {e}")))?;
        }
        let cred_json = serde_json::to_string_pretty(&credential)
            .map_err(|e| PinError::SetupFailed(format!("serialize: {e}")))?;
        std::fs::write(&cred_path, cred_json)
            .map_err(|e| PinError::SetupFailed(format!("write: {e}")))?;
        // 设置文件权限为私有（Unix 0600）
        if let Err(e) = set_private_file(&cred_path) {
            tracing::warn!("Failed to set private permissions on PIN credential: {}", e);
        }
        Ok(())
    }

    /// 禁用 PIN（需要验证主密码）。
    pub fn disable_pin(
        &self,
        account_id: &str,
        password: &str,
        vault_service: &VaultService,
    ) -> Result<(), PinError> {
        // 验证主密码
        vault_service
            .verify_password(account_id, password)
            .map_err(|_| PinError::InvalidPassword)?
            .then_some(())
            .ok_or(PinError::InvalidPassword)?;

        // 删除凭证文件
        let cred_path = self.pin_credential_path(account_id);
        if cred_path.exists() {
            std::fs::remove_file(&cred_path)
                .map_err(|e| PinError::DisableFailed(format!("remove: {e}")))?;
        }

        // 清除 config 中的 PIN 字段
        self.update_config_field(account_id, |c| {
            c.pin_enabled = false;
            c.pin_length = 0;
            c.pin_failed_attempts = 0;
            c.pin_locked_until = None;
        })?;

        // 写审计日志
        if let Some(vg) = vault_service.get_vault_store() {
            let _ = vg.as_ref().log_structured(
                "pin_disabled",
                "auth",
                Some(account_id),
                None,
                "user",
                None,
            );
        }

        Ok(())
    }

    /// PIN 是否已配置（config 标记 + 凭证文件存在）。
    pub fn is_configured(&self, account_id: &str) -> bool {
        let config = match self.read_config(account_id) {
            Ok(c) => c,
            Err(_) => return false,
        };
        config.pin_enabled && self.pin_credential_path(account_id).exists()
    }

    /// 返回 PIN 状态（用于前端 UI 判断）。
    pub fn status(&self, account_id: &str) -> PinStatus {
        let config = match self.read_config(account_id) {
            Ok(c) => c,
            Err(_) => {
                return PinStatus {
                    configured: false,
                    locked: false,
                    remaining_attempts: 0,
                    locked_until: None,
                }
            }
        };

        let configured = config.pin_enabled && self.pin_credential_path(account_id).exists();

        let (locked, locked_until) = if let Some(ref until) = config.pin_locked_until {
            if let Ok(until_time) = chrono::DateTime::parse_from_rfc3339(until) {
                if chrono::Utc::now() < until_time {
                    (true, Some(until.clone()))
                } else {
                    (false, None)
                }
            } else {
                (false, None)
            }
        } else {
            (false, None)
        };

        let remaining_attempts = if locked {
            0
        } else {
            let failed = config.pin_failed_attempts;
            if failed < 5 {
                5u32.saturating_sub(failed)
            } else {
                // 已在锁定逻辑中处理，但可能在解锁期间：剩余 0
                0
            }
        };

        PinStatus {
            configured,
            locked,
            remaining_attempts,
            locked_until,
        }
    }

    /// 重置 PIN 失败计数和锁定状态（在用户通过更强因子——主密码或生物识别——成功登录后调用）。
    /// PIN 是比主密码/生物识别弱的便捷因子，强因子的持有者有权重置自己的 PIN 锁定状态。
    pub fn reset_attempts(&self, account_id: &str) -> Result<(), PinError> {
        let config = self.read_config(account_id)?;
        if !config.pin_enabled
            || (config.pin_failed_attempts == 0 && config.pin_locked_until.is_none())
        {
            return Ok(());
        }
        self.update_config_field(account_id, |c| {
            c.pin_failed_attempts = 0;
            c.pin_locked_until = None;
        })?;
        tracing::info!("PIN attempts reset for {}", account_id);
        Ok(())
    }

    /// 清除 PIN 凭证（在修改密码后调用，因为 KEK 需要 PIN 输入，无法自动重新加密）。
    /// 用户需重新设置 PIN。
    pub fn clear_credential(&self, account_id: &str) -> Result<(), PinError> {
        if !self.is_configured(account_id) {
            return Ok(());
        }

        let cred_path = self.pin_credential_path(account_id);
        if cred_path.exists() {
            let _ = std::fs::remove_file(&cred_path);
        }

        self.update_config_field(account_id, |c| {
            c.pin_enabled = false;
            c.pin_length = 0;
            c.pin_failed_attempts = 0;
            c.pin_locked_until = None;
        })?;

        tracing::info!("PIN credential cleared for {}", account_id);
        Ok(())
    }
}

/// 设置文件权限为私有（Unix 0600）。
fn set_private_file(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
        let mut perms = meta.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(path, perms).map_err(|e| e.to_string())
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

/// 尝试解密 PIN 会话密钥（P005）：先使用生产级 KDF 参数；失败时回退到
/// `from_env()`（旧凭证的开发参数）重试。
///
/// 返回 `(会话密钥, 是否回退了旧参数)`。两个参数都解密失败时返回 Err（PIN 错误）。
/// 注意：生产参数与 `SOLOSOUL_SECURE=1` 下的 from_env 相同，此时仅尝试一次。
fn decrypt_session_key_with_fallback(
    pin: &str,
    salt: &[u8; 16],
    ciphertext: &[u8],
) -> Result<(Vec<u8>, bool), PinError> {
    let production = KdfConfig::production();
    let kek = derive_key(pin, salt, &production)
        .map_err(|e| PinError::UnlockFailed(format!("kdf: {e}")))?;
    let kek_arr: [u8; 32] = kek
        .as_slice()
        .try_into()
        .map_err(|_| PinError::UnlockFailed("kek must be 32 bytes".into()))?;
    if let Ok(key) = decrypt_from_bytes(&kek_arr, ciphertext, None) {
        return Ok((key.to_vec(), false));
    }

    // 回退旧参数（开发模式 8 MiB / 2 iter，仅当与生产参数不同时才有意义）
    let legacy = KdfConfig::from_env();
    if legacy.memory_kb == production.memory_kb
        && legacy.iterations == production.iterations
        && legacy.parallelism == production.parallelism
    {
        return Err(PinError::Incorrect);
    }
    let kek =
        derive_key(pin, salt, &legacy).map_err(|e| PinError::UnlockFailed(format!("kdf: {e}")))?;
    let kek_arr: [u8; 32] = kek
        .as_slice()
        .try_into()
        .map_err(|_| PinError::UnlockFailed("kek must be 32 bytes".into()))?;
    match decrypt_from_bytes(&kek_arr, ciphertext, None) {
        Ok(key) => Ok((key.to_vec(), true)),
        Err(_) => Err(PinError::Incorrect),
    }
}

/// 校验 PIN 格式：仅数字，4~8 位。
fn validate_pin(pin: &str) -> Result<(), PinError> {
    if pin.len() < 4 {
        return Err(PinError::TooShort);
    }
    if pin.len() > 8 {
        return Err(PinError::TooLong);
    }
    if !pin.chars().all(|c| c.is_ascii_digit()) {
        return Err(PinError::Internal("PIN must contain only digits".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VaultService;
    use tempfile::TempDir;

    static PIN_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn setup_env() -> (TempDir, VaultService, String, PinManager) {
        let _guard = PIN_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = TempDir::new().unwrap();
        let base = dir.path().join(".solosoul");
        std::fs::create_dir_all(&base).unwrap();

        let svc = VaultService::with_base_path(base.clone());
        let account = svc
            .create_account("PIN Test", "testpassword123", None)
            .unwrap();
        let account_id = account["id"].as_str().unwrap().to_string();

        // 创建账户后锁定，以便测试 unlock 流程
        svc.lock();

        let mgr = PinManager::new(base);
        (dir, svc, account_id, mgr)
    }

    #[test]
    fn test_validate_pin_too_short() {
        assert!(matches!(validate_pin("123"), Err(PinError::TooShort)));
    }

    #[test]
    fn test_validate_pin_too_long() {
        assert!(matches!(validate_pin("123456789"), Err(PinError::TooLong)));
    }

    #[test]
    fn test_validate_pin_non_digit() {
        assert!(validate_pin("12a4").is_err());
    }

    #[test]
    fn test_validate_pin_ok() {
        assert!(validate_pin("123456").is_ok());
        assert!(validate_pin("1234").is_ok());
        assert!(validate_pin("12345678").is_ok());
    }

    #[test]
    fn test_pin_setup_and_unlock() {
        let (_dir, svc, account_id, mgr) = setup_env();

        // 先解锁才能 setup
        svc.unlock(&account_id, "testpassword123").unwrap();
        mgr.setup_pin(&account_id, "testpassword123", "123456", &svc)
            .unwrap();

        // 锁定后尝试 PIN 解锁
        svc.lock();
        assert!(!svc.is_unlocked());
        mgr.unlock_with_pin(&account_id, "123456", &svc, None, None)
            .unwrap();
        assert!(svc.is_unlocked());

        assert!(mgr.is_configured(&account_id));
    }

    #[test]
    fn test_pin_wrong_pin_increments_attempts() {
        let (_dir, svc, account_id, mgr) = setup_env();

        svc.unlock(&account_id, "testpassword123").unwrap();
        mgr.setup_pin(&account_id, "testpassword123", "123456", &svc)
            .unwrap();
        svc.lock();

        // 错误 PIN
        assert!(matches!(
            mgr.unlock_with_pin(&account_id, "000000", &svc, None, None),
            Err(PinError::Incorrect)
        ));

        let status = mgr.status(&account_id);
        assert_eq!(status.remaining_attempts, 4);
    }

    #[test]
    fn test_pin_lockout() {
        let (_dir, svc, account_id, mgr) = setup_env();

        svc.unlock(&account_id, "testpassword123").unwrap();
        mgr.setup_pin(&account_id, "testpassword123", "123456", &svc)
            .unwrap();
        svc.lock();

        // 连续 5 次错误
        for _ in 0..5 {
            let _ = mgr.unlock_with_pin(&account_id, "000000", &svc, None, None);
        }

        let status = mgr.status(&account_id);
        assert!(status.locked);
        assert!(status.locked_until.is_some());

        // 锁定期间应返回 Locked
        assert!(matches!(
            mgr.unlock_with_pin(&account_id, "123456", &svc, None, None),
            Err(PinError::Locked)
        ));
    }

    #[test]
    fn test_pin_disable() {
        let (_dir, svc, account_id, mgr) = setup_env();

        svc.unlock(&account_id, "testpassword123").unwrap();
        mgr.setup_pin(&account_id, "testpassword123", "123456", &svc)
            .unwrap();
        assert!(mgr.is_configured(&account_id));

        // 禁用
        mgr.disable_pin(&account_id, "testpassword123", &svc)
            .unwrap();
        assert!(!mgr.is_configured(&account_id));
    }

    #[test]
    fn test_pin_not_configured() {
        let (_dir, svc, account_id, mgr) = setup_env();
        assert!(!mgr.is_configured(&account_id));

        assert!(matches!(
            mgr.unlock_with_pin(&account_id, "123456", &svc, None, None),
            Err(PinError::NotConfigured)
        ));
    }

    #[test]
    fn test_compute_lockout_seconds() {
        assert_eq!(PinManager::compute_lockout_seconds(0), 0);
        assert_eq!(PinManager::compute_lockout_seconds(4), 0);
        assert_eq!(PinManager::compute_lockout_seconds(5), 30);
        assert_eq!(PinManager::compute_lockout_seconds(9), 30);
        assert_eq!(PinManager::compute_lockout_seconds(10), 300);
        assert_eq!(PinManager::compute_lockout_seconds(11), 600);
    }
}
