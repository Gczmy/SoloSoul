//! RF022 Host 的真实 SQLite / 加密 ZIP 与 Cloud 完成门槛回归候选。
use super::rf020::{objects, package, Fixture};
use super::*;
use solosoul_vault::ImportSourceKind;
use std::path::{Path, PathBuf};

fn incoming(f: &Fixture, attachments: bool, corrupt: bool) -> PathBuf {
    let package = package(f.dir.path(), objects(), attachments, corrupt, false);
    let source = f
        .service
        .read()
        .unwrap()
        .base_path()
        .join("cloud_sync_incoming")
        .join(&f.account)
        .join("remote-device")
        .join("123-0.solosoul");
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    std::fs::copy(package, &source).unwrap();
    source
}
fn run(
    f: &Fixture,
    path: &Path,
    id: &str,
    kind: ImportSourceKind,
    strategy: ImportStrategy,
) -> ImportResult {
    let svc = f.service.read().unwrap();
    let session = svc.capture_session(&f.account).unwrap();
    import_execute_resumable_for_session(
        &svc,
        &session,
        path.to_string_lossy().into_owned(),
        Zeroizing::new("export-password".into()),
        strategy,
        None,
        None,
        HashMap::new(),
        "en-US",
        None,
        id,
        kind,
    )
    .unwrap()
}
fn resume(
    f: &Fixture,
    id: &str,
    source: Option<&Path>,
    password: Option<&str>,
) -> Result<ImportResult, String> {
    let svc = f.service.read().unwrap();
    let session = svc.capture_session(&f.account).unwrap();
    resume_import_for_session(
        &svc,
        &session,
        id,
        source.map(|p| p.to_string_lossy().into_owned()),
        password.map(|p| Zeroizing::new(p.into())),
        None,
    )
}
fn block_metadata(f: &Fixture) {
    f.db.execute_batch("CREATE TRIGGER rf022_block_meta BEFORE UPDATE OF properties ON objects BEGIN SELECT RAISE(ABORT,'sensitive raw metadata failure'); END;").unwrap();
}
fn unblock_metadata(f: &Fixture) {
    f.db.execute_batch("DROP TRIGGER rf022_block_meta;")
        .unwrap();
}
fn id() -> String {
    Uuid::new_v4().to_string()
}

#[test]
fn rf022_host_database_failure_is_notcommitted_and_operation_id_is_retained() {
    let f = Fixture::new();
    let source = package(f.dir.path(), objects(), true, false, false);
    let operation = id();
    f.reject_nth_object_write(2);
    let result = run(
        &f,
        &source,
        &operation,
        ImportSourceKind::Manual,
        ImportStrategy::Overwrite,
    );
    assert_eq!(result.operation_id.as_deref(), Some(operation.as_str()));
    assert_eq!(result.status, ImportStatus::NotCommitted);
    assert_eq!(result.object_count, 0);
    assert_eq!(result.snapshot_count, 0);
    assert_eq!(result.attachment_files_written, 0);
    assert!(f
        .vault
        .load_import_operation(&f.account, &operation)
        .unwrap()
        .is_none());
    assert!(source.exists());
}

