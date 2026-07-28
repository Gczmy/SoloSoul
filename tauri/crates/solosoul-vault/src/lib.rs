//! SoloSoul Vault 存储库
//!
//! Crate version: see [`VERSION`].
//!
//! 提供：
//! - SQLite 存储 (profiles / metadata / audit_log)
//! - Vault 生命周期管理（open / lock）
//! - 原子文件写入（safe_storage）

/// Crate version (from Cargo.toml at compile time).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod encryption;
pub mod migration;
pub mod profile;
pub mod safe_storage;
pub mod storage;
pub mod template_hash;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Vault configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    pub path: PathBuf,
    pub account_id: String,
    /// 数据加密密钥（Vault 会话密钥）。
    /// 若未设置，VaultStore 拒绝访问任何敏感数据。
    pub data_key: Option<[u8; 32]>,
}

impl VaultConfig {
    pub fn new(account_id: &str, path: PathBuf) -> Self {
        Self {
            account_id: account_id.to_string(),
            path,
            data_key: None,
        }
    }

    pub fn with_data_key(mut self, key: [u8; 32]) -> Self {
        self.data_key = Some(key);
        self
    }
}

/// Vault state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultState {
    Locked,
    Unlocked,
}

/// Vault statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultStats {
    pub profile_count: usize,
    pub total_size_bytes: u64,
    pub last_modified: Option<String>,
    // Breakdown components
    pub profiles_size: u64,
    pub objects_size: u64,
    pub trash_size: u64,
    pub snapshots_size: u64,
    pub attachments_size: u64,
    pub ai_conversations_size: u64,
}

pub use profile::{Profile, ProfileData, ProfileSummary, VersionedProfileData};

// ── Sync helpers ──────────────────────────────────────────

/// HLC timestamp stored in the vault for conflict resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RecordHlc {
    pub wall_time_ms: u64,
    pub counter: u32,
    pub node_id: String,
}

/// Per-table sync watermark for a given peer.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SyncWatermark {
    pub wall_time_ms: u64,
    pub counter: u32,
    pub node_id: String,
}

/// Persistent peer state for device sync.
#[derive(Debug, Clone)]
pub struct PeerSyncState {
    pub peer_node_id: String,
    pub peer_name: Option<String>,
    pub trusted: bool,
    pub public_key_fingerprint: Option<String>,
    pub last_seen: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

/// A single record change produced or consumed by the sync delta engine.
#[derive(Debug, Clone)]
pub struct VaultSyncRecord {
    pub id: String,
    pub table: String,
    pub data: serde_json::Value,
    pub hlc: RecordHlc,
    pub deleted: bool,
}

/// Summary of a persistent sync conflict.
#[derive(Debug, Clone)]
pub struct SyncConflictSummary {
    pub id: String,
    pub table_name: String,
    pub record_id: String,
    pub local_hlc_json: String,
    pub remote_hlc_json: String,
    pub winner: String,
    pub created_at: String,
}

/// Full detail of a persistent sync conflict (includes remote data JSON).
#[derive(Debug, Clone)]
pub struct SyncConflictDetail {
    pub id: String,
    pub table_name: String,
    pub record_id: String,
    pub local_hlc_json: String,
    pub remote_hlc_json: String,
    pub local_data_json: String,
    pub remote_data_json: String,
    pub remote_deleted: bool,
    pub winner: String,
    pub created_at: String,
}

// ── Trash items (§23 回收站功能规范) ────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrashItem {
    pub id: String,
    pub item_type: String,
    pub original_id: String,
    pub original_parent_id: Option<String>,
    pub original_section_type: Option<String>,
    pub original_sort_order: Option<i32>,
    pub data: Vec<u8>,
    pub deleted_at: i64,
    pub expires_at: Option<i64>,
    pub deleted_by: String,
    pub name_snapshot: String,
    pub icon_snapshot: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashItemSummary {
    pub id: String,
    pub item_type: String,
    pub original_id: String,
    pub name: String,
    pub icon_id: Option<String>,
    pub deleted_at: i64,
    pub expires_at: Option<i64>,
    pub original_parent_id: Option<String>,
    pub original_section_type: Option<String>,
    pub contract_type_id: Option<String>,
}
/// Structured audit log entry
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogEntry {
    pub id: i64,
    pub timestamp: String,
    pub action_type: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub entity_name: Option<String>,
    pub performed_by: String,
    pub details: Option<String>,
}

/// A single guide document chunk with its embedding vector for RAG retrieval.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GuideEmbeddingChunk {
    pub id: String,
    pub guide_id: String,
    pub chunk_index: i32,
    pub chunk_text: String,
    pub embedding: Vec<f32>,
    pub model: String,
    pub created_at: String,
}

