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
use chrono::{DateTime, Utc};

// ── Phase 2 云同步配置类型 ──────────────────────────────

/// 云同步配置（存入 Vault 加密字段，仅解锁态可见）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CloudSyncConfig {
    /// 连接器类型：`webdav` / `onedrive` / `baidu` ...
    pub connector_type: String,
    /// 连接器特定配置（JSON 序列化），反序列化为对应 ConnectorConfig 子类型。
    pub config_json: serde_json::Value,
    /// 自动同步开关。
    pub enabled: bool,
    /// 同步频率秒数（≥ 60）。
    pub interval_secs: u64,
    /// 仅 Wi-Fi 同步（移动端）。
    pub wifi_only: bool,
    /// 云端保留策略：最近 N 份 + 每日/周/月各留一份（GFS）。
    pub retention: RetentionPolicy,
    /// 上次同步时间（用于增量判断）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_sync_at: Option<DateTime<Utc>>,
}

/// 保留策略（Grandfather-Father-Son 简化版）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RetentionPolicy {
    /// 最近 N 份全量保留。
    pub recent_full: usize,
    /// 是否保留每日快照（0 点最近一份）。
    pub daily: bool,
    /// 是否保留周快照（周一最近一份）。
    pub weekly: bool,
    /// 是否保留月快照（1 号最近一份）。
    pub monthly: bool,
}

/// WebDAV 专用配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WebDavConfig {
    /// WebDAV 服务器基础 URL（如 `https://dav.jianguoyun.com/dav/`）。
    pub base_url: String,
    /// 用户名（坚果云用邮箱，Nextcloud 用用户名）。
    pub username: String,
    /// 密码 / App Token（入 Vault 前由前端明文传入，后端加密存储）。
    pub password: String,
    /// 云端根路径前缀（默认 `/SoloSoul/`）。
    #[serde(default = "default_root_prefix")]
    pub root_prefix: String,
}

fn default_root_prefix() -> String {
    "/SoloSoul/".to_string()
}

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

// P028: 移除 VersionedProfileData（全 workspace 零调用死代码）
pub use profile::{Profile, ProfileData, ProfileSummary};

// ── Sync helpers ──────────────────────────────────────────

/// HLC timestamp stored in the vault for conflict resolution.
// P019: PartialOrd/Ord 供 cleanup_expired_tombstones 的 min 收敛——
// 字典序 (wall_time_ms, counter, node_id) 与手写三元组比较逐位一致。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RecordHlc {
    pub wall_time_ms: u64,
    pub counter: u32,
    pub node_id: String,
}

/// Per-table sync watermark for a given peer.
/// P019: PartialOrd/Ord 与 RecordHlc 同构（字段顺序一致），供水位 min 收敛。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
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
    /// 客户端类型：macos / windows / linux / android / ios / unknown。
    /// 旧记录/旧客户端握手时为空（未知）。
    pub client_type: Option<String>,
    /// 最近信任时间（unix 秒）。从未信任/已撤销时为 None。
    pub trusted_at: Option<i64>,
    /// 最近一次成功同步/入站会话的对端连接地址（host:port）。
    /// P1#7/#8：在线状态心跳化——成功同步即证明 LAN 可达，即使 mDNS 发现链
    /// 中断，known_peers 也可凭「fresh last_seen + last_addr」显示在线。
    /// 旧记录/旧客户端同步前为 None。
    pub last_addr: Option<String>,
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

/// P115: 同步记录的借用视图——批量应用时避免逐条克隆 JSON `data`。
///
/// `apply_sync_records_batch` 接收该借用视图切片，调用方（sync crate）将
/// 线格式 `SyncRecord` 零克隆地映射为借用视图，仅在需要处持有转换后的 `RecordHlc`。
#[derive(Debug, Clone, Copy)]
pub struct BorrowedSyncRecord<'a> {
    pub id: &'a str,
    pub table: &'a str,
    pub data: &'a serde_json::Value,
    pub hlc: &'a RecordHlc,
    pub deleted: bool,
}