#[test]
fn rf022_host_published_metadata_failure_reopens_and_resumes_without_source_or_password() {
    let f = Fixture::new();
    let source = package(f.dir.path(), objects(), true, false, false);
    let operation = id();
    block_metadata(&f);
    let partial = run(
        &f,
        &source,
        &operation,
        ImportSourceKind::Manual,
        ImportStrategy::Overwrite,
    );
    assert_eq!(partial.status, ImportStatus::Partial);
    assert_eq!(partial.object_count, 2);
    assert_eq!(partial.snapshot_count, 2);
    assert_eq!(partial.attachment_count, 0);
    assert_eq!(partial.attachment_files_written, 2);
    assert_eq!(partial.failure_stage, Some(ImportStage::Attachments));
    assert_eq!(partial.error_code.as_deref(), Some("IMPORT_FAILED"));
    unblock_metadata(&f);
    std::fs::remove_file(source).unwrap();
    let base = {
        let svc = f.service.read().unwrap();
        let base = svc.base_path().to_path_buf();
        svc.lock();
        base
    };
    let Fixture {
        service,
        vault,
        account,
        db,
        dir,
    } = f;
    drop(service);
    drop(vault);
    drop(db);
    // 保留目录，关闭全部旧 SQLite/Vault 句柄后创建新服务，不把同一进程旧锁称为崩溃。
    let reopened = solosoul_core::VaultService::with_base_path(base);
    reopened.unlock(&account, "password123").unwrap();
    let session = reopened.capture_session(&account).unwrap();
    let complete =
        resume_import_for_session(&reopened, &session, &operation, None, None, None).unwrap();
    assert_eq!(complete.status, ImportStatus::Complete);
    assert_eq!(complete.object_count, 2);
    assert_eq!(complete.snapshot_count, 2);
    assert_eq!(complete.attachment_count, 2);
    assert_eq!(complete.attachment_files_written, 2);
    let repeated =
        resume_import_for_session(&reopened, &session, &operation, None, None, None).unwrap();
    assert_eq!(repeated.attachment_count, 2);
    assert_eq!(repeated.snapshot_count, 2);
    drop(session);
    drop(reopened);
    drop(dir);
}

#[test]
fn rf022_host_sameid_keepboth_returns_original_maps_newid_produces_new_copies() {
    let f = Fixture::new();
    let source = package(f.dir.path(), objects(), true, false, false);
    let first = id();
    let result = run(
        &f,
        &source,
        &first,
        ImportSourceKind::Manual,
        ImportStrategy::KeepBoth,
    );
    assert_eq!(result.status, ImportStatus::Complete);
    let first_plan = f
        .vault
        .load_import_operation(&f.account, &first)
        .unwrap()
        .unwrap()
        .start;
    {
        let svc = f.service.read().unwrap();
        let session = svc.capture_session(&f.account).unwrap();
        let summary =
            super::super::operations::operation_summary_for_session(&svc, &session, &first)
                .unwrap();
        assert_eq!(serde_json::to_value(&summary).unwrap()["phase"], "complete");
        assert!(!summary.source_required);
        assert!(!summary.password_required);
        assert!(f
            .vault
            .list_import_operations(&f.account)
            .unwrap()
            .is_empty());
    }
    let repeated = run(
        &f,
        &source,
        &first,
        ImportSourceKind::Manual,
        ImportStrategy::KeepBoth,
    );
    assert_eq!(repeated.object_count, 2);
    assert_eq!(repeated.attachment_count, 2);
    assert_eq!(f.vault.list_object_records(&f.account).unwrap().len(), 2);
    assert_eq!(
        f.vault
            .load_import_operation(&f.account, &first)
            .unwrap()
            .unwrap()
            .start,
        first_plan
    );
    let second = id();
    let copied = run(
        &f,
        &source,
        &second,
        ImportSourceKind::Manual,
        ImportStrategy::KeepBoth,
    );
    assert_eq!(copied.status, ImportStatus::Complete);
    assert_eq!(f.vault.list_object_records(&f.account).unwrap().len(), 4);
    assert_ne!(
        f.vault
            .load_import_operation(&f.account, &second)
            .unwrap()
            .unwrap()
            .start
            .steps[0]
            .attachment_id,
        first_plan.steps[0].attachment_id
    );
}

