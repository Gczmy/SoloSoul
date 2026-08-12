use crate::commands::vault_handle;
use crate::state::AppState;
use serde::Serialize;

use tauri::State;

// ── Snapshot count badge ────────────────────────────────────

use super::*;
#[tauri::command]
pub async fn snapshot_count_batch(
    state: State<'_, AppState>,
    object_ids: Vec<String>,
) -> Result<std::collections::HashMap<String, usize>, String> {
    let vault = vault_handle(&state)?;
    vault.count_snapshots_batch(&object_ids)
}

// ── Snapshot / History commands (§25.5) ─────────────────────

#[tauri::command]
pub async fn snapshot_get_data(
    state: State<'_, AppState>,
    snapshot_id: String,
) -> Result<Option<serde_json::Value>, String> {
    let vault = vault_handle(&state)?;
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
    let vault = vault_handle(&state)?;
    vault.list_snapshots(&object_id)
}

#[tauri::command]
pub async fn snapshot_rollback(
    state: State<'_, AppState>,
    snapshot_id: String,
    object_id: String,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;

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
    if !snapshot["propertyLabels"].is_null() {
        record.property_labels = Some(snapshot["propertyLabels"].clone());
    } else if let Some(labels) = snapshot.get("property_labels") {
        record.property_labels = Some(labels.clone());
    }
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record)?;

    // Save rollback snapshot
    let rollback_data = serde_json::to_vec(&serde_json::json!({
        "name": record.name,
        "tags": record.tags_json,
        "properties": record.properties,
        "propertyLabels": record.property_labels,
    }))
    .unwrap_or_default();
    let _ = vault.save_snapshot(&object_id, "rollback", &rollback_data, "diff_rollback");
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
    state.auto_sync.trigger_debounce();
    Ok(())
}

/// Summary of a child object belonging to a deleted custom page.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashChildSummary {
    pub id: String,
    pub original_id: String,
    pub name: String,
    pub item_type: String,
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
    pub property_labels: Option<serde_json::Value>,
    pub preview_properties: Vec<serde_json::Value>,
    /// Attachments parsed from stored data (active + soft-deleted)
    pub attachments: Vec<TrashAttachmentInfo>,
    pub deleted_attachments: Vec<TrashAttachmentInfo>,
    /// Snapshots from object_snapshots table
    pub snapshots: Vec<serde_json::Value>,
    /// Child objects for page-type trash items (empty for non-page)
    pub child_items: Vec<TrashChildSummary>,
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
    /// 附件描述（随 __attachments 快照携带；旧数据可能缺失）
    pub description: Option<String>,
    /// 附件标签（随 __attachments 快照携带；旧数据可能缺失）
    pub tags: Vec<String>,
}

// ── Trash detail helpers ──────────────────────────────────────

/// 计算回收站剩余保留天数。
fn trash_remaining_days(expires_at: Option<i64>) -> Option<i64> {
    expires_at.map(|exp| {
        let diff_ms = exp - chrono::Utc::now().timestamp_millis();
        std::cmp::max(0, diff_ms / MS_PER_DAY)
    })
}

/// 构造回收站条目的原始位置描述。
fn trash_original_location(trash: &solosoul_vault::TrashItem) -> String {
    match trash.item_type.as_str() {
        "page" => format!("Page: {}", trash.name_snapshot),
        "object" => trash
            .original_section_type
            .as_deref()
            .map(|st| format!("From page: {}", st))
            .unwrap_or_else(|| "From unknown page".to_string()),
        "template" => format!("Template: {}", trash.name_snapshot),
        _ => "Unknown".to_string(),
    }
}

/// 构建 preview_properties：模板条目读取 properties 数组；
/// 对象/页面条目基于 __fields/模板/属性顺序生成（模板删除后仍可显示正确本地化字段名）。
fn build_preview_properties(
    vault: &solosoul_vault::VaultStore,
    trash: &solosoul_vault::TrashItem,
) -> Vec<serde_json::Value> {
    if trash.item_type == "template" {
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
        build_object_preview_properties(vault, trash)
    }
}

