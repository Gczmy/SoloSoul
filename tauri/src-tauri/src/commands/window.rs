use serde::{Deserialize, Serialize};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[derive(Clone, Copy, Deserialize)]
pub struct TitlebarColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

/// 根据 sRGB 分量计算感知亮度（Rec. 601 luma）。
/// 返回 0.0 ~ 255.0，用于判断标题栏使用深色还是浅色 appearance。
/// 仅 macOS 分支调用；Windows/Linux 下非 test 构建为死代码，此处允许。
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn calculate_luminance(color: &TitlebarColor) -> f64 {
    0.299 * f64::from(color.red) + 0.587 * f64::from(color.green) + 0.114 * f64::from(color.blue)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowAppearance {
    pub material: &'static str,
    pub platform: &'static str,
    pub reduce_motion: bool,
    pub high_contrast: bool,
}

/// 主题同步保持原生材质；AppKit 通过 with_webview 保证在主线程调用。
#[tauri::command]
pub async fn set_titlebar_color(
    window: tauri::WebviewWindow,
    color: TitlebarColor,
) -> Result<WindowAppearance, String> {
    #[cfg(target_os = "macos")]
    {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let target = window.clone();
        window
            .with_webview(move |native| {
                let _ = sender.send(macos::apply(&target, native, color));
            })
            .map_err(|e| e.to_string())?;
        receiver.await.map_err(|e| e.to_string())?
    }

    #[cfg(target_os = "windows")]
    {
        windows::apply(&window, color)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = window;
        let _ = color;
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    Ok(WindowAppearance {
        material: "solid",
        platform: if cfg!(target_os = "windows") {
            "windows"
        } else {
            "other"
        },
        reduce_motion: false,
        high_contrast: false,
    })
}

static WINDOW_SHOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[tauri::command]
pub fn show_main_window(window: tauri::WebviewWindow) -> Result<(), String> {
    if window.label() != "main" {
        return Err("Only the main window can finish startup".into());
    }
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    if !WINDOW_SHOWN.load(std::sync::atomic::Ordering::Relaxed) {
        use tauri_plugin_window_state::{StateFlags, WindowExt};
        let _ = window.restore_state(StateFlags::MAXIMIZED | StateFlags::FULLSCREEN);
    }
    window.show().map_err(|e| e.to_string())?;
    WINDOW_SHOWN.store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}

pub(crate) fn setup_startup_window(app: &tauri::AppHandle) {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        use tauri::Manager;
        use tauri_plugin_window_state::{StateFlags, WindowExt};
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.restore_state(StateFlags::SIZE | StateFlags::POSITION);
        }
        let handle = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(12)).await;
            // 模块或 WebView 初始化失败时仍呈现静态错误页，避免窗口永久不可见。
            if !WINDOW_SHOWN.load(std::sync::atomic::Ordering::Relaxed) {
                if let Some(window) = handle.get_webview_window("main") {
                    let _ = show_main_window(window);
                }
            }
        });
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let _ = app;
}

pub(crate) fn poll_accessibility(app: &tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    macos::poll_accessibility(app);
    #[cfg(target_os = "windows")]
    windows::poll_accessibility(app);
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let _ = app;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_titlebar_color_deserialization() {
        let json = r#"{"red": 255, "green": 128, "blue": 64}"#;
        let color: TitlebarColor = serde_json::from_str(json).unwrap();
        assert_eq!(color.red, 255);
        assert_eq!(color.green, 128);
        assert_eq!(color.blue, 64);
    }

    #[test]
    fn test_titlebar_color_black() {
        let json = r#"{"red": 0, "green": 0, "blue": 0}"#;
        let color: TitlebarColor = serde_json::from_str(json).unwrap();
        assert_eq!(color.red, 0);
        assert_eq!(color.green, 0);
        assert_eq!(color.blue, 0);
    }

    #[test]
    fn test_titlebar_color_white() {
        let json = r#"{"red": 255, "green": 255, "blue": 255}"#;
        let color: TitlebarColor = serde_json::from_str(json).unwrap();
        assert_eq!(color.red, 255);
        assert_eq!(color.green, 255);
        assert_eq!(color.blue, 255);
    }

    #[test]
    fn test_titlebar_color_rejects_negative() {
        let json = r#"{"red": -1, "green": 0, "blue": 0}"#;
        let result = serde_json::from_str::<TitlebarColor>(json);
        // u8 deserialization rejects negative values
        assert!(result.is_err());
    }

    #[test]
    fn test_titlebar_color_rejects_above_255() {
        let json = r#"{"red": 256, "green": 0, "blue": 0}"#;
        let result = serde_json::from_str::<TitlebarColor>(json);
        // u8 deserialization rejects values > 255
        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_luminance_black() {
        let color = TitlebarColor {
            red: 0,
            green: 0,
            blue: 0,
        };
        assert_eq!(calculate_luminance(&color), 0.0);
    }

    #[test]
    fn test_calculate_luminance_white() {
        let color = TitlebarColor {
            red: 255,
            green: 255,
            blue: 255,
        };
        assert_eq!(calculate_luminance(&color), 255.0);
    }

    #[test]
    fn test_calculate_luminance_mid_gray() {
        // 128,128,128 → luma ≈ 128（浮点误差），精确值 ≈ 127.99999999999999
        let color = TitlebarColor {
            red: 128,
            green: 128,
            blue: 128,
        };
        let luma = calculate_luminance(&color);
        assert!(
            (luma - 128.0).abs() < 1e-12,
            "expected ~128.0, got {}",
            luma
        );
    }

    #[test]
    fn test_calculate_luminance_dark_theme_threshold() {
        // 纯红 (255,0,0) → luma = 0.299 * 255 ≈ 76.2 < 128 → 深色
        let color = TitlebarColor {
            red: 255,
            green: 0,
            blue: 0,
        };
        let luma = calculate_luminance(&color);
        assert!(luma < 128.0, "red luma {} should be < 128", luma);
    }

    #[test]
    fn test_calculate_luminance_light_theme_threshold() {
        // 纯黄 (255,255,0) → luma = 0.299*255 + 0.587*255 ≈ 225.9 >= 128 → 浅色
        let color = TitlebarColor {
            red: 255,
            green: 255,
            blue: 0,
        };
        let luma = calculate_luminance(&color);
        assert!(luma >= 128.0, "yellow luma {} should be >= 128", luma);
    }

    #[test]
    fn test_calculate_luminance_blue_is_dark() {
        // 纯蓝 (0,0,255) → luma = 0.114 * 255 ≈ 29.1 < 128 → 深色
        let color = TitlebarColor {
            red: 0,
            green: 0,
            blue: 255,
        };
        let luma = calculate_luminance(&color);
        assert!(luma < 128.0, "blue luma {} should be < 128", luma);
    }

    #[test]
    fn test_calculate_luminance_green_is_bright() {
        // 纯绿 (0,255,0) → luma = 0.587 * 255 ≈ 149.7 >= 128 → 浅色
        let color = TitlebarColor {
            red: 0,
            green: 255,
            blue: 0,
        };
        let luma = calculate_luminance(&color);
        assert!(luma >= 128.0, "green luma {} should be >= 128", luma);
    }

    #[test]
    fn test_titlebar_color_rejects_empty() {
        let json = r#"{}"#;
        let result = serde_json::from_str::<TitlebarColor>(json);
        assert!(result.is_err());
    }
}
