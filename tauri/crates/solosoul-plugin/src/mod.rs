//! SoloSoul 插件系统核心
//!
//! 提供插件安装、更新、卸载、沙箱执行、审计与授权管理。

pub mod audit;
pub mod consent;
pub mod error;
pub mod event;
pub mod field;
pub mod host;
pub mod manager;
pub mod manifest;
pub mod paths;
pub mod rate_limiter;
pub mod registry;
pub mod sandbox;
pub mod session;
pub mod store;

pub use audit::PluginAuditLogger;
pub use consent::ConsentManager;
pub use error::PluginError;
pub use event::PluginEvent;
pub use event::PluginEventSink;
pub use field::FieldResolver;
pub use host::{register_host_functions, SoloHostFunctions, SoloHostState};
pub use manager::PluginManager;
pub use manifest::{
    MarketPluginInfo, PluginAuditAction, PluginAuditEntry, PluginInstallResult, PluginLogLine,
    PluginManifest, PluginNetworkPolicy, PluginResult, PluginResultPayload, PluginTier,
    RegistryEntry, RegistryVersion,
};
pub use rate_limiter::{RateLimiter, RateLimiterMap};
pub use registry::PluginRegistry;
pub use sandbox::WasmSandbox;
pub use session::{PluginSession, PluginSessionInfo, PluginSessionManager};
pub use store::{compute_sha256, PluginStore};
