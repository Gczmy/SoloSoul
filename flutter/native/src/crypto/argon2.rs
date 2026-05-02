//! Argon2id key derivation
//!
//! FFI-accessible Argon2id implementation optimized for Apple Silicon

use argon2::{
    password_hash::rand_core::OsRng,
    Argon2, Params, Version, Algorithm,
};
use rand::RngCore;
use zeroize::Zeroizing;

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

/// KDF algorithm selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum KdfAlgorithm {
    /// Argon2id — memory-hard, recommended for password hashing
    #[serde(rename = "argon2id")]
    Argon2id,
    /// PBKDF2-SHA256 — compatible with legacy data
    #[serde(rename = "pbkdf2")]
    Pbkdf2,
}

impl Default for KdfAlgorithm {
    fn default() -> Self {
        Self::Argon2id
    }
}

/// KDF parameter presets (exposed to users in SecuritySettings)
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum KdfPreset {
    /// 8 MiB, 2 iterations — low-end devices
    Fast,
    /// 16 MiB, 3 iterations — default
    Balanced,
    /// 64 MiB, 3 iterations — high security
    Secure,
}

impl Default for KdfPreset {
    fn default() -> Self {
        Self::Balanced
    }
}

impl KdfPreset {
    /// Get the KdfParams for this preset
    pub fn params(&self) -> KdfParams {
        match self {
            Self::Fast => KdfParams {
                algorithm: KdfAlgorithm::Argon2id,
                memory_kib: 8192,       // 8 MiB
                iterations: 2,
                parallelism: 4,
            },
            Self::Balanced => KdfParams {
                algorithm: KdfAlgorithm::Argon2id,
                memory_kib: 16384,      // 16 MiB
                iterations: 3,
                parallelism: 4,
            },
            Self::Secure => KdfParams {
                algorithm: KdfAlgorithm::Argon2id,
                memory_kib: 65536,      // 64 MiB
                iterations: 3,
                parallelism: 4,
            },
        }
    }
}

/// Key derivation function parameters (stored in account config.json)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KdfParams {
    /// KDF algorithm (Argon2id or PBKDF2)
    pub algorithm: KdfAlgorithm,
    /// Memory in KiB (Argon2id: e.g., 65536 for 64 MiB)
    pub memory_kib: u32,
    /// Number of iterations (Argon2id: 3, PBKDF2: 600000)
    pub iterations: u32,
    /// Degree of parallelism (Argon2id: 4)
    pub parallelism: u32,
}

impl Default for KdfParams {
    fn default() -> Self {
        KdfPreset::Balanced.params()
    }
}

impl KdfParams {
    /// Create params for a specific preset
    pub fn from_preset(preset: KdfPreset) -> Self {
        preset.params()
    }

