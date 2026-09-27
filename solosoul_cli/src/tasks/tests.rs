use super::*;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::AtomicUsize;
use std::sync::{mpsc as std_mpsc, MutexGuard};
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::oneshot;

const ACCOUNT_A: &str = "acc_rf211_a";
const ACCOUNT_B: &str = "acc_rf211_b";
const WAIT: Duration = Duration::from_secs(5);

struct Fixture {
    service: Arc<VaultService>,
    _directory: TempDir,
    _serial: MutexGuard<'static, ()>,
}

impl Fixture {
    fn new() -> Self {
        let serial = crate::VAULT_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let directory = TempDir::new().unwrap();
        let service = Arc::new(VaultService::with_base_path(directory.path().to_path_buf()));
        service
            .create_account_with_id(ACCOUNT_A, "RF211 synthetic A", crate::TEST_PASSWORD, None)
            .unwrap();
        Self {
            service,
            _directory: directory,
            _serial: serial,
        }
    }

    fn session(&self) -> VaultSession {
        let account = self.service.get_current_account().unwrap();
        self.service.capture_session(&account).unwrap()
    }

    fn unlock_a(&self) {
        self.service
            .unlock(ACCOUNT_A, crate::TEST_PASSWORD)
            .unwrap();
    }
}

/// 即使断言失败，也回收本测试的全部 Future 后才释放 Vault 和临时目录。
struct ManagedTasks(Tasks);

impl ManagedTasks {
    fn new(fixture: &Fixture) -> Self {
        Self(Tasks::new(Arc::clone(&fixture.service)))
    }
}

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
        if let Ok(runtime) = crate::util::shared_runtime() {
            runtime.block_on(self.0.shutdown());
        }
    }
}

struct DropFlag(Arc<AtomicBool>);

impl Drop for DropFlag {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

struct Gate {
    started: std_mpsc::Receiver<()>,
    release: Option<oneshot::Sender<()>>,
}

impl Gate {
    fn wait_started(&self) {
        self.started.recv_timeout(WAIT).unwrap();
    }

