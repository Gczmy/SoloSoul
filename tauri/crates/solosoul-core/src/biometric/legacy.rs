//! 旧版基于本地加密文件的生物识别凭证存储。
//!
//! 仅用于：
//! - 测试 mock（避免在 CI/测试中使用真实 Keychain）。
//! - 用户升级后清理旧版 `biometric_key` 文件（不自动迁移，因旧方案安全性不足）。

use super::{BiometricError, BiometricStorage};
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

/// P002: 旧版 XOR 迁移路径（LEGACY_XOR_KEY / legacy_xor_decrypt / is_legacy_key_file /
/// migrate_legacy_key_file）已删除——迁移窗口已关闭（count_legacy_key_files 诊断持续为 0）。
/// 遗留的 64-hex XOR 文件读作「格式无效」，提示重新启用生物识别后重写为新格式。
/// 当前文件格式为 AES-256-GCM blob（>64 hex），与新检测逻辑天然互斥。
#[cfg(test)]
const TEST_FILE_KEY_SALT: &[u8] = b"test-only-biometric-file-key-salt";

// 注：不再使用静态密钥 BIO_FILE_KEY_SECRET。生产环境的文件加密密钥
// 通过 account_id 派生（见下方 file_encryption_key），每个账户密钥不同。
// 主要安全防护依赖于 OS 文件权限（0o600）。

pub struct FileBiometricStorage {
    base_path: PathBuf,
}

impl FileBiometricStorage {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    fn key_path(&self, account_id: &str) -> PathBuf {
        self.base_path.join(account_id).join("biometric_key")
    }
}

impl BiometricStorage for FileBiometricStorage {
    fn save(&self, account_id: &str, key_hex: &str, _reason: &str) -> Result<(), BiometricError> {
        write_encrypted_key_file(&self.key_path(account_id), account_id, key_hex)
    }

    fn update(&self, account_id: &str, key_hex: &str) -> Result<(), BiometricError> {
        write_encrypted_key_file(&self.key_path(account_id), account_id, key_hex)
    }

    fn read(&self, account_id: &str, _reason: &str) -> Result<String, BiometricError> {
        read_encrypted_key_file(&self.key_path(account_id), account_id)
    }

    fn delete(&self, account_id: &str) -> Result<(), BiometricError> {
        let path = self.key_path(account_id);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| {
                BiometricError::Other(format!("Failed to remove legacy key file: {e}"))
            })?;
        }
        let _ = std::fs::remove_file(path.with_extension("key.old"));
        Ok(())
    }

    fn exists(&self, account_id: &str) -> bool {
        self.key_path(account_id).exists()
    }

    fn uses_legacy_file(&self) -> bool {
        true
    }
}

#[cfg(not(test))]
fn file_encryption_key(account_id: &str) -> Result<Zeroizing<Vec<u8>>, BiometricError> {
    use sha2::{Digest, Sha256};
    // 移除硬编码静态密钥。用 SHA-256 将 account_id 哈希为 32 字节后
    // 通过 HKDF 派生文件加密密钥。每个账户的密钥唯一，
    // 避免单一二进制泄漏威胁所有账户。
    // 主要安全防护：OS 文件权限 0o600（仅当前用户可读写）。
    let ikm: [u8; 32] = Sha256::digest(account_id.as_bytes()).into();
    let key = solosoul_crypto::hkdf_ext::derive_hkdf_key(
        &ikm,
        b"solosoul:biometric:file",
        b"solosoul:biometric:filekey:v1",
    )
    .map_err(|e| BiometricError::Other(format!("Failed to derive file key: {e}")))?;
    Ok(Zeroizing::new(key.to_vec()))
}

#[cfg(test)]
fn file_encryption_key(account_id: &str) -> Result<Zeroizing<Vec<u8>>, BiometricError> {
    let config = solosoul_crypto::kdf::KdfConfig::development();
    let key = solosoul_crypto::kdf::derive_key(account_id, TEST_FILE_KEY_SALT, &config)
        .map_err(|_| BiometricError::Other("Failed to derive test file key".into()))?;
    Ok(key)
}

fn write_encrypted_key_file(
    path: &Path,
    account_id: &str,
    key_hex: &str,
) -> Result<(), BiometricError> {
    let file_key = file_encryption_key(account_id)?;
    let file_key_arr: [u8; 32] = file_key
        .as_slice()
        .try_into()
        .map_err(|_| BiometricError::Other("File key length invalid".into()))?;
    let blob = solosoul_crypto::aes::encrypt_blob(&file_key_arr, key_hex.as_bytes())
        .map_err(|e| BiometricError::Other(format!("Encrypt failed: {e}")))?;

    let parent = path.parent().ok_or_else(|| {
        BiometricError::Other(format!(
            "Invalid path: no parent directory for {}",
            path.display()
        ))
    })?;
    std::fs::create_dir_all(parent).map_err(|e| {
        BiometricError::Other(format!("Failed to create {}: {}", path.display(), e))
    })?;
    std::fs::write(path, hex::encode(blob.as_slice()))
        .map_err(|e| BiometricError::Other(format!("Failed to write {}: {}", path.display(), e)))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)
            .map_err(|e| {
                BiometricError::Other(format!("Failed to stat {}: {}", path.display(), e))
            })?
            .permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(path, perms).map_err(|e| {
            BiometricError::Other(format!("Failed to chmod {}: {}", path.display(), e))
        })?;
    }
    Ok(())
}

fn read_encrypted_key_file(path: &Path, account_id: &str) -> Result<String, BiometricError> {
    let content =
        std::fs::read_to_string(path).map_err(|_e| BiometricError::KeychainItemNotFound)?;
    let content = content.trim();

    let blob = hex::decode(content).map_err(|_| BiometricError::InvalidKeyFormat)?;

    let file_key = file_encryption_key(account_id)?;
    let file_key_arr: [u8; 32] = file_key
        .as_slice()
        .try_into()
        .map_err(|_| BiometricError::Other("File key length invalid".into()))?;
    let plaintext = solosoul_crypto::aes::decrypt_blob(&file_key_arr, &blob).map_err(|e| {
        BiometricError::Other(format!(
            "{e} (hint: try re-enabling biometric after password login)"
        ))
    })?;
    String::from_utf8(plaintext.to_vec()).map_err(|_| BiometricError::InvalidKeyFormat)
}
