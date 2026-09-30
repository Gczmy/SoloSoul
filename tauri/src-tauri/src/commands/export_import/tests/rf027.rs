use super::super::import::{run_import_job, ImportJob};
use super::rf020::{objects, package, Fixture};
use super::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Mutex};
use std::time::Duration;

const WATCHDOG: Duration = Duration::from_secs(30);
const IDS: [&str; 2] = ["rf020-0", "rf020-1"];
type Progress = Arc<dyn Fn(u8) + Send + Sync>;

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

fn request(path: &Path) -> AdvancedImportRequest {
    AdvancedImportRequest {
        selections: None,
        strategy: ImportStrategy::Overwrite,
        source_path: path.to_str().unwrap().into(),
        password: "export-password".into(),
        selected_attachment_ids: None,
        object_strategies: HashMap::new(),
        locale: "en-US".into(),
    }
}

fn prepare(f: &Fixture, req: AdvancedImportRequest, progress: Option<Progress>) -> ImportJob {
    ImportJob::prepare(f.service.clone(), &f.account, req, progress, |path| {
        Path::new(path).canonicalize().map_err(|e| e.to_string())
    })
    .unwrap()
}

fn run(
    f: &Fixture,
    req: AdvancedImportRequest,
    progress: Option<Progress>,
    calls: Arc<AtomicUsize>,
) -> Result<ImportResult, String> {
    runtime().block_on(run_import_job(
        prepare(f, req, progress),
        ImportJob::run,
        move || {
            calls.fetch_add(1, Ordering::SeqCst);
        },
    ))
}

fn generation(f: &Fixture) -> u64 {
    f.service
        .read()
        .unwrap()
        .capture_session(&f.account)
        .unwrap()
        .generation()
}

fn root(f: &Fixture) -> PathBuf {
    f.service.read().unwrap().base_path().to_path_buf()
}

fn counts(f: &Fixture) -> (usize, usize) {
    let objects =
        f.db.query_row(
            "SELECT COUNT(*) FROM objects WHERE id LIKE 'rf020-%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let snapshots =
        f.db.query_row(
            "SELECT COUNT(*) FROM object_snapshots WHERE object_id LIKE 'rf020-%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    (objects, snapshots)
}

fn current_records(f: &Fixture) -> Vec<ObjectRecord> {
    // 原 Fixture.vault 在切换后失效，必须使用当前账户的新句柄。
    let svc = f.service.read().unwrap();
    let account = svc.get_current_account().unwrap();
    let vault = svc.get_vault_store().unwrap();
    vault
        .list_objects(&account, None, None, None, true, false)
        .unwrap()
        .into_iter()
        .map(|summary| vault.load_object(&summary.id).unwrap().unwrap())
        .collect()
}

fn linked_paths(f: &Fixture) -> Vec<PathBuf> {
    current_records(f)
        .into_iter()
        .flat_map(|record| {
            record.properties["__attachments"]
                .as_array()
                .cloned()
                .unwrap_or_default()
        })
        .filter_map(|att| att["vaultPath"].as_str().map(PathBuf::from))
        .filter(|path| path.is_file())
        .collect()
}

fn attachment_files(base: &Path) -> Vec<PathBuf> {
    fn collect(path: &Path, files: &mut Vec<PathBuf>) {
        if !path.exists() {
            return;
        }
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                collect(&path, files);
            } else {
                files.push(path);
            }
        }
    }
    let mut files = Vec::new();
    collect(&base.join("attachments"), &mut files);
    files.sort();
    files
}

fn assert_attachment_bytes(f: &Fixture, expected_files: usize) {
    let svc = f.service.read().unwrap();
    let session = svc.capture_session(&f.account).unwrap();
    let key = svc.attachment_key_for_session(&session).unwrap();
    let files = attachment_files(svc.base_path());
    assert_eq!(files.len(), expected_files);
    for file in files {
        assert!(solosoul_core::attachment_crypto::is_encrypted_file(&file));
        assert_eq!(
            solosoul_core::attachment_crypto::read_file_decrypted(&key, &file, 1024).unwrap(),
            b"fixture"
        );
    }
}

fn assert_no_staging(base: &Path) {
    for entry in std::fs::read_dir(base).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().into_owned();
        assert!(
            !name.starts_with("solosoul-import-tmp-") && !name.starts_with("attachment-import-"),
            "遗留导入临时目录: {name}"
        );
    }
}

