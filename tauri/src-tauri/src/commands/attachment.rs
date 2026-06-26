//! File attachment commands — attach files to objects, with soft-delete support (§25.6)

use crate::commands::vault_handle;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::State;

/// 单个对象最多允许的活跃附件数量。
const MAX_ACTIVE_ATTACHMENTS: usize = 200;

/// 附件 ID 与对象 ID 允许使用的字符集，防止路径遍历。
fn validate_attachment_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.len() > 64
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!("Invalid attachment id: {}", id));
    }
    Ok(())
}

fn attachment_dir(base: &Path, object_id: &str, attachment_id: &str) -> Result<PathBuf, String> {
    validate_attachment_id(object_id)?;
    validate_attachment_id(attachment_id)?;
    Ok(base.join("attachments").join(object_id).join(attachment_id))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentMeta {
    pub id: String,
    pub object_id: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<String>,
    /// Original source path (transient, only set at upload time)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub src_path: Option<String>,
    /// Vault storage path (persistent, survives original file deletion)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault_path: Option<String>,
}

fn load_attachments(props: &Value) -> Vec<AttachmentMeta> {
    props
        .get("__attachments")
        .and_then(|v| serde_json::from_value::<Vec<AttachmentMeta>>(v.clone()).ok())
        .unwrap_or_default()
}

fn save_attachments(props: &mut Value, atts: &[AttachmentMeta]) {
    if let Value::Object(ref mut obj) = props {
        obj.insert(
            "__attachments".to_string(),
            serde_json::to_value(atts).unwrap_or_default(),
        );
    }
}

#[tauri::command]
pub async fn attachment_list(
    state: State<'_, AppState>,
    object_id: String,
    show_deleted: Option<bool>,
) -> Result<Vec<AttachmentMeta>, String> {
    let show = show_deleted.unwrap_or(false);
    let vault = vault_handle(&state)?;
    match vault.load_object(&object_id) {
        Ok(Some(rec)) => Ok(load_attachments(&rec.properties)
            .into_iter()
            .filter(|a| {
                if show {
                    a.deleted_at.is_some()
                } else {
                    a.deleted_at.is_none()
                }
            })
            .collect()),
        _ => Ok(vec![]),
    }
}

/// Physical delete (permanent — removes metadata + deletes file from disk)
#[tauri::command]
pub async fn attachment_delete(
    state: State<'_, AppState>,
    object_id: String,
    attachment_id: String,
) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let vault = svc
        .get_vault_store()
        .ok_or_else(|| "Vault not unlocked".to_string())?;
    let mut record = vault.load_object(&object_id)?.ok_or("Object not found")?;
    let atts: Vec<AttachmentMeta> = load_attachments(&record.properties)
        .into_iter()
        .filter(|a: &AttachmentMeta| a.id != attachment_id)
        .collect();

    // Also delete the physical file from disk
    let attachments_dir = attachment_dir(svc.base_path(), &object_id, &attachment_id)?;
    std::fs::remove_dir_all(&attachments_dir)
        .map_err(|e| format!("Failed to delete attachment file: {}", e))?;

    save_attachments(&mut record.properties, &atts);
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record)
}

#[tauri::command]
pub async fn attachment_restore(
    state: State<'_, AppState>,
    object_id: String,
    attachment_id: String,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let mut record = vault.load_object(&object_id)?.ok_or("Object not found")?;
    let mut atts = load_attachments(&record.properties);
    if let Some(a) = atts.iter_mut().find(|a| a.id == attachment_id) {
        a.deleted_at = None;
    }
    save_attachments(&mut record.properties, &atts);
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record)
}

#[tauri::command]
pub async fn attachment_save(
    state: State<'_, AppState>,
    object_id: String,
    meta: AttachmentMeta,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let mut record = vault.load_object(&object_id)?.ok_or("Object not found")?;
    let mut atts = load_attachments(&record.properties);
    // §10.4.4: maximum active attachments per object
    let active_count = atts.iter().filter(|a| a.deleted_at.is_none()).count();
    if active_count >= MAX_ACTIVE_ATTACHMENTS {
        return Err(format!(
            "Maximum {} active attachments per object reached",
            MAX_ACTIVE_ATTACHMENTS
        ));
    }
    atts.push(meta);
    save_attachments(&mut record.properties, &atts);
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record)
}

