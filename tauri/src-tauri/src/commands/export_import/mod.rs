//! Export/Import commands — P0+P1+P2: Object-level import/export with password-derived encryption
//!
//! Architecture notes (see docs §14 / §17):
//! - Export scope: page (section_type) → object. No field-level.
//! - Payload: single payload.enc encrypted with AES-256-GCM via Argon2id-derived key.
//! - Salt stored in manifest.json (hex), hint stored plaintext.
//! - P2 extras: tag filtering, preferences export, attachment export, import strategy selection.

use crate::commands::vault_handle;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
pub(crate) use solosoul_core::export_import::AttachmentExportScope;
use solosoul_vault::ObjectSummary;
#[cfg(test)]
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::fs::File;
#[cfg(test)]
use std::io::Read;
#[cfg(test)]
use std::io::Write;
#[cfg(mobile)]
use tauri::Manager;
use tauri::State;
#[cfg(test)]
use uuid::Uuid;
use zeroize::Zeroizing;
use zip::write::SimpleFileOptions;
#[cfg(test)]
use zip::ZipArchive;
use zip::ZipWriter;

// Prefixes used by the frontend to map backend errors to i18n keys.
pub(crate) use crate::services::encrypted_export::EXPORT_ERR_PREFIX;
pub(crate) const IMPORT_ERR_PREFIX: &str = "__IMPORT_ERR__:";

pub(crate) fn export_err(code: &str) -> String {
    format!("{}{}", EXPORT_ERR_PREFIX, code)
}

pub(crate) fn import_err(code: &str) -> String {
    format!("{}{}", IMPORT_ERR_PREFIX, code)
}

pub(crate) fn export_err_with_detail(code: &str, detail: &str) -> String {
    format!("{}{}:{}", EXPORT_ERR_PREFIX, code, detail)
}

pub(crate) fn import_err_with_detail(code: &str, detail: &str) -> String {
    format!("{}{}:{}", IMPORT_ERR_PREFIX, code, detail)
}

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

impl ExportScope {
    pub(crate) fn to_core_scope(
        &self,
        attachments: AttachmentExportScope,
    ) -> solosoul_core::export_import::export::EncryptedExportScope {
        solosoul_core::export_import::export::EncryptedExportScope {
            selected_page_ids: self.selected_page_ids.clone(),
            selected_object_ids: self.selected_object_ids.clone(),
            selected_tags: self.selected_tags.clone(),
            include_all: self.include_all,
            attachments,
            include_preferences: self.include_preferences,
            include_behavioral: self.include_behavioral,
        }
    }

    /// 只适配既有手动 IPC 字段；启用附件但空 ID 数组仍明确表示零选中。
    pub(crate) fn attachment_export_scope(&self) -> AttachmentExportScope {
        AttachmentExportScope::from_manual_selection(
            self.include_attachments,
            &self.selected_attachment_ids,
        )
    }
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

impl ImportResult {
    pub(crate) fn is_complete(&self) -> bool {
        self.status == ImportStatus::Complete
    }

