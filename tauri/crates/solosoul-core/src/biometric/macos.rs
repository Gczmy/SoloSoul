//! macOS 生物识别凭证存储实现（当前使用方案）。
//!
//! 由于团队暂未付费加入 Apple Developer Program，无法获得有效的
//! `keychain-access-groups` entitlement，因此当前 macOS 生物识别不使用
//! Keychain，而是使用本地加密文件存储主密钥。
//!
//! 为了仍然提供生物识别体验：
//! - 保存凭证前由 `BiometricManager::save_credential` 调用系统生物识别弹窗。
//! - 解锁前由 `BiometricManager::unlock` 调用系统生物识别弹窗。
//! - 验证通过后，才读取本地文件中的密钥并解锁 Vault。
//!
//! 未来若加入 Apple Developer Program 并启用 Keychain 方案，请改用
//! `macos_keychain.rs` 中的实现（详见该文件顶部注释）。

use super::{BiometricError, BiometricStorage};
use crate::biometric::legacy::FileBiometricStorage;
use std::path::PathBuf;

pub struct MacOsBiometricStorage {
    file_storage: FileBiometricStorage,
}

impl MacOsBiometricStorage {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            file_storage: FileBiometricStorage::new(base_path),
        }
    }
}

impl BiometricStorage for MacOsBiometricStorage {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_macos_storage_delegates_to_file_storage() {
        let dir = PathBuf::from(std::env::temp_dir())
            .join(format!("solosoul-macos-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let storage = MacOsBiometricStorage::new(dir.clone());
        let account_id = "acc-macos";
        let key_hex = "deadbeef1234567890abcdef1234567890abcdef1234567890abcdef12345678";

        assert!(!storage.exists(account_id));
        storage.save(account_id, key_hex, "reason").unwrap();
        assert!(storage.exists(account_id));
        assert_eq!(storage.read(account_id, "reason").unwrap(), key_hex);
        storage.delete(account_id).unwrap();
        assert!(!storage.exists(account_id));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