/// P036: 对象/页面条目的 preview_properties 构建（阶段化）：
/// 优先使用对象自身保存的 __fields 字段定义获取名称/类型，敏感度以 property_labels
/// 为真实来源；模板删除后仍可显示正确的本地化字段名与敏感度。
fn build_object_preview_properties(
    vault: &solosoul_vault::VaultStore,
    trash: &solosoul_vault::TrashItem,
) -> Vec<serde_json::Value> {
    (|| -> Option<Vec<serde_json::Value>> {
        let data: serde_json::Value = serde_json::from_slice(&trash.data).ok()?;
        let props = data.get("properties")?.as_object()?;
        let fields_def = props.get("__fields").and_then(|v| v.as_object());

        // 字段级敏感度的真实来源是 propertyLabels（对象当前敏感度副本）
        let sensitivity_map = data
            .get("propertyLabels")
            .or_else(|| data.get("property_labels"))
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        // 阶段 1：加载模板（用于排序和补充字段定义）
        let tpl = data
            .get("templateId")
            .or_else(|| data.get("template_id"))
            .and_then(|v| v.as_str())
            .and_then(|tpl_id| vault.load_user_template(tpl_id).ok().flatten());
        let tpl_order: Vec<String> = tpl
            .as_ref()
            .map(|t| t.properties.iter().map(|p| p.id.clone()).collect())
            .unwrap_or_default();

        // 阶段 2：构建字段定义映射（__fields 优先，模板补充）
        let field_defs = collect_field_defs(fields_def, tpl.as_ref());

        // 阶段 3：确定字段顺序：模板顺序 -> __fields 顺序 -> properties 顺序
        let ordered_ids = resolve_field_order(&tpl_order, fields_def, props);

        // 阶段 4：生成结果（敏感度三级 fallback；page 类型不含 sensitiveLevel）
        let is_page = trash.item_type == "page";
        let mut result = Vec::new();
        for field_id in ordered_ids {
            let v = match props.get(&field_id) {
                Some(v) => v,
                None => continue,
            };
            let (name, ptype) = match field_defs.get(&field_id) {
                Some(def) => def.clone(),
                None => (field_id.clone(), "text".to_string()),
            };
            let mut entry = serde_json::json!({
                "fieldId": field_id,
                "key": name,
                "value": v,
                "type": ptype,
            });
            if !is_page {
                let sens =
                    resolve_sensitivity(&field_id, &sensitivity_map, fields_def, tpl.as_ref());
                entry["sensitivityLevel"] = serde_json::Value::String(sens);
            }
            result.push(entry);
        }
        Some(result.into_iter().take(5).collect())
    })()
    .unwrap_or_default()
}

/// P036: 构建字段定义映射：field_id -> (显示名, 类型)。__fields 优先，模板补充缺失项。
fn collect_field_defs(
    fields_def: Option<&serde_json::Map<String, serde_json::Value>>,
    tpl: Option<&solosoul_vault::UserTemplate>,
) -> std::collections::HashMap<String, (String, String)> {
    let mut field_defs: std::collections::HashMap<String, (String, String)> =
        std::collections::HashMap::new();

    if let Some(fields) = fields_def {
        for (field_id, def) in fields {
            let name = def
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(field_id)
                .to_string();
            let ptype = def
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("text")
                .to_string();
            field_defs.insert(field_id.clone(), (name, ptype));
        }
    }

    if let Some(t) = tpl {
        for prop in &t.properties {
            if field_defs.contains_key(&prop.id) {
                continue;
            }
            let ptype = serde_json::to_string(&prop.prop_type)
                .ok()
                .and_then(|s| serde_json::from_str::<String>(&s).ok())
                .unwrap_or_else(|| "text".to_string());
            field_defs.insert(prop.id.clone(), (prop.name.clone(), ptype));
        }
    }
    field_defs
}

/// P036: 确定字段顺序：模板顺序 -> __fields 顺序 -> properties 顺序（跳过 __ 系统键）。
fn resolve_field_order(
    tpl_order: &[String],
    fields_def: Option<&serde_json::Map<String, serde_json::Value>>,
    props: &serde_json::Map<String, serde_json::Value>,
) -> Vec<String> {
    let mut ordered_ids: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for id in tpl_order {
        if seen.insert(id.clone()) {
            ordered_ids.push(id.clone());
        }
    }
    if let Some(fields) = fields_def {
        for id in fields.keys() {
            if seen.insert(id.clone()) {
                ordered_ids.push(id.clone());
            }
        }
    }
    for id in props.keys() {
        if !id.starts_with("__") && seen.insert(id.clone()) {
            ordered_ids.push(id.clone());
        }
    }
    ordered_ids
}

/// P036: 解析字段敏感度：property_labels -> __fields -> 模板 -> internal。
fn resolve_sensitivity(
    field_id: &str,
    sensitivity_map: &serde_json::Map<String, serde_json::Value>,
    fields_def: Option<&serde_json::Map<String, serde_json::Value>>,
    tpl: Option<&solosoul_vault::UserTemplate>,
) -> String {
    sensitivity_map
        .get(field_id)
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| {
            fields_def
                .and_then(|f| f.get(field_id))
                .and_then(|d| d.get("sensitivityLevel"))
                .and_then(|v| v.as_str())
                .map(String::from)
        })
        .or_else(|| {
            tpl.and_then(|t| {
                t.properties
                    .iter()
                    .find(|p| p.id == field_id)
                    .and_then(|p| p.sensitivity_level.clone())
            })
        })
        .unwrap_or_else(|| "internal".to_string())
}

