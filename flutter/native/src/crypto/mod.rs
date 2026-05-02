//! Crypto module - High-performance encryption primitives
//!
//! Provides:
//! - Argon2id key derivation
//! - AES-256-GCM encryption/decryption
//! - Secure random generation
//! - High-level profile data encryption (SOLO blob format)

pub mod aes;
pub mod argon2;
pub mod utils;

pub use aes::*;
pub use argon2::*;
pub use utils::*;

use zeroize::Zeroizing;

/// Minimum size of legacy Dart format: nonce(12) + tag(16) = 28 bytes
const LEGACY_MIN_SIZE: usize = aes::NONCE_SIZE + aes::TAG_SIZE;

/// Encrypt profile data using AES-256-GCM (SOLO blob format).
///
/// Input: raw plaintext bytes (e.g. JSON).
/// Output: SOLO blob (Magic + Version + Nonce + Ciphertext + Tag).
///
/// The key must be a 32-byte AES key derived from the master password.
pub fn encrypt_profile_data(
    key: &[u8; 32],
    plaintext: &[u8],
) -> Result<Zeroizing<Vec<u8>>, String> {
    aes::encrypt_blob(key, plaintext)
}

/// Decrypt profile data, auto-detecting format.
///
/// Supports two formats:
/// 1. **SOLO blob** (new, Rust-native): Magic(4B "SOLO") + Version(1B) + Nonce(12B) + Ciphertext + Tag(16B)
/// 2. **Legacy Dart format**: Nonce(12B) + Ciphertext + Tag(16B)  (no magic/version header)
///
/// Returns the raw plaintext bytes.
pub fn decrypt_profile_data(key: &[u8; 32], data: &[u8]) -> Result<Zeroizing<Vec<u8>>, String> {
    // Try SOLO blob format first (check magic bytes)
    if data.len() >= 33 && &data[0..4] == b"SOLO" {
        return aes::decrypt_blob(key, data);
    }

    // Fall back to legacy Dart format: nonce(12) + ciphertext + tag(16)
    if data.len() >= LEGACY_MIN_SIZE {
        return decrypt_legacy_format(key, data);
    }

    Err("Data too short to be valid encrypted content".to_string())
}

/// Decrypt data in legacy Dart format: nonce(12B) || ciphertext+tag
fn decrypt_legacy_format(key: &[u8; 32], data: &[u8]) -> Result<Zeroizing<Vec<u8>>, String> {
    use aes_gcm::{
        aead::{Aead, KeyInit},
        Aes256Gcm, Nonce,
    };

    let nonce = &data[0..aes::NONCE_SIZE];
    let ciphertext = &data[aes::NONCE_SIZE..];

    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| format!("Invalid key: {}", e))?;

    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|e| format!("Decryption failed (legacy format): {}", e))?;

    Ok(Zeroizing::new(plaintext))
}

/// Migrate data from legacy Dart format to SOLO blob format.
///
/// Decrypts with legacy format, then re-encrypts with SOLO blob format.
/// Returns the new SOLO blob bytes.
pub fn migrate_to_solo_format(
    key: &[u8; 32],
    legacy_data: &[u8],
) -> Result<Zeroizing<Vec<u8>>, String> {
    let plaintext = decrypt_legacy_format(key, legacy_data)?;
    aes::encrypt_blob(key, &plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = [0x42u8; 32];
        let plaintext = b"{\"name\":\"test\",\"data\":\"value\"}";

        let encrypted = encrypt_profile_data(&key, plaintext).unwrap();
        let decrypted = decrypt_profile_data(&key, &encrypted).unwrap();

        assert_eq!(decrypted.as_slice(), plaintext);
    }

    #[test]
    fn test_decrypt_solo_blob_format() {
        let key = [0u8; 32];
        let plaintext = b"Hello, SoloSoul!";

        // encrypt_blob produces SOLO format
        let blob = aes::encrypt_blob(&key, plaintext).unwrap();
        assert_eq!(&blob[0..4], b"SOLO");

        // decrypt_profile_data should handle it
        let decrypted = decrypt_profile_data(&key, &blob).unwrap();
        assert_eq!(decrypted.as_slice(), plaintext);
    }

    #[test]
    fn test_decrypt_legacy_dart_format() {
        use aes_gcm::{
            aead::{Aead, KeyInit, OsRng},
            Aes256Gcm, Nonce,
        };
        use rand::RngCore;

        let key = [0x55u8; 32];
        let plaintext = b"{\"profile\":\"legacy_data\"}";

        // Simulate Dart format: nonce(12) + ciphertext+tag
        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce_bytes), plaintext.as_ref())
            .unwrap();

        let mut legacy_data = Vec::new();
        legacy_data.extend_from_slice(&nonce_bytes);
        legacy_data.extend_from_slice(&ciphertext);

        // decrypt_profile_data should auto-detect legacy format
        let decrypted = decrypt_profile_data(&key, &legacy_data).unwrap();
        assert_eq!(decrypted.as_slice(), plaintext);
    }

    #[test]
    fn test_migrate_to_solo_format() {
        use aes_gcm::{
            aead::{Aead, KeyInit, OsRng},
            Aes256Gcm, Nonce,
        };
        use rand::RngCore;

        let key = [0x77u8; 32];
        let plaintext = b"{\"settings\":\"migrate_test\"}";

        // Create legacy format
        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce_bytes), plaintext.as_ref())
            .unwrap();
        let mut legacy = Vec::new();
        legacy.extend_from_slice(&nonce_bytes);
        legacy.extend_from_slice(&ciphertext);

        // Migrate
        let solo_blob = migrate_to_solo_format(&key, &legacy).unwrap();
        assert_eq!(&solo_blob[0..4], b"SOLO");
        assert_eq!(solo_blob[4], 0x02);

        // Verify the migrated blob decrypts correctly
        let decrypted = decrypt_profile_data(&key, &solo_blob).unwrap();
        assert_eq!(decrypted.as_slice(), plaintext);
    }

    #[test]
    fn test_decrypt_data_too_short() {
        let key = [0u8; 32];
        let result = decrypt_profile_data(&key, &[0u8; 10]);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_wrong_key() {
        let key1 = [0u8; 32];
        let key2 = [1u8; 32];
        let plaintext = b"secret";

        let encrypted = encrypt_profile_data(&key1, plaintext).unwrap();
        let result = decrypt_profile_data(&key2, &encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypt_empty_data() {
        let key = [0u8; 32];
        let encrypted = encrypt_profile_data(&key, b"").unwrap();
        let decrypted = decrypt_profile_data(&key, &encrypted).unwrap();
        assert!(decrypted.is_empty());
    }

    #[test]
    fn test_encrypt_large_data() {
        let key = [0xAAu8; 32];
        let plaintext = vec![0x42u8; 100_000]; // 100KB

        let encrypted = encrypt_profile_data(&key, &plaintext).unwrap();
        let decrypted = decrypt_profile_data(&key, &encrypted).unwrap();
        assert_eq!(decrypted.as_slice(), plaintext.as_slice());
    }
}
