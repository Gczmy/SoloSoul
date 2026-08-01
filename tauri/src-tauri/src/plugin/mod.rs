//! SoloSoul 插件系统（Tauri 薄封装）
//!
//! 全部共享类型与实现逻辑从 `solosoul-plugin` crate 重新导出，
//! 本模块仅保留 Tauri 特有的适配层：
//!
//! - [`TauriChannelSink`]：把 `tauri::ipc::Channel<PluginEvent>` 适配为
//!   crate 的 [`PluginEventSink`]（GUI 前端事件通道）。
//! - [`resolve_market_dir`] / [`new_plugin_manager`]：基于 Tauri 应用句柄
//!   解析市场目录与数据目录并构造 [`PluginManager`]。
//!
//! P012 方向 B 完成后，本地 `event/host/manager/paths/registry/sandbox`
//! 六组实现已全部收敛进 crate，此处不再重复实现（单一实现源）。

// 模块路径兼容（如 `solo_soul::plugin::registry::PluginRegistry`）
pub use solosoul_plugin::{
    audit, consent, error, event, field, host, manager, manifest, paths, rate_limiter, registry,
    sandbox, session, store, version,
};

// 顶层类型重导出（与 crate 保持一致）
pub use solosoul_plugin::audit::PluginAuditLogger;
pub use solosoul_plugin::consent::ConsentManager;
pub use solosoul_plugin::error::PluginError;
pub use solosoul_plugin::event::{PluginEvent, PluginEventSink};
pub use solosoul_plugin::field::FieldResolver;
pub use solosoul_plugin::host::{register_host_functions, SoloHostFunctions, SoloHostState};
pub use solosoul_plugin::manager::PluginManager;
pub use solosoul_plugin::manifest::{
    MarketPluginInfo, PluginAuditAction, PluginAuditEntry, PluginContractBinding,
    PluginContractRole, PluginFieldBinding, PluginInstallResult, PluginLogLine, PluginManifest,
    PluginNetworkPolicy, PluginParam, PluginResult, PluginResultPayload, PluginTier, RegistryEntry,
    RegistryVersion,
};
pub use solosoul_plugin::rate_limiter::{RateLimiter, RateLimiterMap};
pub use solosoul_plugin::registry::PluginRegistry;
pub use solosoul_plugin::sandbox::WasmSandbox;
pub use solosoul_plugin::session::{PluginSession, PluginSessionManager};
pub use solosoul_plugin::store::{compute_sha256, PluginStore};

use std::path::PathBuf;
use tauri::ipc::Channel;
use tauri::Manager;

/// Tauri IPC `Channel` → crate [`PluginEventSink`] 适配器。
///
/// GUI 前端通过 `tauri::ipc::Channel` 接收插件事件；crate 侧统一以
/// [`PluginEventSink`] trait 发送（CLI 亦实现了自己的 sink），
/// 这里把两者桥接起来。
pub struct TauriChannelSink(Channel<PluginEvent>);

impl TauriChannelSink {
    pub fn new(channel: Channel<PluginEvent>) -> Self {
        Self(channel)
    }
}

impl PluginEventSink for TauriChannelSink {
    fn send(&self, event: PluginEvent) -> Result<(), String> {
        self.0.send(event).map_err(|e| e.to_string())
    }
}

/// 解析插件市场根目录（包含 `registry.json` 与 `plugins/`）
///
/// - 优先使用 Tauri 资源目录中的 `SoloSoul_plugin_market`
/// - Android 上 Tauri 的 `resource_dir` 返回 asset:// URL，无法被 `std::fs`
///   直接读取；MainActivity 已将资源复制到 `{filesDir}/app_resources/`，
///   因此移动端优先使用该私有目录
/// - 开发模式（`debug_assertions`）下若不存在，回退到源码相对路径
pub fn resolve_market_dir(app_handle: Option<&tauri::AppHandle>) -> Result<PathBuf, PluginError> {
    if let Some(app) = app_handle {
        #[cfg(target_os = "android")]
        let resource_dir = app
            .path()
            .resolve(".", tauri::path::BaseDirectory::Data)
            .ok()
            .map(|d| d.join("app_resources"));
        #[cfg(not(target_os = "android"))]
        let resource_dir = app.path().resource_dir().ok();

        if let Some(resource_dir) = resource_dir {
            let bundled = resource_dir.join("SoloSoul_plugin_market");
            if bundled.join("registry.json").exists() || bundled.join("plugins").exists() {
                return Ok(bundled);
            }
        }
    }

    #[cfg(debug_assertions)]
    {
        let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("SoloSoul_plugin_market");
        if dev.join("registry.json").exists() || dev.join("plugins").exists() {
            return Ok(dev);
        }
    }

    Err(PluginError::RegistryError(
        "无法定位插件市场目录".to_string(),
    ))
}

/// 基于 Tauri 应用句柄构造插件管理器。
///
/// 移动端使用 Tauri 应用私有数据目录（`Data/.solosoul`），
/// 桌面端使用 `~/.solosoul`（`PluginStore::data_dir`）。
pub fn new_plugin_manager(app_handle: &tauri::AppHandle) -> Result<PluginManager, PluginError> {
    let market_dir = resolve_market_dir(Some(app_handle))?;

    #[cfg(any(target_os = "android", target_os = "ios"))]
    let data_dir = {
        app_handle
            .path()
            .resolve(".", tauri::path::BaseDirectory::Data)
            .map_err(|e| PluginError::StoreError(format!("无法解析应用数据目录: {}", e)))?
            .join(".solosoul")
    };
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    let data_dir = PluginStore::data_dir()?;

    PluginManager::new_with_dirs(market_dir, data_dir)
}
