//! Biometric (Touch ID / Face ID / Windows Hello) primitives.
//!
//! This module is host-agnostic: it knows how to store an obfuscated master key,
//! trigger the OS biometric dialog, and unlock a `VaultService`. The Tauri
//! command wrappers live in `src-tauri/src/commands/biometric.rs` and only
//! forward parameters plus emit events if needed.
//!
//! 实现策略：
//! - macOS：将主密钥保存到受 `kSecAccessControlUserPresence` 保护的 Keychain
//!   Generic Password Item。读取时由系统触发 Touch ID / 设备密码提示框；
//!   仅在开启/验证生物识别时主动调用 LocalAuthentication，打开应用时不会弹框。
//! - Windows：使用本地加密文件 + UserConsentVerifier（Windows Hello）弹窗。
//! - 其他平台：暂不支持，使用 `StubBiometricStorage` 返回友好错误码。

use crate::auth::verify_password_core;
use crate::vault_service::{AccountConfig, VaultService};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod stub;
#[cfg(target_os = "windows")]
mod windows;

/// 旧版基于本地加密文件的存储。用于从旧版本升级时迁移凭证、当前 macOS 方案、以及测试 mock。
pub(crate) mod legacy;

/// ⚠️ 未来 Keychain 方案保留模块。详见 `macos_keychain.rs` 顶部注释。
/// 当前未使用，但保留完整实现以便团队加入 Apple Developer Program 后切换。
#[cfg(target_os = "macos")]
#[allow(dead_code)]
mod macos_keychain;

/// 设备/平台对生物识别的可用性信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiometricAvailability {
    pub available: bool,
    pub configured: bool,
    pub biometry_type: Option<String>,
    pub error: Option<String>,
}

/// 生物识别存储层返回的错误。
#[derive(Debug, Clone)]
pub enum BiometricError {
    /// 用户取消或未能完成生物识别/设备密码验证。
    UserPresenceCancelled,
    /// 设备未设置生物识别/密码，无法创建受 UserPresence 保护的 Keychain 项。
    UserPresenceUnavailable,
    /// 写入 Keychain 失败。
    KeychainWriteFailed(String),
    /// 读取 Keychain 失败。
    KeychainReadFailed(String),
    /// Keychain 中不存在对应凭证。
    KeychainItemNotFound,
    /// macOS Keychain entitlement 缺失（常见于未签名的开发构建）。
    MissingKeychainEntitlement,
    /// 读取到的主密钥格式不正确。
    InvalidKeyFormat,
    /// 旧版凭证迁移/清理失败。
    LegacyMigrationFailed(String),
    /// 当前平台不支持生物识别。
    PlatformNotSupported,
    /// 其他错误。
    Other(String),
}

impl fmt::Display for BiometricError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BiometricError::UserPresenceCancelled => write!(f, "user presence cancelled"),
            BiometricError::UserPresenceUnavailable => write!(f, "user presence unavailable"),
            BiometricError::KeychainWriteFailed(s) => write!(f, "keychain write failed: {s}"),
            BiometricError::KeychainReadFailed(s) => write!(f, "keychain read failed: {s}"),
            BiometricError::KeychainItemNotFound => write!(f, "keychain item not found"),
            BiometricError::MissingKeychainEntitlement => {
                write!(f, "macOS Keychain entitlement is missing")
            }
            BiometricError::InvalidKeyFormat => write!(f, "invalid key format"),
            BiometricError::LegacyMigrationFailed(s) => write!(f, "legacy migration failed: {s}"),
            BiometricError::PlatformNotSupported => write!(f, "platform not supported"),
            BiometricError::Other(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for BiometricError {}

impl BiometricError {
    /// 返回给前端国际化的错误码（不带 `__BIO_ERR__:` 前缀）。
    pub fn code(&self) -> &'static str {
        match self {
            BiometricError::UserPresenceCancelled => "user_presence_cancelled",
            BiometricError::UserPresenceUnavailable => "user_presence_unavailable",
            BiometricError::KeychainWriteFailed(_) => "keychain_write_failed",
            BiometricError::KeychainReadFailed(_) => "keychain_read_failed",
            BiometricError::KeychainItemNotFound => "keychain_item_not_found",
            BiometricError::MissingKeychainEntitlement => "missing_keychain_entitlement",
            BiometricError::InvalidKeyFormat => "invalid_key_format",
            BiometricError::LegacyMigrationFailed(_) => "legacy_migration_failed",
            BiometricError::PlatformNotSupported => "platform_not_supported",
            BiometricError::Other(_) => "other",
        }
    }
}

