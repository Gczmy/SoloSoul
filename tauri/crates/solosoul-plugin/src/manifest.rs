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

/// 插件契约中的一个输入角色（语义槽位）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PluginContractRole {
    /// 角色标识，如 "street"、"city"、"country"。
    pub role_id: String,
    /// 用户可见的标签，如 "街道 / Street"。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// 该角色是否为插件运行所必需。
    #[serde(default)]
    pub required: bool,
    /// 向后兼容：该角色默认对应的字段 ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_property_id: Option<String>,
}

/// 插件绑定的契约类型。空 vec 表示走 legacy `parse_type_property` 路径。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PluginContractBinding {
    /// UserTemplate.contract_type_id 锚点
    pub type_id: String,
    #[serde(default = "default_contract_version")]
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// 是否启用 contract_field strict-gate（默认 false，兼容 legacy 路径）。
    /// 严格模式下，若请求的属性未标记 contract_field，则返回 InvalidField。
    #[serde(default)]
    pub strict_contract_gate: bool,
    /// 别名表：允许以短名（如 "address"）作为 typed 入口
    #[serde(default)]
    pub type_id_aliases: Vec<String>,
    /// 该契约定义的角色列表。
    #[serde(default)]
    pub roles: Vec<PluginContractRole>,
}

impl PluginContractBinding {
    /// 若 manifest 没有声明 roles，从 field_bindings 推导 legacy roles。
    pub fn effective_roles(
        &self,
        manifest_field_bindings: &[PluginFieldBinding],
    ) -> Vec<PluginContractRole> {
        if !self.roles.is_empty() {
            return self.roles.clone();
        }
        manifest_field_bindings
            .iter()
            .filter(|fb| fb.contract_type_id == self.type_id)
            .map(|fb| PluginContractRole {
                role_id: fb.property_id.clone(),
                label: fb.abi_name.clone(),
                required: false,
                default_property_id: Some(fb.property_id.clone()),
            })
            .collect()
    }
}

fn default_contract_version() -> u32 {
    1
}

/// 插件显式需要的 contract 字段。typed-lookup 时 host gate 入口。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PluginFieldBinding {
    pub contract_type_id: String,
    pub property_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abi_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sensitivity: Option<String>,
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
    /// Stage 4 typed-lookup 契约绑定（#[serde(default)] 兼容旧 manifest）
    #[serde(default)]
    pub contracts: Vec<PluginContractBinding>,
    /// Stage 4 typed-lookup 字段绑定
    #[serde(default)]
    pub field_bindings: Vec<PluginFieldBinding>,
    /// 插件国际化名称与描述。key 为 locale（如 "zh-CN" / "en-US"）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub i18n: Option<HashMap<String, HashMap<String, String>>>,
    /// 自定义 UI 标识。声明后前端将使用内置 React 组件渲染该插件的运行界面。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_ui: Option<String>,
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
    /// Stage 4 typed-lookup 契约绑定（#[serde(default)] 兼容旧 registry）
    #[serde(default)]
    pub contracts: Vec<PluginContractBinding>,
    /// Stage 4 typed-lookup 字段绑定
    #[serde(default)]
    pub field_bindings: Vec<PluginFieldBinding>,
    /// 自定义 UI 标识
    #[serde(default, alias = "custom_ui", skip_serializing_if = "Option::is_none")]
    pub custom_ui: Option<String>,
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
    fn test_deserialize_legacy_manifest_without_contracts() {
        let json = r#"{"id":"test","name":"Test","version":"1.0.0","description":"desc","permissions":[],"data_ttl_seconds":300,"network_policy":{"block_all_outbound":true,"allowed_domains":[]},"require_user_confirmation":true,"tier":"p3","category":"test","params":[]}"#;
        let m: PluginManifest = serde_json::from_str(json).unwrap();
        assert!(m.contracts.is_empty());
        assert!(m.field_bindings.is_empty());
    }

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
