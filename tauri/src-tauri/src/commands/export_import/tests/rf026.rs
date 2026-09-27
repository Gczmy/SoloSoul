use super::rf020::{objects, package, Fixture};
use super::*;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use crate::commands::export_import::import::resolve_import_path;
use crate::commands::export_import::import::{run_preview_job, PreviewJob};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

const PASSWORD: &str = "export-password";
const WATCHDOG: Duration = Duration::from_secs(30);
const IDS: [&str; 4] = ["rf020-0", "rf020-1", "rf026-deleted", "rf026-new"];

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

fn prepare(f: &Fixture, path: &Path, password: &str) -> PreviewJob {
    PreviewJob::prepare(
        f.service.clone(),
        path.to_str().unwrap().into(),
        Zeroizing::new(password.into()),
        |path| Path::new(path).canonicalize().map_err(|e| e.to_string()),
    )
    .unwrap()
}

fn run(f: &Fixture, path: &Path, password: &str) -> Result<DecryptedImportPreview, String> {
    runtime().block_on(run_preview_job(prepare(f, path, password), PreviewJob::run))
}

fn payload() -> serde_json::Value {
    let mut value = objects();
    let rows = value["objects"].as_array_mut().unwrap();
    rows[0]["name"] = json!("同名合成对象 🌟");
    rows[0]["contract_type_id"] = json!("contract-person");
    rows[0]["template_id"] = json!("synthetic-template");
    rows[0]["template_type"] = json!("user");
    rows[0]["template_hash"] = json!("stored-hash");
    rows[0]["ignored_template_hash"] = json!("ignored-hash");
    rows[0]["parent_id"] = json!("synthetic-page");
    rows[0]["icon_name"] = json!("star");
    rows[0]["created_at"] = json!("2025-01-02T03:04:05Z");
    rows[0]["updated_at"] = json!("2026-02-03T04:05:06Z");
    rows[0]["sensitivity_level"] = json!("public");
    rows[0]["tags"] = json!(["合成", "binary"]);
    rows[0]["properties"]["values"] = json!([null, false, 0, "", "你好 / café"]);
    let mut removed = rows[0]["properties"]["__attachments"][0].clone();
    removed["id"] = json!("deleted-attachment");
    removed["deletedAt"] = json!("2026-09-26T00:00:00Z");
    rows[0]["properties"]["__attachments"]
        .as_array_mut()
        .unwrap()
        .push(removed);
    rows[1]["name"] = json!("包内原名称");
    for (id, name) in [(IDS[2], "回收站对象"), (IDS[3], "新对象")] {
        rows.push(json!({
            "id": id, "name": name, "type_id": "note",
            "section_type": "identity", "properties": {"marker": name}
        }));
    }
    value
}

fn save_local(f: &Fixture, id: &str, name: &str, deleted: bool) {
    let svc = f.service.read().unwrap();
    let account = svc.get_current_account().unwrap();
    svc.get_vault_store()
        .unwrap()
        .save_object(&ObjectRecord {
            id: id.into(),
            account_id: account,
            type_id: "note".into(),
            section_type: "identity".into(),
            name: name.into(),
            properties: json!({"local": name, "flag": false}),
            is_deleted: deleted,
            created_at: "2025-01-02T03:04:05Z".into(),
            updated_at: "2026-02-03T04:05:06Z".into(),
            version: 3,
            ..Default::default()
        })
        .unwrap();
}

fn database_version(f: &Fixture) -> i64 {
    f.db.query_row("PRAGMA data_version", [], |row| row.get(0))
        .unwrap()
}

fn current_snapshot(f: &Fixture) -> serde_json::Value {
    // 切换后 Fixture.vault 是已锁定的旧 Arc；始终重取当前账户的句柄。
    let svc = f.service.read().unwrap();
    let vault = svc.get_vault_store().unwrap();
    let rows: Vec<_> = IDS
        .iter()
        .map(|id| vault.load_object(id).unwrap())
        .collect();
    json!({"objects": rows, "audit_count": vault.list_audit_log(100).unwrap().len()})
}

