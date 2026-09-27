use super::*;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::{mpsc as std_mpsc, MutexGuard};
use std::task::Poll;
use std::time::Duration;
use tempfile::TempDir;

const ACCOUNT_A: &str = "acc_rf212_tasks_a";
const ACCOUNT_B: &str = "acc_rf212_tasks_b";
const WAIT: Duration = Duration::from_secs(10);
const MODEL_BYTES: &[u8] = b"RF212 synthetic verified model bytes";

struct Fixture {
    service: Arc<VaultService>,
    directory: TempDir,
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
            .create_account_with_id(ACCOUNT_A, "RF212 synthetic A", crate::TEST_PASSWORD, None)
            .unwrap();
        std::fs::write(directory.path().join("keep.txt"), b"RF212 parent sentinel").unwrap();
        Self {
            service,
            directory,
            _serial: serial,
        }
    }

    fn session(&self) -> VaultSession {
        self.service
            .capture_session(&self.service.get_current_account().unwrap())
            .unwrap()
    }

    fn staged_model(&self) -> (TempDir, PathBuf, PathBuf) {
        let stage = tempfile::Builder::new()
            .prefix("rf212-staging-")
            .tempdir_in(self.directory.path())
            .unwrap();
        std::fs::write(stage.path().join("model.bin"), MODEL_BYTES).unwrap();
        let stage_path = stage.path().to_path_buf();
        (
            stage,
            stage_path,
            self.directory.path().join("installed-model"),
        )
    }

    fn assert_sentinel(&self) {
        assert_eq!(
            std::fs::read(self.directory.path().join("keep.txt")).unwrap(),
            b"RF212 parent sentinel"
        );
    }
}

/// 断言失败也先放行同步屏障，再 join 本测试自己的 Future。
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

struct Gate {
    entered: std_mpsc::Receiver<()>,
    release: Option<std_mpsc::Sender<()>>,
}
struct WorkerGate {
    entered: std_mpsc::Sender<()>,
    release: std_mpsc::Receiver<()>,
}
fn gate() -> (Gate, WorkerGate) {
    let (entered_tx, entered) = std_mpsc::channel();
    let (release, release_rx) = std_mpsc::channel();
    (
        Gate {
            entered,
            release: Some(release),
        },
        WorkerGate {
            entered: entered_tx,
            release: release_rx,
        },
    )
}
impl Gate {
    fn wait_entered(&self) {
        self.entered
            .recv_timeout(WAIT)
            .expect("RF212 worker must reach barrier");
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
impl WorkerGate {
    /// 仅测试通过同步屏障控制同一 poll 内的竞争；持会话门闩时不 panic。
    fn pause(self) -> Result<(), String> {
        self.entered
            .send(())
            .map_err(|_| "RF212 barrier receiver closed".to_string())?;
        self.release
            .recv_timeout(WAIT)
            .map_err(|_| "RF212 barrier was not released".to_string())
    }
}

fn installed() -> TaskOutput {
    TaskOutput::EmbedModelInstalled {
        model_id: "rf212-model".into(),
        bytes: MODEL_BYTES.len() as u64,
    }
}

fn publish(stage: &Path, target: &Path) -> Result<TaskOutput, String> {
    // 完成数据在发布前准备，rename 后不添加可能失败的步骤。
    let output = installed();
    std::fs::rename(stage, target).map_err(|_| "RF212 synthetic publish failed".to_string())?;
    Ok(output)
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
        .expect("RF212 expected a joined terminal event")
    })
}

