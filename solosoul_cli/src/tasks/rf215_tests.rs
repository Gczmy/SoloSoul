use super::*;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::AtomicUsize;
use std::sync::{mpsc as std_mpsc, MutexGuard};
use std::time::{Duration, Instant};
use tempfile::TempDir;

const ACCOUNT_A: &str = "acc_rf215_tasks_a";
const ACCOUNT_B: &str = "acc_rf215_tasks_b";
const WAIT: Duration = Duration::from_secs(10);

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
            .create_account_with_id(ACCOUNT_A, "RF215 synthetic A", crate::TEST_PASSWORD, None)
            .unwrap();
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
}

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
    fn wait(&self) {
        self.entered
            .recv_timeout(WAIT)
            .expect("RF215 native work must reach barrier");
    }
    fn release(&mut self) {
        if let Some(sender) = self.release.take() {
            let _ = sender.send(());
        }
    }
}
impl Drop for Gate {
    fn drop(&mut self) {
        self.release();
    }
}
impl WorkerGate {
    fn pause(self) -> Result<(), TaskFailure> {
        self.entered
            .send(())
            .map_err(|_| TaskFailure::Failed("RF215 barrier closed".into()))?;
        self.release
            .recv_timeout(WAIT)
            .map_err(|_| TaskFailure::Failed("RF215 barrier timeout".into()))
    }
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
        .expect("RF215 expected managed event")
    })
}
fn drain(tasks: &mut Tasks) -> Vec<(TaskEvent, bool)> {
    let deadline = Instant::now() + WAIT;
    let mut events = Vec::new();
    while !tasks.is_empty() {
        assert!(Instant::now() < deadline, "RF215 tasks did not settle");
        let event = next_event(tasks);
        let accepted = tasks.apply_event(event.clone(), |_| {});
        events.push((event, accepted));
    }
    events
}
fn terminal(events: &[(TaskEvent, bool)], id: TaskId) -> Vec<(&TaskEventKind, bool)> {
    events
        .iter()
        .filter(|(event, _)| event.identity.task_id == id && event.kind.is_terminal())
        .map(|(event, accepted)| (&event.kind, *accepted))
        .collect()
}
fn message() -> TaskOutput {
    TaskOutput::Message("RF215 synthetic result".into())
}

#[test]
fn rf215_blocking_admission_is_bounded_and_queued_cancel_never_starts_work() {
    let fixture = Fixture::new();
    let mut tasks = ManagedTasks::new(&fixture);
    // 屏障后声明，panic 时先放行实际工作，再由 ManagedTasks join。
    let (mut barrier, worker) = gate();
    let first = tasks
        .spawn_blocking(fixture.session(), move |context, _| {
            worker.pause()?;
            context.commit(|| Ok(message()))
        })
        .unwrap();
    barrier.wait();
    let called = Arc::new(AtomicUsize::new(0));
    let dropped = Arc::new(AtomicBool::new(false));
    let resource = DropFlag(Arc::clone(&dropped));
    let call = Arc::clone(&called);
    let cancelled = tasks
        .spawn_blocking(fixture.session(), move |context, _| {
            let _resource = resource;
            call.fetch_add(1, Ordering::SeqCst);
            context.commit(|| Ok(message()))
        })
        .unwrap();
    let mut queued = Vec::new();
    for _ in 0..3 {
        let call = Arc::clone(&called);
        queued.push(
            tasks
                .spawn_blocking(fixture.session(), move |context, _| {
                    call.fetch_add(1, Ordering::SeqCst);
                    context.commit(|| Ok(message()))
                })
                .unwrap(),
        );
    }
    let call = Arc::clone(&called);
    assert_eq!(
        tasks
            .spawn_blocking(fixture.session(), move |_, _| {
                call.fetch_add(100, Ordering::SeqCst);
                Ok(message())
            })
            .unwrap_err(),
        BLOCKING_QUEUE_FULL
    );
    assert_eq!(called.load(Ordering::SeqCst), 0);
    let states = tasks.poll_events(32);
    assert_eq!(
        states
            .iter()
            .filter(|event| event.kind == TaskEventKind::BlockingState(BlockingTaskState::Queued))
            .count(),
        4
    );
    assert_eq!(
        states
            .iter()
            .filter(|event| event.kind == TaskEventKind::BlockingState(BlockingTaskState::Running))
            .count(),
        1
    );
    for event in states {
        assert!(tasks.apply_event(event, |_| {}));
    }
    assert!(tasks.request_cancel(cancelled));
    assert!(
        dropped.load(Ordering::Acquire),
        "queued captures must be dropped before terminal"
    );
    let event = next_event(&mut tasks);
    assert_eq!(event.identity.task_id, cancelled);
    assert_eq!(event.kind, TaskEventKind::Cancelled);
    assert!(tasks.apply_event(event, |_| {}));
    assert!(!tasks.request_cancel(cancelled));
    barrier.release();
    let events = drain(&mut tasks);
    assert_eq!(called.load(Ordering::SeqCst), 3);
    assert_eq!(
        terminal(&events, first),
        vec![(&TaskEventKind::Completed(message()), true)]
    );
    for id in queued {
        assert_eq!(terminal(&events, id).len(), 1);
    }
    assert!(terminal(&events, cancelled).is_empty());
}

