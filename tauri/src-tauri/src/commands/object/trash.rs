use crate::commands::settings::resolve_ui_prefs_path;
use crate::commands::vault_handle;
use crate::state::AppState;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::State;

use super::*;

/// Result returned by object_restore / trash_restore describing what happened.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreOutcome {
    pub restored_id: String,
    pub name: String,
    pub cascaded_page_name: Option<String>,
    pub cascaded_count: u32,
    pub rebuilt_page_name: Option<String>,
    pub consumed_trash_ids: Vec<String>,
}

impl From<solosoul_core::objects::RestoreResult> for RestoreOutcome {
    fn from(result: solosoul_core::objects::RestoreResult) -> Self {
        Self {
            restored_id: result.restored_id,
            name: result.restored_name,
            cascaded_page_name: result.cascaded_page_name,
            cascaded_count: result.cascaded_count,
            rebuilt_page_name: result.rebuilt_page_name,
            consumed_trash_ids: result.consumed_trash_ids,
        }
    }
}

#[tauri::command]
pub async fn object_trash_list(
    state: State<'_, AppState>,
    account_id: String,
    since: Option<i64>,
) -> Result<Vec<solosoul_vault::TrashItemSummary>, String> {
    let _ = account_id;
    let vault = vault_handle(&state)?;
    // P114: 回收站全量解密移入 spawn_blocking，避免阻塞 tokio worker。
    tokio::task::spawn_blocking(move || vault.list_trash_items(None, since))
        .await
        .map_err(|e| format!("object_trash_list task failed: {e}"))?
}

/// Read the user's language setting from plaintext UI preferences.
fn get_ui_language<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    svc: &solosoul_core::vault_service::VaultService,
) -> String {
    let path = match resolve_ui_prefs_path(app, svc) {
        Ok(p) => p,
        Err(_) => return "en-US".to_string(),
    };
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(prefs) = serde_json::from_str::<serde_json::Value>(&content) {
                if prefs.is_object() {
                    if let Some(lang) = prefs.get("language").and_then(|v| v.as_str()) {
                        return lang.to_string();
                    }
                }
            }
        }
    }
    "en-US".to_string()
}

/// Restore an object from trash. Delegates to solosoul-core::objects::restore_from_trash_with_lang.
#[tauri::command]
pub async fn object_restore(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    trash_id: String,
    lang: Option<String>,
) -> Result<RestoreOutcome, String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();
    let _trash = vault
        .get_trash_item(&trash_id)?
        .ok_or("Trash item not found")?;

    let fallback_lang = get_ui_language(&app, &svc);
    let lang = lang.as_deref().unwrap_or(&fallback_lang);

    let result = solosoul_core::objects::restore_from_trash_with_lang(vault, &trash_id, lang)?;
    state.auto_sync.trigger_debounce();
    state.device_auto_sync.trigger_data_change();

    Ok(result.into())
}

#[tauri::command]
pub async fn object_purge(state: State<'_, AppState>, object_id: String) -> Result<(), String> {
    let vault = vault_handle(&state)?;

    let (obj_name, obj_section) = vault
        .load_object(&object_id)
        .ok()
        .flatten()
        .map(|r| (r.name, r.section_type))
        .unwrap_or_default();
    vault.delete_object(&object_id, false)?;
    vault.delete_trash_item(&object_id).ok();
    let _ = vault.log_structured(
        "object_purge",
        "object",
        Some(&object_id),
        Some(&obj_name),
        "user",
        Some(&format!("section={}", obj_section)),
    );
    state.auto_sync.trigger_debounce();
    state.device_auto_sync.trigger_data_change();
    Ok(())
}

#[tauri::command]
pub async fn trash_restore(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    trash_id: String,
    lang: Option<String>,
) -> Result<RestoreOutcome, String> {
    object_restore(app, state, trash_id, lang).await
}