pub use encryption::DataEncryptionKey;
pub use storage::VaultStore;

// =============================================================================
// Object storage layer — unified object model (P0-1)
// =============================================================================

/// A single unified object stored in the objects table.
/// This is the canonical representation of a user-visible "thing" in SoloSoul.
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct ObjectRecord {
    pub id: String,
    pub account_id: String,
    #[serde(rename = "typeId")]
    pub type_id: String,
    #[serde(rename = "sectionType")]
    pub section_type: String,
    pub name: String,
    #[serde(rename = "iconName")]
    pub icon_name: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    #[serde(rename = "childrenIds")]
    pub children_ids: Vec<String>,
    pub properties: serde_json::Value,
    #[serde(rename = "propertyLabels")]
    pub property_labels: Option<serde_json::Value>,
    #[serde(rename = "sensitivityLevel")]
    pub sensitivity_level: String,
    #[serde(rename = "isDeleted")]
    pub is_deleted: bool,
    #[serde(rename = "deletedAt")]
    pub deleted_at: Option<String>,
    pub tags_json: Vec<String>,
    #[serde(rename = "templateId")]
    pub template_id: Option<String>,
    #[serde(rename = "templateType")]
    pub template_type: Option<String>,
    /// 插件合约类型 ID（用于 plugin-template 兼容）。旧记录缺失时由 `default` 填充为 `None`。
    #[serde(
        rename = "contractTypeId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub contract_type_id: Option<String>,
    #[serde(
        rename = "templateHash",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub template_hash: Option<String>,
    #[serde(
        rename = "ignoredTemplateHash",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub ignored_template_hash: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub version: u32,
}

/// Lightweight summary of an object for listing (no full properties).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ObjectSummary {
    pub id: String,
    pub name: String,
    #[serde(rename = "collectionType")]
    pub collection_type: String,
    #[serde(rename = "sectionType")]
    pub section_type: String,
    #[serde(rename = "sensitivityLevel")]
    pub sensitivity_level: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "isDeleted")]
    pub is_deleted: bool,
    #[serde(rename = "templateId")]
    pub template_id: Option<String>,
    #[serde(rename = "templateType")]
    pub template_type: Option<String>,
    /// 插件合约类型 ID（用于 plugin-template 兼容）。旧记录缺失时由 `default` 填充为 `None`。
    #[serde(
        rename = "contractTypeId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub contract_type_id: Option<String>,
    #[serde(
        rename = "templateHash",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub template_hash: Option<String>,
    #[serde(
        rename = "ignoredTemplateHash",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub ignored_template_hash: Option<String>,
    #[serde(rename = "iconName")]
    pub icon_name: String,
    /// First few property key-value pairs for card previews
    pub properties: serde_json::Value,
    /// Per-field sensitivity overrides: field_name -> sensitivity_level
    #[serde(rename = "propertyLabels", skip_serializing_if = "Option::is_none")]
    pub property_labels: Option<serde_json::Value>,
    pub tags: Vec<String>,
}

// =============================================================================
// Template system types (P1: 模板系统重构)
// =============================================================================

/// Property type for template fields.
/// Serialized as snake_case strings for forward compatibility.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PropertyType {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "multiline")]
    MultilineText,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "date")]
    Date,
    #[serde(rename = "datetime")]
    DateTime,
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "select")]
    Select,
    #[serde(rename = "multiselect")]
    MultiSelect,
    #[serde(rename = "url")]
    Url,
    #[serde(rename = "email")]
    Email,
    #[serde(rename = "phone")]
    Phone,
    #[serde(rename = "file")]
    FileReference,
    /// 动态字段组：模板中仅为容器，对象级可添加任意数量、任意类型的子字段。
    #[serde(rename = "dynamic_group")]
    DynamicGroup,
}

