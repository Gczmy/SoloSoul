//! 插件 Manifest 与注册表类型
//!
//! 本模块定义本地 manifest、市场注册表条目以及返回给前端的 `MarketPluginInfo`。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 单个插件日志行
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginLogLine {
    pub id: String,
    pub level: String,
    pub message: String,
    pub timestamp: i64,
}

/// 插件提交的结构化结果（直接透传 SDK 的 JSON 对象）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PluginResultPayload(pub serde_json::Value);

/// 插件运行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginResult {
    pub exit_code: i32,
    pub logs: Vec<PluginLogLine>,
    pub results: Vec<PluginResultPayload>,
    pub fuel_consumed: u64,
}

/// 插件安装结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInstallResult {
    pub plugin_id: String,
    pub version: String,
    pub installed_at: i64,
}

/// 插件网络策略
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PluginNetworkPolicy {
    #[serde(default = "default_block_all")]
    pub block_all_outbound: bool,
    #[serde(default)]
    pub allowed_domains: Vec<String>,
}

fn default_block_all() -> bool {
    true
}

/// 插件参数类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PluginParamType {
    #[default]
    String,
    Number,
    Boolean,
    Select,
}

/// 插件参数选项（用于 select 类型）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginParamOption {
    pub value: String,
    pub label: String,
}

/// 插件运行参数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginParam {
    pub id: String,
    pub label: String,
    #[serde(rename = "type")]
    pub param_type: PluginParamType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub default_value: Option<String>,
    #[serde(default)]
    pub options: Vec<PluginParamOption>,
}

/// 插件分批启用层级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PluginTier {
    P0,
    P1,
    P2,
    #[default]
    P3,
    P4,
}

impl PluginTier {
    /// 从字符串解析（不区分大小写）
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "p0" => Some(PluginTier::P0),
            "p1" => Some(PluginTier::P1),
            "p2" => Some(PluginTier::P2),
            "p3" => Some(PluginTier::P3),
            "p4" => Some(PluginTier::P4),
            _ => None,
        }
    }

    /// 显示名称
    pub fn label(&self) -> &'static str {
        match self {
            PluginTier::P0 => "P0",
            PluginTier::P1 => "P1",
            PluginTier::P2 => "P2",
            PluginTier::P3 => "P3",
            PluginTier::P4 => "P4",
        }
    }
}

/// 本地已安装插件 manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub required_core_version: Option<String>,
    #[serde(default)]
    pub wasm_hash_sha256: Option<String>,
    #[serde(default = "default_ttl")]
    pub data_ttl_seconds: u64,
    #[serde(default)]
    pub network_policy: PluginNetworkPolicy,
    #[serde(default)]
    pub require_user_confirmation: bool,
    #[serde(default)]
    pub tier: PluginTier,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub params: Vec<PluginParam>,
}

fn default_ttl() -> u64 {
    300
}

/// 市场注册表中单个版本的元数据
///
/// 序列化到前端使用 camelCase；反序列化本地 registry.json（snake_case）通过 alias 兼容。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryVersion {
    pub sha256: String,
    #[serde(default, alias = "plugin_api_version")]
    pub plugin_api_version: Option<String>,
    #[serde(alias = "min_app_version")]
    pub min_app_version: String,
    #[serde(alias = "max_app_version")]
    pub max_app_version: String,
    #[serde(default, alias = "download_url")]
    pub download_url: Option<String>,
    #[serde(default, alias = "raw_url")]
    pub raw_url: Option<String>,
    #[serde(default, alias = "released_at")]
    pub released_at: Option<String>,
    #[serde(default)]
    pub changelog: Option<String>,
}

/// 市场注册表中单个插件条目
///
/// 序列化到前端使用 camelCase；反序列化本地 registry.json（snake_case）通过 alias 兼容。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryEntry {
    pub name: String,
    #[serde(default, rename = "author", alias = "publisher")]
    pub publisher: Option<String>,
    #[serde(default, alias = "latest_version")]
    pub latest_version: Option<String>,
    pub versions: HashMap<String, RegistryVersion>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub i18n: Option<HashMap<String, HashMap<String, String>>>,
    #[serde(default)]
    pub tier: PluginTier,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub params: Vec<PluginParam>,
}

/// 返回给前端的市场插件信息（JSON 使用 camelCase）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketPluginInfo {
    pub plugin_id: String,
    pub installed_version: Option<String>,
    pub has_update: bool,
    pub is_compatible: bool,
    pub tier: PluginTier,
    pub category: String,
    pub registry_entry: RegistryEntry,
}

/// 审计动作类型（与前端 PluginAuditAction 结构保持一致）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum PluginAuditAction {
    PluginInstalled { version: String },
    PluginUninstalled,
    PluginRunStarted,
    PluginRunCompleted { exit_code: i32 },
    PluginRunFailed { reason: String },
    ConsentApproved { field_id: String },
    ConsentDenied { field_id: String },
}

/// 单条审计记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginAuditEntry {
    pub timestamp: String,
    pub plugin_id: String,
    pub session_id: Option<String>,
    pub action: PluginAuditAction,
}

/// 域名匹配辅助函数
///
/// `pattern` 支持 `*` 通配符前缀，例如 `*.example.com`。
pub fn matches_domain(domain: &str, pattern: &str) -> bool {
    let domain = domain.trim().to_lowercase();
    let pattern = pattern.trim().to_lowercase();
    if pattern == "*" || pattern == domain {
        return true;
    }
    if let Some(suffix) = pattern.strip_prefix("*.") {
        if let Some(rest) = domain.strip_suffix(suffix) {
            return !rest.is_empty() && rest.ends_with('.');
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_domain_exact() {
        assert!(matches_domain("example.com", "example.com"));
        assert!(!matches_domain("example.com", "other.com"));
    }

    #[test]
    fn test_matches_domain_wildcard() {
        assert!(matches_domain("api.example.com", "*.example.com"));
        assert!(matches_domain("sub.api.example.com", "*.example.com"));
        assert!(!matches_domain("example.com", "*.example.com"));
        assert!(!matches_domain("api.other.com", "*.example.com"));
    }
}