fn import_temps(f: &Fixture) -> BTreeSet<PathBuf> {
    let root = f.service.read().unwrap().base_path().to_path_buf();
    // 同时检查 Service 根和各账户根，不能因检查错目录漏掉明文临时文件。
    let mut bases = vec![root.clone()];
    bases.extend(
        std::fs::read_dir(&root)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.is_dir()),
    );
    bases
        .into_iter()
        .flat_map(|base| {
            std::fs::read_dir(base)
                .unwrap()
                .map(|e| e.unwrap().path())
                .collect::<Vec<_>>()
        })
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("solosoul-import-tmp-")
        })
        .collect()
}

fn unrelated_temp(f: &Fixture) -> PathBuf {
    let dir = f
        .vault
        .base_path()
        .join("solosoul-import-tmp-other-request");
    std::fs::create_dir(&dir).unwrap();
    let marker = dir.join("keep.txt");
    std::fs::write(&marker, b"another synthetic request owns this directory").unwrap();
    marker
}

struct ReleaseOnDrop(Option<mpsc::Sender<()>>);

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        if let Some(tx) = self.0.take() {
            let _ = tx.send(());
        }
    }
}

/// 复用 RF025 的通道屏障思路，但实际调用预览命令使用的同一调度器。
/// watchdog 仅用于失败时释放 worker；判据是 release 前异步任务已推进且线程不同。
fn run_while_paused(
    job: PreviewJob,
    dto_ready: bool,
    while_paused: impl FnOnce() + Send + 'static,
) -> Result<DecryptedImportPreview, String> {
    let (entered_tx, entered_rx) = mpsc::channel();
    let (async_entered_tx, async_entered_rx) = tokio::sync::oneshot::channel();
    let (progressed_tx, progressed_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let coordinator = std::thread::spawn(move || -> Result<(), String> {
        let release = ReleaseOnDrop(Some(release_tx));
        let worker_thread = entered_rx
            .recv_timeout(WATCHDOG)
            .map_err(|e| format!("预览 worker 没有进入屏障: {e}"))?;
        let async_thread = progressed_rx
            .recv_timeout(WATCHDOG)
            .map_err(|e| format!("预览 worker 暂停时异步任务没有推进: {e}"))?;
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
        let result = run_preview_job(job, move |job| {
            let pause = move || -> Result<(), String> {
                entered_tx
                    .send(std::thread::current().id())
                    .map_err(|e| e.to_string())?;
                async_entered_tx
                    .send(())
                    .map_err(|_| "异步推进接收端已关闭".to_string())?;
                release_rx.recv().map_err(|e| e.to_string())
            };
            if dto_ready {
                let dto = job.run()?;
                assert!(!dto.objects.is_empty(), "屏障前必须已完成真实预览");
                pause()?;
                Ok(dto)
            } else {
                pause()?;
                job.run()
            }
        })
        .await;
        (result, independent_task.await)
    });
    coordinator.join().expect("屏障协调线程不能 panic").unwrap();
    progress_result.unwrap().unwrap();
    result
}

#[test]
fn rf026_real_preview_preserves_dto_conflicts_and_attachment_filter_without_writes() {
    let f = Fixture::new();
    save_local(&f, IDS[0], "同名合成对象 🌟", false);
    save_local(&f, IDS[1], "本地已改名", false);
    save_local(&f, IDS[2], "回收站对象", true);
    let payload = payload();
    let path = package(f.dir.path(), payload.clone(), true, false, true);
    let source = std::fs::read(&path).unwrap();
    let marker = unrelated_temp(&f);
    let temps = import_temps(&f);
    let version = database_version(&f);
    let before = current_snapshot(&f);
    let dto = run(&f, &path, PASSWORD).unwrap();

    assert_eq!(
        dto.objects
            .iter()
            .map(|o| o.id.as_str())
            .collect::<Vec<_>>(),
        IDS
    );
    let first = &dto.objects[0];
    assert_eq!(first.name, "同名合成对象 🌟");
    assert_eq!(first.collection_type, "note");
    assert_eq!(first.section_type, "identity");
    assert_eq!(first.contract_type_id.as_deref(), Some("contract-person"));
    assert_eq!(first.template_id.as_deref(), Some("synthetic-template"));
    assert_eq!(first.template_type.as_deref(), Some("user"));
    assert_eq!(first.template_hash.as_deref(), Some("stored-hash"));
    assert_eq!(first.ignored_template_hash.as_deref(), Some("ignored-hash"));
    assert_eq!(first.parent_id.as_deref(), Some("synthetic-page"));
    assert_eq!(first.icon_name, "star");
    assert_eq!(first.created_at, "2025-01-02T03:04:05Z");
    assert_eq!(first.updated_at, "2026-02-03T04:05:06Z");
    assert_eq!(first.sensitivity_level, "public");
    assert_eq!(first.tags, ["合成", "binary"]);
    assert_eq!(first.properties, payload["objects"][0]["properties"]);
    assert!(first.property_labels.is_none());
    assert!(first.has_attachments);
    assert!(!first.is_deleted);
    assert!(!dto.objects[2].has_attachments);
    assert_eq!(dto.conflicts.len(), 2);
    assert_eq!(dto.conflicts[0].object_id, IDS[0]);
    assert_eq!(dto.conflicts[0].kind, ConflictKind::Identical);
    assert_eq!(dto.conflicts[1].object_id, IDS[1]);
    assert_eq!(dto.conflicts[1].kind, ConflictKind::RenamedLocal);
    assert_eq!(dto.conflicts[1].imported_name, "包内原名称");
    assert_eq!(dto.conflicts[1].existing_name, "本地已改名");
    assert_eq!(
        serde_json::to_value(&dto.attachments).unwrap(),
        json!([
            {"id":"a0", "objectId":"rf020-0", "fileName":"sample.txt", "sizeBytes":7},
            {"id":"a1", "objectId":"rf020-1", "fileName":"sample.txt", "sizeBytes":7}
        ])
    );
    // 既有预览只检查 manifest 声明，不在此读取 preferences.enc。
    assert!(dto.has_preferences);
    assert!(!dto.has_audit_log);
    assert_eq!(database_version(&f), version, "预览不得提交任何数据库写入");
    assert_eq!(current_snapshot(&f), before);
    assert_eq!(std::fs::read(path).unwrap(), source);
    assert_eq!(import_temps(&f), temps);
    assert_eq!(
        std::fs::read(marker).unwrap(),
        b"another synthetic request owns this directory"
    );
}

#[test]
fn rf026_blocking_preview_allows_current_thread_runtime_to_advance() {
    let f = Fixture::new();
    let path = package(f.dir.path(), objects(), false, false, false);
    let version = database_version(&f);
    let temps = import_temps(&f);
    let dto = run_while_paused(prepare(&f, &path, PASSWORD), false, || {}).unwrap();
    assert_eq!(dto.objects.len(), 2);
    assert_eq!(dto.objects[0].id, "rf020-0");
    assert_eq!(dto.objects[0].name, "Synthetic");
    assert!(dto.conflicts.is_empty());
    assert!(
        dto.attachments.is_empty(),
        "manifest 未选择附件时不得展开元数据附件"
    );
    assert!(!dto.has_preferences);
    assert_eq!(database_version(&f), version);
    assert_eq!(import_temps(&f), temps);
}

#[test]
fn rf026_queued_and_ready_previews_reject_lock_switch_and_same_account_reunlock() {
    for dto_ready in [false, true] {
        for mode in ["lock", "switch", "reunlock"] {
            let f = Fixture::new();
            save_local(&f, IDS[0], "A local", false);
            let first_before = current_snapshot(&f);
            let other = format!("acc_{}", Uuid::new_v4().simple());
            f.service
                .read()
                .unwrap()
                .create_account_with_id(&other, "Other", "password456", None)
                .unwrap();
            save_local(&f, IDS[0], "B private data", false);
            let second_before = current_snapshot(&f);
            f.service
                .read()
                .unwrap()
                .unlock(&f.account, "password123")
                .unwrap();
            let path = package(f.dir.path(), objects(), false, false, false);
            let source = std::fs::read(&path).unwrap();
            let temps = import_temps(&f);
            let service = f.service.clone();
            let account = f.account.clone();
            let second = other.clone();
            let error = run_while_paused(prepare(&f, &path, PASSWORD), dto_ready, move || {
                // 捕获/worker 结束后不能持同步服务锁跨 await；try_write 不会挂住坏实现。
                let svc = service.try_write().expect("等待 worker 时不能持有服务锁");
                match mode {
                    "lock" => svc.lock(),
                    "switch" => svc.unlock(&second, "password456").unwrap(),
                    "reunlock" => {
                        svc.lock();
                        svc.unlock(&account, "password123").unwrap();
                    }
                    _ => unreachable!(),
                }
            })
            .unwrap_err();
            assert_eq!(
                error, "Vault session is no longer current",
                "ready={dto_ready}, mode={mode}"
            );
            assert!(!error.contains("B private data"));
            assert_eq!(std::fs::read(&path).unwrap(), source);
            assert_eq!(import_temps(&f), temps);
            for (account, password, before) in [
                (&f.account, "password123", &first_before),
                (&other, "password456", &second_before),
            ] {
                f.service.read().unwrap().unlock(account, password).unwrap();
                assert_eq!(
                    current_snapshot(&f),
                    *before,
                    "ready={dto_ready}, mode={mode}"
                );
            }
        }
    }
}

fn replace_zip_entry(path: &Path, name: &str, replacement: &[u8]) {
    let entries = {
        let mut zip = ZipArchive::new(File::open(path).unwrap()).unwrap();
        (0..zip.len())
            .map(|index| {
                let mut entry = zip.by_index(index).unwrap();
                let name = entry.name().to_string();
                let mut bytes = Vec::new();
                entry.read_to_end(&mut bytes).unwrap();
                (name, bytes)
            })
            .collect::<Vec<_>>()
    };
    assert!(entries.iter().any(|(entry, _)| entry == name));
    let mut zip = ZipWriter::new(File::create(path).unwrap());
    for (entry, bytes) in entries {
        zip.start_file(&entry, SimpleFileOptions::default())
            .unwrap();
        zip.write_all(if entry == name { replacement } else { &bytes })
            .unwrap();
    }
    zip.finish().unwrap();
}

#[test]
fn rf026_real_package_errors_preserve_errors_and_clean_only_owned_temps() {
    let f = Fixture::new();
    save_local(&f, IDS[0], "unchanged local", false);
    let marker = unrelated_temp(&f);
    let temps = import_temps(&f);
    let version = database_version(&f);
    let before = current_snapshot(&f);
    for case in ["password", "zip", "json", "kdf", "cipher"] {
        let path = package(f.dir.path(), objects(), false, false, false);
        match case {
            "zip" => std::fs::write(&path, b"not a ZIP package").unwrap(),
            "json" => {
                let manifest = read_manifest(path.to_str().unwrap()).unwrap();
                let salt = hex::decode(&manifest.salt_hex).unwrap();
                let key = derive_export_key_cfg(PASSWORD, &salt, &manifest.kdf_config()).unwrap();
                let plaintext = b"{\"objects\": [";
                let mut ciphertext = Vec::new();
                solosoul_crypto::cipher::encrypt_chunked_stream(
                    &key,
                    plaintext.len() as u64,
                    &mut std::io::Cursor::new(plaintext),
                    &mut ciphertext,
                )
                .unwrap();
                replace_zip_entry(&path, "payload.enc", &ciphertext);
            }
            "kdf" => {
                let mut manifest = read_manifest_json(path.to_str().unwrap()).unwrap();
                manifest["kdf"] =
                    json!({"algo":"argon2id", "memory_kb":0, "iterations":3, "parallelism":4});
                replace_zip_entry(
                    &path,
                    "manifest.json",
                    &serde_json::to_vec(&manifest).unwrap(),
                );
            }
            "cipher" => {
                let mut encrypted =
                    read_file_from_zip(path.to_str().unwrap(), "payload.enc").unwrap();
                encrypted.truncate(encrypted.len() - 1);
                replace_zip_entry(&path, "payload.enc", &encrypted);
            }
            "password" => {}
            _ => unreachable!(),
        }
        let source = std::fs::read(&path).unwrap();
        let password = if case == "password" {
            "wrong export password"
        } else {
            PASSWORD
        };
        let error = run(&f, &path, password).unwrap_err();
        match case {
            "password" | "cipher" => assert_eq!(error, import_err("DECRYPT_FAILED")),
            "zip" => assert_eq!(error, import_err("INVALID_PACKAGE")),
            "json" => assert!(error.starts_with("Invalid payload: "), "{error}"),
            "kdf" => assert_eq!(error, "manifest 的 kdf 参数非法"),
            _ => unreachable!(),
        }
        assert_eq!(std::fs::read(path).unwrap(), source, "{case}");
        assert_eq!(database_version(&f), version, "{case}");
        assert_eq!(current_snapshot(&f), before, "{case}");
        assert_eq!(import_temps(&f), temps, "{case}");
        assert_eq!(
            std::fs::read(&marker).unwrap(),
            b"another synthetic request owns this directory"
        );
    }
}

#[test]
fn rf026_real_worker_panic_returns_join_error_without_a_preview() {
    let f = Fixture::new();
    let path = package(f.dir.path(), objects(), false, false, false);
    let source = std::fs::read(&path).unwrap();
    let version = database_version(&f);
    let temps = import_temps(&f);
    let error = runtime()
        .block_on(run_preview_job(prepare(&f, &path, PASSWORD), |job| {
            let dto = job.run()?;
            assert_eq!(dto.objects.len(), 2);
            panic!("RF026_SYNTHETIC_PRIVATE_PANIC_MARKER");
        }))
        .unwrap_err();
    assert_eq!(error, "导入预览任务执行失败");
    assert!(!error.contains("RF026_SYNTHETIC_PRIVATE_PANIC_MARKER"));
    assert_eq!(database_version(&f), version);
    assert_eq!(std::fs::read(path).unwrap(), source);
    assert_eq!(import_temps(&f), temps);
}

#[test]
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn rf026_path_rejection_precedes_session_and_worker_uses_resolved_path() {
    let f = Fixture::new();
    let path = package(f.dir.path(), objects(), false, false, false);
    let app = tauri::test::mock_app();
    let traversal = f.dir.path().join("..").join("outside.solosoul");
    f.service.read().unwrap().lock();
    let error = match PreviewJob::prepare(
        f.service.clone(),
        traversal.to_str().unwrap().into(),
        Zeroizing::new(String::new()),
        |input| resolve_import_path(app.handle(), input),
    ) {
        Ok(_) => panic!("路径拒绝不得生成任务"),
        Err(error) => error,
    };
    assert_eq!(
        error, "Path traversal is not allowed",
        "路径授权必须先于会话或密码处理"
    );
    f.service
        .read()
        .unwrap()
        .unlock(&f.account, "password123")
        .unwrap();

    // cfg(test) 的 fs 白名单为文件系统根；这里只验证 resolver 规范化和消费，
    // 不声称覆盖桌面真实 Desktop/Documents/Downloads 的白名单边界。
    let dotted = f.dir.path().join(".").join("incoming.solosoul");
    let canonical = resolve_import_path(app.handle(), dotted.to_str().unwrap()).unwrap();
    assert_eq!(canonical, path.canonicalize().unwrap());
    let original = f.dir.path().join("not-the-resolved-file.solosoul");
    assert!(!original.exists());
    let job = PreviewJob::prepare(
        f.service.clone(),
        original.to_str().unwrap().into(),
        Zeroizing::new(PASSWORD.into()),
        |input| {
            assert_eq!(Path::new(input), original.as_path());
            resolve_import_path(app.handle(), dotted.to_str().unwrap())
        },
    )
    .unwrap();
    let version = database_version(&f);
    let temps = import_temps(&f);
    let dto = runtime()
        .block_on(run_preview_job(job, PreviewJob::run))
        .unwrap();
    assert_eq!(
        dto.objects.len(),
        2,
        "读取原始不存在路径会失败，必须消费 resolver 返回值"
    );
    assert!(!original.exists());
    assert_eq!(database_version(&f), version);
    assert_eq!(import_temps(&f), temps);
}