#[tauri::command]
pub async fn attachment_rename(
    state: State<'_, AppState>,
    object_id: String,
    attachment_id: String,
    new_name: String,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let mut record = vault.load_object(&object_id)?.ok_or("Object not found")?;
    let mut atts = load_attachments(&record.properties);
    if let Some(a) = atts.iter_mut().find(|a| a.id == attachment_id) {
        a.file_name = new_name;
    }
    save_attachments(&mut record.properties, &atts);
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record)
}

#[tauri::command]
pub async fn attachment_soft_delete(
    state: State<'_, AppState>,
    object_id: String,
    attachment_id: String,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let mut record = vault.load_object(&object_id)?.ok_or("Object not found")?;
    let mut atts = load_attachments(&record.properties);
    if let Some(a) = atts.iter_mut().find(|a| a.id == attachment_id) {
        a.deleted_at = Some(chrono::Utc::now().to_rfc3339());
    }

    save_attachments(&mut record.properties, &atts);
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record)
}

/// Batch soft-delete multiple attachments on the same object in a single transaction.
/// Significantly faster than N sequential `attachment_soft_delete` calls.
#[tauri::command]
pub async fn attachment_batch_soft_delete(
    state: State<'_, AppState>,
    object_id: String,
    attachment_ids: Vec<String>,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let mut record = vault.load_object(&object_id)?.ok_or("Object not found")?;
    let mut atts = load_attachments(&record.properties);
    let ids_set: std::collections::HashSet<&str> =
        attachment_ids.iter().map(|s| s.as_str()).collect();
    let now = chrono::Utc::now().to_rfc3339();
    for att in atts.iter_mut().filter(|a| ids_set.contains(a.id.as_str())) {
        att.deleted_at = Some(now.clone());
    }
    save_attachments(&mut record.properties, &atts);
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record)
}

/// Batch restore multiple soft-deleted attachments on the same object in a single transaction.
/// Significantly faster than N sequential `attachment_restore` calls.
#[tauri::command]
pub async fn attachment_batch_restore(
    state: State<'_, AppState>,
    object_id: String,
    attachment_ids: Vec<String>,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let mut record = vault.load_object(&object_id)?.ok_or("Object not found")?;
    let mut atts = load_attachments(&record.properties);
    let ids_set: std::collections::HashSet<&str> =
        attachment_ids.iter().map(|s| s.as_str()).collect();
    for att in atts.iter_mut().filter(|a| ids_set.contains(a.id.as_str())) {
        att.deleted_at = None;
    }
    save_attachments(&mut record.properties, &atts);
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record)
}

/// Batch permanent-delete multiple attachments on the same object in a single transaction.
/// Removes metadata entries AND deletes physical files from disk.
#[tauri::command]
pub async fn attachment_batch_delete(
    state: State<'_, AppState>,
    object_id: String,
    attachment_ids: Vec<String>,
) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let vault = svc
        .get_vault_store()
        .ok_or_else(|| "Vault not unlocked".to_string())?;
    let mut record = vault.load_object(&object_id)?.ok_or("Object not found")?;
    let ids_set: std::collections::HashSet<&str> =
        attachment_ids.iter().map(|s| s.as_str()).collect();

    // Collect physical paths before removing metadata
    let paths_to_remove: Vec<std::path::PathBuf> = load_attachments(&record.properties)
        .iter()
        .filter(|a| ids_set.contains(a.id.as_str()))
        .filter_map(|a| attachment_dir(svc.base_path(), &object_id, &a.id).ok())
        .collect();

    let atts: Vec<AttachmentMeta> = load_attachments(&record.properties)
        .into_iter()
        .filter(|a| !ids_set.contains(a.id.as_str()))
        .collect();

    // Delete physical files
    for dir in &paths_to_remove {
        let _ = std::fs::remove_dir_all(dir);
    }

    save_attachments(&mut record.properties, &atts);
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record)
}