/// 从存储数据解析附件（活跃 + 软删除）。
/// pub(crate)：供 tests/ 子模块直接测试真实解析逻辑。
pub(crate) fn parse_trash_attachments(
    trash: &solosoul_vault::TrashItem,
) -> (Vec<TrashAttachmentInfo>, Vec<TrashAttachmentInfo>) {
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
                // 旧数据可能未保存这两项（AttachmentMeta 序列化时 skip 空值）——
                // 键缺失或类型不符均安全回退为 None / 空数组。
                description: a
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                tags: a
                    .get("tags")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
            };
            if info.deleted_at.is_some() {
                deleted.push(info);
            } else {
                active.push(info);
            }
        }
        Some((active, deleted))
    })();
    parsed.unwrap_or_default()
}

/// 拉取 page 类型回收站条目的子对象（非 page 返回空）。
fn fetch_trash_child_items(
    vault: &solosoul_vault::VaultStore,
    trash: &solosoul_vault::TrashItem,
) -> Vec<TrashChildSummary> {
    if trash.item_type != "page" {
        return Vec::new();
    }
    let all = vault.list_trash_items(None, None).unwrap_or_default();
    let page_id = &trash.original_id;
    let mut children: Vec<TrashChildSummary> = all
        .into_iter()
        .filter(|t| t.item_type == "object" && t.original_section_type.as_deref() == Some(page_id))
        .filter_map(|t| {
            // Look up full TrashItem to get original_id
            let item_id = t.id.clone();
            match vault.get_trash_item(&item_id) {
                Ok(Some(full)) => Some(TrashChildSummary {
                    id: item_id,
                    original_id: full.original_id,
                    name: t.name,
                    item_type: t.item_type,
                }),
                _ => None,
            }
        })
        .collect();
    children.sort_by(|a, b| a.name.cmp(&b.name));
    children
}

/// 从存储数据提取 template_id 与 property_labels。
fn extract_trash_metadata(
    trash: &solosoul_vault::TrashItem,
) -> (Option<String>, Option<serde_json::Value>) {
    (|| -> Option<(String, Option<serde_json::Value>)> {
        let data: serde_json::Value = serde_json::from_slice(&trash.data).ok()?;
        let tpl_id = data
            .get("templateId")
            .or_else(|| data.get("template_id"))
            .and_then(|v| v.as_str())
            .map(String::from);
        let labels = data
            .get("propertyLabels")
            .or_else(|| data.get("property_labels"))
            .cloned();
        Some((tpl_id?, labels))
    })()
    .map(|(id, labels)| (Some(id), labels))
    .unwrap_or((None, None))
}

#[tauri::command]
pub async fn trash_get_detail(
    state: State<'_, AppState>,
    trash_id: String,
) -> Result<TrashDetail, String> {
    let vault = vault_handle(&state)?;
    let trash = vault
        .get_trash_item(&trash_id)?
        .ok_or("Trash item not found")?;

    let remaining_days = trash_remaining_days(trash.expires_at);
    let original_location = trash_original_location(&trash);
    let preview_properties = build_preview_properties(&vault, &trash);
    let (attachments, deleted_attachments) = parse_trash_attachments(&trash);
    let child_items = fetch_trash_child_items(&vault, &trash);
    let snapshots = vault.list_snapshots(&trash.original_id).unwrap_or_default();
    let (template_id, property_labels) = extract_trash_metadata(&trash);

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
        property_labels,
        preview_properties,
        attachments,
        deleted_attachments,
        snapshots,
        child_items,
    })
}

/// Load trash retention period from profile preferences.
pub fn load_trash_retention(vault: &solosoul_vault::VaultStore, account_id: &str) -> String {
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
    DEFAULT_RETENTION.to_string()
}

/// Compute retention ms from period string.
pub fn retention_ms(period: &str) -> i64 {
    match period {
        RETENTION_60D => 60 * MS_PER_DAY,
        RETENTION_HALF_YEAR => 180 * MS_PER_DAY,
        RETENTION_ONE_YEAR => 365 * MS_PER_DAY,
        RETENTION_NEVER => i64::MAX,
        _ => 30 * MS_PER_DAY,
    }
}
