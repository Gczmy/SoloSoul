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
