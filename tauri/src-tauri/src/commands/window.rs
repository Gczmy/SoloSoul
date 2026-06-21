use serde::Deserialize;

#[derive(Deserialize)]
pub struct TitlebarColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

/// 设置 macOS 原生窗口背景色（影响透明标题栏的交通灯区域）。
/// 其他平台直接忽略，避免编译与运行时问题。
#[tauri::command]
pub fn set_titlebar_color(window: tauri::Window, color: TitlebarColor) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{
            NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua,
            NSAppearanceNameDarkAqua, NSColor, NSWindow,
        };

        let ptr = window
            .ns_window()
            .map_err(|e| format!("无法获取 NSWindow: {}", e))? as *mut NSWindow;
        if ptr.is_null() {
            return Err("NSWindow pointer is null".to_string());
        }
        let ns_window = unsafe { &*ptr };
        let bg = NSColor::colorWithRed_green_blue_alpha(
            color.red as f64 / 255.0,
            color.green as f64 / 255.0,
            color.blue as f64 / 255.0,
            1.0,
        );
        ns_window.setBackgroundColor(Some(&bg));

        // 根据标题栏背景亮度设置窗口 appearance，确保深色主题下标题文字为白色。
        let luminance = 0.299 * f64::from(color.red)
            + 0.587 * f64::from(color.green)
            + 0.114 * f64::from(color.blue);
        let appearance_name = if luminance < 128.0 {
            unsafe { NSAppearanceNameDarkAqua }
        } else {
            unsafe { NSAppearanceNameAqua }
        };
        let appearance = NSAppearance::appearanceNamed(appearance_name);
        ns_window.setAppearance(appearance.as_deref());
    }

    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_CAPTION_COLOR};

        let hwnd = window.hwnd().map_err(|e| format!("无法获取 HWND: {}", e))?;

        // 将 RGB 打包为 Windows COLORREF (0x00BBGGRR)
        let caption_color: u32 =
            ((color.blue as u32) << 16) | ((color.green as u32) << 8) | (color.red as u32);

        unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_CAPTION_COLOR,
                &caption_color as *const u32 as *const std::ffi::c_void,
                std::mem::size_of::<u32>() as u32,
            )
            .map_err(|e| format!("DwmSetWindowAttribute 失败: {:?}", e))?;
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = window;
        let _ = color;
    }

    Ok(())
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
}
