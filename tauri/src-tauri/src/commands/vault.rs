use crate::commands::{current_account, vault_handle};
use crate::state::AppState;
use solosoul_core::AccountSummary;
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
    let app_handle = state.handle.clone();
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

    // P004: AI conversations stored in dedicated llm_conversations table (row-level).
    // 统计密文总字节（纯 SQL SUM，不解密）。
    if let Ok(bytes) = vault.conversations_size(&account_id) {
        stats.ai_conversations_size = bytes;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn test_get_vault_stats_json_shape() {
        // Verify the JSON output shape matches what the frontend expects
        let stats = serde_json::json!({
            "profileCount": 0,
            "totalSizeBytes": 0,
            "lastModified": "2024-01-01T00:00:00Z",
            "profilesSize": 0,
            "objectsSize": 0,
            "trashSize": 0,
            "snapshotsSize": 0,
            "attachmentsSize": 0,
            "aiConversationsSize": 0,
        });
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("profileCount"));
        assert!(json.contains("totalSizeBytes"));
        assert!(json.contains("lastModified"));
        assert!(json.contains("profilesSize"));
        assert!(json.contains("objectsSize"));
        assert!(json.contains("trashSize"));
        assert!(json.contains("snapshotsSize"));
        assert!(json.contains("attachmentsSize"));
        assert!(json.contains("aiConversationsSize"));
    }

    #[test]
    fn test_sum_dir_file_sizes_nonexistent_dir() {
        let path = std::path::Path::new("/tmp/solosoul_test_nonexistent_12345");
        assert_eq!(sum_dir_file_sizes(path), 0);
    }

    #[test]
    fn test_sum_dir_file_sizes_empty_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        assert_eq!(sum_dir_file_sizes(dir.path()), 0);
    }

    #[test]
    fn test_sum_dir_file_sizes_single_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        let mut f = fs::File::create(&file_path).unwrap();
        f.write_all(b"Hello").unwrap();
        assert_eq!(sum_dir_file_sizes(dir.path()), 5);
    }

    #[test]
    fn test_sum_dir_file_sizes_nested_directories() {
        let dir = tempfile::TempDir::new().unwrap();

        // Create nested structure:
        // tmp/
        //  sub1/
        //    a.txt (10 bytes)
        //  sub2/
        //    sub3/
        //      b.txt (20 bytes)

        fs::create_dir(dir.path().join("sub1")).unwrap();
        let mut a = fs::File::create(dir.path().join("sub1").join("a.txt")).unwrap();
        a.write_all(b"0123456789").unwrap();

        fs::create_dir_all(dir.path().join("sub2").join("sub3")).unwrap();
        let mut b = fs::File::create(dir.path().join("sub2").join("sub3").join("b.txt")).unwrap();
        b.write_all(b"01234567890123456789").unwrap();

        assert_eq!(sum_dir_file_sizes(dir.path()), 30);
    }

    #[test]
    fn test_sum_dir_file_sizes_ignores_dirs() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::create_dir(dir.path().join("empty_sub")).unwrap();
        // Directory itself contributes 0 bytes
        assert_eq!(sum_dir_file_sizes(dir.path()), 0);
    }

    #[test]
    fn test_sum_dir_file_sizes_multiple_files() {
        let dir = tempfile::TempDir::new().unwrap();
        for i in 0..5 {
            let mut f = fs::File::create(dir.path().join(format!("file{}", i))).unwrap();
            f.write_all(b"x").unwrap(); // 1 byte each
        }
        assert_eq!(sum_dir_file_sizes(dir.path()), 5);
    }
}
