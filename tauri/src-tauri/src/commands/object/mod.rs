//! Object CRUD commands — P0-1: Real object storage layer
//!
//! Uses the `objects` table in solosoul_vault (separate from profiles).
//! Supports: type schemas, parent/child hierarchies, property storage,
//! soft-delete trash, and account-scoped queries.

use crate::commands::{current_account_optional, vault_handle};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateSyncStatus {
    pub needs_sync: bool,
    pub current_hash: Option<String>,
    pub latest_hash: Option<String>,
    pub template_exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncFieldInfo {
    pub id: String,
    pub name: String,
    pub field_type: String,
}

/// 模板同步中某一项字段变更的描述，结构化后交给前端做本地化显示。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "kind", content = "payload")]
pub enum SyncFieldChangeItem {
    Type {
        #[serde(rename = "oldType")]
        old_type: String,
        #[serde(rename = "newType")]
        new_type: String,
    },
    Name {
        #[serde(rename = "oldName")]
        old_name: String,
        #[serde(rename = "newName")]
        new_name: String,
    },
    Sensitivity {
        #[serde(rename = "oldLevel")]
        old_level: String,
        #[serde(rename = "newLevel")]
        new_level: String,
    },
    Options,
    Metadata {
        #[serde(rename = "metadataKeys")]
        metadata_keys: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncFieldChange {
    pub id: String,
    pub name: String,
    pub field_type: String,
    pub changes: Vec<SyncFieldChangeItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncFieldIncompatible {
    pub id: String,
    pub name: String,
    pub old_type: String,
    pub new_type: String,
    pub old_value_preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateSyncResult {
    pub has_changes: bool,
    pub template_hash: String,
    pub fields_added: Vec<SyncFieldInfo>,
    pub fields_deprecated: Vec<SyncFieldInfo>,
    pub fields_updated: Vec<SyncFieldChange>,
    pub fields_incompatible: Vec<SyncFieldIncompatible>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeprecatedField {
    pub id: String,
    pub name: String,
    pub field_type: String,
    pub value: serde_json::Value,
    pub deprecated_at: String,
    pub reason: String,
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

/// 计算模板指纹，用于判断对象是否需要同步模板更新。
/// 排除 id/account_id/created_at/updated_at，按字段 id 稳定排序后序列化再取 SHA-256 前 16 位。
pub fn template_fingerprint(tpl: &solosoul_vault::UserTemplate) -> String {
    let mut props: Vec<&solosoul_vault::TemplateProperty> = tpl.properties.iter().collect();
    props.sort_by(|a, b| a.id.cmp(&b.id));
    let canonical = serde_json::json!({
        "properties": props,
    });
    let bytes = serde_json::to_vec(&canonical).unwrap_or_default();
    let hash = Sha256::digest(&bytes);
    hex::encode(&hash[..8])
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
    // 计算并保存模板指纹，用于后续检测模板是否更新
    let template_hash = input
        .template_id
        .as_deref()
        .and_then(|tid| vault.load_user_template(tid).ok().flatten())
        .map(|tpl| template_fingerprint(&tpl));
    if let Some(ref hash) = template_hash {
        if let Some(obj) = properties.as_object_mut() {
            obj.insert(
                "__templateHash".to_string(),
                serde_json::Value::String(hash.clone()),
            );
        }
    }
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
        template_hash,
        ignored_template_hash: None,
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
        "name": record.name,
        "tags": record.tags_json,
        "properties": record.properties,
        "propertyLabels": record.property_labels,
    }))
    .unwrap_or_default();
    let _ = vault.save_snapshot(&id, "user_edit", &snapshot_data, "diff_created");
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
    // 普通更新不再自动同步模板字段定义与敏感度；仅在用户主动同步时更新。
    // 这里只恢复前端未发送的 __fields 与 __templateName，确保对象结构完整。
    if record.properties.get("__fields").is_none() {
        if let Some(f) = old_fields {
            inject_property_fields(&mut record.properties, &f);
        }
    }
    if record.properties.get("__templateName").is_none() {
        if let Some(name) = old_tpl_name {
            if let Some(obj) = record.properties.as_object_mut() {
                obj.insert("__templateName".to_string(), name);
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
        "propertyLabels": record.property_labels,
    }))
    .unwrap_or_default();
    let _ = vault.save_snapshot(&object_id, "user_edit", &snapshot_data, "diff_updated");

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

/// 获取对象当前保存的模板指纹，优先从根字段 template_hash 读取，否则回退到 properties.__templateHash。
fn get_object_template_hash(record: &ObjectRecord) -> Option<String> {
    record.template_hash.clone().or_else(|| {
        record
            .properties
            .get("__templateHash")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
    })
}

/// 将旧字段定义与值移入 properties.__deprecatedFields。
fn deprecate_field(
    properties: &mut serde_json::Map<String, serde_json::Value>,
    field_id: &str,
    field_def: &serde_json::Map<String, serde_json::Value>,
    old_value: serde_json::Value,
    reason: &str,
) {
    let deprecated_map = match properties
        .get_mut("__deprecatedFields")
        .and_then(|v| v.as_object_mut())
    {
        Some(m) => m,
        None => {
            properties.insert(
                "__deprecatedFields".to_string(),
                serde_json::Value::Object(serde_json::Map::new()),
            );
            properties
                .get_mut("__deprecatedFields")
                .and_then(|v| v.as_object_mut())
                .expect("__deprecatedFields should exist")
        }
    };
    let mut entry = field_def.clone();
    entry.insert("value".to_string(), old_value);
    entry.insert(
        "deprecatedAt".to_string(),
        serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
    );
    entry.insert(
        "reason".to_string(),
        serde_json::Value::String(reason.to_string()),
    );
    deprecated_map.insert(field_id.to_string(), serde_json::Value::Object(entry));
}

/// 尝试将旧值转换为新类型；若无法转换则返回 None。
fn convert_value_for_type(
    old_type: &str,
    new_type: &str,
    value: serde_json::Value,
) -> Option<serde_json::Value> {
    match (old_type, new_type) {
        (a, b) if a == b => Some(value),
        ("text", "multiline") => Some(value),
        ("text", "url") | ("text", "email") | ("text", "phone") => value
            .as_str()
            .map(|s| serde_json::Value::String(s.to_string())),
        ("number", "text") | ("boolean", "text") => {
            Some(serde_json::Value::String(value.to_string()))
        }
        ("text", "number") => value.as_str().and_then(|s| s.parse::<f64>().ok()).map(|n| {
            if n.fract() == 0.0 {
                serde_json::Value::Number(serde_json::Number::from(n as i64))
            } else {
                serde_json::json!(n)
            }
        }),
        ("text", "date") => value.as_str().and_then(|s| {
            if s.len() >= 10 {
                Some(serde_json::Value::String(s[..10].to_string()))
            } else {
                None
            }
        }),
        ("text", "datetime") => value.as_str().and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| serde_json::Value::String(dt.to_rfc3339()))
                .or_else(|| {
                    if s.len() >= 10 {
                        Some(serde_json::Value::String(format!(
                            "{}T00:00:00+00:00",
                            &s[..10]
                        )))
                    } else {
                        None
                    }
                })
        }),
        ("date", "datetime") => value.as_str().map(|s| {
            let base = if s.len() >= 10 { &s[..10] } else { s };
            serde_json::Value::String(format!("{}T00:00:00+00:00", base))
        }),
        ("datetime", "date") => value
            .as_str()
            .map(|s| serde_json::Value::String(s.chars().take(10).collect())),
        ("select", "multiselect") => Some(serde_json::Value::Array(vec![value])),
        _ => None,
    }
}

/// 返回某字段类型的空默认值。
fn default_value_for_type(field_type: &str) -> serde_json::Value {
    match field_type {
        "number" => serde_json::Value::Number(0.into()),
        "boolean" => serde_json::Value::Bool(false),
        "multiselect" | "dynamic_group" => serde_json::Value::Array(vec![]),
        _ => serde_json::Value::String(String::new()),
    }
}

/// 将模板字段定义转为 properties.__fields 中的单个字段定义对象。
fn template_prop_to_field_def(prop: &solosoul_vault::TemplateProperty) -> serde_json::Value {
    let mut def = serde_json::Map::new();
    def.insert(
        "name".to_string(),
        serde_json::Value::String(prop.name.clone()),
    );
    def.insert(
        "type".to_string(),
        serde_json::Value::String(prop.prop_type.as_str().to_string()),
    );
    if let Some(ref opts) = prop.options {
        def.insert(
            "options".to_string(),
            serde_json::Value::Array(
                opts.iter()
                    .map(|o| serde_json::Value::String(o.clone()))
                    .collect(),
            ),
        );
    }
    if let Some(ref sl) = prop.sensitivity_level {
        def.insert(
            "sensitivityLevel".to_string(),
            serde_json::Value::String(sl.clone()),
        );
    }
    if let Some(ref cf) = prop.contract_field {
        def.insert("contractField".to_string(), serde_json::Value::Bool(*cf));
    }
    if let Some(ref da) = prop.deprecated_at {
        def.insert(
            "deprecatedAt".to_string(),
            serde_json::Value::String(da.clone()),
        );
    }
    if let solosoul_vault::PropertyType::DynamicGroup = prop.prop_type {
        if let Some(ref allowed) = prop.allowed_types {
            def.insert(
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
            def.insert(
                "maxItems".to_string(),
                serde_json::Value::Number(max.into()),
            );
        }
    }
    serde_json::Value::Object(def)
}

/// 计算对象与模板之间的同步差异。
fn compute_sync_changes(
    record: &ObjectRecord,
    tpl: &solosoul_vault::UserTemplate,
) -> TemplateSyncResult {
    let latest_hash = template_fingerprint(tpl);
    let current_fields = record
        .properties
        .get("__fields")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let props_obj = record.properties.as_object().cloned().unwrap_or_default();
    // 对象的字段级敏感度真实来源是 property_labels；__fields 中的敏感度仅为创建/上次同步时的快照，
    // 直接用它作基准会导致已更新过的敏感度被误报。
    let labels_map = record
        .property_labels
        .as_ref()
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let mut fields_added = Vec::new();
    let mut fields_deprecated = Vec::new();
    let mut fields_updated = Vec::new();
    let mut fields_incompatible = Vec::new();

    // 模板字段集合
    let tpl_field_ids: std::collections::HashSet<String> =
        tpl.properties.iter().map(|p| p.id.clone()).collect();

    // 新增字段：模板中有，对象中没有
    for prop in &tpl.properties {
        if !current_fields.contains_key(&prop.id) {
            fields_added.push(SyncFieldInfo {
                id: prop.id.clone(),
                name: prop.name.clone(),
                field_type: prop.prop_type.as_str().to_string(),
            });
        }
    }

    // 废弃字段：对象中有，模板中没有
    for (field_id, def) in &current_fields {
        if field_id.starts_with("__") {
            continue;
        }
        if !tpl_field_ids.contains(field_id) {
            let name = def
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(field_id)
                .to_string();
            let field_type = def
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("text")
                .to_string();
            fields_deprecated.push(SyncFieldInfo {
                id: field_id.clone(),
                name,
                field_type,
            });
        }
    }

    // 更新与不兼容字段
    for prop in &tpl.properties {
        let Some(old_def) = current_fields.get(&prop.id) else {
            continue;
        };
        let old_type = old_def
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("text");
        let new_type = prop.prop_type.as_str();
        let old_name = old_def.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let old_value = props_obj
            .get(&prop.id)
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        // 类型改变
        if old_type != new_type {
            if convert_value_for_type(old_type, new_type, old_value.clone()).is_some() {
                // 可安全转换：视为普通更新
                fields_updated.push(SyncFieldChange {
                    id: prop.id.clone(),
                    name: prop.name.clone(),
                    field_type: new_type.to_string(),
                    changes: vec![SyncFieldChangeItem::Type {
                        old_type: old_type.to_string(),
                        new_type: new_type.to_string(),
                    }],
                });
            } else {
                let preview = match &old_value {
                    serde_json::Value::String(s) => s.chars().take(40).collect(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    _ => "(complex value)".to_string(),
                };
                fields_incompatible.push(SyncFieldIncompatible {
                    id: prop.id.clone(),
                    name: prop.name.clone(),
                    old_type: old_type.to_string(),
                    new_type: new_type.to_string(),
                    old_value_preview: preview,
                });
            }
            continue;
        }

        // 同类型下的元数据变化
        let mut changes: Vec<SyncFieldChangeItem> = Vec::new();
        if old_name != prop.name {
            changes.push(SyncFieldChangeItem::Name {
                old_name: old_name.to_string(),
                new_name: prop.name.clone(),
            });
        }
        let old_sl = labels_map
            .get(&prop.id)
            .and_then(|v| v.as_str())
            .or_else(|| old_def.get("sensitivityLevel").and_then(|v| v.as_str()))
            .unwrap_or("internal");
        let new_sl = prop.sensitivity_level.as_deref().unwrap_or("internal");
        if old_sl != new_sl {
            changes.push(SyncFieldChangeItem::Sensitivity {
                old_level: old_sl.to_string(),
                new_level: new_sl.to_string(),
            });
        }
        let old_opts = old_def.get("options").and_then(|v| v.as_array());
        let new_opts = prop.options.as_ref().map(|opts| {
            opts.iter()
                .map(|o| serde_json::Value::String(o.clone()))
                .collect::<Vec<_>>()
        });
        if old_opts != new_opts.as_ref() {
            changes.push(SyncFieldChangeItem::Options);
        }

        // 其他元数据变化：动态字段组 allowedTypes/maxItems、字段 deprecatedAt/contractField
        let mut metadata_keys: Vec<String> = Vec::new();
        if new_type == "dynamic_group" {
            let old_allowed = old_def
                .get("allowedTypes")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>());
            let new_allowed = prop
                .allowed_types
                .as_ref()
                .map(|types| types.iter().map(|t| t.as_str()).collect::<Vec<_>>());
            if old_allowed != new_allowed {
                metadata_keys.push("allowedTypes".to_string());
            }
            let old_max = old_def.get("maxItems").and_then(|v| v.as_u64());
            let new_max = prop.max_items.map(|m| m as u64);
            if old_max != new_max {
                metadata_keys.push("maxItems".to_string());
            }
        }
        let old_deprecated = old_def
            .get("deprecatedAt")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let new_deprecated = prop.deprecated_at.as_deref().unwrap_or("");
        if old_deprecated != new_deprecated {
            metadata_keys.push("deprecatedAt".to_string());
        }
        let old_contract = old_def
            .get("contractField")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let new_contract = prop.contract_field.unwrap_or(false);
        if old_contract != new_contract {
            metadata_keys.push("contractField".to_string());
        }
        if !metadata_keys.is_empty() {
            changes.push(SyncFieldChangeItem::Metadata { metadata_keys });
        }

        if !changes.is_empty() {
            fields_updated.push(SyncFieldChange {
                id: prop.id.clone(),
                name: prop.name.clone(),
                field_type: new_type.to_string(),
                changes,
            });
        }
    }

    let has_changes = !fields_added.is_empty()
        || !fields_deprecated.is_empty()
        || !fields_updated.is_empty()
        || !fields_incompatible.is_empty();

    TemplateSyncResult {
        has_changes,
        template_hash: latest_hash,
        fields_added,
        fields_deprecated,
        fields_updated,
        fields_incompatible,
    }
}

/// 将同步结果应用到对象 properties 上。dry_run=true 时不修改。
fn apply_sync_changes(
    record: &mut ObjectRecord,
    tpl: &solosoul_vault::UserTemplate,
    result: &TemplateSyncResult,
    dry_run: bool,
) {
    if dry_run {
        return;
    }

    // 无论是否有字段差异，都刷新 template_hash，防止对象已是最新但仍因旧 hash 重复提示同步。
    record.template_hash = Some(result.template_hash.clone());
    if let Some(obj) = record.properties.as_object_mut() {
        obj.insert(
            "__templateHash".to_string(),
            serde_json::Value::String(result.template_hash.clone()),
        );
    }

    if !result.has_changes {
        return;
    }

    let mut props_obj = record.properties.as_object().cloned().unwrap_or_default();
    let current_fields = record
        .properties
        .get("__fields")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let tpl_map: std::collections::HashMap<String, &solosoul_vault::TemplateProperty> =
        tpl.properties.iter().map(|p| (p.id.clone(), p)).collect();

    // 1. 处理模板中已删除的字段：移入 __deprecatedFields
    for dep in &result.fields_deprecated {
        if let Some(old_def) = current_fields.get(&dep.id) {
            let old_value = props_obj
                .get(&dep.id)
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            deprecate_field(
                &mut props_obj,
                &dep.id,
                old_def.as_object().unwrap_or(&serde_json::Map::new()),
                old_value,
                "removed_by_template",
            );
            props_obj.remove(&dep.id);
        }
    }

    // 2. 处理新增与更新字段
    for (field_id, prop) in &tpl_map {
        let old_def = current_fields.get(field_id);
        let old_type = old_def
            .and_then(|d| d.get("type"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let new_type = prop.prop_type.as_str();
        let new_def = template_prop_to_field_def(prop);

        if let Some(old) = old_def {
            // 字段已存在：检查类型是否变化
            if old_type != new_type {
                let old_value = props_obj
                    .get(field_id)
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                if let Some(converted) =
                    convert_value_for_type(old_type, new_type, old_value.clone())
                {
                    // 安全转换：保留转换后的值
                    props_obj.insert(field_id.clone(), converted);
                } else {
                    // 不安全：归档旧字段并设置新默认值
                    deprecate_field(
                        &mut props_obj,
                        field_id,
                        old.as_object().unwrap_or(&serde_json::Map::new()),
                        old_value,
                        "type_incompatible",
                    );
                    props_obj.insert(field_id.clone(), default_value_for_type(new_type));
                }
            } else {
                // 同类型：保留原值，仅更新定义
                if !props_obj.contains_key(field_id) {
                    props_obj.insert(field_id.clone(), default_value_for_type(new_type));
                }
            }
        } else {
            // 新增字段：仅当对象中尚无该字段值时才写入默认值，避免旧对象缺少 __fields 时已有数据被覆盖
            if !props_obj.contains_key(field_id) {
                props_obj.insert(field_id.clone(), default_value_for_type(new_type));
            }
        }

        // 更新 __fields 定义（直接在 props_obj 上修改，避免后续写回时被覆盖；缺失时创建）
        if props_obj.get("__fields").is_none() {
            props_obj.insert(
                "__fields".to_string(),
                serde_json::Value::Object(serde_json::Map::new()),
            );
        }
        if let Some(fields) = props_obj
            .get_mut("__fields")
            .and_then(|v| v.as_object_mut())
        {
            fields.insert(field_id.clone(), new_def);
        }
    }

    // 3. 重新生成 property_labels
    rebuild_property_labels(record, tpl);

    // 4. 将更新后的 props_obj 写回 record.properties
    if let Some(obj) = record.properties.as_object_mut() {
        for (k, v) in props_obj {
            obj.insert(k, v);
        }
    }

    // 5. 更新 template_hash
    record.template_hash = Some(result.template_hash.clone());
    if let Some(obj) = record.properties.as_object_mut() {
        obj.insert(
            "__templateHash".to_string(),
            serde_json::Value::String(result.template_hash.clone()),
        );
    }
}

/// 根据模板重新生成对象的 property_labels。
fn rebuild_property_labels(record: &mut ObjectRecord, tpl: &solosoul_vault::UserTemplate) {
    let mut labels_map = serde_json::Map::new();
    for prop in &tpl.properties {
        if let Some(ref sl) = prop.sensitivity_level {
            labels_map.insert(prop.id.clone(), serde_json::Value::String(sl.clone()));
        }
    }
    record.property_labels = if labels_map.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(labels_map))
    };
}

#[tauri::command]
pub async fn object_get_template_sync_status(
    state: State<'_, AppState>,
    object_id: String,
) -> Result<TemplateSyncStatus, String> {
    let vault = vault_handle(&state)?;
    let record = vault
        .load_object(&object_id)?
        .ok_or("Object not found".to_string())?;

    let template_id = match record.template_id.as_deref() {
        Some(tid) => tid,
        None => {
            return Ok(TemplateSyncStatus {
                needs_sync: false,
                current_hash: None,
                latest_hash: None,
                template_exists: false,
            });
        }
    };

    let current_hash = get_object_template_hash(&record);
    let tpl = vault.load_user_template(template_id).ok().flatten();

    match tpl {
        Some(tpl) => {
            let latest_hash = template_fingerprint(&tpl);
            let needs_sync = current_hash.as_ref() != Some(&latest_hash);
            Ok(TemplateSyncStatus {
                needs_sync,
                current_hash,
                latest_hash: Some(latest_hash),
                template_exists: true,
            })
        }
        None => Ok(TemplateSyncStatus {
            needs_sync: false,
            current_hash,
            latest_hash: None,
            template_exists: false,
        }),
    }
}

#[tauri::command]
pub async fn object_sync_with_template(
    state: State<'_, AppState>,
    object_id: String,
    dry_run: bool,
) -> Result<TemplateSyncResult, String> {
    let vault = vault_handle(&state)?;
    let mut record = vault
        .load_object(&object_id)?
        .ok_or("Object not found".to_string())?;

    let template_id = record
        .template_id
        .as_deref()
        .ok_or("Object has no associated template".to_string())?
        .to_string();
    let tpl = vault
        .load_user_template(&template_id)
        .ok()
        .flatten()
        .ok_or("Template not found".to_string())?;

    let result = compute_sync_changes(&record, &tpl);

    if dry_run {
        return Ok(result);
    }

    apply_sync_changes(&mut record, &tpl, &result, false);

    // 校验 dynamic_group 字段
    validate_dynamic_groups(&record.properties)?;

    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    // 应用同步后清除已忽略指纹，避免旧忽略状态干扰未来检测。
    record.ignored_template_hash = None;
    vault.save_object(&record)?;

    // 只有真正发生字段变更时才记录快照与审计日志
    if result.has_changes {
        // §25.5 — Save snapshot for history
        let snapshot_data = serde_json::to_vec(&serde_json::json!({
            "name": record.name,
            "tags": record.tags_json,
            "properties": record.properties,
            "propertyLabels": record.property_labels,
        }))
        .unwrap_or_default();
        let _ = vault.save_snapshot(
            &object_id,
            "template_sync",
            &snapshot_data,
            "diff_template_sync",
        );

        let _ = vault.log_structured(
            "object_sync_template",
            "object",
            Some(&object_id),
            Some(&record.name),
            "user",
            Some(&format!(
                "templateName={} templateId={}",
                tpl.name, template_id
            )),
        );
    }

    Ok(result)
}

/// 忽略当前模板指纹：用户点击「否」后，将 latestHash 持久化到对象，
/// 避免重启后再次提示，直到模板再次变更。
#[tauri::command]
pub async fn object_ignore_template_sync(
    state: State<'_, AppState>,
    object_id: String,
    hash: String,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let mut record = vault
        .load_object(&object_id)?
        .ok_or("Object not found".to_string())?;
    record.ignored_template_hash = Some(hash);
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record)?;
    Ok(())
}

#[tauri::command]
pub async fn object_list_deprecated_fields(
    state: State<'_, AppState>,
    object_id: String,
) -> Result<Vec<DeprecatedField>, String> {
    let vault = vault_handle(&state)?;
    let record = vault
        .load_object(&object_id)?
        .ok_or("Object not found".to_string())?;

    let deprecated = record
        .properties
        .get("__deprecatedFields")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let mut result = Vec::new();
    for (field_id, entry) in deprecated {
        let obj = match entry.as_object() {
            Some(o) => o,
            None => continue,
        };
        result.push(DeprecatedField {
            id: field_id,
            name: obj
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            field_type: obj
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("text")
                .to_string(),
            value: obj.get("value").cloned().unwrap_or(serde_json::Value::Null),
            deprecated_at: obj
                .get("deprecatedAt")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            reason: obj
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        });
    }

    // 按废弃时间倒序排列
    result.sort_by(|a, b| b.deprecated_at.cmp(&a.deprecated_at));
    Ok(result)
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
            "id": rec.id, "accountId": rec.account_id, "typeId": rec.type_id,
            "sectionType": rec.section_type, "name": rec.name, "iconName": rec.icon_name,
            "parentId": rec.parent_id, "childrenIds": rec.children_ids,
            "properties": rec.properties, "propertyLabels": rec.property_labels,
            "sensitivityLevel": rec.sensitivity_level, "tags": rec.tags_json,
            "createdAt": rec.created_at, "updatedAt": rec.updated_at, "version": rec.version,
            "templateId": rec.template_id, "templateType": rec.template_type,
            "contractTypeId": rec.contract_type_id,
            "templateHash": rec.template_hash,
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