/// 平台相关的生物识别凭证存储后端。
pub(crate) trait BiometricStorage: Send + Sync {
    /// 保存凭证。`reason` 用于系统提示框（如适用）。
    fn save(&self, account_id: &str, key_hex: &str, reason: &str) -> Result<(), BiometricError>;

    /// 在已信任上下文中更新凭证中的主密钥，不触发用户提示框。
    fn update(&self, account_id: &str, key_hex: &str) -> Result<(), BiometricError>;

    /// 读取凭证。需要时由系统触发用户验证。
    fn read(&self, account_id: &str, reason: &str) -> Result<String, BiometricError>;

    /// 删除凭证。
    fn delete(&self, account_id: &str) -> Result<(), BiometricError>;

    /// 凭证是否已存在（不触发用户提示框）。
    fn exists(&self, account_id: &str) -> bool;

    /// 该后端是否依赖旧版 `biometric_key` 文件作为主存储。
    /// 返回 true 时，BiometricManager 不会清理该文件（否则凭证会丢失）。
    fn uses_legacy_file(&self) -> bool {
        false
    }
}

fn platform_storage(base_path: PathBuf) -> Box<dyn BiometricStorage + Send + Sync> {
    #[cfg(target_os = "macos")]
    return Box::new(macos::MacOsBiometricStorage::new(base_path));
    #[cfg(target_os = "windows")]
    return Box::new(windows::WindowsBiometricStorage::new(base_path));
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = base_path;
        Box::new(stub::StubBiometricStorage)
    }
}

/// Host-agnostic manager for biometric credentials.
pub struct BiometricManager {
    base_path: PathBuf,
    storage: Box<dyn BiometricStorage + Send + Sync>,
}

impl BiometricManager {
    pub fn new(base_path: PathBuf) -> Self {
        let storage = platform_storage(base_path.clone());
        Self { base_path, storage }
    }

    #[cfg(test)]
    pub(crate) fn with_storage(
        base_path: PathBuf,
        storage: Box<dyn BiometricStorage + Send + Sync>,
    ) -> Self {
        Self { base_path, storage }
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

    fn legacy_key_exists(&self, account_id: &str) -> bool {
        self.bio_key_path(account_id).exists()
    }

    fn flag_enabled(&self, account_id: &str) -> bool {
        let config_path = self.config_path(account_id);
        std::fs::read_to_string(&config_path)
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| v.get("biometricEnabled").and_then(|v| v.as_bool()))
            .unwrap_or(false)
    }

    /// 将旧版本地文件凭证迁移到当前平台存储后端（macOS Keychain）。
    /// 若配置标记未开启、当前后端已存在凭证或没有旧文件，则不执行操作。
    pub fn migrate_legacy_if_needed(&self, account_id: &str) -> Result<(), BiometricError> {
        if !self.flag_enabled(account_id) {
            return Ok(());
        }
        if self.storage.exists(account_id) {
            // 开发环境回退到本地文件时，不能清理该文件，否则凭证会丢失。
            if !self.storage.uses_legacy_file() {
                self.remove_legacy_key_file(account_id);
            }
            return Ok(());
        }
        if !self.legacy_key_exists(account_id) {
            return Ok(());
        }
        let legacy_storage = legacy::FileBiometricStorage::new(self.base_path.clone());
        let key_hex = legacy_storage.read(account_id, "")?;
        self.storage
            .save(account_id, &key_hex, "migrate legacy biometric credential")?;
        // 如果当前后端就是本地文件（开发兜底），save 后文件仍在原位，不能删除。
        if !self.storage.uses_legacy_file() {
            self.remove_legacy_key_file(account_id);
        }
        self.set_config_flag(account_id, true)?;
        Ok(())
    }

    /// Check whether biometric authentication is available and configured for
    /// the given account. `available` refers to the device/platform; `configured`
    /// refers to whether this account has a stored credential.
    pub fn availability(&self, account_id: &str) -> BiometricAvailability {
        #[cfg(target_os = "macos")]
        let (available, bt, err) = query_macos_biometric_availability();
        #[cfg(target_os = "windows")]
        let (available, bt, err) = windows::query_windows_biometric_availability();
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let (available, bt, err) = (false, None, Some("platform not supported".into()));
        let configured = self.is_configured(account_id);
        BiometricAvailability {
            available,
            configured,
            biometry_type: bt,
            error: err,
        }
    }

