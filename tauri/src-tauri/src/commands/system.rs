/// 应用基础信息的 IPC 响应；字段名保持既有 wire 格式。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub app_name: String,
    pub version: String,
    pub os: String,
    pub arch: String,
}

/// 获取应用基础信息（名称、版本、操作系统、架构）。
#[tauri::command]
pub async fn get_app_info() -> Result<AppInfo, String> {
    Ok(AppInfo {
        app_name: "SoloSoul".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
    })
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
/// 桌面端使用 dark_light 检测；移动端由 WebView media query 接管，不返回伪检测结果。
#[tauri::command]
pub fn get_system_theme() -> Result<String, String> {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        Err("SYSTEM_THEME_WEBVIEW_REQUIRED".to_string())
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        use dark_light::Mode;
        let mode =
            dark_light::detect().map_err(|e| format!("Failed to detect system theme: {}", e))?;
        match mode {
            Mode::Dark => Ok("dark".to_string()),
            Mode::Light => Ok("light".to_string()),
            _ => Ok("light".to_string()),
        }
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
        let info = serde_json::to_value(get_app_info().await.unwrap()).unwrap();
        assert_eq!(info.as_object().unwrap().len(), 4);
        assert_eq!(info["appName"], "SoloSoul");
        assert!(info.get("version").and_then(|v| v.as_str()).is_some());
        assert!(info.get("os").and_then(|v| v.as_str()).is_some());
        assert!(info.get("arch").and_then(|v| v.as_str()).is_some());
    }

    #[tokio::test]
    async fn test_get_app_info_version_is_semver() {
        let info = get_app_info().await.unwrap();
        let version = &info.version;
        assert!(
            version.contains('.'),
            "version should be semver: {}",
            version
        );
    }

    #[tokio::test]
    async fn test_get_app_info_os_is_non_empty() {
        let info = get_app_info().await.unwrap();
        let os = &info.os;
        assert!(!os.is_empty(), "OS should not be empty");
    }

    #[tokio::test]
    async fn test_get_app_info_arch_is_non_empty() {
        let info = get_app_info().await.unwrap();
        let arch = &info.arch;
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
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
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

    #[test]
    #[cfg(any(target_os = "android", target_os = "ios"))]
    fn rf201_mobile_system_theme_requires_webview() {
        assert_eq!(
            get_system_theme().unwrap_err(),
            "SYSTEM_THEME_WEBVIEW_REQUIRED"
        );
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
