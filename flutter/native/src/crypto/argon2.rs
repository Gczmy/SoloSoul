//! Argon2id key derivation
//!
//! FFI-accessible Argon2id implementation optimized for Apple Silicon

use argon2::{
    password_hash::rand_core::OsRng,
    Argon2, Params, Version, Algorithm,
};
use rand::RngCore;
use zeroize::{Zeroize, Zeroizing};

/// Result codes for FFI functions
const RESULT_OK: i32 = 0;
const RESULT_NULLPTR: i32 = -1;
const RESULT_INVALID_LEN: i32 = -2;
const RESULT_INVALID_PARAMS: i32 = -3;
const RESULT_HASH_FAILED: i32 = -4;

/// Default Argon2id parameters (64MB memory, 3 iterations, 4 parallelism)
pub const DEFAULT_MEMORY_KIB: u32 = 16384;  // 16MB for faster testing
pub const DEFAULT_ITERATIONS: u32 = 1;      // 1 iteration for faster testing
pub const DEFAULT_PARALLELISM: u32 = 4;

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
    OsRng.fill_bytes(salt_slice);

    RESULT_OK
}

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

/// High-level Argon2id key derivation function
pub fn derive_key(password: &str, salt: &[u8], memory_kib: u32, iterations: u32, parallelism: u32) -> Result<Zeroizing<[u8; 32]>, String> {
    let mut key = [0u8; 32];

    let argon2 = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(memory_kib, iterations, parallelism, Some(32))
            .map_err(|e| format!("Invalid Argon2 params: {}", e))?,
    );

    argon2.hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| format!("Hash failed: {}", e))?;

    Ok(Zeroizing::new(key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_key() {
        let password = "testpassword123";
        let salt = [0u8; 32];
        let result = derive_key(password, &salt, 8 * 1024, 1, 1);

        assert!(result.is_ok());
        let key = result.unwrap();
        // Output should not be all zeros
        assert!(key.iter().any(|&x| x != 0));
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
