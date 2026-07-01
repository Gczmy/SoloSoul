use crate::commands::{current_account, vault_handle};
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
pub async fn snapshot_get(
    state: State<'_, AppState>,
    object_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    let vault = vault_handle(&state)?;
    vault.list_snapshots(&object_id)
}

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
    let vault = vault_handle(&state)?;
    let account_id = current_account(&state)?;
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
    Ok(DEFAULT_RETENTION.to_string())
}

/// Set trash retention period.
#[tauri::command]
pub async fn trash_set_retention(state: State<'_, AppState>, period: String) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let account_id = current_account(&state)?;
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

    let remaining_days = trash.expires_at.map(|exp| {
        let diff_ms = exp - chrono::Utc::now().timestamp_millis();
        std::cmp::max(0, diff_ms / MS_PER_DAY)
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

    // Fetch child items for page-type trash
    let child_items: Vec<TrashChildSummary> = if trash.item_type == "page" {
        let all = vault.list_trash_items(None, None).unwrap_or_default();
        let page_id = &trash.original_id;
        let mut children: Vec<TrashChildSummary> = all
            .into_iter()
            .filter(|t| {
                t.item_type == "object" && t.original_section_type.as_deref() == Some(page_id)
            })
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
    } else {
        Vec::new()
    };

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
