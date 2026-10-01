use super::*;
use solosoul_core::import_activity::begin_owned_root_maintenance;
use solosoul_vault::root_owner::VaultRootOwner;
use solosoul_vault::{VaultConfig, VaultStore};
use std::io::Read;
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::Duration;

fn fixture() -> (tempfile::TempDir, Arc<VaultStore>) {
    let dir = tempfile::tempdir().unwrap();
    let vault = Arc::new(
        VaultStore::open(VaultConfig {
            path: dir.path().to_path_buf(),
            account_id: "acct".into(),
            data_key: Some([0; 32]),
        })
        .unwrap(),
    );
    (dir, vault)
}

#[test]
fn queued_worker_is_registered_before_dispatch_and_receiver_cancel_keeps_activity() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    let (_dir, vault) = fixture();
    let workers = SyncWorkers::new();
    let counter = Arc::new(AtomicUsize::new(0));
    let called = Arc::new(AtomicBool::new(false));
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let blocker = runtime.spawn_blocking(move || {
        entered_tx.send(()).unwrap();
        release_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    });
    entered_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    runtime.block_on(async {
        let body_called = called.clone();
        let body_vault = vault.clone();
        let receiver = workers
            .spawn_session(vault.clone(), counter.clone(), move |_| {
                body_called.store(true, Ordering::SeqCst);
                body_vault.set_sync_node_id("queued-worker").unwrap();
            })
            .unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
        assert!(!called.load(Ordering::SeqCst));
        assert_eq!(
            begin_owned_root_maintenance(vault.root_owner())
                .err()
                .unwrap(),
            "IMPORT_OPERATIONS_ACTIVE"
        );
        drop(receiver);
        assert_eq!(
            begin_owned_root_maintenance(vault.root_owner())
                .err()
                .unwrap(),
            "IMPORT_OPERATIONS_ACTIVE"
        );
        workers.close();
        release_tx.send(()).unwrap();
        blocker.await.unwrap();
        workers.wait().await.unwrap();
        assert!(called.load(Ordering::SeqCst));
        assert_eq!(
            vault.get_sync_node_id().unwrap().as_deref(),
            Some("queued-worker")
        );
        assert_eq!(counter.load(Ordering::SeqCst), 0);
        drop(begin_owned_root_maintenance(vault.root_owner()).unwrap());
    });
}

