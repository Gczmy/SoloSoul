//! Object CRUD commands — P0-1: Real object storage layer
//!
//! Uses the `objects` table in solosoul_vault (separate from profiles).
//! Supports: type schemas, parent/child hierarchies, property storage,
//! soft-delete trash, and account-scoped queries.

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use solosoul_vault::ObjectRecord;
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
    #[serde(rename = "templateId")]
    pub template_id: Option<String>,
    #[serde(rename = "templateType")]
    pub template_type: Option<String>,
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
    #[serde(rename = "templateId")]
    pub template_id: Option<String>,
    #[serde(rename = "templateType")]
    pub template_type: Option<String>,
    /// Optional client-provided ID. If given, the backend uses it instead of generating a new UUID.
    /// This ensures the client's optimistic state stays in sync with the database record.
    pub id: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateObjectInput {
    pub name: String,
    pub properties: serde_json::Value,
    #[serde(rename = "sensitivityLevel")]
    pub sensitivity_level: Option<String>,
    #[serde(rename = "iconName")]
    pub icon_name: Option<String>,
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
    #[serde(rename = "includeDeleted")]
    pub include_deleted: Option<bool>,
}

fn record_to_data(record: &ObjectRecord) -> ObjectData {
    ObjectData {
        id: record.id.clone(),
        account_id: record.account_id.clone(),
        name: record.name.clone(),
        collection_type: record.type_id.clone(),
        properties: record.properties.clone(),
        sensitivity_level: record.sensitivity_level.clone(),
        template_id: record.template_id.clone(),
        template_type: record.template_type.clone(),
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
    let svc = state.vault_service.read().unwrap();
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();

    let type_id = filter.as_ref().and_then(|f| f.collection_type.as_deref());
    let parent_id = filter.as_ref().and_then(|f| f.parent_id.as_deref());
    let keyword = filter.as_ref().and_then(|f| f.keyword.as_deref());

    let include_deleted = filter
        .as_ref()
        .and_then(|f| f.include_deleted)
        .unwrap_or(false);

    // Keyword search is done at SQL level — no N+1 queries
    vault.list_objects(
        &account_id,
        type_id,
        parent_id,
        keyword,
        include_deleted,
        false,
    )
}

#[tauri::command]
pub async fn object_get(
    state: State<'_, AppState>,
    account_id: String,
    object_id: String,
) -> Result<Option<ObjectData>, String> {
    let svc = state.vault_service.read().unwrap();
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();

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
    let svc = state.vault_service.read().unwrap();
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();

    let now = chrono::Utc::now().to_rfc3339();
    let id = input
        .id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // R025: 禁止客户端指定已存在活跃对象的 ID 进行覆盖
    if input.id.is_some() {
        if let Ok(Some(existing)) = vault.load_object(&id) {
            if !existing.is_deleted {
                return Err(format!("Object with ID '{}' already exists", id));
            }
        }
    }

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
        template_id: input.template_id.clone(),
        template_type: input.template_type.clone(),
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
    }))
    .unwrap_or_default();
    let _ = vault.save_snapshot(&id, "user_edit", &snapshot_data, "Created");
    let is_page = input.collection_type == "page";
    let _ = vault.log_structured(
        if is_page {
            "page_create"
        } else {
            "object_create"
        },
        if is_page { "page" } else { "object" },
        Some(&id),
        Some(&input.name),
        "user",
        Some(&format!("section={}", input.collection_type)),
    );
    Ok(record_to_data(&record))
}

#[tauri::command]
pub async fn object_update(
    state: State<'_, AppState>,
    object_id: String,
    input: UpdateObjectInput,
) -> Result<ObjectData, String> {
    let svc = state.vault_service.read().unwrap();
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();

    let mut record = vault
        .load_object(&object_id)?
        .ok_or("Object not found".to_string())?;

    let old_sensitivity = record.sensitivity_level.clone();
    record.name = input.name;
    record.properties = input.properties;
    if let Some(sl) = input.sensitivity_level {
        record.sensitivity_level = sl;
    }
    if let Some(icon_name) = input.icon_name {
        record.icon_name = icon_name;
    }
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;

    vault.save_object(&record)?;

    // §28: bump public_data_version when sensitivity changes to/from public
    let new_sensitivity = &record.sensitivity_level;
    if old_sensitivity != *new_sensitivity
        && (old_sensitivity == "public" || new_sensitivity == "public")
    {
        let account_id = record.account_id.clone();
        let _ = crate::services::llm_context::bump_public_data_version(vault, &account_id);
    }

    // §25.5 — Save snapshot for history
    let snapshot_data = serde_json::to_vec(&serde_json::json!({
        "name": record.name,
        "tags": record.tags_json,
        "properties": record.properties,
    }))
    .unwrap_or_default();
    let _ = vault.save_snapshot(&object_id, "user_edit", &snapshot_data, "");

    let _ = vault.log_structured(
        "object_update",
        "object",
        Some(&object_id),
        Some(&record.name),
        "user",
        Some(&format!("section={}", record.section_type)),
    );
    Ok(record_to_data(&record))
}

