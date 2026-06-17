//! macOS 生物识别凭证存储实现。
//!
//! 使用 Security.framework 的 Keychain Services，将主密钥保存为
//! `kSecAccessControlUserPresence` 约束的 Generic Password Item。
//! 读取时系统自动弹出 Touch ID / 设备密码验证框；`is_configured` 以本地
//! `biometricEnabled` 配置标记为准，避免在应用启动时访问 Keychain 弹框。
//!
//! 开发环境兜底：若 Keychain 返回 `-34018`（缺少 entitlement，未签名构建），
//! 自动回退到旧版本地加密文件存储，并在日志中输出警告。Release 包带
//! `keychain-access-groups` entitlement 时会正常走 Keychain。

use super::{BiometricError, BiometricStorage};
use crate::biometric::legacy::FileBiometricStorage;
use core_foundation::base::{CFType, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::data::{CFData, CFDataRef};
use core_foundation::dictionary::CFMutableDictionary;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use core_foundation_sys::base::{CFRelease, CFTypeRef, OSStatus};
use security_framework::access_control::{ProtectionMode, SecAccessControl};
use security_framework::base::Error as SecError;
use security_framework::passwords_options::AccessControlOptions;
use security_framework_sys::base::{errSecAuthFailed, errSecDuplicateItem, errSecItemNotFound};
use security_framework_sys::item::{
    kSecAttrAccessControl, kSecAttrAccount, kSecAttrService, kSecClass, kSecClassGenericPassword,
    kSecMatchLimit, kSecReturnAttributes, kSecReturnData, kSecUseAuthenticationUI, kSecValueData,
};
use security_framework_sys::keychain_item::{
    SecItemAdd, SecItemCopyMatching, SecItemDelete, SecItemUpdate,
};
use std::path::PathBuf;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};

const SERVICE: &str = "com.solosoul.biometric";
const KEYCHAIN_PROMPT_KEY: &str = "u_OpPrompt";
const KEYCHAIN_UI_FAIL_VALUE: &str = "u_AuthUIF";
const ERR_SEC_USER_CANCELED: OSStatus = -128;
const ERR_SEC_MISSING_ENTITLEMENT: OSStatus = -34018;

fn account_key(account_id: &str) -> String {
    format!("solosoul.biometric.{account_id}")
}

pub struct MacOsBiometricStorage {
    base_path: PathBuf,
    /// 开发环境是否已回退到本地文件存储（缺少 Keychain entitlement 时）。
    used_fallback: AtomicBool,
}

impl MacOsBiometricStorage {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            used_fallback: AtomicBool::new(false),
        }
    }

    fn fallback_storage(&self) -> FileBiometricStorage {
        FileBiometricStorage::new(self.base_path.clone())
    }

    fn access_control() -> Result<SecAccessControl, BiometricError> {
        SecAccessControl::create_with_protection(
            Some(ProtectionMode::AccessibleWhenUnlockedThisDeviceOnly),
            AccessControlOptions::USER_PRESENCE.bits(),
        )
        .map_err(|_e| BiometricError::UserPresenceUnavailable)
    }

    /// 通用查询：class / service / account，用于 match、update、delete。
    fn base_query(account_id: &str) -> CFMutableDictionary<CFString, CFType> {
        let mut dict = CFMutableDictionary::<CFString, CFType>::new();

        let class_key = unsafe { CFString::wrap_under_get_rule(kSecClass) };
        let class_val: CFType =
            unsafe { CFString::wrap_under_get_rule(kSecClassGenericPassword) }.into_CFType();
        dict.add(&class_key, &class_val);

        let service_key = unsafe { CFString::wrap_under_get_rule(kSecAttrService) };
        let service_val: CFType = CFString::from(SERVICE).into_CFType();
        dict.add(&service_key, &service_val);

        let account_key_cf = unsafe { CFString::wrap_under_get_rule(kSecAttrAccount) };
        let account_val: CFType = CFString::from(account_key(account_id).as_str()).into_CFType();
        dict.add(&account_key_cf, &account_val);

        dict
    }

    /// 用于新增的字典：base + access control + value data。
    fn add_query(
        account_id: &str,
        key_hex: &str,
    ) -> Result<CFMutableDictionary<CFString, CFType>, BiometricError> {
        let mut dict = Self::base_query(account_id);

        let access_control = Self::access_control()?;
        let access_key = unsafe { CFString::wrap_under_get_rule(kSecAttrAccessControl) };
        let access_val: CFType = access_control.into_CFType();
        dict.add(&access_key, &access_val);

        let data_key = unsafe { CFString::wrap_under_get_rule(kSecValueData) };
        let data_val: CFType = CFData::from_buffer(key_hex.as_bytes()).into_CFType();
        dict.add(&data_key, &data_val);

        Ok(dict)
    }

    /// 构造用于读取的原始查询字典，包含 `kSecUseOperationPrompt` 以显示自定义原因。
    fn read_query(account_id: &str, reason: &str) -> CFMutableDictionary<CFString, CFType> {
        let mut dict = Self::base_query(account_id);

        let return_data_key = unsafe { CFString::wrap_under_get_rule(kSecReturnData) };
        let true_val: CFType = CFBoolean::from(true).into_CFType();
        dict.add(&return_data_key, &true_val);

        let limit_key = unsafe { CFString::wrap_under_get_rule(kSecMatchLimit) };
        let limit_val: CFType = CFNumber::from(1i64).into_CFType();
        dict.add(&limit_key, &limit_val);

        let prompt_key = CFString::from_static_string(KEYCHAIN_PROMPT_KEY);
        let prompt_val: CFType = CFString::from(reason).into_CFType();
        dict.add(&prompt_key, &prompt_val);

        dict
    }
}