#[tauri::command]
pub async fn attachment_count_batch(
    state: State<'_, AppState>,
    object_ids: Vec<String>,
) -> Result<HashMap<String, usize>, String> {
    let vault = vault_handle(&state)?;
    let mut result = HashMap::new();
    for id in &object_ids {
        if let Ok(Some(rec)) = vault.load_object(id) {
            let count = load_attachments(&rec.properties)
                .iter()
                .filter(|a| a.deleted_at.is_none())
                .count();
            result.insert(id.clone(), count);
        }
    }
    Ok(result)
}

/// Copy a file into vault-managed attachment storage.
/// Returns the vault path that should be stored as `vault_path` on the attachment meta.
///
/// # Security
/// - `src_path` is canonicalized to resolve relative path traversal (`../`).
/// - Source path must NOT be inside vault storage itself (prevents self-referencing).
/// - `file_name` is sanitized to only the final path component.
#[tauri::command]
pub async fn attachment_copy_to_vault(
    state: State<'_, AppState>,
    src_path: String,
    object_id: String,
    attachment_id: String,
    file_name: String,
) -> Result<String, String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let base = svc.base_path().clone();

    // Canonicalize src_path to resolve relative path traversal
    let src = std::path::Path::new(&src_path)
        .canonicalize()
        .map_err(|e| format!("Invalid source path: {}", e))?;

    // Reject if source path is within vault storage (self-referencing)
    // Canonicalize the vault base to match src.canonicalize() symlink resolution.
    let vault_base = base
        .canonicalize()
        .map_err(|_| "Invalid vault base path".to_string())?;
    if src.starts_with(&vault_base) {
        return Err("Source path must not be inside vault storage".to_string());
    }

    let dest_dir = attachment_dir(&base, &object_id, &attachment_id)?;
    std::fs::create_dir_all(&dest_dir).map_err(|e| format!("Mkdir: {}", e))?;

    // R007: sanitize file_name to prevent path traversal; only the final component is allowed.
    let safe_name = std::path::Path::new(&file_name)
        .file_name()
        .ok_or("Invalid file name")?
        .to_string_lossy()
        .to_string();
    let dest_path = dest_dir.join(&safe_name);
    std::fs::copy(&src, &dest_path).map_err(|e| format!("Copy: {}", e))?;
    Ok(dest_path.to_string_lossy().to_string())
}

/// Collect all attachment IDs that are currently referenced in any object's __attachments.
fn load_all_referenced_attachment_ids(
    vault: &solosoul_vault::VaultStore,
    account_id: &str,
) -> Result<std::collections::HashSet<String>, String> {
    let objects = vault.list_objects(account_id, None, None, None, false, false)?;
    let mut active_ids = std::collections::HashSet::new();
    for summary in &objects {
        if let Ok(Some(rec)) = vault.load_object(&summary.id) {
            let atts: Vec<AttachmentMeta> = rec
                .properties
                .get("__attachments")
                .and_then(|v| serde_json::from_value::<Vec<AttachmentMeta>>(v.clone()).ok())
                .unwrap_or_default();
            // Include ALL attachments (both active and soft-deleted) — the soft-deleted ones
            // still have a reference; only truly orphaned files (no __attachments entry at all)
            // should be cleaned up. Permanently deleting removes the entry, so those are orphans.
            for a in &atts {
                active_ids.insert(a.id.clone());
            }
        }
    }
    Ok(active_ids)
}

// ── Types for global attachment tree ────────────────────────────

/// One object in the attachment tree, containing its attachments.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentTreeObject {
    pub object_id: String,
    pub object_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<AttachmentMeta>,
}

