//! RF905：真实原会话 / 云配置 / blocking writer / SQLite；不 mock 任务或准入。
use super::{save_cloud_sync_config_for_service, CloudConfigTask};
use solosoul_core::import_activity::begin_owned_root_maintenance;
use solosoul_core::VaultService;
use solosoul_vault::CloudSyncConfig;
use std::sync::{mpsc, Arc, RwLock};
use std::time::Duration;

const ACCOUNT: &str = "acc_rf905_cloud_config";

struct Fixture {
    service: Arc<RwLock<VaultService>>,
    _dir: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let service = VaultService::try_with_base_path(dir.path().join("vault")).unwrap();
        service
            .create_account_with_id(ACCOUNT, "Test", "password123", None)
            .unwrap();
        save_cloud_sync_config_for_service(&service, ACCOUNT, "password123", config("old"))
            .unwrap();
        Self {
            service: Arc::new(RwLock::new(service)),
            _dir: dir,
        }
    }
    fn task(&self) -> CloudConfigTask {
        CloudConfigTask::capture(&self.service.read().unwrap(), ACCOUNT).unwrap()
    }
}

fn config(marker: &str) -> CloudSyncConfig {
    CloudSyncConfig {
        connector_type: "webdav".into(),
        config_json: serde_json::json!({"marker": marker}),
        ..Default::default()
    }
}

struct ReleaseOnDrop(Option<mpsc::Sender<()>>);
impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        if let Some(sender) = self.0.take() {
            let _ = sender.send(());
        }
    }
}

#[test]
fn rf905_cloud_config_maintenance_rejects_original_task_before_dispatch_without_changing_profile() {
    let f = Fixture::new();
    let service = f.service.read().unwrap();
    let original = service.get_vault_store().unwrap();
    let before = original.load_profile(ACCOUNT).unwrap().unwrap();
    let maintenance = begin_owned_root_maintenance(original.root_owner()).unwrap();
    assert_eq!(
        CloudConfigTask::capture(&service, ACCOUNT).err().as_deref(),
        Some("IMPORT_DIRECTORY_BUSY")
    );
    let after = original.load_profile(ACCOUNT).unwrap().unwrap();
    assert_eq!(after.data, before.data);
    assert_eq!(after.version, before.version);
    drop(maintenance);
    let task = CloudConfigTask::capture(&service, ACCOUNT).unwrap();
    assert!(task.save(&service, "password123", config("new")).unwrap());
    assert_eq!(
        original
            .get_cloud_sync_config(ACCOUNT)
            .unwrap()
            .unwrap()
            .config_json["marker"],
        "new"
    );
}

#[test]
fn rf905_queued_cloud_save_and_delete_never_recapture_switched_current_session() {
    for save in [true, false] {
        let f = Fixture::new();
        let task = f.task();
        let original_path = task.session.vault().base_path().join("vault.db");
        let db = rusqlite::Connection::open(&original_path).unwrap();
        let before: Vec<u8> = db
            .query_row("SELECT data FROM profiles WHERE id=?1", [ACCOUNT], |row| {
                row.get(0)
            })
            .unwrap();
        let service = f.service.read().unwrap();
        service
            .create_account_with_id("acc_rf905_other", "Other", "password456", None)
            .unwrap();
        let result = if save {
            task.save(&service, "password123", config("late"))
        } else {
            task.delete(&service).map(|()| true)
        };
        assert_eq!(
            result.err().as_deref(),
            Some("Vault session is no longer current")
        );
        let after: Vec<u8> = db
            .query_row("SELECT data FROM profiles WHERE id=?1", [ACCOUNT], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(after, before);
        assert!(service
            .get_vault_store()
            .unwrap()
            .get_cloud_sync_config("acc_rf905_other")
            .unwrap()
            .is_none());
        let maintenance = begin_owned_root_maintenance(service.root_owner()).unwrap();
        drop(maintenance);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rf905_cancelled_cloud_config_waiter_keeps_actual_save_and_delete_worker_owned() {
    for save in [true, false] {
        let f = Fixture::new();
        let task = f.task();
        let owner = task.session.vault().root_owner();
        let service = Arc::clone(&f.service);
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let release = ReleaseOnDrop(Some(release_tx));
        let (done_tx, done_rx) = tokio::sync::oneshot::channel();
        let worker = tokio::task::spawn_blocking(move || {
            let _ = entered_tx.send(());
            release_rx.recv().unwrap();
            let result = {
                let service = service.read().unwrap();
                if save {
                    task.save(&service, "password123", config("completed"))
                } else {
                    task.delete(&service).map(|()| true)
                }
            };
            let _ = done_tx.send(result);
        });
        let waiter = tokio::spawn(worker);
        tokio::time::timeout(Duration::from_secs(10), entered_rx)
            .await
            .unwrap()
            .unwrap();
        waiter.abort();
        assert!(waiter.await.unwrap_err().is_cancelled());
        assert_eq!(
            begin_owned_root_maintenance(Arc::clone(&owner))
                .err()
                .as_deref(),
            Some("IMPORT_OPERATIONS_ACTIVE")
        );
        drop(release);
        assert!(tokio::time::timeout(Duration::from_secs(10), done_rx)
            .await
            .unwrap()
            .unwrap()
            .unwrap());
        let maintenance = begin_owned_root_maintenance(owner).unwrap();
        let vault = f.service.read().unwrap().get_vault_store().unwrap();
        let saved = vault.get_cloud_sync_config(ACCOUNT).unwrap();
        if save {
            assert_eq!(saved.unwrap().config_json["marker"], "completed");
        } else {
            assert!(saved.is_none());
        }
        drop(vault);
        drop(maintenance);
    }
}