#[test]
fn rf215_running_cancel_retains_page_handles_and_execution_slot_until_real_join() {
    let fixture = Fixture::new();
    let mut tasks = ManagedTasks::new(&fixture);
    let (mut barrier, worker) = gate();
    let workspace = tempfile::tempdir_in(fixture.directory.path()).unwrap();
    let workspace_path = workspace.path().to_path_buf();
    let page = workspace.path().join("page.png");
    std::fs::write(&page, b"RF215 synthetic page").unwrap();
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(0x0000_0001 | 0x0000_0002);
    }
    let file = options.open(&page).unwrap();
    let dropped = Arc::new(AtomicBool::new(false));
    let resource = DropFlag(Arc::clone(&dropped));
    let (token_tx, token_rx) = std_mpsc::channel();
    let first = tasks
        .spawn_blocking(fixture.session(), move |context, cancellation| {
            // 析构顺序：页文件先于临时目录关闭，Windows 不允许删除时也能回收。
            let _workspace = workspace;
            let _file = file;
            let _resource = resource;
            token_tx.send(cancellation.clone()).unwrap();
            for index in 0..(PROGRESS_CAPACITY + 10) {
                context.report_progress(index as u64, None);
            }
            worker.pause()?;
            cancellation.check().map_err(|_| TaskFailure::Cancelled)?;
            context.commit(|| Ok(message()))
        })
        .unwrap();
    barrier.wait();
    let token = token_rx.recv_timeout(WAIT).unwrap();
    let next_started = Arc::new(AtomicBool::new(false));
    let worker_started = Arc::clone(&next_started);
    let next = tasks
        .spawn_blocking(fixture.session(), move |context, _| {
            worker_started.store(true, Ordering::Release);
            context.commit(|| Ok(message()))
        })
        .unwrap();
    assert!(tasks.request_cancel(first));
    assert!(tasks.request_cancel(first));
    assert!(token.is_cancelled());
    let mut seen = Vec::new();
    for event in tasks.poll_events(32) {
        assert!(!event.kind.is_terminal());
        assert!(tasks.apply_event(event.clone(), |_| {}));
        seen.push(event);
    }
    assert_eq!(
        seen.iter()
            .filter(|event| event.identity.task_id == first
                && event.kind == TaskEventKind::BlockingState(BlockingTaskState::CancelRequested))
            .count(),
        1
    );
    assert!(workspace_path.exists());
    assert!(!dropped.load(Ordering::Acquire));
    assert!(!next_started.load(Ordering::Acquire));
    barrier.release();
    let events = drain(&mut tasks);
    assert_eq!(
        terminal(&events, first),
        vec![(&TaskEventKind::Cancelled, true)]
    );
    assert_eq!(
        terminal(&events, next),
        vec![(&TaskEventKind::Completed(message()), true)]
    );
    assert!(dropped.load(Ordering::Acquire));
    assert!(!workspace_path.exists());
}