impl BiometricStorage for MacOsBiometricStorage {
    fn save(&self, account_id: &str, key_hex: &str, reason: &str) -> Result<(), BiometricError> {
        // 先删除可能存在的旧项，避免 access control 参与 match 导致 SecItemUpdate 失败。
        match self.delete(account_id) {
            Ok(()) | Err(BiometricError::KeychainItemNotFound) => {}
            Err(BiometricError::MissingKeychainEntitlement) => {
                // 当前构建缺少 Keychain entitlement，直接走文件兜底。
            }
            Err(e) => return Err(e),
        }

        let dict = Self::add_query(account_id, key_hex)?;
        let params = dict.to_immutable();
        let status = unsafe {
            SecItemAdd(
                params.as_concrete_TypeRef(),
                ptr::null_mut() as *mut CFTypeRef,
            )
        };

        if status == ERR_SEC_MISSING_ENTITLEMENT {
            tracing::warn!(
                "macOS Keychain entitlement missing for account_id={}; \
                 falling back to file-based biometric storage (dev build).",
                account_id
            );
            self.used_fallback.store(true, Ordering::SeqCst);
            return self.fallback_storage().save(account_id, key_hex, reason);
        }

        if status == errSecDuplicateItem {
            return self.update(account_id, key_hex);
        }

        if let Err(e) = check_status(status) {
            tracing::error!(
                "SecItemAdd failed for account_id={}: code={} ({:?})",
                account_id,
                e.code(),
                e
            );
            return Err(map_write_err(e));
        }
        Ok(())
    }

    fn update(&self, account_id: &str, key_hex: &str) -> Result<(), BiometricError> {
        let query = Self::base_query(account_id).to_immutable();
        let mut update = CFMutableDictionary::<CFString, CFType>::new();
        let data_key = unsafe { CFString::wrap_under_get_rule(kSecValueData) };
        let data_val: CFType = CFData::from_buffer(key_hex.as_bytes()).into_CFType();
        update.add(&data_key, &data_val);

        let status = unsafe {
            SecItemUpdate(
                query.as_concrete_TypeRef(),
                update.to_immutable().as_concrete_TypeRef(),
            )
        };

        if status == ERR_SEC_MISSING_ENTITLEMENT {
            tracing::warn!(
                "macOS Keychain entitlement missing for account_id={}; \
                 falling back to file-based biometric storage (dev build).",
                account_id
            );
            self.used_fallback.store(true, Ordering::SeqCst);
            return self.fallback_storage().update(account_id, key_hex);
        }

        if status == errSecItemNotFound {
            return Err(BiometricError::KeychainItemNotFound);
        }

        if let Err(e) = check_status(status) {
            tracing::error!(
                "SecItemUpdate failed for account_id={}: code={} ({:?})",
                account_id,
                e.code(),
                e
            );
            return Err(map_write_err(e));
        }
        Ok(())
    }

    fn read(&self, account_id: &str, reason: &str) -> Result<String, BiometricError> {
        let params = Self::read_query(account_id, reason).to_immutable();
        let mut result: CFTypeRef = ptr::null();
        let status =
            unsafe { SecItemCopyMatching(params.as_concrete_TypeRef(), &mut result as *mut _) };

        if status == ERR_SEC_MISSING_ENTITLEMENT {
            tracing::warn!(
                "macOS Keychain entitlement missing for account_id={}; \
                 falling back to file-based biometric storage (dev build).",
                account_id
            );
            self.used_fallback.store(true, Ordering::SeqCst);
            return self.fallback_storage().read(account_id, reason);
        }

        if status == errSecItemNotFound {
            return Err(BiometricError::KeychainItemNotFound);
        }
        if status == ERR_SEC_USER_CANCELED {
            return Err(BiometricError::UserPresenceCancelled);
        }
        if status != 0 {
            tracing::error!(
                "SecItemCopyMatching failed for account_id={}: code={}",
                account_id,
                status
            );
            return Err(BiometricError::KeychainReadFailed(format!(
                "status={status}"
            )));
        }
        if result.is_null() {
            return Err(BiometricError::KeychainItemNotFound);
        }

        let data = unsafe { CFData::wrap_under_create_rule(result as CFDataRef) };
        let bytes = data.bytes();
        String::from_utf8(bytes.to_vec()).map_err(|_| BiometricError::InvalidKeyFormat)
    }

