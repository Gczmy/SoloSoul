use crate::commands::settings::resolve_ui_prefs_path;
use crate::commands::vault_handle;
use crate::state::AppState;
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
    vault.list_trash_items(None, since)
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
    Ok(())
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
}
