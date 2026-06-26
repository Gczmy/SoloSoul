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
    // SAFETY: GetUserDefaultUILanguage 是 Windows API kernel32 的线程安全函数，
    // 仅返回当前用户的 UI 语言标识（LANGID），不访问或修改任何 Rust 内存。
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
    let mode = dark_light::detect().map_err(|e| format!("Failed to detect system theme: {}", e))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_app_info_contains_expected_fields() {
        let info = get_app_info().await.unwrap();
        assert_eq!(info["appName"], "SoloSoul");
        assert!(info.get("version").and_then(|v| v.as_str()).is_some());
        assert!(info.get("os").and_then(|v| v.as_str()).is_some());
        assert!(info.get("arch").and_then(|v| v.as_str()).is_some());
    }

    #[tokio::test]
    async fn test_get_app_info_version_is_semver() {
        let info = get_app_info().await.unwrap();
        let version = info["version"].as_str().unwrap();
        assert!(
            version.contains('.'),
            "version should be semver: {}",
            version
        );
    }

    #[tokio::test]
    async fn test_get_app_info_os_is_non_empty() {
        let info = get_app_info().await.unwrap();
        let os = info["os"].as_str().unwrap();
        assert!(!os.is_empty(), "OS should not be empty");
    }

    #[tokio::test]
    async fn test_get_app_info_arch_is_non_empty() {
        let info = get_app_info().await.unwrap();
        let arch = info["arch"].as_str().unwrap();
        assert!(!arch.is_empty(), "Arch should not be empty");
    }

    #[tokio::test]
    async fn test_get_system_locale_returns_locale() {
        let locale = get_system_locale();
        // Sync command — no await needed
        if let Ok(l) = &locale {
            assert!(!l.is_empty());
            assert!(
                l.contains('-') || l.contains('_'),
                "expected locale like en-US or en_US, got: {}",
                l
            );
        }
    }

    #[tokio::test]
    async fn test_get_system_theme_returns_dark_or_light() {
        let theme = get_system_theme();
        match theme {
            Ok(t) => assert!(
                t == "dark" || t == "light",
                "theme must be 'dark' or 'light', got: {}",
                t
            ),
            Err(ref e) => {
                assert!(e.contains("Failed to detect system theme"));
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    #[tokio::test]
    async fn test_get_ui_language_non_windows() {
        let lang = get_ui_language();
        assert!(lang.is_some());
        let l = lang.unwrap();
        assert!(!l.is_empty());
    }
}
