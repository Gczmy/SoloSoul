//! 非 macOS 平台的占位生物识别存储实现。

use super::{BiometricError, BiometricStorage};

pub struct StubBiometricStorage;

impl BiometricStorage for StubBiometricStorage {
    fn save(&self, _account_id: &str, _key_hex: &str, _reason: &str) -> Result<(), BiometricError> {
        Err(BiometricError::PlatformNotSupported)
    }

    fn update(&self, _account_id: &str, _key_hex: &str) -> Result<(), BiometricError> {
        Err(BiometricError::PlatformNotSupported)
    }

    fn read(&self, _account_id: &str, _reason: &str) -> Result<String, BiometricError> {
        Err(BiometricError::PlatformNotSupported)
    }

    fn delete(&self, _account_id: &str) -> Result<(), BiometricError> {
        Err(BiometricError::PlatformNotSupported)
    }

    fn exists(&self, _account_id: &str) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stub_save_returns_platform_not_supported() {
        let stub = StubBiometricStorage;
        let result = stub.save("acc1", "deadbeef", "test");
        assert!(matches!(result, Err(BiometricError::PlatformNotSupported)));
    }

    #[test]
    fn test_stub_update_returns_platform_not_supported() {
        let stub = StubBiometricStorage;
        let result = stub.update("acc1", "deadbeef");
        assert!(matches!(result, Err(BiometricError::PlatformNotSupported)));
    }

    #[test]
    fn test_stub_read_returns_platform_not_supported() {
        let stub = StubBiometricStorage;
        let result = stub.read("acc1", "test");
        assert!(matches!(result, Err(BiometricError::PlatformNotSupported)));
    }

    #[test]
    fn test_stub_delete_returns_platform_not_supported() {
        let stub = StubBiometricStorage;
        let result = stub.delete("acc1");
        assert!(matches!(result, Err(BiometricError::PlatformNotSupported)));
    }

    #[test]
    fn test_stub_exists_returns_false() {
        let stub = StubBiometricStorage;
        assert!(!stub.exists("acc1"));
        assert!(!stub.exists("any_account_id"));
    }
}
