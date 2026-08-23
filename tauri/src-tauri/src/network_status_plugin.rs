//! Android 网络状态检测插件（B-01 · 云同步 Wi-Fi only）。
//!
//! Kotlin 侧 `NetworkStatusPlugin` 经 ConnectivityManager 判定当前是否处于
//! Wi-Fi/以太网；云同步调度器在 `wifi_only` 开启时调用 [`is_on_wifi`] 门控，
//! 蜂络/无连接则跳过本轮，避免计费流量消耗。
//!
//! 桌面端恒返回 true（无 Wi-Fi only 语义）。

use tauri::{
    plugin::{Builder, PluginApi, TauriPlugin},
    AppHandle, Manager, Runtime,
};

#[cfg(target_os = "android")]
use tauri::plugin::PluginHandle;

#[cfg(target_os = "android")]
const PLUGIN_IDENTIFIER: &str = "com.solosoul.app";

/// 插件句柄包装（存入 Tauri state）。
pub struct NetworkStatusPluginHandle<R: Runtime> {
    #[cfg(target_os = "android")]
    handle: PluginHandle<R>,
    #[cfg(not(target_os = "android"))]
    _phantom: std::marker::PhantomData<fn() -> R>,
}

impl<R: Runtime> NetworkStatusPluginHandle<R> {
    /// 当前是否处于适合大流量同步的网络。检测失败按 false 处理（宁可跳过）。
    fn query_on_wifi(&self) -> Result<bool, String> {
        #[cfg(target_os = "android")]
        {
            #[derive(Debug, Clone, serde::Deserialize)]
            struct Wrapper {
                #[serde(rename = "onWifi", default)]
                on_wifi: bool,
            }
            self.handle
                .run_mobile_plugin::<serde_json::Value>("isOnWifi", serde_json::json!({}))
                .map_err(|e| e.to_string())
                .and_then(|v| serde_json::from_value::<Wrapper>(v).map_err(|e| e.to_string()))
                .map(|w| w.on_wifi)
        }
        #[cfg(not(target_os = "android"))]
        {
            Ok(true)
        }
    }
}

/// 云同步调度器使用的门控查询：桌面端恒 true；Android 查询失败按 false（跳过本轮）。
pub fn is_on_wifi<R: Runtime>(app: &AppHandle<R>) -> bool {
    let state = app.try_state::<NetworkStatusPluginHandle<R>>();
    match state {
        Some(h) => h.query_on_wifi().unwrap_or(false),
        None => {
            // 插件未初始化（如单测环境）：按可同步处理，避免阻塞调度器
            !cfg!(target_os = "android")
        }
    }
}

/// 初始化插件：注册 Android Kotlin 插件并将句柄存入 state。
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("network-status")
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
    let handle = api.register_android_plugin(PLUGIN_IDENTIFIER, "NetworkStatusPlugin")?;
    app.manage(NetworkStatusPluginHandle { handle });
    Ok(())
}

#[cfg(not(target_os = "android"))]
fn register_plugin<R: Runtime>(
    app: &AppHandle<R>,
    _api: PluginApi<R, ()>,
) -> Result<(), Box<dyn std::error::Error>> {
    app.manage(NetworkStatusPluginHandle {
        _phantom: std::marker::PhantomData::<fn() -> R>,
    });
    Ok(())
}
