//! SoloSoul Crypto Library - High-performance Argon2id implementation
//!
//! This library provides FFI-accessible Argon2id key derivation
//! optimized for cross-platform performance (including Apple Silicon).

use argon2::{
    password_hash::rand_core::OsRng,
    Argon2, Params, Version, Algorithm,
};
use rand::RngCore;

/// Result codes for FFI functions
const RESULT_OK: i32 = 0;
const RESULT_NULLPTR: i32 = -1;
const RESULT_INVALID_LEN: i32 = -2;
const RESULT_INVALID_PARAMS: i32 = -3;
const RESULT_HASH_FAILED: i32 = -4;

/// Derive a key using Argon2id
///
/// # Safety
/// - All pointer parameters must be non-null and valid
/// - memory_kib must be at least 8 (argon2 minimum)
/// - iterations must be at least 1
/// - output must have at least 32 bytes of space
///
/// # Arguments
/// * `password` - Pointer to password bytes
/// * `password_len` - Length of password
/// * `salt` - Pointer to 32-byte salt
/// * `salt_len` - Must be 32
/// * `memory_kib` - Memory in KiB (e.g., 65536 for 64MB) - MUST BE KiB, NOT KB!
/// * `iterations` - Number of iterations
/// * `parallelism` - Number of parallel threads
/// * `output` - Pointer to output buffer (must be at least 32 bytes)
/// * `output_len` - Must be 32
///
/// # Returns
/// 0 on success, negative error code on failure
#[no_mangle]
pub unsafe extern "C" fn argon2_derive_key(
    password: *const u8,
    password_len: usize,
    salt: *const u8,
    salt_len: usize,
    memory_kib: u32,
    iterations: u32,
    parallelism: u32,
    output: *mut u8,
    output_len: usize,
) -> i32 {
    // Validate pointers
    if password.is_null() || salt.is_null() || output.is_null() {
        return RESULT_NULLPTR;
    }

    // Validate lengths
    if salt_len != 32 || output_len != 32 {
        return RESULT_INVALID_LEN;
    }

    // Create slices from pointers
    let password = std::slice::from_raw_parts(password, password_len);
    let salt = std::slice::from_raw_parts(salt, salt_len);
    let output = std::slice::from_raw_parts_mut(output, output_len);

    // Build Argon2 params (memory in KiB for argon2 crate)
    // Note: memory_kib is already in KiB, no division needed
    let params = match Params::new(memory_kib, iterations, parallelism, Some(32)) {
        Ok(p) => p,
        Err(_) => return RESULT_INVALID_PARAMS,
    };

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    // Hash password into output buffer
    match argon2.hash_password_into(password, salt, output) {
        Ok(_) => RESULT_OK,
        Err(_) => RESULT_HASH_FAILED,
    }
}

/// Generate a cryptographically secure random salt
///
/// # Safety
/// * `salt` must point to at least 32 bytes of writable memory
///
/// # Arguments
/// * `salt` - Pointer to output buffer (must be at least 32 bytes)
/// * `len` - Must be 32
///
/// # Returns
/// 0 on success, negative error code on failure
#[no_mangle]
pub unsafe extern "C" fn argon2_generate_salt(salt: *mut u8, len: usize) -> i32 {
    if salt.is_null() {
        return RESULT_NULLPTR;
    }

    if len != 32 {
        return RESULT_INVALID_LEN;
    }

    let salt_slice = unsafe { std::slice::from_raw_parts_mut(salt, len) };

    // Generate 32 random bytes directly
    OsRng.fill_bytes(salt_slice);

    RESULT_OK
}

