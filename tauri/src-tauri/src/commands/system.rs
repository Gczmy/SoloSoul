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

/// Get the OS UI display language (e.g. "zh-CN", "en-US").
///
/// On Windows, returns the user's *display language* (what menus/dialogs show),
/// not the regional format — those can differ. Falls back to sys-locale on other platforms.
#[tauri::command]
pub fn get_system_locale() -> Result<String, String> {
    let result = get_ui_language();
    tracing::info!("[i18n] get_system_locale command: {:?}", result);
    result.ok_or_else(|| "Failed to detect UI language".to_string())
}

#[cfg(target_os = "windows")]
pub fn get_ui_language() -> Option<String> {
    use windows::Win32::Globalization::GetUserDefaultUILanguage;
    // GetUserDefaultUILanguage returns the UI display language LANGID
    // Low 10 bits = primary language ID. 0x04 = Chinese (all variants)
    let lang_id = unsafe { GetUserDefaultUILanguage() };
    let primary_id = lang_id & 0x3FF;
    if primary_id == 0x04 {
        Some("zh-CN".to_string())
    } else {
        Some("en-US".to_string())
    }
}

#[cfg(not(target_os = "windows"))]
pub fn get_ui_language() -> Option<String> {
    // macOS/Linux: sys_locale is reliable here
    sys_locale::get_locale()
}