/// P115: 单条同步记录的应用结果（含写前本地 HLC，供调用方冲突报告复用）。
#[derive(Debug, Clone, Default)]
pub struct SyncApplyOutcome {
    /// 是否已应用（false = 因 HLC 不新而跳过）。
    pub applied: bool,
    /// 写前本地 HLC（本地无该记录时为 None）。
    /// 当 `applied == false` 且本地 HLC 严格新于远端时构成冲突。
    pub local_hlc: Option<RecordHlc>,
    /// 单条记录级错误（不中断整批事务）。
    pub error: Option<String>,
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

/// P016: 批量持久化同步冲突的输入条目（`VaultStore::save_sync_conflicts_batch` 使用）。
/// 一次批量调用内所有冲突在单事务中 upsert，避免大量冲突时逐条 commit。
#[derive(Debug, Clone)]
pub struct SyncConflictBatchEntry {
    pub table: String,
    pub record_id: String,
    pub local_hlc: RecordHlc,
    pub remote_hlc: RecordHlc,
    pub local_data: serde_json::Value,
    pub remote_data: serde_json::Value,
    pub remote_deleted: bool,
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
pub use storage::object_field_sensitivity_levels;
/// R-4① 方案 2：只读数据密钥探测（独立只读连接，无 open 副作用）。
pub use storage::object_has_attachments;
pub use storage::probe_data_key;
pub use storage::sensitivity_rank;
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
    // P042: IPC 载荷字段名统一为 typeId（与 ObjectRecord 同步载荷一致，前端一套词汇）
    #[serde(rename = "typeId")]
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
    /// 所属父对象 ID（自定义页面子对象），无父级时为 None（P112 附件树按 parent 分组用）。
    #[serde(rename = "parentId", default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    /// First few property key-value pairs for card previews
    pub properties: serde_json::Value,
    /// Per-field sensitivity overrides: field_name -> sensitivity_level
    #[serde(rename = "propertyLabels", skip_serializing_if = "Option::is_none")]
    pub property_labels: Option<serde_json::Value>,
    pub tags: Vec<String>,
    /// 该对象是否包含（未软删的）附件——供导出范围树等 UI 判断是否展示附件展开图标。
    /// 由 `properties.__attachments` 推导；metadata-only 路径（properties 未解密）为 false。
    /// 注意：本结构体为逐字段显式 rename（无 rename_all），必须显式 camelCase 序列化。
    #[serde(rename = "hasAttachments", default)]
    pub has_attachments: bool,
    /// 对象各字段的敏感度等级集合（去重、按 public < internal < sensitive < critical 排序）。
    /// 反映字段级敏感度分布（区别于 `sensitivity_level` 记录级）；供导出范围树等 UI 展示徽章。
    /// 由 `list_objects` 等解密路径从 property_labels / __fields / 模板定义推导；
    /// metadata-only 路径（properties 未解密）为空数组。
    #[serde(
        rename = "sensitivityLevels",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub sensitivity_levels: Vec<String>,
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
    fn object_summary_serializes_has_attachments_camel_case() {
        // 防回归：导出范围树等 IPC 载荷前端按 camelCase（hasAttachments）读取；
        // ObjectSummary 为逐字段显式 rename（无 rename_all），新增字段必须显式 camelCase。
        let s = ObjectSummary {
            id: "1".to_string(),
            name: "n".to_string(),
            collection_type: "note".to_string(),
            section_type: "identity".to_string(),
            sensitivity_level: "internal".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
            is_deleted: false,
            template_id: None,
            template_type: None,
            contract_type_id: None,
            template_hash: None,
            ignored_template_hash: None,
            icon_name: "document".to_string(),
            parent_id: None,
            properties: serde_json::Value::Null,
            property_labels: None,
            tags: vec![],
            has_attachments: true,
            sensitivity_levels: vec![],
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["hasAttachments"], serde_json::Value::Bool(true));
        // 不得以 snake_case 键序列化（此前正是此缺陷导致前端 hasAttachments 恒为 undefined）
        assert!(v.get("has_attachments").is_none());
        // 既有关键字段命名不受影响
        assert_eq!(v["typeId"], serde_json::Value::String("note".to_string()));
        assert_eq!(
            v["sensitivityLevel"],
            serde_json::Value::String("internal".to_string())
        );
        // 字段敏感度集合同样显式 camelCase（sensitivityLevels），空数组不序列化
        assert!(v.get("sensitivityLevels").is_none());
        assert!(v.get("sensitivity_levels").is_none());
    }

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