#[test]
fn rf022_host_sameid_changed_options_and_source_are_rejected_without_new_writes() {
    let f = Fixture::new();
    let source = package(f.dir.path(), objects(), false, false, false);
    let operation = id();
    run(
        &f,
        &source,
        &operation,
        ImportSourceKind::Manual,
        ImportStrategy::Overwrite,
    )
    .require_complete()
    .unwrap();
    let mismatch = run(
        &f,
        &source,
        &operation,
        ImportSourceKind::Manual,
        ImportStrategy::KeepBoth,
    );
    assert_eq!(mismatch.status, ImportStatus::Partial);
    assert_eq!(f.vault.list_object_records(&f.account).unwrap().len(), 2);
    // New salted bytes are not allowed to substitute for the operation's original source.
    let changed = package(f.dir.path(), objects(), false, false, false);
    let changed_result = resume(&f, &operation, Some(&changed), Some("export-password")).unwrap();
    assert_eq!(changed_result.status, ImportStatus::Partial);
    assert_eq!(changed_result.object_count, 2);
    assert_eq!(f.vault.list_object_records(&f.account).unwrap().len(), 2);
}

#[test]
fn rf022_host_accepted_empty_journal_resume_failure_is_partial_not_notcommitted() {
    let f = Fixture::new();
    let source = package(f.dir.path(), objects(), false, false, true);
    let operation = id();
    let svc = f.service.read().unwrap();
    let session = svc.capture_session(&f.account).unwrap();
    let outcome = import_execute_resumable_for_session(
        &svc,
        &session,
        source.to_string_lossy().into_owned(),
        Zeroizing::new("export-password".into()),
        ImportStrategy::Overwrite,
        Some(vec![]),
        Some(vec![]),
        HashMap::new(),
        "en-US",
        None,
        &operation,
        ImportSourceKind::Manual,
    )
    .unwrap();
    assert_eq!(outcome.status, ImportStatus::Partial);
    assert_eq!(outcome.object_count, 0);
    assert_eq!(outcome.template_count, 0);
    assert_eq!(outcome.failure_stage, Some(ImportStage::Preferences));
    assert!(f
        .vault
        .load_import_operation(&f.account, &operation)
        .unwrap()
        .is_some());
    drop(svc);
    assert_eq!(
        resume(&f, &operation, None, None).unwrap().status,
        ImportStatus::Partial
    );
}

#[test]
fn rf022_host_unknown_operation_and_foreign_account_fail_closed() {
    let f = Fixture::new();
    assert_eq!(
        resume(&f, &id(), None, None).unwrap_err(),
        "__IMPORT_ERR__:OPERATION_NOT_FOUND"
    );
    let source = package(f.dir.path(), objects(), true, false, false);
    let operation = id();
    block_metadata(&f);
    run(
        &f,
        &source,
        &operation,
        ImportSourceKind::Manual,
        ImportStrategy::Overwrite,
    );
    unblock_metadata(&f);
    let svc = f.service.read().unwrap();
    svc.create_account_with_id("acc_rf022_other", "Other", "password456", None)
        .unwrap();
    let session = svc.capture_session("acc_rf022_other").unwrap();
    assert!(resume_import_for_session(&svc, &session, &operation, None, None, None).is_err());
    assert!(session
        .vault()
        .list_object_records("acc_rf022_other")
        .unwrap()
        .is_empty());
}

#[test]
fn rf022_cloud_allskip_retry_resumes_attachments_and_waterline_requires_complete_journal() {
    let f = Fixture::new();
    let source = incoming(&f, true, false);
    let first = id();
    block_metadata(&f);
    let partial = run(
        &f,
        &source,
        &first,
        ImportSourceKind::Cloud,
        ImportStrategy::SkipExisting,
    );
    assert_eq!(partial.status, ImportStatus::Partial);
    assert_eq!(partial.object_count, 2);
    assert_eq!(partial.attachment_count, 0);
    assert_eq!(partial.attachment_files_written, 2);
    let svc = f.service.read().unwrap();
    let session = svc.capture_session(&f.account).unwrap();
    assert!(
        crate::sync::cloud_auto_sync::finalize_cloud_import(&svc, &session, &source, &first)
            .is_err()
    );
    assert!(source.exists());
    assert!(f
        .vault
        .get_sys_config("cloud_applied:remote-device")
        .unwrap()
        .is_none());
    drop(svc);
    unblock_metadata(&f);
    let retry = run(
        &f,
        &source,
        &id(),
        ImportSourceKind::Cloud,
        ImportStrategy::SkipExisting,
    );
    assert_eq!(retry.operation_id.as_deref(), Some(first.as_str()));
    assert_eq!(retry.status, ImportStatus::Complete);
    assert_eq!(retry.object_count, 2);
    assert_eq!(retry.attachment_count, 2);
    assert_eq!(retry.snapshot_count, 2);
    // The shared backend uses the original database counts/map even though a new Skip preparation
    // would now find every object present and otherwise skip all attachment publication.
    assert_eq!(f.vault.list_object_records(&f.account).unwrap().len(), 2);
    let svc = f.service.read().unwrap();
    let session = svc.capture_session(&f.account).unwrap();
    crate::sync::cloud_auto_sync::finalize_cloud_import(&svc, &session, &source, &first).unwrap();
    assert!(!source.exists());
    assert_eq!(
        f.vault
            .get_sys_config("cloud_applied:remote-device")
            .unwrap()
            .as_deref(),
        Some("123-0")
    );
}