#[test]
fn rf212_cancel_wins_before_commit_without_publishing_or_leaking_staging() {
    let fixture = Fixture::new();
    let mut tasks = ManagedTasks::new(&fixture);
    let (stage, stage_path, target) = fixture.staged_model();
    let worker_target = target.clone();
    let (mut barrier, worker_barrier) = gate();
    let invoked = Arc::new(AtomicBool::new(false));
    let worker_invoked = Arc::clone(&invoked);
    let (result_tx, result_rx) = std_mpsc::channel();
    let task_id = tasks
        .spawn(fixture.session(), move |context| async move {
            // 当前 poll 已开始，abort 不会跳过后续同步 commit，直接验证它的取消门闩。
            worker_barrier.pause().map_err(TaskFailure::Failed)?;
            let result = context.commit(|| {
                worker_invoked.store(true, Ordering::Release);
                publish(stage.path(), &worker_target)
            });
            let _ = result_tx.send(result.clone());
            result
        })
        .unwrap();
    barrier.wait_entered();
    assert!(tasks.request_cancel(task_id));
    assert!(!tasks.is_empty());
    barrier.release();
    assert_eq!(
        result_rx.recv_timeout(WAIT).unwrap(),
        Err(TaskFailure::Cancelled)
    );
    let event = next_event(&mut tasks);
    assert_eq!(event.kind, TaskEventKind::Cancelled);
    assert!(!invoked.load(Ordering::Acquire));
    assert!(!target.exists());
    assert!(!stage_path.exists());
    assert!(tasks.apply_event(event, |_| {}));
    assert!(tasks.is_empty());
    fixture.assert_sentinel();
}

#[test]
fn rf212_commit_wins_and_cancellation_cannot_replace_real_joined_success() {
    let fixture = Fixture::new();
    let mut tasks = ManagedTasks::new(&fixture);
    let (stage, stage_path, target) = fixture.staged_model();
    let worker_target = target.clone();
    let (mut barrier, worker_barrier) = gate();
    let task_id = tasks
        .spawn(fixture.session(), move |context| async move {
            context.commit(|| {
                let output = publish(stage.path(), &worker_target)?;
                // 放大发布完成与 Future 返回之间的窗口；这里只控制测试时序。
                worker_barrier.pause()?;
                Ok(output)
            })
        })
        .unwrap();
    barrier.wait_entered();
    assert_eq!(
        std::fs::read(target.join("model.bin")).unwrap(),
        MODEL_BYTES
    );
    assert!(!tasks.request_cancel(task_id));
    tasks.cancel_all();
    assert!(
        tasks.poll_events(32).is_empty(),
        "publish itself must not synthesize completion"
    );
    assert!(
        !tasks.is_empty(),
        "join owner must remain until Future actually returns"
    );
    barrier.release();
    let event = next_event(&mut tasks);
    assert_eq!(event.kind, TaskEventKind::Completed(installed()));
    let duplicate = event.clone();
    let mut published = 0;
    assert!(tasks.apply_event(event, |_| published += 1));
    assert!(!tasks.apply_event(duplicate, |_| published += 1));
    assert_eq!(published, 1);
    assert!(!stage_path.exists());
    assert!(tasks.is_empty());
    fixture.assert_sentinel();
}

#[test]
fn rf212_commit_failure_is_not_masked_as_cancelled_and_fresh_task_can_retry() {
    let fixture = Fixture::new();
    let mut tasks = ManagedTasks::new(&fixture);
    let (stage, stage_path, target) = fixture.staged_model();
    let (mut barrier, worker_barrier) = gate();
    let task_id = tasks
        .spawn(fixture.session(), move |context| async move {
            let _stage = stage;
            context.commit(|| {
                worker_barrier.pause()?;
                Err("RF212 synthetic commit failure".into())
            })
        })
        .unwrap();
    barrier.wait_entered();
    assert!(!tasks.request_cancel(task_id));
    tasks.cancel_all();
    barrier.release();
    let event = next_event(&mut tasks);
    assert_eq!(
        event.kind,
        TaskEventKind::Failed("RF212 synthetic commit failure".into())
    );
    assert!(tasks.apply_event(event, |_| {}));
    assert!(!stage_path.exists());
    assert!(!target.exists());

    let (stage, retry_stage_path, retry_target) = fixture.staged_model();
    tasks
        .spawn(fixture.session(), move |context| async move {
            context.commit(|| publish(stage.path(), &retry_target))
        })
        .unwrap();
    let event = next_event(&mut tasks);
    assert_eq!(event.kind, TaskEventKind::Completed(installed()));
    assert!(tasks.apply_event(event, |_| {}));
    assert_eq!(
        std::fs::read(target.join("model.bin")).unwrap(),
        MODEL_BYTES
    );
    assert!(!retry_stage_path.exists());
    assert!(tasks.is_empty());
    fixture.assert_sentinel();
}