#[tokio::test]
async fn cancelled_join_wait_restores_handles_and_does_not_report_idle() {
    let (_dir, vault) = fixture();
    let workers = SyncWorkers::new();
    let counter = Arc::new(AtomicUsize::new(0));
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let receiver = workers
        .spawn_session(vault.clone(), counter.clone(), move |_| {
            let _ = entered_tx.send(());
            release_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        })
        .unwrap();
    entered_rx.await.unwrap();
    workers.close();
    let waiter_workers = workers.clone();
    let waiter = tokio::spawn(async move { waiter_workers.wait().await });
    tokio::time::timeout(Duration::from_secs(10), async {
        while !workers.state.lock().unwrap().handles.is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    waiter.abort();
    assert!(waiter.await.unwrap_err().is_cancelled());
    assert_eq!(workers.state.lock().unwrap().handles.len(), 1);
    assert_eq!(
        begin_owned_root_maintenance(vault.root_owner())
            .err()
            .unwrap(),
        "IMPORT_OPERATIONS_ACTIVE"
    );
    release_tx.send(()).unwrap();
    receiver.await.unwrap();
    workers.wait().await.unwrap();
    assert_eq!(counter.load(Ordering::SeqCst), 0);
    drop(begin_owned_root_maintenance(vault.root_owner()).unwrap());
}

#[tokio::test]
async fn real_worker_keeps_root_owner_when_all_external_store_handles_are_dropped() {
    let (dir, vault) = fixture();
    let workers = SyncWorkers::new();
    let counter = Arc::new(AtomicUsize::new(0));
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let receiver = workers
        .spawn_session(vault.clone(), counter, move |_| {
            let _ = entered_tx.send(());
            release_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        })
        .unwrap();
    entered_rx.await.unwrap();
    workers.close();
    drop(vault);
    drop(receiver);
    assert!(VaultRootOwner::acquire(dir.path()).is_err());
    release_tx.send(()).unwrap();
    workers.wait().await.unwrap();
    drop(VaultRootOwner::acquire(dir.path()).unwrap());
}

#[tokio::test]
async fn maintenance_refuses_new_dispatch_without_running_or_counting_the_body() {
    let (_dir, vault) = fixture();
    let workers = SyncWorkers::new();
    let counter = Arc::new(AtomicUsize::new(0));
    let called = Arc::new(AtomicBool::new(false));
    let maintenance = begin_owned_root_maintenance(vault.root_owner()).unwrap();
    let body_called = called.clone();
    let result = workers.spawn_session(vault.clone(), counter.clone(), move |_| {
        body_called.store(true, Ordering::SeqCst);
    });
    assert_eq!(result.err().unwrap(), "IMPORT_DIRECTORY_BUSY");
    assert_eq!(counter.load(Ordering::SeqCst), 0);
    assert!(!called.load(Ordering::SeqCst));
    assert!(workers.state.lock().unwrap().handles.is_empty());
    drop(maintenance);
    workers.close();
    workers.wait().await.unwrap();
}

#[tokio::test]
async fn idle_background_pins_original_root_without_blocking_maintenance() {
    let (dir, vault) = fixture();
    let workers = SyncWorkers::new();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = mpsc::channel();
    workers
        .spawn_background(vault.clone(), move || {
            let _ = entered_tx.send(());
            release_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        })
        .unwrap();
    entered_rx.await.unwrap();
    drop(begin_owned_root_maintenance(vault.root_owner()).unwrap());
    drop(vault);
    assert!(VaultRootOwner::acquire(dir.path()).is_err());
    workers.close();
    release_tx.send(()).unwrap();
    workers.wait().await.unwrap();
    drop(VaultRootOwner::acquire(dir.path()).unwrap());
}

#[tokio::test]
async fn network_interrupt_unblocks_real_socket_but_wait_still_joins_worker() {
    let (_dir, vault) = fixture();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (mut server, _) = listener.accept().unwrap();
    let workers = SyncWorkers::new();
    let counter = Arc::new(AtomicUsize::new(0));
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let receiver = workers
        .spawn_session(vault.clone(), counter.clone(), move |permit| {
            permit.track_stream(&server).unwrap();
            let _ = entered_tx.send(());
            let mut byte = [0u8; 1];
            server.read(&mut byte)
        })
        .unwrap();
    entered_rx.await.unwrap();
    assert_eq!(
        begin_owned_root_maintenance(vault.root_owner())
            .err()
            .unwrap(),
        "IMPORT_OPERATIONS_ACTIVE"
    );
    workers.close();
    workers.interrupt_network();
    let read_result = receiver.await.unwrap();
    assert!(matches!(read_result, Ok(0) | Err(_)));
    workers.wait().await.unwrap();
    assert_eq!(counter.load(Ordering::SeqCst), 0);
    drop(client);
    drop(begin_owned_root_maintenance(vault.root_owner()).unwrap());
}

#[tokio::test]
async fn panicking_worker_releases_activity_and_stop_reports_join_failure_after_real_exit() {
    let (_dir, vault) = fixture();
    let workers = SyncWorkers::new();
    let counter = Arc::new(AtomicUsize::new(0));
    let receiver = workers
        .spawn_session::<(), _>(vault.clone(), counter.clone(), |_| {
            panic!("rf905 injected worker panic")
        })
        .unwrap();
    assert!(receiver.await.is_err());
    workers.close();
    assert!(workers
        .wait()
        .await
        .unwrap_err()
        .starts_with("__SYNC_ERR__:session_failed:"));
    assert_eq!(counter.load(Ordering::SeqCst), 0);
    drop(begin_owned_root_maintenance(vault.root_owner()).unwrap());
}

#[tokio::test]
async fn owned_account_stores_share_native_root_gate_instead_of_separate_account_path_gates() {
    let dir = tempfile::tempdir().unwrap();
    let owner = VaultRootOwner::acquire(dir.path()).unwrap();
    let mut stores = Vec::new();
    for account in ["account-a", "account-b"] {
        let path = dir.path().join(account);
        std::fs::create_dir_all(&path).unwrap();
        stores.push(Arc::new(
            VaultStore::open_owned(
                VaultConfig {
                    path,
                    account_id: account.into(),
                    data_key: Some([0; 32]),
                },
                owner.clone(),
            )
            .unwrap(),
        ));
    }
    let workers = SyncWorkers::new();
    let counter = Arc::new(AtomicUsize::new(0));
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let receiver = workers
        .spawn_session(stores[0].clone(), counter, move |_| {
            let _ = entered_tx.send(());
            release_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        })
        .unwrap();
    entered_rx.await.unwrap();
    assert_eq!(
        begin_owned_root_maintenance(stores[1].root_owner())
            .err()
            .unwrap(),
        "IMPORT_OPERATIONS_ACTIVE"
    );
    release_tx.send(()).unwrap();
    workers.close();
    receiver.await.unwrap();
    workers.wait().await.unwrap();
    drop(begin_owned_root_maintenance(stores[1].root_owner()).unwrap());
}
