use tauri::State;
use crate::core::sensitivity::{SensitivityManager, SensitivityLevel, SensitivityMap, SensitivityLogEntry};

#[tauri::command]
pub async fn sensitivity_get_field(
    manager: State<'_, SensitivityManager>,
    field_id: String,
) -> Result<String, String> {
    let map = manager.map.read().await;
    Ok(map.get(&field_id).as_str().to_string())
}

#[tauri::command]
pub async fn sensitivity_get_map(
    manager: State<'_, SensitivityManager>,
) -> Result<SensitivityMap, String> {
    let map = manager.map.read().await;
    Ok(map.clone())
}

#[tauri::command]
pub async fn sensitivity_update_field(
    manager: State<'_, SensitivityManager>,
    field_id: String,
    new_level: String,
    password: String,
    reason: Option<String>,
) -> Result<(), String> {
    if password.is_empty() {
        return Err("Password required to change sensitivity".to_string());
    }

    let level = SensitivityLevel::parse_level(&new_level)
        .ok_or_else(|| format!("Invalid level: {}", new_level))?;

    let mut map = manager.map.write().await;
    let old = map.get(&field_id);

    // Downgrade protection: downgrading requires password (already provided)
    // Upgrade is always allowed
    if (level as i32) < (old as i32) {
        // Only debug-level password check; real impl would verify against vault
        if password != "debug" {
            return Err("Invalid password".to_string());
        }
    }

    map.set(&field_id, level);
    let mut log = manager.log.write().await;
    log.push(&field_id, old, level, reason.unwrap_or_default());
    Ok(())
}

#[tauri::command]
pub async fn sensitivity_get_log(
    manager: State<'_, SensitivityManager>,
    limit: Option<usize>,
) -> Result<Vec<SensitivityLogEntry>, String> {
    let log = manager.log.read().await;
    let entries: Vec<_> = log.entries.iter().rev().take(limit.unwrap_or(100)).cloned().collect();
    Ok(entries)
}
