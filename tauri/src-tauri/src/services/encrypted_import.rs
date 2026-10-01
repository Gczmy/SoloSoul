//! RF-024：Core 的普通导入数据与 Host wire DTO / i18n 错误前缀之间的薄适配。
use crate::commands::export_import::*;
use solosoul_core::export_import::import as core;
use std::collections::HashMap;
use zeroize::Zeroizing;
pub(crate) fn map_import_failure(error: core::ImportFailure) -> String {
    match error {
        core::ImportFailure::Code {
            code,
            detail: Some(detail),
        } => import_err_with_detail(&code, &detail),
        core::ImportFailure::Code { code, detail: None } => import_err(&code),
        core::ImportFailure::Backend(message) => message,
    }
}
impl ImportStrategy {
    pub(crate) fn to_core(self) -> core::AdvancedImportStrategy {
        match self {
            Self::SkipExisting => core::AdvancedImportStrategy::SkipExisting,
            Self::Overwrite => core::AdvancedImportStrategy::Overwrite,
            Self::KeepBoth => core::AdvancedImportStrategy::KeepBoth,
        }
    }
}
impl From<core::ImportStatus> for ImportStatus {
    fn from(value: core::ImportStatus) -> Self {
        match value {
            core::ImportStatus::Complete => Self::Complete,
            core::ImportStatus::Partial => Self::Partial,
            core::ImportStatus::NotCommitted => Self::NotCommitted,
        }
    }
}
impl From<core::ImportStage> for ImportStage {
    fn from(value: core::ImportStage) -> Self {
        match value {
            core::ImportStage::Preparation => Self::Preparation,
            core::ImportStage::Templates => Self::Templates,
            core::ImportStage::Objects => Self::Objects,
            core::ImportStage::Snapshots => Self::Snapshots,
            core::ImportStage::Attachments => Self::Attachments,
            core::ImportStage::Preferences => Self::Preferences,
        }
    }
}
impl From<core::ConflictKind> for ConflictKind {
    fn from(value: core::ConflictKind) -> Self {
        match value {
            core::ConflictKind::Identical => Self::Identical,
            core::ConflictKind::RenamedLocal => Self::RenamedLocal,
        }
    }
}
impl From<core::ImportOutcome> for ImportResult {
    fn from(value: core::ImportOutcome) -> Self {
        Self {
            operation_id: value.operation_id,
            session_generation: value.session_generation,
            object_count: value.object_count,
            attachment_count: value.attachment_count,
            status: value.status.into(),
            template_count: value.template_count,
            snapshot_count: value.snapshot_count,
            preferences_imported: value.preferences_imported,
            attachment_files_written: value.attachment_files_written,
            failure_stage: value.failure_stage.map(Into::into),
            error_code: value.error_code,
        }
    }
}
impl From<core::AttachmentImportInfo> for AttachmentImportInfo {
    fn from(value: core::AttachmentImportInfo) -> Self {
        Self {
            id: value.id,
            object_id: value.object_id,
            file_name: value.file_name,
            size_bytes: value.size_bytes,
        }
    }
}
impl From<core::ConflictInfo> for ConflictInfo {
    fn from(value: core::ConflictInfo) -> Self {
        Self {
            object_id: value.object_id,
            imported_name: value.imported_name,
            existing_name: value.existing_name,
            kind: value.kind.into(),
        }
    }
}
impl From<core::DecryptedImportPreview> for DecryptedImportPreview {
    fn from(value: core::DecryptedImportPreview) -> Self {
        Self {
            objects: value.objects,
            conflicts: value.conflicts.into_iter().map(Into::into).collect(),
            has_preferences: value.has_preferences,
            has_audit_log: value.has_audit_log,
            attachments: value.attachments.into_iter().map(Into::into).collect(),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn core_request(
    source_path: String,
    password: Zeroizing<String>,
    strategy: ImportStrategy,
    selections: Option<Vec<ImportSelection>>,
    selected_attachment_ids: Option<Vec<String>>,
    object_strategies: HashMap<String, ImportStrategy>,
    locale: &str,
    operation: Option<(String, solosoul_vault::ImportSourceKind)>,
) -> core::EncryptedImportRequest {
    core::EncryptedImportRequest {
        source_path,
        password,
        operation,
        options: core::ImportOptions {
            strategy: strategy.to_core(),
            selections: selections.map(|values| {
                values
                    .into_iter()
                    .map(|value| core::ImportSelection {
                        object_id: value.object_id,
                        selected: value.selected,
                    })
                    .collect()
            }),
            selected_attachment_ids,
            object_strategies: object_strategies
                .into_iter()
                .map(|(id, strategy)| (id, strategy.to_core()))
                .collect(),
            locale: locale.to_owned(),
        },
    }
}
