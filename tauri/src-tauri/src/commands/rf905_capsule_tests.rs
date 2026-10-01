//! RF905：真实 VaultService / SQLite / root 准入；不 mock Store 或许可。
use super::{vault_handle_for_service, ActivityVaultHandle};
use solosoul_core::import_activity::{begin_owned_root_activity, begin_owned_root_maintenance};
use solosoul_core::VaultService;
use solosoul_vault::ObjectRecord;
use std::sync::{mpsc, Arc, RwLock};
use std::time::Duration;

struct Fixture {
    service: RwLock<VaultService>,
    _dir: tempfile::TempDir,
}
impl Fixture {
    fn new(unlocked: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let service = VaultService::try_with_base_path(dir.path().join("vault")).unwrap();
        if unlocked {
            service
                .create_account_with_id("acc_rf905_account", "Test", "password123", None)
                .unwrap();
        }
        Self {
            service: RwLock::new(service),
            _dir: dir,
        }
    }
    fn owner(&self) -> Arc<solosoul_vault::root_owner::VaultRootOwner> {
        self.service.read().unwrap().root_owner()
    }
    fn handle(&self) -> ActivityVaultHandle {
        vault_handle_for_service(&self.service).unwrap()
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

fn object() -> ObjectRecord {
    ObjectRecord {
        id: "rf905-object".into(),
        account_id: "acc_rf905_account".into(),
        name: "Capsule object".into(),
        type_id: "note".into(),
        section_type: "identity".into(),
        properties: serde_json::json!({"note": "private capsule regression"}),
        created_at: "2026-10-01T00:00:00Z".into(),
        updated_at: "2026-10-01T00:00:00Z".into(),
        ..Default::default()
    }
}

#[test]
fn rf905_capsule_clones_keep_real_store_and_admission_until_last_drop() {
    let f = Fixture::new(true);
    let owner = f.owner();
    let first = f.handle();
    let second = first.clone();
    first.save_object(&object()).unwrap();
    assert_eq!(
        second
            .as_ref()
            .load_object("rf905-object")
            .unwrap()
            .unwrap()
            .properties,
        object().properties
    );
    drop(first);
    assert_eq!(
        begin_owned_root_maintenance(Arc::clone(&owner))
            .err()
            .as_deref(),
        Some("IMPORT_OPERATIONS_ACTIVE")
    );
    drop(second);
    let maintenance = begin_owned_root_maintenance(owner).unwrap();
    drop(maintenance);
}

#[test]
fn rf905_maintenance_rejects_new_capsule_before_write_and_preserves_records() {
    let f = Fixture::new(true);
    {
        let handle = f.handle();
        handle.save_object(&object()).unwrap();
    }
    let store = f.service.read().unwrap().get_vault_store().unwrap();
    let maintenance = begin_owned_root_maintenance(f.owner()).unwrap();
    assert_eq!(
        vault_handle_for_service(&f.service).err().as_deref(),
        Some("IMPORT_DIRECTORY_BUSY")
    );
    assert_eq!(
        store
            .load_object("rf905-object")
            .unwrap()
            .unwrap()
            .properties,
        object().properties
    );
    drop(store);
    drop(maintenance);
    assert!(vault_handle_for_service(&f.service).is_ok());
}

#[test]
fn rf905_locked_handle_failure_releases_its_temporary_admission() {
    let f = Fixture::new(false);
    assert_eq!(
        vault_handle_for_service(&f.service).err().as_deref(),
        Some("Vault not unlocked")
    );
    let maintenance = begin_owned_root_maintenance(f.owner()).unwrap();
    drop(maintenance);
}

#[test]
fn rf905_arc_plugin_boundary_has_overlapping_native_worker_admission() {
    let f = Fixture::new(true);
    let owner = f.owner();
    let caller = f.handle();
    let raw = caller.store_arc();
    // 模拟真实 Native 接收方的准入 API；不能只抽出 Arc 就释放 caller。
    let native_activity = begin_owned_root_activity(raw.root_owner()).unwrap();
    drop(caller);
    assert_eq!(
        begin_owned_root_maintenance(Arc::clone(&owner))
            .err()
            .as_deref(),
        Some("IMPORT_OPERATIONS_ACTIVE")
    );
    raw.save_object(&object()).unwrap();
    drop(raw);
    drop(native_activity);
    let maintenance = begin_owned_root_maintenance(owner).unwrap();
    drop(maintenance);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rf905_cancelled_waiter_cannot_release_capsule_held_by_actual_blocking_writer() {
    let f = Fixture::new(true);
    let owner = f.owner();
    let handle = f.handle();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release = ReleaseOnDrop(Some(release_tx));
    let (done_tx, done_rx) = tokio::sync::oneshot::channel();
    let worker = tokio::task::spawn_blocking(move || {
        let capsule = handle;
        let _ = entered_tx.send(());
        release_rx.recv().unwrap();
        let result = capsule.save_object(&object());
        drop(capsule);
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
    tokio::time::timeout(Duration::from_secs(10), done_rx)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let maintenance = begin_owned_root_maintenance(owner).unwrap();
    let store = f.service.read().unwrap().get_vault_store().unwrap();
    assert_eq!(
        store
            .load_object("rf905-object")
            .unwrap()
            .unwrap()
            .properties,
        object().properties
    );
    drop(store);
    drop(maintenance);
}