    fn release(&mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
    }
}

impl Drop for Gate {
    fn drop(&mut self) {
        self.release();
    }
}

fn blocked_task(
    tasks: &mut Tasks,
    session: VaultSession,
    progress: bool,
) -> (TaskId, Gate, Arc<AtomicBool>) {
    let (started_tx, started) = std_mpsc::channel();
    let (release_tx, release_rx) = oneshot::channel();
    let dropped = Arc::new(AtomicBool::new(false));
    let worker_dropped = Arc::clone(&dropped);
    let task_id = tasks
        .spawn(session, move |context| async move {
            let _resource = DropFlag(worker_dropped);
            if progress {
                context.report_progress(1, Some(2));
            }
            let _ = started_tx.send(());
            let _ = release_rx.await;
            Ok(TaskOutput::Message("RF211 synthetic completion".into()))
        })
        .unwrap();
    (
        task_id,
        Gate {
            started,
            release: Some(release_tx),
        },
        dropped,
    )
}

fn wait_until(mut condition: impl FnMut() -> bool) {
    crate::util::shared_runtime().unwrap().block_on(async {
        tokio::time::timeout(WAIT, async {
            while !condition() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("RF211 worker did not reach its deterministic barrier");
    });
}

fn next_event(tasks: &mut Tasks) -> TaskEvent {
    crate::util::shared_runtime().unwrap().block_on(async {
        tokio::time::timeout(WAIT, async {
            loop {
                if let Some(event) = tasks.poll_events(1).pop() {
                    return event;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("RF211 expected a task event")
    })
}

#[test]
fn rf211_progress_and_completion_require_the_original_session_and_single_delivery() {
    let fixture = Fixture::new();
    let session = fixture.session();
    let generation = session.generation();
    let mut tasks = ManagedTasks::new(&fixture);
    let (task_id, mut gate, dropped) = blocked_task(&mut tasks, session, true);
    gate.wait_started();
    let progress = next_event(&mut tasks);
    assert_eq!(progress.identity.task_id, task_id);
    assert_eq!(progress.identity.account_id, ACCOUNT_A);
    assert_eq!(progress.identity.session_generation, generation);
    assert_eq!(
        progress.kind,
        TaskEventKind::Progress {
            current: 1,
            total: Some(2),
        }
    );
    let old_progress = progress.clone();
    let mut applied = Vec::new();
    assert!(tasks.apply_event(progress, |kind| applied.push(kind)));
    assert!(!tasks.is_empty());

    gate.release();
    let completed = next_event(&mut tasks);
    assert!(
        dropped.load(Ordering::Acquire),
        "resource drops before join publishes completion"
    );
    assert_eq!(
        completed.kind,
        TaskEventKind::Completed(TaskOutput::Message("RF211 synthetic completion".into()))
    );
    let duplicate = completed.clone();
    assert!(tasks.apply_event(completed, |kind| applied.push(kind)));
    assert!(!tasks.apply_event(duplicate, |kind| applied.push(kind)));
    assert!(!tasks.apply_event(old_progress, |kind| applied.push(kind)));
    assert_eq!(applied.len(), 2);
    assert!(tasks.is_empty());
}

#[test]
fn rf211_unknown_or_early_terminal_events_cannot_finish_a_live_task() {
    let fixture = Fixture::new();
    let session = fixture.session();
    let generation = session.generation();
    let mut tasks = ManagedTasks::new(&fixture);
    let (task_id, gate, _) = blocked_task(&mut tasks, session, false);
    gate.wait_started();
    let mut applied = 0;
    for (id, account, version) in [
        (TaskId(Uuid::new_v4()), ACCOUNT_A, generation),
        (task_id, ACCOUNT_B, generation),
        (task_id, ACCOUNT_A, generation.wrapping_add(1)),
        (task_id, ACCOUNT_A, generation),
    ] {
        let forged = TaskEvent {
            identity: TaskIdentity {
                task_id: id,
                account_id: account.into(),
                session_generation: version,
            },
            kind: TaskEventKind::Completed(TaskOutput::Message("unissued result".into())),
        };
        assert!(!tasks.apply_event(forged, |_| applied += 1));
        assert!(
            tasks.is_current(task_id),
            "forged identity must not invalidate real work"
        );
    }
    assert_eq!(applied, 0);
    assert!(!tasks.is_empty());
    assert!(tasks.request_cancel(task_id));
    let cancelled = next_event(&mut tasks);
    assert_eq!(cancelled.kind, TaskEventKind::Cancelled);
    assert!(tasks.apply_event(cancelled, |_| applied += 1));
    assert_eq!(applied, 1);
    assert!(tasks.is_empty());
}

#[test]
fn rf211_cancel_is_a_request_until_join_and_drops_suspended_future_resources() {
    let fixture = Fixture::new();
    let mut tasks = ManagedTasks::new(&fixture);
    let (task_id, gate, dropped) = blocked_task(&mut tasks, fixture.session(), false);
    gate.wait_started();
    assert!(tasks.request_cancel(task_id));
    assert!(
        !tasks.is_empty(),
        "requesting cancellation must retain the join owner"
    );
    let event = next_event(&mut tasks);
    assert_eq!(event.kind, TaskEventKind::Cancelled);
    assert!(dropped.load(Ordering::Acquire));
    assert!(
        !tasks.request_cancel(task_id),
        "already joined work is no longer cancellable"
    );
    assert!(tasks.apply_event(event, |_| {}));
    assert!(tasks.is_empty());
}

#[test]
fn rf211_cancellation_after_worker_completion_preserves_its_real_success() {
    let fixture = Fixture::new();
    let mut tasks = ManagedTasks::new(&fixture);
    let task_id = tasks
        .spawn(fixture.session(), |_| async {
            Ok(TaskOutput::Message("already completed".into()))
        })
        .unwrap();
    wait_until(|| tasks.records[&task_id].abort.is_finished());
    assert!(tasks.request_cancel(task_id));
    let event = next_event(&mut tasks);
    assert_eq!(
        event.kind,
        TaskEventKind::Completed(TaskOutput::Message("already completed".into()))
    );
    assert!(tasks.apply_event(event, |_| {}));
    assert!(tasks.is_empty());
}

#[test]
fn rf211_failure_and_panic_are_joined_terminal_events_without_panic_payload() {
    let fixture = Fixture::new();
    let mut tasks = ManagedTasks::new(&fixture);
    tasks
        .spawn(fixture.session(), |_| async {
            Err(TaskFailure::Failed(
                "RF211 synthetic expected failure".into(),
            ))
        })
        .unwrap();
    tasks
        .spawn(fixture.session(), |_| async {
            panic!("RF211 synthetic private panic payload");
        })
        .unwrap();
    let mut failures = Vec::new();
    for _ in 0..2 {
        let event = next_event(&mut tasks);
        assert!(tasks.apply_event(event, |kind| {
            if let TaskEventKind::Failed(message) = kind {
                failures.push(message);
            }
        }));
    }
    failures.sort();
    let mut expected = vec![
        "RF211 synthetic expected failure".to_string(),
        TASK_PANICKED.into(),
    ];
    expected.sort();
    assert_eq!(failures, expected);
    assert!(failures
        .iter()
        .all(|message| !message.contains("private panic payload")));
    assert!(tasks.is_empty());
}

#[test]
fn rf211_lock_reunlock_and_account_switch_reject_old_progress_and_ready_results() {
    for transition in ["lock", "same-account", "other-account"] {
        let fixture = Fixture::new();
        let original = fixture.session();
        let original_generation = original.generation();
        let mut tasks = ManagedTasks::new(&fixture);
        let completed_id = tasks
            .spawn(original.clone(), |_| async {
                Ok(TaskOutput::Message("old account result".into()))
            })
            .unwrap();
        let completed = next_event(&mut tasks);
        let (pending_id, mut gate, dropped) = blocked_task(&mut tasks, original, true);
        gate.wait_started();
        let progress = next_event(&mut tasks);

        match transition {
            "lock" => fixture.service.lock(),
            "same-account" => {
                fixture.service.lock();
                fixture.unlock_a();
                assert_ne!(fixture.session().generation(), original_generation);
            }
            "other-account" => {
                fixture
                    .service
                    .create_account_with_id(
                        ACCOUNT_B,
                        "RF211 synthetic B",
                        crate::TEST_PASSWORD,
                        None,
                    )
                    .unwrap();
                assert_eq!(fixture.session().account_id(), ACCOUNT_B);
            }
            _ => unreachable!(),
        }
        // 原终态已取出后直接锁定：不先调用 cancel_stale，也不能遗留已回收任务的进度。
        if transition == "lock" {
            let mut published = false;
            assert!(!tasks.apply_event(completed.clone(), |_| published = true));
            assert!(!published);
            assert!(!tasks.is_current(completed_id));
        }
        let mut invalidated = tasks.cancel_stale();
        invalidated.sort_by_key(|id| id.0);
        let mut expected = if transition == "lock" {
            vec![pending_id]
        } else {
            vec![completed_id, pending_id]
        };
        expected.sort_by_key(|id| id.0);
        assert_eq!(invalidated, expected);
        assert!(
            tasks.cancel_stale().is_empty(),
            "stale notifications are delivered once"
        );
        let mut applied = Vec::new();
        assert!(!tasks.apply_event(completed, |kind| applied.push(kind)));
        assert!(!tasks.is_current(completed_id));
        assert!(!tasks.is_current(pending_id));
        assert!(!tasks.apply_event(progress, |kind| applied.push(kind)));
        gate.release();
        let terminal = next_event(&mut tasks);
        assert!(dropped.load(Ordering::Acquire));
        assert!(!tasks.apply_event(terminal, |kind| applied.push(kind)));
        assert!(applied.is_empty());
        assert!(tasks.is_empty());

        if transition == "lock" {
            fixture.unlock_a();
        }
        tasks
            .spawn(fixture.session(), |_| async {
                Ok(TaskOutput::Message("fresh account result".into()))
            })
            .unwrap();
        let event = next_event(&mut tasks);
        assert!(tasks.apply_event(event, |kind| applied.push(kind)));
        assert_eq!(
            applied,
            [TaskEventKind::Completed(TaskOutput::Message(
                "fresh account result".into()
            ))]
        );
    }
}

#[test]
fn rf211_progress_is_bounded_and_terminal_bypasses_a_full_progress_queue() {
    let fixture = Fixture::new();
    let mut tasks = ManagedTasks::new(&fixture);
    let (started_tx, started) = std_mpsc::channel();
    let (release_tx, release_rx) = oneshot::channel();
    let accepted = Arc::new(AtomicUsize::new(0));
    let worker_accepted = Arc::clone(&accepted);
    let flood_id = tasks
        .spawn(fixture.session(), move |context| async move {
            for current in 0..1024 {
                if context.report_progress(current, Some(1024)) {
                    worker_accepted.fetch_add(1, Ordering::Relaxed);
                }
            }
            let _ = started_tx.send(());
            let _ = release_rx.await;
            Ok(TaskOutput::Message("flood task complete".into()))
        })
        .unwrap();
    let mut gate = Gate {
        started,
        release: Some(release_tx),
    };
    gate.wait_started();
    let accepted = accepted.load(Ordering::Acquire);
    assert!(
        accepted > 0 && accepted < 1024,
        "producer cannot build an unbounded progress backlog"
    );
    assert!(tasks.poll_events(0).is_empty());
    let terminal_id = tasks
        .spawn(fixture.session(), |_| async {
            Ok(TaskOutput::Message("unrelated terminal".into()))
        })
        .unwrap();
    wait_until(|| tasks.records[&terminal_id].abort.is_finished());
    let mut events = tasks.poll_events(1);
    assert_eq!(events.len(), 1);
    let terminal = events.pop().unwrap();
    assert_eq!(terminal.identity.task_id, terminal_id);
    assert!(matches!(terminal.kind, TaskEventKind::Completed(_)));
    assert!(tasks.apply_event(terminal, |_| {}));
    let progress = tasks.poll_events(3);
    assert_eq!(progress.len(), 3);
    for event in progress {
        assert_eq!(event.identity.task_id, flood_id);
        assert!(matches!(event.kind, TaskEventKind::Progress { .. }));
        assert!(tasks.apply_event(event, |_| {}));
    }
    gate.release();
    wait_until(|| tasks.records[&flood_id].abort.is_finished());
    let terminal = next_event(&mut tasks);
    assert!(matches!(terminal.kind, TaskEventKind::Completed(_)));
    assert!(tasks.apply_event(terminal, |_| {}));
    assert!(
        tasks.poll_events(1024).is_empty(),
        "queued progress cannot revive terminal work"
    );
    assert!(tasks.is_empty());
}

#[test]
fn rf211_shutdown_joins_all_suspended_tasks_and_closes_admission() {
    let fixture = Fixture::new();
    let mut tasks = ManagedTasks::new(&fixture);
    let (_, first, first_dropped) = blocked_task(&mut tasks, fixture.session(), true);
    let (_, second, second_dropped) = blocked_task(&mut tasks, fixture.session(), true);
    first.wait_started();
    second.wait_started();
    let failed = tasks
        .spawn(fixture.session(), |_| async {
            Err(TaskFailure::Failed(
                "RF211 synthetic shutdown failure".into(),
            ))
        })
        .unwrap();
    let panicked = tasks
        .spawn(fixture.session(), |_| async {
            panic!("RF211 synthetic shutdown panic");
        })
        .unwrap();
    wait_until(|| {
        tasks.records[&failed].abort.is_finished() && tasks.records[&panicked].abort.is_finished()
    });
    let report = crate::util::shared_runtime()
        .unwrap()
        .block_on(tasks.shutdown());
    assert_eq!(report.joined, 4);
    assert_eq!(report.cancelled, 2);
    assert_eq!(report.failed, 1);
    assert_eq!(report.panicked, 1);
    assert!(first_dropped.load(Ordering::Acquire));
    assert!(second_dropped.load(Ordering::Acquire));
    assert!(tasks.is_empty());
    assert!(tasks.poll_events(32).is_empty());
    let executed = Arc::new(AtomicBool::new(false));
    let worker_executed = Arc::clone(&executed);
    assert!(tasks
        .spawn(fixture.session(), move |_| async move {
            worker_executed.store(true, Ordering::Release);
            Ok(TaskOutput::Message("must not start".into()))
        })
        .is_err());
    assert!(!executed.load(Ordering::Acquire));
    let again = crate::util::shared_runtime()
        .unwrap()
        .block_on(tasks.shutdown());
    assert_eq!(again, ShutdownReport::default());
}
