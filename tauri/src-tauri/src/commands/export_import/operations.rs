//! RF022：只公开当前账户任务摘要与指定任务恢复，不向 IPC 暴露 journal/map/路径写许可。
use super::contracts::ImportResult;
use super::errors;
use super::errors::TransferFailure;
use crate::commands::error::{BackendError, BackendErrorCode as Code, BackendErrorStage as Stage};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use solosoul_core::export_import::operation::import_credential_requirements;
use solosoul_core::{VaultService, VaultSession};
use solosoul_vault::{ImportOperationPhase, ImportOperationRecord, ImportSourceKind};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tauri::State;
use zeroize::Zeroizing;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImportOperationSummaryPhase {
    RecordsCommitted,
    Attachments,
    Preferences,
    Complete,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImportOperationSummarySource {
    Manual,
    Cloud,
    Recovery,
    Cli,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOperationSummary {
    pub operation_id: String,
    pub phase: ImportOperationSummaryPhase,
    pub source_kind: ImportOperationSummarySource,
    pub source_name: String,
    pub created_at: String,
    pub updated_at: String,
    pub source_required: bool,
    pub password_required: bool,
    pub outcome: ImportResult,
}

fn summary(
    record: &ImportOperationRecord,
    generation: u64,
) -> Result<ImportOperationSummary, TransferFailure> {
    let phase = match record.phase {
        ImportOperationPhase::RecordsCommitted => ImportOperationSummaryPhase::RecordsCommitted,
        ImportOperationPhase::Attachments => ImportOperationSummaryPhase::Attachments,
        ImportOperationPhase::Preferences => ImportOperationSummaryPhase::Preferences,
        ImportOperationPhase::Complete => ImportOperationSummaryPhase::Complete,
        ImportOperationPhase::Abandoned => {
            return Err(TransferFailure::new(
                Code::ImportOperationAbandoned,
                Stage::Read,
                "__IMPORT_ERR__:OPERATION_ABANDONED",
            ))
        }
    };
    let kind = match record.start.source_kind {
        ImportSourceKind::Manual => ImportOperationSummarySource::Manual,
        ImportSourceKind::Cloud => ImportOperationSummarySource::Cloud,
        ImportSourceKind::Recovery => ImportOperationSummarySource::Recovery,
        ImportSourceKind::Cli => ImportOperationSummarySource::Cli,
    };
    let credentials = import_credential_requirements(record)
        .map_err(|error| TransferFailure::new(Code::ImportReadFailed, Stage::Read, error))?;
    let mut outcome = ImportResult::default();
    super::import::apply_operation_result(record, generation, &mut outcome);
    let timestamp = |ms: i64| {
        chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
            .map(|time| time.to_rfc3339())
            .unwrap_or_default()
    };
    Ok(ImportOperationSummary {
        operation_id: record.start.operation_id.clone(),
        phase,
        source_kind: kind,
        source_name: record
            .start
            .plan
            .get("sourceName")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Imported package")
            .into(),
        created_at: timestamp(record.created_at_ms),
        updated_at: timestamp(record.updated_at_ms),
        source_required: credentials.source_required,
        password_required: credentials.password_required,
        outcome,
    })
}

#[cfg(test)]
pub(crate) fn operation_summary_for_session(
    svc: &VaultService,
    session: &VaultSession,
    id: &str,
) -> Result<ImportOperationSummary, String> {
    operation_summary_for_session_safe(svc, session, id).map_err(TransferFailure::into_legacy)
}
fn operation_summary_for_session_safe(
    svc: &VaultService,
    session: &VaultSession,
    id: &str,
) -> Result<ImportOperationSummary, TransferFailure> {
    let record = errors::session(svc, session, |vault| {
        vault
            .load_import_operation(session.account_id(), id)
            .map_err(|e| TransferFailure::new(Code::ImportReadFailed, Stage::Read, e))
    })?
    .ok_or_else(|| {
        TransferFailure::new(
            Code::ImportOperationNotFound,
            Stage::Read,
            "__IMPORT_ERR__:OPERATION_NOT_FOUND",
        )
    })?;
    summary(&record, session.generation())
}

#[tauri::command]
pub async fn import_operations_list(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<Vec<ImportOperationSummary>, BackendError> {
    let svc = state.vault_service.read().map_err(|_| errors::internal())?;
    let session = errors::capture(&svc, &account_id)?;
    let records = errors::session(&svc, &session, |vault| {
        vault
            .list_import_operations(&account_id)
            .map_err(|e| TransferFailure::new(Code::ImportReadFailed, Stage::Read, e))
    })?;
    records
        .iter()
        .map(|record| summary(record, session.generation()).map_err(Into::into))
        .collect()
}
#[tauri::command]
pub async fn import_operation_get(
    state: State<'_, AppState>,
    account_id: String,
    operation_id: String,
) -> Result<ImportOperationSummary, BackendError> {
    let svc = state.vault_service.read().map_err(|_| errors::internal())?;
    let session = errors::capture(&svc, &account_id)?;
    operation_summary_for_session_safe(&svc, &session, &operation_id).map_err(Into::into)
}

struct ResumeJob {
    service: Arc<RwLock<VaultService>>,
    session: VaultSession,
    operation_id: String,
    source_path: Option<String>,
    password: Option<Zeroizing<String>>,
    _activity: solosoul_core::import_activity::ImportActivityGuard,
}
impl ResumeJob {
    fn prepare(
        service: Arc<RwLock<VaultService>>,
        account: &str,
        id: String,
        password: Option<String>,
        source: Option<String>,
        resolve: impl FnOnce(&str) -> Result<PathBuf, String>,
    ) -> Result<Self, TransferFailure> {
        let source_path = source
            .map(|path| {
                resolve(&path).map_err(errors::path).and_then(|resolved| {
                    resolved
                        .into_os_string()
                        .into_string()
                        .map_err(|_| errors::path("Invalid import path encoding".into()))
                })
            })
            .transpose()?;
        let (session, activity) = {
            let svc = service.read().map_err(|_| errors::internal())?;
            let session = errors::capture(&svc, account)?;
            operation_summary_for_session_safe(&svc, &session, &id)?;
            (
                session,
                solosoul_core::import_activity::begin_import_activity(svc.base_path())
                    .map_err(errors::activity)?,
            )
        };
        Ok(Self {
            service,
            session,
            operation_id: id,
            source_path,
            password: password.map(Zeroizing::new),
            _activity: activity,
        })
    }
    fn run(self) -> Result<ImportResult, TransferFailure> {
        let svc = self.service.read().map_err(|_| errors::internal())?;
        let result = solosoul_core::export_import::import::resume_encrypted_import(
            &svc,
            &self.session,
            &self.operation_id,
            self.source_path,
            self.password,
            None,
        );
        match result {
            Ok(outcome) => Ok(outcome.into()),
            Err(error) => {
                errors::session(&svc, &self.session, |_| Ok(()))?;
                Err(errors::import_failure(error))
            }
        }
    }
}

#[tauri::command]
pub async fn import_operation_resume<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: State<'_, AppState>,
    account_id: String,
    operation_id: String,
    password: Option<String>,
    source_path: Option<String>,
) -> Result<ImportResult, BackendError> {
    let job = ResumeJob::prepare(
        Arc::clone(&state.vault_service),
        &account_id,
        operation_id,
        password,
        source_path,
        |path| super::import::resolve_import_path(&app, path),
    )?;
    let service = Arc::clone(&job.service);
    let session = job.session.clone();
    let result = tokio::task::spawn_blocking(move || job.run())
        .await
        .map_err(|_| {
            TransferFailure::new(
                Code::TransferTaskUnconfirmed,
                Stage::Task,
                "导入恢复任务执行失败",
            )
        })??;
    if result.is_complete() {
        if let Ok(svc) = service.read() {
            let _ = svc.with_session(&session, |_| {
                state.auto_sync.trigger_debounce();
                Ok(())
            });
        }
    }
    Ok(result)
}
