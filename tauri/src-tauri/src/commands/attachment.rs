//! File attachment commands — attach files to objects, with soft-delete support (§25.6)

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::State;

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
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
    match vault.load_object(&object_id) {
        Ok(Some(rec)) => Ok(load_attachments(&rec.properties)
            .into_iter()
            .filter(|a| if show { a.deleted_at.is_some() } else { a.deleted_at.is_none() })
            .collect()),
        _ => Ok(vec![]),
    }
}

/// Physical delete (permanent — removes from object properties entirely)
#[tauri::command]
pub async fn attachment_delete(
    state: State<'_, AppState>,
    object_id: String,
    attachment_id: String,
) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
    let mut record = vault.load_object(&object_id)?.ok_or("Object not found")?;
    let atts: Vec<AttachmentMeta> = load_attachments(&record.properties)
        .into_iter()
        .filter(|a: &AttachmentMeta| a.id != attachment_id)
        .collect();
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
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
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
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
    let mut record = vault.load_object(&object_id)?.ok_or("Object not found")?;
    let mut atts = load_attachments(&record.properties);
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
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
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
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
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

#[tauri::command]
pub async fn attachment_count_batch(
    state: State<'_, AppState>,
    object_ids: Vec<String>,
) -> Result<HashMap<String, usize>, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
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
#[tauri::command]
pub async fn attachment_copy_to_vault(
    state: State<'_, AppState>,
    src_path: String,
    object_id: String,
    attachment_id: String,
    file_name: String,
) -> Result<String, String> {
    let svc = state.vault_service.read().await;
    let base = svc.base_path().clone();
    let dest_dir = base.join("attachments").join(&object_id).join(&attachment_id);
    std::fs::create_dir_all(&dest_dir).map_err(|e| format!("Mkdir: {}", e))?;
    let dest_path = dest_dir.join(&file_name);
    std::fs::copy(&src_path, &dest_path).map_err(|e| format!("Copy: {}", e))?;
    Ok(dest_path.to_string_lossy().to_string())
}
