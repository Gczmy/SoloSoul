//! Object CRUD commands — P0-1: Real object storage layer
//!
//! Uses the `objects` table in solosoul_vault (separate from profiles).
//! Supports: type schemas, parent/child hierarchies, property storage,
//! soft-delete trash, and account-scoped queries.

use crate::commands::{current_account_optional, vault_handle};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use solosoul_vault::{ObjectRecord, PropertyType};
use tauri::State;
use uuid::Uuid;

/// 校验 properties 中的 dynamic_group 字段值。
/// 要求：数组；每个元素含 id/name/type/value；type 可被解析；不超出 maxItems；类型在 allowedTypes 内。
pub fn validate_dynamic_groups(properties: &serde_json::Value) -> Result<(), String> {
    let fields = match properties.get("__fields").and_then(|v| v.as_object()) {
        Some(f) => f,
        None => return Ok(()),
    };
    let props = match properties.as_object() {
        Some(p) => p,
        None => return Ok(()),
    };

    for (key, field_def) in fields {
        if field_def.get("type").and_then(|v| v.as_str()) != Some("dynamic_group") {
            continue;
        }
        let value = match props.get(key) {
            Some(v) => v,
            None => continue,
        };
        let items = value
            .as_array()
            .ok_or_else(|| format!("字段 '{}' 是动态字段组，其值必须是数组", key))?;

        let allowed: Option<Vec<&str>> = field_def
            .get("allowedTypes")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect());
        let max_items = field_def
            .get("maxItems")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize);

        if let Some(max) = max_items {
            if items.len() > max {
                return Err(format!(
                    "字段 '{}' 最多允许 {} 个子字段，当前 {} 个",
                    key,
                    max,
                    items.len()
                ));
            }
        }

        for (idx, item) in items.iter().enumerate() {
            let obj = item
                .as_object()
                .ok_or_else(|| format!("字段 '{}' 的第 {} 个子字段必须是对象", key, idx + 1))?;
            for required in ["id", "name", "type", "value"] {
                if !obj.contains_key(required) {
                    return Err(format!(
                        "字段 '{}' 的第 {} 个子字段缺少 '{}'",
                        key,
                        idx + 1,
                        required
                    ));
                }
            }
            let child_type = obj
                .get("type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("字段 '{}' 的第 {} 个子字段 type 无效", key, idx + 1))?;
            if PropertyType::parse(child_type).is_none() {
                return Err(format!(
                    "字段 '{}' 的第 {} 个子字段类型 '{}' 不存在",
                    key,
                    idx + 1,
                    child_type
                ));
            }
            if let Some(ref allowed) = allowed {
                if !allowed.contains(&child_type) {
                    return Err(format!(
                        "字段 '{}' 的第 {} 个子字段类型 '{}' 不在允许列表 {:?} 中",
                        key,
                        idx + 1,
                        child_type,
                        allowed
                    ));
                }
            }
        }
    }
    Ok(())
}

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
    #[serde(rename = "propertyLabels")]
    pub property_labels: Option<serde_json::Value>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "deletedAt")]
    pub deleted_at: Option<String>,
    #[serde(rename = "contractTypeId")]
    pub contract_type_id: Option<String>,
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

/// 从模板继承字段级敏感度映射。
/// 返回 `{ "fieldId": "sensitive|critical|internal|public" }` 的 JSON 对象。
/// 此映射保存在对象的 `property_labels` 中，即使模板被删除，对象仍保留自己的敏感度副本。
pub fn inherit_property_labels(
    vault: &solosoul_vault::VaultStore,
    template_id: Option<&str>,
) -> Option<serde_json::Value> {
    // 复用 inherit_template_properties 避免重复加载模板
    let (labels, _) = inherit_template_properties(vault, template_id);
    labels
}

