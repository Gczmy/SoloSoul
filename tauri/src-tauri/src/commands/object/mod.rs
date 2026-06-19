//! Object CRUD commands — P0-1: Real object storage layer
//!
//! Uses the `objects` table in solosoul_vault (separate from profiles).
//! Supports: type schemas, parent/child hierarchies, property storage,
//! soft-delete trash, and account-scoped queries.

use crate::commands::{current_account_optional, vault_handle};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use solosoul_vault::ObjectRecord;
use tauri::State;
use uuid::Uuid;

/// 一天的毫秒数。
pub const MS_PER_DAY: i64 = 24 * 3600 * 1000;

/// 回收站保留期选项。
pub const RETENTION_30D: &str = "30d";
pub const RETENTION_60D: &str = "60d";
pub const RETENTION_HALF_YEAR: &str = "half_year";
pub const RETENTION_ONE_YEAR: &str = "one_year";
pub const RETENTION_NEVER: &str = "never";
pub const DEFAULT_RETENTION: &str = RETENTION_30D;

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

/// 从模板继承 contract_type_id。
/// 若创建对象时指定了模板 ID，且对应模板存在 `contract_type_id`，则自动复制到对象上。
pub fn inherit_contract_type_id(
    vault: &solosoul_vault::VaultStore,
    template_id: Option<&str>,
) -> Option<String> {
    template_id.and_then(|tid| {
        vault
            .load_user_template(tid)
            .ok()
            .flatten()
            .and_then(|t| t.contract_type_id)
    })
}

pub fn record_to_data(record: &ObjectRecord) -> ObjectData {
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
    let vault = vault_handle(&state)?;

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
    let vault = vault_handle(&state)?;

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
    let vault = vault_handle(&state)?;

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

    // §13.10.3: 从模板继承 contract_type_id
    let contract_type_id = inherit_contract_type_id(&vault, input.template_id.as_deref());

    let record = ObjectRecord {
        contract_type_id,
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
    let vault = vault_handle(&state)?;

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
        let _ = crate::services::llm_context::bump_public_data_version(&vault, &account_id);
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
    let vault = vault_handle(&state)?;

    // Load retention period from preferences
    let account_id = current_account_optional(&state).unwrap_or_default();
    let period = load_trash_retention(&vault, &account_id);
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
            "contract_type_id": rec.contract_type_id,
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

// ── Sub-modules ─────────────────────────────────────────────

pub mod snapshot;
#[cfg(test)]
mod tests;
pub mod trash;

// Re-export all command functions so that `commands::object::xxx` paths remain valid.
pub use snapshot::*;
pub use trash::*;
