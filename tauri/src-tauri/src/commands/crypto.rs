#![allow(unused_variables)]
use crate::state::AppState;
use tauri::State;

/// Encrypt arbitrary bytes using the vault's session key
#[tauri::command]
pub async fn encrypt_bytes(state: State<'_, AppState>, data: Vec<u8>) -> Result<Vec<u8>, String> {
    let svc = state.vault_service.read().await;
    let session_key = svc.get_session_key().ok_or("Vault not unlocked")?;
    let key: [u8; 32] = session_key
        .as_slice()
        .try_into()
        .map_err(|_| "Invalid key length")?;
    solosoul_crypto::aes::encrypt_blob(&key, &data).map(|b| b.to_vec())
}

/// Decrypt SOLO blob bytes using the vault's session key
#[tauri::command]
pub async fn decrypt_bytes(state: State<'_, AppState>, data: Vec<u8>) -> Result<Vec<u8>, String> {
    let svc = state.vault_service.read().await;
    let session_key = svc.get_session_key().ok_or("Vault not unlocked")?;
    let key: [u8; 32] = session_key
        .as_slice()
        .try_into()
        .map_err(|_| "Invalid key length")?;
    solosoul_crypto::aes::decrypt_blob(&key, &data).map(|b| b.to_vec())
}

/// Encrypt data with an explicit 32-byte key (no vault needed)
#[tauri::command]
pub async fn encrypt_with_key(key: Vec<u8>, plaintext: Vec<u8>) -> Result<Vec<u8>, String> {
    let key_arr: [u8; 32] = key
        .as_slice()
        .try_into()
        .map_err(|_| format!("Key must be 32 bytes, got {}", key.len()))?;
    solosoul_crypto::aes::encrypt_blob(&key_arr, &plaintext).map(|b| b.to_vec())
}

/// Decrypt data with an explicit 32-byte key (no vault needed)
#[tauri::command]
pub async fn decrypt_with_key(key: Vec<u8>, ciphertext: Vec<u8>) -> Result<Vec<u8>, String> {
    let key_arr: [u8; 32] = key
        .as_slice()
        .try_into()
        .map_err(|_| format!("Key must be 32 bytes, got {}", key.len()))?;
    solosoul_crypto::aes::decrypt_blob(&key_arr, &ciphertext).map(|b| b.to_vec())
}

/// Derive a key from password and salt using Argon2id
#[tauri::command]
pub async fn derive_key(
    password: String,
    salt: Vec<u8>,
    memory_kb: u32,
    iterations: u32,
    parallelism: u32,
) -> Result<Vec<u8>, String> {
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
    use rand::RngCore;
    let mut salt = vec![0u8; length as usize];
    rand::thread_rng().fill_bytes(&mut salt);
    salt
}

/// Constant-time comparison of two byte slices
#[tauri::command]
pub async fn constant_time_compare(a: Vec<u8>, b: Vec<u8>) -> bool {
    solosoul_crypto::secure::secure_compare(&a, &b)
}

/// Get vault statistics
#[tauri::command]
pub async fn get_vault_stats(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
    let stats = vault.stats()?;
    Ok(serde_json::json!({
        "profile_count": stats.profile_count,
        "total_size_bytes": stats.total_size_bytes,
        "last_modified": stats.last_modified,
    }))
}
