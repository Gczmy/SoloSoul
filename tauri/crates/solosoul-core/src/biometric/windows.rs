//! Windows Hello 生物识别凭证存储与验证实现。
//!
//! 实现策略：
//! - 凭证存储使用 **DPAPI**（`CryptProtectData` / `CryptUnprotectData`）保护本地文件。
//!   P002：旧实现把主密钥存进 `biometric_key` 文件，文件加密密钥仅由公开的
//!   `SHA256(account_id)` 经 HKDF 派生——任何用户态进程都能重算密钥还原主密钥，
//!   完全绕过 Windows Hello。DPAPI 的加解密绑定当前 Windows 用户登录凭据，
//!   用户态进程无法仅凭公开值还原主密钥。
//! - 可用性检测使用 `UserConsentVerifier::CheckAvailabilityAsync()`。
//! - 用户验证使用 `UserConsentVerifier::RequestVerificationAsync()`。
//! - Windows Hello 始终允许 PIN 作为用户验证回退，因此 strict/strict=false
//!   在此平台上行为相同（已验证 = 成功，其他 = 失败）。
//! - 兼容迁移：读取时若文件不带 DPAPI 魔数头（旧版 XOR / SHA256 派生格式），
//!   先用 `legacy::FileBiometricStorage` 解出密钥，再原子改写为 DPAPI 格式。

use super::{BiometricError, BiometricStorage};
use crate::biometric::legacy::FileBiometricStorage;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use zeroize::Zeroizing;

/// DPAPI 文件格式魔数头。旧版文件（XOR / SHA256 派生 AES）不带此前缀，
/// 用于在读取时识别并迁移。
const DPAPI_MAGIC: &[u8] = b"SOLOSOUL_DPAPI_V1\0";

pub struct WindowsBiometricStorage {
    base_path: PathBuf,
    /// 仅用于兼容迁移：读取旧版（非 DPAPI）凭证文件。
    legacy_storage: FileBiometricStorage,
}

impl WindowsBiometricStorage {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            legacy_storage: FileBiometricStorage::new(base_path.clone()),
            base_path,
        }
    }

    fn key_path(&self, account_id: &str) -> PathBuf {
        self.base_path.join(account_id).join("biometric_key")
    }
}

impl BiometricStorage for WindowsBiometricStorage {
    fn save(&self, account_id: &str, key_hex: &str, _reason: &str) -> Result<(), BiometricError> {
        write_dpapi_key_file(&self.key_path(account_id), key_hex)
    }

    fn update(&self, account_id: &str, key_hex: &str) -> Result<(), BiometricError> {
        write_dpapi_key_file(&self.key_path(account_id), key_hex)
    }

    fn read(&self, account_id: &str, _reason: &str) -> Result<String, BiometricError> {
        let path = self.key_path(account_id);
        let content = std::fs::read(&path).map_err(|_| BiometricError::KeychainItemNotFound)?;
        // DPAPI 格式：直接解密。
        if content.starts_with(DPAPI_MAGIC) {
            return read_dpapi_key_file(&content[DPAPI_MAGIC.len()..]);
        }
        // 旧版格式（迁移前写入）：走 legacy 解密，成功后原子改写为 DPAPI 格式。
        // 读取失败则原样返回（文件可能损坏），不回写任何内容。
        let key_hex = self
            .legacy_storage
            .read(account_id, "")
            .map_err(|_| BiometricError::KeychainItemNotFound)?;
        if let Err(e) = migrate_to_dpapi(&path, &key_hex) {
            tracing::warn!(
                "Failed to migrate biometric key file to DPAPI for {}: {}",
                account_id,
                e
            );
        }
        Ok(key_hex)
    }

    fn delete(&self, account_id: &str) -> Result<(), BiometricError> {
        let path = self.key_path(account_id);
        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| BiometricError::Other(format!("Failed to remove key file: {e}")))?;
        }
        let _ = std::fs::remove_file(path.with_extension("key.old"));
        Ok(())
    }

    fn exists(&self, account_id: &str) -> bool {
        self.key_path(account_id).exists()
    }

    fn uses_legacy_file(&self) -> bool {
        // 凭证仍存于 legacy 路径 `biometric_key`（现为 DPAPI 格式），
        // BiometricManager 不得清理该文件，否则凭证丢失。
        true
    }
}