    /// Save a biometric credential for the account after verifying the password
    /// and requiring the user to authenticate biometrically.
    pub fn save_credential(
        &self,
        account_id: &str,
        password: &str,
        reason: &str,
    ) -> Result<(), BiometricError> {
        self.verify_password(password, account_id)?;
        // save_credential 使用严格策略（仅生物识别，不允许密码回退），
        // 确保用户确实有可用的生物识别，而不是弹出密码框。
        trigger_system_biometric(reason, true)?;
        let key_hex = derive_master_key(password, account_id, &self.base_path)?;
        self.storage.save(account_id, &key_hex, reason)?;
        self.set_config_flag(account_id, true)?;
        if !self.storage.uses_legacy_file() {
            self.remove_legacy_key_file(account_id);
        }
        Ok(())
    }

    /// 在已信任上下文中直接更新生物识别凭证中保存的主密钥。
    /// 用于修改密码后保持生物识别继续生效，不触发系统生物识别对话框。
    pub fn update_credential(&self, account_id: &str, key_hex: &str) -> Result<(), BiometricError> {
        if !self.is_configured(account_id) {
            return Ok(());
        }
        self.storage.update(account_id, key_hex)?;
        self.set_config_flag(account_id, true)?;
        if !self.storage.uses_legacy_file() {
            self.remove_legacy_key_file(account_id);
        }
        Ok(())
    }

    /// Unlock the vault using the stored biometric key. Returns the biometry
    /// type that was used (e.g. `"touchId"`).
    pub fn unlock(
        &self,
        account_id: &str,
        vault_service: &VaultService,
        reason: &str,
    ) -> Result<String, BiometricError> {
        self.migrate_legacy_if_needed(account_id)?;
        // 在读取凭证前先触发系统生物识别弹窗（Touch ID / 设备密码）。
        // 这样即使使用本地文件存储，也能保证用户身份验证。
        // unlock 保留设备密码回退（strict=false），避免指纹失败/锁定后无法登录。
        trigger_system_biometric(reason, false)?;
        let key_hex = self.storage.read(account_id, reason)?;
        let key_bytes = hex::decode(&key_hex).map_err(|_| BiometricError::InvalidKeyFormat)?;
        let key: [u8; 32] = key_bytes
            .as_slice()
            .try_into()
            .map_err(|_| BiometricError::InvalidKeyFormat)?;
        vault_service
            .unlock_with_session_key(account_id, &key)
            .map_err(BiometricError::Other)?;
        Ok(self
            .availability(account_id)
            .biometry_type
            .unwrap_or_else(|| "unknown".into()))
    }

    /// Delete the stored biometric credential after password verification.
    pub fn delete_credential(
        &self,
        account_id: &str,
        password: &str,
    ) -> Result<(), BiometricError> {
        self.verify_password(password, account_id)?;
        // Keychain 项可能不存在（旧账户只有本地文件），NotFound 时不阻断关闭流程。
        match self.storage.delete(account_id) {
            Ok(()) | Err(BiometricError::KeychainItemNotFound) => {}
            Err(e) => return Err(e),
        }
        self.set_config_flag(account_id, false)?;
        self.remove_legacy_key_file(account_id);
        Ok(())
    }

    /// Trigger the system biometric dialog as a self-test.
    /// 使用严格策略确保实际触发生物识别，不回落到设备密码。
    pub fn test(&self, reason: &str) -> Result<bool, BiometricError> {
        trigger_system_biometric(reason, true)?;
        Ok(true)
    }

    /// Verify the master password for the account.
    pub fn verify_password(&self, password: &str, account_id: &str) -> Result<(), BiometricError> {
        let cfg = read_account_config(account_id, &self.base_path)?;
        if verify_password_core(password, &cfg).map_err(BiometricError::Other)? {
            Ok(())
        } else {
            Err(BiometricError::Other("Invalid password".into()))
        }
    }

    /// 从当前账户配置派生主密钥并返回十六进制字符串。
    pub fn derive_key_hex(
        &self,
        password: &str,
        account_id: &str,
    ) -> Result<String, BiometricError> {
        derive_master_key(password, account_id, &self.base_path)
    }

