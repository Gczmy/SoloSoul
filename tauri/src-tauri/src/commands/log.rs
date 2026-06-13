//! Operation log commands — system activity audit trail
//!
//! Uses VaultStore's audit_log table (AES-256-GCM encrypted at rest via
//! SQLite Vault). This replaces the old flat-file operations.jsonl approach.
//!
//! # Architecture
//!
//! - **AuditLogEntry** — structured record with action_type, entity_type,
//!   entity_id, entity_name, performed_by, and details (JSON metadata).
//! - **log_write** — IPC command for frontend code to record arbitrary
//!   audit events (import/export, preference changes, biometric ops).
//! - **log_get_recent** — IPC command to load recent entries for display.
//! - **log_export** — IPC command to export all entries as JSON.
//! - Backend CRUD operations log directly via VaultStore::log_structured.
//!
//! All entries are stored in Vault's SQLite database, which is encrypted
//! with AES-256-GCM via app-layer encryption. Data is only accessible
//! when the vault is unlocked.

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

/// Request payload for writing a new audit log entry from the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteLogRequest {
    pub action_type: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub entity_name: Option<String>,
    pub details: Option<String>,
}

/// Frontend-facing audit log entry (matches vault's AuditLogEntry).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogResponse {
    pub id: i64,
    pub timestamp: String,
    pub action_type: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub entity_name: Option<String>,
    pub performed_by: String,
    pub details: Option<String>,
}

fn to_response(entry: solosoul_vault::AuditLogEntry) -> AuditLogResponse {
    AuditLogResponse {
        id: entry.id,
        timestamp: entry.timestamp,
        action_type: entry.action_type,
        entity_type: entry.entity_type,
        entity_id: entry.entity_id,
        entity_name: entry.entity_name,
        performed_by: entry.performed_by,
        details: entry.details,
    }
}

/// Write a structured audit log entry to the vault's audit_log table.
#[tauri::command]
pub async fn log_write(state: State<'_, AppState>, request: WriteLogRequest) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();

    vault.log_structured(
        &request.action_type,
        &request.entity_type,
        request.entity_id.as_deref(),
        request.entity_name.as_deref(),
        "user",
        request.details.as_deref(),
    )
}

/// Get recent audit log entries, newest first.
#[tauri::command]
pub async fn log_get_recent(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<AuditLogResponse>, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();

    let entries = vault.list_audit_log(limit.unwrap_or(100))?;
    Ok(entries.into_iter().map(to_response).collect())
}

/// Export all audit log entries as a JSON file at the given path.
/// If no path is provided, writes to ~/.solosoul/logs/export_audit_log.json
/// Returns the path to the exported file.
#[tauri::command]
pub async fn log_export(
    state: State<'_, AppState>,
    export_path: Option<String>,
) -> Result<String, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();

    let entries = vault.list_audit_log(10000)?;
    let json = serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())?;

    // R011: restrict export to the vault's logs directory; user-supplied paths are
    // reduced to a single file name to prevent writing to arbitrary locations.
    let logs_dir = svc.base_path().join("logs");
    let path = if let Some(p) = export_path {
        let file_name = std::path::Path::new(&p)
            .file_name()
            .ok_or("Invalid export path")?
            .to_string_lossy()
            .to_string();
        logs_dir.join(file_name)
    } else {
        logs_dir.join("export_audit_log.json")
    };
    std::fs::create_dir_all(&logs_dir).map_err(|e| e.to_string())?;
    std::fs::write(&path, &json).map_err(|e| e.to_string())?;

    Ok(path.to_string_lossy().to_string())
}