// ---------------------------------------------------------------------------
// DPAPI 凭证文件读写（P002）
// ---------------------------------------------------------------------------

/// 原子迁移旧版凭证文件为 DPAPI 格式：先写临时文件，再 `rename` 覆盖原文件。
/// 直接原地写入若在写入中途崩溃，会留下带魔数头但载荷损坏的文件，
/// 后续读取按 DPAPI 路径解密失败且不再尝试 legacy 兜底 → 凭证数据丢失。
/// 同卷 `rename` 是原子的，旧文件一直保留到新文件完整落盘。
fn migrate_to_dpapi(path: &Path, key_hex: &str) -> Result<(), BiometricError> {
    let tmp_path = path.with_extension("key.new");
    write_dpapi_key_file(&tmp_path, key_hex)?;
    std::fs::rename(&tmp_path, path).map_err(|e| {
        BiometricError::Other(format!(
            "Failed to replace legacy key file with DPAPI format: {}",
            e
        ))
    })?;
    Ok(())
}

fn write_dpapi_key_file(path: &Path, key_hex: &str) -> Result<(), BiometricError> {
    let blob = dpapi_protect(key_hex.as_bytes())?;
    let parent = path.parent().ok_or_else(|| {
        BiometricError::Other(format!(
            "Invalid path: no parent directory for {}",
            path.display()
        ))
    })?;
    std::fs::create_dir_all(parent).map_err(|e| {
        BiometricError::Other(format!("Failed to create {}: {}", path.display(), e))
    })?;
    let mut out = Vec::with_capacity(DPAPI_MAGIC.len() + blob.len());
    out.extend_from_slice(DPAPI_MAGIC);
    out.extend_from_slice(&blob);
    std::fs::write(path, out)
        .map_err(|e| BiometricError::Other(format!("Failed to write {}: {}", path.display(), e)))?;
    Ok(())
}

fn read_dpapi_key_file(payload: &[u8]) -> Result<String, BiometricError> {
    let plain = dpapi_unprotect(payload)?;
    // 注：zeroize 1.8 的 Zeroizing 无 into_inner，只能 to_vec 拷贝一份明文；
    // 原 Zeroizing 缓冲区仍会在 drop 时清零。返回的 String 本就是调用方要的明文。
    String::from_utf8(plain.to_vec()).map_err(|_| BiometricError::InvalidKeyFormat)
}