    /// Return true if the account has enabled biometric AND a stored credential exists.
    /// 兼容旧版本地文件：只要 flag 为 true 且 Keychain/旧文件任意一个存在，即视为已配置。
    pub fn is_configured(&self, account_id: &str) -> bool {
        let has_flag = self.flag_enabled(account_id);
        let has_key = self.storage.exists(account_id) || self.legacy_key_exists(account_id);
        tracing::debug!(
            "biometric is_configured for {}: flag={}, key={}",
            account_id,
            has_flag,
            has_key
        );
        has_flag && has_key
    }

    /// 读取已保存的生物识别主密钥（十六进制字符串）。
    /// 在 Keychain 方案下会触发系统验证提示。
    pub fn read_stored_key_hex(
        &self,
        account_id: &str,
        reason: &str,
    ) -> Result<String, BiometricError> {
        self.storage.read(account_id, reason)
    }

    fn remove_legacy_key_file(&self, account_id: &str) {
        let path = self.bio_key_path(account_id);
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }
        let _ = std::fs::remove_file(path.with_extension("key.old"));
    }

    fn set_config_flag(&self, account_id: &str, enabled: bool) -> Result<(), BiometricError> {
        let config_path = self.config_path(account_id);
        let s = std::fs::read_to_string(&config_path)
            .map_err(|_| BiometricError::Other("Account not found".into()))?;
        let mut v: serde_json::Value =
            serde_json::from_str(&s).map_err(|_| BiometricError::Other("Parse error".into()))?;
        v["biometricEnabled"] = serde_json::Value::Bool(enabled);
        std::fs::write(
            config_path,
            serde_json::to_string_pretty(&v).map_err(|e| BiometricError::Other(e.to_string()))?,
        )
        .map_err(|e| BiometricError::Other(e.to_string()))
    }
}

pub fn is_macos() -> bool {
    std::env::consts::OS == "macos"
}

fn read_account_config(
    account_id: &str,
    base_path: &Path,
) -> Result<AccountConfig, BiometricError> {
    let p = base_path.join(account_id).join("config.json");
    let s = std::fs::read_to_string(&p)
        .map_err(|_| BiometricError::Other("Account not found".into()))?;
    serde_json::from_str(&s).map_err(|_| BiometricError::Other("Parse error".into()))
}

fn derive_master_key(
    password: &str,
    account_id: &str,
    base_path: &Path,
) -> Result<String, BiometricError> {
    let cfg = read_account_config(account_id, base_path)?;
    let salt_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &cfg.salt)
        .map_err(|_| BiometricError::Other("Invalid salt".into()))?;
    let salt: [u8; 16] = salt_bytes
        .as_slice()
        .try_into()
        .map_err(|_| BiometricError::Other("Bad salt len".into()))?;
    let k = cfg.kdf_config();
    let mk = solosoul_crypto::kdf::derive_key(password, &salt, &k)
        .map_err(|_| BiometricError::Other("KDF failed".into()))?;
    Ok(hex::encode(mk.as_slice()))
}

pub fn trigger_system_biometric(reason: &str, strict: bool) -> Result<(), BiometricError> {
    #[cfg(target_os = "macos")]
    return trigger_macos_biometric(reason, strict);
    #[cfg(target_os = "windows")]
    return windows::trigger_windows_biometric(reason, strict);
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (reason, strict);
        Err(BiometricError::PlatformNotSupported)
    }
}

