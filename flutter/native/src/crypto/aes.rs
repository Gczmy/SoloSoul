//! AES-256-GCM encryption/decryption
//!
//! Provides authenticated encryption with AES-256-GCM

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use rand::RngCore;
use zeroize::{Zeroize, Zeroizing};

/// AES-256-GCM nonce size (12 bytes)
pub const NONCE_SIZE: usize = 12;

/// AES-256-GCM key size (32 bytes)
pub const KEY_SIZE: usize = 32;

/// Auth tag size (16 bytes)
pub const TAG_SIZE: usize = 16;

/// Encrypted blob structure:
/// - Magic (4 bytes): "SOLO"
/// - Version (1 byte)
/// - Nonce (12 bytes)
/// - Ciphertext + Auth Tag (variable)

/// Magic bytes for encrypted blob
const BLOB_MAGIC: [u8; 4] = [0x53, 0x4F, 0x4C, 0x4F]; // "SOLO"
/// Current blob version
const BLOB_VERSION: u8 = 0x02;

/// Encrypt data using AES-256-GCM with a blob format
///
/// Returns: Magic (4) + Version (1) + Nonce (12) + Ciphertext (*) + Tag (16)
pub fn encrypt_blob(key: &[u8; 32], plaintext: &[u8]) -> Result<Zeroizing<Vec<u8>>, String> {
    // Create cipher
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| format!("Invalid key: {}", e))?;

    // Generate random nonce
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce_bytes);

    // Encrypt
    let ciphertext = cipher.encrypt(
        Nonce::from_slice(&nonce_bytes),
        plaintext,
    ).map_err(|e| format!("Encryption failed: {}", e))?;

    // Build blob: Magic + Version + Nonce + Ciphertext + Tag
    let mut blob = Vec::with_capacity(4 + 1 + NONCE_SIZE + ciphertext.len());
    blob.extend_from_slice(&BLOB_MAGIC);
    blob.push(BLOB_VERSION);
    blob.extend_from_slice(&nonce_bytes);
    blob.extend_from_slice(&ciphertext);

    Ok(Zeroizing::new(blob))
}

/// Decrypt data from a blob format
pub fn decrypt_blob(key: &[u8; 32], blob: &[u8]) -> Result<Zeroizing<Vec<u8>>, String> {
    // Validate minimum size: Magic (4) + Version (1) + Nonce (12) + Tag (16) = 33
    if blob.len() < 33 {
        return Err("Blob too short".to_string());
    }

    // Verify magic
    if &blob[0..4] != &BLOB_MAGIC {
        return Err("Invalid blob magic".to_string());
    }

    // Get version
    let version = blob[4];
    if version != BLOB_VERSION {
        return Err(format!("Unsupported blob version: {}", version));
    }

    // Extract nonce
    let nonce = &blob[5..17];

    // Extract ciphertext (everything after nonce)
    let ciphertext = &blob[17..];

    // Create cipher
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| format!("Invalid key: {}", e))?;

    // Decrypt
    let plaintext = cipher.decrypt(
        Nonce::from_slice(nonce),
        ciphertext,
    ).map_err(|e| format!("Decryption failed: {}", e))?;

    Ok(Zeroizing::new(plaintext))
}

/// Low-level FFI encryption function
///
/// # Safety
/// All pointers must be non-null and valid
#[no_mangle]
pub unsafe extern "C" fn aes_256_gcm_encrypt(
    key: *const u8,
    plaintext: *const u8,
    plaintext_len: usize,
    nonce: *mut u8,
    ciphertext: *mut u8,
    ciphertext_len: *mut usize,
) -> i32 {
    // Validate pointers
    if key.is_null() || plaintext.is_null() || nonce.is_null() ||
       ciphertext.is_null() || ciphertext_len.is_null() {
        return -1;
    }

    let key_slice = std::slice::from_raw_parts(key, KEY_SIZE);
    let plaintext_slice = std::slice::from_raw_parts(plaintext, plaintext_len);

    let cipher = match Aes256Gcm::new_from_slice(key_slice) {
        Ok(c) => c,
        Err(_) => return -3,
    };

    // Use the provided nonce
    let nonce_bytes = std::slice::from_raw_parts(nonce, NONCE_SIZE);

    // Encrypt
    let ciphertext_output = match cipher.encrypt(
        Nonce::from_slice(nonce_bytes),
        plaintext_slice,
    ) {
        Ok(c) => c,
        Err(_) => return -4,
    };

    // Copy ciphertext
    let ct_slice = std::slice::from_raw_parts_mut(ciphertext, ciphertext_output.len());
    ct_slice.copy_from_slice(&ciphertext_output);

    // Set length
    *ciphertext_len = ciphertext_output.len();

    0
}

/// Low-level FFI decryption function
///
/// # Safety
/// All pointers must be non-null and valid
#[no_mangle]
pub unsafe extern "C" fn aes_256_gcm_decrypt(
    key: *const u8,
    ciphertext: *const u8,
    ciphertext_len: usize,
    nonce: *const u8,
    plaintext: *mut u8,
    plaintext_len: *mut usize,
) -> i32 {
    // Validate pointers
    if key.is_null() || ciphertext.is_null() || nonce.is_null() ||
       plaintext.is_null() || plaintext_len.is_null() {
        return -1;
    }

    let key_slice = std::slice::from_raw_parts(key, KEY_SIZE);
    let ciphertext_slice = std::slice::from_raw_parts(ciphertext, ciphertext_len);
    let nonce_slice = std::slice::from_raw_parts(nonce, NONCE_SIZE);

    let cipher = match Aes256Gcm::new_from_slice(key_slice) {
        Ok(c) => c,
        Err(_) => return -3,
    };

    let plaintext_output = match cipher.decrypt(
        Nonce::from_slice(nonce_slice),
        ciphertext_slice,
    ) {
        Ok(p) => p,
        Err(_) => return -4,
    };

    let pt_slice = std::slice::from_raw_parts_mut(plaintext, plaintext_output.len());
    pt_slice.copy_from_slice(&plaintext_output);

    *plaintext_len = plaintext_output.len();

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_blob() {
        let key = [0u8; 32];
        let plaintext = b"Hello, SoloSoul!";

        let blob = encrypt_blob(&key, plaintext).expect("Encryption failed");
        let decrypted = decrypt_blob(&key, &blob).expect("Decryption failed");

        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_encrypt_decrypt_with_different_key() {
        let key1 = [0u8; 32];
        let key2 = [1u8; 32];
        let plaintext = b"Secret data";

        let blob = encrypt_blob(&key1, plaintext).expect("Encryption failed");
        let result = decrypt_blob(&key2, &blob);

        assert!(result.is_err());
    }
}
