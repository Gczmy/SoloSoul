use super::*;
use crate::commands::export_import::tests::rf020::{objects, package, Fixture};
use solosoul_core::cloud_sync::{
    CloudObjectMeta, CloudResult, CloudSyncError, DeviceSnapshotMeta, LatestIndex,
};
use std::sync::Mutex;
use tokio::sync::Notify;

struct MemoryConnector {
    index: Vec<u8>,
    package: Vec<u8>,
    downloads: AtomicUsize,
}

#[async_trait::async_trait]
impl CloudConnector for MemoryConnector {
    fn connector_type(&self) -> &'static str {
        "test-memory"
    }
    async fn test_connection(&self) -> CloudResult<()> {
        Ok(())
    }
    async fn ensure_dir(&self, _: &str) -> CloudResult<()> {
        Ok(())
    }
    async fn upload(
        &self,
        _: &str,
        _: Pin<Box<dyn tokio::io::AsyncRead + Send>>,
        _: u64,
    ) -> CloudResult<(String, String)> {
        panic!("downstream test must not upload")
    }
    async fn download(
        &self,
        path: &str,
        mut writer: Pin<&mut (dyn tokio::io::AsyncWrite + Send + Unpin)>,
    ) -> CloudResult<u64> {
        let bytes = if path.ends_with("latest.json") {
            &self.index
        } else {
            self.downloads.fetch_add(1, Ordering::SeqCst);
            &self.package
        };
        writer.write_all(bytes).await?;
        Ok(bytes.len() as u64)
    }
    async fn list(&self, _: &str) -> CloudResult<Vec<CloudObjectMeta>> {
        Ok(vec![])
    }
    async fn delete(&self, _: &str) -> CloudResult<()> {
        panic!("downstream test must not delete remote data")
    }
    async fn head(&self, path: &str) -> CloudResult<CloudObjectMeta> {
        if !path.ends_with("latest.json") {
            return Err(CloudSyncError::NotFound(path.into()));
        }
        Ok(CloudObjectMeta {
            path: path.into(),
            size: self.index.len() as u64,
            modified: chrono::Utc::now(),
            etag: Some("etag".into()),
        })
    }
}

fn connector(f: &Fixture, with_attachments: bool) -> MemoryConnector {
    let path = package(f.dir.path(), objects(), with_attachments, false, false);
    let package = std::fs::read(path).unwrap();
    let mut index = LatestIndex::default();
    index.devices.insert(
        "remote-device".into(),
        DeviceSnapshotMeta {
            device_id: "remote-device".into(),
            device_name: None,
            hlc: "123-0".into(),
            remote_path: "/remote/snapshot.solosoul".into(),
            size: package.len() as u64,
            etag: "etag".into(),
            uploaded_at: chrono::Utc::now(),
        },
    );
    MemoryConnector {
        index: serde_json::to_vec(&index).unwrap(),
        package,
        downloads: AtomicUsize::new(0),
    }
}

fn context(f: &Fixture, events: Arc<Mutex<Vec<serde_json::Value>>>) -> CloudPreContext {
    let svc = f.service.read().unwrap();
    CloudPreContext {
        service: f.service.clone(),
        session: svc.capture_session(&f.account).unwrap(),
        account_id: f.account.clone(),
        config: solosoul_vault::CloudSyncConfig {
            snapshot_password: "export-password".into(),
            auto_import: true,
            ..Default::default()
        },
        base_path: svc.base_path().to_path_buf(),
        device_id: "local-device".into(),
        emit_event: Arc::new(move |_, payload| {
            events.lock().unwrap().push(payload);
            Ok(())
        }),
        barrier: None,
    }
}