#[test]
fn rf215_shutdown_awaits_running_blocking_work_and_drops_queued_captures() {
    let fixture = Fixture::new();
    let mut tasks = ManagedTasks::new(&fixture);
    let (mut barrier, worker) = gate();
    let (token_tx, token_rx) = std_mpsc::channel();
    let running_dropped = Arc::new(AtomicBool::new(false));
    let running_resource = DropFlag(Arc::clone(&running_dropped));
    tasks
        .spawn_blocking(fixture.session(), move |_, cancellation| {
            let _resource = running_resource;
            token_tx.send(cancellation.clone()).unwrap();
            worker.pause()?;
            cancellation.check().map_err(|_| TaskFailure::Cancelled)?;
            Ok(message())
        })
        .unwrap();
    barrier.wait();
    let token = token_rx.recv_timeout(WAIT).unwrap();
    let queued_dropped = Arc::new(AtomicBool::new(false));
    let queued_resource = DropFlag(Arc::clone(&queued_dropped));
    let queued_called = Arc::new(AtomicBool::new(false));
    let called = Arc::clone(&queued_called);
    tasks
        .spawn_blocking(fixture.session(), move |_, _| {
            let _resource = queued_resource;
            called.store(true, Ordering::Release);
            Ok(message())
        })
        .unwrap();
    // 所有断言和 shutdown Future 位于同一作用域；退出前必 drop 屏障再 join 线程。
    let (done_tx, done_rx) = std_mpsc::channel();
    let thread = std::thread::spawn(move || {
        let report = crate::util::shared_runtime()
            .unwrap()
            .block_on(tasks.shutdown());
        done_tx.send((report, tasks.is_empty())).unwrap();
    });
    struct JoinOnDrop(
        Option<std::thread::JoinHandle<()>>,
        Option<std_mpsc::Sender<()>>,
    );
    impl Drop for JoinOnDrop {
        fn drop(&mut self) {
            if let Some(sender) = self.1.take() {
                let _ = sender.send(());
            }
            if let Some(thread) = self.0.take() {
                let _ = thread.join();
            }
        }
    }
    let mut cleanup = JoinOnDrop(Some(thread), barrier.release.take());
    let deadline = Instant::now() + WAIT;
    while !token.is_cancelled() || !queued_dropped.load(Ordering::Acquire) {
        assert!(
            Instant::now() < deadline,
            "RF215 shutdown must request cancellation"
        );
        std::thread::yield_now();
    }
    assert!(matches!(
        done_rx.try_recv(),
        Err(std_mpsc::TryRecvError::Empty)
    ));
    assert!(!running_dropped.load(Ordering::Acquire));
    assert!(!queued_called.load(Ordering::Acquire));
    cleanup.1.take().unwrap().send(()).unwrap();
    let (report, empty) = done_rx.recv_timeout(WAIT).unwrap();
    assert_eq!(report.joined, 1, "queued closure was never spawned");
    assert_eq!(report.cancelled, 1);
    assert!(empty);
    assert!(running_dropped.load(Ordering::Acquire));
}

