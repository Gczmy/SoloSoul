//! RF318：阶段决定机器错误；legacy 自由文本只留在内部兼容入口，不进 IPC/日志。
use crate::commands::error::{BackendError, BackendErrorCode as Code, BackendErrorStage as Stage};
use crate::commands::ActivityVaultHandle;
use crate::state::AppState;
use solosoul_core::{VaultService, VaultSession};

pub(crate) struct TransferFailure {
    code: Code,
    stage: Stage,
    legacy: String,
    completed: Option<u64>,
}
impl TransferFailure {
    pub(crate) fn new(code: Code, stage: Stage, cause: impl ToString) -> Self {
        Self {
            code,
            stage,
            legacy: cause.to_string(),
            completed: None,
        }
    }
    pub(crate) fn completed(mut self, count: usize) -> Self {
        self.completed = Some(count as u64);
        self
    }
    #[cfg(test)]
    pub(crate) fn into_legacy(self) -> String {
        self.legacy
    }
}
impl std::fmt::Debug for TransferFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransferFailure")
            .field("code", &self.code)
            .field("stage", &self.stage)
            .field("completed", &self.completed)
            .finish()
    }
}
impl From<TransferFailure> for BackendError {
    fn from(failure: TransferFailure) -> Self {
        let error = BackendError::caused_by(failure.code, failure.stage, failure.legacy);
        match failure.completed {
            Some(count) => error.completed_count(count),
            None => error,
        }
    }
}
pub(crate) fn internal() -> TransferFailure {
    TransferFailure::new(
        Code::InternalError,
        Stage::Task,
        "Vault service lock poisoned",
    )
}
pub(crate) fn activity(cause: String) -> TransferFailure {
    let code = match cause.as_str() {
        "IMPORT_DIRECTORY_BUSY" | "IMPORT_OPERATIONS_ACTIVE" => Code::VaultBusy,
        _ => Code::InternalError,
    };
    TransferFailure::new(code, Stage::Task, cause)
}
pub(crate) fn path(cause: String) -> TransferFailure {
    TransferFailure::new(Code::TransferInvalidPath, Stage::Validate, cause)
}
pub(crate) fn capture(svc: &VaultService, account: &str) -> Result<VaultSession, TransferFailure> {
    let code = if svc.get_current_account().is_none() {
        Code::VaultLocked
    } else {
        Code::SessionExpired
    };
    svc.capture_session(account)
        .map_err(|cause| TransferFailure::new(code, Stage::Read, cause))
}
pub(crate) fn session<T>(
    svc: &VaultService,
    session: &VaultSession,
    work: impl FnOnce(&solosoul_vault::VaultStore) -> Result<T, TransferFailure>,
) -> Result<T, TransferFailure> {
    let mut result = None;
    svc.with_session(session, |vault| {
        result = Some(work(vault));
        Ok(())
    })
    .map_err(|cause| TransferFailure::new(Code::SessionExpired, Stage::Read, cause))?;
    result.ok_or_else(internal)?
}
pub(crate) fn vault_handle(state: &AppState) -> Result<ActivityVaultHandle, TransferFailure> {
    let svc = state.vault_service.read().map_err(|_| internal())?;
    let activity = solosoul_core::import_activity::begin_owned_root_activity(svc.root_owner())
        .map_err(activity)?;
    let store = svc.get_vault_store().ok_or_else(|| {
        TransferFailure::new(Code::VaultLocked, Stage::Read, "Vault not unlocked")
    })?;
    Ok(ActivityVaultHandle {
        store,
        _activity: std::sync::Arc::new(activity),
    })
}
pub(crate) fn export_failure(
    error: solosoul_core::export_import::export::ExportFailure,
) -> TransferFailure {
    use solosoul_core::export_import::export::ExportFailure as E;
    let code = match &error {
        E::PasswordEmpty => Code::ExportPasswordRequired,
        E::SameAsMasterPassword => Code::ExportPasswordMatchesMaster,
        E::MasterVerifyFailed(_) => Code::ExportPasswordCheckFailed,
        E::NoObjectsSelected => Code::ExportScopeEmpty,
        E::AttachmentTooLarge(_) => Code::ExportAttachmentTooLarge,
        E::TotalSizeExceeded => Code::ExportTooLarge,
        E::Backend(_) => Code::ExportFailed,
    };
    TransferFailure::new(
        code,
        match &error {
            E::MasterVerifyFailed(_) => Stage::Read,
            E::Backend(_) => Stage::Task,
            _ => Stage::Validate,
        },
        crate::services::encrypted_export::map_export_failure(error),
    )
}
pub(crate) fn import_failure(
    error: solosoul_core::export_import::import::ImportFailure,
) -> TransferFailure {
    use solosoul_core::export_import::import::ImportFailure as I;
    let code = match &error {
        I::Code { code, .. } => match code.as_str() {
            "FILE_NOT_FOUND" => Code::ImportFileMissing,
            "INVALID_PACKAGE" => Code::ImportInvalidPackage,
            "MISSING_MANIFEST" => Code::ImportManifestMissing,
            "MISSING_SALT" => Code::ImportSaltMissing,
            "DECRYPT_FAILED" => Code::ImportDecryptFailed,
            "PASSWORD_REQUIRED" => Code::ImportPasswordRequired,
            "BAD_PASSWORD" => Code::ImportBadPassword,
            "INVALID_OPERATION_ID" => Code::ImportInvalidOperation,
            "INVALID_CLOUD_IMPORT_OPTIONS" => Code::ImportInvalidCloudOptions,
            "OPERATION_MISMATCH" => Code::ImportOperationMismatch,
            "OPERATION_NOT_FOUND" => Code::ImportOperationNotFound,
            "OPERATION_ABANDONED" => Code::ImportOperationAbandoned,
            "OPERATION_CONFLICT" => Code::ImportOperationConflict,
            _ => Code::ImportFailed,
        },
        I::Backend(cause)
            if matches!(
                cause.as_str(),
                "IMPORT_DIRECTORY_BUSY" | "IMPORT_OPERATIONS_ACTIVE"
            ) =>
        {
            Code::VaultBusy
        }
        I::Backend(cause) if cause == "OPERATION_CONFLICT" => Code::ImportOperationConflict,
        I::Backend(_) => Code::ImportFailed,
    };
    TransferFailure::new(
        code,
        match code {
            Code::ImportFileMissing | Code::ImportDecryptFailed => Stage::Read,
            Code::ImportInvalidPackage
            | Code::ImportManifestMissing
            | Code::ImportSaltMissing
            | Code::ImportPasswordRequired
            | Code::ImportBadPassword
            | Code::ImportInvalidOperation
            | Code::ImportInvalidCloudOptions
            | Code::ImportOperationMismatch => Stage::Validate,
            _ => Stage::Task,
        },
        crate::services::encrypted_import::map_import_failure(error),
    )
}