/// One page (section type or custom page) in the attachment tree.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentTreePage {
    #[serde(default)]
    pub page_id: Option<String>,
    pub page_name: String,
    #[serde(default)]
    pub page_icon: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objects: Vec<AttachmentTreeObject>,
}

/// Result of listing all attachments across all objects, grouped by page.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentListAllResult {
    /// Pages with active (non-deleted) attachments.
    pub pages: Vec<AttachmentTreePage>,
    /// Pages with deleted attachments (for trash view).
    pub trash_pages: Vec<AttachmentTreePage>,
}

/// List all attachments across all objects, grouped by page.
/// Custom pages use parent_id to find child objects;
/// remaining objects are grouped by section_type (built-in sections).
#[tauri::command]
pub async fn attachment_list_all(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<AttachmentListAllResult, String> {
    let vault = vault_handle(&state)?;
    let objects = vault.list_objects(&account_id, None, None, None, false, false)?;

    // Separate page objects from other objects
    let mut page_objects: Vec<solosoul_vault::ObjectSummary> = Vec::new();
    let mut section_groups: std::collections::BTreeMap<String, Vec<solosoul_vault::ObjectSummary>> =
        std::collections::BTreeMap::new();

    for obj in &objects {
        if obj.collection_type == "page" {
            page_objects.push(obj.clone());
        } else {
            section_groups
                .entry(obj.section_type.clone())
                .or_default()
                .push(obj.clone());
        }
    }

    let pages = build_attachment_tree_pages(
        &vault, &account_id, &page_objects, &section_groups, false,
    )?;
    let trash_pages = build_attachment_tree_pages(
        &vault, &account_id, &page_objects, &section_groups, true,
    )?;

    Ok(AttachmentListAllResult { pages, trash_pages })
}

/// Build attachment tree pages for a given filter (active vs trash).
fn build_attachment_tree_pages(
    vault: &solosoul_vault::VaultStore,
    account_id: &str,
    page_objects: &[solosoul_vault::ObjectSummary],
    section_groups: &std::collections::BTreeMap<String, Vec<solosoul_vault::ObjectSummary>>,
    only_deleted: bool,
) -> Result<Vec<AttachmentTreePage>, String> {
    let template_cache: std::cell::RefCell<std::collections::HashMap<String, Option<String>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
    let build_objects_with_attachments =
        |objs: &[solosoul_vault::ObjectSummary], only_del: bool| -> Vec<AttachmentTreeObject> {
            objs.iter()
                .filter_map(|summary| {
                    let record = vault.load_object(&summary.id).ok()??;
                    let all_atts = load_attachments(&record.properties);
                    let filtered: Vec<AttachmentMeta> = all_atts
                        .into_iter()
                        .filter(|a| {
                            if only_del {
                                a.deleted_at.is_some()
                            } else {
                                a.deleted_at.is_none()
                            }
                        })
                        .collect();
                    if filtered.is_empty() {
                        None
                    } else {
                        let template_name = record.template_id.as_ref().and_then(|tid| {
                            let mut cache = template_cache.borrow_mut();
                            cache.get(tid).cloned().unwrap_or_else(|| {
                                let name = vault
                                    .load_user_template(tid)
                                    .ok()
                                    .flatten()
                                    .map(|t| t.name.clone());
                                cache.insert(tid.clone(), name.clone());
                                name
                            })
                        });
                        Some(AttachmentTreeObject {
                            object_id: summary.id.clone(),
                            object_name: summary.name.clone(),
                            template_name,
                            attachments: filtered,
                        })
                    }
                })
                .collect()
        };

    let mut pages: Vec<AttachmentTreePage> = Vec::new();
    let mut child_ids_assigned: std::collections::HashSet<String> =
        std::collections::HashSet::new();

    // For custom pages: find children via parent_id
    for page_obj in page_objects {
        let children = vault
            .list_objects(account_id, None, Some(&page_obj.id), None, false, false)
            .unwrap_or_default();
        for child in &children {
            child_ids_assigned.insert(child.id.clone());
        }
        let objects_with_attachments = build_objects_with_attachments(&children, only_deleted);
        if !objects_with_attachments.is_empty() {
            pages.push(AttachmentTreePage {
                page_id: Some(page_obj.id.clone()),
                page_name: page_obj.name.clone(),
                page_icon: Some(page_obj.icon_name.clone()),
                objects: objects_with_attachments,
            });
        }
    }

    // For remaining objects: group by section_type (built-in sections)
    for (section, objs) in section_groups {
        let unassigned: Vec<_> = objs
            .iter()
            .filter(|o| !child_ids_assigned.contains(&o.id))
            .cloned()
            .collect();
        if unassigned.is_empty() {
            continue;
        }
        let objects_with_attachments = build_objects_with_attachments(&unassigned, only_deleted);
        if !objects_with_attachments.is_empty() {
            pages.push(AttachmentTreePage {
                page_id: None,
                page_name: section.clone(),
                page_icon: Some(section.clone()),
                objects: objects_with_attachments,
            });
        }
    }

    Ok(pages)
}

/// Download an attachment file to a user-chosen destination path.
/// Copies the file from vault storage to the specified destination.
#[tauri::command]
pub async fn attachment_download(
    state: State<'_, AppState>,
    src_path: String,
    dest_path: String,
) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let vault_base = svc
        .base_path()
        .canonicalize()
        .map_err(|_| "Invalid vault base path".to_string())?;

    // Security: ensure the source path is within vault storage
    let src = std::path::Path::new(&src_path)
        .canonicalize()
        .map_err(|e| format!("Invalid source path: {}", e))?;
    if !src.starts_with(&vault_base) {
        return Err("Source path must be within vault storage".to_string());
    }

    // Create parent directory and copy the file
    let dest = std::path::Path::new(&dest_path);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create destination directory: {}", e))?;
    }
    std::fs::copy(&src, dest).map_err(|e| format!("Failed to copy file: {}", e))?;

    Ok(())
}