/// Encrypt data using AES-256-GCM
///
/// # Safety
/// All pointers must be non-null and valid
///
/// # Arguments
/// * `key` - 32-byte encryption key
/// * `plaintext` - Data to encrypt
/// * `plaintext_len` - Length of plaintext
/// * `nonce` - 12-byte nonce (will be filled with random bytes)
/// * `ciphertext` - Output buffer (must be at least plaintext_len + 16 bytes)
/// * `ciphertext_len` - Pointer to output ciphertext length
///
/// # Returns
/// 0 on success, negative error code on failure
#[no_mangle]
pub unsafe extern "C" fn aes_256_gcm_encrypt(
    key: *const u8,
    plaintext: *const u8,
    plaintext_len: usize,
    nonce: *mut u8,
    ciphertext: *mut u8,
    ciphertext_len: *mut usize,
) -> i32 {
    use aes_gcm::{
        aead::{Aead, KeyInit},
        Aes256Gcm, Nonce,
    };
    use argon2::password_hash::rand_core::{OsRng, RngCore};

    // Validate pointers
    if key.is_null() || plaintext.is_null() || nonce.is_null() ||
       ciphertext.is_null() || ciphertext_len.is_null() {
        return RESULT_NULLPTR;
    }

    // Validate key length
    let key_slice = unsafe { std::slice::from_raw_parts(key, 32) };
    let plaintext_slice = unsafe { std::slice::from_raw_parts(plaintext, plaintext_len) };

    // Create cipher
    let cipher = match Aes256Gcm::new_from_slice(key_slice) {
        Ok(c) => c,
        Err(_) => return RESULT_INVALID_PARAMS,
    };

    // Generate random nonce
    let nonce_bytes = unsafe { std::slice::from_raw_parts_mut(nonce, 12) };
    OsRng.fill_bytes(nonce_bytes);

    // Encrypt
    let ciphertext_output = match cipher.encrypt(
        Nonce::from_slice(nonce_bytes),
        plaintext_slice,
    ) {
        Ok(c) => c,
        Err(_) => return RESULT_HASH_FAILED,
    };

    // Copy ciphertext
    let ct_slice = unsafe { std::slice::from_raw_parts_mut(ciphertext, ciphertext_output.len()) };
    ct_slice.copy_from_slice(&ciphertext_output);

    // Set length
    *ciphertext_len = ciphertext_output.len();

    RESULT_OK
}

/// Decrypt data using AES-256-GCM
///
/// # Safety
/// All pointers must be non-null and valid
///
/// # Arguments
/// * `key` - 32-byte encryption key
/// * `ciphertext` - Encrypted data
/// * `ciphertext_len` - Length of ciphertext
/// * `nonce` - 12-byte nonce used for encryption
/// * `plaintext` - Output buffer
/// * `plaintext_len` - Pointer to output plaintext length
///
/// # Returns
/// 0 on success, negative error code on failure
#[no_mangle]
pub unsafe extern "C" fn aes_256_gcm_decrypt(
    key: *const u8,
    ciphertext: *const u8,
    ciphertext_len: usize,
    nonce: *const u8,
    plaintext: *mut u8,
    plaintext_len: *mut usize,
) -> i32 {
    use aes_gcm::{
        aead::{Aead, KeyInit},
        Aes256Gcm, Nonce,
    };

    // Validate pointers
    if key.is_null() || ciphertext.is_null() || nonce.is_null() ||
       plaintext.is_null() || plaintext_len.is_null() {
        return RESULT_NULLPTR;
    }

    let key_slice = unsafe { std::slice::from_raw_parts(key, 32) };
    let ciphertext_slice = unsafe { std::slice::from_raw_parts(ciphertext, ciphertext_len) };
    let nonce_slice = unsafe { std::slice::from_raw_parts(nonce, 12) };

    let cipher = match Aes256Gcm::new_from_slice(key_slice) {
        Ok(c) => c,
        Err(_) => return RESULT_INVALID_PARAMS,
    };

    let plaintext_output = match cipher.decrypt(
        Nonce::from_slice(nonce_slice),
        ciphertext_slice,
    ) {
        Ok(p) => p,
        Err(_) => return RESULT_HASH_FAILED,
    };

    let pt_slice = unsafe { std::slice::from_raw_parts_mut(plaintext, plaintext_output.len()) };
    pt_slice.copy_from_slice(&plaintext_output);

    *plaintext_len = plaintext_output.len();

    RESULT_OK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_key() {
        let password = b"testpassword123";
        let salt = [0u8; 32];
        let mut output = [0u8; 32];

        // Use small params for test speed
        let result = unsafe {
            argon2_derive_key(
                password.as_ptr(),
                password.len(),
                salt.as_ptr(),
                salt.len(),
                8 * 1024, // 8KB
                1,
                1,
                output.as_mut_ptr(),
                output.len(),
            )
        };

        assert_eq!(result, RESULT_OK);
        // Output should not be all zeros
        assert!(output.iter().any(|&x| x != 0));
    }

    #[test]
    fn test_generate_salt() {
        let mut salt = [0u8; 32];

        let result = unsafe {
            argon2_generate_salt(salt.as_mut_ptr(), salt.len())
        };

        assert_eq!(result, RESULT_OK);
        // Salt should not be all zeros
        assert!(salt.iter().any(|&x| x != 0));
    }
}
