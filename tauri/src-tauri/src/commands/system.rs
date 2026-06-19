/// 获取应用基础信息（名称、版本、操作系统、架构）。
#[tauri::command]
pub async fn get_app_info() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "appName": "SoloSoul",
        "version": env!("CARGO_PKG_VERSION"),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
    }))
}

/// 获取系统 UI 显示语言（例如 "zh-CN"、"en-US"）。
#[tauri::command]
pub fn get_system_locale() -> Result<String, String> {
    let result = get_ui_language();
    tracing::info!("[i18n] get_system_locale command: {:?}", result);
    result.ok_or_else(|| "Failed to detect UI language".to_string())
}

#[cfg(target_os = "windows")]
pub fn get_ui_language() -> Option<String> {
    use windows::Win32::Globalization::GetUserDefaultUILanguage;
    const LANGID_PRIMARY_MASK: u16 = 0x3FF;
    const LANGID_CHINESE: u16 = 0x04;
    let lang_id = unsafe { GetUserDefaultUILanguage() };
    let primary_id = lang_id & LANGID_PRIMARY_MASK;
    if primary_id == LANGID_CHINESE {
        Some("zh-CN".to_string())
    } else {
        Some("en-US".to_string())
    }
}

/// 获取系统外观主题（light / dark）。
/// dark_light::detect() returns Result<Mode, Error> in newer versions.
#[tauri::command]
pub fn get_system_theme() -> Result<String, String> {
    use dark_light::Mode;
    let mode = dark_light::detect()
        .map_err(|e| format!("Failed to detect system theme: {}", e))?;
    match mode {
        Mode::Dark => Ok("dark".to_string()),
        Mode::Light => Ok("light".to_string()),
        _ => Ok("light".to_string()),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn get_ui_language() -> Option<String> {
    sys_locale::get_locale()
}