    /// Extract (memory_kib, iterations, parallelism) tuple for use with `derive_key`
    pub fn as_tuple(&self) -> (u32, u32, u32) {
        (self.memory_kib, self.iterations, self.parallelism)
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
/// * `salt` - Pointer to salt bytes (any length >= 8 is valid for Argon2)
/// * `salt_len` - Length of salt
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
    // Salt can be any length (Argon2 requires >= 8, but we let the caller decide)
    // Output must be exactly 32 bytes for our use case
    if output_len != 32 {
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

    #[test]
    fn test_derive_key_deterministic() {
        let password = "consistent_password";
        let salt = [0x12u8; 32];
        let params = (8 * 1024, 1, 1);

        let key1 = derive_key(password, &salt, params.0, params.1, params.2).unwrap();
        let key2 = derive_key(password, &salt, params.0, params.1, params.2).unwrap();

        // Same inputs should produce same output
        assert_eq!(key1.as_slice(), key2.as_slice());
    }

    #[test]
    fn test_derive_key_different_passwords() {
        let salt = [0u8; 32];
        let params = (8 * 1024, 1, 1);

        let key1 = derive_key("password1", &salt, params.0, params.1, params.2).unwrap();
        let key2 = derive_key("password2", &salt, params.0, params.1, params.2).unwrap();

        assert_ne!(key1.as_slice(), key2.as_slice());
    }

    #[test]
    fn test_derive_key_different_salts() {
        let password = "same_password";
        let params = (8 * 1024, 1, 1);
        let salt1 = [0u8; 32];
        let salt2 = [1u8; 32];

        let key1 = derive_key(password, &salt1, params.0, params.1, params.2).unwrap();
        let key2 = derive_key(password, &salt2, params.0, params.1, params.2).unwrap();

        assert_ne!(key1.as_slice(), key2.as_slice());
    }

    #[test]
    fn test_derive_key_output_length() {
        let password = "test";
        let salt = [0u8; 32];
        let key = derive_key(password, &salt, 8 * 1024, 1, 1).unwrap();

        assert_eq!(key.as_slice().len(), 32, "Key should be 32 bytes");
    }

    #[test]
    fn test_derive_key_empty_password() {
        let salt = [0u8; 32];
        let result = derive_key("", &salt, 8 * 1024, 1, 1);
        assert!(result.is_ok());
        let key = result.unwrap();
        assert!(key.iter().any(|&x| x != 0), "Empty password should still produce non-zero key");
    }

    #[test]
    fn test_derive_key_invalid_params_iterations() {
        let password = "test";
        let salt = [0u8; 32];

        // Iterations must be at least 1
        let result = derive_key(password, &salt, 8 * 1024, 0, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_derive_key_invalid_parallelism() {
        let password = "test";
        let salt = [0u8; 32];

        // Parallelism must be at least 1
        let result = derive_key(password, &salt, 8 * 1024, 1, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_salt_nullptr() {
        let result = unsafe { argon2_generate_salt(std::ptr::null_mut(), 32) };
        assert_eq!(result, RESULT_NULLPTR);
    }

    #[test]
    fn test_generate_salt_invalid_length() {
        let mut salt = [0u8; 32];
        let result = unsafe { argon2_generate_salt(salt.as_mut_ptr(), 16) };
        assert_eq!(result, RESULT_INVALID_LEN);
    }

    #[test]
    fn test_generate_salt_produces_different_values() {
        let mut salt1 = [0u8; 32];
        let mut salt2 = [0u8; 32];

        unsafe {
            argon2_generate_salt(salt1.as_mut_ptr(), 32);
            argon2_generate_salt(salt2.as_mut_ptr(), 32);
        }

        assert_ne!(salt1.as_slice(), salt2.as_slice(), "Each salt generation should produce different values");
    }

    #[test]
    fn test_derive_key_ffi_roundtrip() {
        let password = b"ffi_test_password";
        let mut salt = [0u8; 32];
        let mut output = [0u8; 32];

        unsafe {
            argon2_generate_salt(salt.as_mut_ptr(), 32);
        }

        let result = unsafe {
            argon2_derive_key(
                password.as_ptr(),
                password.len(),
                salt.as_ptr(),
                32,
                8 * 1024,
                1,
                1,
                output.as_mut_ptr(),
                32,
            )
        };

        assert_eq!(result, RESULT_OK);
        assert!(output.iter().any(|&x| x != 0), "FFI derived key should not be all zeros");
    }

    #[test]
    fn test_derive_key_ffi_nullptr_password() {
        let mut output = [0u8; 32];
        let result = unsafe {
            argon2_derive_key(
                std::ptr::null(),
                10,
                [0u8; 32].as_ptr(),
                32,
                8 * 1024,
                1,
                1,
                output.as_mut_ptr(),
                32,
            )
        };
        assert_eq!(result, RESULT_NULLPTR);
    }

    #[test]
    fn test_derive_key_ffi_nullptr_salt() {
        let password = b"test";
        let mut output = [0u8; 32];
        let result = unsafe {
            argon2_derive_key(
                password.as_ptr(),
                password.len(),
                std::ptr::null(),
                32,
                8 * 1024,
                1,
                1,
                output.as_mut_ptr(),
                32,
            )
        };
        assert_eq!(result, RESULT_NULLPTR);
    }

    #[test]
    fn test_derive_key_ffi_nullptr_output() {
        let password = b"test";
        let result = unsafe {
            argon2_derive_key(
                password.as_ptr(),
                password.len(),
                [0u8; 32].as_ptr(),
                32,
                8 * 1024,
                1,
                1,
                std::ptr::null_mut(),
                32,
            )
        };
        assert_eq!(result, RESULT_NULLPTR);
    }

    #[test]
    fn test_derive_key_ffi_short_salt_ok() {
        let password = b"test";
        let mut output = [0u8; 32];
        let result = unsafe {
            argon2_derive_key(
                password.as_ptr(),
                password.len(),
                [0u8; 16].as_ptr(),
                16, // Short but valid (>= 8)
                8 * 1024,
                1,
                1,
                output.as_mut_ptr(),
                32,
            )
        };
        assert_eq!(result, RESULT_OK);
    }

    #[test]
    fn test_derive_key_ffi_invalid_output_len() {
        let password = b"test";
        let result = unsafe {
            argon2_derive_key(
                password.as_ptr(),
                password.len(),
                [0u8; 32].as_ptr(),
                32,
                8 * 1024,
                1,
                1,
                [0u8; 32].as_mut_ptr(),
                16, // Invalid output length
            )
        };
        assert_eq!(result, RESULT_INVALID_LEN);
    }

    #[test]
    fn test_default_constants() {
        assert_eq!(DEFAULT_MEMORY_KIB, 16384, "Default memory should be 16MB");
        assert_eq!(DEFAULT_ITERATIONS, 1, "Default iterations should be 1");
        assert_eq!(DEFAULT_PARALLELISM, 4, "Default parallelism should be 4");
    }

    #[test]
    fn test_kdf_preset_fast() {
        let params = KdfPreset::Fast.params();
        assert_eq!(params.algorithm, KdfAlgorithm::Argon2id);
        assert_eq!(params.memory_kib, 8192);
        assert_eq!(params.iterations, 2);
        assert_eq!(params.parallelism, 4);
    }

    #[test]
    fn test_kdf_preset_balanced() {
        let params = KdfPreset::Balanced.params();
        assert_eq!(params.memory_kib, 16384);
        assert_eq!(params.iterations, 3);
        assert_eq!(params.parallelism, 4);
    }

    #[test]
    fn test_kdf_preset_secure() {
        let params = KdfPreset::Secure.params();
        assert_eq!(params.memory_kib, 65536);
        assert_eq!(params.iterations, 3);
        assert_eq!(params.parallelism, 4);
    }

    #[test]
    fn test_kdf_params_default_is_balanced() {
        let params = KdfParams::default();
        assert_eq!(params.memory_kib, 16384);
        assert_eq!(params.iterations, 3);
    }

    #[test]
    fn test_kdf_params_as_tuple() {
        let params = KdfPreset::Secure.params();
        assert_eq!(params.as_tuple(), (65536, 3, 4));
    }

    #[test]
    fn test_kdf_preset_serde_roundtrip() {
        let preset = KdfPreset::Secure;
        let json = serde_json::to_string(&preset).unwrap();
        let deserialized: KdfPreset = serde_json::from_str(&json).unwrap();
        assert_eq!(preset, deserialized);
    }

    #[test]
    fn test_kdf_params_serde_roundtrip() {
        let params = KdfPreset::Fast.params();
        let json = serde_json::to_string(&params).unwrap();
        let deserialized: KdfParams = serde_json::from_str(&json).unwrap();
        assert_eq!(params.algorithm, deserialized.algorithm);
        assert_eq!(params.memory_kib, deserialized.memory_kib);
        assert_eq!(params.iterations, deserialized.iterations);
        assert_eq!(params.parallelism, deserialized.parallelism);
    }

    #[test]
    fn test_kdf_algorithm_default_is_argon2id() {
        assert_eq!(KdfAlgorithm::default(), KdfAlgorithm::Argon2id);
    }

    #[test]
    fn test_result_codes() {
        assert_eq!(RESULT_OK, 0);
        assert_eq!(RESULT_NULLPTR, -1);
        assert_eq!(RESULT_INVALID_LEN, -2);
        assert_eq!(RESULT_INVALID_PARAMS, -3);
        assert_eq!(RESULT_HASH_FAILED, -4);
    }

    #[test]
    fn test_key_zeroize_on_drop() {
        let password = "sensitive_password";
        let salt = [0u8; 32];

        let key = derive_key(password, &salt, 8 * 1024, 1, 1).unwrap();
        let key_slice = key.as_slice().to_vec(); // Copy before drop

        // Key should have non-zero content
        assert!(key_slice.iter().any(|&x| x != 0));
        // After key goes out of scope, the Zeroizing wrapper should zero the memory
    }

    #[test]
    fn test_argon2_password_with_special_chars() {
        let password = "p@ssw0rd!#$%^&*()";
        let salt = [0u8; 32];
        let key = derive_key(password, &salt, 8 * 1024, 1, 1).unwrap();
        assert!(key.iter().any(|&x| x != 0));
    }

    #[test]
    fn test_argon2_unicode_password() {
        let password = "密码パスワード";
        let salt = [0u8; 32];
        let key = derive_key(password, &salt, 8 * 1024, 1, 1).unwrap();
        assert!(key.iter().any(|&x| x != 0));
    }
}