#[cfg(target_os = "macos")]
fn trigger_macos_biometric(reason: &str, strict: bool) -> Result<(), BiometricError> {
    use std::ffi::{c_void, CString};
    use std::sync::mpsc;

    use block2::RcBlock;
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, NSObject};

    let la_name = c"LAContext";
    let la_cls = AnyClass::get(la_name).ok_or(BiometricError::UserPresenceUnavailable)?;

    // SAFETY: LAContext 是已知的 Objective-C 类，msg_send! 通过 objc2 运行时安全调用 ObjC 消息发送。
    // alloc/init 是标准 ObjC 构造模式，返回的可保留对象由调用方负责 release。
    let ctx: *mut NSObject = unsafe {
        let alloc: *mut NSObject = msg_send![la_cls, alloc];
        msg_send![alloc, init]
    };
    if ctx.is_null() {
        return Err(BiometricError::UserPresenceUnavailable);
    }

    let c_reason =
        CString::new(reason).map_err(|_| BiometricError::Other("invalid reason string".into()))?;
    let ns_name = c"NSString";
    let ns_cls = AnyClass::get(ns_name)
        .ok_or_else(|| BiometricError::Other("NSString class not found".into()))?;
    // SAFETY: NSString 是已知 ObjC 类 +initWithUTF8String: 接收非空 C 字符串指针，
    // c_reason 是刚分配的 CString，在 msg_send 期间保持有效。
    let ns_reason: *mut NSObject = unsafe {
        let alloc: *mut NSObject = msg_send![ns_cls, alloc];
        msg_send![alloc, initWithUTF8String: c_reason.as_ptr()]
    };
    if ns_reason.is_null() {
        return Err(BiometricError::Other("failed to create NSString".into()));
    }

    let (tx, rx) = mpsc::channel::<bool>();

    let block = RcBlock::new(move |success: i8, _error: *mut c_void| {
        let _ = tx.send(success != 0);
    });

    // strict=true: LAPolicyDeviceOwnerAuthenticationWithBiometrics = 1（仅生物识别，无密码回退）
    // strict=false: LAPolicyDeviceOwnerAuthentication = 2（生物识别优先，失败可回退设备密码）
    let policy: i64 = if strict { 1 } else { 2 };
    // SAFETY: ctx 与 ns_reason 均为刚创建的非空 ObjC 对象指针；evaluatePolicy:reply:
    // 在 block 返回前不会释放这些参数；block 是 RcBlock，保证在跨线程回调期间有效。
    unsafe {
        let _: () = msg_send![
            ctx,
            evaluatePolicy: policy,
            localizedReason: ns_reason,
            reply: &*block,
        ];
    }

    let success = rx
        .recv()
        .map_err(|_| BiometricError::UserPresenceCancelled)?;

    // Release manually-owned ObjC objects (MRC)
    // SAFETY: ctx 和 ns_reason 均为 alloc/init 产生的 +1 retain 对象，evaluatePolicy:reply:
    // 同步执行完毕不再需要它们，在此 release 归还所有权是标准 MRC 模式。
    unsafe {
        let _: () = msg_send![ctx, release];
        let _: () = msg_send![ns_reason, release];
    }

    if success {
        Ok(())
    } else {
        Err(BiometricError::UserPresenceCancelled)
    }
}

