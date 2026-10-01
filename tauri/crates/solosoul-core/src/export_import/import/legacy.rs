//! RF-024：旧 CLI 参数/策略与计数兼容，提交和恢复使用统一 Core 管线。
use super::super::*;
use super::commit;
fn cli_import_options(path: &Path, strategy: ImportStrategy) -> serde_json::Value {
    serde_json::json!({
        "strategy": match strategy {
            ImportStrategy::SkipExisting => "skip",
            ImportStrategy::Overwrite => "overwrite",
            ImportStrategy::Merge => "merge",
        },
        "sourceName": path.file_name().unwrap_or_default().to_string_lossy(),
        "includeAttachments": true,
        "includePreferences": true,
    })
}

#[allow(clippy::too_many_arguments)]
fn finish_core_import_operation(
    service: &crate::VaultService,
    session: &crate::VaultSession,
    accepted: solosoul_vault::ImportOperationRecord,
    native_root: &Path,
    owned: Option<&operation::OwnedImportPackage>,
    password: Option<&str>,
    attachment_key: &[u8; 32],
) -> Result<CoreImportOperationOutcome, ExportError> {
    let operation_id = accepted.start.operation_id.clone();
    let object_write_count = import_operation_object_count(&accepted)?;
    let execution = commit::complete_operation(
        service,
        session,
        &accepted,
        native_root,
        owned,
        password,
        attachment_key,
        None,
    );
    let committed = execution.progress;
    let (current, error_code) = match execution.outcome {
        Ok(current) => (current, None),
        Err(error) => (
            execution.persisted.unwrap_or(accepted),
            Some(core_resume_error_code(&error)),
        ),
    };
    Ok(CoreImportOperationOutcome {
        operation_id,
        complete: error_code.is_none()
            && current.phase == solosoul_vault::ImportOperationPhase::Complete,
        object_write_count,
        attachment_count: current.attachment_count.max(committed.committed_count),
        written_file_count: current
            .steps
            .iter()
            .filter(|step| {
                matches!(
                    step.phase,
                    solosoul_vault::ImportAttachmentPhase::Published
                        | solosoul_vault::ImportAttachmentPhase::MetadataCommitted
                )
            })
            .count()
            .max(committed.written_file_count),
        preferences_imported: current.preferences_imported,
        error_code,
    })
}

#[allow(clippy::too_many_arguments)]
pub(in crate::export_import) fn execute(
    service: &crate::VaultService,
    session: &crate::VaultSession,
    operation_id: &str,
    path: &Path,
    password: &str,
    strategy: ImportStrategy,
    native_root: &Path,
    attachment_key: &[u8; 32],
) -> Result<CoreImportOperationOutcome, ExportError> {
    service.with_session(session, |_| Ok(()))?;
    uuid::Uuid::parse_str(operation_id).map_err(|_| "import_invalid_operation_id")?;
    let _activity = crate::import_activity::begin_import_activity(native_root)?;
    let owned = operation::OwnedImportPackage::capture(path, native_root)?;
    let options = cli_import_options(path, strategy);
    let fingerprint = operation::import_request_fingerprint(&options)?;
    let root_binding = operation::import_root_binding(native_root, session.vault())?;
    let existing = service.with_session(session, |vault| {
        vault.load_import_operation(session.account_id(), operation_id)
    })?;
    if let Some(existing) = existing {
        if existing.start.source_kind != solosoul_vault::ImportSourceKind::Cli
            || existing.start.source != *owned.source_proof()
            || existing.start.root_binding != root_binding
            || existing.start.request_fingerprint != fingerprint
        {
            return Err("OPERATION_CONFLICT".into());
        }
        return finish_core_import_operation(
            service,
            session,
            existing,
            native_root,
            Some(&owned),
            Some(password),
            attachment_key,
        );
    }
    // 原 Core 纯准备器保留 Skip 批前 membership、重复新 ID 写计数、模板 hash 与 Keep 历史。
    let opened = owned.decrypt(password, native_root)?;
    let now = chrono::Utc::now().to_rfc3339();
    let mut prepared = prepare_import_database(
        session.vault(),
        session.account_id(),
        &opened.payload,
        strategy,
        &now,
    )?;
    let mut start = operation::prepare_import_operation(
        operation_id,
        solosoul_vault::ImportSourceKind::Cli,
        options,
        &owned,
        &opened.payload,
        &prepared.imported_object_ids,
        &HashMap::new(),
        None,
        &now,
        native_root,
        session.vault(),
        &prepared.view,
        &mut prepared.batch,
        opened.has_attachments,
        opened.has_preferences,
    )?;
    // 这是派生写计数，放在加密计划顶层，不污染请求 fingerprint/同 ID 再发的比较。
    start.plan["cliObjectWriteCount"] = serde_json::json!(prepared.batch.objects.len());
    let accepted = commit::commit_operation(
        service,
        session,
        &prepared.view.revision,
        &prepared.batch,
        &start,
    )?;
    let result = finish_core_import_operation(
        service,
        session,
        accepted.operation,
        native_root,
        Some(&owned),
        Some(password),
        attachment_key,
    )?;
    if result.complete && !accepted.already_committed {
        let _ = service.with_session(session, |vault| {
            let _ = vault.log_structured(
                "import_execute",
                "import",
                None,
                None,
                "user",
                Some(&format!(
                    "imported {} objects (operation: {})",
                    result.object_write_count, operation_id
                )),
            );
            Ok(())
        });
    }
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
pub(in crate::export_import) fn resume(
    service: &crate::VaultService,
    session: &crate::VaultSession,
    operation_id: &str,
    source_path: Option<&Path>,
    package_password: Option<&str>,
    native_root: &Path,
    attachment_key: &[u8; 32],
) -> Result<CoreImportOperationOutcome, ExportError> {
    service.with_session(session, |_| Ok(()))?;
    let _activity = crate::import_activity::begin_import_activity(native_root)?;
    let accepted = service
        .with_session(session, |vault| {
            vault.load_import_operation(session.account_id(), operation_id)
        })?
        .ok_or("OPERATION_NOT_FOUND")?;
    // copy/proof 与恢复 worker 同属实际许可；Native 再核验原 proof/root/epoch。
    let owned = source_path
        .map(|path| operation::OwnedImportPackage::capture(path, native_root))
        .transpose()?;
    if owned
        .as_ref()
        .is_some_and(|source| source.source_proof() != &accepted.start.source)
    {
        return Err("import_source_changed".into());
    }
    finish_core_import_operation(
        service,
        session,
        accepted,
        native_root,
        owned.as_ref(),
        package_password,
        attachment_key,
    )
}
