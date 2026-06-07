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

/// Get vault statistics with breakdown components
#[tauri::command]
pub async fn get_vault_stats(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
    let mut stats = vault.stats()?;

    // Attachments stored at base_path/attachments/{objectId}/{attachmentId}/
    let attachments_dir = svc.base_path().join("attachments");
    stats.attachments_size = sum_dir_file_sizes(&attachments_dir);

    // AI conversations stored inside profiles (in the preferences JSON blob)
    // Estimate by loading profile data and checking llmConversations key
    if let Some(account_id) = &svc.get_current_account() {
        if let Ok(Some(profile)) = vault.load_profile(account_id) {
            if !profile.data.is_empty() {
                if let Ok(data) = serde_json::from_slice::<serde_json::Value>(&profile.data) {
                    if let Some(convs) = data.pointer("/preferences/llmConversations") {
                        if let Some(arr) = convs.as_array() {
                            let raw = serde_json::to_vec(arr).unwrap_or_default();
                            stats.ai_conversations_size = raw.len() as u64;
                        }
                    }
                }
            }
        }
    }

    let total = stats.profiles_size
        + stats.objects_size
        + stats.trash_size
        + stats.snapshots_size
        + stats.attachments_size
        + stats.ai_conversations_size;

    Ok(serde_json::json!({
        "profileCount": stats.profile_count,
        "totalSizeBytes": total,
        "lastModified": stats.last_modified,
        "profilesSize": stats.profiles_size,
        "objectsSize": stats.objects_size,
        "trashSize": stats.trash_size,
        "snapshotsSize": stats.snapshots_size,
        "attachmentsSize": stats.attachments_size,
        "aiConversationsSize": stats.ai_conversations_size,
    }))
}

/// Recursively sum file sizes under a directory (returns 0 if path doesn't exist).
fn sum_dir_file_sizes(dir: &std::path::Path) -> u64 {
    if !dir.exists() {
        return 0;
    }
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                total += sum_dir_file_sizes(&path);
            } else if path.is_file() {
                if let Ok(meta) = path.metadata() {
                    total += meta.len();
                }
            }
        }
    }
    total
}
