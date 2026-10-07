use crate::commands::{current_account, vault_handle};
use crate::state::AppState;
use solosoul_core::AccountSummary;
use tauri::{Emitter, State};

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
    // P016: 命令入口 Zeroizing 包装，避免明文残留堆内存
    let old_password = zeroize::Zeroizing::new(old_password);
    let new_password = zeroize::Zeroizing::new(new_password);
    // 在改变同步状态前核验维护准入；原 root 身份随真正 worker 保持。
    let (owner, maintenance) = {
        let svc = state
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        let owner = svc.root_owner();
        let maintenance = solosoul_core::import_activity::begin_owned_root_maintenance(
            std::sync::Arc::clone(&owner),
        )?;
        solosoul_core::import_activity::ensure_imports_idle(owner.root(), Some(&account_id))?;
        (owner, maintenance)
    };
    // listener 持有旧 Store；真正退出后才允许重加密并发布新 Store。
    state.sync_service.disable_and_wait().await?;
    let service = state.vault_service.clone();
    tokio::task::spawn_blocking(move || {
        let svc = service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        if !std::sync::Arc::ptr_eq(&svc.root_owner(), &owner) {
            return Err("VAULT_ROOT_MISMATCH".to_string());
        }
        svc.change_password_with_maintenance(
            &account_id,
            &old_password,
            &new_password,
            &maintenance,
        )
    })
    .await
    .map_err(|_| "Password change task failed".to_string())?
}

#[tauri::command]
pub async fn vault_list_accounts(
    state: State<'_, AppState>,
) -> Result<Vec<AccountSummary>, String> {
    #[cfg(all(feature = "native-perf", target_os = "windows"))]
    let perf_attempt = crate::native_perf::auth_trace::begin_accounts();
    #[cfg(all(feature = "native-perf", target_os = "windows"))]
    let perf = perf_attempt.as_ref().map(|attempt| attempt.handle());
    #[cfg(all(feature = "native-perf", target_os = "windows"))]
    let perf_queue = perf.as_ref().and_then(|trace| trace.span("blocking-queue"));
    let vault_service = state.vault_service.clone();
    let result = tokio::task::spawn_blocking(move || {
        #[cfg(all(feature = "native-perf", target_os = "windows"))]
        crate::native_perf::auth_trace::complete(perf_queue);
        #[cfg(all(feature = "native-perf", target_os = "windows"))]
        let perf_read = perf
            .as_ref()
            .and_then(|trace| trace.span("worker-vault-read"));
        let svc = vault_service
            .read()
            .map_err(|_| "Vault service lock is poisoned".to_string())?;
        #[cfg(all(feature = "native-perf", target_os = "windows"))]
        {
            crate::native_perf::auth_trace::complete(perf_read);
            if let Some(trace) = &perf {
                trace.verify_root(svc.root_owner().root());
            }
        }
        #[cfg(all(feature = "native-perf", target_os = "windows"))]
        let perf_list = perf.as_ref().and_then(|trace| trace.span("accounts-list"));
        let accounts = svc.list_accounts();
        if accounts.is_empty() {
            return Err("Vault account cache is empty".to_string());
        }
        #[cfg(all(feature = "native-perf", target_os = "windows"))]
        crate::native_perf::auth_trace::complete(perf_list);
        Ok(accounts)
    })
    .await
    .map_err(|e| format!("vault_list_accounts task failed: {}", e))?;
    #[cfg(all(feature = "native-perf", target_os = "windows"))]
    if result.is_ok() {
        if let Some(attempt) = perf_attempt {
            attempt.complete();
        }
    }
    result
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

/// 修改账户名（账户 ID 不可变）：同步更新账户 config 与 accounts 清单。
#[tauri::command]
pub async fn vault_rename_account(
    state: State<'_, AppState>,
    account_id: String,
    new_name: String,
) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    svc.rename_account(&account_id, &new_name)
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
