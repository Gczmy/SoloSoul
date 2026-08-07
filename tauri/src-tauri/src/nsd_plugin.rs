//! Android NSD（Network Service Discovery）插件桥。
//!
//! 移动端设备发现不使用桌面端的 mdns-sd，而是通过 Android NsdManager 实现。
//! 该插件把 Rust 调用转发到 Kotlin 端，并返回发现的服务列表。

use serde::{Deserialize, Serialize};
use tauri::{
    plugin::{Builder, PluginApi, TauriPlugin},
    AppHandle, Manager, Runtime,
};

#[cfg(target_os = "android")]
use tauri::plugin::PluginHandle;

#[cfg(target_os = "android")]
const PLUGIN_IDENTIFIER: &str = "com.solosoul.app";

/// 发现的服务信息。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NsdServiceInfo {
    pub node_id: String,
    pub account_id: String,
    /// 桌面端广播的 account_hash（旧版/移动端可能缺失，默认空串）。
    #[serde(default)]
    pub account_hash: String,
    pub fingerprint: String,
    /// 对端客户端类型（macos/windows/android...），由 TXT 广播解析，默认空串。
    #[serde(default)]
    pub client_type: String,
    /// 服务的 mDNS 实例名（桌面端为 SoloSoul-<fp8> 可读设备名；旧版为 node_<uuid>）。
    /// 用于安卓端「已发现设备」显示名回退，默认空串。
    #[serde(default)]
    pub service_name: String,
    pub host: String,
    pub port: u16,
}

/// 注册服务时传入的参数。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterServicePayload {
    pub port: u16,
    pub node_id: String,
    pub account_id: String,
    pub fingerprint: String,
    /// 本机客户端类型（macos/windows/android...），广播进 TXT 供对端展示图标。
    #[serde(default)]
    pub client_type: String,
}

/// 插件句柄包装，便于在 command 中通过 Tauri state 获取。
pub struct NsdPluginHandle<R: Runtime> {
    #[cfg(target_os = "android")]
    handle: PluginHandle<R>,
    #[cfg(not(target_os = "android"))]
    _phantom: std::marker::PhantomData<fn() -> R>,
}

impl<R: Runtime> NsdPluginHandle<R> {
    /// 开始 NSD 发现。
    pub fn start_discovery(&self) -> Result<(), String> {
        #[cfg(target_os = "android")]
        {
            self.handle
                .run_mobile_plugin::<serde_json::Value>("startDiscovery", serde_json::json!({}))
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            Err("NSD discovery is only supported on Android".to_string())
        }
    }

    /// 停止 NSD 发现。
    pub fn stop_discovery(&self) -> Result<(), String> {
        #[cfg(target_os = "android")]
        {
            self.handle
                .run_mobile_plugin::<serde_json::Value>("stopDiscovery", serde_json::json!({}))
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            Err("NSD discovery is only supported on Android".to_string())
        }
    }

    /// 注册本地 NSD 服务。
    pub fn register_service(&self, payload: RegisterServicePayload) -> Result<(), String> {
        #[cfg(target_os = "android")]
        {
            self.handle
                .run_mobile_plugin::<serde_json::Value>("registerService", payload)
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = payload;
            Err("NSD service registration is only supported on Android".to_string())
        }
    }

    /// 注销本地 NSD 服务。
    pub fn unregister_service(&self) -> Result<(), String> {
        #[cfg(target_os = "android")]
        {
            self.handle
                .run_mobile_plugin::<serde_json::Value>("unregisterService", serde_json::json!({}))
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            Err("NSD service registration is only supported on Android".to_string())
        }
    }

    /// 获取当前已发现的服务列表。
    pub fn get_discovered_services(&self) -> Result<Vec<NsdServiceInfo>, String> {
        #[cfg(target_os = "android")]
        {
            #[derive(Debug, Clone, Deserialize)]
            struct Wrapper {
                services: Vec<NsdServiceInfo>,
            }
            self.handle
                .run_mobile_plugin::<serde_json::Value>(
                    "getDiscoveredServices",
                    serde_json::json!({}),
                )
                .map_err(|e| e.to_string())
                .and_then(|v| serde_json::from_value::<Wrapper>(v).map_err(|e| e.to_string()))
                .map(|w| w.services)
        }
        #[cfg(not(target_os = "android"))]
        {
            Ok(Vec::new())
        }
    }

    /// 请求 NSD 所需的运行时权限（Android 上为 NEARBY_WIFI_DEVICES 或 ACCESS_FINE_LOCATION）。
    pub fn request_permissions(&self) -> Result<(), String> {
        #[cfg(target_os = "android")]
        {
            // 命令名必须与 Kotlin 侧 @Command 方法名对齐：NsdPlugin.kt 暴露的是
            // requestNsdPermissions（不是 requestPermissions）。命令名不匹配时
            // run_mobile_plugin 直接报错 → Android 扫不到设备（Bug B）。
            self.handle
                .run_mobile_plugin::<serde_json::Value>(
                    "requestNsdPermissions",
                    serde_json::json!({}),
                )
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            Ok(())
        }
    }
}

/// 初始化插件：注册 Android Kotlin 插件并将句柄存入 state。
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("nsd")
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
    let handle = api.register_android_plugin(PLUGIN_IDENTIFIER, "NsdPlugin")?;
    app.manage(NsdPluginHandle { handle });
    Ok(())
}

#[cfg(not(target_os = "android"))]
fn register_plugin<R: Runtime>(
    app: &AppHandle<R>,
    _api: PluginApi<R, ()>,
) -> Result<(), Box<dyn std::error::Error>> {
    app.manage(NsdPluginHandle {
        _phantom: std::marker::PhantomData::<fn() -> R>,
    });
    Ok(())
}