#[test]
fn rf022_cloud_forged_completion_unknown_id_changed_source_and_manual_kind_never_finalize() {
    let f = Fixture::new();
    let source = incoming(&f, false, false);
    let svc = f.service.read().unwrap();
    let session = svc.capture_session(&f.account).unwrap();
    assert!(
        crate::sync::cloud_auto_sync::finalize_cloud_import(&svc, &session, &source, &id())
            .is_err()
    );
    drop(svc);
    let manual = id();
    run(
        &f,
        &source,
        &manual,
        ImportSourceKind::Manual,
        ImportStrategy::Overwrite,
    )
    .require_complete()
    .unwrap();
    let svc = f.service.read().unwrap();
    let session = svc.capture_session(&f.account).unwrap();
    assert!(
        crate::sync::cloud_auto_sync::finalize_cloud_import(&svc, &session, &source, &manual)
            .is_err()
    );
    drop(svc);
    let cloud = id();
    let complete = run(
        &f,
        &source,
        &cloud,
        ImportSourceKind::Cloud,
        ImportStrategy::SkipExisting,
    );
    assert_eq!(complete.status, ImportStatus::Complete);
    std::fs::write(
        &source,
        b"different source even though old frontend result says Complete",
    )
    .unwrap();
    let svc = f.service.read().unwrap();
    let session = svc.capture_session(&f.account).unwrap();
    assert!(crate::sync::cloud_auto_sync::finalize_cloud_import(
        &svc,
        &session,
        &source,
        complete.operation_id.as_deref().unwrap()
    )
    .is_err());
    assert!(source.exists());
    assert!(f
        .vault
        .get_sys_config("cloud_applied:remote-device")
        .unwrap()
        .is_none());
}

#[test]
fn rf022_operation_summary_tracks_real_source_credential_need_without_sensitive_fields() {
    let f = Fixture::new();
    let source = package(f.dir.path(), objects(), true, true, false);
    let operation = id();
    let partial = run(
        &f,
        &source,
        &operation,
        ImportSourceKind::Manual,
        ImportStrategy::Overwrite,
    );
    assert_eq!(partial.status, ImportStatus::Partial);
    let svc = f.service.read().unwrap();
    let session = svc.capture_session(&f.account).unwrap();
    let summary =
        super::super::operations::operation_summary_for_session(&svc, &session, &operation)
            .unwrap();
    assert!(summary.source_required);
    assert!(summary.password_required);
    assert_eq!(summary.outcome.attachment_count, 0);
    assert_eq!(summary.outcome.attachment_files_written, 1);
    assert_eq!(summary.source_name, "incoming.solosoul");
    let serialized = serde_json::to_string(&summary).unwrap();
    assert!(!serialized.contains("export-password"));
    assert!(!serialized.contains("sourceProof"));
    assert!(!serialized.contains("rootBinding"));
    assert!(!serialized.contains(f.dir.path().to_str().unwrap()));
}
