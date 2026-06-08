//! Object CRUD commands — P0-1: Real object storage layer
//!
//! Uses the `objects` table in solosoul_vault (separate from profiles).
//! Supports: type schemas, parent/child hierarchies, property storage,
//! soft-delete trash, and account-scoped queries.

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use solosoul_vault::{ObjectRecord, ObjectSummary};
use std::collections::HashMap;
use tauri::State;
use uuid::Uuid;

// ── Frontend-facing types ──────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectData {
    pub id: String,
    #[serde(rename = "accountId")]
    pub account_id: String,
    pub name: String,
    #[serde(rename = "collectionType")]
    pub collection_type: String,
    pub properties: serde_json::Value,
    #[serde(rename = "sensitivityLevel")]
    pub sensitivity_level: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "deletedAt")]
    pub deleted_at: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateObjectInput {
    #[serde(rename = "accountId")]
    pub account_id: String,
    pub name: String,
    #[serde(rename = "collectionType")]
    pub collection_type: String,
    pub properties: serde_json::Value,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    #[serde(rename = "iconName")]
    pub icon_name: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateObjectInput {
    pub name: String,
    pub properties: serde_json::Value,
    #[serde(rename = "sensitivityLevel")]
    pub sensitivity_level: Option<String>,
}

#[derive(Deserialize)]
pub struct ObjectFilter {
    #[serde(rename = "collectionType")]
    pub collection_type: Option<String>,
    #[serde(rename = "sensitivityLevel")]
    pub sensitivity_level: Option<String>,
    pub keyword: Option<String>,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
}

fn record_to_data(record: &ObjectRecord) -> ObjectData {
    ObjectData {
        id: record.id.clone(),
        account_id: record.account_id.clone(),
        name: record.name.clone(),
        collection_type: record.type_id.clone(),
        properties: record.properties.clone(),
        sensitivity_level: record.sensitivity_level.clone(),
        created_at: record.created_at.clone(),
        updated_at: record.updated_at.clone(),
        deleted_at: record.deleted_at.clone(),
    }
}

// ── Commands ───────────────────────────────────────────────

#[tauri::command]
pub async fn object_list(
    state: State<'_, AppState>,
    account_id: String,
    filter: Option<ObjectFilter>,
) -> Result<Vec<solosoul_vault::ObjectSummary>, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let type_id = filter.as_ref().and_then(|f| f.collection_type.as_deref());
    let parent_id = filter.as_ref().and_then(|f| f.parent_id.as_deref());
    let keyword = filter.as_ref().and_then(|f| f.keyword.as_deref());

    // Keyword search is done at SQL level — no N+1 queries
    vault.list_objects(&account_id, type_id, parent_id, keyword, false, false)
}

#[tauri::command]
pub async fn object_get(
    state: State<'_, AppState>,
    account_id: String,
    object_id: String,
) -> Result<Option<ObjectData>, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    match vault.load_object(&object_id)? {
        Some(rec) => {
            if rec.account_id != account_id || rec.is_deleted {
                Ok(None)
            } else {
                Ok(Some(record_to_data(&rec)))
            }
        }
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn object_create(
    state: State<'_, AppState>,
    input: CreateObjectInput,
) -> Result<ObjectData, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let now = chrono::Utc::now().to_rfc3339();
    let id = Uuid::new_v4().to_string();

    let record = ObjectRecord {
        id: id.clone(),
        account_id: input.account_id.clone(),
        type_id: input.collection_type.clone(),
        section_type: input.collection_type.clone(), // §25.1.3: page affiliation (currently mirrors type_id)
        name: input.name.clone(),
        icon_name: input.icon_name.unwrap_or_else(|| "document".to_string()),
        parent_id: input.parent_id.clone(),
        children_ids: vec![],
        properties: input.properties.clone(),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        created_at: now.clone(),
        updated_at: now,
        version: 1,
    };

    // If parent specified, update parent's children_ids
    if let Some(ref pid) = input.parent_id {
        if let Ok(Some(mut parent)) = vault.load_object(pid) {
            if !parent.children_ids.contains(&id) {
                parent.children_ids.push(id.clone());
                parent.updated_at = chrono::Utc::now().to_rfc3339();
                parent.version += 1;
                vault.save_object(&parent)?;
            }
        }
    }

    vault.save_object(&record)?;
    // §25.5 — Initial snapshot on create
    let snapshot_data = serde_json::to_vec(&serde_json::json!({
        "name": record.name, "tags": record.tags_json, "properties": record.properties,
    })).unwrap_or_default();
    let _ = vault.save_snapshot(&id, "user_edit", &snapshot_data, "Created");
    let _ = vault.log_structured("object_create", "object", Some(&id), Some(&input.name), "user", Some(&format!("section={}", input.collection_type)));
    Ok(record_to_data(&record))
}

