//! Object CRUD commands — P0-1: Real object storage layer
//!
//! Uses the `objects` table in solosoul_vault (separate from profiles).
//! Supports: type schemas, parent/child hierarchies, property storage,
//! soft-delete trash, and account-scoped queries.

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use solosoul_vault::{ObjectRecord, ObjectSummary};
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
    let _ = vault.log_action("object_create", &format!("id={} name={} type={}", id, input.name, input.collection_type));
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
    let _ = vault.log_action("object_update", &format!("id={} name={}", object_id, record.name));
    Ok(record_to_data(&record))
}

#[tauri::command]
pub async fn object_delete(state: State<'_, AppState>, object_id: String) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
    // Create trash item snapshot before soft-deleting (§23.2)
    if let Ok(Some(rec)) = vault.load_object(&object_id) {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let retention_ms = 30 * 24 * 3600 * 1000i64; // default 30 days
        let trash = solosoul_vault::TrashItem {
            id: format!("trash_{}", uuid::Uuid::new_v4()),
            item_type: "object".to_string(),
            original_id: object_id.clone(),
            original_parent_id: rec.parent_id.clone(),
            original_sort_order: None,
            data: serde_json::to_vec(&rec.properties).unwrap_or_default(),
            deleted_at: now_ms,
            expires_at: Some(now_ms + retention_ms),
            deleted_by: "user".to_string(),
            name_snapshot: rec.name.clone(),
            icon_snapshot: Some(rec.icon_name.clone()),
        };
        let _ = vault.save_trash_item(&trash);
    }
    vault.delete_object(&object_id, true)?;
    let _ = vault.log_action("object_delete", &format!("id={} (soft)", object_id));
    Ok(())
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

#[tauri::command]
pub async fn object_restore(state: State<'_, AppState>, trash_id: String) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
    // Find trash item, restore original object, remove from trash
    if let Ok(Some(trash)) = vault.get_trash_item(&trash_id) {
        vault.restore_object(&trash.original_id)?;
        vault.delete_trash_item(&trash_id)?;
    }
    let _ = vault.log_action("object_restore", &format!("trash_id={}", trash_id));
    Ok(())
}

#[tauri::command]
pub async fn object_purge(state: State<'_, AppState>, object_id: String) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
    vault.delete_object(&object_id, false)?;
    // Also remove any trash entry for this object
    vault.delete_trash_item(&object_id).ok();
    let _ = vault.log_action("object_purge", &format!("id={}", object_id));
    Ok(())
}

#[tauri::command]
pub async fn trash_restore(state: State<'_, AppState>, trash_id: String) -> Result<(), String> {
    object_restore(state, trash_id).await
}
