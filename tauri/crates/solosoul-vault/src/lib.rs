#![cfg_attr(not(test), allow(dead_code))]
#![cfg_attr(not(test), allow(unused_imports))]
//! SoloSoul Vault 存储库
//!
//! 提供：
//! - SQLite 存储 (profiles / metadata / audit_log)
//! - Vault 生命周期管理（open / lock）
//! - 原子文件写入（safe_storage）

pub mod migration;
pub mod profile;
pub mod safe_storage;
pub mod storage;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Vault configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    pub path: PathBuf,
    pub account_id: String,
    pub sqlcipher_key: Option<Vec<u8>>,
}

impl VaultConfig {
    pub fn new(account_id: &str, path: PathBuf) -> Self {
        Self {
            account_id: account_id.to_string(),
            path,
            sqlcipher_key: None,
        }
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
    pub name: String,
    pub icon_id: Option<String>,
    pub deleted_at: i64,
    pub expires_at: Option<i64>,
    pub original_parent_name: Option<String>,
    pub original_section_type: Option<String>,
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

pub use storage::VaultStore;

// =============================================================================
// Object storage layer — unified object model (P0-1)
// =============================================================================

/// A single unified object stored in the objects table.
/// This is the canonical representation of a user-visible "thing" in SoloSoul.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    /// First few property key-value pairs for card previews
    pub properties: serde_json::Value,
    pub tags: Vec<String>,
}

impl ObjectSummary {
    pub fn from_record(r: &ObjectRecord) -> Self {
        Self {
            id: r.id.clone(),
            name: r.name.clone(),
            collection_type: r.type_id.clone(),
            section_type: r.section_type.clone(),
            sensitivity_level: r.sensitivity_level.clone(),
            created_at: r.created_at.clone(),
            updated_at: r.updated_at.clone(),
            is_deleted: r.is_deleted,
            template_id: r.template_id.clone(),
            template_type: r.template_type.clone(),
            properties: r.properties.clone(),
            tags: r.tags_json.clone(),
        }
    }
}

// =============================================================================
// Template system types (P1: 模板系统重构)
// =============================================================================

/// Property type for template fields.
/// Serialized as snake_case strings for forward compatibility.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PropertyType {
    Text,
    MultilineText,
    Number,
    Date,
    DateTime,
    Boolean,
    Select,
    MultiSelect,
    Url,
    Email,
    Phone,
    FileReference,
}

impl PropertyType {
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
            serde_json::Value::Array(arr) if arr.len() > 1 => PropertyType::MultiSelect,
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

/// A single property definition within a user template.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
}

impl TemplateProperty {
    /// Return the effective sensitivity level, migrating legacy `sensitive` boolean.
    pub fn effective_sensitivity_level(&self) -> Option<String> {
        self.sensitivity_level.clone().or_else(|| {
            self.sensitive.map(|s| {
                if s {
                    "sensitive".to_string()
                } else {
                    "internal".to_string()
                }
            })
        })
    }
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
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}
