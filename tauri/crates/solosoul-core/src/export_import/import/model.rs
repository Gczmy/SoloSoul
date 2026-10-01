//! RF-024：导入服务的普通数据与结果，不依赖宿主锁、IPC 或前端错误前缀。
use serde::{Deserialize, Serialize};
use solosoul_vault::ObjectSummary;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub enum AdvancedImportStrategy {
    /// Skip conflicting objects (keep existing)
    #[default]
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
pub struct ImportOutcome {
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

impl ImportOutcome {
    pub fn is_complete(&self) -> bool {
        self.status == ImportStatus::Complete
    }

    pub fn require_complete(self) -> Result<Self, String> {
        if self.is_complete() {
            Ok(self)
        } else {
            Err(format!(
                "Import incomplete: stage={:?}, objects={}, attachments={}",
                self.failure_stage, self.object_count, self.attachment_count
            ))
        }
    }

    pub(super) fn fail(&mut self, stage: ImportStage, error: &str) {
        self.status = if self.object_count
            + self.attachment_count
            + self.template_count
            + self.snapshot_count
            + self.attachment_files_written
            > 0
            || self.preferences_imported
        {
            ImportStatus::Partial
        } else {
            ImportStatus::NotCommitted
        };
        self.failure_stage = Some(stage);
        // 只允许无明细的已知密码码保留；原始错误可能含敏感 JSON/路径。
        self.error_code = Some(
            match error {
                "PASSWORD_REQUIRED" => "PASSWORD_REQUIRED",
                "BAD_PASSWORD" => "BAD_PASSWORD",
                "DECRYPT_FAILED" => "DECRYPT_FAILED",
                _ => "IMPORT_FAILED",
            }
            .to_string(),
        );
    }
}

#[derive(Debug, Clone, Default)]
pub struct ImportOptions {
    pub strategy: AdvancedImportStrategy,
    pub selections: Option<Vec<ImportSelection>>,
    pub selected_attachment_ids: Option<Vec<String>>,
    pub object_strategies: std::collections::HashMap<String, AdvancedImportStrategy>,
    pub locale: String,
}
/// 路径已由客户端边界授权；密码由实际同步 worker 持有并擦除。
pub struct EncryptedImportRequest {
    pub source_path: String,
    pub password: zeroize::Zeroizing<String>,
    pub options: ImportOptions,
    /// None 保留旧一次性入口；实际 GUI/Cloud/Recovery 使用持久操作。
    pub operation: Option<(String, solosoul_vault::ImportSourceKind)>,
}
#[derive(Debug)]
pub enum ImportFailure {
    Code {
        code: String,
        detail: Option<String>,
    },
    Backend(String),
}
impl From<String> for ImportFailure {
    fn from(value: String) -> Self {
        let (code, detail) = value
            .split_once(':')
            .map(|(c, d)| (c, Some(d.to_string())))
            .unwrap_or((&value, None));
        if matches!(
            code,
            "FILE_NOT_FOUND"
                | "INVALID_PACKAGE"
                | "MISSING_MANIFEST"
                | "MISSING_SALT"
                | "DECRYPT_FAILED"
                | "PASSWORD_REQUIRED"
                | "BAD_PASSWORD"
                | "INVALID_OPERATION_ID"
                | "INVALID_CLOUD_IMPORT_OPTIONS"
                | "OPERATION_MISMATCH"
                | "OPERATION_NOT_FOUND"
                | "OPERATION_ABANDONED"
        ) {
            Self::Code {
                code: code.to_string(),
                detail,
            }
        } else {
            Self::Backend(value)
        }
    }
}
impl std::fmt::Display for ImportFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Code {
                code,
                detail: Some(detail),
            } => write!(f, "{code}:{detail}"),
            Self::Code { code, detail: None } => f.write_str(code),
            Self::Backend(message) => f.write_str(message),
        }
    }
}
impl std::error::Error for ImportFailure {}
