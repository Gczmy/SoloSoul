/// 编译支持与运行可用性分开表示；未知/失败不可默认为支持。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityStatus {
    Supported,
    Unsupported,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateMethod {
    None,
    Tauri,
    AndroidApk,
}

/// implementation 标明桥接种类，不承诺已授权、已配置或当前视觉效果。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCapability {
    pub status: CapabilityStatus,
    pub reason: Option<String>,
    pub implementation: Option<String>,
}

impl PlatformCapability {
    fn unsupported(reason: &str) -> Self {
        Self {
            status: CapabilityStatus::Unsupported,
            reason: Some(reason.into()),
            implementation: None,
        }
    }

    fn implemented(available: bool, implementation: &str, reason: &str) -> Self {
        Self {
            status: if available {
                CapabilityStatus::Supported
            } else {
                CapabilityStatus::Unavailable
            },
            reason: (!available).then(|| reason.into()),
            implementation: Some(implementation.into()),
        }
    }
}

/// 启动期可读取的能力；不包含账户、凭证或敏感字段。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCapabilities {
    pub os: String,
    pub update_method: UpdateMethod,
    pub update: PlatformCapability,
    pub ocr: PlatformCapability,
    pub native_material: PlatformCapability,
    pub biometric: PlatformCapability,
    pub file_open: PlatformCapability,
}

/// 只读运行探测输入。默认全部不可用，禁止占位句柄/检测失败成为支持。
#[derive(Debug, Default)]
struct PlatformProbes {
    updater: bool,
    ocr: bool,
    native_material: bool,
    biometric: bool,
    file_open: bool,
}

fn resolve_platform_capabilities(os: &str, probes: &PlatformProbes) -> PlatformCapabilities {
    let mut result = PlatformCapabilities {
        os: os.into(),
        update_method: UpdateMethod::None,
        update: PlatformCapability::unsupported("unknown_platform"),
        ocr: PlatformCapability::unsupported("unknown_platform"),
        native_material: PlatformCapability::unsupported("unknown_platform"),
        biometric: PlatformCapability::unsupported("unknown_platform"),
        file_open: PlatformCapability::unsupported("unknown_platform"),
    };
    match os {
        "macos" | "windows" | "linux" => {
            result.update_method = UpdateMethod::Tauri;
            result.update = PlatformCapability::implemented(
                probes.updater,
                "tauri_updater",
                "updater_unavailable",
            );
            result.ocr =
                PlatformCapability::implemented(probes.ocr, "desktop_ocr", "ocr_unavailable");
            result.file_open = PlatformCapability::implemented(
                probes.file_open,
                "system_opener",
                "file_open_unavailable",
            );
            if os == "linux" {
                result.native_material =
                    PlatformCapability::unsupported("native_material_not_implemented");
                result.biometric = PlatformCapability::unsupported("biometric_not_implemented");
            } else {
                result.native_material = PlatformCapability::implemented(
                    probes.native_material,
                    if os == "macos" {
                        "macos_material"
                    } else {
                        "windows_material"
                    },
                    "native_material_unavailable",
                );
                result.biometric = PlatformCapability::implemented(
                    probes.biometric,
                    if os == "macos" {
                        "touch_id"
                    } else {
                        "windows_hello"
                    },
                    "biometric_unavailable",
                );
            }
        }
        "android" => {
            result.update_method = UpdateMethod::AndroidApk;
            result.update = PlatformCapability::implemented(
                probes.updater,
                "android_apk",
                "apk_bridge_unavailable",
            );
            result.ocr =
                PlatformCapability::implemented(probes.ocr, "ml_kit", "ocr_bridge_unavailable");
            result.native_material = PlatformCapability::implemented(
                probes.native_material,
                "android_window_blur",
                "native_material_unavailable",
            );
            result.biometric = PlatformCapability::implemented(
                probes.biometric,
                "android_keystore",
                "biometric_unavailable",
            );
            result.file_open = PlatformCapability::implemented(
                probes.file_open,
                "android_file_provider",
                "file_open_bridge_unavailable",
            );
        }
        "ios" => {
            result.update = PlatformCapability::unsupported("ios_in_app_update_not_implemented");
            result.ocr = PlatformCapability::unsupported("ios_ocr_not_implemented");
            result.native_material =
                PlatformCapability::unsupported("native_material_not_implemented");
            result.biometric = PlatformCapability::implemented(
                probes.biometric,
                "ios_biometric",
                "biometric_unavailable",
            );
            result.file_open = PlatformCapability::unsupported("ios_file_open_not_implemented");
        }
        _ => {}
    }
    result
}

