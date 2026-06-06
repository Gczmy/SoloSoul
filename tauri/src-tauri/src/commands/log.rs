//! Operation log commands — system activity audit trail

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub module: String,
    pub message: String,
    pub details: Option<String>,
}

/// Simple file-backed operation log store
pub struct OpLogStore {
    entries: RwLock<Vec<LogEntry>>,
    log_path: PathBuf,
}

impl OpLogStore {
    pub fn new(base_path: PathBuf) -> Self {
        let log_dir = base_path.join("logs");
        let log_path = log_dir.join("operations.jsonl");
        let _ = fs::create_dir_all(&log_dir);

        let entries = Self::load_existing(&log_path);
        Self {
            entries: RwLock::new(entries),
            log_path,
        }
    }

    fn load_existing(path: &PathBuf) -> Vec<LogEntry> {
        if !path.exists() {
            return vec![];
        }
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return vec![],
        };
        content
            .lines()
            .filter_map(|line| serde_json::from_str::<LogEntry>(line).ok())
            .collect()
    }

    pub fn append(&self, entry: LogEntry) {
        // Keep in memory
        if let Ok(mut entries) = self.entries.write() {
            entries.push(entry.clone());
            // Keep last 5000 entries
            let excess = entries.len().saturating_sub(5000);
            if excess > 0 {
                entries.drain(0..excess);
            }
        }
        // Persist to JSONL
        if let Ok(line) = serde_json::to_string(&entry) {
            let _ = fs::write(&self.log_path, line + "\n");
        }
    }

    pub fn get_recent(&self, limit: usize) -> Vec<LogEntry> {
        self.entries
            .read()
            .ok()
            .map(|entries| {
                let mut sorted = entries.clone();
                sorted.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                sorted.truncate(limit);
                sorted
            })
            .unwrap_or_default()
    }

    pub fn export(&self) -> Vec<LogEntry> {
        self.entries
            .read()
            .ok()
            .map(|e| e.clone())
            .unwrap_or_default()
    }
}

#[tauri::command]
pub async fn log_get_recent(
    state: tauri::State<'_, crate::state::AppState>,
    limit: Option<usize>,
) -> Result<Vec<LogEntry>, String> {
    let svc = state.vault_service.read().await;
    let base = svc.base_path().clone();
    // Use a simple approach — read from a known log path
    let log_path = base.join("logs").join("operations.jsonl");
    if !log_path.exists() {
        return Ok(vec![]);
    }
    let content = fs::read_to_string(&log_path).unwrap_or_default();
    let entries: Vec<LogEntry> = content
        .lines()
        .filter_map(|line| serde_json::from_str::<LogEntry>(line).ok())
        .collect();

    let limit = limit.unwrap_or(100);
    let mut sorted = entries;
    sorted.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    sorted.truncate(limit);
    Ok(sorted)
}

#[tauri::command]
pub async fn log_export(state: tauri::State<'_, crate::state::AppState>) -> Result<String, String> {
    let svc = state.vault_service.read().await;
    let base = svc.base_path().clone();
    let log_path = base.join("logs").join("operations.jsonl");

    if !log_path.exists() {
        return Ok("[]".to_string());
    }

    let export_path = base.join("logs").join("export_log.json");
    let content = fs::read_to_string(&log_path).unwrap_or_default();
    let entries: Vec<LogEntry> = content
        .lines()
        .filter_map(|line| serde_json::from_str::<LogEntry>(line).ok())
        .collect();

    let json = serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())?;
    fs::write(&export_path, &json).map_err(|e| e.to_string())?;

    Ok(export_path.to_string_lossy().to_string())
}
