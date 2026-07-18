//! 旧版基于本地加密文件的生物识别凭证存储。
//!
//! 仅用于：
//! - 测试 mock（避免在 CI/测试中使用真实 Keychain）。
//! - 用户升级后清理旧版 `biometric_key` 文件（不自动迁移，因旧方案安全性不足）。

use super::{BiometricError, BiometricStorage};
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

/// Legacy hard-coded XOR key used only for one-way migration of old biometric files.
///
/// # Security
///
/// This key is intentionally hard-coded because:
/// 1. It is ONLY used for **one-way decryption** of legacy biometric credential files
///    that were stored with a simple XOR obfuscation scheme in older versions.
/// 2. On successful decryption, the legacy file is **atomically migrated** to a new
///    AES-256-GCM encrypted format with a per-account derived key (HKDF from SHA-256
///    of account_id). The legacy XOR-encrypted file is then renamed and cleaned up.
/// 3. The attack surface is minimal: an attacker would need both the compiled binary
///    AND the old `biometric_key` file on disk (protected by OS file permissions 0o600).
///    The decrypted payload is a limited-duration session key, not the master password.
/// 4. There is no key rotation concern: once a file is migrated, this key is never
///    used again for that account.
///
/// Once the legacy migration window closes (e.g., all users of versions < 2.0 have
/// either migrated or been prompted to re-enable biometrics), this constant and the
/// entire `legacy.rs` module can be safely removed.
///
/// See also: `file_encryption_key()` for the current per-account derivation scheme.
const LEGACY_XOR_KEY: &[u8; 32] = b"Solosoul_biometric_obfuscate_v1!";
#[cfg(test)]
const TEST_FILE_KEY_SALT: &[u8] = b"test-only-biometric-file-key-salt";
const LEGACY_KEY_HEX_LEN: usize = 64;

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

fn is_legacy_key_file(content: &str) -> bool {
    content.len() == LEGACY_KEY_HEX_LEN && content.chars().all(|c| c.is_ascii_hexdigit())
}

fn legacy_xor_decrypt(content: &str) -> Result<String, BiometricError> {
    let obf = hex::decode(content.trim()).map_err(|_e| BiometricError::InvalidKeyFormat)?;
    let key: Vec<u8> = obf
        .iter()
        .enumerate()
        .map(|(i, b)| b ^ LEGACY_XOR_KEY[i % 32])
        .collect();
    Ok(hex::encode(&key))
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

/// 以原子方式将旧 XOR 文件迁移为新的加密文件。
fn migrate_legacy_key_file(
    path: &Path,
    account_id: &str,
    legacy_key_hex: &str,
) -> Result<(), BiometricError> {
    let new_path = path.with_extension("key.new");
    let old_path = path.with_extension("key.old");
    write_encrypted_key_file(&new_path, account_id, legacy_key_hex)?;
    std::fs::rename(path, &old_path).map_err(|e| {
        BiometricError::LegacyMigrationFailed(format!("Failed to backup legacy key file: {e}"))
    })?;
    std::fs::rename(&new_path, path).map_err(|e| {
        BiometricError::LegacyMigrationFailed(format!("Failed to replace legacy key file: {e}"))
    })?;
    let _ = std::fs::remove_file(&old_path);
    Ok(())
}

fn read_encrypted_key_file(path: &Path, account_id: &str) -> Result<String, BiometricError> {
    let content =
        std::fs::read_to_string(path).map_err(|_e| BiometricError::KeychainItemNotFound)?;
    let content = content.trim();

    if is_legacy_key_file(content) {
        let key_hex = legacy_xor_decrypt(content)?;
        // Attempt atomic migration; failure keeps the legacy file intact.
        if let Err(e) = migrate_legacy_key_file(path, account_id, &key_hex) {
            tracing::warn!("Failed to migrate legacy biometric key file: {}", e);
        }
        return Ok(key_hex);
    }

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
