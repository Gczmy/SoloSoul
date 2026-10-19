//! 系统外观通知独立于窗口的显式主题，避免手动模式遮蔽系统变化。
use std::sync::atomic::{AtomicU8, Ordering};
use tauri::Emitter;
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
use tauri::Manager;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

static LAST_THEME: AtomicU8 = AtomicU8::new(u8::MAX);

pub(super) fn notify(app: &tauri::AppHandle) {
    super::poll_accessibility(app);
    if let Ok(theme) = crate::commands::system::get_system_theme() {
        let mode = u8::from(theme == "dark");
        if LAST_THEME.swap(mode, Ordering::Relaxed) != mode {
            let _ = app.emit("system-theme-changed", theme);
        }
    }
}

pub(crate) fn install(app: &tauri::AppHandle) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    windows::install(app)?;
    #[cfg(target_os = "macos")]
    {
        macos::install(app)?;
        app.plugin(
            tauri::plugin::Builder::new("system-appearance")
                .on_event(|_, event| {
                    if matches!(event, tauri::RunEvent::Exit) {
                        macos::uninstall();
                    }
                })
                .build(),
        )
        .map_err(|error| error.to_string())?;
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let window = app
            .get_webview_window("main")
            .ok_or("Main window unavailable")?;
        let handle = app.clone();
        window.on_window_event(move |event| {
            if matches!(event, tauri::WindowEvent::ThemeChanged(_)) {
                notify(&handle);
            }
        });
    }
    notify(app);
    Ok(())
}