/// Permanently delete a trash item (by trash_id → looks up original_id).
#[tauri::command]
pub async fn trash_permanent_delete(
    state: State<'_, AppState>,
    trash_id: String,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;

    if let Ok(Some(trash)) = vault.get_trash_item(&trash_id) {
        if trash.item_type != "template" {
            vault.delete_object(&trash.original_id, false)?;
        }
        let _ = vault.log_structured(
            "trash_permanent_delete",
            "trash_item",
            Some(&trash_id),
            Some(&trash.name_snapshot),
            "user",
            Some(&format!("original_id={}", trash.original_id)),
        );
        vault.delete_trash_item(&trash_id).ok();
        state.auto_sync.trigger_debounce();
        state.device_auto_sync.trigger_data_change();
        return Ok(());
    }
    vault.delete_trash_item(&trash_id).ok();
    let _ = vault.log_structured(
        "trash_permanent_delete",
        "trash_item",
        Some(&trash_id),
        None,
        "user",
        None,
    );
    state.auto_sync.trigger_debounce();
    state.device_auto_sync.trigger_data_change();
    Ok(())
}

/// 在账户登录/解锁完成后，自动清理所有已过期的回收站项目。
/// 该方法会遍历 `trash_items` 表中 `expires_at` 早于当前时间的项目，
/// 对非 template 类型项目先物理删除原始对象，再记录审计日志并从回收站移除。
///
/// 清理逻辑在 tokio 后台 blocking 任务中执行，不阻塞登录/解锁响应。
/// 清理失败仅记录日志，不影响登录/解锁结果。
///
/// 通过 `AppState.trash_cleanup_running` 保证全局同时只运行一个清理任务。
pub fn run_expired_trash_cleanup(state: &crate::state::AppState) {
    // CAS 设置运行标志：如果已有任务在运行，则直接跳过，避免并发重复清理。
    match state.trash_cleanup_running.compare_exchange(
        false,
        true,
        Ordering::Acquire,
        Ordering::Relaxed,
    ) {
        Ok(_) => {}
        Err(_) => {
            tracing::info!("[trash_cleanup] already running, skipping duplicate run");
            return;
        }
    }

    // 使用 Drop guard 确保无论清理成功、失败还是 panic，标志位都会被重置。
    struct CleanupGuard(Arc<std::sync::atomic::AtomicBool>);
    impl Drop for CleanupGuard {
        fn drop(&mut self) {
            self.0.store(false, Ordering::Release);
        }
    }

    let state = state.clone();
    tokio::task::spawn_blocking(move || {
        let _guard = CleanupGuard(state.trash_cleanup_running.clone());

        let result = (|| -> Result<usize, String> {
            let svc = state
                .vault_service
                .read()
                .map_err(|_| "Vault service lock poisoned".to_string())?;
            let vault = svc.get_vault_store().ok_or("Vault not unlocked")?;
            vault.cleanup_expired_trash()
        })();

        match result {
            Ok(0) => {
                tracing::info!("[trash_cleanup] no expired trash items to clean");
            }
            Ok(count) => {
                tracing::info!("[trash_cleanup] cleaned {} expired trash item(s)", count);
                state.auto_sync.trigger_debounce();
                state.device_auto_sync.trigger_data_change();
            }
            Err(e) => {
                tracing::error!("[trash_cleanup] failed to clean expired trash: {}", e);
            }
        }
    });
}