/// 使用 canEvaluatePolicy:error:（policy=1）检测 macOS 设备是否真正支持并已启用生物识别。
/// 返回 (available, biometry_type, error_message)。
#[cfg(target_os = "macos")]
pub(crate) fn query_macos_biometric_availability() -> (bool, Option<String>, Option<String>) {
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, NSObject};

    let la_name = c"LAContext";
    let la_cls = match AnyClass::get(la_name) {
        Some(cls) => cls,
        None => return (false, None, Some("LAContext class not found".into())),
    };

    // SAFETY: same pattern as trigger_macos_biometric
    let ctx: *mut NSObject = unsafe {
        let alloc: *mut NSObject = msg_send![la_cls, alloc];
        msg_send![alloc, init]
    };
    if ctx.is_null() {
        return (false, None, Some("failed to create LAContext".into()));
    }

    // LAPolicyDeviceOwnerAuthenticationWithBiometrics = 1
    let mut error: *mut NSObject = std::ptr::null_mut();
    let success: i8 = unsafe { msg_send![ctx, canEvaluatePolicy: 1i64, error: &mut error] };

    let biometry_type: i64 = unsafe { msg_send![ctx, biometryType] };

    // SAFETY: release the context
    unsafe {
        let _: () = msg_send![ctx, release];
    }

    if success != 0 {
        // canEvaluatePolicy: 成功 → 设备真的支持生物识别
        let bt = match biometry_type {
            1 => Some("touchId".to_string()),
            2 => Some("faceId".to_string()),
            3 => Some("opticId".to_string()),
            _ => None,
        };
        (true, bt, None)
    } else {
        let err_msg = if !error.is_null() {
            // SAFETY: error is non-null from canEvaluatePolicy returning NO
            let code: i64 = unsafe { msg_send![error, code] };
            // LAError codes:
            // LAErrorBiometryNotAvailable = 6
            // LAErrorBiometryNotEnrolled = 7
            // LAErrorBiometryLockout = 8
            // LAErrorPasscodeNotSet = 5
            match code {
                5 => "passcode not set on device".into(),
                6 => "biometry not available on this device".into(),
                7 => "biometry not enrolled (no fingers registered)".into(),
                8 => "biometry locked out".into(),
                _ => format!("canEvaluatePolicy failed (code={})", code),
            }
        } else {
            "biometric authentication not available".into()
        };
        (false, None, Some(err_msg))
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

    fn file_storage(base: PathBuf) -> Box<dyn BiometricStorage + Send + Sync> {
        Box::new(legacy::FileBiometricStorage::new(base))
    }

    fn manager_from_home() -> BiometricManager {
        let home = std::env::var("HOME").unwrap();
        let base = std::path::PathBuf::from(home).join(".solosoul");
        BiometricManager::with_storage(base.clone(), file_storage(base))
    }

    fn create_test_account_config(password: &str) -> (crate::vault_service::AccountConfig, String) {
        let salt = solosoul_crypto::kdf::generate_salt();
        let salt_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, salt);
        let kdf_cfg = solosoul_crypto::kdf::KdfConfig::balanced();
        let master_key = solosoul_crypto::kdf::derive_key(password, &salt, &kdf_cfg).unwrap();
        let mk: [u8; 32] = master_key.as_slice().try_into().unwrap();
        let verify_hash = hex::encode(
            solosoul_crypto::hkdf_ext::derive_hkdf_key(&mk, &salt, b"SOLOSOUL_VAULT_VERIFY_v1")
                .unwrap(),
        );
        let master_key_hex = hex::encode(master_key.as_slice());

        let cfg = crate::vault_service::AccountConfig {
            account_id: "test_acc".to_string(),
            name: "Test".to_string(),
            salt: salt_b64,
            verify_hash,
            created_at: chrono::Utc::now().to_rfc3339(),
            crypto_version: 3,
            password_hint: None,
            last_login_at: None,
            last_operation_at: None,
            last_operation_desc: None,
            biometric_enabled: false,
            pin_enabled: false,
            pin_length: 0,
            pin_failed_attempts: 0,
            pin_locked_until: None,
            kdf_memory_kb: None,
            kdf_iterations: None,
            kdf_parallelism: None,
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
    fn test_file_storage_roundtrip() {
        with_temp_home(|_path| {
            let base = std::path::PathBuf::from(std::env::var("HOME").unwrap()).join(".solosoul");
            let storage = file_storage(base.clone());
            let account_id = "acc-1";
            std::fs::create_dir_all(base.join(account_id)).unwrap();
            let key_hex = "deadbeef1234567890abcdef1234567890abcdef1234567890abcdef12345678";
            storage.save(account_id, key_hex, "reason").unwrap();
            let read_back = storage.read(account_id, "reason").unwrap();
            assert_eq!(read_back, key_hex);
            storage.delete(account_id).unwrap();
            assert!(storage.read(account_id, "reason").is_err());
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
            manager.set_config_flag(account_id, true).unwrap();
            manager
                .storage
                .save(account_id, "aabbccdd", "reason")
                .unwrap();

            assert!(manager.is_configured(account_id));

            // Disable flag
            manager.set_config_flag(account_id, false).unwrap();
            assert!(!manager.is_configured(account_id));
        });
    }

    #[test]
    fn test_legacy_migration_to_storage() {
        with_temp_home(|path| {
            let account_id = "acc-migrate";
            let base = path.join(".solosoul");
            let acct_path = base.join(account_id);
            std::fs::create_dir_all(&acct_path).unwrap();

            // Simulate old account with biometricEnabled=true and legacy file
            let config = serde_json::json!({
                "accountId": account_id,
                "name": "Test",
                "salt": "c2FsdDEyMzQ1Njc=",
                "verifyHash": "abcd",
                "createdAt": "2024-01-01T00:00:00Z",
                "cryptoVersion": 2,
                "biometricEnabled": true,
            });
            std::fs::write(
                acct_path.join("config.json"),
                serde_json::to_string_pretty(&config).unwrap(),
            )
            .unwrap();

            // Legacy key file
            let legacy_key = "deadbeef1234567890abcdef1234567890abcdef1234567890abcdef12345678";
            let legacy_storage = legacy::FileBiometricStorage::new(base.clone());
            legacy_storage
                .save(account_id, legacy_key, "reason")
                .unwrap();
            assert!(manager_from_home().legacy_key_exists(account_id));

            // Migrate to a separate target storage that does not use the legacy file path.
            struct NonLegacyFileStorage(legacy::FileBiometricStorage);
            impl BiometricStorage for NonLegacyFileStorage {
                fn save(
                    &self,
                    account_id: &str,
                    key_hex: &str,
                    reason: &str,
                ) -> Result<(), BiometricError> {
                    self.0.save(account_id, key_hex, reason)
                }
                fn update(&self, account_id: &str, key_hex: &str) -> Result<(), BiometricError> {
                    self.0.update(account_id, key_hex)
                }
                fn read(&self, account_id: &str, reason: &str) -> Result<String, BiometricError> {
                    self.0.read(account_id, reason)
                }
                fn delete(&self, account_id: &str) -> Result<(), BiometricError> {
                    self.0.delete(account_id)
                }
                fn exists(&self, account_id: &str) -> bool {
                    self.0.exists(account_id)
                }
                fn uses_legacy_file(&self) -> bool {
                    false
                }
            }

            let target_base = base.join("migrated");
            std::fs::create_dir_all(&target_base).unwrap();
            let manager = BiometricManager::with_storage(
                base.clone(),
                Box::new(NonLegacyFileStorage(legacy::FileBiometricStorage::new(
                    target_base.clone(),
                ))),
            );

            manager.migrate_legacy_if_needed(account_id).unwrap();

            // Legacy file should be removed; target storage should contain the same key
            assert!(!manager.legacy_key_exists(account_id));
            let target_storage = legacy::FileBiometricStorage::new(target_base);
            assert_eq!(target_storage.read(account_id, "").unwrap(), legacy_key);
            assert!(manager.is_configured(account_id));
        });
    }

    #[test]
    fn test_delete_credential_ignores_missing_keychain() {
        with_temp_home(|path| {
            let account_id = "acc-delete-missing";
            let base = path.join(".solosoul");
            let acct_path = base.join(account_id);
            std::fs::create_dir_all(&acct_path).unwrap();

            // Account with biometric enabled but no Keychain item and no legacy file
            let config = serde_json::json!({
                "accountId": account_id,
                "name": "Test",
                "salt": "c2FsdDEyMzQ1Njc=",
                "verifyHash": "abcd",
                "createdAt": "2024-01-01T00:00:00Z",
                "cryptoVersion": 2,
                "biometricEnabled": true,
            });
            std::fs::write(
                acct_path.join("config.json"),
                serde_json::to_string_pretty(&config).unwrap(),
            )
            .unwrap();

            // Create a VaultService-compatible config so verify_password passes
            let (cfg, _) = create_test_account_config("mypassword");
            std::fs::write(
                acct_path.join("config.json"),
                serde_json::to_string_pretty(&cfg).unwrap(),
            )
            .unwrap();

            let manager = BiometricManager::with_storage(
                base.clone(),
                Box::new(legacy::FileBiometricStorage::new(base.join("target"))),
            );

            // Should succeed even though the storage item is missing
            manager.delete_credential(account_id, "mypassword").unwrap();
            assert!(!manager.flag_enabled(account_id));
        });
    }

    #[test]
    fn test_set_config_flag_missing_account() {
        with_temp_home(|_path| {
            let manager = manager_from_home();
            let result = manager.set_config_flag("nonexistent", true);
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

            let manager = BiometricManager::with_storage(
                path.join(".solosoul"),
                file_storage(path.join(".solosoul")),
            );
            assert!(manager.verify_password(password, account_id).is_ok());
        });
    }

    #[test]
    fn test_macos_biometric_availability_shape() {
        // 使用 manager.availability() 以保持平台无关，避免直接调用 #[cfg] 限定的函数
        let manager = manager_from_home();
        let result = manager.availability("nonexistent");
        let available = result.available;
        let bt = result.biometry_type;
        let err = result.error;
        // 只验证返回结构，不验证具体值（CI 可能无 Touch ID 硬件）
        if let Some(ref bt_val) = bt {
            assert!(
                bt_val == "touchId" || bt_val == "faceId" || bt_val == "opticId",
                "unexpected biometry_type: {}",
                bt_val
            );
        }
        if let Some(ref err_msg) = err {
            assert!(!err_msg.is_empty(), "error message should not be empty");
        }
        // available=true 时必须有 biometry_type
        if available {
            assert!(bt.is_some(), "available=true must have biometry_type");
        }
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

            let manager = BiometricManager::with_storage(
                path.join(".solosoul"),
                file_storage(path.join(".solosoul")),
            );
            assert!(manager
                .verify_password("wrongpassword", account_id)
                .is_err());
        });
    }
}
