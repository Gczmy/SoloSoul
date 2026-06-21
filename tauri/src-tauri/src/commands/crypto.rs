use crate::state::AppState;
use tauri::State;

/// Encrypt arbitrary bytes using the vault's session key
#[tauri::command]
pub async fn encrypt_bytes(state: State<'_, AppState>, data: Vec<u8>) -> Result<Vec<u8>, String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let session_key = svc.get_session_key().ok_or("Vault not unlocked")?;
    let key: [u8; solosoul_crypto::aes::KEY_SIZE] = session_key
        .as_slice()
        .try_into()
        .map_err(|_| "Invalid key length")?;
    solosoul_crypto::aes::encrypt_blob(&key, &data).map(|b| b.to_vec())
}

/// Decrypt SOLO blob bytes using the vault's session key
#[tauri::command]
pub async fn decrypt_bytes(state: State<'_, AppState>, data: Vec<u8>) -> Result<Vec<u8>, String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let session_key = svc.get_session_key().ok_or("Vault not unlocked")?;
    let key: [u8; solosoul_crypto::aes::KEY_SIZE] = session_key
        .as_slice()
        .try_into()
        .map_err(|_| "Invalid key length")?;
    solosoul_crypto::aes::decrypt_blob(&key, &data).map(|b| b.to_vec())
}

/// Encrypt data with an explicit 32-byte key (no vault needed)
#[tauri::command]
pub async fn encrypt_with_key(key: Vec<u8>, plaintext: Vec<u8>) -> Result<Vec<u8>, String> {
    let key_arr: [u8; solosoul_crypto::aes::KEY_SIZE] =
        key.as_slice().try_into().map_err(|_| {
            format!(
                "Key must be {} bytes, got {}",
                solosoul_crypto::aes::KEY_SIZE,
                key.len()
            )
        })?;
    solosoul_crypto::aes::encrypt_blob(&key_arr, &plaintext).map(|b| b.to_vec())
}

/// Decrypt data with an explicit 32-byte key (no vault needed)
#[tauri::command]
pub async fn decrypt_with_key(key: Vec<u8>, ciphertext: Vec<u8>) -> Result<Vec<u8>, String> {
    let key_arr: [u8; solosoul_crypto::aes::KEY_SIZE] =
        key.as_slice().try_into().map_err(|_| {
            format!(
                "Key must be {} bytes, got {}",
                solosoul_crypto::aes::KEY_SIZE,
                key.len()
            )
        })?;
    solosoul_crypto::aes::decrypt_blob(&key_arr, &ciphertext).map(|b| b.to_vec())
}

/// Maximum Argon2 parameters accepted from the frontend to prevent DoS.
const MAX_MEMORY_KB: u32 = 64 * 1024;
const MAX_ITERATIONS: u32 = 10;
const MAX_PARALLELISM: u32 = 16;
const MAX_SALT_LENGTH: u32 = 64;

/// Derive a key from password and salt using Argon2id
#[tauri::command]
pub async fn derive_key(
    password: String,
    salt: Vec<u8>,
    memory_kb: u32,
    iterations: u32,
    parallelism: u32,
) -> Result<Vec<u8>, String> {
    if memory_kb > MAX_MEMORY_KB {
        return Err(format!("memory_kb exceeds maximum {}", MAX_MEMORY_KB));
    }
    if iterations > MAX_ITERATIONS {
        return Err(format!("iterations exceeds maximum {}", MAX_ITERATIONS));
    }
    if parallelism > MAX_PARALLELISM {
        return Err(format!("parallelism exceeds maximum {}", MAX_PARALLELISM));
    }
    let config = solosoul_crypto::KdfConfig {
        memory_kb,
        iterations,
        parallelism,
    };
    solosoul_crypto::derive_key(&password, &salt, &config)
        .map(|k| k.to_vec())
        .map_err(|e| format!("Key derivation failed: {}", e))
}

/// Generate cryptographically secure random bytes
#[tauri::command]
pub async fn generate_salt(length: u32) -> Vec<u8> {
    use rand::rngs::OsRng;
    use rand::RngCore;
    if length == 0 || length > MAX_SALT_LENGTH {
        return vec![];
    }
    let mut salt = vec![0u8; length as usize];
    OsRng.fill_bytes(&mut salt);
    salt
}

