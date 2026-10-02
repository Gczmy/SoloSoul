//! RF-317：仅按失败阶段编码；原始 URL、响应、会话内容和 IO cause 不进入载荷/日志。
use crate::commands::{
    error::{BackendError, BackendErrorCode as Code, BackendErrorStage as Stage},
    ActivityVaultHandle,
};
use crate::state::AppState;
use solosoul_core::import_activity::begin_owned_root_activity;

pub(super) fn activity(cause: String) -> BackendError {
    let code = if cause == "IMPORT_DIRECTORY_BUSY" {
        Code::VaultBusy
    } else {
        Code::InternalError
    };
    BackendError::caused_by(code, Stage::Read, cause)
}
pub(super) fn vault_handle(state: &AppState) -> Result<ActivityVaultHandle, BackendError> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| BackendError::new(Code::InternalError))?;
    let activity = begin_owned_root_activity(svc.root_owner()).map_err(activity)?;
    let store = svc
        .get_vault_store()
        .ok_or_else(|| BackendError::new(Code::VaultLocked))?;
    Ok(ActivityVaultHandle {
        store,
        _activity: std::sync::Arc::new(activity),
    })
}
pub(super) fn provider_read(cause: String) -> BackendError {
    BackendError::caused_by(Code::LlmProviderReadFailed, Stage::Read, cause)
}
pub(super) fn provider_write(cause: String) -> BackendError {
    BackendError::caused_by(Code::LlmProviderWriteFailed, Stage::Write, cause)
}
pub(super) fn conversation_read(cause: String) -> BackendError {
    BackendError::caused_by(Code::LlmConversationReadFailed, Stage::Read, cause)
}
pub(super) fn conversation_write(cause: String) -> BackendError {
    BackendError::caused_by(Code::LlmConversationWriteFailed, Stage::Write, cause)
}
pub(super) fn invalid_request(cause: String) -> BackendError {
    BackendError::caused_by(Code::LlmInvalidRequest, Stage::Validate, cause)
}
pub(super) fn guide(cause: String) -> BackendError {
    BackendError::caused_by(Code::LlmGuideFailed, Stage::Read, cause)
}
pub(super) fn embedding<E>(cause: E) -> BackendError {
    BackendError::caused_by(Code::LlmEmbeddingFailed, Stage::Task, cause)
}
pub(super) fn provider_resolve(
    cause: solosoul_core::llm::service::ChatProviderError,
) -> BackendError {
    use solosoul_core::llm::service::ChatProviderError as P;
    let code = match cause {
        P::NotSaved | P::MissingConfig => Code::LlmProviderNotConfigured,
        P::Disabled => Code::LlmProviderDisabled,
        P::ReadFailed | P::InvalidConfig | P::InvalidCredentials => Code::LlmProviderReadFailed,
    };
    BackendError::caused_by(code, Stage::Read, cause)
}
pub(super) fn network(cause: reqwest::Error) -> BackendError {
    let code = if cause.is_timeout() {
        Code::LlmTimeout
    } else {
        Code::LlmNetworkFailed
    };
    BackendError::caused_by(code, Stage::Task, cause)
}
pub(super) fn response(cause: reqwest::Error) -> BackendError {
    BackendError::caused_by(Code::LlmResponseInvalid, Stage::Read, cause)
}
pub(super) fn http_status(status: reqwest::StatusCode) -> BackendError {
    let code = if status.as_u16() == 429 {
        Code::LlmRateLimited
    } else if status.is_server_error() {
        Code::LlmProviderUnavailable
    } else {
        Code::LlmProviderRejected
    };
    BackendError::new(code).at(Stage::Read)
}