#[tauri::command]
pub async fn object_update(
    state: State<'_, AppState>,
    object_id: String,
    input: UpdateObjectInput,
) -> Result<ObjectData, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let mut record = vault
        .load_object(&object_id)?
        .ok_or("Object not found".to_string())?;

    record.name = input.name;
    record.properties = input.properties;
    if let Some(sl) = input.sensitivity_level {
        record.sensitivity_level = sl;
    }
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;

    vault.save_object(&record)?;

    // §25.5 — Save snapshot for history
    let snapshot_data = serde_json::to_vec(&serde_json::json!({
        "name": record.name,
        "tags": record.tags_json,
        "properties": record.properties,
    })).unwrap_or_default();
    let _ = vault.save_snapshot(&object_id, "user_edit", &snapshot_data, "");

    let _ = vault.log_structured("object_update", "object", Some(&object_id), Some(&record.name), "user", Some(&format!("section={}", record.section_type)));
    Ok(record_to_data(&record))
}

#[tauri::command]
pub async fn object_delete(state: State<'_, AppState>, object_id: String) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    // Load retention period from preferences
    let account_id = svc.get_current_account().unwrap_or_default();
    let period = load_trash_retention(vault, &account_id);
    let retention_ms = retention_ms(&period);

    if let Ok(Some(rec)) = vault.load_object(&object_id) {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let obj_name = rec.name.clone();
        let obj_section = rec.section_type.clone();
        // Store complete ObjectRecord as data (§23.2.2)
        let full_record = serde_json::json!({
            "id": rec.id, "account_id": rec.account_id, "type_id": rec.type_id,
            "section_type": rec.section_type, "name": rec.name, "icon_name": rec.icon_name,
            "parent_id": rec.parent_id, "children_ids": rec.children_ids,
            "properties": rec.properties, "property_labels": rec.property_labels,
            "sensitivity_level": rec.sensitivity_level, "tags": rec.tags_json,
            "created_at": rec.created_at, "updated_at": rec.updated_at, "version": rec.version,
        });
        let trash = solosoul_vault::TrashItem {
            id: format!("trash_{}", uuid::Uuid::new_v4()),
            item_type: "object".to_string(),
            original_id: object_id.clone(),
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
        vault.delete_object(&object_id, true)?;
        let _ = vault.log_structured("object_delete", "object", Some(&object_id), Some(&obj_name), "user", Some(&format!("section={}", obj_section)));
        return Ok(());
    }
    Err("Object not found".to_string())
}

#[tauri::command]
pub async fn object_trash_list(
    state: State<'_, AppState>,
    _account_id: String,
) -> Result<Vec<solosoul_vault::TrashItemSummary>, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
    vault.list_trash_items(None, None)
}

/// Read the user's language setting from plaintext UI preferences.
fn get_ui_language(svc: &crate::services::vault_service::VaultService) -> String {
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
fn restored_suffix(language: &str) -> &'static str {
    match language {
        "zh-CN" => "（已恢复）",
        _ => " (restored)",
    }
}