/// 内部合并函数：一次加载模板，同时返回 property_labels 和 __fields。
fn inherit_template_properties(
    vault: &solosoul_vault::VaultStore,
    template_id: Option<&str>,
) -> (Option<serde_json::Value>, serde_json::Value) {
    let Some(tid) = template_id else {
        return (None, serde_json::Value::Null);
    };
    let tpl = match vault.load_user_template(tid).ok().flatten() {
        Some(t) => t,
        None => return (None, serde_json::Value::Null),
    };

    let mut labels_map = serde_json::Map::new();
    let mut fields_map = serde_json::Map::new();

    for prop in &tpl.properties {
        // property_labels
        if let Some(ref sl) = prop.sensitivity_level {
            labels_map.insert(prop.id.clone(), serde_json::Value::String(sl.clone()));
        }

        // __fields
        let mut field_def = serde_json::Map::new();
        field_def.insert(
            "name".to_string(),
            serde_json::Value::String(prop.name.clone()),
        );
        field_def.insert(
            "type".to_string(),
            serde_json::Value::String(prop.prop_type.as_str().to_string()),
        );
        if let Some(ref opts) = prop.options {
            field_def.insert(
                "options".to_string(),
                serde_json::Value::Array(
                    opts.iter()
                        .map(|o| serde_json::Value::String(o.clone()))
                        .collect(),
                ),
            );
        }
        if let Some(ref da) = prop.deprecated_at {
            field_def.insert(
                "deprecatedAt".to_string(),
                serde_json::Value::String(da.clone()),
            );
        }
        if let Some(ref cf) = prop.contract_field {
            field_def.insert("contractField".to_string(), serde_json::Value::Bool(*cf));
        }
        if let PropertyType::DynamicGroup = prop.prop_type {
            if let Some(ref allowed) = prop.allowed_types {
                field_def.insert(
                    "allowedTypes".to_string(),
                    serde_json::Value::Array(
                        allowed
                            .iter()
                            .map(|t| serde_json::Value::String(t.as_str().to_string()))
                            .collect(),
                    ),
                );
            }
            if let Some(max) = prop.max_items {
                field_def.insert(
                    "maxItems".to_string(),
                    serde_json::Value::Number(max.into()),
                );
            }
        }
        fields_map.insert(prop.id.clone(), serde_json::Value::Object(field_def));
    }

    let labels = if labels_map.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(labels_map))
    };
    let fields = if fields_map.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::Value::Object(fields_map)
    };
    (labels, fields)
}

/// 从模板继承字段定义（字段名 + 类型等），嵌入到 `properties` 的 `__fields` 键中。
/// 即使模板被删除，对象仍保留字段定义副本。
pub(crate) fn inherit_property_fields(
    vault: &solosoul_vault::VaultStore,
    template_id: Option<&str>,
) -> serde_json::Value {
    // 复用 inherit_template_properties 避免重复加载模板
    let (_, fields) = inherit_template_properties(vault, template_id);
    fields
}

/// 将 `__fields` 注入到 properties JSON 对象中。
pub(crate) fn inject_property_fields(
    properties: &mut serde_json::Value,
    fields: &serde_json::Value,
) {
    if fields.is_null() {
        return;
    }
    if let Some(obj) = properties.as_object_mut() {
        obj.insert("__fields".to_string(), fields.clone());
    }
}

