use super::rf020::Fixture;
use super::*;
use crate::commands::export_import::export::{run_export_job, ExportJob};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

const OBJECT_ID: &str = "rf025-object";
const EXPORT_PASSWORD: &str = "rf025-export-password";
const WATCHDOG: Duration = Duration::from_secs(30);

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

fn request(target: &Path) -> ExportRequest {
    ExportRequest {
        scope: ExportScope {
            selected_page_ids: vec![],
            selected_object_ids: vec![OBJECT_ID.into()],
            selected_tags: vec![],
            include_attachments: false,
            selected_attachment_ids: vec![],
            include_preferences: false,
            include_behavioral: false,
            include_all: false,
        },
        password: EXPORT_PASSWORD.into(),
        password_hint: None,
        save_path: target.to_string_lossy().into_owned(),
    }
}

fn prepare(f: &Fixture, req: ExportRequest) -> ExportJob {
    // 仅接受本夹具已准备的临时输出；不冒称覆盖 AppHandle 的路径授权分支。
    let exports = f.dir.path().join("exports");
    ExportJob::prepare(f.service.clone(), &f.account, req, move |path| {
        let path = Path::new(path);
        assert_eq!(path.parent(), Some(exports.as_path()));
        Ok(path.to_string_lossy().into_owned())
    })
    .unwrap()
}

fn save_object(f: &Fixture, account: &str, marker: &str) {
    let svc = f.service.read().unwrap();
    assert_eq!(svc.get_current_account().as_deref(), Some(account));
    svc.get_vault_store()
        .unwrap()
        .save_object(&ObjectRecord {
            id: OBJECT_ID.into(),
            account_id: account.into(),
            type_id: "note".into(),
            section_type: "identity".into(),
            name: format!("合成对象 {marker}"),
            properties: json!({"value": marker, "empty": "", "flag": false}),
            created_at: "2026-09-27T00:00:00Z".into(),
            updated_at: "2026-09-27T00:00:00Z".into(),
            version: 1,
            ..Default::default()
        })
        .unwrap();
}

fn current_object(f: &Fixture) -> ObjectRecord {
    f.service
        .read()
        .unwrap()
        .get_vault_store()
        .unwrap()
        .load_object(OBJECT_ID)
        .unwrap()
        .unwrap()
}

fn successful_exports(f: &Fixture) -> usize {
    f.service
        .read()
        .unwrap()
        .get_vault_store()
        .unwrap()
        .list_audit_log(100)
        .unwrap()
        .into_iter()
        .filter(|entry| entry.action_type == "export_execute")
        .count()
}

fn directory_bytes(target: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    std::fs::read_dir(target.parent().unwrap())
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            let bytes = std::fs::read(&path).expect("输出目录不能留下临时子目录");
            (path, bytes)
        })
        .collect()
}

fn payload_temps(f: &Fixture) -> BTreeSet<PathBuf> {
    let base = f.service.read().unwrap().base_path().to_path_buf();
    std::fs::read_dir(base)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("solosoul-export-tmp-")
        })
        .collect()
}

fn existing_package(f: &Fixture) -> (PathBuf, ExportRequest) {
    save_object(f, &f.account, "original");
    let directory = f.dir.path().join("exports");
    std::fs::create_dir(&directory).unwrap();
    std::fs::write(directory.join("keep.txt"), b"unrelated synthetic file").unwrap();
    let target = directory.join("existing.solosoul");
    let req = request(&target);
    execute_export_core(
        &f.service.read().unwrap(),
        &f.account,
        &req,
        target.to_str().unwrap(),
    )
    .unwrap();
    let mut zip = ZipArchive::new(File::open(&target).unwrap()).unwrap();
    assert!(zip.by_name("payload.enc").unwrap().size() > 0);
    (target, req)
}

struct ReleaseOnDrop(Option<mpsc::Sender<()>>);

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        if let Some(sender) = self.0.take() {
            let _ = sender.send(());
        }
    }
}

