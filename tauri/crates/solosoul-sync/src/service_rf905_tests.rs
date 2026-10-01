use super::*;
use crate::noise::NoiseKeys;
use solosoul_core::import_activity::begin_owned_root_maintenance;
use solosoul_vault::root_owner::VaultRootOwner;
use solosoul_vault::{VaultConfig, VaultStore};
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::time::Duration;

async fn fixture() -> (
    tempfile::TempDir,
    Arc<VaultStore>,
    Arc<SyncManager>,
    Arc<SyncService>,
) {
    let dir = tempfile::tempdir().unwrap();
    let service_vault = VaultService::try_with_base_path(dir.path().to_path_buf()).unwrap();
    let account_path = dir.path().join("acct");
    std::fs::create_dir_all(&account_path).unwrap();
    let vault = Arc::new(
        VaultStore::open_owned(
            VaultConfig {
                path: account_path,
                account_id: "acct".into(),
                data_key: Some([0; 32]),
            },
            service_vault.root_owner(),
        )
        .unwrap(),
    );
    let manager = Arc::new(SyncManager::new(
        "node-rf905".into(),
        "acct".into(),
        NoiseKeys::generate(),
        vault.clone(),
        "127.0.0.1:0",
    ));
    manager.start().await.unwrap();
    let service = Arc::new(SyncService::new(Arc::new(RwLock::new(service_vault))));
    *service.manager.lock().await = Some(manager.clone());
    (dir, vault, manager, service)
}

async fn blocking_database_worker(
    vault: &Arc<VaultStore>,
    manager: &SyncManager,
) -> mpsc::Sender<()> {
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let body_vault = vault.clone();
    let receiver = manager
        .spawn_worker_for_test(move || {
            let _ = entered_tx.send(());
            release_rx.recv_timeout(Duration::from_secs(10)).unwrap();
            body_vault.set_sync_node_id("rf905-after-release").unwrap();
        })
        .unwrap();
    drop(receiver);
    entered_rx.await.unwrap();
    release_tx
}

