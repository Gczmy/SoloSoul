//! 真实加密包、会话、导入台账和 production worker 的结构化拒绝回归。
use super::rf020::{objects, package, Fixture};
use super::*;
use crate::commands::error::BackendError;
use crate::commands::export_import::errors::TransferFailure;
use crate::commands::export_import::export::{run_export_job_safe, ExportJob};
use crate::commands::export_import::import::{run_import_job_safe, ImportJob, PreviewJob};
use std::path::PathBuf;
fn packet(e: TransferFailure) -> serde_json::Value {
    let p = serde_json::to_value(BackendError::from(e)).unwrap();
    assert!(!p.to_string().contains("RF318_PRIVATE"));
    assert!(!p.to_string().contains("incoming.solosoul"));
    p
}
fn request(path: &std::path::Path, password: &str) -> ExportRequest {
    ExportRequest {
        scope: ExportScope {
            selected_page_ids: vec![],
            selected_object_ids: vec![],
            selected_tags: vec![],
            include_attachments: false,
            selected_attachment_ids: vec![],
            include_preferences: false,
            include_behavioral: false,
            include_all: false,
        },
        password: password.into(),
        password_hint: None,
        save_path: path.to_string_lossy().into_owned(),
    }
}
fn import_request(path: &std::path::Path) -> AdvancedImportRequest {
    AdvancedImportRequest {
        selections: None,
        strategy: ImportStrategy::Overwrite,
        source_path: path.to_string_lossy().into_owned(),
        password: "export-password".into(),
        selected_attachment_ids: None,
        object_strategies: HashMap::new(),
        locale: "en-US".into(),
        operation_id: Some(Uuid::new_v4().to_string()),
    }
}
fn preview(f: &Fixture, path: &std::path::Path, password: &str) -> PreviewJob {
    PreviewJob::prepare_safe(
        f.service.clone(),
        path.to_string_lossy().into_owned(),
        Zeroizing::new(password.into()),
        |p| Ok(PathBuf::from(p)),
    )
    .unwrap()
}
#[test]
fn rf318_preview_bad_package_password_and_missing_file_are_safe_without_writes() {
    let f = Fixture::new();
    let incoming = package(f.dir.path(), objects(), false, false, false);
    let before = std::fs::read(&incoming).unwrap();
    let error = packet(
        preview(&f, &incoming, "RF318_PRIVATE_WRONG_PASSWORD")
            .run_safe()
            .unwrap_err(),
    );
    assert_eq!(error["code"], "IMPORT_DECRYPT_FAILED");
    assert_eq!(error["retryable"], true);
    assert_eq!(std::fs::read(&incoming).unwrap(), before);
    let bad = f.dir.path().join("RF318_PRIVATE_BAD.solosoul");
    std::fs::write(&bad, b"RF318_PRIVATE_INVALID_ZIP").unwrap();
    assert_eq!(
        packet(preview(&f, &bad, "export-password").run_safe().unwrap_err())["code"],
        "IMPORT_INVALID_PACKAGE"
    );
    let missing = f.dir.path().join("RF318_PRIVATE_MISSING.solosoul");
    assert_eq!(
        packet(
            preview(&f, &missing, "export-password")
                .run_safe()
                .unwrap_err()
        )["code"],
        "IMPORT_FILE_MISSING"
    );
    assert_eq!(
        f.db.query_row("SELECT COUNT(*) FROM objects", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert!(!std::fs::read_dir(f.vault.base_path()).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with("solosoul-import-")));
}
#[test]
fn rf318_export_validations_and_path_denial_preserve_existing_file() {
    let f = Fixture::new();
    let target = f.dir.path().join("RF318_PRIVATE_DEST.solosoul");
    std::fs::write(&target, b"existing valid output").unwrap();
    for (password, code) in [
        ("", "EXPORT_PASSWORD_REQUIRED"),
        ("password123", "EXPORT_PASSWORD_MATCHES_MASTER"),
        ("export-password", "EXPORT_SCOPE_EMPTY"),
    ] {
        let job = ExportJob::prepare_safe(
            f.service.clone(),
            &f.account,
            request(&target, password),
            |p| Ok(p.into()),
        )
        .unwrap();
        assert_eq!(packet(job.run_safe().unwrap_err())["code"], code);
        assert_eq!(std::fs::read(&target).unwrap(), b"existing valid output");
    }
    let e = ExportJob::prepare_safe(
        f.service.clone(),
        &f.account,
        request(&target, "export-password"),
        |_| Err("RF318_PRIVATE_DENIED_PATH".into()),
    )
    .err()
    .unwrap();
    assert_eq!(packet(e)["code"], "TRANSFER_INVALID_PATH");
    assert_eq!(std::fs::read(target).unwrap(), b"existing valid output");
}
#[test]
fn rf318_queued_jobs_reject_expired_sessions_and_busy_root() {
    let f = Fixture::new();
    let incoming = package(f.dir.path(), objects(), false, false, false);
    let job = preview(&f, &incoming, "export-password");
    let target = f.dir.path().join("RF318_PRIVATE_DEST.solosoul");
    let export = ExportJob::prepare_safe(
        f.service.clone(),
        &f.account,
        request(&target, "export-password"),
        |p| Ok(p.into()),
    )
    .unwrap();
    let maintenance = solosoul_core::import_activity::begin_owned_root_maintenance(
        f.service.read().unwrap().root_owner(),
    )
    .unwrap();
    assert_eq!(packet(export.run_safe().unwrap_err())["code"], "VAULT_BUSY");
    drop(maintenance);
    f.service.write().unwrap().lock();
    assert_eq!(
        packet(job.run_safe().unwrap_err())["code"],
        "SESSION_EXPIRED"
    );
    let e = PreviewJob::prepare_safe(
        f.service.clone(),
        incoming.to_string_lossy().into_owned(),
        Zeroizing::new("export-password".into()),
        |p| Ok(p.into()),
    )
    .err()
    .unwrap();
    assert_eq!(packet(e)["code"], "VAULT_LOCKED");
}
#[test]
fn rf318_actual_import_keeps_atomic_not_committed_and_attachment_partial_outcomes() {
    for attachment_failure in [false, true] {
        let f = Fixture::new();
        let path = package(
            f.dir.path(),
            objects(),
            attachment_failure,
            attachment_failure,
            false,
        );
        if !attachment_failure {
            f.reject_nth_object_write(2);
        }
        let job = ImportJob::prepare_safe(
            f.service.clone(),
            &f.account,
            import_request(&path),
            None,
            |p| Ok(p.into()),
        )
        .unwrap();
        let outcome = job.run_safe().unwrap();
        let encoded = serde_json::to_value(&outcome).unwrap();
        assert_eq!(
            encoded["status"],
            if attachment_failure {
                "partial"
            } else {
                "notCommitted"
            }
        );
        assert_eq!(outcome.object_count, if attachment_failure { 2 } else { 0 });
        assert_eq!(outcome.attachment_count, 0);
        assert_eq!(
            outcome.attachment_files_written,
            usize::from(attachment_failure)
        );
        assert!(!outcome.is_complete());
        assert!(outcome.operation_id.is_some());
        let stored =
            f.db.query_row(
                "SELECT COUNT(*) FROM objects WHERE id LIKE 'rf020-%'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap();
        assert_eq!(stored, if attachment_failure { 2 } else { 0 });
        assert!(!encoded
            .to_string()
            .contains("sensitive injected database detail"));
    }
}
#[test]
fn rf318_mutating_worker_panic_is_unconfirmed_and_never_calls_completion() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let p = packet(
        rt.block_on(run_export_job_safe(|| panic!("RF318_PRIVATE_PANIC")))
            .unwrap_err(),
    );
    assert_eq!(p["code"], "TRANSFER_TASK_UNCONFIRMED");
    assert_eq!(p["retryable"], false);
    assert!(p["safeDetails"].get("completedCount").is_none());
    let f = Fixture::new();
    let incoming = package(f.dir.path(), objects(), false, false, false);
    let job = ImportJob::prepare_safe(
        f.service.clone(),
        &f.account,
        import_request(&incoming),
        None,
        |p| Ok(p.into()),
    )
    .unwrap();
    let completed = std::sync::atomic::AtomicBool::new(false);
    let p = packet(
        rt.block_on(run_import_job_safe(
            job,
            |_| panic!("RF318_PRIVATE_PANIC"),
            || completed.store(true, std::sync::atomic::Ordering::SeqCst),
        ))
        .unwrap_err(),
    );
    assert_eq!(p["code"], "TRANSFER_TASK_UNCONFIRMED");
    assert!(!completed.load(std::sync::atomic::Ordering::SeqCst));
}
