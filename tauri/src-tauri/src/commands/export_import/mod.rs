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
use solosoul_vault::ObjectSummary;
use std::collections::{BTreeSet, HashMap};
use std::fs::File;
use std::io::{Read, Write};
use tauri::State;
use uuid::Uuid;
use zip::write::SimpleFileOptions;
use zip::ZipArchive;
use zip::ZipWriter;

use super::attachment::AttachmentMeta;

pub(crate) fn generate_id() -> String {
    Uuid::new_v4().to_string()
}

// Prefixes used by the frontend to map backend errors to i18n keys.
pub(crate) const EXPORT_ERR_PREFIX: &str = "__EXPORT_ERR__:";
pub(crate) const IMPORT_ERR_PREFIX: &str = "__IMPORT_ERR__:";

/// 导出审计日志时最多读取的条数。
pub(crate) const MAX_AUDIT_LOG_EXPORT: usize = 100_000;
pub(crate) fn export_err(code: &str) -> String {
    format!("{}{}", EXPORT_ERR_PREFIX, code)
}

pub(crate) fn import_err(code: &str) -> String {
    format!("{}{}", IMPORT_ERR_PREFIX, code)
}

/// 校验附件物理路径位于 Vault 的 attachments 目录内，防止导出时通过恶意 src_path 读取任意文件。
pub(crate) fn validate_attachment_path(
    base: &std::path::Path,
    path: &std::path::Path,
) -> Result<(), String> {
    let base_abs = std::path::absolute(base).map_err(|e| e.to_string())?;
    let path_abs = std::path::absolute(path).map_err(|e| e.to_string())?;
    if !path_abs.starts_with(&base_abs) {
        return Err(format!(
            "Attachment path escapes vault attachments directory: {}",
            path.display()
        ));
    }
    Ok(())
}

/// 导出/导入包中的对象 ID 与附件 ID 字符集校验。
pub(crate) fn validate_export_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.len() > 64
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!("Invalid export id: {}", id));
    }
    Ok(())
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
    pub selections: Vec<ImportSelection>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub object_count: usize,
    pub attachment_count: usize,
}

// ── Helpers ────────────────────────────────────────────────────

pub(crate) fn derive_export_key(password: &str, salt: &[u8]) -> Result<[u8; 32], String> {
    use solosoul_crypto::kdf::{derive_key, KdfConfig};
    let key_vec = derive_key(password, salt, &KdfConfig::balanced()).map_err(|e| e.to_string())?;
    let mut key = [0u8; 32];
    key.copy_from_slice(&key_vec);
    Ok(key)
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

/// Collect all objects matching the given scope.
pub(crate) fn collect_scope_objects(
    vault: &solosoul_vault::VaultStore,
    account_id: &str,
    scope: &ExportScope,
) -> Result<Vec<solosoul_vault::ObjectRecord>, String> {
    let all = vault.list_objects(account_id, None, None, None, false, false)?;
    let mut selected_ids: BTreeSet<String> = scope.selected_object_ids.iter().cloned().collect();

    // Add all IDs belonging to selected pages
    for summary in &all {
        if !scope.selected_page_ids.is_empty()
            && scope.selected_page_ids.contains(&summary.section_type)
        {
            selected_ids.insert(summary.id.clone());
        }
    }

    // Filter by tags (P2): if selected_tags is non-empty, keep only objects with ANY matching tag
    if !scope.selected_tags.is_empty() {
        selected_ids.retain(|id| {
            all.iter()
                .any(|s| s.id == *id && s.tags.iter().any(|t| scope.selected_tags.contains(t)))
        });
    }

    let mut records = Vec::new();
    for id in &selected_ids {
        if let Ok(Some(rec)) = vault.load_object(id) {
            records.push(rec);
        }
    }
    Ok(records)
}

// ── Sub-modules ─────────────────────────────────────────────

pub mod export;
pub mod helpers;
pub mod import;
#[cfg(test)]
pub mod tests;

pub use export::*;
pub(crate) use helpers::*;
pub use import::*;