/// 三个真实异步边界切换账户；无 sleep 判时，恢复后复用同一已下载包。
#[tokio::test]
async fn rf003_account_switch_at_download_import_and_waterline_preserves_source() {
    for stage in [
        CloudTestStage::Downloaded,
        CloudTestStage::BeforeImport,
        CloudTestStage::BeforeWaterline,
    ] {
        let f = Fixture::new();
        let connector = connector(&f, false);
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut pre = context(&f, events.clone());
        let reached = Arc::new(Notify::new());
        let resume = Arc::new(Notify::new());
        pre.barrier = Some({
            let reached = reached.clone();
            let resume = resume.clone();
            Arc::new(move |actual| {
                let reached = reached.clone();
                let resume = resume.clone();
                Box::pin(async move {
                    if actual == stage {
                        reached.notify_one();
                        resume.notified().await;
                    }
                })
            })
        });
        let key = format!("{APPLIED_KEY_PREFIX}remote-device");
        f.vault.set_sys_config(&key, "previous-a").unwrap();
        let switch = async {
            reached.notified().await;
            let svc = f.service.read().unwrap();
            svc.create_account_with_id("acc_rf003_b", "B", "password456", None)
                .unwrap();
            svc.get_vault_store()
                .unwrap()
                .set_sys_config(&key, "previous-b")
                .unwrap();
            resume.notify_one();
        };
        let (result, ()) = tokio::time::timeout(Duration::from_secs(30), async {
            tokio::join!(detect_and_fetch_incoming(&connector, &pre), switch)
        })
        .await
        .expect("cloud checkpoint was not reached/resumed");
        assert!(result.is_err());
        let incoming = pre
            .base_path
            .join(INCOMING_DIR)
            .join(&f.account)
            .join("remote-device/123-0.solosoul");
        assert!(incoming.is_file(), "stale round must retain its source");
        assert!(
            events.lock().unwrap().is_empty(),
            "stale account must not publish incoming/success"
        );
        assert!(pre
            .emit(
                "cloud-sync-status",
                serde_json::json!({"phase":"sync_complete"})
            )
            .is_err());
        {
            let svc = f.service.read().unwrap();
            let b = svc.get_vault_store().unwrap();
            assert!(b.load_object("rf020-0").unwrap().is_none());
            assert_eq!(
                b.get_sys_config(&key).unwrap().as_deref(),
                Some("previous-b")
            );
            assert!(b.get_cloud_sync_config(&f.account).unwrap().is_none());
            svc.unlock(&f.account, "password123").unwrap();
        }
        let fresh = context(&f, events.clone());
        assert_eq!(
            fresh
                .session
                .vault()
                .get_sys_config(&key)
                .unwrap()
                .as_deref(),
            Some("previous-a")
        );
        // 重进原账户重新捕获会话。旧 token 即使账户相同也继续拒绝。
        assert!(auto_import_one(&pre, incoming.to_str().unwrap())
            .await
            .is_err());
        detect_and_fetch_incoming(&connector, &fresh).await.unwrap();
        assert!(!incoming.exists());
        assert_eq!(
            fresh
                .session
                .vault()
                .get_sys_config(&key)
                .unwrap()
                .as_deref(),
            Some("123-0")
        );
        assert_eq!(connector.downloads.load(Ordering::SeqCst), 1);
        assert!(fresh
            .session
            .vault()
            .load_object("rf020-0")
            .unwrap()
            .is_some());
    }
}

#[tokio::test]
async fn rf003_complete_download_flushes_and_imports_attachments() {
    let f = Fixture::new();
    let connector = connector(&f, true);
    let pre = context(&f, Arc::new(Mutex::new(Vec::new())));
    detect_and_fetch_incoming(&connector, &pre).await.unwrap();
    let obj = pre.session.vault().load_object("rf020-0").unwrap().unwrap();
    let path = obj.properties["__attachments"][0]["vaultPath"]
        .as_str()
        .unwrap();
    assert!(Path::new(path).is_file());
    assert!(solosoul_core::attachment_crypto::is_encrypted_file(
        Path::new(path)
    ));
    let attachment_key = f
        .service
        .read()
        .unwrap()
        .attachment_key_for_session(&pre.session)
        .unwrap();
    let restored = f.dir.path().join("verified-attachment");
    solosoul_core::attachment_crypto::copy_decrypt_file(
        &attachment_key,
        Path::new(path),
        &restored,
    )
    .unwrap();
    assert_eq!(std::fs::read(restored).unwrap(), b"fixture");
    assert_eq!(
        pre.session
            .vault()
            .get_sys_config("cloud_applied:remote-device")
            .unwrap()
            .as_deref(),
        Some("123-0")
    );
}

#[tokio::test]
async fn rf003_stale_export_and_import_do_not_use_new_account() {
    let f = Fixture::new();
    let pre = context(&f, Arc::new(Mutex::new(Vec::new())));
    let input = package(f.dir.path(), objects(), false, false, false);
    f.service
        .read()
        .unwrap()
        .create_account_with_id("acc_rf003_b", "B", "password456", None)
        .unwrap();
    let output = f.dir.path().join("stale-export.solosoul");
    assert!(export_full_snapshot(&pre, &output).await.is_err());
    assert!(!output.exists());
    assert!(auto_import_one(&pre, input.to_str().unwrap())
        .await
        .is_err());
    assert!(input.exists());
    let b = f.service.read().unwrap().get_vault_store().unwrap();
    assert!(b.load_object("rf020-0").unwrap().is_none());
}

