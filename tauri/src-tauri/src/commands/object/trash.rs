use crate::commands::vault_handle;
use crate::state::AppState;
use tauri::State;

use super::*;
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
fn get_ui_language(svc: &solosoul_core::vault_service::VaultService) -> String {
    let path = svc.base_path().join("ui_preferences.json");
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

/// Get the "(restored)" suffix localized to the user's language.
pub(crate) fn restored_suffix(language: &str) -> &'static str {
    match language {
        "zh-CN" => "（已恢复）",
        _ => " (restored)",
    }
}

/// Restore an object from trash. Handles conflict: if an object with the same ID
/// already exists, restore as a new copy with name appended " (restored)".
#[tauri::command]
pub async fn object_restore(
    state: State<'_, AppState>,
    trash_id: String,
    lang: Option<String>,
) -> Result<String, String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();

    let trash = vault
        .get_trash_item(&trash_id)?
        .ok_or("Trash item not found")?;

    // Deserialize the full record from stored data
    let record_data: serde_json::Value =
        serde_json::from_slice(&trash.data).map_err(|e| format!("Invalid trash data: {}", e))?;

    // Use original_section_type if present, fall back to stored data
    let target_section = trash
        .original_section_type
        .as_deref()
        .or(record_data["section_type"].as_str())
        .unwrap_or("identity");

    // Check if a non-deleted object with the same name exists in the target section (conflict)
    let account_id = record_data["account_id"].as_str().unwrap_or("imported");
    let objects = vault
        .list_objects(
            account_id,
            None,
            None,
            Some(&trash.name_snapshot),
            false,
            false,
        )
        .unwrap_or_default();
    let exists = objects
        .iter()
        .any(|o| o.name == trash.name_snapshot && o.section_type == target_section);

    let fallback_lang = get_ui_language(&svc);
    let lang = lang.as_deref().unwrap_or(&fallback_lang);
    let suffix = restored_suffix(lang);

    let new_id = if exists {
        format!(
            "{}_{}",
            trash.original_id,
            uuid::Uuid::new_v4()
                .to_string()
                .split('-')
                .next()
                .unwrap_or("restored")
        )
    } else {
        trash.original_id.clone()
    };

    let new_name = if exists {
        format!("{}{}", trash.name_snapshot, suffix)
    } else {
        trash.name_snapshot.clone()
    };

    // §13.10.3: 从模板继承 contract_type_id
    let restore_contract_type_id =
        inherit_contract_type_id(vault, record_data["template_id"].as_str());

    let now = chrono::Utc::now().to_rfc3339();
    let record = solosoul_vault::ObjectRecord {
        contract_type_id: restore_contract_type_id,
        id: new_id.clone(),
        account_id: record_data["account_id"]
            .as_str()
            .unwrap_or("imported")
            .to_string(),
        type_id: record_data["type_id"]
            .as_str()
            .unwrap_or("note")
            .to_string(),
        section_type: target_section.to_string(),
        name: new_name,
        icon_name: record_data["icon_name"]
            .as_str()
            .unwrap_or("document")
            .to_string(),
        parent_id: record_data["parent_id"].as_str().map(String::from),
        children_ids: record_data["children_ids"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        properties: record_data["properties"].clone(),
        property_labels: if record_data["property_labels"].is_null() {
            None
        } else {
            Some(record_data["property_labels"].clone())
        },
        sensitivity_level: record_data["sensitivity_level"]
            .as_str()
            .unwrap_or("internal")
            .to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: record_data["tags"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        template_id: record_data["template_id"].as_str().map(String::from),
        template_type: record_data["template_type"].as_str().map(String::from),
        template_hash: record_data["template_hash"].as_str().map(String::from),
        ignored_template_hash: record_data["ignored_template_hash"].as_str().map(String::from),
        created_at: record_data["created_at"]
            .as_str()
            .unwrap_or(&now)
            .to_string(),
        updated_at: now,
        version: record_data["version"].as_u64().unwrap_or(1) as u32,
    };

    vault.save_object(&record)?;
    // If restored under a new ID (conflict), copy history snapshots so they aren't lost.
    if new_id != trash.original_id {
        let _ = vault.copy_snapshots(&trash.original_id, &new_id);
    }
    vault.delete_trash_item(&trash_id)?;
    let _ = vault.log_structured(
        "object_restore",
        "object",
        Some(&trash.original_id),
        Some(&trash.name_snapshot),
        "user",
        Some(&format!(
            "section={} was_conflict={}",
            target_section, exists
        )),
    );

    Ok(new_id)
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
    state: State<'_, AppState>,
    trash_id: String,
    lang: Option<String>,
) -> Result<String, String> {
    object_restore(state, trash_id, lang).await
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
/// If `page_object_id` is provided, the custom page object is also deleted.
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
                    "account_id": rec.account_id,
                    "type_id": rec.type_id,
                    "section_type": rec.section_type,
                    "name": rec.name,
                    "icon_name": rec.icon_name,
                    "parent_id": rec.parent_id,
                    "children_ids": rec.children_ids,
                    "properties": rec.properties,
                    "property_labels": rec.property_labels,
                    "sensitivity_level": rec.sensitivity_level,
                    "tags": rec.tags_json,
                    "created_at": rec.created_at,
                    "updated_at": rec.updated_at,
                    "version": rec.version,
                    "template_id": rec.template_id,
                    "template_type": rec.template_type,
                    "contract_type_id": rec.contract_type_id,
                    "template_hash": rec.template_hash,
                }))
                .unwrap_or_default(),
                deleted_at: now_ms,
                expires_at: Some(now_ms + retention_ms),
                deleted_by: "user".to_string(),
                name_snapshot: rec.name.clone(),
                icon_snapshot: Some(rec.icon_name.clone()),
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
                    "account_id": rec.account_id,
                    "type_id": rec.type_id,
                    "section_type": rec.section_type,
                    "name": rec.name,
                    "icon_name": rec.icon_name,
                    "parent_id": rec.parent_id,
                    "children_ids": rec.children_ids,
                    "properties": rec.properties,
                    "property_labels": rec.property_labels,
                    "sensitivity_level": rec.sensitivity_level,
                    "tags": rec.tags_json,
                    "created_at": rec.created_at,
                    "updated_at": rec.updated_at,
                    "version": rec.version,
                    "template_id": rec.template_id,
                    "template_type": rec.template_type,
                    "contract_type_id": rec.contract_type_id,
                    "template_hash": rec.template_hash,
                });
                let trash = solosoul_vault::TrashItem {
                    id: format!("trash_{}", uuid::Uuid::new_v4()),
                    item_type: "object".to_string(),
                    original_id: rec.id.clone(),
                    original_parent_id: None,
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

/// Restore a page (all trash items with matching original_section_type).
#[tauri::command]
pub async fn page_restore(
    state: State<'_, AppState>,
    section_type: String,
    lang: Option<String>,
) -> Result<usize, String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();
    let fallback_lang = get_ui_language(&svc);
    let lang = lang.as_deref().unwrap_or(&fallback_lang);
    let suffix = restored_suffix(lang);

    // Fetch all trash items and filter by original_section_type
    let all = vault.list_trash_items(None, None)?;
    let mut count = 0usize;
    for item in &all {
        if item.original_section_type.as_deref() == Some(&section_type) {
            // Use object_restore logic inline
            if let Ok(Some(trash)) = vault.get_trash_item(&item.id) {
                let record_data: serde_json::Value =
                    serde_json::from_slice(&trash.data).unwrap_or_default();
                let account_id = record_data["account_id"].as_str().unwrap_or("");
                let active = vault
                    .list_objects(
                        account_id,
                        None,
                        None,
                        Some(&trash.name_snapshot),
                        false,
                        false,
                    )
                    .unwrap_or_default();
                let exists = active.iter().any(|o| o.name == trash.name_snapshot);
                let new_id = if exists {
                    format!(
                        "{}_{}",
                        trash.original_id,
                        uuid::Uuid::new_v4()
                            .to_string()
                            .split('-')
                            .next()
                            .unwrap_or("restored")
                    )
                } else {
                    trash.original_id.clone()
                };
                let new_name = if exists {
                    format!("{}{}", trash.name_snapshot, suffix)
                } else {
                    trash.name_snapshot.clone()
                };
                if let Ok(record_data) = serde_json::from_slice::<serde_json::Value>(&trash.data) {
                    let now = chrono::Utc::now().to_rfc3339();
                    let record = solosoul_vault::ObjectRecord {
                        contract_type_id: None,
                        id: new_id.clone(),
                        account_id: record_data["account_id"]
                            .as_str()
                            .unwrap_or("imported")
                            .to_string(),
                        type_id: record_data["type_id"]
                            .as_str()
                            .unwrap_or("note")
                            .to_string(),
                        section_type: section_type.clone(),
                        name: new_name,
                        icon_name: record_data["icon_name"]
                            .as_str()
                            .unwrap_or("document")
                            .to_string(),
                        parent_id: record_data["parent_id"].as_str().map(String::from),
                        children_ids: record_data["children_ids"]
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        properties: record_data["properties"].clone(),
                        property_labels: if record_data["property_labels"].is_null() {
                            None
                        } else {
                            Some(record_data["property_labels"].clone())
                        },
                        sensitivity_level: record_data["sensitivity_level"]
                            .as_str()
                            .unwrap_or("internal")
                            .to_string(),
                        is_deleted: false,
                        deleted_at: None,
                        tags_json: Vec::new(),
                        template_id: record_data["template_id"].as_str().map(String::from),
                        template_type: record_data["template_type"].as_str().map(String::from),
                        template_hash: record_data["template_hash"].as_str().map(String::from),
                        ignored_template_hash: record_data["ignored_template_hash"].as_str().map(String::from),
                        created_at: record_data["created_at"]
                            .as_str()
                            .unwrap_or(&now)
                            .to_string(),
                        updated_at: now,
                        version: record_data["version"].as_u64().unwrap_or(1) as u32,
                    };
                    if vault.save_object(&record).is_ok() {
                        // Copy snapshots when restoring under a new ID (conflict) so history isn't lost.
                        if new_id != trash.original_id {
                            let _ = vault.copy_snapshots(&trash.original_id, &new_id);
                        }
                        vault.delete_trash_item(&item.id).ok();
                        count += 1;
                    }
                }
            }
        }
    }

    let _ = vault.log_structured(
        "page_restore",
        "page",
        Some(&section_type),
        None,
        "user",
        Some(&format!("count={}", count)),
    );
    Ok(count)
}
