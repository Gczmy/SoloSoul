//! AES-256-GCM encryption/decryption
//!
//! Provides authenticated encryption with AES-256-GCM

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use rand::RngCore;
use zeroize::Zeroizing;

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
///
/// Magic bytes for encrypted blob
pub const BLOB_MAGIC: [u8; 4] = [0x53, 0x4F, 0x4C, 0x4F]; // "SOLO"
/// Current blob version (v2 single-chunk)
pub const BLOB_VERSION: u8 = 0x02;

/// Encrypt data using AES-256-GCM with a blob format
///
/// Returns: Magic (4) + Version (1) + Nonce (12) + Ciphertext (*) + Tag (16)
pub fn encrypt_blob(key: &[u8; 32], plaintext: &[u8]) -> Result<Zeroizing<Vec<u8>>, String> {
    // Create cipher
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| format!("Invalid key: {}", e))?;

    // Generate random nonce
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce_bytes);

    // Encrypt
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext)
        .map_err(|e| format!("Encryption failed: {}", e))?;

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
    if blob[0..4] != BLOB_MAGIC {
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
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| format!("Invalid key: {}", e))?;

    // Decrypt
    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|e| format!("Decryption failed: {}", e))?;

    Ok(Zeroizing::new(plaintext))
}

/// SOLO blob v3 version marker
pub const BLOB_VERSION_V3: u8 = 0x03;

/// Default chunk size for v3 format: 1 MiB
pub const DEFAULT_CHUNK_SIZE: usize = 1024 * 1024;

/// Encrypt data using chunked AES-256-GCM (SOLO blob v3 format).
///
/// Each chunk is independently encrypted with a unique nonce.
/// This is useful when the full plaintext is already in memory
/// but we want the v3 format (e.g., for consistency or future streaming).
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
    let chunk_count = ((original_size + chunk_size as u64 - 1) / chunk_size as u64) as u32;

    // Header: Magic(4) + Version(1) + OriginalSize(8) + ChunkSize(4) + ChunkCount(4) = 21 bytes
    let header_size = 4 + 1 + 8 + 4 + 4;
    let total_ciphertext_size: usize = (0..chunk_count as usize)
        .map(|i| {
            let start = i * chunk_size;
            let end = ((start + chunk_size) as usize).min(plaintext.len());
            let plain_len = end - start;
            NONCE_SIZE + plain_len + TAG_SIZE
        })
        .sum();

    let mut blob = Vec::with_capacity(header_size + total_ciphertext_size);

    // Write header
    blob.extend_from_slice(&BLOB_MAGIC);
    blob.push(BLOB_VERSION_V3);
    blob.extend_from_slice(&original_size.to_be_bytes());
    blob.extend_from_slice(&(chunk_size as u32).to_be_bytes());
    blob.extend_from_slice(&chunk_count.to_be_bytes());

    // Encrypt each chunk
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| format!("Invalid key: {}", e))?;

    for i in 0..chunk_count as usize {
        let start = i * chunk_size;
        let end = (start + chunk_size).min(plaintext.len());
        let chunk = &plaintext[start..end];

        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);

        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce_bytes), chunk)
            .map_err(|e| format!("Chunk {} encryption failed: {}", i, e))?;

        blob.extend_from_slice(&nonce_bytes);
        blob.extend_from_slice(&ciphertext);
    }

    Ok(blob)
}

/// Decrypt data from a v3 chunked blob format.
pub fn decrypt_chunked_blob(
    key: &[u8; 32],
    blob: &[u8],
) -> Result<Zeroizing<Vec<u8>>, String> {
    // Minimum header size
    if blob.len() < 21 {
        return Err("Blob too short for v3 header".to_string());
    }

    // Verify magic
    if &blob[0..4] != &BLOB_MAGIC {
        return Err("Invalid blob magic".to_string());
    }

    // Verify version
    if blob[4] != BLOB_VERSION_V3 {
        return Err(format!("Expected v3 blob, got version: {}", blob[4]));
    }

    // Parse header
    let original_size = u64::from_be_bytes([
        blob[5], blob[6], blob[7], blob[8],
        blob[9], blob[10], blob[11], blob[12],
    ]);
    let chunk_size = u32::from_be_bytes([blob[13], blob[14], blob[15], blob[16]]) as usize;
    let chunk_count = u32::from_be_bytes([blob[17], blob[18], blob[19], blob[20]]);

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| format!("Invalid key: {}", e))?;

    let mut plaintext = Vec::with_capacity(original_size as usize);
    let mut offset = 21; // After header

    for i in 0..chunk_count as usize {
        // Read nonce
        if offset + NONCE_SIZE > blob.len() {
            return Err(format!("Chunk {}: nonce truncated", i));
        }
        let nonce = &blob[offset..offset + NONCE_SIZE];
        offset += NONCE_SIZE;

        // Determine expected plaintext size for this chunk
        let is_last_chunk = i == chunk_count as usize - 1;
        let expected_plain_size = if is_last_chunk {
            (original_size as usize - plaintext.len())
        } else {
            chunk_size
        };
        let expected_cipher_size = expected_plain_size + TAG_SIZE;

        if offset + expected_cipher_size > blob.len() {
            return Err(format!(
                "Chunk {}: ciphertext truncated (expected {}, got {})",
                i,
                expected_cipher_size,
                blob.len() - offset
            ));
        }

        let ciphertext = &blob[offset..offset + expected_cipher_size];
        offset += expected_cipher_size;

        let decrypted = cipher
            .decrypt(Nonce::from_slice(nonce), ciphertext)
            .map_err(|e| format!("Chunk {} decryption failed: {}", i, e))?;

        plaintext.extend_from_slice(&decrypted);
    }

    if plaintext.len() != original_size as usize {
        return Err(format!(
            "Size mismatch: expected {}, got {}",
            original_size,
            plaintext.len()
        ));
    }

    Ok(Zeroizing::new(plaintext))
}