async fn wait_for_stopping(service: &SyncService) {
    tokio::time::timeout(Duration::from_secs(10), async {
        while service.stopping.lock().unwrap().is_none() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn disable_waits_real_worker_while_status_queries_remain_reachable() {
    let (_dir, vault, manager, service) = fixture().await;
    let release = blocking_database_worker(&vault, &manager).await;
    let disabling_service = service.clone();
    let disable = tokio::spawn(async move { disabling_service.disable_and_wait().await });
    wait_for_stopping(&service).await;
    assert!(
        !tokio::time::timeout(Duration::from_secs(2), service.is_enabled())
            .await
            .unwrap()
    );
    assert!(
        !disable.is_finished(),
        "disable success cannot precede the real worker write/drop"
    );
    assert_eq!(
        begin_owned_root_maintenance(vault.root_owner())
            .err()
            .unwrap(),
        "IMPORT_OPERATIONS_ACTIVE"
    );
    release.send(()).unwrap();
    disable.await.unwrap().unwrap();
    assert_eq!(
        vault.get_sync_node_id().unwrap().as_deref(),
        Some("rf905-after-release")
    );
    assert_eq!(manager.active_sessions_counter().load(Ordering::SeqCst), 0);
    drop(begin_owned_root_maintenance(vault.root_owner()).unwrap());
}

#[tokio::test]
async fn cancelled_disable_awaiter_keeps_independent_cleanup_and_next_wait_joins_it() {
    let (_dir, vault, manager, service) = fixture().await;
    let release = blocking_database_worker(&vault, &manager).await;
    let disabling_service = service.clone();
    let disable = tokio::spawn(async move { disabling_service.enable(false).await });
    wait_for_stopping(&service).await;
    disable.abort();
    assert!(disable.await.unwrap_err().is_cancelled());
    assert!(!service.is_enabled().await);
    assert_eq!(
        begin_owned_root_maintenance(vault.root_owner())
            .err()
            .unwrap(),
        "IMPORT_OPERATIONS_ACTIVE"
    );
    let waiting_service = service.clone();
    let wait = tokio::spawn(async move { waiting_service.wait_until_stopped().await });
    tokio::task::yield_now().await;
    assert!(!wait.is_finished());
    release.send(()).unwrap();
    wait.await.unwrap().unwrap();
    assert_eq!(manager.active_sessions_counter().load(Ordering::SeqCst), 0);
    assert_eq!(
        vault.get_sync_node_id().unwrap().as_deref(),
        Some("rf905-after-release")
    );
}

#[tokio::test]
async fn reenable_cannot_overtake_cancelled_old_disable_cleanup() {
    let (_dir, vault, manager, service) = fixture().await;
    let release = blocking_database_worker(&vault, &manager).await;
    let disabling_service = service.clone();
    let disable = tokio::spawn(async move { disabling_service.enable(false).await });
    wait_for_stopping(&service).await;
    disable.abort();
    assert!(disable.await.unwrap_err().is_cancelled());
    let enabling_service = service.clone();
    let enable = tokio::spawn(async move { enabling_service.enable(true).await });
    tokio::task::yield_now().await;
    assert!(!enable.is_finished());
    assert_eq!(
        begin_owned_root_maintenance(vault.root_owner())
            .err()
            .unwrap(),
        "IMPORT_OPERATIONS_ACTIVE"
    );
    release.send(()).unwrap();
    // Fixture 没有解锁 Service 账户；只有旧任务 join 后才允许来到该真实前置校验。
    assert_eq!(enable.await.unwrap().unwrap_err(), "Vault is not unlocked");
    assert_eq!(
        vault.get_sync_node_id().unwrap().as_deref(),
        Some("rf905-after-release")
    );
}

#[tokio::test]
async fn service_drop_after_cancelled_disable_keeps_root_until_actual_worker_and_cleanup_exit() {
    let (dir, vault, manager, service) = fixture().await;
    let release = blocking_database_worker(&vault, &manager).await;
    let disabling_service = service.clone();
    let disable = tokio::spawn(async move { disabling_service.enable(false).await });
    wait_for_stopping(&service).await;
    let completion = service.stopping.lock().unwrap().clone().unwrap();
    disable.abort();
    assert!(disable.await.unwrap_err().is_cancelled());
    drop(service);
    drop(manager);
    drop(vault);
    assert!(VaultRootOwner::acquire(dir.path()).is_err());
    release.send(()).unwrap();
    completion.wait().await.unwrap();
    // completion 本身证明 cleanup 已先释放 Manager/Store，不能靠额外轮询弥补。
    drop(
        VaultRootOwner::acquire(dir.path()).expect("completed cleanup must release the old owner"),
    );
}

#[tokio::test]
async fn unpolled_outbound_caller_does_not_retain_manager_after_disable_join() {
    use std::future::Future;
    use std::net::TcpListener;
    use std::task::{Context, Poll, Waker};

    let (_dir, _vault, manager, service) = fixture().await;
    let old_manager = Arc::downgrade(&manager);
    let server = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let (accepted_tx, accepted_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let peer = tokio::task::spawn_blocking(move || {
        let (stream, _) = server.accept().unwrap();
        let _ = accepted_tx.send(());
        // 只控制 peer 关闭时机；真实 connect/Noise/worker registry 不替换。
        let _ = release_rx.recv_timeout(Duration::from_secs(10));
        drop(stream);
    });

    let mut sending = Box::pin(service.sync_with_device(addr));
    {
        // 只 poll 到派发，此后禁用/join 完成前后旧 caller 均不再被 poll。
        let mut context = Context::from_waker(Waker::noop());
        assert!(matches!(sending.as_mut().poll(&mut context), Poll::Pending));
    }
    accepted_rx.await.unwrap();
    drop(manager);
    let disabling_service = service.clone();
    let disable = tokio::spawn(async move { disabling_service.disable_and_wait().await });
    wait_for_stopping(&service).await;
    release_tx.send(()).unwrap();
    disable.await.unwrap().unwrap();
    peer.await.unwrap();
    assert!(
        old_manager.upgrade().is_none(),
        "unpolled receiver awaiter must not pin completed old manager"
    );
    assert!(
        sending.await.is_err(),
        "synthetic peer closed before finishing Noise handshake"
    );
}

fn sync_caller_audit_count(vault: &VaultStore) -> usize {
    vault
        .list_audit_log(1000)
        .unwrap()
        .iter()
        .filter(|entry| entry.action_type == "sync_with_device")
        .count()
}

#[tokio::test]
async fn successful_late_caller_cannot_audit_retired_or_new_account_and_old_root_releases() {
    use std::future::Future;
    use std::task::{Context, Poll, Waker};

    let dir = tempfile::tempdir().unwrap();
    let original_root = dir.path().join("original-root");
    let old_vault_service = VaultService::try_with_base_path(original_root.clone()).unwrap();
    let created = old_vault_service
        .create_account("original", "rf905-original-password", None)
        .unwrap();
    let original_account = created["id"].as_str().unwrap().to_string();
    old_vault_service
        .unlock(&original_account, "rf905-original-password")
        .unwrap();
    let old_owner = Arc::downgrade(&old_vault_service.root_owner());
    let old_vault = old_vault_service.get_vault_store().unwrap();
    let original_node = "0123456789abcdef0123456789abcdef";
    let peer_node = "fedcba9876543210fedcba9876543210";
    old_vault.set_sync_node_id(original_node).unwrap();
    let original_keys = NoiseKeys::generate();
    let peer_keys = NoiseKeys::generate();
    let peer_vault = Arc::new(
        VaultStore::open(VaultConfig {
            path: dir.path().join("peer-root"),
            account_id: original_account.clone(),
            data_key: Some([0; 32]),
        })
        .unwrap(),
    );
    peer_vault.set_sync_node_id(peer_node).unwrap();
    crate::shared::trust_peer_fallback(&old_vault, peer_node, true, Some(peer_keys.fingerprint()))
        .unwrap();
    crate::shared::trust_peer_fallback(
        &peer_vault,
        original_node,
        true,
        Some(original_keys.fingerprint()),
    )
    .unwrap();
    let original_manager = Arc::new(SyncManager::new(
        original_node.into(),
        original_account.clone(),
        original_keys,
        old_vault.clone(),
        "127.0.0.1:0",
    ));
    let peer_manager = Arc::new(SyncManager::new(
        peer_node.into(),
        original_account.clone(),
        peer_keys,
        peer_vault.clone(),
        "127.0.0.1:0",
    ));
    original_manager.start().await.unwrap();
    let peer_port = peer_manager.start().await.unwrap();
    let service = Arc::new(SyncService::new(Arc::new(RwLock::new(old_vault_service))));
    *service.manager.lock().await = Some(original_manager.clone());
    let old_manager = Arc::downgrade(&original_manager);
    let sessions = original_manager.active_sessions_counter();
    assert_eq!(sync_caller_audit_count(&old_vault), 0);

    let mut sending = Box::pin(service.sync_with_device(format!("127.0.0.1:{peer_port}")));
    {
        let mut context = Context::from_waker(Waker::noop());
        assert!(matches!(sending.as_mut().poll(&mut context), Poll::Pending));
    }
    // 真实双向信任 Noise + SQLite 同步完成，只让 caller 的 owned receiver 暂不被 poll。
    tokio::time::timeout(Duration::from_secs(15), async {
        while sessions.load(Ordering::SeqCst) != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    service.disable_and_wait().await.unwrap();
    peer_manager.stop_and_wait().await.unwrap();
    drop(original_manager);
    assert!(old_manager.upgrade().is_none());
    assert_eq!(sync_caller_audit_count(&old_vault), 0);

    let replacement_service =
        VaultService::try_with_base_path(dir.path().join("replacement-root")).unwrap();
    let replacement = replacement_service
        .create_account("replacement", "rf905-replacement-password", None)
        .unwrap();
    let replacement_account = replacement["id"].as_str().unwrap().to_string();
    replacement_service
        .unlock(&replacement_account, "rf905-replacement-password")
        .unwrap();
    let replacement_vault = replacement_service.get_vault_store().unwrap();
    let replacement_manager = Arc::new(SyncManager::new(
        "00112233445566778899aabbccddeeff".into(),
        replacement_account,
        NoiseKeys::generate(),
        replacement_vault.clone(),
        "127.0.0.1:0",
    ));
    *service.vault_service.write().unwrap() = replacement_service;
    *service.manager.lock().await = Some(replacement_manager);
    drop(old_vault);
    assert!(
        old_owner.upgrade().is_none(),
        "completed workers and late caller must release original native root"
    );
    drop(
        VaultRootOwner::acquire(&original_root)
            .expect("old root is available before polling late caller"),
    );
    assert_eq!(sync_caller_audit_count(&replacement_vault), 0);
    sending
        .await
        .expect("real loopback sync must have succeeded; no fabricated result");
    assert_eq!(
        sync_caller_audit_count(&replacement_vault),
        0,
        "old caller must not audit the replacement account"
    );

    let reopened = VaultService::try_with_base_path(original_root).unwrap();
    reopened.load_accounts();
    reopened
        .unlock(&original_account, "rf905-original-password")
        .unwrap();
    assert_eq!(
        sync_caller_audit_count(&reopened.get_vault_store().unwrap()),
        0,
        "retired manager is never upgraded just to audit"
    );
    service.disable_and_wait().await.unwrap();
}
