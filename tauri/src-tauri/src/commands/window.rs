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
        use objc2_app_kit::{NSColor, NSWindow};

        let ptr = window
            .ns_window()
            .map_err(|e| format!("无法获取 NSWindow: {}", e))? as *mut NSWindow;
        let ns_window = unsafe { &*ptr };
        let bg = NSColor::colorWithRed_green_blue_alpha(
            color.red as f64 / 255.0,
            color.green as f64 / 255.0,
            color.blue as f64 / 255.0,
            1.0,
        );
        ns_window.setBackgroundColor(Some(&bg));
    }

    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_CAPTION_COLOR};

        let hwnd = window.hwnd().map_err(|e| format!("无法获取 HWND: {}", e))?;

        // 将 RGB 打包为 Windows COLORREF (0x00BBGGRR)
        let caption_color: u32 =
            ((color.blue as u32) << 16) | ((color.green as u32) << 8) | (color.red as u32);

        unsafe {
            DwmSetWindowAttribute(
                HWND(hwnd as isize),
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
