use crate::state::AppState;
use tauri::State;

/// Encrypt arbitrary bytes using the vault's session key
#[tauri::command]
pub async fn encrypt_bytes(state: State<'_, AppState>, data: Vec<u8>) -> Result<Vec<u8>, String> {
    let svc = state.vault_service.read().unwrap();
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
    let svc = state.vault_service.read().unwrap();
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

/// Get vault statistics with breakdown components
#[tauri::command]
pub async fn get_vault_stats(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let svc = state.vault_service.read().unwrap();
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();
    let mut stats = vault.stats()?;

    // Attachments stored at base_path/attachments/{objectId}/{attachmentId}/
    // Only count sizes of attachments that are referenced in object metadata
    // (orphaned files from old attachment_delete bug will be ignored)
    // Only count attachment files that are referenced in object __attachments metadata
    // (orphaned files from legacy attachment_delete bug are excluded)
    // R020: use a single batch query instead of N+1 load_object calls.
    let base_dir = svc.base_path().join("attachments");
    let mut attachments_size = 0u64;
    if let Some(account_id) = &svc.get_current_account() {
        if let Ok(objects) = vault.list_object_attachment_ids(account_id) {
            for (object_id, att_ids) in objects {
                for att_id in att_ids {
                    let att_dir = base_dir.join(&object_id).join(&att_id);
                    attachments_size += sum_dir_file_sizes(&att_dir);
                }
            }
        }
    }
    stats.attachments_size = attachments_size;

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