#[test]
fn rf215_session_invalidation_cancels_running_and_queued_work_without_releasing_slot() {
    for change in ["lock", "reunlock", "switch"] {
        let fixture = Fixture::new();
        let second_key = if change == "switch" {
            fixture
                .service
                .create_account_with_id(ACCOUNT_B, "RF215 synthetic B", crate::TEST_PASSWORD, None)
                .unwrap();
            let key = fixture.service.get_session_key().unwrap();
            fixture.service.lock();
            fixture
                .service
                .unlock(ACCOUNT_A, crate::TEST_PASSWORD)
                .unwrap();
            Some(key)
        } else {
            None
        };
        let original = fixture.session();
        let original_key = fixture.service.get_session_key().unwrap();
        let generation = original.generation();
        let mut tasks = ManagedTasks::new(&fixture);
        let (mut barrier, worker) = gate();
        let published = Arc::new(AtomicBool::new(false));
        let publish = Arc::clone(&published);
        let first = tasks
            .spawn_blocking(fixture.session(), move |context, _| {
                worker.pause()?;
                context.commit(|| {
                    publish.store(true, Ordering::Release);
                    Ok(message())
                })
            })
            .unwrap();
        barrier.wait();
        let queued_called = Arc::new(AtomicBool::new(false));
        let called = Arc::clone(&queued_called);
        let queued = tasks
            .spawn_blocking(fixture.session(), move |context, _| {
                called.store(true, Ordering::Release);
                context.commit(|| Ok(message()))
            })
            .unwrap();
        fixture.service.lock();
        if change != "lock" {
            let account = if change == "switch" {
                ACCOUNT_B
            } else {
                ACCOUNT_A
            };
            assert_eq!(
                fixture
                    .service
                    .unlock(account, crate::TEST_PASSWORD)
                    .unwrap_err(),
                "IMPORT_OPERATIONS_ACTIVE"
            );
            assert!(!fixture.service.is_unlocked());
            assert!(fixture.service.get_current_account().is_none());
            assert!(fixture.service.get_session_key().is_none());
            assert!(fixture.service.get_vault_store().is_none());
            // A/B key 均来自 job 前的真实认证；使用 PIN/biometric 同一生产入口。
            let key = if change == "switch" {
                second_key.as_ref().unwrap()
            } else {
                &original_key
            };
            fixture
                .service
                .unlock_with_session_key(account, key)
                .unwrap();
            assert_eq!(fixture.session().account_id(), account);
            assert_eq!(fixture.session().generation(), generation.wrapping_add(2));
        }
        assert!(fixture.service.with_session(&original, |_| Ok(())).is_err());
        let stale = tasks.cancel_stale();
        assert!(stale.contains(&first) && stale.contains(&queued));
        assert!(tasks.cancel_stale().is_empty());
        let event = next_event(&mut tasks);
        assert_eq!(event.identity.task_id, queued);
        assert_eq!(event.kind, TaskEventKind::Cancelled);
        assert!(!tasks.apply_event(event, |_| panic!("stale queued result reached UI")));
        assert!(!tasks.is_empty());
        assert!(!queued_called.load(Ordering::Acquire));
        let new_started = Arc::new(AtomicBool::new(false));
        let new_id = if change != "lock" {
            let started = Arc::clone(&new_started);
            Some(
                tasks
                    .spawn_blocking(fixture.session(), move |context, _| {
                        started.store(true, Ordering::Release);
                        context.commit(|| Ok(message()))
                    })
                    .unwrap(),
            )
        } else {
            None
        };
        assert!(!new_started.load(Ordering::Acquire));
        barrier.release();
        let events = drain(&mut tasks);
        assert!(!published.load(Ordering::Acquire));
        assert_eq!(
            terminal(&events, first),
            vec![(&TaskEventKind::Cancelled, false)]
        );
        if let Some(id) = new_id {
            assert_eq!(
                terminal(&events, id),
                vec![(&TaskEventKind::Completed(message()), true)]
            );
        }
    }
}

