use super::*;
use solosoul_core::import_activity::begin_owned_root_maintenance;
use solosoul_vault::{VaultConfig, VaultStore};
use std::io::Read;
use std::net::TcpStream;

fn fixture(addr: &str) -> (tempfile::TempDir, Arc<VaultStore>, Arc<SyncManager>) {
    let dir = tempfile::tempdir().unwrap();
    let vault = Arc::new(
        VaultStore::open(VaultConfig {
            path: dir.path().to_path_buf(),
            account_id: "acct".into(),
            data_key: Some([0; 32]),
        })
        .unwrap(),
    );
    let manager = Arc::new(SyncManager::new(
        "node-rf905".into(),
        "acct".into(),
        NoiseKeys::generate(),
        vault.clone(),
        addr,
    ));
    (dir, vault, manager)
}

async fn wait_active(manager: &SyncManager) {
    tokio::time::timeout(Duration::from_secs(10), async {
        while manager.active_sessions.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn bind_failure_rolls_back_running_and_listener_port() {
    let occupied = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = occupied.local_addr().unwrap().to_string();
    let (_dir, _vault, manager) = fixture(&addr);
    assert!(manager
        .start()
        .await
        .unwrap_err()
        .starts_with("bind failed:"));
    assert!(!manager.running.load(Ordering::SeqCst));
    assert_eq!(manager.listen_port(), 0);
    drop(occupied);
    manager.start().await.unwrap();
    manager.stop_and_wait().await.unwrap();
    assert!(TcpListener::bind(&addr).is_ok());
}

#[tokio::test]
async fn mdns_registration_failure_joins_already_dispatched_accept_and_can_retry() {
    let reservation = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = reservation.local_addr().unwrap().to_string();
    drop(reservation);
    let (_dir, _vault, manager) = fixture(&addr);
    // 真正关闭 daemon，使 register 的真实发送通道失败；不 mock register/accept。
    let daemon = ServiceDaemon::new().unwrap();
    daemon
        .shutdown()
        .unwrap()
        .recv_timeout(Duration::from_secs(10))
        .unwrap();
    assert!(manager.start_with_daemon(daemon).await.is_err());
    assert!(!manager.running.load(Ordering::SeqCst));
    assert_eq!(manager.listen_port(), 0);
    let rebound = TcpListener::bind(&addr)
        .expect("failed startup must release accept listener before returning");
    drop(rebound);
    manager.start().await.unwrap();
    manager.stop_and_wait().await.unwrap();
}

#[tokio::test]
async fn idle_enabled_listener_allows_maintenance_and_maintenance_rejects_outbound_before_connect()
{
    let (_dir, vault, manager) = fixture("127.0.0.1:0");
    manager.start().await.unwrap();
    let target = TcpListener::bind("127.0.0.1:0").unwrap();
    target.set_nonblocking(true).unwrap();
    let maintenance = begin_owned_root_maintenance(vault.root_owner()).unwrap();
    assert_eq!(
        manager
            .sync_with_peer(&target.local_addr().unwrap().to_string())
            .await
            .unwrap_err(),
        "IMPORT_DIRECTORY_BUSY"
    );
    assert_eq!(
        target.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert_eq!(manager.active_sessions.load(Ordering::SeqCst), 0);
    drop(maintenance);
    manager.stop_and_wait().await.unwrap();
}

#[tokio::test]
async fn maintenance_rejects_real_inbound_connection_without_pairing_or_database_mutation() {
    let (_dir, vault, manager) = fixture("127.0.0.1:0");
    manager.start().await.unwrap();
    let pairing = Arc::new(AtomicUsize::new(0));
    let callback_pairing = pairing.clone();
    manager.set_peer_callback(Some(Arc::new(move |_| {
        callback_pairing.fetch_add(1, Ordering::SeqCst);
    })));
    let original_peers = vault.list_peers().unwrap();
    let maintenance = begin_owned_root_maintenance(vault.root_owner()).unwrap();
    let port = manager.listen_port();
    tokio::task::spawn_blocking(move || {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let mut byte = [0u8; 1];
        assert!(matches!(stream.read(&mut byte), Ok(0) | Err(_)));
    })
    .await
    .unwrap();
    assert_eq!(manager.active_sessions.load(Ordering::SeqCst), 0);
    assert_eq!(pairing.load(Ordering::SeqCst), 0);
    assert_eq!(vault.list_peers().unwrap().len(), original_peers.len());
    drop(maintenance);
    manager.stop_and_wait().await.unwrap();
}

#[tokio::test]
async fn inbound_socket_worker_blocks_maintenance_until_stop_really_joins() {
    let (_dir, vault, manager) = fixture("127.0.0.1:0");
    manager.start().await.unwrap();
    let client = TcpStream::connect(("127.0.0.1", manager.listen_port())).unwrap();
    wait_active(&manager).await;
    assert_eq!(
        begin_owned_root_maintenance(vault.root_owner())
            .err()
            .unwrap(),
        "IMPORT_OPERATIONS_ACTIVE"
    );
    manager
        .stop_and_wait_with_grace(Duration::ZERO)
        .await
        .unwrap();
    assert_eq!(manager.active_sessions.load(Ordering::SeqCst), 0);
    assert_eq!(manager.listen_port(), 0);
    drop(client);
    drop(begin_owned_root_maintenance(vault.root_owner()).unwrap());
}

#[tokio::test]
async fn cancelling_real_outbound_awaiter_does_not_release_worker_activity() {
    let (_dir, vault, manager) = fixture("127.0.0.1:0");
    manager.start().await.unwrap();
    let server = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let (accepted_tx, accepted_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let peer = tokio::task::spawn_blocking(move || {
        let (stream, _) = server.accept().unwrap();
        let _ = accepted_tx.send(());
        release_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        drop(stream);
    });
    let sending_manager = manager.clone();
    let sending = tokio::spawn(async move { sending_manager.sync_with_peer(&addr).await });
    accepted_rx.await.unwrap();
    wait_active(&manager).await;
    sending.abort();
    assert!(sending.await.unwrap_err().is_cancelled());
    assert_eq!(
        begin_owned_root_maintenance(vault.root_owner())
            .err()
            .unwrap(),
        "IMPORT_OPERATIONS_ACTIVE"
    );
    // 本例检验取消 awaiter 后仍由真实 outbound worker 持 Activity。
    // 上面的 ACTIVE 断言发生在对端仍被屏障阻塞时；之后明确释放对端，
    // 避免把 stop 的实际 join 等待当成必须早于 peer 的 10s 防挂起时限。
    // tracked socket shutdown 的行为由独立 network_interrupt 用例检验。
    release_tx.send(()).unwrap();
    manager
        .stop_and_wait_with_grace(Duration::ZERO)
        .await
        .unwrap();
    drop(begin_owned_root_maintenance(vault.root_owner()).unwrap());
    peer.await.unwrap();
}

#[tokio::test]
async fn completed_stop_releases_listener_before_same_manager_restart() {
    let (_dir, _vault, manager) = fixture("127.0.0.1:0");
    let first_port = manager.start().await.unwrap();
    manager.stop_and_wait().await.unwrap();
    let released = TcpListener::bind(("127.0.0.1", first_port)).unwrap();
    drop(released);
    assert_ne!(manager.start().await.unwrap(), 0);
    manager.stop_and_wait().await.unwrap();
}

#[tokio::test]
async fn short_shared_writes_are_rejected_by_same_maintenance_gate() {
    let (_dir, vault, manager) = fixture("127.0.0.1:0");
    let original_node = vault.get_sync_node_id().unwrap();
    let maintenance = begin_owned_root_maintenance(vault.root_owner()).unwrap();
    assert_eq!(
        crate::shared::get_or_create_sync_identity(&vault)
            .err()
            .unwrap(),
        "IMPORT_DIRECTORY_BUSY"
    );
    assert_eq!(
        manager.trust_peer("peer", true, Some("fp")).unwrap_err(),
        "IMPORT_DIRECTORY_BUSY"
    );
    assert_eq!(
        manager.forget_peer("peer").unwrap_err(),
        "IMPORT_DIRECTORY_BUSY"
    );
    crate::shared::audit_log(&vault, "rf905-denied", None, None);
    assert_eq!(vault.get_sync_node_id().unwrap(), original_node);
    assert!(vault.list_peers().unwrap().is_empty());
    drop(maintenance);
}

#[tokio::test]
async fn cancelled_direct_manager_stop_keeps_independent_join_and_restart_waits_for_it() {
    let (_dir, vault, manager) = fixture("127.0.0.1:0");
    manager.start().await.unwrap();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let body_vault = vault.clone();
    drop(
        manager
            .spawn_worker_for_test(move || {
                let _ = entered_tx.send(());
                release_rx.recv_timeout(Duration::from_secs(10)).unwrap();
                body_vault.set_sync_node_id("rf905-direct-stop").unwrap();
            })
            .unwrap(),
    );
    entered_rx.await.unwrap();
    let stopping_manager = manager.clone();
    let stop = tokio::spawn(async move { stopping_manager.stop_and_wait().await });
    tokio::time::timeout(Duration::from_secs(10), async {
        while manager.running.load(Ordering::SeqCst) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    stop.abort();
    assert!(stop.await.unwrap_err().is_cancelled());
    assert_eq!(
        begin_owned_root_maintenance(vault.root_owner())
            .err()
            .unwrap(),
        "IMPORT_OPERATIONS_ACTIVE"
    );
    let restarting_manager = manager.clone();
    let restart = tokio::spawn(async move { restarting_manager.start().await });
    tokio::task::yield_now().await;
    assert!(!restart.is_finished());
    release_tx.send(()).unwrap();
    assert_ne!(restart.await.unwrap().unwrap(), 0);
    assert_eq!(
        vault.get_sync_node_id().unwrap().as_deref(),
        Some("rf905-direct-stop")
    );
    drop(begin_owned_root_maintenance(vault.root_owner()).unwrap());
    manager.stop_and_wait().await.unwrap();
}