/// Open an attachment with the system's default application.
/// The path is resolved from the attachment metadata and verified to be inside
/// the vault's `attachments` directory before opening.
#[tauri::command]
pub async fn attachment_open(
    state: State<'_, AppState>,
    object_id: String,
    attachment_id: String,
) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let vault = svc
        .get_vault_store()
        .ok_or_else(|| "Vault not unlocked".to_string())?;

    let record = vault.load_object(&object_id)?.ok_or("Object not found")?;
    let att = load_attachments(&record.properties)
        .into_iter()
        .find(|a| a.id == attachment_id)
        .ok_or("Attachment not found")?;

    let path_str = att
        .vault_path
        .as_ref()
        .or(att.src_path.as_ref())
        .ok_or("Attachment has no file path")?;

    let vault_base = svc
        .base_path()
        .canonicalize()
        .map_err(|_| "Invalid vault base path".to_string())?;
    let attachments_dir = vault_base.join("attachments");

    let path = std::path::Path::new(path_str)
        .canonicalize()
        .map_err(|e| format!("Cannot access attachment file: {}", e))?;
    if !path.starts_with(&attachments_dir) {
        return Err("Attachment path is outside vault storage".to_string());
    }

    opener::open(&path).map_err(|e| format!("Failed to open file: {}", e))?;
    Ok(())
}

