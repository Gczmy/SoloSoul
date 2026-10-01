//! 对象边界按实际失败阶段构造错误，不解析数据库/字段校验的自由文本。
use crate::commands::{
    error::{BackendError, BackendErrorCode as Code, BackendErrorStage as Stage},
    ActivityVaultHandle,
};
use crate::state::AppState;
use solosoul_core::import_activity::begin_owned_root_activity;
use solosoul_vault::VaultStore;

pub(super) fn vault_handle(state: &AppState) -> Result<ActivityVaultHandle, BackendError> {
    vault_handle_for_service(&state.vault_service)
}
pub(super) fn vault_handle_for_service(
    service: &std::sync::RwLock<solosoul_core::VaultService>,
) -> Result<ActivityVaultHandle, BackendError> {
    let svc = service
        .read()
        .map_err(|_| BackendError::new(Code::InternalError))?;
    let activity = begin_owned_root_activity(svc.root_owner()).map_err(|cause| {
        // 既有基础设施返回稳定 token；这里只适配 token，不猜测 IO 英文正文。
        let code = if cause == "IMPORT_DIRECTORY_BUSY" {
            Code::VaultBusy
        } else {
            Code::InternalError
        };
        BackendError::caused_by(code, Stage::Read, cause)
    })?;
    let store = svc
        .get_vault_store()
        .ok_or_else(|| BackendError::new(Code::VaultLocked))?;
    Ok(ActivityVaultHandle {
        store,
        _activity: std::sync::Arc::new(activity),
    })
}
pub(super) fn current_account(state: &AppState) -> Result<String, BackendError> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| BackendError::new(Code::InternalError))?;
    svc.get_current_account()
        .ok_or_else(|| BackendError::new(Code::VaultLocked))
}
pub(super) fn read(cause: String) -> BackendError {
    BackendError::caused_by(Code::ObjectReadFailed, Stage::Read, cause)
}
pub(super) fn write(cause: String) -> BackendError {
    BackendError::caused_by(Code::ObjectWriteFailed, Stage::Write, cause)
}
pub(super) fn validate(cause: String) -> BackendError {
    BackendError::caused_by(Code::ObjectValidationFailed, Stage::Validate, cause)
}
pub(super) fn task(cause: tokio::task::JoinError) -> BackendError {
    BackendError::caused_by(Code::InternalError, Stage::Task, cause)
}
pub(super) fn create(cause: solosoul_core::objects::CreateRecordError) -> BackendError {
    let (code, stage) = match cause.stage {
        solosoul_core::objects::CreateRecordErrorStage::TemplateRead => {
            (Code::ObjectTemplateReadFailed, Stage::Template)
        }
        solosoul_core::objects::CreateRecordErrorStage::Validation => {
            (Code::ObjectValidationFailed, Stage::Validate)
        }
    };
    BackendError::caused_by(code, stage, cause)
}
pub(super) fn rollback(cause: solosoul_core::objects::RollbackError) -> BackendError {
    use solosoul_core::objects::RollbackErrorStage as R;
    let (code, stage) = match cause.stage {
        R::SnapshotOwner => (Code::SnapshotReadFailed, Stage::SnapshotOwner),
        R::SnapshotNotFound => (Code::SnapshotNotFound, Stage::SnapshotRead),
        R::Ownership => (Code::SnapshotOwnershipMismatch, Stage::SnapshotOwner),
        R::SnapshotRead => (Code::SnapshotReadFailed, Stage::SnapshotRead),
        R::SnapshotParse => (Code::SnapshotInvalid, Stage::SnapshotParse),
        R::ObjectRead => (Code::ObjectReadFailed, Stage::ObjectRead),
        R::ObjectNotFound => (Code::ObjectNotFound, Stage::ObjectRead),
        R::Labels => (Code::SnapshotInvalid, Stage::Labels),
        R::Version => (Code::SnapshotRollbackFailed, Stage::Version),
        R::Serialize => (Code::SnapshotRollbackFailed, Stage::Serialize),
        R::ObjectSave => (Code::ObjectWriteFailed, Stage::ObjectSave),
    };
    BackendError::caused_by(code, stage, cause)
}
/// 已提交对象的附属写入仍为 best-effort，失败诊断不记录对象名称/字段或原始 cause。
pub(super) fn save_snapshot_best_effort(
    vault: &VaultStore,
    object_id: &str,
    triggered_by: &str,
    data: &[u8],
    diff_summary: &str,
) {
    if let Err(cause) = vault.save_snapshot(object_id, triggered_by, data, diff_summary) {
        warn_followup(Stage::SnapshotSave, cause);
    }
}
#[allow(clippy::too_many_arguments)]
pub(super) fn log_audit_best_effort(
    vault: &VaultStore,
    action_type: &str,
    entity_type: &str,
    entity_id: Option<&str>,
    entity_name: Option<&str>,
    performed_by: &str,
    details: Option<&str>,
) {
    if let Err(cause) = vault.log_structured(
        action_type,
        entity_type,
        entity_id,
        entity_name,
        performed_by,
        details,
    ) {
        warn_followup(Stage::Audit, cause);
    }
}
pub(super) fn warn_followup(stage: Stage, _cause: String) {
    // 保留原诊断类别，已提交成功不能改成可重试失败。
    let message = match stage {
        Stage::SnapshotSave => "Snapshot save failed",
        _ => "Audit log write failed",
    };
    tracing::warn!(stage = ?stage, cause_type = "String", "{}", message);
}
