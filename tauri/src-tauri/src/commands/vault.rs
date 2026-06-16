use crate::commands::{current_account, vault_handle};
use crate::state::AppState;
use solosoul_core::auth::verify_password_core;
use solosoul_core::{AccountConfig, AccountSummary};
use tauri::{Emitter, State};

#[tauri::command]
pub async fn unlock(
    state: State<'_, AppState>,
    account_id: String,
    password: String,
) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    svc.unlock(&account_id, &password)?;
    Ok(())
}

#[tauri::command]
pub async fn lock(state: State<'_, AppState>) -> Result<(), String> {
    let app_handle = state.app_handle().clone();
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    svc.lock();
    // Emit event so frontend can clear sensitive stores and redirect to login
    let _ = app_handle.emit("vault-locked", ());
    Ok(())
}

#[tauri::command]
pub async fn get_state(state: State<'_, AppState>) -> Result<String, String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    Ok(svc.get_vault_state())
}

#[tauri::command]
pub async fn change_password(
    state: State<'_, AppState>,
    account_id: String,
    old_password: String,
    new_password: String,
) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    svc.change_password(&account_id, &old_password, &new_password)
}

#[tauri::command]
pub async fn delete_account(
    state: State<'_, AppState>,
    account_id: String,
    password: String,
) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    // Verify password before allowing destructive account deletion
    let config_path = svc.base_path().join(&account_id).join("config.json");
    let content =
        std::fs::read_to_string(&config_path).map_err(|_| "Account not found".to_string())?;
    let config: AccountConfig =
        serde_json::from_str(&content).map_err(|_| "Parse error".to_string())?;
    if !verify_password_core(&password, &config)? {
        return Err("Invalid password".to_string());
    }
    drop(config);
    svc.delete_account(&account_id)
}

#[tauri::command]
pub async fn vault_list_accounts(
    state: State<'_, AppState>,
) -> Result<Vec<AccountSummary>, String> {
    let vault_service = state.vault_service.clone();
    tokio::task::spawn_blocking(move || {
        let svc = vault_service
            .read()
            .map_err(|_| "Vault service lock is poisoned".to_string())?;
        let accounts = svc.list_accounts();
        if accounts.is_empty() {
            return Err("Vault account cache is empty".to_string());
        }
        Ok(accounts)
    })
    .await
    .map_err(|e| format!("vault_list_accounts task failed: {}", e))?
}

#[tauri::command]
pub async fn vault_update_hint(
    state: State<'_, AppState>,
    account_id: String,
    hint: Option<String>,
) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    svc.update_password_hint(&account_id, hint.as_deref().unwrap_or(""))
}

/// Get vault statistics with breakdown components.
#[tauri::command]
pub async fn get_vault_stats(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let vault = vault_handle(&state)?;
    let account_id = current_account(&state)?;
    let mut stats = vault.stats()?;

    // Attachments stored at base_path/attachments/{objectId}/{attachmentId}/
    // Only count attachment files that are referenced in object __attachments metadata
    // (orphaned files from legacy attachment_delete bug are excluded)
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let base_dir = svc.base_path().join("attachments");
    let mut attachments_size = 0u64;
    if let Ok(objects) = vault.list_object_attachment_ids(&account_id) {
        for (object_id, att_ids) in objects {
            for att_id in att_ids {
                let att_dir = base_dir.join(&object_id).join(&att_id);
                attachments_size += sum_dir_file_sizes(&att_dir);
            }
        }
    }
    stats.attachments_size = attachments_size;

    // AI conversations stored inside profiles (in the preferences JSON blob)
    if let Ok(Some(profile)) = vault.load_profile(&account_id) {
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
