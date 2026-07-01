//! Windows Hello 生物识别凭证存储与验证实现。
//!
//! 实现策略：
//! - 凭证存储复用 `legacy::FileBiometricStorage`（本地加密文件），
//!   避免依赖 Windows Credential Manager 导致弹窗和锁定问题。
//! - 可用性检测使用 `UserConsentVerifier::CheckAvailabilityAsync()`。
//! - 用户验证使用 `UserConsentVerifier::RequestVerificationAsync()`。
//! - Windows Hello 始终允许 PIN 作为用户验证回退，因此 strict/strict=false
//!   在此平台上行为相同（已验证 = 成功，其他 = 失败）。

use super::{BiometricError, BiometricStorage};
use crate::biometric::legacy::FileBiometricStorage;
use std::path::PathBuf;
use std::sync::OnceLock;

pub struct WindowsBiometricStorage {
    file_storage: FileBiometricStorage,
}

impl WindowsBiometricStorage {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            file_storage: FileBiometricStorage::new(base_path),
        }
    }
}

impl BiometricStorage for WindowsBiometricStorage {
    fn save(&self, account_id: &str, key_hex: &str, reason: &str) -> Result<(), BiometricError> {
        self.file_storage.save(account_id, key_hex, reason)
    }

    fn update(&self, account_id: &str, key_hex: &str) -> Result<(), BiometricError> {
        self.file_storage.update(account_id, key_hex)
    }

    fn read(&self, account_id: &str, reason: &str) -> Result<String, BiometricError> {
        self.file_storage.read(account_id, reason)
    }

    fn delete(&self, account_id: &str) -> Result<(), BiometricError> {
        self.file_storage.delete(account_id)
    }

    fn exists(&self, account_id: &str) -> bool {
        self.file_storage.exists(account_id)
    }

    fn uses_legacy_file(&self) -> bool {
        true
    }
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
    if let Err(e) = ensure_mta() {
        return Err(e);
    }

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
    fn test_windows_storage_delegates_to_file_storage() {
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
        storage.delete(account_id).unwrap();
        assert!(!storage.exists(account_id));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_windows_biometric_availability_shape() {
        // 只验证返回结构，不验证具体值（CI/无 Windows Hello 硬件的机器可能不可用）
        let (available, bt, err) = query_windows_biometric_availability();
        assert!(available == false || available == true);
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
