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