/// 使用 DPAPI `CryptProtectData` 加密数据。加密密钥绑定当前 Windows 用户
/// 登录凭据；输出缓冲区由系统分配，使用后必须 `LocalFree`。
fn dpapi_protect(data: &[u8]) -> Result<Zeroizing<Vec<u8>>, BiometricError> {
    use windows::core::PWSTR;
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{
        CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    // 空输入时传 null 而非悬垂指针（零长切片的 as_ptr() 非空但指向越界）。
    let input = CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: if data.is_empty() {
            std::ptr::null_mut()
        } else {
            data.as_ptr() as *mut u8
        },
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    // SAFETY: input 引用 data 的合法内存（空输入时 pbData 为 null + cbData 0）；
    // output 由系统写入并分配缓冲区；CRYPTPROTECT_UI_FORBIDDEN 禁止弹窗，
    // pOptionalEntropy/pvReserved/pPromptStruct 均为 None。失败时
    // output.pbData 保持空，无需释放。成功后必须 LocalFree(output.pbData)。
    unsafe {
        CryptProtectData(
            &input,
            PWSTR::null(),
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
        .map_err(|e| BiometricError::Other(format!("DPAPI CryptProtectData failed: {e}")))?;
    }
    // SAFETY: output.pbData 由 CryptProtectData 成功分配，长度 output.cbData。
    let out = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    // SAFETY: output.pbData 是 CryptProtectData 用 LocalAlloc 分配的缓冲区。
    unsafe {
        let _ = LocalFree(HLOCAL(output.pbData as *mut core::ffi::c_void));
    }
    Ok(Zeroizing::new(out))
}

/// 使用 DPAPI `CryptUnprotectData` 解密。仅当前 Windows 用户（登录凭据）
/// 能成功解密；跨用户/跨机器解密失败。
fn dpapi_unprotect(blob: &[u8]) -> Result<Zeroizing<Vec<u8>>, BiometricError> {
    use windows::core::PWSTR;
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{
        CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    // 与 dpapi_protect 对称：空输入时传 null 而非悬垂指针。
    let input = CRYPT_INTEGER_BLOB {
        cbData: blob.len() as u32,
        pbData: if blob.is_empty() {
            std::ptr::null_mut()
        } else {
            blob.as_ptr() as *mut u8
        },
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    let mut description = PWSTR::null();
    // SAFETY: input 引用 blob 合法内存；output/description 由系统写入；
    // CRYPTPROTECT_UI_FORBIDDEN 禁止弹窗。成功后必须 LocalFree 两个缓冲区。
    unsafe {
        CryptUnprotectData(
            &input,
            Some(&mut description),
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
        .map_err(|e| {
            BiometricError::Other(format!(
                "DPAPI CryptUnprotectData failed (credential may belong to another user/profile): {e}"
            ))
        })?;
    }
    // SAFETY: output.pbData 由 CryptUnprotectData 成功分配，长度 output.cbData。
    let out = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    // SAFETY: output.pbData 与 description 均由系统 LocalAlloc 分配。
    unsafe {
        let _ = LocalFree(HLOCAL(output.pbData as *mut core::ffi::c_void));
        if !description.is_null() {
            let _ = LocalFree(HLOCAL(description.as_ptr() as *mut core::ffi::c_void));
        }
    }
    Ok(Zeroizing::new(out))
}

/// 确保当前线程已初始化为多线程单元（MTA），WinRT API 需要此初始化。
fn ensure_mta() -> Result<(), BiometricError> {
    // 使用 OnceLock 确保只初始化一次
    static INIT: OnceLock<Result<(), ()>> = OnceLock::new();

    INIT.get_or_init(|| {
        // SAFETY: CoInitializeEx 是 Windows COM 初始化函数。
        // COINIT_MULTITHREADED = 0x0 (第二个参数)
        unsafe {
            let hr = windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_MULTITHREADED,
            );
            // S_OK = 0, S_FALSE = 1 (already initialized, which is fine)
            if hr.is_ok() || hr.0 == 1 {
                Ok(())
            } else {
                tracing::warn!("CoInitializeEx failed with HRESULT={}", hr.0);
                Err(())
            }
        }
    });

    INIT.get()
        .copied()
        .unwrap_or(Err(()))
        .map_err(|_| BiometricError::Other("COM initialization failed".into()))?;
    Ok(())
}

/// 使用 UserConsentVerifier::CheckAvailabilityAsync() 检测 Windows 设备
/// 是否支持并已配置 Windows Hello 生物识别。
/// 返回 (available, biometry_type, error_message)。
pub(crate) fn query_windows_biometric_availability() -> (bool, Option<String>, Option<String>) {
    if let Err(e) = ensure_mta() {
        return (false, None, Some(format!("COM init failed: {:?}", e)));
    }

    match UserConsentVerifierHelper::check_availability() {
        Ok(availability) => match availability {
            UserConsentVerifierAvailabilityHelper::Available => {
                // Windows Hello 统一标识为 windowsHello，不区分指纹/面部/虹膜
                (true, Some("windowsHello".to_string()), None)
            }
            UserConsentVerifierAvailabilityHelper::DeviceNotPresent => {
                (false, None, Some("Windows Hello device not present".into()))
            }
            UserConsentVerifierAvailabilityHelper::NotConfiguredForUser => (
                false,
                None,
                Some("Windows Hello not configured for this user".into()),
            ),
            UserConsentVerifierAvailabilityHelper::DisabledByPolicy => {
                (false, None, Some("Windows Hello disabled by policy".into()))
            }
            UserConsentVerifierAvailabilityHelper::DeviceBusy => {
                (false, None, Some("Windows Hello device busy".into()))
            }
        },
        Err(e) => (
            false,
            None,
            Some(format!("CheckAvailabilityAsync failed: {e}")),
        ),
    }
}

/// 使用 UserConsentVerifier::RequestVerificationAsync() 触发 Windows Hello 验证弹窗。
/// Windows Hello 始终允许 PIN 回退，因此 strict 参数在此平台上无实际区分效果。
pub(crate) fn trigger_windows_biometric(reason: &str, _strict: bool) -> Result<(), BiometricError> {
    ensure_mta()?;

    match UserConsentVerifierHelper::request_verification(reason) {
        Ok(result) => match result {
            UserConsentVerificationResultHelper::Verified => Ok(()),
            UserConsentVerificationResultHelper::Canceled => {
                Err(BiometricError::UserPresenceCancelled)
            }
            UserConsentVerificationResultHelper::DeviceNotPresent
            | UserConsentVerificationResultHelper::NotConfiguredForUser
            | UserConsentVerificationResultHelper::DisabledByPolicy
            | UserConsentVerificationResultHelper::DeviceBusy => {
                Err(BiometricError::UserPresenceUnavailable)
            }
            UserConsentVerificationResultHelper::RetriesExhausted => {
                Err(BiometricError::UserPresenceCancelled)
            }
        },
        Err(e) => {
            // 检查是否是缺少 Windows Hello 支持的常见错误
            let msg = e.to_string().to_lowercase();
            if msg.contains("not available") || msg.contains("not configured") {
                Err(BiometricError::UserPresenceUnavailable)
            } else {
                Err(BiometricError::Other(format!(
                    "Windows Hello verification failed: {e}"
                )))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helper types and wrappers around the `windows` crate WinRT projections
// ---------------------------------------------------------------------------

enum UserConsentVerifierAvailabilityHelper {
    Available,
    DeviceNotPresent,
    NotConfiguredForUser,
    DisabledByPolicy,
    DeviceBusy,
}

enum UserConsentVerificationResultHelper {
    Verified,
    DeviceNotPresent,
    NotConfiguredForUser,
    DisabledByPolicy,
    DeviceBusy,
    RetriesExhausted,
    Canceled,
}

struct UserConsentVerifierHelper;

impl UserConsentVerifierHelper {
    fn check_availability() -> Result<UserConsentVerifierAvailabilityHelper, String> {
        use windows::Security::Credentials::UI::UserConsentVerifier;

        let async_op = UserConsentVerifier::CheckAvailabilityAsync()
            .map_err(|e| format!("CheckAvailabilityAsync call failed: {e}"))?;

        let status = async_op
            .get()
            .map_err(|e| format!("CheckAvailabilityAsync result failed: {e}"))?;

        // UserConsentVerifierAvailability enum:
        // Available = 0, DeviceNotPresent = 1, NotConfiguredForUser = 2,
        // DisabledByPolicy = 3, DeviceBusy = 4
        match status.0 {
            0 => Ok(UserConsentVerifierAvailabilityHelper::Available),
            1 => Ok(UserConsentVerifierAvailabilityHelper::DeviceNotPresent),
            2 => Ok(UserConsentVerifierAvailabilityHelper::NotConfiguredForUser),
            3 => Ok(UserConsentVerifierAvailabilityHelper::DisabledByPolicy),
            4 => Ok(UserConsentVerifierAvailabilityHelper::DeviceBusy),
            other => Err(format!("unexpected availability status: {other}")),
        }
    }

    fn request_verification(reason: &str) -> Result<UserConsentVerificationResultHelper, String> {
        use windows::core::HSTRING;
        use windows::Security::Credentials::UI::UserConsentVerifier;

        let message = HSTRING::from(reason);

        let async_op = UserConsentVerifier::RequestVerificationAsync(&message)
            .map_err(|e| format!("RequestVerificationAsync call failed: {e}"))?;

        let result = async_op
            .get()
            .map_err(|e| format!("RequestVerificationAsync result failed: {e}"))?;

        // UserConsentVerificationResult enum:
        // Verified = 0, DeviceNotPresent = 1, NotConfiguredForUser = 2,
        // DisabledByPolicy = 3, DeviceBusy = 4, RetriesExhausted = 5, Canceled = 6
        match result.0 {
            0 => Ok(UserConsentVerificationResultHelper::Verified),
            1 => Ok(UserConsentVerificationResultHelper::DeviceNotPresent),
            2 => Ok(UserConsentVerificationResultHelper::NotConfiguredForUser),
            3 => Ok(UserConsentVerificationResultHelper::DisabledByPolicy),
            4 => Ok(UserConsentVerificationResultHelper::DeviceBusy),
            5 => Ok(UserConsentVerificationResultHelper::RetriesExhausted),
            6 => Ok(UserConsentVerificationResultHelper::Canceled),
            other => Err(format!("unexpected verification result: {other}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_storage_dpapi_roundtrip() {
        let dir =
            std::env::temp_dir().join(format!("solosoul-windows-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let storage = WindowsBiometricStorage::new(dir.clone());
        let account_id = "acc-windows";
        let key_hex = "deadbeef1234567890abcdef1234567890abcdef1234567890abcdef12345678";

        assert!(!storage.exists(account_id));
        storage.save(account_id, key_hex, "reason").unwrap();
        assert!(storage.exists(account_id));
        assert_eq!(storage.read(account_id, "reason").unwrap(), key_hex);

        // 文件必须以 DPAPI 魔数头开头（而非旧版明文派生格式）。
        let raw = std::fs::read(dir.join(account_id).join("biometric_key")).unwrap();
        assert!(raw.starts_with(DPAPI_MAGIC), "file must be DPAPI format");

        storage.update(account_id, key_hex).unwrap();
        assert_eq!(storage.read(account_id, "reason").unwrap(), key_hex);

        storage.delete(account_id).unwrap();
        assert!(!storage.exists(account_id));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_windows_storage_migrates_legacy_file() {
        let dir =
            std::env::temp_dir().join(format!("solosoul-windows-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let account_id = "acc-windows-legacy";
        let key_hex = "deadbeef1234567890abcdef1234567890abcdef1234567890abcdef12345678";

        // 先用旧版文件存储写入（模拟升级前的凭证），不带 DPAPI 魔数头。
        let legacy = FileBiometricStorage::new(dir.clone());
        legacy.save(account_id, key_hex, "reason").unwrap();
        let raw = std::fs::read(dir.join(account_id).join("biometric_key")).unwrap();
        assert!(
            !raw.starts_with(DPAPI_MAGIC),
            "legacy file must not have DPAPI magic"
        );

        // DPAPI 存储读取应透明迁移并返回相同密钥。
        let storage = WindowsBiometricStorage::new(dir.clone());
        assert_eq!(storage.read(account_id, "reason").unwrap(), key_hex);

        // 迁移后文件必须已是 DPAPI 格式。
        let migrated = std::fs::read(dir.join(account_id).join("biometric_key")).unwrap();
        assert!(
            migrated.starts_with(DPAPI_MAGIC),
            "file must be migrated to DPAPI"
        );

        // 二次读取（纯 DPAPI 路径）仍应成功。
        assert_eq!(storage.read(account_id, "reason").unwrap(), key_hex);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_windows_biometric_availability_shape() {
        // 只验证返回结构，不验证具体值（CI/无 Windows Hello 硬件的机器可能不可用）
        let (available, bt, err) = query_windows_biometric_availability();
        // available 的类型即 bool，`== false || == true` 恒真、无验证意义（clippy
        // bool_comparison），删除该恒真断言；后续分支仍按 available 值校验一致性。
        if let Some(ref bt_val) = bt {
            assert_eq!(bt_val, "windowsHello");
        }
        if let Some(ref err_msg) = err {
            assert!(!err_msg.is_empty(), "error message should not be empty");
        }
        if available {
            assert!(bt.is_some(), "available=true must have biometry_type");
            assert_eq!(bt.as_deref(), Some("windowsHello"));
        }
    }
}