/// Delete a page (section_type) and all its objects into trash.
/// If `page_object_id` is provided, the custom page object is also deleted into trash.
#[tauri::command]
pub async fn page_delete(
    state: State<'_, AppState>,
    account_id: String,
    section_type: String,
    page_object_id: Option<String>,
) -> Result<usize, String> {
    let vault = vault_handle(&state)?;

    // P114: 全表筛选 + 逐对象二次解密 + 回收站写入移入 spawn_blocking。
    let count = tokio::task::spawn_blocking(move || -> Result<usize, String> {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let period = load_trash_retention(&vault, &account_id);
        let retention_ms = retention_ms(&period);
        let mut count = 0usize;

        let mut page_name = String::new();

        // Delete the custom page object itself if provided
        if let Some(pid) = &page_object_id {
            if let Ok(Some(rec)) = vault.load_object(pid) {
                page_name = rec.name.clone();
                let trash = solosoul_vault::TrashItem {
                    id: format!("trash_{}", uuid::Uuid::new_v4()),
                    item_type: "page".to_string(),
                    original_id: rec.id.clone(),
                    original_parent_id: None,
                    original_section_type: Some(rec.section_type.clone()),
                    original_sort_order: None,
                    data: serde_json::to_vec(&serde_json::json!({
                        "id": rec.id,
                        "accountId": rec.account_id,
                        "typeId": rec.type_id,
                        "sectionType": rec.section_type,
                        "name": rec.name,
                        "iconName": rec.icon_name,
                        "parentId": rec.parent_id,
                        "childrenIds": rec.children_ids,
                        "properties": rec.properties,
                        "propertyLabels": rec.property_labels,
                        "sensitivityLevel": rec.sensitivity_level,
                        "tags": rec.tags_json,
                        "createdAt": rec.created_at,
                        "updatedAt": rec.updated_at,
                        "version": rec.version,
                        "templateId": rec.template_id,
                        "templateType": rec.template_type,
                        "contractTypeId": rec.contract_type_id,
                        "templateHash": rec.template_hash,
                    }))
                    .unwrap_or_default(),
                    deleted_at: now_ms,
                    expires_at: Some(now_ms + retention_ms),
                    deleted_by: "user".to_string(),
                    name_snapshot: rec.name.clone(),
                    icon_snapshot: Some(rec.icon_name),
                };
                let _ = vault.save_trash_item(&trash);
                vault.delete_object(pid, true)?;
                count += 1;
            }
        }

        // Delete all objects in this section_type
        let objects = vault
            .list_objects(&account_id, None, None, None, false, false)
            .map_err(|e| format!("list: {}", e))?;
        for obj in &objects {
            if obj.section_type == section_type || obj.collection_type == section_type {
                if page_name.is_empty() {
                    page_name = section_type.clone();
                }
                if let Ok(Some(rec)) = vault.load_object(&obj.id) {
                    let full_record = serde_json::json!({
                        "id": rec.id,
                        "accountId": rec.account_id,
                        "typeId": rec.type_id,
                        "sectionType": rec.section_type,
                        "name": rec.name,
                        "iconName": rec.icon_name,
                        "parentId": rec.parent_id,
                        "childrenIds": rec.children_ids,
                        "properties": rec.properties,
                        "propertyLabels": rec.property_labels,
                        "sensitivityLevel": rec.sensitivity_level,
                        "tags": rec.tags_json,
                        "createdAt": rec.created_at,
                        "updatedAt": rec.updated_at,
                        "version": rec.version,
                        "templateId": rec.template_id,
                        "templateType": rec.template_type,
                        "contractTypeId": rec.contract_type_id,
                        "templateHash": rec.template_hash,
                        "parentPageName": page_name,
                        "parentPageIcon": rec.icon_name,
                    });
                    let trash = solosoul_vault::TrashItem {
                        id: format!("trash_{}", uuid::Uuid::new_v4()),
                        item_type: "object".to_string(),
                        original_id: rec.id.clone(),
                        original_parent_id: rec.parent_id.clone(),
                        original_section_type: Some(rec.section_type.clone()),
                        original_sort_order: None,
                        data: serde_json::to_vec(&full_record).unwrap_or_default(),
                        deleted_at: now_ms,
                        expires_at: Some(now_ms + retention_ms),
                        deleted_by: "user".to_string(),
                        name_snapshot: rec.name.clone(),
                        icon_snapshot: Some(rec.icon_name.clone()),
                    };
                    let _ = vault.save_trash_item(&trash);
                    vault.delete_object(&obj.id, true)?;
                    count += 1;
                }
            }
        }

        let _ = vault.log_structured(
            "page_delete",
            "page",
            Some(&section_type),
            if page_name.is_empty() {
                None
            } else {
                Some(&page_name)
            },
            "user",
            Some(&format!("count={}", count)),
        );
        Ok(count)
    })
    .await
    .map_err(|e| format!("page_delete task failed: {e}"))??;

    state.auto_sync.trigger_debounce();
    state.device_auto_sync.trigger_data_change();
    Ok(count)
}
