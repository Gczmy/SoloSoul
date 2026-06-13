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
}

fn default_ttl() -> u64 {
    300
}

/// 市场注册表中单个版本的元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryVersion {
    pub sha256: String,
    #[serde(default)]
    pub plugin_api_version: Option<String>,
    pub min_app_version: String,
    pub max_app_version: String,
    #[serde(default)]
    pub download_url: Option<String>,
    #[serde(default)]
    pub raw_url: Option<String>,
    #[serde(default)]
    pub released_at: Option<String>,
    #[serde(default)]
    pub changelog: Option<String>,
}

/// 市场注册表中单个插件条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub name: String,
    #[serde(default)]
    pub publisher: Option<String>,
    #[serde(default)]
    pub latest_version: Option<String>,
    pub versions: HashMap<String, RegistryVersion>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub i18n: Option<HashMap<String, HashMap<String, String>>>,
}

/// 返回给前端的市场插件信息（JSON 使用 camelCase）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketPluginInfo {
    pub plugin_id: String,
    pub installed_version: Option<String>,
    pub has_update: bool,
    pub is_compatible: bool,
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
