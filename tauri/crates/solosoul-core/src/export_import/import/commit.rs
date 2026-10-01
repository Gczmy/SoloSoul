//! RF-024：所有持久导入共用原会话提交门槛与附件/偏好恢复收尾。
use super::super::operation::{resume_import_operation, OwnedImportPackage};
use super::super::{AttachmentImportProgress, ExportError};
use crate::{VaultService, VaultSession};
use solosoul_vault::{
    ImportBatchRevision, ImportDatabaseBatch, ImportOperationCommit, ImportOperationRecord,
    ImportOperationStart,
};
use std::path::Path;
pub(super) fn commit_operation(
    service: &VaultService,
    session: &VaultSession,
    revision: &ImportBatchRevision,
    batch: &ImportDatabaseBatch,
    start: &ImportOperationStart,
) -> Result<ImportOperationCommit, String> {
    service.with_session(session, |vault| {
        vault.commit_import_batch_with_operation(session.account_id(), revision, batch, start)
    })
}
pub(super) struct OperationExecution {
    pub outcome: Result<ImportOperationRecord, ExportError>,
    pub persisted: Option<ImportOperationRecord>,
    pub progress: AttachmentImportProgress,
}
#[allow(clippy::too_many_arguments)]
pub(super) fn complete_operation(
    service: &VaultService,
    session: &VaultSession,
    accepted: &ImportOperationRecord,
    native_root: &Path,
    owned: Option<&OwnedImportPackage>,
    password: Option<&str>,
    attachment_key: &[u8; 32],
    callback: Option<&(dyn Fn(u8) + Send + Sync)>,
) -> OperationExecution {
    let mut progress = AttachmentImportProgress::default();
    let outcome = resume_import_operation(
        service,
        session,
        &accepted.start.operation_id,
        native_root,
        owned,
        password,
        attachment_key,
        callback,
        &mut progress,
    );
    // 原会话失效后不得转向新账户读取进度；已提交计数仍由本 worker 返回。
    let persisted = if outcome.is_err() {
        service
            .with_session(session, |vault| {
                vault.load_import_operation(session.account_id(), &accepted.start.operation_id)
            })
            .ok()
            .flatten()
    } else {
        None
    };
    OperationExecution {
        outcome,
        persisted,
        progress,
    }
}