#[test]
fn rf215_blocking_finish_commit_and_cancel_keep_one_authentic_terminal() {
    let fixture = Fixture::new();
    let mut tasks = ManagedTasks::new(&fixture);
    let (mut barrier, worker) = gate();
    let id = tasks
        .spawn_blocking(fixture.session(), move |context, _| {
            context.commit(|| {
                // 同步 commit 已取得许可；屏障只用于证明 cancel 不改写终态。
                worker
                    .pause()
                    .map_err(|_| "RF215 commit barrier".to_string())?;
                Ok(message())
            })
        })
        .unwrap();
    barrier.wait();
    assert!(!tasks.request_cancel(id));
    assert!(!tasks.request_cancel(id));
    barrier.release();
    let event = loop {
        let event = next_event(&mut tasks);
        if event.kind.is_terminal() {
            break event;
        }
        assert!(tasks.apply_event(event, |_| {}));
    };
    let mut wrong_id = event.clone();
    wrong_id.identity.task_id = TaskId(Uuid::new_v4());
    assert!(!tasks.apply_event(wrong_id, |_| panic!("wrong task accepted")));
    let mut wrong_generation = event.clone();
    wrong_generation.identity.session_generation += 1;
    assert!(!tasks.apply_event(wrong_generation, |_| panic!("wrong generation accepted")));
    let mut wrong_body = event.clone();
    wrong_body.kind = TaskEventKind::Completed(TaskOutput::Message("forged".into()));
    assert!(!tasks.apply_event(wrong_body, |_| panic!("forged result accepted")));
    assert_eq!(event.kind, TaskEventKind::Completed(message()));
    assert!(tasks.apply_event(event.clone(), |_| {}));
    assert!(!tasks.apply_event(event, |_| panic!("duplicate terminal accepted")));
    assert!(tasks.is_empty());
}

#[test]
fn rf215_ready_result_is_rejected_after_same_account_reunlock_without_monitor_poll() {
    let fixture = Fixture::new();
    let mut tasks = ManagedTasks::new(&fixture);
    let id = tasks
        .spawn_blocking(fixture.session(), |context, _| {
            context.commit(|| Ok(message()))
        })
        .unwrap();
    let event = loop {
        let event = next_event(&mut tasks);
        if event.kind.is_terminal() {
            break event;
        }
        assert!(tasks.apply_event(event, |_| {}));
    };
    assert_eq!(event.identity.task_id, id);
    fixture.service.lock();
    fixture
        .service
        .unlock(ACCOUNT_A, crate::TEST_PASSWORD)
        .unwrap();
    assert!(!tasks.apply_event(event, |_| panic!("old OCR text reached new session")));
    assert!(tasks.is_empty());
}

#[test]
fn rf215_native_panic_and_error_release_slot_and_allow_following_work() {
    let fixture = Fixture::new();
    let mut tasks = ManagedTasks::new(&fixture);
    let resource_dropped = Arc::new(AtomicBool::new(false));
    let resource = DropFlag(Arc::clone(&resource_dropped));
    let panic_id = tasks
        .spawn_blocking(fixture.session(), move |_, _| {
            let _resource = resource;
            panic!("RF215 synthetic private panic payload");
        })
        .unwrap();
    let failed = tasks
        .spawn_blocking(fixture.session(), |_, _| {
            Err(TaskFailure::Failed("RF215 synthetic error".into()))
        })
        .unwrap();
    let successful = tasks
        .spawn_blocking(fixture.session(), |context, _| {
            context.commit(|| Ok(message()))
        })
        .unwrap();
    let events = drain(&mut tasks);
    assert!(resource_dropped.load(Ordering::Acquire));
    assert_eq!(
        terminal(&events, panic_id),
        vec![(&TaskEventKind::Failed(TASK_PANICKED.into()), true)]
    );
    assert_eq!(
        terminal(&events, failed),
        vec![(&TaskEventKind::Failed("RF215 synthetic error".into()), true)]
    );
    assert_eq!(
        terminal(&events, successful),
        vec![(&TaskEventKind::Completed(message()), true)]
    );
}