/// Restore an object from trash. Handles conflict: if an object with the same ID
/// already exists, restore as a new copy with name appended " (restored)".
#[tauri::command]
pub async fn object_restore(state: State<'_, AppState>, trash_id: String) -> Result<String, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let trash = vault.get_trash_item(&trash_id)?.ok_or("Trash item not found")?;

    // Deserialize the full record from stored data
    let record_data: serde_json::Value = serde_json::from_slice(&trash.data)
        .map_err(|e| format!("Invalid trash data: {}", e))?;

    // Use original_section_type if present, fall back to stored data
    let target_section = trash.original_section_type
        .as_deref()
        .or(record_data["section_type"].as_str())
        .unwrap_or("identity");

    // Check if a non-deleted object with the same name exists in the target section (conflict)
    let account_id = record_data["account_id"].as_str().unwrap_or("imported");
    let objects = vault.list_objects(account_id, None, None, Some(&trash.name_snapshot), false, false).unwrap_or_default();
    let exists = objects.iter().any(|o| o.name == trash.name_snapshot && o.section_type == target_section);

    let lang = get_ui_language(&svc);
    let suffix = restored_suffix(&lang);

    let new_id = if exists {
        format!("{}_{}", trash.original_id, uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("restored"))
    } else {
        trash.original_id.clone()
    };

    let new_name = if exists {
        format!("{}{}", trash.name_snapshot, suffix)
    } else {
        trash.name_snapshot.clone()
    };

    let now = chrono::Utc::now().to_rfc3339();
    let record = solosoul_vault::ObjectRecord {
        id: new_id.clone(),
        account_id: record_data["account_id"].as_str().unwrap_or("imported").to_string(),
        type_id: record_data["type_id"].as_str().unwrap_or("note").to_string(),
        section_type: target_section.to_string(),
        name: new_name,
        icon_name: record_data["icon_name"].as_str().unwrap_or("document").to_string(),
        parent_id: record_data["parent_id"].as_str().map(String::from),
        children_ids: record_data["children_ids"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default(),
        properties: record_data["properties"].clone(),
        property_labels: if record_data["property_labels"].is_null() {
            None
        } else {
            Some(record_data["property_labels"].clone())
        },
        sensitivity_level: record_data["sensitivity_level"].as_str().unwrap_or("internal").to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: record_data["tags"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default(),
        created_at: record_data["created_at"].as_str().unwrap_or(&now).to_string(),
        updated_at: now,
        version: record_data["version"].as_u64().unwrap_or(1) as u32,
    };

    vault.save_object(&record)?;
    vault.delete_trash_item(&trash_id)?;
    let _ = vault.log_structured("object_restore", "object", Some(&trash.original_id), Some(&trash.name_snapshot), "user",
        Some(&format!("section={} was_conflict={}", target_section, exists)));

    Ok(new_id)
}

#[tauri::command]
pub async fn object_purge(state: State<'_, AppState>, object_id: String) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let (obj_name, obj_section) = vault.load_object(&object_id).ok().flatten()
        .map(|r| (r.name, r.section_type))
        .unwrap_or_default();
    vault.delete_object(&object_id, false)?;
    vault.delete_trash_item(&object_id).ok();
    let _ = vault.log_structured("object_purge", "object", Some(&object_id), Some(&obj_name), "user", Some(&format!("section={}", obj_section)));
    Ok(())
}

#[tauri::command]
pub async fn trash_restore(state: State<'_, AppState>, trash_id: String) -> Result<String, String> {
    object_restore(state, trash_id).await
}

/// Permanently delete a trash item (by trash_id → looks up original_id).
#[tauri::command]
pub async fn trash_permanent_delete(
    state: State<'_, AppState>,
    trash_id: String,
) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    if let Ok(Some(trash)) = vault.get_trash_item(&trash_id) {
        vault.delete_object(&trash.original_id, false)?;
        let _ = vault.log_structured("trash_permanent_delete", "trash_item", Some(&trash_id), Some(&trash.name_snapshot), "user",
            Some(&format!("original_id={}", trash.original_id)));
        vault.delete_trash_item(&trash_id).ok();
        return Ok(());
    }
    vault.delete_trash_item(&trash_id).ok();
    let _ = vault.log_structured("trash_permanent_delete", "trash_item", Some(&trash_id), None, "user", None);
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
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let now_ms = chrono::Utc::now().timestamp_millis();
    let period = load_trash_retention(vault, &account_id);
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
                    "id": rec.id, "account_id": rec.account_id, "type_id": rec.type_id,
                    "section_type": rec.section_type, "name": rec.name, "icon_name": rec.icon_name,
                    "properties": rec.properties,
                })).unwrap_or_default(),
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
    let objects = vault.list_objects(&account_id, None, None, None, false, false)
        .map_err(|e| format!("list: {}", e))?;
    for obj in &objects {
        if obj.section_type == section_type || obj.collection_type == section_type {
            if page_name.is_empty() {
                page_name = section_type.clone();
            }
            if let Ok(Some(rec)) = vault.load_object(&obj.id) {
                let full_record = serde_json::json!({
                    "id": rec.id, "account_id": rec.account_id, "type_id": rec.type_id,
                    "section_type": rec.section_type, "name": rec.name, "icon_name": rec.icon_name,
                    "properties": rec.properties,
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

    let _ = vault.log_structured("page_delete", "page", Some(&section_type), if page_name.is_empty() { None } else { Some(&page_name) }, "user", Some(&format!("count={}", count)));
    Ok(count)
}

/// Restore a page (all trash items with matching original_section_type).
#[tauri::command]
pub async fn page_restore(
    state: State<'_, AppState>,
    section_type: String,
) -> Result<usize, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
    let lang = get_ui_language(&svc);
    let suffix = restored_suffix(&lang);

    // Fetch all trash items and filter by original_section_type
    let all = vault.list_trash_items(None, None)?;
    let mut count = 0usize;
    for item in &all {
        if item.original_section_type.as_deref() == Some(&section_type) {
            // Use object_restore logic inline
            if let Ok(Some(trash)) = vault.get_trash_item(&item.id) {
                let record_data: serde_json::Value = serde_json::from_slice(&trash.data).unwrap_or_default();
                let account_id = record_data["account_id"].as_str().unwrap_or("");
                let active = vault.list_objects(account_id, None, None, Some(&trash.name_snapshot), false, false).unwrap_or_default();
                let exists = active.iter().any(|o| o.name == trash.name_snapshot);
                let new_id = if exists {
                    format!("{}_{}", trash.original_id, uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("restored"))
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
                        id: new_id,
                        account_id: record_data["account_id"].as_str().unwrap_or("imported").to_string(),
                        type_id: record_data["type_id"].as_str().unwrap_or("note").to_string(),
                        section_type: section_type.clone(),
                        name: new_name,
                        icon_name: record_data["icon_name"].as_str().unwrap_or("document").to_string(),
                        parent_id: record_data["parent_id"].as_str().map(String::from),
                        children_ids: record_data["children_ids"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default(),
                        properties: record_data["properties"].clone(),
                        property_labels: if record_data["property_labels"].is_null() { None } else { Some(record_data["property_labels"].clone()) },
                        sensitivity_level: record_data["sensitivity_level"].as_str().unwrap_or("internal").to_string(),
                        is_deleted: false, deleted_at: None,
                        tags_json: Vec::new(),
                        created_at: record_data["created_at"].as_str().unwrap_or(&now).to_string(),
                        updated_at: now,
                        version: record_data["version"].as_u64().unwrap_or(1) as u32,
                    };
                    if vault.save_object(&record).is_ok() {
                        vault.delete_trash_item(&item.id).ok();
                        count += 1;
                    }
                }
            }
        }
    }

    let _ = vault.log_structured("page_restore", "page", Some(&section_type), None, "user", Some(&format!("count={}", count)));
    Ok(count)
}

// ── Snapshot count badge ────────────────────────────────────

#[tauri::command]
pub async fn snapshot_count_batch(
    state: State<'_, AppState>,
    object_ids: Vec<String>,
) -> Result<std::collections::HashMap<String, usize>, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
    vault.count_snapshots_batch(&object_ids)
}

// ── Snapshot / History commands (§25.5) ─────────────────────

#[tauri::command]
pub async fn snapshot_get(
    state: State<'_, AppState>,
    object_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
    vault.list_snapshots(&object_id)
}

#[tauri::command]
pub async fn snapshot_get_data(
    state: State<'_, AppState>,
    snapshot_id: String,
) -> Result<Option<serde_json::Value>, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
    match vault.get_snapshot(&snapshot_id)? {
        Some(data) => serde_json::from_slice(&data).map(Some).map_err(|e| e.to_string()),
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn snapshot_list(
    state: State<'_, AppState>,
    object_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
    vault.list_snapshots(&object_id)
}

#[tauri::command]
pub async fn snapshot_rollback(
    state: State<'_, AppState>,
    snapshot_id: String,
    object_id: String,
) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    // Get snapshot data
    let data = vault.get_snapshot(&snapshot_id)?.ok_or("Snapshot not found")?;
    let snapshot: serde_json::Value = serde_json::from_slice(&data).map_err(|e| format!("Parse: {}", e))?;

    // Load current object and restore from snapshot
    let mut record = vault.load_object(&object_id)?.ok_or("Object not found")?;
    if let Some(name) = snapshot["name"].as_str() { record.name = name.to_string(); }
    if let Some(tags) = snapshot["tags"].as_array() {
        record.tags_json = tags.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
    }
    if !snapshot["properties"].is_null() { record.properties = snapshot["properties"].clone(); }
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record)?;

    // Save rollback snapshot
    let rollback_data = serde_json::to_vec(&serde_json::json!({
        "name": record.name, "tags": record.tags_json, "properties": record.properties,
    })).unwrap_or_default();
    let _ = vault.save_snapshot(&object_id, "rollback", &rollback_data, "Rolled back to previous version");
    let _ = vault.log_structured("object_rollback", "object", Some(&object_id), Some(&record.name), "user",
        Some(&format!("section={} snapshot={}", record.section_type, snapshot_id)));
    Ok(())
}

/// Get trash retention preferences for the current account.
#[tauri::command]
pub async fn trash_get_retention(
    state: State<'_, AppState>,
) -> Result<String, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
    let account_id = svc.get_current_account().ok_or("No account")?;
    if let Ok(Some(profile)) = vault.load_profile(&account_id) {
        if !profile.data.is_empty() {
            if let Ok(data) = serde_json::from_slice::<serde_json::Value>(&profile.data) {
                if let Some(ret) = data.pointer("/preferences/trashRetention").and_then(|v| v.as_str()) {
                    return Ok(ret.to_string());
                }
            }
        }
    }
    Ok("30d".to_string())
}