    #[cfg(test)]
    pub(crate) fn require_complete(self) -> Result<Self, String> {
        if self.is_complete() {
            Ok(self)
        } else {
            Err(format!(
                "Import incomplete: stage={:?}, objects={}, attachments={}",
                self.failure_stage, self.object_count, self.attachment_count
            ))
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────

#[cfg(test)]
pub(crate) fn derive_export_key(
    password: &str,
    salt: &[u8],
) -> Result<Zeroizing<[u8; 32]>, String> {
    use solosoul_crypto::kdf::KdfConfig;
    derive_export_key_cfg(password, salt, &KdfConfig::from_env())
}

/// 以指定 KDF 参数派生导出密钥（P202：导入端按 manifest 声明的 `kdf` 字段调用；
/// 导出端默认走 `from_env()`——release 为 production/OWASP，debug 为 development）。
/// P024: 薄包装 `solosoul-crypto::kdf::derive_export_key` 单一实现，仅映射错误类型。
/// P018: 返回 `Zeroizing<[u8;32]>`，导出密钥不再以裸数组残留在内存。
#[cfg(test)]
pub(crate) fn derive_export_key_cfg(
    password: &str,
    salt: &[u8],
    config: &solosoul_crypto::kdf::KdfConfig,
) -> Result<Zeroizing<[u8; 32]>, String> {
    solosoul_crypto::kdf::derive_export_key(password, salt, config).map_err(|e| e.to_string())
}

/// Load attachment metadata from object properties.
pub(crate) fn load_attachments(
    props: &serde_json::Value,
) -> Vec<super::attachment::AttachmentMeta> {
    props
        .get("__attachments")
        .and_then(|v| {
            serde_json::from_value::<Vec<super::attachment::AttachmentMeta>>(v.clone()).ok()
        })
        .unwrap_or_default()
}

/// 仅保留 RF-014 原枚举回归；生产全量后台导出显式使用 AttachmentExportScope::All。
/// 调用方提供原会话的 Vault，不在枚举过程中重新读取当前账户。
#[cfg(test)]
pub(crate) fn collect_all_attachment_ids(
    vault: &solosoul_vault::VaultStore,
    account_id: &str,
) -> Result<Vec<String>, String> {
    let objects = vault.list_objects(account_id, None, None, None, false, false)?;
    let mut ids = Vec::new();
    for obj in objects {
        for att in load_attachments(&obj.properties) {
            if att.deleted_at.is_none() {
                ids.push(att.id);
            }
        }
    }
    Ok(ids)
}

/// Collect all objects matching the given scope.
///
/// P005: `list_objects` 实际逐行解密完整 properties（非轻量摘要）。旧实现先 `list_objects`
/// 全量解密取 id，再对每个 id 单独 `load_object` 二次解密（双重解密 + N+1 查询）。
/// 现改为：include_all 分支直接 `list_object_records`（一次解密完整记录）按页面/标签过滤；
/// selected 分支用 `list_object_metadata_with_tags`（纯 SQL 元数据，不解密 properties）
/// 筛出命中 id，再一次 `load_objects_batch` 批量解密加载（N010-⑥：注释与 P003 实现对齐）。
#[cfg(test)]
pub(crate) fn collect_scope_objects(
    vault: &solosoul_vault::VaultStore,
    account_id: &str,
    scope: &ExportScope,
) -> Result<Vec<solosoul_vault::ObjectRecord>, String> {
    solosoul_core::export_import::export::collect_advanced_objects(
        vault,
        account_id,
        &scope.to_core_scope(scope.attachment_export_scope()),
    )
}

/// Collect the user templates to be packaged in the export.
///
/// - 全量导出（`include_all`，如恢复主机）：打包账户**全部**用户模板（含预置种子模板，
///   以及未被任何对象引用的模板），保证跨设备恢复后模板数量与旧设备一致。
/// - 部分导出：仅打包被导出对象引用的模板（快照隔离，保持既有语义）。
///
/// 体积估算与导出执行共用此逻辑，保证「导出前展示的模板清单」与最终包内 templates 口径一致。
#[cfg(test)]
pub(crate) fn collect_export_templates(
    vault: &solosoul_vault::VaultStore,
    account_id: &str,
    scope: &ExportScope,
    records: &[solosoul_vault::ObjectRecord],
) -> Result<Vec<solosoul_vault::UserTemplate>, String> {
    solosoul_core::export_import::export::collect_advanced_templates(
        vault,
        account_id,
        &scope.to_core_scope(scope.attachment_export_scope()),
        records,
    )
}

// ── Sub-modules ─────────────────────────────────────────────

pub mod export;
pub mod export_docx;
pub mod helpers;
pub mod import;
mod operations;
#[cfg(test)]
pub mod tests;

pub use export::*;
pub use export_docx::*;
pub(crate) use helpers::*;
pub use import::*;
pub use operations::*;
