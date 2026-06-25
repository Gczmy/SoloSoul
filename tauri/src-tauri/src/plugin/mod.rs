//! SoloSoul 插件系统核心
//!
//! 共享类型从 `solosoul-plugin` crate 重新导出，
//! Tauri 特有模块保持本地实现。

// 从 crate 重新导出已删除的共享模块（保持 super::xxx 路径兼容）
pub use solosoul_plugin::{audit, consent, error, manifest, rate_limiter, session, store};

// 从 crate 重新导出的类型（无本地副本）
pub use solosoul_plugin::audit::PluginAuditLogger;
pub use solosoul_plugin::consent::ConsentManager;
pub use solosoul_plugin::error::PluginError;

// PluginEvent、FieldResolver、PluginRegistry、WasmSandbox 使用本地模块（Tauri 特有）
pub use solosoul_plugin::manifest::{
    MarketPluginInfo, PluginAuditAction, PluginAuditEntry, PluginInstallResult, PluginLogLine,
    PluginManifest, PluginNetworkPolicy, PluginParam, PluginContractBinding, PluginFieldBinding,
    PluginResult, PluginResultPayload, PluginTier, RegistryEntry, RegistryVersion,
};
pub use solosoul_plugin::rate_limiter::{RateLimiter, RateLimiterMap};
pub use solosoul_plugin::session::{PluginSession, PluginSessionInfo, PluginSessionManager};
pub use solosoul_plugin::store::{compute_sha256, PluginStore};

// Tauri 特有本地模块
pub mod event;
pub mod field;
pub mod host;
pub mod manager;
pub mod paths;
pub mod registry;
pub mod sandbox;

// 从本地模块重新导出（保持 super::xxx 兼容）
pub use manager::PluginManager;
pub use event::PluginEvent;
pub use field::FieldResolver;
pub use host::{register_host_functions, SoloHostFunctions, SoloHostState};
pub use registry::PluginRegistry;
pub use sandbox::WasmSandbox;