#[tauri::command]
pub async fn object_delete(state: State<'_, AppState>, object_id: String) -> Result<(), String> {
    let svc = state.vault_service.read().unwrap();
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();

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
            "template_id": rec.template_id, "template_type": rec.template_type,
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
        let _ = vault.log_structured(
            "object_delete",
            "object",
            Some(&object_id),
            Some(&obj_name),
            "user",
            Some(&format!("section={}", obj_section)),
        );
        return Ok(());
    }
    Err("Object not found".to_string())
}

#[tauri::command]
pub async fn object_trash_list(
    state: State<'_, AppState>,
    account_id: String,
    since: Option<i64>,
) -> Result<Vec<solosoul_vault::TrashItemSummary>, String> {
    let _ = account_id;
    let svc = state.vault_service.read().unwrap();
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();
    vault.list_trash_items(None, since)
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
pub async fn object_restore(
    state: State<'_, AppState>,
    trash_id: String,
    lang: Option<String>,
) -> Result<String, String> {
    let svc = state.vault_service.read().unwrap();
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

    let now = chrono::Utc::now().to_rfc3339();
    let record = solosoul_vault::ObjectRecord {
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
    let svc = state.vault_service.read().unwrap();
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();

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
    let svc = state.vault_service.read().unwrap();
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();

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
    let svc = state.vault_service.read().unwrap();
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();

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
    let svc = state.vault_service.read().unwrap();
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

// ── Snapshot count badge ────────────────────────────────────

#[tauri::command]
pub async fn snapshot_count_batch(
    state: State<'_, AppState>,
    object_ids: Vec<String>,
) -> Result<std::collections::HashMap<String, usize>, String> {
    let svc = state.vault_service.read().unwrap();
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();
    vault.count_snapshots_batch(&object_ids)
}

// ── Snapshot / History commands (§25.5) ─────────────────────

#[tauri::command]
pub async fn snapshot_get(
    state: State<'_, AppState>,
    object_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    let svc = state.vault_service.read().unwrap();
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();
    vault.list_snapshots(&object_id)
}

#[tauri::command]
pub async fn snapshot_get_data(
    state: State<'_, AppState>,
    snapshot_id: String,
) -> Result<Option<serde_json::Value>, String> {
    let svc = state.vault_service.read().unwrap();
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();
    match vault.get_snapshot(&snapshot_id)? {
        Some(data) => serde_json::from_slice(&data)
            .map(Some)
            .map_err(|e| e.to_string()),
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn snapshot_list(
    state: State<'_, AppState>,
    object_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    let svc = state.vault_service.read().unwrap();
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();
    vault.list_snapshots(&object_id)
}

#[tauri::command]
pub async fn snapshot_rollback(
    state: State<'_, AppState>,
    snapshot_id: String,
    object_id: String,
) -> Result<(), String> {
    let svc = state.vault_service.read().unwrap();
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();

    // Get snapshot data
    let data = vault
        .get_snapshot(&snapshot_id)?
        .ok_or("Snapshot not found")?;
    let snapshot: serde_json::Value =
        serde_json::from_slice(&data).map_err(|e| format!("Parse: {}", e))?;

    // Load current object and restore from snapshot
    let mut record = vault.load_object(&object_id)?.ok_or("Object not found")?;
    if let Some(name) = snapshot["name"].as_str() {
        record.name = name.to_string();
    }
    if let Some(tags) = snapshot["tags"].as_array() {
        record.tags_json = tags
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
    }
    if !snapshot["properties"].is_null() {
        record.properties = snapshot["properties"].clone();
    }
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record)?;

    // Save rollback snapshot
    let rollback_data = serde_json::to_vec(&serde_json::json!({
        "name": record.name, "tags": record.tags_json, "properties": record.properties,
    }))
    .unwrap_or_default();
    let _ = vault.save_snapshot(
        &object_id,
        "rollback",
        &rollback_data,
        "Rolled back to previous version",
    );
    let _ = vault.log_structured(
        "object_rollback",
        "object",
        Some(&object_id),
        Some(&record.name),
        "user",
        Some(&format!(
            "section={} snapshot={}",
            record.section_type, snapshot_id
        )),
    );
    Ok(())
}

/// Get trash retention preferences for the current account.
#[tauri::command]
pub async fn trash_get_retention(state: State<'_, AppState>) -> Result<String, String> {
    let svc = state.vault_service.read().unwrap();
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();
    let account_id = svc.get_current_account().ok_or("No account")?;
    if let Ok(Some(profile)) = vault.load_profile(&account_id) {
        if !profile.data.is_empty() {
            if let Ok(data) = serde_json::from_slice::<serde_json::Value>(&profile.data) {
                if let Some(ret) = data
                    .pointer("/preferences/trashRetention")
                    .and_then(|v| v.as_str())
                {
                    return Ok(ret.to_string());
                }
            }
        }
    }
    Ok("30d".to_string())
}

/// Set trash retention period.
#[tauri::command]
pub async fn trash_set_retention(state: State<'_, AppState>, period: String) -> Result<(), String> {
    let svc = state.vault_service.read().unwrap();
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();
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
    let prefs = data
        .as_object_mut()
        .ok_or("Invalid")?
        .entry("preferences".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    prefs["trashRetention"] = serde_json::Value::String(period.clone());
    profile.data = serde_json::to_vec(&data).map_err(|e| e.to_string())?;
    profile.updated_at = chrono::Utc::now();
    profile.version += 1;
    vault.save_profile(&profile)?;
    let _ = vault.log_structured(
        "trash_set_retention",
        "preference",
        None,
        None,
        "user",
        Some(&format!("period={}", period)),
    );
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
    pub template_id: Option<String>,
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
    let svc = state.vault_service.read().unwrap();
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();
    let trash = vault
        .get_trash_item(&trash_id)?
        .ok_or("Trash item not found")?;

    let remaining_days = trash.expires_at.map(|exp| {
        let diff_ms = exp - chrono::Utc::now().timestamp_millis();
        std::cmp::max(0, diff_ms / 86400000)
    });

    let original_location = match trash.item_type.as_str() {
        "page" => format!("Page: {}", trash.name_snapshot),
        "object" => trash
            .original_section_type
            .as_deref()
            .map(|st| format!("From page: {}", st))
            .unwrap_or_else(|| "From unknown page".to_string()),
        "template" => format!("Template: {}", trash.name_snapshot),
        _ => "Unknown".to_string(),
    };

    let preview_properties: Vec<serde_json::Value> = if trash.item_type == "template" {
        (|| -> Option<Vec<serde_json::Value>> {
            let data: serde_json::Value = serde_json::from_slice(&trash.data).ok()?;
            let props = data.get("properties")?.as_array()?;
            Some(
                props
                    .iter()
                    .filter_map(|p| {
                        let name = p.get("name")?.as_str()?;
                        let prop_type = p.get("type")?.as_str()?;
                        let sensitivity = p
                            .get("sensitivityLevel")
                            .and_then(|v| v.as_str())
                            .unwrap_or("internal");
                        Some(serde_json::json!({
                            "key": name,
                            "value": prop_type,
                            "type": prop_type,
                            "sensitivityLevel": sensitivity
                        }))
                    })
                    .collect(),
            )
        })()
        .unwrap_or_default()
    } else {
        (|| -> Option<Vec<serde_json::Value>> {
            let data: serde_json::Value = serde_json::from_slice(&trash.data).ok()?;
            let props = data.get("properties")?.as_object()?;
            // Load template to get field metadata and ordering
            let tpl_fields: Vec<(String, String, String, String)> =
                if let Some(tpl_id) = data.get("template_id").and_then(|v| v.as_str()) {
                    vault
                        .load_user_template(tpl_id)
                        .ok()
                        .flatten()
                        .map(|tpl| {
                            tpl.properties
                                .into_iter()
                                .map(|p| {
                                    let sens = p
                                        .sensitivity_level
                                        .unwrap_or_else(|| "internal".to_string());
                                    let ptype = serde_json::to_string(&p.prop_type)
                                        .ok()
                                        .and_then(|s| serde_json::from_str::<String>(&s).ok())
                                        .unwrap_or_else(|| "text".to_string());
                                    (p.id, p.name, ptype, sens)
                                })
                                .collect()
                        })
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };
            let mut result = Vec::new();
            // Follow template order
            for (field_id, field_name, field_type, sensitivity) in &tpl_fields {
                if let Some(v) = props.get(field_id) {
                    result.push(serde_json::json!({
                        "key": field_name,
                        "value": v,
                        "type": field_type,
                        "sensitivityLevel": sensitivity
                    }));
                }
            }
            // Fallback: any properties not in template (orphaned fields)
            let known: std::collections::HashSet<String> =
                tpl_fields.iter().map(|(id, _, _, _)| id.clone()).collect();
            for (k, v) in props.iter() {
                if !k.starts_with("__") && !known.contains(k) {
                    result.push(serde_json::json!({"key": k, "value": v}));
                }
            }
            Some(result.into_iter().take(5).collect())
        })()
        .unwrap_or_default()
    };

    // Parse attachments from stored data
    let parsed = (|| -> Option<(Vec<TrashAttachmentInfo>, Vec<TrashAttachmentInfo>)> {
        let data: serde_json::Value = serde_json::from_slice(&trash.data).ok()?;
        let props = data.get("properties")?;
        let atts: Vec<serde_json::Value> = props
            .get("__attachments")
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
                deleted_at: if a["deletedAt"].is_null() {
                    None
                } else {
                    a["deletedAt"].as_str().map(String::from)
                },
            };
            if info.deleted_at.is_some() {
                deleted.push(info);
            } else {
                active.push(info);
            }
        }
        Some((active, deleted))
    })();
    let (attachments, deleted_attachments) = parsed.unwrap_or_default();

    // Fetch snapshots
    let snapshots = vault.list_snapshots(&trash.original_id).unwrap_or_default();

    // Extract template_id from stored data
    let template_id = (|| -> Option<String> {
        let data: serde_json::Value = serde_json::from_slice(&trash.data).ok()?;
        data.get("template_id")
            .and_then(|v| v.as_str())
            .map(String::from)
    })();

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
        template_id,
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
                if let Some(ret) = data
                    .pointer("/preferences/trashRetention")
                    .and_then(|v| v.as_str())
                {
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

#[cfg(test)]
mod tests {
    use super::*;
    use solosoul_vault::{ObjectRecord, Profile, TrashItem, VaultConfig, VaultStore};
    use tempfile::TempDir;

    fn setup_vault() -> (VaultStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let config =
            VaultConfig::new("test_account", dir.path().to_path_buf()).with_data_key([0x42u8; 32]);
        let vault = VaultStore::open(config).unwrap();
        (vault, dir)
    }

    #[test]
    fn test_record_to_data_conversion() {
        let record = ObjectRecord {
            id: "obj-1".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Test Object".to_string(),
            icon_name: "document".to_string(),
            parent_id: Some("parent-1".to_string()),
            children_ids: vec!["child-1".to_string()],
            properties: serde_json::json!({"key": "value"}),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec!["tag1".to_string()],
            template_id: None,
            template_type: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-02T00:00:00Z".to_string(),
            version: 1,
        };
        let data = record_to_data(&record);
        assert_eq!(data.id, "obj-1");
        assert_eq!(data.account_id, "acc-1");
        assert_eq!(data.collection_type, "note");
        assert_eq!(data.name, "Test Object");
        assert_eq!(data.sensitivity_level, "internal");
        assert_eq!(data.deleted_at, None);
    }

    #[test]
    fn test_object_data_serde_roundtrip() {
        let original = ObjectData {
            id: "obj-1".to_string(),
            account_id: "acc-1".to_string(),
            name: "Test".to_string(),
            collection_type: "note".to_string(),
            properties: serde_json::json!({"foo": "bar"}),
            sensitivity_level: "public".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            deleted_at: Some("2024-02-01T00:00:00Z".to_string()),
            template_id: None,
            template_type: None,
        };
        let json = serde_json::to_string(&original).unwrap();
        assert!(json.contains("accountId"));
        assert!(json.contains("collectionType"));
        let restored: ObjectData = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, original.id);
        assert_eq!(restored.name, original.name);
    }

    #[test]
    fn test_restored_suffix_localization() {
        assert_eq!(restored_suffix("zh-CN"), "（已恢复）");
        assert_eq!(restored_suffix("en-US"), " (restored)");
        assert_eq!(restored_suffix("ja-JP"), " (restored)");
        assert_eq!(restored_suffix(""), " (restored)");
    }

    #[test]
    fn test_retention_ms_parsing() {
        assert_eq!(retention_ms("30d"), 30 * 24 * 3600 * 1000i64);
        assert_eq!(retention_ms("60d"), 60 * 24 * 3600 * 1000i64);
        assert_eq!(retention_ms("half_year"), 180 * 24 * 3600 * 1000i64);
        assert_eq!(retention_ms("one_year"), 365 * 24 * 3600 * 1000i64);
        assert_eq!(retention_ms("never"), i64::MAX);
        assert_eq!(retention_ms("unknown"), 30 * 24 * 3600 * 1000i64);
    }

    #[test]
    fn test_object_filter_deserialization() {
        let json = r#"{"collectionType":"note","keyword":"test"}"#;
        let filter: ObjectFilter = serde_json::from_str(json).unwrap();
        assert_eq!(filter.collection_type, Some("note".to_string()));
        assert_eq!(filter.keyword, Some("test".to_string()));
        assert_eq!(filter.sensitivity_level, None);
        assert_eq!(filter.parent_id, None);
    }

    #[test]
    fn test_create_object_input_deserialization() {
        let json = r#"{"accountId":"acc-1","name":"My Note","collectionType":"note","properties":{},"parentId":"parent-1","iconName":"star"}"#;
        let input: CreateObjectInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.account_id, "acc-1");
        assert_eq!(input.icon_name, Some("star".to_string()));
        assert_eq!(input.parent_id, Some("parent-1".to_string()));
    }

    #[test]
    fn test_vault_object_save_and_load() {
        let (vault, _dir) = setup_vault();
        let record = ObjectRecord {
            id: "obj-1".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Test Note".to_string(),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({"content": "hello"}),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
        };
        vault.save_object(&record).unwrap();
        let loaded = vault.load_object("obj-1").unwrap().unwrap();
        assert_eq!(loaded.name, "Test Note");
        assert_eq!(loaded.properties, serde_json::json!({"content": "hello"}));
    }

    #[test]
    fn test_vault_object_list_and_soft_delete() {
        let (vault, _dir) = setup_vault();
        for i in 0..3 {
            let record = ObjectRecord {
                id: format!("obj-{}", i),
                account_id: "acc-1".to_string(),
                type_id: "note".to_string(),
                section_type: "identity".to_string(),
                name: format!("Note {}", i),
                icon_name: "document".to_string(),
                parent_id: None,
                children_ids: vec![],
                properties: serde_json::Value::Object(serde_json::Map::new()),
                property_labels: None,
                sensitivity_level: "internal".to_string(),
                is_deleted: false,
                deleted_at: None,
                tags_json: vec![],
                template_id: None,
                template_type: None,
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
                version: 1,
            };
            vault.save_object(&record).unwrap();
        }
        let all = vault
            .list_objects("acc-1", None, None, None, false, false)
            .unwrap();
        assert_eq!(all.len(), 3);

        vault.delete_object("obj-1", true).unwrap();
        let remaining = vault
            .list_objects("acc-1", None, None, None, false, false)
            .unwrap();
        assert_eq!(remaining.len(), 2);

        let deleted = vault
            .list_objects("acc-1", None, None, None, false, true)
            .unwrap();
        assert_eq!(deleted.len(), 1);
    }

    #[test]
    fn test_update_object_input_deserialization() {
        let json =
            r#"{"name":"Updated Name","properties":{"key":"val"},"sensitivityLevel":"private"}"#;
        let input: UpdateObjectInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, "Updated Name");
        assert_eq!(input.sensitivity_level, Some("private".to_string()));
    }

    #[test]
    fn test_object_create_with_parent() {
        let (vault, _dir) = setup_vault();
        let parent = ObjectRecord {
            id: "parent-1".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Parent".to_string(),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({}),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
        };
        vault.save_object(&parent).unwrap();

        let child = ObjectRecord {
            id: "child-1".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Child".to_string(),
            icon_name: "document".to_string(),
            parent_id: Some("parent-1".to_string()),
            children_ids: vec![],
            properties: serde_json::json!({}),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
        };
        vault.save_object(&child).unwrap();

        // Simulate object_create parent update logic
        if let Ok(Some(mut p)) = vault.load_object("parent-1") {
            if !p.children_ids.contains(&"child-1".to_string()) {
                p.children_ids.push("child-1".to_string());
                p.updated_at = chrono::Utc::now().to_rfc3339();
                p.version += 1;
                vault.save_object(&p).unwrap();
            }
        }

        let updated_parent = vault.load_object("parent-1").unwrap().unwrap();
        assert!(updated_parent.children_ids.contains(&"child-1".to_string()));
    }

    #[test]
    fn test_trash_item_lifecycle() {
        let (vault, _dir) = setup_vault();
        let trash = TrashItem {
            id: "trash_001".to_string(),
            item_type: "object".to_string(),
            original_id: "obj-1".to_string(),
            original_parent_id: Some("parent-1".to_string()),
            original_section_type: Some("identity".to_string()),
            original_sort_order: Some(1),
            data: serde_json::to_vec(&serde_json::json!({"name": "Test"})).unwrap_or_default(),
            deleted_at: 1234567890,
            expires_at: Some(1234567890 + 30 * 24 * 3600 * 1000),
            deleted_by: "user".to_string(),
            name_snapshot: "Test Object".to_string(),
            icon_snapshot: Some("document".to_string()),
        };
        vault.save_trash_item(&trash).unwrap();
        let loaded = vault.get_trash_item("trash_001").unwrap().unwrap();
        assert_eq!(loaded.original_id, "obj-1");
        assert_eq!(loaded.name_snapshot, "Test Object");
        assert_eq!(loaded.item_type, "object");
        assert_eq!(loaded.icon_snapshot, Some("document".to_string()));
        vault.delete_trash_item("trash_001").unwrap();
        assert!(vault.get_trash_item("trash_001").unwrap().is_none());
    }

    #[test]
    fn test_object_soft_delete_with_trash_item() {
        let (vault, _dir) = setup_vault();
        let record = ObjectRecord {
            id: "obj-del-1".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Delete Me".to_string(),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({"content": "hello"}),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
        };
        vault.save_object(&record).unwrap();

        let now_ms = chrono::Utc::now().timestamp_millis();
        let full_record = serde_json::json!({
            "id": record.id, "account_id": record.account_id, "type_id": record.type_id,
            "section_type": record.section_type, "name": record.name, "icon_name": record.icon_name,
            "parent_id": record.parent_id, "children_ids": record.children_ids,
            "properties": record.properties, "property_labels": record.property_labels,
            "sensitivity_level": record.sensitivity_level, "tags": record.tags_json,
            "created_at": record.created_at, "updated_at": record.updated_at, "version": record.version,
        });
        let trash_id = format!("trash_{}", uuid::Uuid::new_v4());
        let trash = TrashItem {
            id: trash_id.clone(),
            item_type: "object".to_string(),
            original_id: record.id.clone(),
            original_parent_id: record.parent_id.clone(),
            original_section_type: Some(record.section_type.clone()),
            original_sort_order: None,
            data: serde_json::to_vec(&full_record).unwrap_or_default(),
            deleted_at: now_ms,
            expires_at: Some(now_ms + retention_ms("30d")),
            deleted_by: "user".to_string(),
            name_snapshot: record.name.clone(),
            icon_snapshot: Some(record.icon_name.clone()),
        };
        vault.save_trash_item(&trash).unwrap();
        vault.delete_object(&record.id, true).unwrap();

        let trash_list = vault.list_trash_items(None, None).unwrap();
        assert_eq!(trash_list.len(), 1);
        assert_eq!(trash_list[0].name, "Delete Me");

        let loaded_trash = vault.get_trash_item(&trash_id).unwrap().unwrap();
        assert_eq!(loaded_trash.original_id, record.id);

        let active = vault
            .list_objects("acc-1", None, None, None, false, false)
            .unwrap();
        assert_eq!(active.len(), 0);
    }

    #[test]
    fn test_hard_delete_purges_object() {
        let (vault, _dir) = setup_vault();
        let record = ObjectRecord {
            id: "obj-purge-1".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Purge Me".to_string(),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({"content": "bye"}),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
        };
        vault.save_object(&record).unwrap();

        let trash = TrashItem {
            id: "trash_purge_1".to_string(),
            item_type: "object".to_string(),
            original_id: record.id.clone(),
            original_parent_id: None,
            original_section_type: Some(record.section_type.clone()),
            original_sort_order: None,
            data: serde_json::to_vec(&serde_json::json!({"name": "Purge Me"})).unwrap_or_default(),
            deleted_at: chrono::Utc::now().timestamp_millis(),
            expires_at: None,
            deleted_by: "user".to_string(),
            name_snapshot: record.name.clone(),
            icon_snapshot: None,
        };
        vault.save_trash_item(&trash).unwrap();

        // Hard delete object and trash item (object_purge equivalent)
        vault.delete_object(&record.id, false).unwrap();
        vault.delete_trash_item(&trash.id).unwrap();

        assert!(vault.load_object(&record.id).unwrap().is_none());
        assert!(vault.get_trash_item(&trash.id).unwrap().is_none());
    }

    #[test]
    fn test_trash_permanent_delete_flow() {
        let (vault, _dir) = setup_vault();
        let record = ObjectRecord {
            id: "obj-perm-1".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Perm Delete".to_string(),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({}),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
        };
        vault.save_object(&record).unwrap();

        let trash = TrashItem {
            id: "trash_perm_1".to_string(),
            item_type: "object".to_string(),
            original_id: record.id.clone(),
            original_parent_id: None,
            original_section_type: Some(record.section_type.clone()),
            original_sort_order: None,
            data: serde_json::to_vec(&serde_json::json!({"name": "Perm Delete"}))
                .unwrap_or_default(),
            deleted_at: chrono::Utc::now().timestamp_millis(),
            expires_at: None,
            deleted_by: "user".to_string(),
            name_snapshot: record.name.clone(),
            icon_snapshot: None,
        };
        vault.save_trash_item(&trash).unwrap();
        vault.delete_object(&record.id, true).unwrap();

        // Simulate trash_permanent_delete command logic
        if let Ok(Some(t)) = vault.get_trash_item("trash_perm_1") {
            vault.delete_object(&t.original_id, false).unwrap();
            vault.delete_trash_item("trash_perm_1").unwrap();
        }

        assert!(vault.load_object(&record.id).unwrap().is_none());
        assert!(vault.get_trash_item("trash_perm_1").unwrap().is_none());
    }

    #[test]
    fn test_snapshot_operations() {
        let (vault, _dir) = setup_vault();
        let record = ObjectRecord {
            id: "obj-snap-1".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Snapshot Test".to_string(),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({"content": "v1"}),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
        };
        vault.save_object(&record).unwrap();

        let snap1 = serde_json::to_vec(&serde_json::json!({
            "name": "Snapshot Test", "tags": [], "properties": {"content": "v1"}
        }))
        .unwrap();
        vault
            .save_snapshot(&record.id, "user_edit", &snap1, "Created")
            .unwrap();

        let snap2 = serde_json::to_vec(&serde_json::json!({
            "name": "Snapshot Test Updated", "tags": [], "properties": {"content": "v2"}
        }))
        .unwrap();
        vault
            .save_snapshot(&record.id, "user_edit", &snap2, "")
            .unwrap();

        let snapshots = vault.list_snapshots(&record.id).unwrap();
        assert_eq!(snapshots.len(), 2);

        let snap_id = snapshots[0]["id"].as_str().unwrap();
        let data = vault.get_snapshot(snap_id).unwrap().unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&data).unwrap();
        assert!(parsed.get("name").is_some());

        let counts = vault.count_snapshots_batch(&[record.id.clone()]).unwrap();
        assert_eq!(counts.get(&record.id), Some(&2));
    }

    #[test]
    fn test_copy_snapshots() {
        let (vault, _dir) = setup_vault();
        let record = ObjectRecord {
            id: "obj-copy-1".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Copy Snap Test".to_string(),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({}),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
        };
        vault.save_object(&record).unwrap();

        let snap = serde_json::to_vec(&serde_json::json!({"name": "v1"})).unwrap();
        vault
            .save_snapshot(&record.id, "user_edit", &snap, "")
            .unwrap();
        vault
            .save_snapshot(&record.id, "user_edit", &snap, "")
            .unwrap();

        let new_id = "obj-copy-2";
        vault.copy_snapshots(&record.id, new_id).unwrap();

        let original_snaps = vault.list_snapshots(&record.id).unwrap();
        let copied_snaps = vault.list_snapshots(new_id).unwrap();
        assert_eq!(original_snaps.len(), 2);
        assert_eq!(copied_snaps.len(), 2);
    }

    #[test]
    fn test_snapshot_rollback_via_vault() {
        let (vault, _dir) = setup_vault();
        let record = ObjectRecord {
            id: "obj-roll-1".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Original".to_string(),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({"content": "v1"}),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec!["tag1".to_string()],
            template_id: None,
            template_type: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
        };
        vault.save_object(&record).unwrap();

        // Save snapshot
        let snap = serde_json::to_vec(&serde_json::json!({
            "name": "Original", "tags": ["tag1"], "properties": {"content": "v1"}
        }))
        .unwrap();
        vault
            .save_snapshot(&record.id, "user_edit", &snap, "")
            .unwrap();

        // Update object
        let mut updated = vault.load_object(&record.id).unwrap().unwrap();
        updated.name = "Updated".to_string();
        updated.properties = serde_json::json!({"content": "v2"});
        updated.tags_json = vec!["tag2".to_string()];
        updated.version += 1;
        vault.save_object(&updated).unwrap();

        // Rollback: load snapshot and restore (snapshot_rollback logic)
        let snapshots = vault.list_snapshots(&record.id).unwrap();
        let snap_id = snapshots[0]["id"].as_str().unwrap();
        let data = vault.get_snapshot(snap_id).unwrap().unwrap();
        let snapshot: serde_json::Value = serde_json::from_slice(&data).unwrap();

        let mut rec = vault.load_object(&record.id).unwrap().unwrap();
        if let Some(name) = snapshot["name"].as_str() {
            rec.name = name.to_string();
        }
        if let Some(tags) = snapshot["tags"].as_array() {
            rec.tags_json = tags
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }
        if !snapshot["properties"].is_null() {
            rec.properties = snapshot["properties"].clone();
        }
        rec.updated_at = chrono::Utc::now().to_rfc3339();
        rec.version += 1;
        vault.save_object(&rec).unwrap();

        // Save rollback snapshot
        let rollback_data = serde_json::to_vec(&serde_json::json!({
            "name": rec.name, "tags": rec.tags_json, "properties": rec.properties,
        }))
        .unwrap_or_default();
        let _ = vault.save_snapshot(
            &record.id,
            "rollback",
            &rollback_data,
            "Rolled back to previous version",
        );

        // Verify rollback
        let rolled = vault.load_object(&record.id).unwrap().unwrap();
        assert_eq!(rolled.name, "Original");
        assert_eq!(rolled.properties, serde_json::json!({"content": "v1"}));
        assert_eq!(rolled.tags_json, vec!["tag1"]);

        let final_snaps = vault.list_snapshots(&record.id).unwrap();
        assert_eq!(final_snaps.len(), 2);
    }

    #[test]
    fn test_page_section_delete_and_restore() {
        let (vault, _dir) = setup_vault();
        let section = "work";
        for i in 0..3 {
            let record = ObjectRecord {
                id: format!("obj-page-{}", i),
                account_id: "acc-1".to_string(),
                type_id: "note".to_string(),
                section_type: section.to_string(),
                name: format!("Work Note {}", i),
                icon_name: "document".to_string(),
                parent_id: None,
                children_ids: vec![],
                properties: serde_json::json!({"idx": i}),
                property_labels: None,
                sensitivity_level: "internal".to_string(),
                is_deleted: false,
                deleted_at: None,
                tags_json: vec![],
                template_id: None,
                template_type: None,
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
                version: 1,
            };
            vault.save_object(&record).unwrap();
        }

        // Simulate page_delete: create trash items and soft delete all in section
        let now_ms = chrono::Utc::now().timestamp_millis();
        for i in 0..3 {
            let id = format!("obj-page-{}", i);
            let rec = vault.load_object(&id).unwrap().unwrap();
            let full_record = serde_json::json!({
                "id": rec.id, "account_id": rec.account_id, "type_id": rec.type_id,
                "section_type": rec.section_type, "name": rec.name, "icon_name": rec.icon_name,
                "properties": rec.properties,
            });
            let trash = TrashItem {
                id: format!("trash_page_{}", i),
                item_type: "object".to_string(),
                original_id: id.clone(),
                original_parent_id: None,
                original_section_type: Some(section.to_string()),
                original_sort_order: None,
                data: serde_json::to_vec(&full_record).unwrap_or_default(),
                deleted_at: now_ms,
                expires_at: Some(now_ms + retention_ms("30d")),
                deleted_by: "user".to_string(),
                name_snapshot: rec.name.clone(),
                icon_snapshot: Some(rec.icon_name.clone()),
            };
            vault.save_trash_item(&trash).unwrap();
            vault.delete_object(&id, true).unwrap();
        }

        // Verify active list is empty
        let active = vault
            .list_objects("acc-1", None, None, None, false, false)
            .unwrap();
        assert_eq!(active.len(), 0);

        // Verify trash items exist
        let trash_items = vault.list_trash_items(None, None).unwrap();
        assert_eq!(trash_items.len(), 3);

        // Restore via VaultStore restore_object and delete trash items
        for item in &trash_items {
            let full = vault.get_trash_item(&item.id).unwrap().unwrap();
            vault.restore_object(&full.original_id).unwrap();
            vault.delete_trash_item(&item.id).unwrap();
        }

        // Verify restored
        let restored_active = vault
            .list_objects("acc-1", None, None, None, false, false)
            .unwrap();
        assert_eq!(restored_active.len(), 3);
    }

    #[test]
    fn test_page_restore_from_trash_reconstruction() {
        let (vault, _dir) = setup_vault();
        let section = "finance";
        for i in 0..2 {
            let record = ObjectRecord {
                id: format!("obj-fin-{}", i),
                account_id: "acc-1".to_string(),
                type_id: "note".to_string(),
                section_type: section.to_string(),
                name: format!("Finance {}", i),
                icon_name: "document".to_string(),
                parent_id: None,
                children_ids: vec![],
                properties: serde_json::json!({"amount": i * 100}),
                property_labels: None,
                sensitivity_level: "internal".to_string(),
                is_deleted: false,
                deleted_at: None,
                tags_json: vec![],
                template_id: None,
                template_type: None,
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
                version: 1,
            };
            vault.save_object(&record).unwrap();
        }

        // Soft delete and trash
        let now_ms = chrono::Utc::now().timestamp_millis();
        for i in 0..2 {
            let id = format!("obj-fin-{}", i);
            let rec = vault.load_object(&id).unwrap().unwrap();
            let data = serde_json::json!({
                "id": rec.id, "account_id": rec.account_id, "type_id": rec.type_id,
                "section_type": rec.section_type, "name": rec.name, "icon_name": rec.icon_name,
                "properties": rec.properties, "parent_id": rec.parent_id,
                "children_ids": rec.children_ids, "property_labels": rec.property_labels,
                "sensitivity_level": rec.sensitivity_level, "tags": rec.tags_json,
                "created_at": rec.created_at, "updated_at": rec.updated_at, "version": rec.version,
            });
            let trash = TrashItem {
                id: format!("trash_fin_{}", i),
                item_type: "object".to_string(),
                original_id: id.clone(),
                original_parent_id: rec.parent_id.clone(),
                original_section_type: Some(section.to_string()),
                original_sort_order: None,
                data: serde_json::to_vec(&data).unwrap_or_default(),
                deleted_at: now_ms,
                expires_at: Some(now_ms + retention_ms("30d")),
                deleted_by: "user".to_string(),
                name_snapshot: rec.name.clone(),
                icon_snapshot: Some(rec.icon_name.clone()),
            };
            vault.save_trash_item(&trash).unwrap();
            vault.delete_object(&id, true).unwrap();
        }

        // Replicate page_restore logic inline
        let all_trash = vault.list_trash_items(None, None).unwrap();
        let mut count = 0usize;
        for item in &all_trash {
            if item.original_section_type.as_deref() == Some(section) {
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
                        format!("{}{}", trash.name_snapshot, restored_suffix("en-US"))
                    } else {
                        trash.name_snapshot.clone()
                    };
                    if let Ok(record_data) =
                        serde_json::from_slice::<serde_json::Value>(&trash.data)
                    {
                        let now = chrono::Utc::now().to_rfc3339();
                        let record = ObjectRecord {
                            id: new_id.clone(),
                            account_id: record_data["account_id"]
                                .as_str()
                                .unwrap_or("imported")
                                .to_string(),
                            type_id: record_data["type_id"]
                                .as_str()
                                .unwrap_or("note")
                                .to_string(),
                            section_type: section.to_string(),
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
                            template_id: None,
                            template_type: None,
                            created_at: record_data["created_at"]
                                .as_str()
                                .unwrap_or(&now)
                                .to_string(),
                            updated_at: now,
                            version: record_data["version"].as_u64().unwrap_or(1) as u32,
                        };
                        if vault.save_object(&record).is_ok() {
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

        assert_eq!(count, 2);
        let restored = vault
            .list_objects("acc-1", None, None, None, false, false)
            .unwrap();
        assert_eq!(restored.len(), 2);
    }

    #[test]
    fn test_trash_detail_serialization() {
        let detail = TrashDetail {
            id: "trash_001".to_string(),
            item_type: "object".to_string(),
            original_id: "obj-1".to_string(),
            name: "Test Object".to_string(),
            section_type: Some("identity".to_string()),
            deleted_at: 1234567890,
            expires_at: Some(1234567890000),
            deleted_by: "user".to_string(),
            remaining_days: Some(29),
            original_location: "From page: identity".to_string(),
            template_id: None,
            preview_properties: vec![serde_json::json!({"key": "title", "value": "Hello"})],
            attachments: vec![TrashAttachmentInfo {
                id: "att-1".to_string(),
                file_name: "file.pdf".to_string(),
                mime_type: "application/pdf".to_string(),
                size_bytes: 1024,
                created_at: "2024-01-01T00:00:00Z".to_string(),
                deleted_at: None,
            }],
            deleted_attachments: vec![],
            snapshots: vec![serde_json::json!({"id": "snap-1", "timestamp": 0})],
        };
        let json = serde_json::to_string(&detail).unwrap();
        assert!(json.contains("\"id\":\"trash_001\""));
        assert!(json.contains("\"itemType\":\"object\""));
        assert!(json.contains("\"originalId\":\"obj-1\""));
        assert!(json.contains("\"name\":\"Test Object\""));
        assert!(json.contains("\"sectionType\":\"identity\""));
        assert!(json.contains("\"deletedAt\":1234567890"));
        assert!(json.contains("\"remainingDays\":29"));
        assert!(json.contains("\"originalLocation\":\"From page: identity\""));
        assert!(json.contains("\"previewProperties\""));
        assert!(json.contains("\"attachments\""));
        assert!(json.contains("\"deletedAttachments\""));
        assert!(json.contains("\"snapshots\""));
    }

    #[test]
    fn test_load_trash_retention_default() {
        let (vault, _dir) = setup_vault();
        let period = load_trash_retention(&vault, "nonexistent");
        assert_eq!(period, "30d");
    }

    #[test]
    fn test_load_trash_retention_from_profile() {
        let (vault, _dir) = setup_vault();
        let account_id = "acc-retention";
        let prefs = serde_json::json!({
            "preferences": {
                "trashRetention": "60d"
            }
        });
        let profile = Profile::new_with_id(account_id, "Test", serde_json::to_vec(&prefs).unwrap());
        vault.save_profile(&profile).unwrap();
        let period = load_trash_retention(&vault, account_id);
        assert_eq!(period, "60d");
    }
}
