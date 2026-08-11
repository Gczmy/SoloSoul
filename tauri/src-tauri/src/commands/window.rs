use serde::Deserialize;

#[derive(Deserialize)]
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
        // SAFETY: ptr 是 Tauri 框架通过 ns_window() 返回的有效 NSWindow 指针，
        // 已经过非空检查。在 Tauri 窗口生命周期内该指针始终有效。&*ptr 创建
        // 一个借用引用，不转移所有权，Tauri 负责 NSWindow 的生命周期。
        let ns_window = unsafe { &*ptr };
        let bg = NSColor::colorWithRed_green_blue_alpha(
            color.red as f64 / 255.0,
            color.green as f64 / 255.0,
            color.blue as f64 / 255.0,
            1.0,
        );
        ns_window.setBackgroundColor(Some(&bg));

        // 根据标题栏背景亮度设置窗口 appearance，确保深色主题下标题文字为白色。
        let luminance = calculate_luminance(&color);
        let appearance_name = if luminance < 128.0 {
            // SAFETY: NSAppearanceNameDarkAqua 是 AppKit 公开的全局 NSString 常量，
            // 从 ObjC extern 静态变量中读取不会产生数据竞争或内存安全问题。
            unsafe { NSAppearanceNameDarkAqua }
        } else {
            // SAFETY: NSAppearanceNameAqua 是 AppKit 公开的全局 NSString 常量。
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

        // SAFETY: DwmSetWindowAttribute 是 Windows DWM API；hwnd 是 Tauri 提供的
        // 有效窗口句柄，caption_color 是栈上分配的 u32 值，size 正确。调用期间
        // 不会修改 Rust 内存，仅由 DWM 读取标题栏颜色配置。
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