/// 生产调度器若回归为 inline 执行，异步任务无法在 release 前报告推进。
/// watchdog 仅防挂死；所有失败路径仍释放 blocking worker，超时不会变成成功证据。
fn run_while_paused(
    job: ExportJob,
    while_paused: impl FnOnce() + Send + 'static,
) -> Result<String, String> {
    let (entered_tx, entered_rx) = mpsc::channel();
    let (async_entered_tx, async_entered_rx) = tokio::sync::oneshot::channel();
    let (progressed_tx, progressed_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let coordinator = std::thread::spawn(move || -> Result<(), String> {
        let release = ReleaseOnDrop(Some(release_tx));
        let worker_thread = entered_rx
            .recv_timeout(WATCHDOG)
            .map_err(|e| format!("导出 worker 没有进入屏障: {e}"))?;
        let async_thread = progressed_rx
            .recv_timeout(WATCHDOG)
            .map_err(|e| format!("blocking worker 暂停时异步任务没有推进: {e}"))?;
        assert_ne!(worker_thread, async_thread);
        while_paused();
        drop(release);
        Ok(())
    });
    let (result, progress_result) = runtime().block_on(async move {
        let independent_task = tokio::spawn(async move {
            async_entered_rx.await.map_err(|e| e.to_string())?;
            progressed_tx
                .send(std::thread::current().id())
                .map_err(|e| e.to_string())
        });
        let result = run_export_job(move || {
            entered_tx
                .send(std::thread::current().id())
                .map_err(|e| e.to_string())?;
            async_entered_tx
                .send(())
                .map_err(|_| "异步推进接收端已关闭".to_string())?;
            release_rx.recv().map_err(|e| e.to_string())?;
            job.run()
        })
        .await;
        (result, independent_task.await)
    });
    coordinator.join().expect("屏障协调线程不能 panic").unwrap();
    progress_result.unwrap().unwrap();
    result
}

#[test]
fn rf025_blocking_export_advances_current_thread_runtime_and_imports_output() {
    let f = Fixture::new();
    let (target, req) = existing_package(&f);
    let before = directory_bytes(&target);
    let previous_audits = successful_exports(&f);
    let previous_temps = payload_temps(&f);
    save_object(&f, &f.account, "updated / Unicode 🌍");
    let expected = current_object(&f);
    let job = prepare(&f, req);

    let result = run_while_paused(job, || {}).unwrap();
    assert_eq!(result, target.to_str().unwrap());
    let after = directory_bytes(&target);
    assert_eq!(
        after.keys().collect::<Vec<_>>(),
        before.keys().collect::<Vec<_>>()
    );
    assert_ne!(after[&target], before[&target]);
    assert_eq!(
        after[&target.parent().unwrap().join("keep.txt")],
        before[&target.parent().unwrap().join("keep.txt")]
    );
    assert_eq!(payload_temps(&f), previous_temps);
    assert_eq!(successful_exports(&f), previous_audits + 1);

    let restored = Fixture::new();
    let svc = restored.service.read().unwrap();
    let session = svc.capture_session(&restored.account).unwrap();
    let result = import_execute_for_session(
        &svc,
        &session,
        result,
        Zeroizing::new(EXPORT_PASSWORD.into()),
        ImportStrategy::Overwrite,
        None,
        None,
        HashMap::new(),
        "en-US",
        None,
    )
    .unwrap();
    assert_eq!(result.status, ImportStatus::Complete, "{result:?}");
    assert_eq!(result.object_count, 1);
    let actual = session.vault().load_object(OBJECT_ID).unwrap().unwrap();
    assert_eq!(actual.account_id, restored.account);
    assert_eq!(actual.name, expected.name);
    assert_eq!(actual.properties, expected.properties);
}

#[test]
fn rf025_queued_export_rejects_lock_switch_and_same_account_reunlock() {
    for mode in ["lock", "switch", "reunlock"] {
        let f = Fixture::new();
        let second_account = format!("acc_{}", Uuid::new_v4().simple());
        f.service
            .read()
            .unwrap()
            .create_account_with_id(&second_account, "Other", "password456", None)
            .unwrap();
        save_object(&f, &second_account, "other account data");
        let second_before = serde_json::to_value(current_object(&f)).unwrap();
        let second_audits = successful_exports(&f);
        f.service
            .read()
            .unwrap()
            .unlock(&f.account, "password123")
            .unwrap();
        let (target, req) = existing_package(&f);
        let first_before = serde_json::to_value(current_object(&f)).unwrap();
        let before = directory_bytes(&target);
        let previous_temps = payload_temps(&f);
        let first_audits = successful_exports(&f);
        let job = prepare(&f, req);
        let service = f.service.clone();
        let first_id = f.account.clone();
        let second_id = second_account.clone();

        let error = run_while_paused(job, move || {
            let svc = service.read().unwrap();
            match mode {
                "lock" => svc.lock(),
                "switch" => svc.unlock(&second_id, "password456").unwrap(),
                "reunlock" => {
                    svc.lock();
                    svc.unlock(&first_id, "password123").unwrap();
                }
                _ => unreachable!(),
            }
        })
        .unwrap_err();
        assert_eq!(error, "Vault session is no longer current", "{mode}");
        assert_eq!(directory_bytes(&target), before, "{mode}");
        assert_eq!(payload_temps(&f), previous_temps, "{mode}");

        // 必须重取当前 Vault；Fixture.vault 是原 Arc，切换后已被正常锁定。
        for (account, password, object, audit_count) in [
            (&f.account, "password123", &first_before, first_audits),
            (
                &second_account,
                "password456",
                &second_before,
                second_audits,
            ),
        ] {
            f.service.read().unwrap().unlock(account, password).unwrap();
            assert_eq!(
                serde_json::to_value(current_object(&f)).unwrap(),
                *object,
                "{mode}"
            );
            assert_eq!(successful_exports(&f), audit_count, "{mode}");
        }
    }
}

#[test]
fn rf025_async_export_preserves_preflight_errors_and_existing_package() {
    let f = Fixture::new();
    let (target, request) = existing_package(&f);
    let before = directory_bytes(&target);
    let previous_temps = payload_temps(&f);
    let audit_count = successful_exports(&f);
    for expected in [
        "PASSWORD_EMPTY",
        "SAME_AS_MASTER_PASSWORD",
        "NO_OBJECTS_SELECTED",
    ] {
        let mut req = request.clone();
        match expected {
            "PASSWORD_EMPTY" => req.password.clear(),
            "SAME_AS_MASTER_PASSWORD" => req.password = "password123".into(),
            "NO_OBJECTS_SELECTED" => req.scope.selected_object_ids = vec!["missing-object".into()],
            _ => unreachable!(),
        }
        let job = prepare(&f, req);
        let error = runtime()
            .block_on(run_export_job(move || job.run()))
            .unwrap_err();
        assert_eq!(error, export_err(expected));
        assert_eq!(directory_bytes(&target), before);
        assert_eq!(payload_temps(&f), previous_temps);
        assert_eq!(successful_exports(&f), audit_count);
    }
}

#[test]
fn rf025_worker_panic_becomes_join_error_without_publishing() {
    let f = Fixture::new();
    let (target, req) = existing_package(&f);
    let before = directory_bytes(&target);
    let previous_temps = payload_temps(&f);
    let audit_count = successful_exports(&f);
    let job = prepare(&f, req);
    let error = runtime()
        .block_on(run_export_job(move || {
            let _owned_job = job;
            panic!("RF025_SYNTHETIC_PRIVATE_PANIC_MARKER");
        }))
        .unwrap_err();
    assert_eq!(error, "导出任务执行失败");
    assert!(!error.contains("RF025_SYNTHETIC_PRIVATE_PANIC_MARKER"));
    assert_eq!(directory_bytes(&target), before);
    assert_eq!(payload_temps(&f), previous_temps);
    assert_eq!(successful_exports(&f), audit_count);
}

#[test]
fn rf025_prepare_propagates_path_rejection_before_export() {
    let f = Fixture::new();
    let (target, mut req) = existing_package(&f);
    let before = directory_bytes(&target);
    let audit_count = successful_exports(&f);
    // 若错误地先运行密码校验，空密码会遮盖 resolver 的路径拒绝。
    req.password.clear();
    let save_path = req.save_path.clone();
    let error = ExportJob::prepare(f.service.clone(), &f.account, req, |path| {
        assert_eq!(path, save_path);
        Err("synthetic export destination rejected".to_string())
    })
    .err()
    .expect("路径拒绝不能产生可执行 job");
    assert_eq!(error, "synthetic export destination rejected");
    assert_eq!(directory_bytes(&target), before);
    assert_eq!(successful_exports(&f), audit_count);
}