    fn delete(&self, account_id: &str) -> Result<(), BiometricError> {
        let query = Self::base_query(account_id).to_immutable();
        let status = unsafe { SecItemDelete(query.as_concrete_TypeRef()) };

        if status == ERR_SEC_MISSING_ENTITLEMENT {
            tracing::warn!(
                "macOS Keychain entitlement missing for account_id={}; \
                 falling back to file-based biometric storage (dev build).",
                account_id
            );
            self.used_fallback.store(true, Ordering::SeqCst);
            return self.fallback_storage().delete(account_id);
        }

        if status == errSecItemNotFound {
            return Err(BiometricError::KeychainItemNotFound);
        }

        if let Err(e) = check_status(status) {
            tracing::error!(
                "SecItemDelete failed for account_id={}: code={} ({:?})",
                account_id,
                e.code(),
                e
            );
            return Err(map_write_err(e));
        }
        Ok(())
    }

    fn exists(&self, account_id: &str) -> bool {
        let mut dict = Self::base_query(account_id);

        let return_attrs_key = unsafe { CFString::wrap_under_get_rule(kSecReturnAttributes) };
        let true_val: CFType = CFBoolean::from(true).into_CFType();
        dict.add(&return_attrs_key, &true_val);

        let limit_key = unsafe { CFString::wrap_under_get_rule(kSecMatchLimit) };
        let limit_val: CFType = CFNumber::from(1i64).into_CFType();
        dict.add(&limit_key, &limit_val);

        let ui_key = unsafe { CFString::wrap_under_get_rule(kSecUseAuthenticationUI) };
        let ui_val: CFType = CFString::from_static_string(KEYCHAIN_UI_FAIL_VALUE).into_CFType();
        dict.add(&ui_key, &ui_val);

        let params = dict.to_immutable();
        let mut result: CFTypeRef = ptr::null();
        let status =
            unsafe { SecItemCopyMatching(params.as_concrete_TypeRef(), &mut result as *mut _) };

        if !result.is_null() {
            unsafe { CFRelease(result) };
        }

        if status == ERR_SEC_MISSING_ENTITLEMENT {
            return self.fallback_storage().exists(account_id);
        }

        // 项存在但需要认证时会返回 user-canceled / auth-failed；只要不是"未找到"就认为存在。
        status != errSecItemNotFound
    }

    fn uses_legacy_file(&self) -> bool {
        self.used_fallback.load(Ordering::SeqCst)
    }
}

fn check_status(status: OSStatus) -> Result<(), SecError> {
    if status == 0 {
        Ok(())
    } else {
        Err(SecError::from_code(status))
    }
}

fn map_write_err(e: SecError) -> BiometricError {
    let code = e.code();
    if code == ERR_SEC_USER_CANCELED {
        BiometricError::UserPresenceCancelled
    } else if code == errSecAuthFailed {
        BiometricError::UserPresenceUnavailable
    } else if code == ERR_SEC_MISSING_ENTITLEMENT {
        BiometricError::MissingKeychainEntitlement
    } else {
        BiometricError::KeychainWriteFailed(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_key_is_stable() {
        assert_eq!(account_key("acc-123"), "solosoul.biometric.acc-123");
    }

    #[test]
    fn test_read_missing_returns_not_found() {
        let base = PathBuf::from("/tmp");
        let storage = MacOsBiometricStorage::new(base);
        let account_id = format!("test-macos-bio-{}", uuid::Uuid::new_v4());
        // 未配置时应直接返回 KeychainItemNotFound，不应弹出 Touch ID 提示框。
        let err = storage
            .read(&account_id, "test prompt")
            .expect_err("should fail for missing item");
        assert!(matches!(err, BiometricError::KeychainItemNotFound));
    }

    #[test]
    fn test_exists_missing_without_prompt() {
        let base = PathBuf::from("/tmp");
        let storage = MacOsBiometricStorage::new(base);
        let account_id = format!("test-macos-bio-{}", uuid::Uuid::new_v4());
        // 未配置时不应弹框；返回 false。
        assert!(!storage.exists(&account_id));
    }

    #[test]
    fn test_delete_missing_returns_not_found() {
        let base = PathBuf::from("/tmp");
        let storage = MacOsBiometricStorage::new(base);
        let account_id = format!("test-macos-bio-{}", uuid::Uuid::new_v4());
        let err = storage
            .delete(&account_id)
            .expect_err("should fail for missing item");
        assert!(matches!(err, BiometricError::KeychainItemNotFound));
    }
}
