//! Vault 数据层透明加密
//!
//! 使用 `solosoul_crypto::aes::encrypt_blob/decrypt_blob`（SOLO v2 格式）对敏感字段进行
//! AES-256-GCM 认证加密。该层对命令层和前端完全透明：VaultStore 在写入前自动加密、
//! 读取后自动解密，并兼容旧版本明文数据。

use base64::Engine as _;
use solosoul_crypto::aes::{decrypt_blob, encrypt_blob};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// 数据加密密钥（32 字节 AES-256 密钥）。
/// 包装 `[u8; 32]` 并实现自动安全擦除。
#[derive(Clone, Debug)]
pub struct DataEncryptionKey(pub [u8; 32]);

impl Zeroize for DataEncryptionKey {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl ZeroizeOnDrop for DataEncryptionKey {}

impl DataEncryptionKey {
    pub fn new(key: [u8; 32]) -> Self {
        Self(key)
    }
}

/// 文本列加密后的前缀。旧明文数据以 `{` / `[` / `"` 等字符开头，不会误判。
pub const ENCRYPTED_TEXT_PREFIX: &str = "solo:";

/// 加密任意二进制字段，输出 SOLO v2 blob。
pub fn encrypt_field(key: &DataEncryptionKey, plaintext: &[u8]) -> Result<Vec<u8>, String> {
    if plaintext.is_empty() {
        return Ok(Vec::new());
    }
    encrypt_blob(&key.0, plaintext).map(|z| z.to_vec())
}

/// 解密字段。如果数据不是 SOLO blob（旧版本明文），直接返回原数据。
pub fn decrypt_field(key: &DataEncryptionKey, ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    if ciphertext.is_empty() {
        return Ok(Vec::new());
    }
    if is_encrypted_blob(ciphertext) {
        decrypt_blob(&key.0, ciphertext).map(|z| z.to_vec())
    } else {
        Ok(ciphertext.to_vec())
    }
}

/// 检查字节串是否为已加密的 SOLO blob。
pub fn is_encrypted_blob(data: &[u8]) -> bool {
    data.len() >= 5 && data[0..4] == solosoul_crypto::aes::BLOB_MAGIC
}

/// 加密文本列：SOLO blob → base64 → `"solo:" + base64。
/// 这样可以在不修改 SQLite 列类型的情况下存储加密数据。
pub fn encrypt_text_field(key: &DataEncryptionKey, plaintext: &str) -> Result<String, String> {
    if plaintext.is_empty() {
        return Ok(String::new());
    }
    // 若已经是加密文本，避免重复加密
    if plaintext.starts_with(ENCRYPTED_TEXT_PREFIX) {
        return Ok(plaintext.to_string());
    }
    let blob = encrypt_field(key, plaintext.as_bytes())?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&blob);
    Ok(format!("{}{}", ENCRYPTED_TEXT_PREFIX, b64))
}

/// 解密文本列。若不是 `"solo:"` 前缀，视为明文直接返回。
pub fn decrypt_text_field(key: &DataEncryptionKey, ciphertext: &str) -> Result<String, String> {
    if ciphertext.is_empty() {
        return Ok(String::new());
    }
    if let Some(b64) = ciphertext.strip_prefix(ENCRYPTED_TEXT_PREFIX) {
        let blob = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| format!("Base64 decode failed: {}", e))?;
        let plain = decrypt_field(key, &blob)?;
        String::from_utf8(plain).map_err(|e| format!("UTF-8 decode failed: {}", e))
    } else {
        Ok(ciphertext.to_string())
    }
}

/// 便捷函数：将可能为旧明文的文本字段在写入前转为加密格式。
/// 如果已经是加密格式则直接返回。
pub fn ensure_encrypted_text(key: &DataEncryptionKey, value: &str) -> Result<String, String> {
    if value.starts_with(ENCRYPTED_TEXT_PREFIX) || value.is_empty() {
        Ok(value.to_string())
    } else {
        encrypt_text_field(key, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> DataEncryptionKey {
        DataEncryptionKey([0x42u8; 32])
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = test_key();
        let plain = b"Hello, SoloSoul!";
        let blob = encrypt_field(&key, plain).unwrap();
        assert!(is_encrypted_blob(&blob));
        let decrypted = decrypt_field(&key, &blob).unwrap();
        assert_eq!(decrypted, plain);
    }

    #[test]
    fn test_decrypt_plaintext_compatible() {
        let key = test_key();
        let plain = b"legacy plaintext";
        let decrypted = decrypt_field(&key, plain).unwrap();
        assert_eq!(decrypted, plain);
    }

    #[test]
    fn test_text_field_roundtrip() {
        let key = test_key();
        let plain = r#"{"secret": "value"}"#;
        let encrypted = encrypt_text_field(&key, plain).unwrap();
        assert!(encrypted.starts_with(ENCRYPTED_TEXT_PREFIX));
        let decrypted = decrypt_text_field(&key, &encrypted).unwrap();
        assert_eq!(decrypted, plain);
    }

    #[test]
    fn test_text_field_plaintext_compatible() {
        let key = test_key();
        let plain = "legacy text";
        let decrypted = decrypt_text_field(&key, plain).unwrap();
        assert_eq!(decrypted, plain);
    }

    #[test]
    fn test_empty_data_roundtrip() {
        let key = test_key();
        assert!(encrypt_field(&key, b"").unwrap().is_empty());
        assert!(decrypt_field(&key, b"").unwrap().is_empty());
        assert!(encrypt_text_field(&key, "").unwrap().is_empty());
        assert!(decrypt_text_field(&key, "").unwrap().is_empty());
    }

    #[test]
    fn test_idempotent_text_encryption() {
        let key = test_key();
        let plain = "sensitive";
        let once = encrypt_text_field(&key, plain).unwrap();
        let twice = encrypt_text_field(&key, &once).unwrap();
        assert_eq!(once, twice);
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = DataEncryptionKey([0u8; 32]);
        let key2 = DataEncryptionKey([1u8; 32]);
        let blob = encrypt_field(&key1, b"secret").unwrap();
        assert!(decrypt_field(&key2, &blob).is_err());
    }
}