/// Scan attachments directory and remove files not referenced in any object's metadata.
#[tauri::command]
pub async fn attachment_cleanup_orphans(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<usize, String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();

    let active_ids = load_all_referenced_attachment_ids(vault, &account_id)?;
    let base_dir = svc.base_path().join("attachments");

    if !base_dir.exists() {
        return Ok(0);
    }

    let mut removed = 0usize;
    let mut total_freed = 0u64;
    if let Ok(object_entries) = std::fs::read_dir(&base_dir) {
        for obj_entry in object_entries.flatten() {
            let obj_path = obj_entry.path();
            if !obj_path.is_dir() {
                continue;
            }
            if let Ok(att_entries) = std::fs::read_dir(&obj_path) {
                for att_entry in att_entries.flatten() {
                    let att_path = att_entry.path();
                    let att_id = att_entry.file_name().to_string_lossy().to_string();
                    if !active_ids.contains(&att_id) {
                        // Orphaned — delete it
                        if let Ok(meta) = att_path.metadata() {
                            total_freed += meta.len();
                        }
                        let _ = std::fs::remove_dir_all(&att_path);
                        removed += 1;
                    }
                }
            }
            // Remove empty object directories too
            if std::fs::read_dir(&obj_path)
                .map(|mut d| d.next().is_none())
                .unwrap_or(false)
            {
                let _ = std::fs::remove_dir(&obj_path);
            }
        }
    }

    let _ = vault.log_structured(
        "attachment_cleanup",
        "attachment",
        None,
        None,
        "system",
        Some(&format!(
            "removed {} orphaned attachments, freed {} bytes",
            removed, total_freed
        )),
    );
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solosoul_vault::{ObjectRecord, VaultConfig, VaultStore};
    use tempfile::TempDir;

    fn setup_vault() -> (VaultStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let config =
            VaultConfig::new("test_account", dir.path().to_path_buf()).with_data_key([0x42u8; 32]);
        let vault = VaultStore::open(config).unwrap();
        (vault, dir)
    }

    #[test]
    fn test_attachment_meta_serde_roundtrip() {
        let original = AttachmentMeta {
            id: "att-1".to_string(),
            object_id: "obj-1".to_string(),
            file_name: "test.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            size_bytes: 1024,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            deleted_at: Some("2024-02-01T00:00:00Z".to_string()),
            src_path: Some("/tmp/test.pdf".to_string()),
            vault_path: Some("/vault/test.pdf".to_string()),
        };
        let json = serde_json::to_string(&original).unwrap();
        let restored: AttachmentMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, original.id);
        assert_eq!(restored.object_id, original.object_id);
        assert_eq!(restored.file_name, original.file_name);
        assert_eq!(restored.mime_type, original.mime_type);
        assert_eq!(restored.size_bytes, original.size_bytes);
        assert_eq!(restored.created_at, original.created_at);
        assert_eq!(restored.deleted_at, original.deleted_at);
        assert_eq!(restored.src_path, original.src_path);
        assert_eq!(restored.vault_path, original.vault_path);
    }

    #[test]
    fn test_load_attachments_empty() {
        let props = serde_json::json!({"title": "hello"});
        let atts = load_attachments(&props);
        assert!(atts.is_empty());
    }

    #[test]
    fn test_load_attachments_some() {
        let atts = vec![AttachmentMeta {
            id: "att-1".to_string(),
            object_id: "obj-1".to_string(),
            file_name: "a.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            size_bytes: 100,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            deleted_at: None,
            src_path: None,
            vault_path: None,
        }];
        let props = serde_json::json!({"title": "hello", "__attachments": atts});
        let loaded = load_attachments(&props);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "att-1");
    }

    #[test]
    fn test_save_and_load_attachments() {
        let mut props = serde_json::json!({"title": "hello"});
        let atts = vec![AttachmentMeta {
            id: "att-1".to_string(),
            object_id: "obj-1".to_string(),
            file_name: "a.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            size_bytes: 100,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            deleted_at: None,
            src_path: None,
            vault_path: None,
        }];
        save_attachments(&mut props, &atts);
        let loaded = load_attachments(&props);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "att-1");
        assert_eq!(loaded[0].file_name, "a.pdf");
    }

    #[test]
    fn test_load_all_referenced_attachment_ids() {
        let (vault, _dir) = setup_vault();
        let account_id = "acc-1";

        let record1 = ObjectRecord {
            contract_type_id: None,
            id: "obj-1".to_string(),
            account_id: account_id.to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Note 1".to_string(),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({
                "__attachments": [
                    AttachmentMeta {
                        id: "att-1".to_string(),
                        object_id: "obj-1".to_string(),
                        file_name: "a.pdf".to_string(),
                        mime_type: "application/pdf".to_string(),
                        size_bytes: 100,
                        created_at: "2024-01-01T00:00:00Z".to_string(),
                        deleted_at: None,
                        src_path: None,
                        vault_path: None,
                    },
                    AttachmentMeta {
                        id: "att-2".to_string(),
                        object_id: "obj-1".to_string(),
                        file_name: "b.pdf".to_string(),
                        mime_type: "application/pdf".to_string(),
                        size_bytes: 200,
                        created_at: "2024-01-01T00:00:00Z".to_string(),
                        deleted_at: Some("2024-02-01T00:00:00Z".to_string()),
                        src_path: None,
                        vault_path: None,
                    },
                ]
            }),
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
        vault.save_object(&record1).unwrap();

        let record2 = ObjectRecord {
            contract_type_id: None,
            id: "obj-2".to_string(),
            account_id: account_id.to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Note 2".to_string(),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({
                "__attachments": [
                    AttachmentMeta {
                        id: "att-3".to_string(),
                        object_id: "obj-2".to_string(),
                        file_name: "c.pdf".to_string(),
                        mime_type: "application/pdf".to_string(),
                        size_bytes: 300,
                        created_at: "2024-01-01T00:00:00Z".to_string(),
                        deleted_at: None,
                        src_path: None,
                        vault_path: None,
                    },
                ]
            }),
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
        vault.save_object(&record2).unwrap();

        let ids = load_all_referenced_attachment_ids(&vault, account_id).unwrap();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains("att-1"));
        assert!(ids.contains("att-2"));
        assert!(ids.contains("att-3"));
    }

    #[test]
    fn test_vault_attachment_filtering() {
        let (vault, _dir) = setup_vault();
        let mut record = ObjectRecord {
            contract_type_id: None,
            id: "obj-1".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Note".to_string(),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({
                "__attachments": [
                    AttachmentMeta {
                        id: "att-1".to_string(),
                        object_id: "obj-1".to_string(),
                        file_name: "active.pdf".to_string(),
                        mime_type: "application/pdf".to_string(),
                        size_bytes: 100,
                        created_at: "2024-01-01T00:00:00Z".to_string(),
                        deleted_at: None,
                        src_path: None,
                        vault_path: None,
                    },
                    AttachmentMeta {
                        id: "att-2".to_string(),
                        object_id: "obj-1".to_string(),
                        file_name: "deleted.pdf".to_string(),
                        mime_type: "application/pdf".to_string(),
                        size_bytes: 200,
                        created_at: "2024-01-01T00:00:00Z".to_string(),
                        deleted_at: Some("2024-02-01T00:00:00Z".to_string()),
                        src_path: None,
                        vault_path: None,
                    },
                ]
            }),
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

        let rec = vault.load_object("obj-1").unwrap().unwrap();
        let atts = load_attachments(&rec.properties);
        let active: Vec<_> = atts.iter().filter(|a| a.deleted_at.is_none()).collect();
        let deleted: Vec<_> = atts.iter().filter(|a| a.deleted_at.is_some()).collect();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "att-1");
        assert_eq!(deleted.len(), 1);
        assert_eq!(deleted[0].id, "att-2");

        // Test soft-delete helper logic inline
        let mut atts_mut = load_attachments(&rec.properties);
        if let Some(a) = atts_mut.iter_mut().find(|a| a.id == "att-1") {
            a.deleted_at = Some("2024-03-01T00:00:00Z".to_string());
        }
        save_attachments(&mut record.properties, &atts_mut);
        vault.save_object(&record).unwrap();

        let rec2 = vault.load_object("obj-1").unwrap().unwrap();
        let atts2 = load_attachments(&rec2.properties);
        assert_eq!(atts2.iter().filter(|a| a.deleted_at.is_none()).count(), 0);
        assert_eq!(atts2.iter().filter(|a| a.deleted_at.is_some()).count(), 2);
    }
}