fn save_marker(f: &Fixture, name: &str) {
    let svc = f.service.read().unwrap();
    svc.get_vault_store()
        .unwrap()
        .save_object(&ObjectRecord {
            id: IDS[0].into(),
            account_id: svc.get_current_account().unwrap(),
            type_id: "note".into(),
            section_type: "identity".into(),
            name: name.into(),
            properties: json!({"marker":name}),
            created_at: "2026-09-27T00:00:00Z".into(),
            updated_at: "2026-09-27T00:00:00Z".into(),
            version: 7,
            ..Default::default()
        })
        .unwrap();
}

fn other_account(f: &Fixture) -> String {
    let account = format!("acc_{}", Uuid::new_v4().simple());
    f.service
        .read()
        .unwrap()
        .create_account_with_id(&account, "Other", "password456", None)
        .unwrap();
    account
}

fn assert_failed(
    result: &ImportResult,
    status: ImportStatus,
    stage: ImportStage,
    objects: usize,
    snapshots: usize,
) {
    assert_eq!(result.status, status);
    assert_eq!(result.failure_stage, Some(stage));
    assert_eq!(result.error_code.as_deref(), Some("IMPORT_FAILED"));
    assert_eq!(result.object_count, objects);
    assert_eq!(result.snapshot_count, snapshots);
    assert_eq!(result.template_count, 0);
    assert!(!result.preferences_imported);
}

#[derive(Clone, Copy)]
enum PauseAt {
    Queued,
    Progress(u8),
    Complete,
}

struct ReleaseOnDrop(Option<mpsc::Sender<()>>);
impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        if let Some(tx) = self.0.take() {
            let _ = tx.send(());
        }
    }
}

/// 同一生产调度器 + 真实进度回调。超时只做防挂死释放，不是性能通过阈值。
/// 会话动作另有完成通道，只有动作在 release 前完成才能通过；坏门闩实现先释放再 join。
fn run_paused(
    f: &Fixture,
    req: AdvancedImportRequest,
    at: PauseAt,
    while_paused: impl FnOnce() + Send + 'static,
    calls: Arc<AtomicUsize>,
) -> Result<ImportResult, String> {
    let (entered_tx, entered_rx) = mpsc::channel();
    let (async_entered_tx, async_entered_rx) = tokio::sync::oneshot::channel();
    let (progressed_tx, progressed_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let state = Mutex::new(Some((entered_tx, async_entered_tx, release_rx)));
    let pause: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
        let (entered, async_entered, release) =
            state.lock().unwrap().take().expect("屏障只能进入一次");
        entered.send(std::thread::current().id()).unwrap();
        async_entered.send(()).unwrap();
        release.recv().unwrap();
    });
    let progress_values = Arc::new(Mutex::new(Vec::new()));
    let progress = if let PauseAt::Progress(target) = at {
        let pause = pause.clone();
        let values = progress_values.clone();
        Some(Arc::new(move |pct| {
            values.lock().unwrap().push(pct);
            if pct == target {
                pause();
            }
        }) as Progress)
    } else {
        None
    };
    let job = prepare(f, req, progress);
    let coordinator = std::thread::spawn(move || -> Result<(), String> {
        let release = ReleaseOnDrop(Some(release_tx));
        let worker = entered_rx
            .recv_timeout(WATCHDOG)
            .map_err(|e| format!("导入未进入屏障: {e}"))?;
        let executor = progressed_rx
            .recv_timeout(WATCHDOG)
            .map_err(|e| format!("blocking 导入暂停时 runtime 未推进: {e}"))?;
        assert_ne!(worker, executor);
        let (finished_tx, finished_rx) = mpsc::channel();
        let action = std::thread::spawn(move || {
            while_paused();
            finished_tx.send(()).unwrap();
        });
        let finished = finished_rx.recv_timeout(WATCHDOG);
        drop(release);
        action.join().expect("屏障内动作不能 panic");
        finished.map_err(|e| format!("动作未在释放导入前完成: {e}"))
    });
    let (result, progressed) = runtime().block_on(async move {
        let independent = tokio::spawn(async move {
            async_entered_rx.await.map_err(|e| e.to_string())?;
            progressed_tx
                .send(std::thread::current().id())
                .map_err(|e| e.to_string())
        });
        let result = run_import_job(
            job,
            move |job| match at {
                PauseAt::Queued => {
                    pause();
                    job.run()
                }
                PauseAt::Progress(_) => job.run(),
                PauseAt::Complete => {
                    let result = job.run()?;
                    assert!(result.is_complete(), "必须先真实完成，再测试副作用接纳");
                    pause();
                    Ok(result)
                }
            },
            move || {
                calls.fetch_add(1, Ordering::SeqCst);
            },
        )
        .await;
        (result, independent.await)
    });
    coordinator.join().expect("协调线程不能 panic").unwrap();
    progressed.unwrap().unwrap();
    if let PauseAt::Progress(target) = at {
        let values = progress_values.lock().unwrap();
        assert!(values.contains(&target));
        assert!(values.windows(2).all(|pair| pair[0] <= pair[1]));
    }
    result
}

