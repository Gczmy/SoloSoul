//! Android 状态栏/导航栏图标颜色控制
//!
//! 由于 Tauri v2 的 `AppHandle` 不公开 `run_on_android_context`，无法直接从普通
//! command 中修改系统栏前景色。这里使用一个最小化的 Tauri mobile plugin 仅用于
//! 在 setup 中获取 `PluginHandle`，并把该 handle 存入 Tauri state；真正的 command
//! 通过主应用的 `generate_handler!` 注册，从而避免额外的 plugin ACL 权限配置。
//!
//! Kotlin 端通过 `WindowInsetsControllerCompat` 动态设置
//! `isAppearanceLightStatusBars` / `isAppearanceLightNavigationBars`。
//! 前端在切换主题时调用 `set_status_bar_style`，传入 `"dark"` 或 `"light"`，
//! 使系统栏图标/文字颜色与应用主题保持一致。

use serde::{Deserialize, Serialize};
use tauri::{
    plugin::{Builder, PluginApi, TauriPlugin},
    AppHandle, Manager, Runtime,
};

#[cfg(target_os = "android")]
use tauri::plugin::PluginHandle;

/// 状态栏风格参数。
#[derive(Debug, Deserialize, Serialize)]
pub struct SetStatusBarStylePayload {
    pub style: String,
}

/// Android 插件标识。
#[cfg(target_os = "android")]
const PLUGIN_IDENTIFIER: &str = "com.solosoul.app";

/// 插件句柄包装，便于在 command 中通过 Tauri state 获取。
/// 桌面端无实际句柄，使用函数指针 PhantomData 保证 StatusBarPluginHandle
/// 自动实现 Send + Sync，满足 Tauri state 的约束。
pub struct StatusBarPluginHandle<R: Runtime> {
    #[cfg(target_os = "android")]
    handle: PluginHandle<R>,
    #[cfg(not(target_os = "android"))]
    _phantom: std::marker::PhantomData<fn() -> R>,
}

impl<R: Runtime> StatusBarPluginHandle<R> {
    /// 设置状态栏/导航栏图标风格。
    /// - `"light"` → 深色图标/文字（浅色背景）。
    /// - `"dark"`  → 浅色图标/文字（深色背景）。
    pub fn set_style(&self, style: String) -> Result<(), String> {
        #[cfg(target_os = "android")]
        {
            self.handle
                .run_mobile_plugin("setStyle", SetStatusBarStylePayload { style })
                .map(|_: serde_json::Value| ())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = style;
            Ok(())
        }
    }
}

/// 初始化插件：注册 Android Kotlin 插件并将句柄存入 state。
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("status-bar")
        .setup(|_app, api| {
            register_plugin::<R>(_app, api)?;
            Ok(())
        })
        .build()
}

#[cfg(target_os = "android")]
fn register_plugin<R: Runtime>(
    app: &AppHandle<R>,
    api: PluginApi<R, ()>,
) -> Result<(), Box<dyn std::error::Error>> {
    let handle = api.register_android_plugin(PLUGIN_IDENTIFIER, "StatusBarPlugin")?;
    app.manage(StatusBarPluginHandle { handle });
    Ok(())
}

#[cfg(not(target_os = "android"))]
fn register_plugin<R: Runtime>(
    app: &AppHandle<R>,
    _api: PluginApi<R, ()>,
) -> Result<(), Box<dyn std::error::Error>> {
    app.manage(StatusBarPluginHandle {
        _phantom: std::marker::PhantomData::<fn() -> R>,
    });
    Ok(())
}

/// 设置 Android 状态栏与导航栏图标/文字风格。
/// - `"light"`：深色图标/文字（浅色背景）。
/// - `"dark"`：浅色图标/文字（深色背景）。
/// 非 Android 平台直接忽略。
#[tauri::command]
pub fn set_status_bar_style<R: Runtime>(
    app: AppHandle<R>,
    payload: SetStatusBarStylePayload,
) -> Result<(), String> {
    let handle = app.state::<StatusBarPluginHandle<R>>();
    handle.set_style(payload.style)
}
