//! RF213：真实 CLI 入口、localhost peer 与 Noise/配对协议；不以假 stop 证明回收。
use super::*;
use crate::app::AppPhase;
use crate::events::Event;
use crate::tasks::{TaskEvent, TaskEventKind};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::io::Read;
use std::net::{Ipv4Addr, Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tempfile::TempDir;

const ACCOUNT: &str = "acc_rf213_sync";
const REMOTE_NODE: &str = "node_rf213_remote";
// 超时只防测试死锁，须覆盖现有30秒 stop 宽限期与系统 socket 超时。
const WAIT: Duration = Duration::from_secs(100);

enum PeerMode {
    Stalled,
    Disconnect,
    Protocol {
        vault: Arc<solosoul_vault::VaultStore>,
        keys: NoiseKeys,
    },
}
struct Peer {
    address: String,
    entered: mpsc::Receiver<()>,
    stop: Arc<AtomicBool>,
    socket: Arc<Mutex<Option<TcpStream>>>,
    closed: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<(), String>>>,
}
impl Peer {
    fn new(mode: PeerMode) -> Self {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = listener.local_addr().unwrap().to_string();
        listener.set_nonblocking(true).unwrap();
        let (tx, entered) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let socket = Arc::new(Mutex::new(None::<TcpStream>));
        let closed = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let sockets = socket.clone();
        let closing = closed.clone();
        let worker = thread::spawn(move || {
            let deadline = Instant::now() + WAIT;
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        if stopping.load(Ordering::Acquire) {
                            return Ok(());
                        }
                        if Instant::now() >= deadline {
                            return Err("RF213 peer accept timeout".into());
                        }
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(e) => return Err(e.to_string()),
                }
            };
            // Windows accept 可能继承非阻塞状态；真实 SyncTransport 也显式切回阻塞。
            stream.set_nonblocking(false).unwrap();
            stream.set_read_timeout(Some(WAIT)).unwrap();
            *sockets.lock().unwrap() = Some(stream.try_clone().unwrap());
            match mode {
                PeerMode::Stalled | PeerMode::Disconnect => {
                    let mut byte = [0];
                    let n = stream.read(&mut byte).map_err(|e| e.to_string())?;
                    if n == 0 {
                        return Err("RF213 initiator sent no handshake".into());
                    }
                    tx.send(()).map_err(|e| e.to_string())?;
                    if matches!(mode, PeerMode::Disconnect) {
                        stream.shutdown(Shutdown::Both).map_err(|e| e.to_string())?;
                        sockets.lock().unwrap().take();
                    }
                    if matches!(mode, PeerMode::Stalled) {
                        let mut bytes = [0; 1024];
                        loop {
                            match stream.read(&mut bytes) {
                                Ok(0) => break,
                                Ok(_) => {}
                                Err(e)
                                    if matches!(
                                        e.kind(),
                                        std::io::ErrorKind::ConnectionReset
                                            | std::io::ErrorKind::ConnectionAborted
                                    ) =>
                                {
                                    break
                                }
                                Err(e) => return Err(e.to_string()),
                            }
                        }
                    }
                }
                PeerMode::Protocol { vault, keys } => {
                    tx.send(()).map_err(|e| e.to_string())?;
                    let addr = stream.peer_addr().unwrap().to_string();
                    let mut transport =
                        solosoul_sync::transport::SyncTransport::from_stream(stream);
                    let result = solosoul_sync::session::handle_inbound(
                        &mut transport,
                        REMOTE_NODE,
                        ACCOUNT,
                        &keys,
                        vault,
                        addr,
                        None,
                    );
                    // 响应端发送 pairing_pending，自己的返回值为 Peer not trusted。
                    if let Err(e) = result {
                        if e != "Peer not trusted" {
                            return Err(e);
                        }
                    }
                }
            }
            closing.store(true, Ordering::Release);
            Ok(())
        });
        Self {
            address,
            entered,
            stop,
            socket,
            closed,
            worker: Some(worker),
        }
    }
    fn wait_entered(&self) {
        self.entered
            .recv_timeout(WAIT)
            .expect("actual sync initiator must reach peer");
    }
    fn join(&mut self) {
        self.worker.take().unwrap().join().unwrap().unwrap();
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(socket) = self
            .socket
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
        {
            let _ = socket.shutdown(Shutdown::Both);
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

struct Fixture {
    app: App,
    peer: Option<Peer>,
    remote: Option<(Arc<VaultService>, TempDir)>,
    _dir: TempDir,
    _serial: MutexGuard<'static, ()>,
}
impl Fixture {
    fn new() -> Self {
        let serial = crate::VAULT_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let dir = TempDir::new().unwrap();
        let service = Arc::new(VaultService::with_base_path(dir.path().to_path_buf()));
        service
            .create_account_with_id(ACCOUNT, "RF213 synthetic local", crate::TEST_PASSWORD, None)
            .unwrap();
        let mut app = App::new(service).unwrap();
        app.phase = AppPhase::Home {
            account_id: ACCOUNT.into(),
        };
        app.i18n.set_locale("en-US");
        Self {
            app,
            peer: None,
            remote: None,
            _dir: dir,
            _serial: serial,
        }
    }
    fn start(&mut self, mode: PeerMode) -> TaskId {
        self.peer = Some(Peer::new(mode));
        let address = self.peer.as_ref().unwrap().address.clone();
        handle(&mut self.app, &["with", &address]).unwrap();
        assert_eq!(
            self.app.sync_tasks.len(),
            1,
            "command must dispatch without waiting on peer"
        );
        let id = *self.app.sync_tasks.keys().next().unwrap();
        self.peer.as_ref().unwrap().wait_entered();
        self.app.drain_task_events(32).unwrap();
        id
    }
    fn start_protocol(&mut self, trusted: bool) -> TaskId {
        let dir = TempDir::new().unwrap();
        let remote = Arc::new(VaultService::with_base_path(dir.path().to_path_buf()));
        remote
            .create_account_with_id(
                ACCOUNT,
                "RF213 synthetic remote",
                crate::TEST_PASSWORD,
                None,
            )
            .unwrap();
        let local_store = self.app.vault_service.get_vault_store().unwrap();
        let (local_node, local_keys) = sync_identity(&local_store);
        let remote_store = remote.get_vault_store().unwrap();
        let keys = NoiseKeys::generate();
        if trusted {
            let local = SyncManager::new(
                local_node.clone(),
                ACCOUNT.into(),
                local_keys.clone(),
                local_store,
                "127.0.0.1:0",
            );
            local
                .trust_peer(REMOTE_NODE, true, Some(&keys.fingerprint()))
                .unwrap();
            let responder = SyncManager::new(
                REMOTE_NODE.into(),
                ACCOUNT.into(),
                keys.clone(),
                remote_store.clone(),
                "127.0.0.1:0",
            );
            responder
                .trust_peer(&local_node, true, Some(&local_keys.fingerprint()))
                .unwrap();
        }
        self.remote = Some((remote, dir));
        self.start(PeerMode::Protocol {
            vault: remote_store,
            keys,
        })
    }
    fn finish(&mut self) {
        crate::util::shared_runtime().unwrap().block_on(async {
            tokio::time::timeout(WAIT, async {
                loop {
                    self.app.drain_task_events(32).unwrap();
                    if self.app.tasks.is_empty() {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            })
            .await
            .expect("actual sync and cleanup must finish");
        });
        assert!(self.app.sync_tasks.is_empty());
        let peer = self.peer.as_mut().unwrap();
        peer.join();
        assert!(
            peer.closed.load(Ordering::Acquire),
            "client cleanup must really close peer socket"
        );
    }
    fn completed_without_applying(&mut self) -> TaskEvent {
        crate::util::shared_runtime().unwrap().block_on(async {
            tokio::time::timeout(WAIT, async {
                loop {
                    for event in self.app.tasks.poll_events(32) {
                        if matches!(
                            &event.kind,
                            TaskEventKind::Completed(TaskOutput::SyncCompleted { .. })
                        ) {
                            return event;
                        }
                        self.app.handle_event(Event::Task(event)).unwrap();
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap()
        })
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.app.shutdown_tasks();
        self.peer.take();
    }
}

#[test]
fn rf213_real_stalled_peer_allows_keys_ticks_progress_and_cancel_waits_actual_socket_cleanup() {
    let mut f = Fixture::new();
    let id = f.start(PeerMode::Stalled);
    f.app
        .handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('z'),
            KeyModifiers::NONE,
        )))
        .unwrap();
    assert_eq!(f.app.command_input.value, "z");
    f.app.handle_event(Event::Tick).unwrap();
    assert!(matches!(f.app.phase, AppPhase::Home { .. }));
    assert_eq!(f.app.sync_tasks[&id].stage, SyncStage::Synchronizing);
    let address = f.peer.as_ref().unwrap().address.clone();
    handle(&mut f.app, &["with", &address]).unwrap();
    assert_eq!(f.app.sync_tasks.len(), 1);
    assert!(f.app.sync_tasks.contains_key(&id));
    handle(&mut f.app, &["jobs"]).unwrap();
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(125, 12)).unwrap();
    terminal
        .draw(|frame| {
            let area = frame.area();
            crate::screens::sync_status::render(
                frame,
                area,
                &[],
                "",
                &f.app.sync_tasks,
                &f.app.i18n,
            );
        })
        .unwrap();
    let display: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(display.contains(&id.0.to_string()));
    assert!(display.contains("Synchronizing"));
    handle(&mut f.app, &["cancel", "invalid-id"]).unwrap();
    assert!(!f.app.sync_tasks[&id].cancelling);
    f.app.error_message = None;
    handle(&mut f.app, &["cancel", &id.0.to_string()]).unwrap();
    assert!(f.app.sync_tasks[&id].cancelling);
    assert!(
        solosoul_core::import_activity::begin_owned_root_maintenance(
            f.app.vault_service.root_owner()
        )
        .is_err()
    );
    f.finish();
    assert!(f.app.success_message.is_none());
    assert!(f.app.error_message.is_none());
    assert!(f.app.info_message.as_ref().unwrap().contains("cancelled"));
    assert!(
        solosoul_core::import_activity::begin_owned_root_maintenance(
            f.app.vault_service.root_owner()
        )
        .is_ok()
    );
}

#[test]
fn rf213_lock_cancels_actual_peer_and_old_progress_cannot_restore_decrypted_page() {
    let mut f = Fixture::new();
    let id = f.start(PeerMode::Stalled);
    let old_store = f.app.vault_service.get_vault_store().unwrap();
    f.app.previous_phase = Some(f.app.phase.clone());
    crate::commands::auth::lock(&mut f.app);
    assert!(matches!(f.app.phase, AppPhase::Locked));
    assert!(f.app.previous_phase.is_none());
    assert!(f.app.sync_tasks.is_empty());
    f.finish();
    assert!(matches!(f.app.phase, AppPhase::Locked));
    assert!(f.app.previous_phase.is_none());
    assert!(f.app.success_message.is_none());
    assert!(f.app.error_message.is_none());
    assert!(!f.app.tasks.is_current(id));
    assert!(
        old_store
            .list_objects(ACCOUNT, None, None, None, false, false)
            .is_err(),
        "retired Store must reject further commits/reads"
    );
}

#[test]
fn rf213_real_peer_handshake_failure_is_failure_after_cleanup_without_success() {
    let mut f = Fixture::new();
    f.start(PeerMode::Disconnect);
    f.finish();
    assert!(f.app.success_message.is_none());
    assert!(f.app.error_message.as_ref().unwrap().contains("failed"));
    assert!(matches!(f.app.phase, AppPhase::Home { .. }));
}

#[test]
fn rf213_actual_noise_pairing_pending_and_followup_cancel_never_report_success() {
    let mut f = Fixture::new();
    f.start_protocol(false);
    f.finish();
    assert!(f.app.success_message.is_none());
    assert!(f
        .app
        .error_message
        .as_ref()
        .unwrap()
        .contains("pairing_pending:"));
    handle(&mut f.app, &["cancel"]).unwrap();
    assert!(f.app.success_message.is_none());
    assert!(f
        .app
        .error_message
        .as_ref()
        .unwrap()
        .contains("pairing_pending:"));
    let peers = f
        .remote
        .as_ref()
        .unwrap()
        .0
        .get_vault_store()
        .unwrap()
        .list_peers()
        .unwrap();
    assert!(
        peers.is_empty(),
        "untrusted responder must not persist peer before consent"
    );
    let local_peers = f
        .app
        .vault_service
        .get_vault_store()
        .unwrap()
        .list_peers()
        .unwrap();
    assert_eq!(local_peers.len(), 1);
    assert!(!local_peers[0].trusted);
}

#[test]
fn rf213_actual_trusted_noise_sync_completes_without_changing_current_page() {
    let mut f = Fixture::new();
    f.start_protocol(true);
    f.finish();
    assert!(f.app.error_message.is_none(), "{:?}", f.app.error_message);
    let message = &f.app.success_message.as_ref().unwrap().0;
    assert!(message.contains("completed"));
    assert!(message.contains("records applied="));
    assert!(matches!(f.app.phase, AppPhase::Home { .. }));
}

#[test]
fn rf213_real_completed_result_is_rejected_after_same_account_reunlock() {
    let mut f = Fixture::new();
    let id = f.start_protocol(true);
    let event = f.completed_without_applying();
    assert_eq!(event.identity.task_id, id);
    assert!(f.app.success_message.is_none());
    crate::commands::auth::lock(&mut f.app);
    f.app
        .vault_service
        .unlock(ACCOUNT, crate::TEST_PASSWORD)
        .unwrap();
    f.app.phase = AppPhase::Home {
        account_id: ACCOUNT.into(),
    };
    f.app.handle_event(Event::Task(event)).unwrap();
    assert!(f.app.success_message.is_none());
    assert!(f.app.error_message.is_none());
    assert!(matches!(f.app.phase, AppPhase::Home { .. }));
    assert!(f.app.previous_phase.is_none());
    f.finish();
}

#[test]
fn rf213_exit_waits_actual_peer_shutdown_before_releasing_task_activity() {
    let mut f = Fixture::new();
    f.start(PeerMode::Stalled);
    f.app.shutdown_tasks().unwrap();
    assert!(f.app.tasks.is_empty());
    assert!(f.app.sync_tasks.is_empty());
    assert!(f.app.success_message.is_none());
    f.peer.as_mut().unwrap().join();
    assert!(f.peer.as_ref().unwrap().closed.load(Ordering::Acquire));
    assert!(
        solosoul_core::import_activity::begin_owned_root_maintenance(
            f.app.vault_service.root_owner()
        )
        .is_ok()
    );
}
