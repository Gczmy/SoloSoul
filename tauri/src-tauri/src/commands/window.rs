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

    #[cfg(not(target_os = "macos"))]
    {
        let _ = window;
        let _ = color;
    }

    Ok(())
}
