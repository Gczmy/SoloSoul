//! AES-256-GCM blob encryption (SOLO format)
//!
//! Provides authenticated encryption with AES-256-GCM in SOLO blob format:
//! v2: Magic(4) + Version(1) + Nonce(12) + Ciphertext+Tag(16)

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use zeroize::Zeroizing;

use crate::cipher::CipherError;

pub const NONCE_SIZE: usize = 12;
pub const KEY_SIZE: usize = 32;
pub const TAG_SIZE: usize = 16;

pub const BLOB_MAGIC: [u8; 4] = [0x53, 0x4F, 0x4C, 0x4F]; // "SOLO"
pub const BLOB_VERSION: u8 = 0x02;

fn key_err(e: impl std::fmt::Display) -> CipherError {
    CipherError::BlobFormat(format!("密钥无效: {}", e))
}
fn enc_err(e: impl std::fmt::Display) -> CipherError {
    CipherError::BlobFormat(format!("加密失败: {}", e))
}
fn fmt_err(msg: impl Into<String>) -> CipherError {
    CipherError::BlobFormat(msg.into())
}

/// Encrypt data using AES-256-GCM with SOLO blob format (v2)
pub fn encrypt_blob(key: &[u8; 32], plaintext: &[u8]) -> Result<Zeroizing<Vec<u8>>, CipherError> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(key_err)?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher.encrypt(&nonce, plaintext).map_err(enc_err)?;

    let mut blob = Vec::with_capacity(4 + 1 + NONCE_SIZE + ciphertext.len());
    blob.extend_from_slice(&BLOB_MAGIC);
    blob.push(BLOB_VERSION);
    blob.extend_from_slice(nonce.as_slice());
    blob.extend_from_slice(&ciphertext);
    Ok(Zeroizing::new(blob))
}

/// Decrypt SOLO blob (v2) format
pub fn decrypt_blob(key: &[u8; 32], blob: &[u8]) -> Result<Zeroizing<Vec<u8>>, CipherError> {
    if blob.len() < 33 {
        return Err(fmt_err("密文 Blob 过短"));
    }
    if blob[0..4] != BLOB_MAGIC {
        return Err(fmt_err("无效的 Blob 魔数"));
    }
    if blob[4] != BLOB_VERSION {
        return Err(fmt_err(format!("不支持的 Blob 版本: {}", blob[4])));
    }

    let cipher = Aes256Gcm::new_from_slice(key).map_err(key_err)?;
    let nonce = Nonce::from_slice(&blob[5..17]);
    let plaintext = cipher.decrypt(nonce, &blob[17..]).map_err(enc_err)?;
    Ok(Zeroizing::new(plaintext))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_blob() {
        let key = [0u8; 32];
        let plaintext = b"Hello, SoloSoul!";
        let blob = encrypt_blob(&key, plaintext).unwrap();
        let decrypted = decrypt_blob(&key, &blob).unwrap();
        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_encrypt_decrypt_empty() {
        let key = [0u8; 32];
        let blob = encrypt_blob(&key, b"").unwrap();
        let decrypted = decrypt_blob(&key, &blob).unwrap();
        assert!(decrypted.is_empty());
    }

    #[test]
    fn test_blob_format() {
        let key = [0u8; 32];
        let blob = encrypt_blob(&key, b"test").unwrap();
        assert_eq!(&blob[0..4], &BLOB_MAGIC);
        assert_eq!(blob[4], BLOB_VERSION);
    }

    #[test]
    fn test_decrypt_wrong_key() {
        let key1 = [0u8; 32];
        let key2 = [1u8; 32];
        let blob = encrypt_blob(&key1, b"secret").unwrap();
        assert!(decrypt_blob(&key2, &blob).is_err());
    }
}