/// 将模板元信息（名称、图标等）注入到 properties JSON 对象中，
/// 即使模板被删除，对象仍能显示模板名称。
pub(crate) fn inject_template_meta(
    vault: &solosoul_vault::VaultStore,
    template_id: Option<&str>,
    properties: &mut serde_json::Value,
) {
    let Some(tid) = template_id else { return };
    let Some(tpl) = vault.load_user_template(tid).ok().flatten() else {
        return;
    };
    if let Some(obj) = properties.as_object_mut() {
        obj.insert(
            "__templateName".to_string(),
            serde_json::Value::String(tpl.name.clone()),
        );
    }
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
        property_labels: record.property_labels.clone(),
        contract_type_id: record.contract_type_id.clone(),
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
    // §Bugfix: 从模板继承字段级敏感度，确保模板删除后对象仍保留敏感度信息
    let property_labels = inherit_property_labels(&vault, input.template_id.as_deref());
    // §Bugfix: 从模板继承字段定义（名称+类型），确保模板删除后对象仍保留字段名和类型
    let property_fields = inherit_property_fields(&vault, input.template_id.as_deref());
    let mut properties = input.properties.clone();
    inject_property_fields(&mut properties, &property_fields);
    // §Bugfix: 保存模板名称，模板删除后仍可显示
    inject_template_meta(&vault, input.template_id.as_deref(), &mut properties);
    // 校验 dynamic_group 字段
    validate_dynamic_groups(&properties)?;

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
        properties,
        property_labels,
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
    // Preserve old __fields and __templateName before overwriting properties (前端不发送这两项)
    let old_fields = record.properties.get("__fields").cloned();
    let old_tpl_name = record.properties.get("__templateName").cloned();
    record.name = input.name;
    record.properties = input.properties;
    if let Some(sl) = input.sensitivity_level {
        record.sensitivity_level = sl;
    }
    if let Some(icon_name) = input.icon_name {
        record.icon_name = icon_name;
    }
    // §Bugfix: 更新时重新从模板同步字段敏感度
    if record.template_id.is_some() {
        if let Some(labels) = inherit_property_labels(&vault, record.template_id.as_deref()) {
            record.property_labels = Some(labels);
        }
        // §Bugfix: 重新注入 __fields（前端不发送 __fields，需从模板重新继承）
        let fields = inherit_property_fields(&vault, record.template_id.as_deref());
        if !fields.is_null() {
            inject_property_fields(&mut record.properties, &fields);
        } else if let Some(f) = old_fields {
            // 模板已被删除 — 保留已有的 __fields
            inject_property_fields(&mut record.properties, &f);
        }
        // §Bugfix: 更新模板名称（模板已删除时保留旧值）
        inject_template_meta(
            &vault,
            record.template_id.as_deref(),
            &mut record.properties,
        );
        if record.properties.get("__templateName").is_none() {
            if let Some(name) = old_tpl_name {
                if let Some(obj) = record.properties.as_object_mut() {
                    obj.insert("__templateName".to_string(), name);
                }
            }
        }
    }
    // 校验 dynamic_group 字段
    validate_dynamic_groups(&record.properties)?;
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

/// 为已有对象补齐 `property_labels`（适用于模板删除前创建的对象）。
/// 扫描所有活跃对象，若其有 `template_id` 但 `property_labels` 为 None，
/// 则从模板继承字段敏感度。
#[tauri::command]
pub async fn object_backfill_property_labels(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<usize, String> {
    let vault = vault_handle(&state)?;
    let objects = vault.list_objects(&account_id, None, None, None, false, false)?;
    let mut count = 0usize;
    for obj in &objects {
        // 只处理有 template_id 但缺少 property_labels 的对象
        if obj.template_id.is_none() {
            continue;
        }
        let mut record = match vault.load_object(&obj.id)? {
            Some(r) => r,
            None => continue,
        };
        if record.property_labels.is_some() {
            continue;
        }
        if let Some(labels) = inherit_property_labels(&vault, record.template_id.as_deref()) {
            record.property_labels = Some(labels);
            record.updated_at = chrono::Utc::now().to_rfc3339();
            record.version += 1;
            vault.save_object(&record)?;
            count += 1;
        }
    }
    tracing::info!("[migrate] backfilled property_labels for {} objects", count);
    Ok(count)
}

/// 为已有对象补齐 `__fields`（适用于模板删除前创建的对象，或 __fields 功能上线前创建的对象）。
/// 扫描所有活跃对象，若其有 `template_id` 但 `properties` 中缺少 `__fields` 键，
/// 则从模板继承字段定义并注入。
#[tauri::command]
pub async fn object_backfill_property_fields(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<usize, String> {
    let vault = vault_handle(&state)?;
    let objects = vault.list_objects(&account_id, None, None, None, false, false)?;
    let mut count = 0usize;
    for obj in &objects {
        if obj.template_id.is_none() {
            continue;
        }
        let mut record = match vault.load_object(&obj.id)? {
            Some(r) => r,
            None => continue,
        };
        // 已有 __fields 则跳过
        if record.properties.get("__fields").is_some() {
            continue;
        }
        let fields = inherit_property_fields(&vault, record.template_id.as_deref());
        if !fields.is_null() {
            inject_property_fields(&mut record.properties, &fields);
            // §Bugfix: 同时补齐 __templateName
            inject_template_meta(
                &vault,
                record.template_id.as_deref(),
                &mut record.properties,
            );
            record.updated_at = chrono::Utc::now().to_rfc3339();
            record.version += 1;
            vault.save_object(&record)?;
            count += 1;
        }
    }
    tracing::info!("[migrate] backfilled __fields for {} objects", count);
    Ok(count)
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