#[test]
fn rf003_switch_during_import_stops_later_records_and_attachment_publication() {
    // 90 位于 core 附件循环中：入口已校验，但文件尚未发布。
    for pause_at in [40, 90] {
        let f = Fixture::new();
        let session = f
            .service
            .read()
            .unwrap()
            .capture_session(&f.account)
            .unwrap();
        let input = package(f.dir.path(), objects(), true, false, false);
        let switched = Arc::new(AtomicBool::new(false));
        let callback = {
            let service = f.service.clone();
            let switched = switched.clone();
            Arc::new(move |progress| {
                if progress >= pause_at && !switched.swap(true, Ordering::SeqCst) {
                    service
                        .read()
                        .unwrap()
                        .create_account_with_id("acc_rf003_b", "B", "password456", None)
                        .unwrap();
                }
            })
        };
        let outcome = crate::commands::export_import::import_execute_for_session(
            &f.service.read().unwrap(),
            &session,
            input.to_string_lossy().into_owned(),
            zeroize::Zeroizing::new("export-password".into()),
            crate::commands::export_import::ImportStrategy::Overwrite,
            None,
            None,
            Default::default(),
            "en-US",
            Some(callback),
        )
        .unwrap();
        assert!(switched.load(Ordering::SeqCst));
        assert_eq!(
            outcome.status,
            crate::commands::export_import::ImportStatus::Partial
        );
        assert_eq!(outcome.object_count, if pause_at == 40 { 1 } else { 2 });
        assert_eq!(outcome.attachment_count, 0);
        assert_eq!(outcome.attachment_files_written, 0);
        let vault_root = f.service.read().unwrap().base_path().to_path_buf();
        for entry in std::fs::read_dir(&vault_root).unwrap() {
            assert!(
                !entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with("attachment-import-"),
                "expired import must remove unpublished attachment staging"
            );
        }
        assert!(!vault_root.join("attachments").exists());
        assert!(input.is_file());
        let b = f.service.read().unwrap().get_vault_store().unwrap();
        assert!(b.load_object("rf020-0").unwrap().is_none());
        assert!(b.load_object("rf020-1").unwrap().is_none());
    }
}

#[test]
fn rf003_events_identify_source_and_stop_after_same_account_reunlock() {
    let f = Fixture::new();
    let events = Arc::new(Mutex::new(Vec::new()));
    let pre = context(&f, events.clone());
    pre.emit(
        "cloud-sync-status",
        serde_json::json!({"phase":"sync_start"}),
    )
    .unwrap();
    assert_eq!(events.lock().unwrap()[0]["accountId"], f.account);
    assert_eq!(
        events.lock().unwrap()[0]["sessionGeneration"],
        pre.session.generation()
    );
    let svc = f.service.read().unwrap();
    let original_key = svc.attachment_key_for_session(&pre.session).unwrap();
    svc.lock();
    svc.unlock(&f.account, "password123").unwrap();
    assert!(svc.attachment_key_for_session(&pre.session).is_err());
    let fresh = svc.capture_session(&f.account).unwrap();
    assert_eq!(
        *svc.attachment_key_for_session(&fresh).unwrap(),
        *original_key
    );
    drop(svc);
    assert!(pre
        .emit(
            "cloud-sync-status",
            serde_json::json!({"phase":"sync_complete"})
        )
        .is_err());
    assert_eq!(events.lock().unwrap().len(), 1);
}

#[test]
fn rf003_round_finish_preserves_current_settings_and_rejects_replacement_session() {
    let f = Fixture::new();
    let events = Arc::new(Mutex::new(Vec::new()));
    let pre = context(&f, events.clone());
    let cfg = solosoul_vault::CloudSyncConfig {
        interval_secs: 120,
        enabled: true,
        ..Default::default()
    };
    f.vault.set_cloud_sync_config(&f.account, cfg).unwrap();
    pre.finish("manual").unwrap();
    let saved = f.vault.get_cloud_sync_config(&f.account).unwrap().unwrap();
    assert_eq!(saved.interval_secs, 120);
    assert!(saved.last_sync_at.is_some());
    assert_eq!(events.lock().unwrap().len(), 1);
    f.service
        .read()
        .unwrap()
        .create_account_with_id("acc_rf003_b", "B", "password456", None)
        .unwrap();
    assert!(pre.finish("manual").is_err());
    assert_eq!(events.lock().unwrap().len(), 1);
    let b = f.service.read().unwrap().get_vault_store().unwrap();
    assert!(b.get_cloud_sync_config(&f.account).unwrap().is_none());
    assert!(b.get_cloud_sync_config("acc_rf003_b").unwrap().is_none());
}
