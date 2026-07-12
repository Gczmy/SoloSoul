//! 插件管理器移动端占位实现
//!
//! 移动端暂不支持 Wasmtime 插件运行时，所有操作返回空结果或“不支持”错误，
//! 但保持与桌面端完全一致的公共 API。

use super::{
    MarketPluginInfo, PluginAuditEntry, PluginError, PluginEvent, PluginInstallResult,
    PluginManifest, PluginResult, PluginSession, PluginTier,
};
use solosoul_vault::VaultStore;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::ipc::Channel;

/// 插件管理器（移动端占位）。
pub struct PluginManager;

impl PluginManager {
    /// 创建插件管理器（开发模式，无 app_handle）
    pub fn new() -> Result<Self, PluginError> {
        Ok(Self)
    }

    /// 创建插件管理器（Release 模式，使用 Tauri 应用句柄）
    pub fn new_with_app_handle(_app_handle: &tauri::AppHandle) -> Result<Self, PluginError> {
        tracing::warn!("PluginManager is not supported on mobile; running in no-op mode");
        Ok(Self)
    }

    /// 列出市场中所有插件，可按 tier 过滤
    pub fn list_all(
        &self,
        _tier_filter: Option<PluginTier>,
    ) -> Result<Vec<MarketPluginInfo>, PluginError> {
        Ok(vec![])
    }

    /// 列出已安装插件
    pub fn list_installed(&self) -> Result<Vec<PluginManifest>, PluginError> {
        Ok(vec![])
    }

    /// 从市场注册表安装指定版本插件
    pub async fn install_from_registry(
        &self,
        _plugin_id: &str,
        _version: &str,
    ) -> Result<PluginInstallResult, PluginError> {
        Err(PluginError::ExecutionFailed(
            "Plugin installation is not supported on mobile".to_string(),
        ))
    }

    /// 更新插件到注册表最新版本
    pub async fn update(&self, _plugin_id: &str) -> Result<PluginInstallResult, PluginError> {
        Err(PluginError::ExecutionFailed(
            "Plugin update is not supported on mobile".to_string(),
        ))
    }

    /// 卸载插件
    pub fn uninstall(&self, _plugin_id: &str) -> Result<(), PluginError> {
        Err(PluginError::ExecutionFailed(
            "Plugin uninstall is not supported on mobile".to_string(),
        ))
    }

    /// 运行插件
    #[allow(clippy::too_many_arguments)]
    pub async fn run(
        &self,
        _plugin_id: &str,
        _params: HashMap<String, String>,
        _channel: Channel<PluginEvent>,
        _vault_store: Option<Arc<VaultStore>>,
        _account_id: Option<String>,
    ) -> Result<PluginResult, PluginError> {
        Err(PluginError::ExecutionFailed(
            "Plugin execution is not supported on mobile".to_string(),
        ))
    }

    /// 响应授权请求
    pub async fn consent_response(
        &self,
        _request_id: &str,
        _approved: bool,
        _value: Option<String>,
    ) -> Result<(), PluginError> {
        Err(PluginError::ExecutionFailed(
            "Plugin consent is not supported on mobile".to_string(),
        ))
    }

    /// 响应对话框请求
    pub async fn dialog_response(
        &self,
        _request_id: &str,
        _value: Option<String>,
    ) -> Result<(), PluginError> {
        Err(PluginError::ExecutionFailed(
            "Plugin dialog is not supported on mobile".to_string(),
        ))
    }

    /// 列出活跃会话
    pub fn list_sessions(&self) -> Result<Vec<PluginSession>, PluginError> {
        Ok(vec![])
    }

    /// 获取审计日志
    pub fn audit_log(&self, _limit: Option<usize>) -> Result<Vec<PluginAuditEntry>, PluginError> {
        Ok(vec![])
    }

    /// 刷新注册表（从远程拉取并验证签名）
    pub async fn update_registry(&self) -> Result<(), PluginError> {
        Err(PluginError::ExecutionFailed(
            "Plugin registry update is not supported on mobile".to_string(),
        ))
    }
}
