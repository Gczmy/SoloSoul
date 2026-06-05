//! AES-256-GCM blob encryption (SOLO format)
//!
//! Provides authenticated encryption with AES-256-GCM in SOLO blob format:
//! - v2: Magic(4) + Version(1) + Nonce(12) + Ciphertext+Tag(16)
//! - v3: Magic(4) + Version(1) + Header(16) + [Nonce(12) + Ciphertext+Tag(16)]*N

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use zeroize::Zeroizing;

pub const NONCE_SIZE: usize = 12;
pub const KEY_SIZE: usize = 32;
pub const TAG_SIZE: usize = 16;

pub const BLOB_MAGIC: [u8; 4] = [0x53, 0x4F, 0x4C, 0x4F]; // "SOLO"
pub const BLOB_VERSION: u8 = 0x02;
pub const BLOB_VERSION_V3: u8 = 0x03;
pub const DEFAULT_CHUNK_SIZE: usize = 1024 * 1024;

/// Encrypt data using AES-256-GCM with SOLO blob format (v2)
pub fn encrypt_blob(key: &[u8; 32], plaintext: &[u8]) -> Result<Zeroizing<Vec<u8>>, String> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| format!("Invalid key: {}", e))?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| format!("Encryption failed: {}", e))?;

    let mut blob = Vec::with_capacity(4 + 1 + NONCE_SIZE + ciphertext.len());
    blob.extend_from_slice(&BLOB_MAGIC);
    blob.push(BLOB_VERSION);
    blob.extend_from_slice(nonce.as_slice());
    blob.extend_from_slice(&ciphertext);
    Ok(Zeroizing::new(blob))
}

/// Decrypt SOLO blob (v2) format
pub fn decrypt_blob(key: &[u8; 32], blob: &[u8]) -> Result<Zeroizing<Vec<u8>>, String> {
    if blob.len() < 33 {
        return Err("Blob too short".to_string());
    }
    if blob[0..4] != BLOB_MAGIC {
        return Err("Invalid blob magic".to_string());
    }
    if blob[4] != BLOB_VERSION {
        return Err(format!("Unsupported blob version: {}", blob[4]));
    }

    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| format!("Invalid key: {}", e))?;
    let nonce = Nonce::from_slice(&blob[5..17]);
    let plaintext = cipher
        .decrypt(nonce, &blob[17..])
        .map_err(|e| format!("Decryption failed: {}", e))?;
    Ok(Zeroizing::new(plaintext))
}

/// Encrypt data using chunked AES-256-GCM (SOLO blob v3)
pub fn encrypt_chunked_blob(
    key: &[u8; 32],
    plaintext: &[u8],
    chunk_size: usize,
) -> Result<Vec<u8>, String> {
    let chunk_size = if chunk_size == 0 {
        DEFAULT_CHUNK_SIZE
    } else {
        chunk_size
    };
    let original_size = plaintext.len() as u64;
    let chunk_count = original_size.div_ceil(chunk_size as u64) as u32;

    let mut blob = Vec::new();
    blob.extend_from_slice(&BLOB_MAGIC);
    blob.push(BLOB_VERSION_V3);
    blob.extend_from_slice(&original_size.to_be_bytes());
    blob.extend_from_slice(&(chunk_size as u32).to_be_bytes());
    blob.extend_from_slice(&chunk_count.to_be_bytes());

    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| format!("Invalid key: {}", e))?;

    for i in 0..chunk_count as usize {
        let start = i * chunk_size;
        let end = (start + chunk_size).min(plaintext.len());
        let chunk = &plaintext[start..end];

        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher
            .encrypt(&nonce, chunk)
            .map_err(|e| format!("Chunk {} encryption failed: {}", i, e))?;

        blob.extend_from_slice(nonce.as_slice());
        blob.extend_from_slice(&ciphertext);
    }
    Ok(blob)
}

/// Decrypt v3 chunked blob
pub fn decrypt_chunked_blob(key: &[u8; 32], blob: &[u8]) -> Result<Zeroizing<Vec<u8>>, String> {
    if blob.len() < 21 {
        return Err("Blob too short for v3 header".to_string());
    }
    if blob[0..4] != BLOB_MAGIC || blob[4] != BLOB_VERSION_V3 {
        return Err("Invalid v3 blob".to_string());
    }

    let original_size = u64::from_be_bytes(blob[5..13].try_into().unwrap());
    let chunk_size = u32::from_be_bytes(blob[13..17].try_into().unwrap()) as usize;
    let chunk_count = u32::from_be_bytes(blob[17..21].try_into().unwrap());

    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| format!("Invalid key: {}", e))?;
    let mut plaintext = Vec::with_capacity(original_size as usize);
    let mut offset = 21;

    for i in 0..chunk_count as usize {
        let nonce = Nonce::from_slice(&blob[offset..offset + NONCE_SIZE]);
        offset += NONCE_SIZE;

        let is_last = i == chunk_count as usize - 1;
        let expected_plain = if is_last {
            original_size as usize - plaintext.len()
        } else {
            chunk_size
        };
        let expected_cipher = expected_plain + TAG_SIZE;

        if offset + expected_cipher > blob.len() {
            return Err(format!("Chunk {}: ciphertext truncated", i));
        }

        let decrypted = cipher
            .decrypt(nonce, &blob[offset..offset + expected_cipher])
            .map_err(|e| format!("Chunk {} decryption failed: {}", i, e))?;
        offset += expected_cipher;
        plaintext.extend_from_slice(&decrypted);
    }
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

    #[test]
    fn test_chunked_roundtrip() {
        let key = [0x42u8; 32];
        let data = vec![0xABu8; 5 * 1024 * 1024];
        let blob = encrypt_chunked_blob(&key, &data, 1024 * 1024).unwrap();
        let decrypted = decrypt_chunked_blob(&key, &blob).unwrap();
        assert_eq!(decrypted.as_slice(), data.as_slice());
    }
}
