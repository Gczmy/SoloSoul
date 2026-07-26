//! Android 锁屏状态检测插件
//!
//! 通过 `KeyguardManager.isKeyguardLocked` 判断设备当前是否处于锁屏状态，
//! 供前端 `useAutoLock` 区分「系统锁屏」与「仅切后台」。

use serde::{Deserialize, Serialize};
use tauri::{
    plugin::{Builder, PluginApi, TauriPlugin},
    AppHandle, Manager, Runtime,
};

#[cfg(target_os = "android")]
use tauri::plugin::PluginHandle;

#[cfg(target_os = "android")]
const PLUGIN_IDENTIFIER: &str = "com.solosoul.app";

/// 锁屏状态查询响应。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IsScreenLockedResponse {
    pub locked: bool,
}

/// 插件句柄包装，便于在 command 中通过 Tauri state 获取。
pub struct LockStatePluginHandle<R: Runtime> {
    #[cfg(target_os = "android")]
    handle: PluginHandle<R>,
    #[cfg(not(target_os = "android"))]
    _phantom: std::marker::PhantomData<fn() -> R>,
}

impl<R: Runtime> LockStatePluginHandle<R> {
    /// 查询当前是否处于锁屏状态。
    /// 非 Android 平台始终返回 false（桌面端没有对应的锁屏监听）。
    pub fn is_screen_locked(&self) -> Result<bool, String> {
        #[cfg(target_os = "android")]
        {
            let response: IsScreenLockedResponse = self
                .handle
                .run_mobile_plugin("isScreenLocked", serde_json::json!({}))
                .map_err(|e| e.to_string())
                .and_then(|v| {
                    serde_json::from_value(v).map_err(|e| format!("Invalid response: {}", e))
                })?;
            Ok(response.locked)
        }
        #[cfg(not(target_os = "android"))]
        {
            Ok(false)
        }
    }

    /// 撤掉锁屏时显示的原生窗口遮盖层。
    /// 非 Android 平台为 no-op（没有对应的原生遮盖）。
    pub fn dismiss_lock_mask(&self) -> Result<(), String> {
        #[cfg(target_os = "android")]
        {
            self.handle
                .run_mobile_plugin::<serde_json::Value>("dismissLockMask", serde_json::json!({}))
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            Ok(())
        }
    }

    /// 查询是否有未被 JS 确认的锁屏挂起标记。
    /// 前端启动/认证后主动拉取，闭合「事件已丢失但标记仍在」的环路。
    /// 非 Android 平台始终返回 false。
    pub fn get_lock_pending(&self) -> Result<bool, String> {
        #[cfg(target_os = "android")]
        {
            #[derive(Debug, Clone, Deserialize)]
            struct Wrapper {
                pending: bool,
            }
            self.handle
                .run_mobile_plugin::<serde_json::Value>("getLockPending", serde_json::json!({}))
                .map_err(|e| e.to_string())
                .and_then(|v| serde_json::from_value::<Wrapper>(v).map_err(|e| e.to_string()))
                .map(|w| w.pending)
        }
        #[cfg(not(target_os = "android"))]
        {
            Ok(false)
        }
    }
}

/// 初始化插件：注册 Android Kotlin 插件并将句柄存入 state。
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("lock-state")
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
    let handle = api.register_android_plugin(PLUGIN_IDENTIFIER, "LockStatePlugin")?;
    app.manage(LockStatePluginHandle { handle });
    Ok(())
}

#[cfg(not(target_os = "android"))]
fn register_plugin<R: Runtime>(
    app: &AppHandle<R>,
    _api: PluginApi<R, ()>,
) -> Result<(), Box<dyn std::error::Error>> {
    app.manage(LockStatePluginHandle {
        _phantom: std::marker::PhantomData::<fn() -> R>,
    });
    Ok(())
}

/// 查询当前是否处于锁屏状态。
/// 桌面端/iOS 始终返回 false。
#[tauri::command]
pub fn is_screen_locked<R: Runtime>(app: AppHandle<R>) -> Result<bool, String> {
    let handle = app.state::<LockStatePluginHandle<R>>();
    handle.is_screen_locked()
}

/// 撤掉锁屏时显示的原生窗口遮盖层。
/// 前端完成锁定并进入登录页后调用；桌面端/iOS 为 no-op。
#[tauri::command]
pub fn dismiss_lock_mask<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let handle = app.state::<LockStatePluginHandle<R>>();
    handle.dismiss_lock_mask()
}

/// 查询是否有未被 JS 确认的锁屏挂起标记。
/// 前端启动/认证后主动拉取；桌面端/iOS 始终返回 false。
#[tauri::command]
pub fn get_lock_pending<R: Runtime>(app: AppHandle<R>) -> Result<bool, String> {
    let handle = app.state::<LockStatePluginHandle<R>>();
    handle.get_lock_pending()
}
