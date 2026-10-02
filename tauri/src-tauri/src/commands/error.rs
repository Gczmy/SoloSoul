//! IPC 错误只携带可公开的机器信息；cause 不进入序列化、Display 或 Debug。
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BackendErrorCode {
    VaultLocked,
    VaultBusy,
    SessionExpired,
    InternalError,
    ObjectNameRequired,
    ObjectNameTooLong,
    ObjectPayloadTooLarge,
    ObjectNotFound,
    ObjectIdExists,
    ObjectValidationFailed,
    ObjectReadFailed,
    ObjectWriteFailed,
    ObjectTemplateMissing,
    ObjectTemplateNotFound,
    ObjectTemplateReadFailed,
    SnapshotReadFailed,
    SnapshotNotFound,
    SnapshotInvalid,
    SnapshotOwnershipMismatch,
    SnapshotRollbackFailed,
    LlmInvalidRequest,
    LlmProviderNotConfigured,
    LlmProviderDisabled,
    LlmProviderNotRegistered,
    LlmConfirmationCancelled,
    LlmConfirmationTimeout,
    LlmProviderReadFailed,
    LlmProviderWriteFailed,
    LlmNetworkFailed,
    LlmTimeout,
    LlmProviderRejected,
    LlmProviderUnavailable,
    LlmRateLimited,
    LlmResponseInvalid,
    LlmConversationNotFound,
    LlmConversationReadFailed,
    LlmConversationWriteFailed,
    LlmReplySaveFailed,
    LlmContextReadFailed,
    LlmUsageFailed,
    LlmGuideFailed,
    LlmEmbeddingFailed,
}
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum BackendErrorStage {
    Validate,
    Read,
    Write,
    Task,
    Template,
    SnapshotOwner,
    SnapshotRead,
    SnapshotParse,
    ObjectRead,
    Labels,
    Version,
    Serialize,
    ObjectSave,
    SnapshotSave,
    Audit,
}
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SafeErrorDetails {
    pub stage: BackendErrorStage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
}
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BackendError {
    pub code: BackendErrorCode,
    pub safe_details: Option<SafeErrorDetails>,
    pub retryable: bool,
}
impl BackendError {
    pub fn new(code: BackendErrorCode) -> Self {
        use BackendErrorCode as Code;
        Self {
            code,
            safe_details: None,
            retryable: matches!(
                code,
                Code::VaultLocked
                    | Code::VaultBusy
                    | Code::ObjectReadFailed
                    | Code::ObjectTemplateReadFailed
                    | Code::SnapshotReadFailed
                    | Code::LlmNetworkFailed
                    | Code::LlmTimeout
                    | Code::LlmProviderUnavailable
                    | Code::LlmRateLimited
                    | Code::LlmProviderReadFailed
                    | Code::LlmConversationReadFailed
            ),
        }
    }
    pub fn at(mut self, stage: BackendErrorStage) -> Self {
        self.safe_details = Some(SafeErrorDetails { stage, limit: None });
        self
    }
    pub fn limit(mut self, limit: u64) -> Self {
        if let Some(details) = &mut self.safe_details {
            details.limit = Some(limit);
        }
        self
    }
    /// cause 的自由文本可能含字段值、SQL、路径、密钥或 panic 载荷，绝不记录它的 Display。
    /// 脱敏诊断仅保留失败阶段与 Rust cause 类型；安全信息与返回载荷一致。
    pub fn caused_by<E>(code: BackendErrorCode, stage: BackendErrorStage, _cause: E) -> Self {
        let error = Self::new(code).at(stage);
        use BackendErrorCode as Code;
        let message = match code {
            Code::ObjectNameRequired
            | Code::ObjectNameTooLong
            | Code::ObjectPayloadTooLarge
            | Code::ObjectNotFound
            | Code::ObjectIdExists
            | Code::ObjectValidationFailed
            | Code::ObjectReadFailed
            | Code::ObjectWriteFailed
            | Code::ObjectTemplateMissing
            | Code::ObjectTemplateNotFound
            | Code::ObjectTemplateReadFailed
            | Code::SnapshotReadFailed
            | Code::SnapshotNotFound
            | Code::SnapshotInvalid
            | Code::SnapshotOwnershipMismatch
            | Code::SnapshotRollbackFailed => "Object operation failed",
            _ => "Backend operation failed",
        };
        tracing::warn!(code = ?code, stage = ?stage, cause_type = std::any::type_name::<E>(), message);
        error
    }
}
impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.code)
    }
}
impl std::error::Error for BackendError {}
