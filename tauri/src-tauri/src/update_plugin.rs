//! Android APK 安装插件桥接。
//!
//! 将 Rust 后端下载到缓存目录的 APK 文件路径传递给 Kotlin `UpdatePlugin`，
//! 由 Kotlin 端通过 FileProvider + ACTION_VIEW Intent 触发系统安装器。
//!
//! 桌面端无实际功能，接收调用时返回错误。

use serde::{Deserialize, Serialize};
use tauri::{
    plugin::{Builder, PluginApi, TauriPlugin},
    AppHandle, Manager, Runtime,
};

#[cfg(target_os = "android")]
use tauri::plugin::PluginHandle;

/// Android 插件包名。
#[cfg(target_os = "android")]
const PLUGIN_IDENTIFIER: &str = "com.solosoul.app";

/// 调用 Kotlin 插件时传入的参数。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallApkPayload {
    pub file_path: String,
}

/// Kotlin 插件返回的安装结果。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallApkResult {
    pub success: bool,
}

/// 插件句柄包装，便于在 command 中通过 Tauri state 获取。
pub struct UpdatePluginHandle<R: Runtime> {
    #[cfg(target_os = "android")]
    handle: PluginHandle<R>,
    #[cfg(not(target_os = "android"))]
    _phantom: std::marker::PhantomData<fn() -> R>,
}

impl<R: Runtime> UpdatePluginHandle<R> {
    /// 在 Android 端通过 FileProvider + Intent 安装 APK。
    /// 非 Android 平台直接返回不支持错误。
    pub fn install_apk(&self, payload: InstallApkPayload) -> Result<InstallApkResult, String> {
        #[cfg(target_os = "android")]
        {
            self.handle
                .run_mobile_plugin("installApk", payload)
                .map_err(|e| e.to_string())
                .and_then(|v| {
                    serde_json::from_value::<InstallApkResult>(v).map_err(|e| e.to_string())
                })
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = payload;
            Err("APK 安装仅支持 Android 平台".to_string())
        }
    }
}

/// 初始化插件：注册 Android Kotlin 插件并将句柄存入 state。
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("update")
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
    let handle = api.register_android_plugin(PLUGIN_IDENTIFIER, "UpdatePlugin")?;
    app.manage(UpdatePluginHandle { handle });
    Ok(())
}

#[cfg(not(target_os = "android"))]
fn register_plugin<R: Runtime>(
    app: &AppHandle<R>,
    _api: PluginApi<R, ()>,
) -> Result<(), Box<dyn std::error::Error>> {
    app.manage(UpdatePluginHandle {
        _phantom: std::marker::PhantomData::<fn() -> R>,
    });
    Ok(())
}

/// 安装已下载的 APK（移动端入口，调用 Kotlin 插件）。
#[tauri::command]
pub async fn android_install_apk<R: Runtime>(
    app: AppHandle<R>,
    file_path: String,
) -> Result<InstallApkResult, String> {
    let handle = app.state::<UpdatePluginHandle<R>>();
    handle.install_apk(InstallApkPayload { file_path })
}
