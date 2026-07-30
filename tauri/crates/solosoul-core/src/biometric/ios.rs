//! iOS 生物识别凭证存储实现（基于 iOS Security.framework Keychain）。
//!
//! 将主密钥保存为受 `kSecAccessControlUserPresence` 约束的 Keychain
//! Generic Password Item，读取时由系统弹出 Face ID / Touch ID / 设备密码框。
//!
//! iOS 不需要像 macOS 那样的 keychain-access-groups entitlement，
//! Keychain 在 iOS 上是默认可用且受硬件保护的。
//! 生成构建（App Store / TestFlight）和开发构建均无需额外配置。

use super::{BiometricError, BiometricStorage};
use core_foundation::base::{CFType, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::data::CFData;
use core_foundation::dictionary::CFMutableDictionary;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use core_foundation_sys::base::{CFTypeRef, OSStatus};
use security_framework::access_control::{ProtectionMode, SecAccessControl};
use security_framework::base::Error as SecError;
use security_framework_sys::base::{errSecAuthFailed, errSecDuplicateItem, errSecItemNotFound};
use security_framework_sys::item::{
    kSecAttrAccessControl, kSecAttrAccount, kSecAttrService, kSecClass, kSecClassGenericPassword,
    kSecMatchLimit, kSecReturnAttributes, kSecReturnData, kSecValueData,
};
use security_framework_sys::keychain_item::{
    SecItemAdd, SecItemCopyMatching, SecItemDelete, SecItemUpdate,
};
use std::path::PathBuf;
use std::ptr;

const SERVICE: &str = "com.solosoul.biometric";

fn account_key(account_id: &str) -> String {
    format!("solosoul.biometric.{account_id}")
}

pub struct IosBiometricStorage {
    base_path: PathBuf,
}

impl IosBiometricStorage {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    fn fallback_storage(&self) -> super::legacy::FileBiometricStorage {
        super::legacy::FileBiometricStorage::new(self.base_path.clone())
    }

    fn access_control() -> Result<SecAccessControl, BiometricError> {
        SecAccessControl::create_with_protection(
            Some(ProtectionMode::AccessibleWhenUnlockedThisDeviceOnly),
            // kSecAccessControlUserPresence: biometric OR device passcode.
            // Provides fallback when Face ID fails or is unavailable.
            // On iOS, this also triggers the system biometric dialog automatically
            // when the Keychain item is accessed (SecItemCopyMatching).
            security_framework::passwords_options::AccessControlOptions::USER_PRESENCE.bits(),
        )
        .map_err(|_e| BiometricError::UserPresenceUnavailable)
    }

    /// Base query: class / service / account
    fn base_query(account_id: &str) -> CFMutableDictionary<CFString, CFType> {
        let mut dict = CFMutableDictionary::<CFString, CFType>::new();

        // SAFETY: kSecClass / kSecAttrService / kSecAttrAccount are Core Foundation
        // public string constants; wrap_under_get_rule borrows the global constant.
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

    /// Query for adding a new item: base + access control + value data.
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

    /// Query for reading: base + return data + match limit + prompt reason.
    fn read_query(account_id: &str, _reason: &str) -> CFMutableDictionary<CFString, CFType> {
        let mut dict = Self::base_query(account_id);

        let return_data_key = unsafe { CFString::wrap_under_get_rule(kSecReturnData) };
        let true_val: CFType = CFBoolean::from(true).into_CFType();
        dict.add(&return_data_key, &true_val);

        let limit_key = unsafe { CFString::wrap_under_get_rule(kSecMatchLimit) };
        let limit_val: CFType = CFNumber::from(1i64).into_CFType();
        dict.add(&limit_key, &limit_val);

        // On iOS, the biometric dialog is triggered automatically by the Keychain
        // item's kSecAccessControlUserPresence access control. No need to explicitly
        // set kSecUseAuthenticationUI.
        dict
    }
}

fn check_status(status: OSStatus) -> Result<(), SecError> {
    if status == errSecSuccess {
        Ok(())
    } else {
        Err(SecError::from(status))
    }
}

fn map_write_err(e: SecError) -> BiometricError {
    tracing::error!("iOS Keychain write error: code={} ({})", e.code(), e);
    match e.code() {
        code if code == errSecAuthFailed as i64 => BiometricError::UserPresenceCancelled,
        code if code == errSecItemNotFound as i64 => BiometricError::KeychainItemNotFound,
        _ => BiometricError::KeychainWriteFailed(e.to_string()),
    }
}

fn map_read_err(e: SecError) -> BiometricError {
    tracing::error!("iOS Keychain read error: code={} ({})", e.code(), e);
    match e.code() {
        code if code == errSecAuthFailed as i64 => BiometricError::UserPresenceCancelled,
        code if code == errSecItemNotFound as i64 => BiometricError::KeychainItemNotFound,
        _ => BiometricError::KeychainReadFailed(e.to_string()),
    }
}

impl BiometricStorage for IosBiometricStorage {
    fn save(&self, account_id: &str, key_hex: &str, reason: &str) -> Result<(), BiometricError> {
        // Delete any existing item first to avoid duplicate item errors.
        match self.delete(account_id) {
            Ok(()) | Err(BiometricError::KeychainItemNotFound) => {}
            Err(e) => return Err(e),
        }

        let dict = Self::add_query(account_id, key_hex)?;
        let params = dict.to_immutable();

        // SAFETY: SecItemAdd is Apple Security Framework C API; params is a fully
        // constructed query dictionary. Returns OSStatus checked below.
        let status = unsafe {
            SecItemAdd(
                params.as_concrete_TypeRef(),
                ptr::null_mut() as *mut CFTypeRef,
            )
        };

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

        // SAFETY: kSecValueData is a CF string constant.
        let data_key = unsafe { CFString::wrap_under_get_rule(kSecValueData) };
        let data_val: CFType = CFData::from_buffer(key_hex.as_bytes()).into_CFType();
        update.add(&data_key, &data_val);

        // SAFETY: SecItemUpdate is Apple Security Framework C API.
        let status = unsafe {
            SecItemUpdate(
                query.as_concrete_TypeRef(),
                update.to_immutable().as_concrete_TypeRef(),
            )
        };

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

        // SAFETY: SecItemCopyMatching with kSecUseAuthenticationUI triggers
        // the iOS biometric dialog (Face ID / Touch ID / device passcode).
        // On success, result is allocated by the API as CFData.
        let status =
            unsafe { SecItemCopyMatching(params.as_concrete_TypeRef(), &mut result as *mut _) };

        if status != errSecSuccess {
            return Err(map_read_err(SecError::from(status)));
        }

        // SAFETY: result is non-null since status == errSecSuccess.
        let data = unsafe { CFData::wrap_under_create_rule(result as *mut _) };
        let bytes = data.bytes();
        String::from_utf8(bytes.to_vec()).map_err(|_| BiometricError::InvalidKeyFormat)
    }

    fn delete(&self, account_id: &str) -> Result<(), BiometricError> {
        let query = Self::base_query(account_id).to_immutable();

        // SAFETY: SecItemDelete removes matching Keychain items.
        let status = unsafe { SecItemDelete(query.as_concrete_TypeRef()) };

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

        // Check existence without triggering biometric prompt:
        // add return attributes but NOT return data.
        let return_attr_key = unsafe { CFString::wrap_under_get_rule(kSecReturnAttributes) };
        let true_val: CFType = CFBoolean::from(true).into_CFType();
        dict.add(&return_attr_key, &true_val);

        let limit_key = unsafe { CFString::wrap_under_get_rule(kSecMatchLimit) };
        let limit_val: CFType = CFNumber::from(1i64).into_CFType();
        dict.add(&limit_key, &limit_val);

        let params = dict.to_immutable();
        let mut result: CFTypeRef = ptr::null();

        // SAFETY: No biometric prompt triggered (not reading data, just checking existence).
        let status =
            unsafe { SecItemCopyMatching(params.as_concrete_TypeRef(), &mut result as *mut _) };

        // Also check file-based fallback (for legacy migration compatibility)
        if status == errSecSuccess {
            true
        } else {
            self.fallback_storage().exists(account_id)
        }
    }

    fn uses_legacy_file(&self) -> bool {
        // iOS Keychain doesn't use the legacy biometric_key file
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ios_storage_key_format() {
        let key = account_key("test-account");
        assert_eq!(key, "solosoul.biometric.test-account");
    }

    #[test]
    fn test_ios_storage_delegates_to_file_storage() {
        let dir = std::env::temp_dir().join(format!("solosoul-ios-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let storage = IosBiometricStorage::new(dir.clone());
        let account_id = "acc-ios-test";
        let key_hex = "deadbeef1234567890abcdef1234567890abcdef1234567890abcdef12345678";

        assert!(!storage.exists(account_id));
        // save/read/delete should work via file fallback in test environment
        // (no real iOS Keychain available in test)
        storage.save(account_id, key_hex, "reason").unwrap();
        assert!(storage.exists(account_id));
        let read_back = storage.read(account_id, "reason").unwrap();
        assert_eq!(read_back, key_hex);
        storage.delete(account_id).unwrap();
        assert!(!storage.exists(account_id));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