/// Set trash retention period.
#[tauri::command]
pub async fn trash_set_retention(
    state: State<'_, AppState>,
    period: String,
) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
    let account_id = svc.get_current_account().ok_or("No account")?;
    let mut profile = match vault.load_profile(&account_id) {
        Ok(Some(p)) => p,
        Ok(None) => solosoul_vault::Profile::new_with_id(&account_id, &account_id, Vec::new()),
        Err(e) => return Err(format!("Load: {}", e)),
    };
    let mut data: serde_json::Value = if profile.data.is_empty() {
        serde_json::Value::Object(serde_json::Map::new())
    } else {
        serde_json::from_slice(&profile.data).map_err(|e| format!("Parse: {}", e))?
    };
    let prefs = data.as_object_mut().ok_or("Invalid")?
        .entry("preferences".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    prefs["trashRetention"] = serde_json::Value::String(period.clone());
    profile.data = serde_json::to_vec(&data).map_err(|e| e.to_string())?;
    profile.updated_at = chrono::Utc::now();
    profile.version += 1;
    vault.save_profile(&profile)?;
    let _ = vault.log_structured("trash_set_retention", "preference", None, None, "user", Some(&format!("period={}", period)));
    Ok(())
}

/// Get full detail of a trash item including preview data.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashDetail {
    pub id: String,
    pub item_type: String,
    pub original_id: String,
    pub name: String,
    pub section_type: Option<String>,
    pub deleted_at: i64,
    pub expires_at: Option<i64>,
    pub deleted_by: String,
    pub remaining_days: Option<i64>,
    pub original_location: String,
    pub preview_properties: Vec<serde_json::Value>,
    /// Attachments parsed from stored data (active + soft-deleted)
    pub attachments: Vec<TrashAttachmentInfo>,
    pub deleted_attachments: Vec<TrashAttachmentInfo>,
    /// Snapshots from object_snapshots table
    pub snapshots: Vec<serde_json::Value>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashAttachmentInfo {
    pub id: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub created_at: String,
    pub deleted_at: Option<String>,
}

