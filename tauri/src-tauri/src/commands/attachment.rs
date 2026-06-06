//! File attachment commands — attach encrypted files to objects

use crate::state::AppState;
use serde::{Deserialize, Serialize};
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
}

#[tauri::command]
pub async fn attachment_list(
    state: State<'_, AppState>,
    object_id: String,
) -> Result<Vec<AttachmentMeta>, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    // Load the object and return its attachments list
    match vault.load_object(&object_id) {
        Ok(Some(rec)) => {
            let props = rec.properties;
            let atts = props
                .get("__attachments")
                .and_then(|v| serde_json::from_value::<Vec<AttachmentMeta>>(v.clone()).ok())
                .unwrap_or_default();
            Ok(atts)
        }
        _ => Ok(vec![]),
    }
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

    let mut atts: Vec<AttachmentMeta> = record
        .properties
        .get("__attachments")
        .and_then(|v| serde_json::from_value::<Vec<AttachmentMeta>>(v.clone()).ok())
        .unwrap_or_default();

    atts.push(meta);
    if let serde_json::Value::Object(ref mut obj) = record.properties {
        obj.insert(
            "__attachments".to_string(),
            serde_json::to_value(&atts).map_err(|e| e.to_string())?,
        );
    }

    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record)
}

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

    let atts: Vec<AttachmentMeta> = record
        .properties
        .get("__attachments")
        .and_then(|v| serde_json::from_value::<Vec<AttachmentMeta>>(v.clone()).ok())
        .unwrap_or_default()
        .into_iter()
        .filter(|a: &AttachmentMeta| a.id != attachment_id)
        .collect();

    if let serde_json::Value::Object(ref mut obj) = record.properties {
        obj.insert(
            "__attachments".to_string(),
            serde_json::to_value(&atts).map_err(|e| e.to_string())?,
        );
    }

    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record)
}
