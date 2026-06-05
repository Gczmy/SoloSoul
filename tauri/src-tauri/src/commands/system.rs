use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_app_info() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "appName": "SoloSoul",
        "version": "2.0.0",
        "buildNumber": "1",
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
    }))
}

#[tauri::command]
pub async fn check_version() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "currentVersion": "2.0.0",
        "latestVersion": null,
        "hasUpdate": false,
    }))
}