#[tauri::command]
pub async fn trash_get_detail(
    state: State<'_, AppState>,
    trash_id: String,
) -> Result<TrashDetail, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
    let trash = vault.get_trash_item(&trash_id)?.ok_or("Trash item not found")?;

    let remaining_days = trash.expires_at.map(|exp| {
        let diff_ms = exp - chrono::Utc::now().timestamp_millis();
        std::cmp::max(0, diff_ms / 86400000)
    });

    let original_location = match trash.item_type.as_str() {
        "page" => format!("Page: {}", trash.name_snapshot),
        "object" => trash.original_section_type
            .as_deref()
            .map(|st| format!("From page: {}", st))
            .unwrap_or_else(|| "From unknown page".to_string()),
        _ => "Unknown".to_string(),
    };

    let preview_properties: Vec<serde_json::Value> = (|| -> Option<Vec<serde_json::Value>> {
        let data: serde_json::Value = serde_json::from_slice(&trash.data).ok()?;
        let props = data.get("properties")?;
        let obj = props.as_object()?;
        Some(obj.iter()
            .filter(|(k, _)| !k.starts_with("__"))
            .take(5)
            .map(|(k, v)| serde_json::json!({"key": k, "value": v}))
            .collect())
    })().unwrap_or_default();

    // Parse attachments from stored data
    let parsed = (|| -> Option<(Vec<TrashAttachmentInfo>, Vec<TrashAttachmentInfo>)> {
        let data: serde_json::Value = serde_json::from_slice(&trash.data).ok()?;
        let props = data.get("properties")?;
        let atts: Vec<serde_json::Value> = props.get("__attachments")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        let mut active = Vec::new();
        let mut deleted = Vec::new();
        for a in &atts {
            let info = TrashAttachmentInfo {
                id: a["id"].as_str().unwrap_or("").to_string(),
                file_name: a["fileName"].as_str().unwrap_or("").to_string(),
                mime_type: a["mimeType"].as_str().unwrap_or("").to_string(),
                size_bytes: a["sizeBytes"].as_u64().unwrap_or(0),
                created_at: a["createdAt"].as_str().unwrap_or("").to_string(),
                deleted_at: if a["deletedAt"].is_null() { None } else { a["deletedAt"].as_str().map(String::from) },
            };
            if info.deleted_at.is_some() { deleted.push(info); } else { active.push(info); }
        }
        Some((active, deleted))
    })();
    let (attachments, deleted_attachments) = parsed.unwrap_or_default();

    // Fetch snapshots
    let snapshots = vault.list_snapshots(&trash.original_id).unwrap_or_default();

    Ok(TrashDetail {
        id: trash.id,
        item_type: trash.item_type,
        original_id: trash.original_id,
        name: trash.name_snapshot,
        section_type: trash.original_section_type,
        deleted_at: trash.deleted_at,
        expires_at: trash.expires_at,
        deleted_by: trash.deleted_by,
        remaining_days,
        original_location,
        preview_properties,
        attachments,
        deleted_attachments,
        snapshots,
    })
}

/// Load trash retention period from profile preferences.
fn load_trash_retention(vault: &solosoul_vault::VaultStore, account_id: &str) -> String {
    if let Ok(Some(profile)) = vault.load_profile(account_id) {
        if !profile.data.is_empty() {
            if let Ok(data) = serde_json::from_slice::<serde_json::Value>(&profile.data) {
                if let Some(ret) = data.pointer("/preferences/trashRetention").and_then(|v| v.as_str()) {
                    return ret.to_string();
                }
            }
        }
    }
    "30d".to_string()
}

/// Compute retention ms from period string.
fn retention_ms(period: &str) -> i64 {
    match period {
        "60d" => 60 * 24 * 3600 * 1000i64,
        "half_year" => 180 * 24 * 3600 * 1000i64,
        "one_year" => 365 * 24 * 3600 * 1000i64,
        "never" => i64::MAX,
        _ => 30 * 24 * 3600 * 1000i64,
    }
}
