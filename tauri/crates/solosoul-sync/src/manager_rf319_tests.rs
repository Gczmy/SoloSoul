use super::*;
use solosoul_core::import_activity::begin_owned_root_maintenance;
use solosoul_vault::{VaultConfig, VaultStore};
fn fixture(node: &str) -> (tempfile::TempDir, Arc<VaultStore>, Arc<SyncManager>) {
    let dir = tempfile::tempdir().unwrap();
    let vault = Arc::new(
        VaultStore::open(VaultConfig {
            path: dir.path().into(),
            account_id: "acct".into(),
            data_key: Some([0; 32]),
        })
        .unwrap(),
    );
    let manager = Arc::new(SyncManager::new(
        node.into(),
        "acct".into(),
        NoiseKeys::generate(),
        vault.clone(),
        "127.0.0.1:0",
    ));
    (dir, vault, manager)
}
#[tokio::test]
async fn rf319_managed_worker_classifies_actual_refusal_and_handshake_failure() {
    let (_dir, _vault, m) = fixture("rf319-client");
    m.start().await.unwrap();
    let reserved = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = reserved.local_addr().unwrap().to_string();
    drop(reserved);
    let failure = m
        .dispatch_sync_with_peer_typed(&addr)
        .unwrap()
        .await
        .unwrap()
        .unwrap_err();
    assert_eq!(failure.kind(), SyncFailureKind::ConnectRefused);
    let legacy = m.sync_with_peer(&addr).await.unwrap_err();
    assert!(legacy.starts_with("__SYNC_ERR__:connect_failed:"));
    let target = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = target.local_addr().unwrap().to_string();
    let peer = tokio::task::spawn_blocking(move || {
        let (stream, _) = target.accept().unwrap();
        drop(stream);
    });
    let failure = m
        .dispatch_sync_with_peer_typed(&addr)
        .unwrap()
        .await
        .unwrap()
        .unwrap_err();
    assert_eq!(failure.kind(), SyncFailureKind::HandshakeFailed);
    peer.await.unwrap();
    m.stop_and_wait().await.unwrap();
    assert_eq!(m.active_sessions.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn rf319_actual_pairing_handshake_keeps_peer_and_matching_sas() {
    let (_d1, _v1, a) = fixture("rf319-node-A");
    let (_d2, _v2, b) = fixture("rf319-node-B");
    let (tx, rx) = tokio::sync::oneshot::channel();
    let tx = Arc::new(std::sync::Mutex::new(Some(tx)));
    b.set_peer_callback(Some(Arc::new(move |info| {
        if let Some(tx) = tx.lock().unwrap().take() {
            let _ = tx.send(info);
        }
    })));
    a.start().await.unwrap();
    b.start().await.unwrap();
    let error = a
        .dispatch_sync_with_peer_typed(&format!("127.0.0.1:{}", b.listen_port()))
        .unwrap()
        .await
        .unwrap()
        .unwrap_err();
    assert_eq!(error.kind(), SyncFailureKind::PairingPending);
    let info = tokio::time::timeout(Duration::from_secs(10), rx)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(info.node_id, "rf319-node-A");
    let (peer, sas) = error.pairing().unwrap();
    assert_eq!(peer, "rf319-node-B");
    assert_eq!(sas, Some(info.sas_code.as_str()));
    assert_eq!(sas.unwrap().len(), 6);
    assert!(!format!("{error:?}").contains(sas.unwrap()));
    a.stop_and_wait().await.unwrap();
    b.stop_and_wait().await.unwrap();
}
#[tokio::test]
async fn rf319_typed_preflight_preserves_root_maintenance_and_legacy_guards() {
    let (_dir, vault, m) = fixture("rf319-node");
    let e = m.dispatch_sync_with_peer_typed("127.0.0.1:1").unwrap_err();
    assert_eq!(e.kind(), SyncFailureKind::NotRunning);
    assert_eq!(
        m.dispatch_sync_with_peer("127.0.0.1:1").unwrap_err(),
        "__SYNC_ERR__:not_running"
    );
    m.start().await.unwrap();
    let target = TcpListener::bind("127.0.0.1:0").unwrap();
    target.set_nonblocking(true).unwrap();
    let guard = begin_owned_root_maintenance(vault.root_owner()).unwrap();
    let e = m
        .dispatch_sync_with_peer_typed(&target.local_addr().unwrap().to_string())
        .unwrap_err();
    assert_eq!(e.kind(), SyncFailureKind::VaultBusy);
    assert_eq!(
        target.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert_eq!(m.active_sessions.load(Ordering::SeqCst), 0);
    drop(guard);
    m.stop_and_wait().await.unwrap();
}