#[test]
fn rf027_real_progress_barrier_yields_current_thread_and_imports_encrypted_attachments() {
    let f = Fixture::new();
    let path = package(f.dir.path(), objects(), true, false, false);
    let source = std::fs::read(&path).unwrap();
    let expected_generation = generation(&f);
    let calls = Arc::new(AtomicUsize::new(0));
    let service = f.service.clone();
    let result = run_paused(
        &f,
        request(&path),
        PauseAt::Progress(40),
        move || {
            let svc = service.read().unwrap();
            let vault = svc.get_vault_store().unwrap();
            assert!(vault.load_object(IDS[0]).unwrap().is_none());
            assert!(vault.load_object(IDS[1]).unwrap().is_none());
            assert_eq!(vault.list_snapshots(IDS[0]).unwrap().len(), 0);
            assert_eq!(vault.list_snapshots(IDS[1]).unwrap().len(), 0);
        },
        calls.clone(),
    )
    .unwrap();
    assert_eq!(result.session_generation, expected_generation);
    assert_eq!(result.status, ImportStatus::Complete);
    assert_eq!(result.object_count, 2);
    assert_eq!(result.snapshot_count, 2);
    assert_eq!(result.attachment_count, 2);
    assert_eq!(result.attachment_files_written, 2);
    assert_eq!(result.template_count, 0);
    assert!(!result.preferences_imported);
    assert_eq!(result.failure_stage, None);
    assert_eq!(result.error_code, None);
    assert_eq!(counts(&f), (2, 2));
    assert_eq!(linked_paths(&f).len(), 2);
    assert_attachment_bytes(&f, 2);
    for id in IDS {
        let history = f.vault.list_snapshots(id).unwrap();
        assert_eq!(history[0]["triggeredBy"], "import");
        assert_eq!(history[0]["diffSummary"], "diff_imported");
        let snapshot: ObjectRecord = serde_json::from_slice(
            &f.vault
                .get_snapshot(history[0]["id"].as_str().unwrap())
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(snapshot.id, id);
        assert_eq!(snapshot.account_id, f.account);
        assert_eq!(snapshot.name, "Synthetic");
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(std::fs::read(path).unwrap(), source);
    assert_no_staging(&root(&f));
}

#[test]
fn rf027_queued_import_preserves_original_session_for_lock_switch_and_reunlock() {
    for mode in ["lock", "switch", "reunlock"] {
        let f = Fixture::new();
        save_marker(&f, "A unchanged");
        let a_before = serde_json::to_value(current_records(&f)).unwrap();
        let other = other_account(&f);
        save_marker(&f, "B unchanged");
        let b_before = serde_json::to_value(current_records(&f)).unwrap();
        f.service
            .read()
            .unwrap()
            .unlock(&f.account, "password123")
            .unwrap();
        let expected_generation = generation(&f);
        let path = package(f.dir.path(), objects(), true, false, false);
        let source = std::fs::read(&path).unwrap();
        let base = root(&f);
        let service = f.service.clone();
        let account = f.account.clone();
        let second = other.clone();
        let calls = Arc::new(AtomicUsize::new(0));
        let result = run_paused(
            &f,
            request(&path),
            PauseAt::Queued,
            move || {
                let svc = service.try_write().expect("排队时不能持有服务读锁");
                match mode {
                    "lock" => svc.lock(),
                    "switch" => svc.unlock(&second, "password456").unwrap(),
                    "reunlock" => {
                        svc.lock();
                        svc.unlock(&account, "password123").unwrap();
                    }
                    _ => unreachable!(),
                }
            },
            calls.clone(),
        )
        .unwrap();
        assert_eq!(result.session_generation, expected_generation);
        assert_failed(
            &result,
            ImportStatus::NotCommitted,
            ImportStage::Preparation,
            0,
            0,
        );
        assert_eq!(result.attachment_count, 0);
        assert_eq!(result.attachment_files_written, 0);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(std::fs::read(&path).unwrap(), source);
        assert!(attachment_files(&base).is_empty());
        assert_no_staging(&base);
        for (account, password, before) in [
            (&f.account, "password123", &a_before),
            (&other, "password456", &b_before),
        ] {
            f.service.read().unwrap().unlock(account, password).unwrap();
            assert_eq!(serde_json::to_value(current_records(&f)).unwrap(), *before);
            let vault = f.service.read().unwrap().get_vault_store().unwrap();
            assert!(vault.list_snapshots(IDS[0]).unwrap().is_empty());
            assert!(vault.list_snapshots(IDS[1]).unwrap().is_empty());
        }
    }
}

#[test]
fn rf027_async_results_keep_complete_partial_and_not_committed_counts() {
    for reject in [None, Some(1), Some(2)] {
        let f = Fixture::new();
        if let Some(nth) = reject {
            f.reject_nth_object_write(nth);
        }
        let path = package(f.dir.path(), objects(), false, false, false);
        let source = std::fs::read(&path).unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let expected_generation = generation(&f);
        let result = run(&f, request(&path), None, calls.clone()).unwrap();
        assert_eq!(result.session_generation, expected_generation);
        match reject {
            None => {
                assert!(result.is_complete());
                assert_eq!((result.object_count, result.snapshot_count), (2, 2));
                assert_eq!(result.failure_stage, None);
                assert_eq!(result.error_code, None);
            }
            Some(1) => assert_failed(
                &result,
                ImportStatus::NotCommitted,
                ImportStage::Objects,
                0,
                0,
            ),
            Some(2) => assert_failed(
                &result,
                ImportStatus::NotCommitted,
                ImportStage::Objects,
                0,
                0,
            ),
            _ => unreachable!(),
        }
        assert_eq!(counts(&f), (result.object_count, result.snapshot_count));
        assert_eq!(result.attachment_count, 0);
        assert_eq!(result.attachment_files_written, 0);
        assert_eq!(calls.load(Ordering::SeqCst), usize::from(reject.is_none()));
        assert!(!serde_json::to_string(&result)
            .unwrap()
            .contains("sensitive injected"));
        assert_eq!(std::fs::read(path).unwrap(), source);
        assert_no_staging(&root(&f));
    }
}

#[test]
fn rf027_attachment_snapshot_and_preferences_failures_remain_partial() {
    for fault in ["cipher", "metadata", "snapshot", "preferences"] {
        let f = Fixture::new();
        if fault == "metadata" {
            f.reject_nth_object_write(4);
        }
        if fault == "snapshot" {
            f.db.execute_batch("CREATE TRIGGER rf027_snapshot BEFORE INSERT ON object_snapshots BEGIN SELECT RAISE(ABORT, 'synthetic private snapshot fault'); END;").unwrap();
        }
        let attachments = fault == "cipher" || fault == "metadata";
        let path = package(
            f.dir.path(),
            objects(),
            attachments,
            fault == "cipher",
            fault == "preferences",
        );
        let source = std::fs::read(&path).unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let result = run(&f, request(&path), None, calls.clone()).unwrap();
        let (stage, objects, snapshots, linked, files) = match fault {
            "cipher" => (ImportStage::Attachments, 2, 2, 0, 1),
            "metadata" => (ImportStage::Attachments, 2, 2, 1, 2),
            "snapshot" => (ImportStage::Snapshots, 0, 0, 0, 0),
            "preferences" => (ImportStage::Preferences, 2, 2, 0, 0),
            _ => unreachable!(),
        };
        let status = if fault == "snapshot" {
            ImportStatus::NotCommitted
        } else {
            ImportStatus::Partial
        };
        assert_failed(&result, status, stage, objects, snapshots);
        assert_eq!(counts(&f), (objects, snapshots));
        assert_eq!(result.attachment_count, linked);
        assert_eq!(linked_paths(&f).len(), linked);
        assert_eq!(result.attachment_files_written, files);
        assert_attachment_bytes(&f, files);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(!serde_json::to_string(&result)
            .unwrap()
            .contains("synthetic private"));
        assert_eq!(std::fs::read(path).unwrap(), source);
        assert_no_staging(&root(&f));
    }
}

#[test]
fn rf027_running_import_stops_later_commits_at_object_and_attachment_progress() {
    for pause_at in [40, 90] {
        for mode in ["lock", "switch"] {
            let f = Fixture::new();
            let other = other_account(&f);
            f.service
                .read()
                .unwrap()
                .unlock(&f.account, "password123")
                .unwrap();
            let expected_generation = generation(&f);
            let path = package(f.dir.path(), objects(), true, false, false);
            let source = std::fs::read(&path).unwrap();
            let base = root(&f);
            let service = f.service.clone();
            let second = other.clone();
            let calls = Arc::new(AtomicUsize::new(0));
            let result = run_paused(
                &f,
                request(&path),
                PauseAt::Progress(pause_at),
                move || {
                    // worker 保留服务 read 的既有行为，但进度回调不能占住会话门闩或 DB 锁。
                    let svc = service.read().unwrap();
                    let vault = svc.get_vault_store().unwrap();
                    // RF021：40 是准备进度，90 才位于数据库提交后的附件阶段。
                    assert_eq!(
                        vault.list_snapshots(IDS[0]).unwrap().len(),
                        usize::from(pause_at == 90)
                    );
                    assert_eq!(vault.load_object(IDS[0]).unwrap().is_some(), pause_at == 90);
                    assert_eq!(vault.load_object(IDS[1]).unwrap().is_some(), pause_at == 90);
                    if mode == "lock" {
                        svc.lock();
                    } else {
                        svc.unlock(&second, "password456").unwrap();
                    }
                },
                calls.clone(),
            )
            .unwrap();
            let count = if pause_at == 40 { 0 } else { 2 };
            assert_eq!(result.session_generation, expected_generation);
            assert_failed(
                &result,
                if pause_at == 40 {
                    ImportStatus::NotCommitted
                } else {
                    ImportStatus::Partial
                },
                if pause_at == 40 {
                    ImportStage::Objects
                } else {
                    ImportStage::Attachments
                },
                count,
                count,
            );
            assert_eq!(counts(&f), (count, count));
            assert_eq!(result.attachment_count, 0);
            assert_eq!(result.attachment_files_written, 0);
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            assert!(attachment_files(&base).is_empty());
            assert_no_staging(&base);
            assert_eq!(std::fs::read(path).unwrap(), source);
            f.service
                .read()
                .unwrap()
                .unlock(&other, "password456")
                .unwrap();
            assert!(current_records(&f).is_empty());
            let b = f.service.read().unwrap().get_vault_store().unwrap();
            assert!(b.list_snapshots(IDS[0]).unwrap().is_empty());
            assert!(b.list_snapshots(IDS[1]).unwrap().is_empty());
            f.service
                .read()
                .unwrap()
                .unlock(&f.account, "password123")
                .unwrap();
            assert_eq!(current_records(&f).len(), count);
        }
    }
}

#[test]
fn rf027_completed_stale_import_keeps_result_but_suppresses_completion_callback() {
    for mode in ["lock", "switch", "reunlock"] {
        let f = Fixture::new();
        let other = other_account(&f);
        f.service
            .read()
            .unwrap()
            .unlock(&f.account, "password123")
            .unwrap();
        let expected_generation = generation(&f);
        let path = package(f.dir.path(), objects(), true, false, false);
        let source = std::fs::read(&path).unwrap();
        let base = root(&f);
        let service = f.service.clone();
        let account = f.account.clone();
        let second = other.clone();
        let calls = Arc::new(AtomicUsize::new(0));
        let result = run_paused(
            &f,
            request(&path),
            PauseAt::Complete,
            move || {
                let svc = service.try_write().expect("完成后等待期间不得持服务读锁");
                match mode {
                    "lock" => svc.lock(),
                    "switch" => svc.unlock(&second, "password456").unwrap(),
                    "reunlock" => {
                        svc.lock();
                        svc.unlock(&account, "password123").unwrap();
                    }
                    _ => unreachable!(),
                }
            },
            calls.clone(),
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(&result).unwrap(),
            json!({
                "sessionGeneration":expected_generation, "objectCount":2, "attachmentCount":2,
                "status":"complete", "templateCount":0, "snapshotCount":2,
                "preferencesImported":false, "attachmentFilesWritten":2,
                "failureStage":null, "errorCode":null
            })
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(counts(&f), (2, 2));
        assert_eq!(std::fs::read(path).unwrap(), source);
        assert_no_staging(&base);
        f.service
            .read()
            .unwrap()
            .unlock(&other, "password456")
            .unwrap();
        assert!(current_records(&f).is_empty());
        f.service
            .read()
            .unwrap()
            .unlock(&f.account, "password123")
            .unwrap();
        assert_eq!(linked_paths(&f).len(), 2);
        assert_attachment_bytes(&f, 2);
    }
}

#[test]
fn rf027_worker_panic_after_first_commit_returns_join_error_without_zero_commit_claim() {
    let f = Fixture::new();
    let path = package(f.dir.path(), objects(), true, false, false);
    let source = std::fs::read(&path).unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let progress: Progress = Arc::new(|pct| {
        // RF021：准备进度40没有写入；附件进度90发生在完整数据库批次提交后。
        if pct == 90 {
            panic!("RF027_SYNTHETIC_PRIVATE_PANIC_MARKER");
        }
    });
    let error = run(&f, request(&path), Some(progress), calls.clone()).unwrap_err();
    assert_eq!(error, "导入任务执行失败");
    assert!(!error.contains("RF027_SYNTHETIC_PRIVATE_PANIC_MARKER"));
    assert_eq!(counts(&f), (2, 2), "JoinError 发生前已提交，不能伪装零写入");
    assert!(f.vault.load_object(IDS[0]).unwrap().is_some());
    assert!(f.vault.load_object(IDS[1]).unwrap().is_some());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(attachment_files(&root(&f)).is_empty());
    assert_no_staging(&root(&f));
    assert_eq!(std::fs::read(path).unwrap(), source);
}

#[test]
fn rf027_owned_request_keeps_selection_strategy_locale_and_password_semantics() {
    for case in [
        "all",
        "none",
        "no_attachments",
        "one_attachment",
        "skip",
        "override",
        "password",
        "empty_password",
        "resolved_path",
    ] {
        let f = Fixture::new();
        let path = package(f.dir.path(), objects(), true, false, false);
        let mut req = request(&path);
        if case == "skip" || case == "override" {
            save_marker(&f, "local survives");
            req.strategy = ImportStrategy::SkipExisting;
        }
        let original = f.vault.load_object(IDS[0]).unwrap();
        match case {
            "none" => req.selections = Some(vec![]),
            "no_attachments" => req.selected_attachment_ids = Some(vec![]),
            "one_attachment" => req.selected_attachment_ids = Some(vec!["a1".into()]),
            "override" => {
                req.selections = Some(vec![
                    ImportSelection {
                        object_id: IDS[0].into(),
                        selected: true,
                    },
                    ImportSelection {
                        object_id: IDS[1].into(),
                        selected: false,
                    },
                ]);
                req.object_strategies
                    .insert(IDS[0].into(), ImportStrategy::KeepBoth);
                req.selected_attachment_ids = Some(vec!["a0".into()]);
                req.locale = "zh-CN".into();
            }
            "password" => req.password = "incorrect export password".into(),
            "empty_password" => req.password.clear(),
            _ => {}
        }
        let source = std::fs::read(&path).unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let result = if case == "resolved_path" {
            let invalid = f.dir.path().join("not-the-authorized-package.solosoul");
            assert!(!invalid.exists());
            req.source_path = invalid.to_str().unwrap().into();
            // 只检验 owned job 消费 resolver 返回路径，不冒称 AppHandle 白名单 E2E。
            let job = ImportJob::prepare(f.service.clone(), &f.account, req, None, |input| {
                assert_eq!(Path::new(input), invalid.as_path());
                path.canonicalize().map_err(|e| e.to_string())
            })
            .unwrap();
            let completed = calls.clone();
            let result = runtime()
                .block_on(run_import_job(job, ImportJob::run, move || {
                    completed.fetch_add(1, Ordering::SeqCst);
                }))
                .unwrap();
            assert!(!invalid.exists());
            result
        } else {
            run(&f, req, None, calls.clone()).unwrap()
        };
        if case == "password" || case == "empty_password" {
            assert_eq!(result.status, ImportStatus::NotCommitted);
            assert_eq!(result.failure_stage, Some(ImportStage::Preparation));
            assert_eq!(
                result.error_code.as_deref(),
                Some(if case == "password" {
                    "DECRYPT_FAILED"
                } else {
                    "PASSWORD_REQUIRED"
                })
            );
            assert_eq!(
                (
                    result.object_count,
                    result.snapshot_count,
                    result.attachment_count,
                    result.attachment_files_written
                ),
                (0, 0, 0, 0)
            );
            assert_eq!(counts(&f), (0, 0));
            assert_eq!(calls.load(Ordering::SeqCst), 0);
        } else {
            assert!(result.is_complete(), "{case}: {result:?}");
            let (objects, attachments) = match case {
                "none" => (0, 0),
                "no_attachments" => (2, 0),
                "one_attachment" => (2, 1),
                "skip" | "override" => (1, 1),
                _ => (2, 2),
            };
            assert_eq!(result.object_count, objects, "{case}");
            assert_eq!(result.snapshot_count, objects, "{case}");
            assert_eq!(result.attachment_count, attachments, "{case}");
            assert_eq!(result.attachment_files_written, attachments, "{case}");
            assert_eq!(linked_paths(&f).len(), attachments, "{case}");
            assert_attachment_bytes(&f, attachments);
            assert_eq!(
                calls.load(Ordering::SeqCst),
                1,
                "空选择仍保持既有 complete 结果"
            );
        }
        if case == "skip" || case == "override" {
            assert_eq!(
                serde_json::to_value(f.vault.load_object(IDS[0]).unwrap()).unwrap(),
                serde_json::to_value(&original).unwrap()
            );
        }
        if case == "override" {
            let records = current_records(&f);
            assert_eq!(records.len(), 2);
            let copy = records.iter().find(|record| record.id != IDS[0]).unwrap();
            assert_eq!(copy.name, "Synthetic（导入）");
            assert!(Uuid::parse_str(&copy.id).is_ok());
            assert!(f.vault.load_object(IDS[1]).unwrap().is_none());
            assert_eq!(copy.properties["__attachments"][0]["objectId"], copy.id);
            assert_eq!(f.vault.list_snapshots(&copy.id).unwrap().len(), 1);
        }
        if case == "one_attachment" {
            let first = f.vault.load_object(IDS[0]).unwrap().unwrap();
            let second = f.vault.load_object(IDS[1]).unwrap().unwrap();
            assert!(first.properties["__attachments"][0]["vaultPath"].is_null());
            assert!(Path::new(
                second.properties["__attachments"][0]["vaultPath"]
                    .as_str()
                    .unwrap()
            )
            .is_file());
        }
        assert_eq!(std::fs::read(path).unwrap(), source);
        assert_no_staging(&root(&f));
    }
}

#[test]
fn rf027_completed_result_survives_poisoned_service_without_completion_callback() {
    let f = Fixture::new();
    let path = package(f.dir.path(), objects(), true, false, false);
    let source = std::fs::read(&path).unwrap();
    let base = root(&f);
    let calls = Arc::new(AtomicUsize::new(0));
    let callback_calls = calls.clone();
    let expected = Arc::new(Mutex::new(None));
    let captured = expected.clone();
    let service = f.service.clone();
    let result = runtime()
        .block_on(run_import_job(
            prepare(&f, request(&path), None),
            move |job| {
                let result = job.run()?;
                assert!(result.is_complete());
                *captured.lock().unwrap() = Some(serde_json::to_value(&result).unwrap());
                let poisoned = std::thread::spawn(move || {
                    let _guard = service.write().unwrap();
                    panic!("RF027_SYNTHETIC_SERVICE_POISON");
                });
                assert!(poisoned.join().is_err());
                Ok(result)
            },
            move || {
                callback_calls.fetch_add(1, Ordering::SeqCst);
            },
        ))
        .unwrap();
    assert!(f.service.is_poisoned());
    assert_eq!(
        serde_json::to_value(&result).unwrap(),
        expected.lock().unwrap().clone().unwrap()
    );
    assert_eq!(result.status, ImportStatus::Complete);
    assert_eq!(
        (
            result.object_count,
            result.snapshot_count,
            result.attachment_count,
            result.attachment_files_written
        ),
        (2, 2, 2, 2)
    );
    assert_eq!(counts(&f), (2, 2));
    // 仅外层服务 RwLock 损坏，不把它误认为已提交 Vault 数据损坏或导入回滚。
    assert!(f.vault.load_object(IDS[0]).unwrap().is_some());
    assert!(f.vault.load_object(IDS[1]).unwrap().is_some());
    assert_eq!(attachment_files(&base).len(), 2);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(std::fs::read(path).unwrap(), source);
    assert_no_staging(&base);
}
