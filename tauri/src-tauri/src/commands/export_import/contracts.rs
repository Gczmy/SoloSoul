//! 备份/传输 wire DTO 的单一来源；生成器只读，不执行默认值函数。
use serde::{Deserialize, Serialize};
use solosoul_vault::ObjectSummary;
use std::collections::HashMap;

// ── Public types (↔ frontend) ──────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageGroup {
    pub section_type: String,
    pub page_name: String,
    pub object_count: usize,
    pub objects: Vec<ObjectSummary>,
}

/// Scope selection transmitted from frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportScope {
    pub selected_page_ids: Vec<String>, // section_types to export fully
    pub selected_object_ids: Vec<String>, // specific object IDs
    pub selected_tags: Vec<String>,     // P1: tag filter (intersection with selectedObjectIds)
    pub include_attachments: bool,      // P1: include attachment files
    pub selected_attachment_ids: Vec<String>, // P1: fine-grained attachment selection (empty = none)
    pub include_preferences: bool,            // P2: include user preferences
    pub include_behavioral: bool,             // future: include behavioral data
    /// 导出全部对象（用于恢复主机等后端流程，前端普通导出保持 false）。
    #[serde(default)]
    pub include_all: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    pub scope: ExportScope,
    pub password: String,
    pub password_hint: Option<String>,
    pub save_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportEstimate {
    pub object_count: usize,
    pub attachment_count: usize,
    pub attachment_selected_count: usize,
    pub estimated_bytes: u64,
    /// 随本次导出一并打包的用户模板（快照）数量与名称，
    /// 让导出者在执行前明确知道哪些模板会被导出。
    pub template_count: usize,
    pub template_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub file_path: String,
    pub version: String,
    pub object_count: usize,
    pub has_attachments: bool,
    pub extra_files: Vec<String>,
    pub export_time: Option<String>,
    pub password_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentImportInfo {
    pub id: String,
    pub object_id: String,
    pub file_name: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecryptedImportPreview {
    pub objects: Vec<ObjectSummary>,
    pub conflicts: Vec<ConflictInfo>,
    pub has_preferences: bool,
    pub has_audit_log: bool,
    pub attachments: Vec<AttachmentImportInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConflictKind {
    /// ID 相同、名称相同
    Identical,
    /// ID 相同、名称不同（无法判断是本地改名还是导入包名称被修改）
    RenamedLocal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictInfo {
    pub object_id: String,
    pub imported_name: String,
    pub existing_name: String,
    pub kind: ConflictKind,
}

/// P2: import strategy for conflict resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImportStrategy {
    /// Skip conflicting objects (keep existing)
    SkipExisting,
    /// Overwrite all (imported data replaces existing)
    Overwrite,

    /// Keep both: import object gets new UUID, name suffixed with （导入）
    KeepBoth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSelection {
    pub object_id: String,
    pub selected: bool,
}
/// 默认 locale，当前端未传时兜底使用英文。
pub(crate) fn default_locale() -> String {
    "en-US".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdvancedImportRequest {
    /// Fresh 的幂等标识；旧客户端省略时由 Native 生成。
    #[serde(default)]
    pub operation_id: Option<String>,
    /// None = 全量；Some([]) = 不选对象。普通界面继续发送显式选择数组。
    pub selections: Option<Vec<ImportSelection>>,
    pub strategy: ImportStrategy,
    pub source_path: String,
    pub password: String,
    /// 选中的附件 ID（旧 ID，来自导出包）。None = 导入所有附件，Some([]) = 不导入附件。
    pub selected_attachment_ids: Option<Vec<String>>,
    /// 单对象策略覆盖（object_id → ImportStrategy）
    #[serde(default)]
    pub object_strategies: HashMap<String, ImportStrategy>,
    /// 当前界面语言（如 "en-US"、"zh-CN"），用于生成副本名称后缀
    #[serde(default = "default_locale")]
    pub locale: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImportStatus {
    #[default]
    Complete,
    Partial,
    NotCommitted,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImportStage {
    #[default]
    Preparation,
    Templates,
    Objects,
    Snapshots,
    Attachments,
    Preferences,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    /// 同一导入恢复固定使用此 ID；旧的一次性内部 API 不产生 journal。
    pub operation_id: Option<String>,
    pub session_generation: u64,
    pub object_count: usize,
    pub attachment_count: usize,
    pub status: ImportStatus,
    pub template_count: usize,
    pub snapshot_count: usize,
    pub preferences_imported: bool,
    /// 文件已写入但未必关联成功；不计入 attachment_count。
    pub attachment_files_written: usize,
    pub failure_stage: Option<ImportStage>,
    /// 固定错误码，不携带包内容、路径或数据库错误文本。
    pub error_code: Option<String>,
}

#[cfg(test)]
mod tests;
