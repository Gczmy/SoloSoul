//! 插件 Host Functions 移动端占位实现。
//!
//! 移动端暂不支持 WASM 插件运行时，因此本模块仅提供与桌面端相同的公共类型签名，
//! 所有实际功能均为空实现，确保 crate 在移动平台可编译。

use crate::{
    ConsentManager, FieldResolver, PluginAuditLogger, PluginError, PluginManifest,
    PluginResultPayload, RateLimiter,
};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

/// 占位 Linker 类型，保持 `register_host_functions` 签名一致。
pub struct Linker<T>(PhantomData<T>);

impl<T> Linker<T> {
    /// 创建空 Linker。
    pub fn new(_engine: &()) -> Self {
        Self(PhantomData)
    }
}

/// 传递给 Wasm Store 的状态占位。
pub struct SoloHostState {
    pub host: SoloHostFunctions,
}

/// 自定义 Host Functions 数据占位。
#[allow(clippy::module_name_repetitions)]
pub struct SoloHostFunctions {
    pub plugin_id: String,
    pub plugin_name: String,
    pub session_id: String,
    pub manifest: PluginManifest,
    pub params: HashMap<String, String>,
    pub logs: Mutex<Vec<crate::manifest::PluginLogLine>>,
    pub results: Mutex<Vec<PluginResultPayload>>,
    pub audit: Arc<PluginAuditLogger>,
    pub rate_limiter: Arc<RateLimiter>,
    pub consent_manager: Arc<ConsentManager>,
    pub field_resolver: Arc<FieldResolver>,
    pub workspace_dir: Option<std::path::PathBuf>,
}

impl SoloHostFunctions {
    /// 创建 Host Functions 数据（占位）。
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        plugin_id: impl Into<String>,
        plugin_name: impl Into<String>,
        session_id: impl Into<String>,
        manifest: PluginManifest,
        params: HashMap<String, String>,
        audit: Arc<PluginAuditLogger>,
        rate_limiter: Arc<RateLimiter>,
        consent_manager: Arc<ConsentManager>,
        field_resolver: Arc<FieldResolver>,
        _channel: std::sync::Arc<dyn crate::event::PluginEventSink>,
    ) -> Self {
        Self::new_with_workspace(
            plugin_id,
            plugin_name,
            session_id,
            manifest,
            params,
            audit,
            rate_limiter,
            consent_manager,
            field_resolver,
            _channel,
            None,
        )
    }

    /// 创建 Host Functions 数据并指定临时工作区目录（占位）。
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_workspace(
        plugin_id: impl Into<String>,
        plugin_name: impl Into<String>,
        session_id: impl Into<String>,
        manifest: PluginManifest,
        params: HashMap<String, String>,
        audit: Arc<PluginAuditLogger>,
        rate_limiter: Arc<RateLimiter>,
        consent_manager: Arc<ConsentManager>,
        field_resolver: Arc<FieldResolver>,
        _channel: std::sync::Arc<dyn crate::event::PluginEventSink>,
        workspace_dir: Option<std::path::PathBuf>,
    ) -> Self {
        Self {
            plugin_id: plugin_id.into(),
            plugin_name: plugin_name.into(),
            session_id: session_id.into(),
            manifest,
            params,
            logs: Mutex::new(Vec::new()),
            results: Mutex::new(Vec::new()),
            audit,
            rate_limiter,
            consent_manager,
            field_resolver,
            workspace_dir,
        }
    }

    /// 取出运行期间收集的日志（占位，始终返回空）。
    pub fn take_logs(&self) -> Vec<crate::manifest::PluginLogLine> {
        Vec::new()
    }

    /// 取出运行期间收集的结构化结果（占位，始终返回空）。
    pub fn take_results(&self) -> Vec<PluginResultPayload> {
        Vec::new()
    }
}

/// 注册所有 Host Functions 到 linker（占位，无操作）。
pub fn register_host_functions(_linker: &mut Linker<SoloHostState>) -> Result<(), PluginError> {
    Err(PluginError::ExecutionFailed(
        "移动端暂不支持插件运行时".to_string(),
    ))
}