/// updater 的状态类型是依赖私有类型，不能 try_state；插件初始化顺序已保证
/// 此探测发生在 updater setup 之后。保存只读构建结果，命令缺状态时安全禁用。
#[cfg(desktop)]
struct DesktopUpdaterAvailability(bool);

pub fn init_platform_capabilities<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("platform-capabilities")
        .setup(|_app, _api| {
            #[cfg(desktop)]
            {
                use tauri::Manager;
                use tauri_plugin_updater::UpdaterExt;
                _app.manage(DesktopUpdaterAvailability(_app.updater().is_ok()));
            }
            Ok(())
        })
        .build()
}

/// 仅查询编译实现/运行桥接，启动期无需 Vault 解锁；不扫描、不安装、不认证。
#[tauri::command]
pub async fn get_platform_capabilities(
    app: tauri::AppHandle,
) -> Result<PlatformCapabilities, String> {
    use tauri::Manager;
    let mut probes = PlatformProbes::default();
    #[cfg(desktop)]
    {
        probes.updater = app
            .try_state::<DesktopUpdaterAvailability>()
            .is_some_and(|s| s.0);
        // 模型安装与用户授权是后续操作的前置条件，不改变引擎实现存在这一能力。
        probes.ocr = true;
        probes.file_open = true;
        probes.native_material = cfg!(any(target_os = "macos", target_os = "windows"))
            && app.get_webview_window("main").is_some();
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            if let Some(state) = app.try_state::<crate::state::AppState>() {
                let base = state
                    .vault_service
                    .read()
                    .ok()
                    .map(|svc| svc.base_path().clone());
                if let Some(base) = base {
                    // 空账户只查询设备可用性，绝不调用可能迁移凭证的 command。
                    probes.biometric = tokio::task::spawn_blocking(move || {
                        solosoul_core::biometric::BiometricManager::new(base)
                            .availability("")
                            .available
                    })
                    .await
                    .unwrap_or(false);
                }
            }
        }
    }
    #[cfg(target_os = "android")]
    {
        probes.updater = app
            .try_state::<crate::update_plugin::UpdatePluginHandle<tauri::Wry>>()
            .is_some();
        probes.ocr = app
            .try_state::<crate::mobile_ocr_plugin::MobileOcrPluginHandle<tauri::Wry>>()
            .is_some();
        probes.file_open = app
            .try_state::<crate::attachment_import_plugin::AttachmentImportPluginHandle<tauri::Wry>>(
            )
            .is_some();
        if app
            .try_state::<crate::android_glass_plugin::AndroidGlassHandle<tauri::Wry>>()
            .is_some()
        {
            probes.native_material =
                crate::android_glass_plugin::android_glass_capabilities(app.clone())
                    .await
                    .ok()
                    .and_then(|v| v["windowBlur"].as_bool())
                    .unwrap_or(false);
        }
        let android_app = app.clone();
        probes.biometric = tokio::task::spawn_blocking(move || {
            android_app
                .try_state::<crate::keystore_plugin::KeystorePluginHandle<tauri::Wry>>()
                .and_then(|s| s.check_biometric_availability().ok())
                .is_some_and(|s| (s.strong_available || s.weak_available) && !s.lockout)
        })
        .await
        .unwrap_or(false);
    }
    #[cfg(target_os = "ios")]
    {
        let ios_app = app.clone();
        probes.biometric = tokio::task::spawn_blocking(move || {
            ios_app
                .try_state::<tauri_plugin_biometric::Biometric<tauri::Wry>>()
                .and_then(|s| s.status().ok())
                .is_some_and(|s| s.is_available)
        })
        .await
        .unwrap_or(false);
    }
    let capabilities = resolve_platform_capabilities(std::env::consts::OS, &probes);
    tracing::info!(
        "[platform-capabilities] {}",
        serde_json::to_string(&capabilities).map_err(|e| e.to_string())?
    );
    Ok(capabilities)
}

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

    #[test]
    fn rf205_fixed_platform_contract_fixtures_match_compiled_matrix() {
        let fixtures: serde_json::Value = serde_json::from_str(include_str!(
            "../../../src/lib/__fixtures__/platformCapabilities.json"
        ))
        .unwrap();
        for os in ["macos", "windows", "linux", "android", "ios", "future-os"] {
            let actual =
                serde_json::to_value(resolve_platform_capabilities(os, &all_bridges_available()))
                    .unwrap();
            assert_eq!(actual, fixtures[os], "平台契约 fixture 不一致: {os}");
        }
    }

    fn all_bridges_available() -> PlatformProbes {
        PlatformProbes {
            updater: true,
            ocr: true,
            native_material: true,
            biometric: true,
            file_open: true,
        }
    }

    #[test]
    fn rf205_unknown_platform_never_enables_any_action() {
        let value = resolve_platform_capabilities("future-os", &all_bridges_available());
        assert_eq!(value.update_method, UpdateMethod::None);
        for capability in [
            value.update,
            value.ocr,
            value.native_material,
            value.biometric,
            value.file_open,
        ] {
            assert_eq!(capability.status, CapabilityStatus::Unsupported);
            assert_eq!(capability.reason.as_deref(), Some("unknown_platform"));
            assert!(capability.implementation.is_none());
        }
    }

    #[test]
    fn rf205_missing_bridges_disable_actions_without_erasing_the_install_method() {
        for (os, method) in [
            ("macos", UpdateMethod::Tauri),
            ("windows", UpdateMethod::Tauri),
            ("linux", UpdateMethod::Tauri),
            ("android", UpdateMethod::AndroidApk),
        ] {
            let value = resolve_platform_capabilities(os, &PlatformProbes::default());
            assert_eq!(value.update_method, method);
            for capability in [value.update, value.ocr, value.file_open] {
                assert_eq!(capability.status, CapabilityStatus::Unavailable);
                assert!(capability.reason.is_some());
                assert!(capability.implementation.is_some());
            }
        }
    }

    #[test]
    fn rf205_ios_placeholder_bridges_cannot_enable_android_actions() {
        let value = resolve_platform_capabilities("ios", &all_bridges_available());
        assert_eq!(value.update_method, UpdateMethod::None);
        assert_eq!(value.biometric.status, CapabilityStatus::Supported);
        for capability in [
            value.update,
            value.ocr,
            value.native_material,
            value.file_open,
        ] {
            assert_eq!(capability.status, CapabilityStatus::Unsupported);
            assert!(capability.reason.is_some());
        }
    }

    #[test]
    fn rf205_linux_retains_desktop_fallback_without_claiming_native_material_or_biometric() {
        let value = resolve_platform_capabilities("linux", &all_bridges_available());
        assert_eq!(value.update_method, UpdateMethod::Tauri);
        assert_eq!(value.update.status, CapabilityStatus::Supported);
        assert_eq!(value.ocr.status, CapabilityStatus::Supported);
        assert_eq!(value.file_open.status, CapabilityStatus::Supported);
        assert_eq!(value.native_material.status, CapabilityStatus::Unsupported);
        assert_eq!(value.biometric.status, CapabilityStatus::Unsupported);
    }

    #[test]
    fn rf205_capability_wire_format_has_explicit_status_and_nullable_reason() {
        let value = resolve_platform_capabilities("android", &all_bridges_available());
        let json = serde_json::to_value(value).unwrap();
        assert_eq!(json["updateMethod"], "android_apk");
        assert_eq!(json["ocr"]["status"], "supported");
        assert_eq!(json["ocr"]["implementation"], "ml_kit");
        assert!(json["ocr"]["reason"].is_null());
        assert!(json.get("nativeMaterial").is_some());
        assert!(json.get("fileOpen").is_some());
        assert_eq!(json.as_object().unwrap().len(), 7);
    }

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
            // sys-locale 在 Unix 可返回 C/POSIX 或仅语言标识，不能强制地区分隔符。
            assert!(
                l.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
                "locale should contain only identifier characters, got: {}",
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
