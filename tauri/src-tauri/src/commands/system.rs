use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_app_info() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "appName": "SoloSoul",
        "version": "2.0.0",
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

/// Get the system locale (e.g. "zh-CN", "en-US").
/// Uses the OS locale API directly — more reliable than navigator.language in WebView2.
#[tauri::command]
pub fn get_system_locale() -> Result<String, String> {
    sys_locale::get_locale().ok_or_else(|| "Failed to detect system locale".to_string())
}
