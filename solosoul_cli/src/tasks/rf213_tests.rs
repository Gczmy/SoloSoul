use super::*;
use std::ops::{Deref, DerefMut};
use std::sync::{mpsc as std_mpsc, MutexGuard};
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::oneshot;

const WAIT: Duration = Duration::from_secs(15);
struct Fixture {
    service: Arc<VaultService>,
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
            .create_account_with_id(
                "acc_rf213_tasks",
                "RF213 synthetic",
                crate::TEST_PASSWORD,
                None,
            )
            .unwrap();
        Self {
            service,
            _dir: dir,
            _serial: serial,
        }
    }
    fn session(&self) -> VaultSession {
        self.service.capture_session("acc_rf213_tasks").unwrap()
    }
}
struct ManagedTasks(Tasks);
impl Deref for ManagedTasks {
    type Target = Tasks;
    fn deref(&self) -> &Tasks {
        &self.0
    }
}
impl DerefMut for ManagedTasks {
    fn deref_mut(&mut self) -> &mut Tasks {
        &mut self.0
    }
}
impl Drop for ManagedTasks {
    fn drop(&mut self) {
        crate::util::shared_runtime()
            .unwrap()
            .block_on(self.0.shutdown());
    }
}
struct Gate(Option<oneshot::Sender<()>>);
impl Gate {
    fn release(&mut self) {
        if let Some(tx) = self.0.take() {
            let _ = tx.send(());
        }
    }
}
impl Drop for Gate {
    fn drop(&mut self) {
        self.release();
    }
}
fn gate() -> (Gate, oneshot::Receiver<()>) {
    let (tx, rx) = oneshot::channel();
    (Gate(Some(tx)), rx)
}
fn terminal(tasks: &mut Tasks) -> TaskEvent {
    crate::util::shared_runtime().unwrap().block_on(async {
        tokio::time::timeout(WAIT, async {
            loop {
                for event in tasks.poll_events(32) {
                    if event.kind.is_terminal() {
                        return event;
                    }
                    tasks.apply_event(event, |_| {});
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("RF213 real Future must join")
    })
}

#[test]
fn rf213_cancel_before_wait_wakes_and_retains_activity_until_real_cleanup() {
    let fixture = Fixture::new();
    let mut tasks = ManagedTasks(Tasks::new(fixture.service.clone()));
    let (mut start, start_rx) = gate();
    let (mut cleanup, cleanup_rx) = gate();
    let (entered_tx, entered) = std_mpsc::channel();
    let (cancel_tx, cancelled) = std_mpsc::channel();
    let ended = Arc::new(AtomicBool::new(false));
    let worker_ended = ended.clone();
    let id = tasks
        .spawn_cooperative(fixture.session(), move |context| async move {
            entered_tx.send(()).unwrap();
            let _ = start_rx.await;
            context.cancelled().await;
            cancel_tx.send(()).unwrap();
            let _ = cleanup_rx.await;
            worker_ended.store(true, Ordering::Release);
            Err(TaskFailure::Cancelled)
        })
        .unwrap();
    entered.recv_timeout(WAIT).unwrap();
    assert!(tasks.request_cancel(id));
    assert!(tasks.request_cancel(id));
    start.release();
    cancelled
        .recv_timeout(WAIT)
        .expect("cancel before notified registration must not be lost");
    assert!(!ended.load(Ordering::Acquire));
    assert!(tasks.poll_events(32).is_empty());
    assert!(!tasks.is_empty());
    assert!(
        solosoul_core::import_activity::begin_owned_root_maintenance(fixture.service.root_owner())
            .is_err()
    );
    cleanup.release();
    let event = terminal(&mut tasks);
    assert_eq!(event.identity.task_id, id);
    assert_eq!(event.kind, TaskEventKind::Cancelled);
    assert!(ended.load(Ordering::Acquire));
    assert!(tasks.apply_event(event, |_| {}));
    assert!(tasks.is_empty());
    assert!(
        solosoul_core::import_activity::begin_owned_root_maintenance(fixture.service.root_owner())
            .is_ok()
    );
}

#[test]
fn rf213_cancelled_shutdown_waiter_keeps_cooperative_cleanup_owned_and_joins_later() {
    let fixture = Fixture::new();
    let mut tasks = ManagedTasks(Tasks::new(fixture.service.clone()));
    let (mut cleanup, cleanup_rx) = gate();
    let (entered_tx, entered) = std_mpsc::channel();
    let (cancel_tx, cancelled) = std_mpsc::channel();
    tasks
        .spawn_cooperative(fixture.session(), move |context| async move {
            entered_tx.send(()).unwrap();
            context.cancelled().await;
            cancel_tx.send(()).unwrap();
            let _ = cleanup_rx.await;
            Err(TaskFailure::Cancelled)
        })
        .unwrap();
    entered.recv_timeout(WAIT).unwrap();
    crate::util::shared_runtime().unwrap().block_on(async {
        assert!(
            tokio::time::timeout(Duration::from_millis(60), tasks.shutdown())
                .await
                .is_err()
        );
    });
    cancelled.recv_timeout(WAIT).unwrap();
    assert!(!tasks.is_empty());
    assert!(
        solosoul_core::import_activity::begin_owned_root_maintenance(fixture.service.root_owner())
            .is_err()
    );
    cleanup.release();
    let result = crate::util::shared_runtime()
        .unwrap()
        .block_on(tasks.shutdown());
    assert_eq!(result.joined, 1);
    assert_eq!(result.cancelled, 1);
    assert_eq!(result.panicked, 0);
    assert!(tasks.is_empty());
}

#[test]
fn rf213_cooperative_real_failure_survives_cancel_request_and_forged_progress_is_rejected() {
    let fixture = Fixture::new();
    let mut tasks = ManagedTasks(Tasks::new(fixture.service.clone()));
    let (mut cleanup, cleanup_rx) = gate();
    let (identity_tx, identity) = std_mpsc::channel();
    let id = tasks
        .spawn_cooperative(fixture.session(), move |context| async move {
            identity_tx.send(context.identity().clone()).unwrap();
            let _ = cleanup_rx.await;
            Err(TaskFailure::Failed("RF213 real cleanup failure".into()))
        })
        .unwrap();
    let identity = identity.recv_timeout(WAIT).unwrap();
    let mut forged = identity.clone();
    forged.task_id = TaskId(Uuid::new_v4());
    assert!(!tasks.apply_event(
        TaskEvent {
            identity: forged,
            kind: TaskEventKind::SyncProgress(SyncStage::Stopping)
        },
        |_| panic!("forged event applied")
    ));
    assert!(tasks.request_cancel(id));
    cleanup.release();
    let event = terminal(&mut tasks);
    assert_eq!(
        event.kind,
        TaskEventKind::Failed("RF213 real cleanup failure".into())
    );
    assert!(tasks.apply_event(event, |_| {}));
    assert!(tasks.is_empty());
}
