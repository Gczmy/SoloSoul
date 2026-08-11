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

use crate::commands::vault_handle;
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

/// P019: `log_write` 允许的 action_type 枚举白名单——前端仅能写关键字段查看审计，
/// 禁止伪造登录/导出/备份等系统级动作的审计条目（审计日志作为安全证据的可信度）。
const FRONTEND_LOG_ACTION_TYPES: &[&str] = &[
    "critical_field_login",
    "critical_field_pin",
    "critical_field_touch_id",
    "critical_field_windows_hello",
    "critical_field_face_id",
];

/// Write a structured audit log entry to the vault's audit_log table.
#[tauri::command]
pub async fn log_write(state: State<'_, AppState>, request: WriteLogRequest) -> Result<(), String> {
    if !FRONTEND_LOG_ACTION_TYPES.contains(&request.action_type.as_str()) {
        return Err(format!(
            "action_type '{}' is not allowed from frontend",
            request.action_type
        ));
    }
    let vault = vault_handle(&state)?;

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
    let vault = vault_handle(&state)?;

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
    // P023: 块作用域取出所有权数据，守卫在 await 前释放（RwLockReadGuard 非 Send，
    // 不能跨 await 存活）。
    let (vault, logs_dir) = {
        let svc = state
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        let vault = svc.get_vault_store().ok_or("Vault not unlocked")?;
        // R011: restrict export to the vault's logs directory; user-supplied paths are
        // reduced to a single file name to prevent writing to arbitrary locations.
        let logs_dir = svc.base_path().join("logs");
        (vault, logs_dir)
    };
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

    // P023: 逐行解密万行审计日志 + JSON 序列化 + 写盘为同步重 IO，移入 spawn_blocking。
    let path_out = tokio::task::spawn_blocking(move || {
        std::fs::create_dir_all(&logs_dir).map_err(|e| e.to_string())?;
        let entries = vault.list_audit_log(10000)?;
        let json = serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())?;
        std::fs::write(&path, &json).map_err(|e| e.to_string())?;
        Ok::<_, String>(path.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| format!("log_export task failed: {e}"))??;

    Ok(path_out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_log_request_camelcase_serde() {
        let req = WriteLogRequest {
            action_type: "export".to_string(),
            entity_type: "vault".to_string(),
            entity_id: Some("acc-1".to_string()),
            entity_name: None,
            details: Some("exported 10 objects".to_string()),
        };
        let json = serde_json::to_string(&req).unwrap();
        // Verify camelCase field names (has rename_all = "camelCase")
        assert!(json.contains("actionType"));
        assert!(json.contains("entityType"));
        assert!(json.contains("entityId"));
        assert!(!json.contains("action_type"), "should use camelCase");

        // Roundtrip
        let restored: WriteLogRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.action_type, "export");
        assert_eq!(restored.details.as_deref(), Some("exported 10 objects"));
        assert!(restored.entity_name.is_none());
    }

    #[test]
    fn test_audit_log_response_camelcase_serde() {
        let resp = AuditLogResponse {
            id: 42,
            timestamp: "2024-06-01T12:00:00Z".to_string(),
            action_type: "login".to_string(),
            entity_type: "auth".to_string(),
            entity_id: Some("acc-1".to_string()),
            entity_name: Some("Alice".to_string()),
            performed_by: "user".to_string(),
            details: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"actionType\""));
        assert!(json.contains("\"performedBy\""));
        assert!(json.contains("\"entityName\""));

        let restored: AuditLogResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, 42);
        assert_eq!(restored.action_type, "login");
        assert!(restored.details.is_none());
    }

    #[test]
    fn test_to_response_conversion() {
        let entry = solosoul_vault::AuditLogEntry {
            id: 1,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            action_type: "create".to_string(),
            entity_type: "object".to_string(),
            entity_id: Some("obj-1".to_string()),
            entity_name: Some("My Object".to_string()),
            performed_by: "user".to_string(),
            details: Some("name=test".to_string()),
        };
        let resp = to_response(entry);
        assert_eq!(resp.id, 1);
        assert_eq!(resp.action_type, "create");
        assert_eq!(resp.entity_id.as_deref(), Some("obj-1"));
        assert_eq!(resp.details.as_deref(), Some("name=test"));
    }

    #[test]
    fn test_to_response_handles_none_fields() {
        let entry = solosoul_vault::AuditLogEntry {
            id: 2,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            action_type: "login".to_string(),
            entity_type: "auth".to_string(),
            entity_id: None,
            entity_name: None,
            performed_by: "system".to_string(),
            details: None,
        };
        let resp = to_response(entry);
        assert!(resp.entity_id.is_none());
        assert!(resp.entity_name.is_none());
        assert!(resp.details.is_none());
        assert_eq!(resp.performed_by, "system");
    }

    #[test]
    fn test_get_recent_respects_limit() {
        let (vault, _dir) = setup_vault();
        // Write 3 log entries
        for i in 0..3 {
            vault
                .log_structured(&format!("action{}", i), "test", None, None, "user", None)
                .unwrap();
        }
        // list_audit_log with limit 2
        let entries = vault.list_audit_log(2).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_get_recent_respects_default_limit() {
        let (vault, _dir) = setup_vault();
        // list_audit_log with default (100) on empty vault returns 0
        let entries = vault.list_audit_log(100).unwrap();
        assert_eq!(entries.len(), 0);
    }

    fn setup_vault() -> (solosoul_vault::VaultStore, tempfile::TempDir) {
        let dir = tempfile::TempDir::new().unwrap();
        let config = solosoul_vault::VaultConfig::new("test", dir.path().to_path_buf())
            .with_data_key([0x42u8; 32]);
        let vault = solosoul_vault::VaultStore::open(config).unwrap();
        (vault, dir)
    }
}