/// Constant-time comparison of two byte slices
#[tauri::command]
pub async fn constant_time_compare(a: Vec<u8>, b: Vec<u8>) -> bool {
    solosoul_crypto::secure::secure_compare(&a, &b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_derive_key_rejects_excessive_memory() {
        let result = derive_key("pwd".to_string(), vec![0u8; 16], MAX_MEMORY_KB + 1, 1, 1).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("memory_kb exceeds maximum"));
    }

    #[tokio::test]
    async fn test_derive_key_rejects_excessive_iterations() {
        let result = derive_key(
            "pwd".to_string(),
            vec![0u8; 16],
            1024,
            MAX_ITERATIONS + 1,
            1,
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("iterations exceeds maximum"));
    }

    #[tokio::test]
    async fn test_derive_key_rejects_excessive_parallelism() {
        let result = derive_key(
            "pwd".to_string(),
            vec![0u8; 16],
            1024,
            1,
            MAX_PARALLELISM + 1,
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("parallelism exceeds maximum"));
    }

    #[tokio::test]
    async fn test_derive_key_accepts_boundary_values() {
        let result = derive_key(
            "pwd".to_string(),
            vec![0u8; 16],
            MAX_MEMORY_KB,
            MAX_ITERATIONS,
            MAX_PARALLELISM,
        )
        .await;
        // Boundary values must NOT be rejected by parameter validation.
        // If Err, verify it's a KDF execution error (e.g. OOM), not a parameter validation error.
        if let Err(ref msg) = result {
            assert!(
                !msg.contains("exceeds maximum"),
                "Boundary values must pass parameter validation: {}",
                msg
            );
        }
    }

    #[tokio::test]
    async fn test_derive_key_accepts_valid_params() {
        let result = derive_key("test_password".to_string(), vec![0u8; 16], 8, 1, 1).await;
        assert!(
            result.is_ok(),
            "Expected OK with minimal params: {:?}",
            result.err()
        );
        let key = result.unwrap();
        assert_eq!(key.len(), 32, "Derived key must be 32 bytes");
    }

    #[tokio::test]
    async fn test_generate_salt_zero_length_returns_empty() {
        let result = generate_salt(0).await;
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_generate_salt_exceeds_max_returns_empty() {
        let result = generate_salt(MAX_SALT_LENGTH + 1).await;
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_generate_salt_valid_length() {
        let result = generate_salt(32).await;
        assert_eq!(result.len(), 32);
        assert!(
            result.iter().any(|&b| b != 0),
            "salt should have non-zero bytes"
        );
    }

    #[tokio::test]
    async fn test_generate_salt_max_length() {
        let result = generate_salt(MAX_SALT_LENGTH).await;
        assert_eq!(result.len(), MAX_SALT_LENGTH as usize);
    }

    #[tokio::test]
    async fn test_constant_time_compare_equal() {
        assert!(constant_time_compare(vec![1, 2, 3], vec![1, 2, 3]).await);
    }

    #[tokio::test]
    async fn test_constant_time_compare_different() {
        assert!(!constant_time_compare(vec![1, 2, 3], vec![1, 2, 4]).await);
    }

    #[tokio::test]
    async fn test_constant_time_compare_different_lengths() {
        assert!(!constant_time_compare(vec![1, 2, 3], vec![1, 2]).await);
    }

    #[tokio::test]
    async fn test_constant_time_compare_empty() {
        assert!(constant_time_compare(vec![], vec![]).await);
    }

    #[tokio::test]
    async fn test_constant_time_compare_one_empty() {
        assert!(!constant_time_compare(vec![1], vec![]).await);
    }

    #[tokio::test]
    async fn test_encrypt_with_key_wrong_size_rejected() {
        let result = encrypt_with_key(vec![0u8; 16], vec![1, 2, 3]).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Key must be"));
    }

    #[tokio::test]
    async fn test_encrypt_with_key_empty_key_rejected() {
        let result = encrypt_with_key(vec![], vec![1, 2, 3]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_encrypt_decrypt_with_key_roundtrip() {
        let key = vec![0xABu8; 32];
        let plaintext = b"Hello, encrypted world!".to_vec();

        let ciphertext = encrypt_with_key(key.clone(), plaintext.clone())
            .await
            .unwrap();
        assert!(!ciphertext.is_empty());
        assert_ne!(ciphertext, plaintext);

        let decrypted = decrypt_with_key(key, ciphertext).await.unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[tokio::test]
    async fn test_decrypt_with_key_wrong_key_fails() {
        let key = vec![0xABu8; 32];
        let wrong_key = vec![0xBAu8; 32];
        let plaintext = b"secret data".to_vec();

        let ciphertext = encrypt_with_key(key, plaintext).await.unwrap();
        let result = decrypt_with_key(wrong_key, ciphertext).await;
        assert!(result.is_err());
    }
}