#[test]
fn rf215_native_work_keeps_current_thread_heartbeat_and_async_tasks_live() {
    let fixture = Fixture::new();
    let mut tasks = ManagedTasks::new(&fixture);
    let (mut barrier, worker) = gate();
    let blocking = tasks
        .spawn_blocking(fixture.session(), move |context, _| {
            worker.pause()?;
            context.commit(|| Ok(message()))
        })
        .unwrap();
    barrier.wait();
    let async_id = tasks
        .spawn(fixture.session(), |_| async {
            Ok(TaskOutput::Message("async still runs".into()))
        })
        .unwrap();
    let heartbeat = Arc::new(AtomicUsize::new(0));
    let ticks = Arc::clone(&heartbeat);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        tokio::time::timeout(WAIT, async {
            let beat = tokio::spawn(async move {
                for _ in 0..20 {
                    ticks.fetch_add(1, Ordering::Relaxed);
                    tokio::task::yield_now().await;
                }
            });
            let mut finished_async = false;
            while !finished_async {
                for event in tasks.poll_events(1) {
                    assert!(!(event.identity.task_id == blocking && event.kind.is_terminal()));
                    if event.identity.task_id == async_id && event.kind.is_terminal() {
                        finished_async = true;
                    }
                    assert!(tasks.apply_event(event, |_| {}));
                }
                tokio::task::yield_now().await;
            }
            beat.await.unwrap();
        })
        .await
        .expect("RF215 current-thread heartbeat must remain live");
    });
    assert_eq!(heartbeat.load(Ordering::Relaxed), 20);
    barrier.release();
    let events = drain(&mut tasks);
    assert_eq!(
        terminal(&events, blocking),
        vec![(&TaskEventKind::Completed(message()), true)]
    );
}

#[test]
fn rf215_original_session_guards_queued_start_and_finish_without_stale_polling() {
    let fixture = Fixture::new();
    let original = fixture.session();
    let original_key = fixture.service.get_session_key().unwrap();
    let generation = original.generation();
    let mut tasks = ManagedTasks::new(&fixture);
    let (mut barrier, worker) = gate();
    let published = Arc::new(AtomicBool::new(false));
    let publish = Arc::clone(&published);
    let running = tasks
        .spawn_blocking(fixture.session(), move |context, _| {
            worker.pause()?;
            // 故意不读 cancellation：最终原会话门闩仍必须挡住旧正文。
            context.commit(|| {
                publish.store(true, Ordering::Release);
                Ok(message())
            })
        })
        .unwrap();
    barrier.wait();
    let called = Arc::new(AtomicBool::new(false));
    let worker_called = Arc::clone(&called);
    let queued = tasks
        .spawn_blocking(fixture.session(), move |context, _| {
            worker_called.store(true, Ordering::Release);
            context.commit(|| Ok(message()))
        })
        .unwrap();
    fixture.service.lock();
    assert_eq!(
        fixture
            .service
            .unlock(ACCOUNT_A, crate::TEST_PASSWORD)
            .unwrap_err(),
        "IMPORT_OPERATIONS_ACTIVE"
    );
    assert!(!fixture.service.is_unlocked());
    assert!(fixture.service.get_current_account().is_none());
    assert!(fixture.service.get_session_key().is_none());
    assert!(fixture.service.get_vault_store().is_none());
    // 实际认证的新会话只供 UI/新任务使用，原 worker 的 Session 不得 recapture。
    fixture
        .service
        .unlock_with_session_key(ACCOUNT_A, &original_key)
        .unwrap();
    assert_eq!(fixture.session().generation(), generation.wrapping_add(2));
    assert!(fixture.service.with_session(&original, |_| Ok(())).is_err());
    // 不调用 cancel_stale，也不接纳进度（接纳失败会触发取消）；只能由原始
    // 闭包启动检查和最终 commit 阻止旧工作，避免 UI 轮询偶然满足断言。
    barrier.release();
    let mut events = Vec::new();
    while !tasks.is_empty() {
        let event = next_event(&mut tasks);
        if event.kind.is_terminal() {
            let accepted = tasks.apply_event(event.clone(), |_| panic!("old result accepted"));
            events.push((event, accepted));
        }
    }
    assert!(!published.load(Ordering::Acquire));
    assert!(!called.load(Ordering::Acquire));
    assert_eq!(
        terminal(&events, running),
        vec![(&TaskEventKind::Cancelled, false)]
    );
    assert_eq!(
        terminal(&events, queued),
        vec![(&TaskEventKind::Cancelled, false)]
    );
}