impl PropertyType {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "text" => Some(Self::Text),
            "multiline" => Some(Self::MultilineText),
            "number" => Some(Self::Number),
            "date" => Some(Self::Date),
            "datetime" => Some(Self::DateTime),
            "boolean" => Some(Self::Boolean),
            "select" => Some(Self::Select),
            "multiselect" => Some(Self::MultiSelect),
            "url" => Some(Self::Url),
            "email" => Some(Self::Email),
            "phone" => Some(Self::Phone),
            "file" => Some(Self::FileReference),
            "dynamic_group" => Some(Self::DynamicGroup),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            PropertyType::Text => "text",
            PropertyType::MultilineText => "multiline",
            PropertyType::Number => "number",
            PropertyType::Date => "date",
            PropertyType::DateTime => "datetime",
            PropertyType::Boolean => "boolean",
            PropertyType::Select => "select",
            PropertyType::MultiSelect => "multiselect",
            PropertyType::Url => "url",
            PropertyType::Email => "email",
            PropertyType::Phone => "phone",
            PropertyType::FileReference => "file",
            PropertyType::DynamicGroup => "dynamic_group",
        }
    }

    /// Infer property type from a JSON value and key name heuristic.
    pub fn infer_from_value(value: &serde_json::Value, key: &str) -> Self {
        match value {
            serde_json::Value::Bool(_) => PropertyType::Boolean,
            serde_json::Value::Number(n) => {
                if n.is_i64() || n.is_u64() || n.is_f64() {
                    PropertyType::Number
                } else {
                    PropertyType::Text
                }
            }
            serde_json::Value::Array(arr) => {
                // 动态字段组：数组元素均为含 id/name/type/value 的对象
                if !arr.is_empty()
                    && arr.iter().all(|item| {
                        item.as_object().is_some_and(|o| {
                            o.contains_key("id")
                                && o.contains_key("name")
                                && o.contains_key("type")
                                && o.contains_key("value")
                        })
                    })
                {
                    return PropertyType::DynamicGroup;
                }
                if arr.len() > 1 {
                    PropertyType::MultiSelect
                } else {
                    PropertyType::Text
                }
            }
            serde_json::Value::String(s) => {
                let lower = key.to_lowercase();
                let s_lower = s.to_lowercase();
                // Date heuristics: key contains date-related words
                if lower.contains("date")
                    || lower.contains("birth")
                    || lower.contains("issue")
                    || lower.contains("expir")
                    || lower.contains("deadline")
                {
                    // Try to parse as ISO date
                    if chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok() {
                        return PropertyType::Date;
                    }
                }
                if lower.contains("email") || s.contains('@') {
                    return PropertyType::Email;
                }
                if lower.contains("phone") || lower.contains("tel") || lower.contains("mobile") {
                    return PropertyType::Phone;
                }
                if lower.contains("url") || lower.contains("link") || lower.contains("website") {
                    return PropertyType::Url;
                }
                if s_lower == "true" || s_lower == "false" {
                    return PropertyType::Boolean;
                }
                PropertyType::Text
            }
            _ => PropertyType::Text,
        }
    }
}

/// 模板字段到插件契约角色的绑定。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContractRoleBinding {
    /// 插件契约类型 ID，如 "com.solosoul.official.address-fmt/v1"。
    pub contract_type_id: String,
    /// 契约内的角色 ID，如 "street"。
    pub role_id: String,
}

/// A single property definition within a user template.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateProperty {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub prop_type: PropertyType,
    /// Sensitivity level: "public" | "internal" | "sensitive" | "critical".
    /// Replaces the legacy `sensitive` boolean.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sensitivity_level: Option<String>,
    /// Legacy boolean flag — kept for backward-compat during deserialization only.
    #[serde(default, skip_serializing)]
    pub sensitive: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    /// Deprecated timestamp — if set, the field is soft-deleted but retained for old objects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated_at: Option<String>,
    /// 当此属性映射到插件合约中的字段时为 true（用于 plugin-template 兼容）。
    /// 旧记录缺失时由 `default` 填充为 `None`。
    #[serde(
        rename = "contractField",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub contract_field: Option<bool>,
    /// 新版绑定：一个字段可绑定到多个插件契约的多个角色。
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "contractBindings"
    )]
    pub contract_bindings: Option<Vec<ContractRoleBinding>>,
    /// 动态字段组允许创建的子字段类型；空/缺失表示不限制。
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "allowedTypes"
    )]
    pub allowed_types: Option<Vec<PropertyType>>,
    /// 动态字段组允许的最大子字段数量；缺失表示无限制。
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "maxItems")]
    pub max_items: Option<u32>,
}

/// A user-defined object template stored in the vault.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserTemplate {
    pub id: String,
    pub account_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_id: Option<String>,
    pub properties: Vec<TemplateProperty>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    /// 插件合约类型 ID（用于 plugin-template 兼容）。旧记录缺失时由 `default` 填充为 `None`。
    #[serde(
        rename = "contractTypeId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub contract_type_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn property_type_dynamic_group_roundtrip() {
        assert_eq!(
            PropertyType::parse("dynamic_group"),
            Some(PropertyType::DynamicGroup)
        );
        assert_eq!(PropertyType::DynamicGroup.as_str(), "dynamic_group");
    }

    #[test]
    fn infer_from_value_dynamic_group() {
        let value = serde_json::json!([
            { "id": "1", "name": "手机", "type": "phone", "value": "123" }
        ]);
        assert_eq!(
            PropertyType::infer_from_value(&value, "contactMethods"),
            PropertyType::DynamicGroup
        );
    }
}