#[test]
fn rf212_original_session_must_still_own_commit_after_lock_reunlock_or_switch() {
    for transition in ["lock", "reunlock", "switch"] {
        let fixture = Fixture::new();
        let mut tasks = ManagedTasks::new(&fixture);
        let original = fixture.session();
        let generation = original.generation();
        let (stage, stage_path, target) = fixture.staged_model();
        let worker_target = target.clone();
        let (mut barrier, worker_barrier) = gate();
        let invoked = Arc::new(AtomicBool::new(false));
        let worker_invoked = Arc::clone(&invoked);
        let (result_tx, result_rx) = std_mpsc::channel();
        tasks
            .spawn(original, move |context| async move {
                worker_barrier.pause().map_err(TaskFailure::Failed)?;
                let result = context.commit(|| {
                    worker_invoked.store(true, Ordering::Release);
                    publish(stage.path(), &worker_target)
                });
                let _ = result_tx.send(result.clone());
                result
            })
            .unwrap();
        barrier.wait_entered();
        fixture.service.lock();
        match transition {
            "reunlock" => {
                fixture
                    .service
                    .unlock(ACCOUNT_A, crate::TEST_PASSWORD)
                    .unwrap();
                assert_ne!(fixture.session().generation(), generation);
            }
            "switch" => {
                fixture
                    .service
                    .create_account_with_id(
                        ACCOUNT_B,
                        "RF212 synthetic B",
                        crate::TEST_PASSWORD,
                        None,
                    )
                    .unwrap();
                assert_eq!(fixture.session().account_id(), ACCOUNT_B);
            }
            "lock" => {}
            _ => unreachable!(),
        }
        barrier.release();
        assert_eq!(
            result_rx.recv_timeout(WAIT).unwrap(),
            Err(TaskFailure::Cancelled)
        );
        let event = next_event(&mut tasks);
        assert_eq!(event.kind, TaskEventKind::Cancelled);
        let mut ui_changed = false;
        assert!(!tasks.apply_event(event, |_| ui_changed = true));
        assert!(!ui_changed);
        assert!(!invoked.load(Ordering::Acquire));
        assert!(!target.exists());
        assert!(!stage_path.exists());
        assert!(tasks.is_empty());
        fixture.assert_sentinel();
    }
}

#[test]
fn rf212_shutdown_waits_for_claimed_commit_instead_of_aborting_published_work() {
    let fixture = Fixture::new();
    let mut tasks = ManagedTasks::new(&fixture);
    let (stage, stage_path, target) = fixture.staged_model();
    let worker_target = target.clone();
    let (mut barrier, worker_barrier) = gate();
    tasks
        .spawn(fixture.session(), move |context| async move {
            context.commit(|| {
                let output = publish(stage.path(), &worker_target)?;
                worker_barrier.pause()?;
                Ok(output)
            })
        })
        .unwrap();
    barrier.wait_entered();
    let report = crate::util::shared_runtime().unwrap().block_on(async {
        let shutdown = tasks.shutdown();
        tokio::pin!(shutdown);
        // 真实 poll 已执行 cancel_all 和 join；提交仍停在屏障，不得提前退出。
        std::future::poll_fn(|cx| match shutdown.as_mut().poll(cx) {
            Poll::Pending => Poll::Ready(()),
            Poll::Ready(_) => panic!("RF212 shutdown returned before the committing worker joined"),
        })
        .await;
        barrier.release();
        shutdown.await
    });
    assert_eq!(
        report,
        ShutdownReport {
            joined: 1,
            cancelled: 0,
            failed: 0,
            panicked: 0
        }
    );
    assert_eq!(
        std::fs::read(target.join("model.bin")).unwrap(),
        MODEL_BYTES
    );
    assert!(!stage_path.exists());
    assert!(tasks.is_empty());
    assert!(tasks.poll_events(32).is_empty());
    fixture.assert_sentinel();
}
