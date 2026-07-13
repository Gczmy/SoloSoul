//! 插件管理器移动端占位实现。
//!
//! 移动端暂不支持 WASM 插件运行时，因此本模块仅提供与桌面端相同的公共类型签名，
//! 所有实际功能均为空实现或返回错误，确保 crate 在移动平台可编译。

use crate::{
    ConsentManager, FieldResolver, MarketPluginInfo, PluginAuditEntry, PluginAuditLogger,
    PluginError, PluginEvent, PluginEventSink, PluginInstallResult, PluginManifest, PluginRegistry,
    PluginResult, PluginSession, PluginSessionManager, PluginStore, PluginTier, RateLimiter,
    WasmSandbox,
};
use solosoul_vault::VaultStore;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// 插件管理器占位。
pub struct PluginManager {
    store: PluginStore,
    registry: PluginRegistry,
    market_dir: PathBuf,
    session_manager: PluginSessionManager,
    audit: Arc<PluginAuditLogger>,
    rate_limiter: Arc<RateLimiter>,
    consent_manager: Arc<ConsentManager>,
    field_resolver: Arc<FieldResolver>,
    sandbox: WasmSandbox,
}

impl PluginManager {
    /// 创建插件管理器（开发模式，无 app_handle）
    pub fn new() -> Result<Self, PluginError> {
        let market_dir = super::paths::default_market_dir();
        let audit_path = PluginStore::data_dir()?.join("plugin_audit.jsonl");
        Ok(Self {
            store: PluginStore::new()?,
            registry: PluginRegistry::new(),
            market_dir,
            session_manager: PluginSessionManager::new(),
            audit: Arc::new(PluginAuditLogger::new(Some(audit_path))),
            rate_limiter: Arc::new(RateLimiter::new(60)),
            consent_manager: Arc::new(ConsentManager::new()),
            field_resolver: Arc::new(FieldResolver::new()),
            sandbox: WasmSandbox::new(),
        })
    }

    /// 创建插件管理器（Release 模式，使用 Tauri 资源目录）
    pub fn new_with_resource_dir(resource_dir: &std::path::PathBuf) -> Result<Self, PluginError> {
        let market_dir = super::paths::resolve_market_dir(Some(resource_dir))?;
        let audit_path = PluginStore::data_dir()?.join("plugin_audit.jsonl");
        Ok(Self {
            store: PluginStore::new()?,
            registry: PluginRegistry::new_with_resource_dir(resource_dir)?,
            market_dir,
            session_manager: PluginSessionManager::new(),
            audit: Arc::new(PluginAuditLogger::new(Some(audit_path))),
            rate_limiter: Arc::new(RateLimiter::new(60)),
            consent_manager: Arc::new(ConsentManager::new()),
            field_resolver: Arc::new(FieldResolver::new()),
            sandbox: WasmSandbox::new(),
        })
    }

    /// 列出市场中所有插件（占位，返回空列表）。
    pub fn list_all(
        &self,
        _tier_filter: Option<PluginTier>,
    ) -> Result<Vec<MarketPluginInfo>, PluginError> {
        Ok(Vec::new())
    }

    /// 列出已安装插件（占位，返回空列表）。
    pub fn list_installed(&self) -> Result<Vec<PluginManifest>, PluginError> {
        Ok(Vec::new())
    }

    /// 从市场注册表安装指定版本插件（占位，返回错误）。
    pub fn install_from_registry(
        &self,
        _plugin_id: &str,
        _version: &str,
    ) -> Result<PluginInstallResult, PluginError> {
        Err(PluginError::ExecutionFailed(
            "移动端暂不支持插件运行时".to_string(),
        ))
    }

    /// 更新插件到注册表最新版本（占位，返回错误）。
    pub fn update(&self, _plugin_id: &str) -> Result<PluginInstallResult, PluginError> {
        Err(PluginError::ExecutionFailed(
            "移动端暂不支持插件运行时".to_string(),
        ))
    }

    /// 卸载插件（占位，返回错误）。
    pub fn uninstall(&self, _plugin_id: &str) -> Result<(), PluginError> {
        Err(PluginError::ExecutionFailed(
            "移动端暂不支持插件运行时".to_string(),
        ))
    }

    /// 运行插件（占位，返回错误）。
    pub async fn run(
        &self,
        _plugin_id: &str,
        _params: HashMap<String, String>,
        _channel: std::sync::Arc<dyn PluginEventSink>,
        _vault_store: Option<Arc<VaultStore>>,
        _account_id: Option<String>,
    ) -> Result<PluginResult, PluginError> {
        Err(PluginError::ExecutionFailed(
            "移动端暂不支持插件运行时".to_string(),
        ))
    }

    /// 响应授权请求（占位，无操作）。
    pub async fn consent_response(
        &self,
        request_id: &str,
        approved: bool,
        value: Option<String>,
    ) -> Result<(), PluginError> {
        let response_value = if approved { value } else { None };
        self.consent_manager
            .respond(request_id, response_value)
            .await
            .map_err(|_| PluginError::ConsentDenied)?;
        Ok(())
    }

    /// 响应对话框请求（占位，无操作）。
    pub async fn dialog_response(
        &self,
        request_id: &str,
        value: Option<String>,
    ) -> Result<(), PluginError> {
        self.consent_manager
            .respond(request_id, value)
            .await
            .map_err(|e| PluginError::ExecutionFailed(format!("对话框响应失败: {}", e)))?;
        Ok(())
    }

    /// 列出活跃会话（占位，返回空列表）。
    pub fn list_sessions(&self) -> Result<Vec<PluginSession>, PluginError> {
        Ok(Vec::new())
    }

    /// 获取审计日志（占位，返回空列表）。
    pub fn audit_log(&self, _limit: Option<usize>) -> Result<Vec<PluginAuditEntry>, PluginError> {
        Ok(Vec::new())
    }

    /// 刷新注册表（占位，返回错误）。
    pub async fn update_registry(&self) -> Result<(), PluginError> {
        Err(PluginError::ExecutionFailed(
            "移动端暂不支持插件运行时".to_string(),
        ))
    }
}