/// Check if a byte slice is in v3 (chunked) format.
pub fn is_chunked_blob(blob: &[u8]) -> bool {
    blob.len() >= 5
        && &blob[0..4] == &BLOB_MAGIC
        && blob[4] == BLOB_VERSION_V3
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
    if key.is_null()
        || plaintext.is_null()
        || nonce.is_null()
        || ciphertext.is_null()
        || ciphertext_len.is_null()
    {
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
    let ciphertext_output = match cipher.encrypt(Nonce::from_slice(nonce_bytes), plaintext_slice) {
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
    if key.is_null()
        || ciphertext.is_null()
        || nonce.is_null()
        || plaintext.is_null()
        || plaintext_len.is_null()
    {
        return -1;
    }

    let key_slice = std::slice::from_raw_parts(key, KEY_SIZE);
    let ciphertext_slice = std::slice::from_raw_parts(ciphertext, ciphertext_len);
    let nonce_slice = std::slice::from_raw_parts(nonce, NONCE_SIZE);

    let cipher = match Aes256Gcm::new_from_slice(key_slice) {
        Ok(c) => c,
        Err(_) => return -3,
    };

    let plaintext_output = match cipher.decrypt(Nonce::from_slice(nonce_slice), ciphertext_slice) {
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

    #[test]
    fn test_encrypt_decrypt_empty_plaintext() {
        let key = [0u8; 32];
        let plaintext = b"";

        let blob = encrypt_blob(&key, plaintext).expect("Encryption failed");
        assert!(
            blob.len() > 17,
            "Blob should contain magic, version, nonce, and tag"
        );
        let decrypted = decrypt_blob(&key, &blob).expect("Decryption failed");
        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_encrypt_decrypt_large_plaintext() {
        let key = [0x42u8; 32];
        let plaintext = vec![0u8; 1_000_000]; // 1MB of data

        let blob = encrypt_blob(&key, &plaintext).expect("Encryption failed");
        let decrypted = decrypt_blob(&key, &blob).expect("Decryption failed");
        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_encrypt_produces_unique_nonces() {
        let key = [0u8; 32];
        let plaintext = b"Test message";

        let blob1 = encrypt_blob(&key, plaintext).expect("Encryption failed");
        let blob2 = encrypt_blob(&key, plaintext).expect("Encryption failed");

        // Extract nonces (bytes 5-17 for version 2)
        let nonce1 = &blob1[5..17];
        let nonce2 = &blob2[5..17];
        assert_ne!(
            nonce1, nonce2,
            "Each encryption should produce a unique nonce"
        );
    }

    #[test]
    fn test_decrypt_invalid_magic() {
        let key = [0u8; 32];
        let mut blob = vec![0u8; 100];
        blob[0] = 0xFF; // Invalid magic
        blob[4] = BLOB_VERSION;

        let result = decrypt_blob(&key, &blob);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid blob magic"));
    }

    #[test]
    fn test_decrypt_unsupported_version() {
        let key = [0u8; 32];
        let mut blob = encrypt_blob(&key, b"test").expect("Encryption failed");
        blob[4] = 0xFF; // Unsupported version

        let result = decrypt_blob(&key, &blob);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported blob version"));
    }

    #[test]
    fn test_decrypt_truncated_blob() {
        let key = [0u8; 32];
        let blob = vec![0u8; 10]; // Too short

        let result = decrypt_blob(&key, &blob);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Blob too short"));
    }

    #[test]
    fn test_decrypt_tampered_ciphertext() {
        let key = [0u8; 32];
        let mut blob = encrypt_blob(&key, b"Secret message").expect("Encryption failed");

        // Tamper with the ciphertext
        if blob.len() > 20 {
            blob[20] ^= 0xFF;
        }

        let result = decrypt_blob(&key, &blob);
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypt_decrypt_special_characters() {
        let key = [0x1Au8; 32];
        let plaintext = b"Hello, \x00\xff\xe2\x80\xb9 World! \n\t\r";

        let blob = encrypt_blob(&key, plaintext).expect("Encryption failed");
        let decrypted = decrypt_blob(&key, &blob).expect("Decryption failed");
        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_encrypt_decrypt_unicode() {
        let key = [0x1Bu8; 32];
        let plaintext = "你好世界 🌍 مرحبا";

        let blob = encrypt_blob(&key, plaintext.as_bytes()).expect("Encryption failed");
        let decrypted = decrypt_blob(&key, &blob).expect("Decryption failed");
        assert_eq!(plaintext.as_bytes(), decrypted.as_slice());
    }

    #[test]
    fn test_blob_format_structure() {
        let key = [0u8; 32];
        let blob = encrypt_blob(&key, b"Test").expect("Encryption failed");

        // Verify structure: Magic (4) + Version (1) + Nonce (12) + Ciphertext + Tag (16)
        assert_eq!(&blob[0..4], &BLOB_MAGIC, "Magic bytes should be SOLO");
        assert_eq!(blob[4], BLOB_VERSION, "Version should be 2");
        assert!(
            blob.len() >= 4 + 1 + NONCE_SIZE + 16,
            "Blob should have minimum structure"
        );
    }

    #[test]
    fn test_ffi_encrypt_decrypt_roundtrip() {
        let key = [0x55u8; 32];
        let plaintext = b"FFI test data with more content to encrypt";
        let mut nonce = [0u8; NONCE_SIZE];
        let mut ciphertext = vec![0u8; plaintext.len() + 16]; // Extra space for tag
        let mut ciphertext_len = ciphertext.len();

        // Encrypt
        let encrypt_result = unsafe {
            aes_256_gcm_encrypt(
                key.as_ptr(),
                plaintext.as_ptr(),
                plaintext.len(),
                nonce.as_mut_ptr(),
                ciphertext.as_mut_ptr(),
                &mut ciphertext_len,
            )
        };
        assert_eq!(encrypt_result, 0, "FFI encryption should succeed");

        // Decrypt
        let mut decrypted = vec![0u8; ciphertext_len];
        let mut decrypted_len = decrypted.len();
        let decrypt_result = unsafe {
            aes_256_gcm_decrypt(
                key.as_ptr(),
                ciphertext.as_ptr(),
                ciphertext_len,
                nonce.as_ptr(),
                decrypted.as_mut_ptr(),
                &mut decrypted_len,
            )
        };
        assert_eq!(decrypt_result, 0, "FFI decryption should succeed");
        assert_eq!(&decrypted[..decrypted_len], plaintext);
    }

    #[test]
    fn test_ffi_encrypt_with_zero_nonce() {
        let key = [0xAAu8; 32];
        let plaintext = b"Zero nonce test";
        let mut nonce = [0u8; NONCE_SIZE];
        let mut ciphertext = vec![0u8; 256];
        let mut ciphertext_len = ciphertext.len();

        let result = unsafe {
            aes_256_gcm_encrypt(
                key.as_ptr(),
                plaintext.as_ptr(),
                plaintext.len(),
                nonce.as_mut_ptr(),
                ciphertext.as_mut_ptr(),
                &mut ciphertext_len,
            )
        };
        assert_eq!(result, 0);
    }

    #[test]
    fn test_ffi_decrypt_with_wrong_nonce_fails() {
        let key = [0xBBu8; 32];
        let plaintext = b"Test data";
        let mut nonce = [0u8; NONCE_SIZE];
        let mut ciphertext = vec![0u8; 256];
        let mut ciphertext_len = 0;

        // Encrypt with correct nonce
        unsafe {
            aes_256_gcm_encrypt(
                key.as_ptr(),
                plaintext.as_ptr(),
                plaintext.len(),
                nonce.as_mut_ptr(),
                ciphertext.as_mut_ptr(),
                &mut ciphertext_len,
            )
        };

        // Modify nonce
        nonce[0] ^= 0xFF;

        // Try to decrypt with wrong nonce
        let mut decrypted = vec![0u8; ciphertext_len];
        let mut decrypted_len = decrypted.len();
        let result = unsafe {
            aes_256_gcm_decrypt(
                key.as_ptr(),
                ciphertext.as_ptr(),
                ciphertext_len,
                nonce.as_ptr(),
                decrypted.as_mut_ptr(),
                &mut decrypted_len,
            )
        };
        assert_ne!(result, 0, "Decryption with wrong nonce should fail");
    }

    #[test]
    fn test_constants_defined() {
        assert_eq!(NONCE_SIZE, 12, "Nonce should be 12 bytes for AES-GCM");
        assert_eq!(KEY_SIZE, 32, "Key should be 32 bytes for AES-256");
        assert_eq!(TAG_SIZE, 16, "Auth tag should be 16 bytes");
    }
}
