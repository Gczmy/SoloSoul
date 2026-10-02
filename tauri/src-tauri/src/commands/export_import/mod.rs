//! Export/Import commands — P0+P1+P2: Object-level import/export with password-derived encryption
//!
//! Architecture notes (see docs §14 / §17):
//! - Export scope: page (section_type) → object. No field-level.
//! - Payload: single payload.enc encrypted with AES-256-GCM via Argon2id-derived key.
//! - Salt stored in manifest.json (hex), hint stored plaintext.
//! - P2 extras: tag filtering, preferences export, attachment export, import strategy selection.

pub(crate) use solosoul_core::export_import::AttachmentExportScope;
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::fs::File;
#[cfg(test)]
use std::io::Read;
#[cfg(test)]
use std::io::Write;
#[cfg(test)]
use uuid::Uuid;
#[cfg(test)]
use zeroize::Zeroizing;
#[cfg(test)]
use zip::write::SimpleFileOptions;
#[cfg(test)]
use zip::ZipArchive;
#[cfg(test)]
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

// Wire DTO 保留原模块访问路径，业务适配仍在 Host。
pub use contracts::{
    AdvancedImportRequest, AttachmentImportInfo, ConflictInfo, ConflictKind,
    DecryptedImportPreview, ExportEstimate, ExportRequest, ExportScope, ImportPreview,
    ImportResult, ImportSelection, ImportStage, ImportStatus, ImportStrategy, PageGroup,
};

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

pub mod contracts;
pub(crate) mod errors;
pub mod export;
pub mod export_docx;
pub mod helpers;
pub mod import;
pub mod operations;
#[cfg(test)]
pub mod tests;

pub use export::{
    __cmd__export_estimate_size, __cmd__export_execute, __cmd__export_get_attachments_batch,
    __cmd__export_get_scope_tree, __tauri_command_name_export_estimate_size,
    __tauri_command_name_export_execute, __tauri_command_name_export_get_attachments_batch,
    __tauri_command_name_export_get_scope_tree, export_estimate_size, export_execute,
    export_get_attachments_batch, export_get_scope_tree, AttachmentInfo,
};
#[cfg(test)]
pub(crate) use export::{
    execute_export_core, execute_export_core_with_attachment_scope, execute_export_for_session,
    execute_export_for_session_with_attachment_scope,
};
pub use export_docx::{
    __cmd__export_document_preflight, __cmd__export_objects_document,
    __tauri_command_name_export_document_preflight, __tauri_command_name_export_objects_document,
    export_document_preflight, export_objects_document, DocumentSensitivity, ExportDocumentResult,
};
#[cfg(test)]
pub(crate) use helpers::read_manifest_json;
#[cfg(test)]
pub(crate) use helpers::{
    build_package_ids, create_export_output, decrypt_zip_entry_streaming, read_file_from_zip,
    read_manifest, read_manifest_json_limited, resolve_cross_scope_references,
    resolve_value_references, unique_object_name,
};
pub use import::{
    __cmd__import_decrypt_preview, __cmd__import_execute_advanced, __cmd__import_parse_package,
    __tauri_command_name_import_decrypt_preview, __tauri_command_name_import_execute_advanced,
    __tauri_command_name_import_parse_package, import_decrypt_preview, import_execute_advanced,
    import_parse_package,
};
#[cfg(test)]
pub(crate) use import::{
    build_selected_ids, import_execute_for_session, import_execute_internal,
    import_execute_resumable_for_session, resume_import_for_session,
};
pub use operations::{
    __cmd__import_operation_get, __cmd__import_operation_resume, __cmd__import_operations_list,
    __tauri_command_name_import_operation_get, __tauri_command_name_import_operation_resume,
    __tauri_command_name_import_operations_list, import_operation_get, import_operation_resume,
    import_operations_list, ImportOperationSummary, ImportOperationSummaryPhase,
    ImportOperationSummarySource,
};
#[cfg(test)]
pub(crate) use solosoul_core::export_import::import::{
    rebuild_imported_templates, restore_package_snapshots, snapshots_any_restorable,
    wrap_attachment_progress,
};

#[cfg(test)]
pub(crate) use import::cleanup_orphan_import_temps;
