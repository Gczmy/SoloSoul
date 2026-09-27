use super::{
    capture_ocr_session, EventSink, OcrJobContext, OcrJobEvent, OcrJobState, OcrJobs,
    OCR_DUPLICATE_TASK, OCR_QUEUE_FULL, OCR_SESSION_STALE,
};
use serde_json::{json, Value};
use solosoul_core::ocr::control::{OcrCancellation, OCR_CANCELLED};
use solosoul_core::{VaultService, VaultSession};
use solosoul_vault::ObjectRecord;
use std::future::Future;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex, RwLock};
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use uuid::Uuid;
use zeroize::Zeroizing;

const WATCHDOG: Duration = Duration::from_secs(30);
const SENTINEL_ID: &str = "rf029-existing";
const RESULT_ID: &str = "rf029-published";
type Task = JoinHandle<Result<usize, String>>;
type Work = Box<
    dyn FnOnce(OcrJobContext) -> Pin<Box<dyn Future<Output = Result<usize, String>> + Send>> + Send,
>;

struct Fixture {
    service: Arc<RwLock<VaultService>>,
    published: Arc<AtomicUsize>,
    account_a: String,
    account_b: String,
    key_a: Zeroizing<[u8; 32]>,
    key_b: Zeroizing<[u8; 32]>,
    // Windows：service/数据库先析构，目录最后清理。
    dir: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let service = Arc::new(RwLock::new(VaultService::with_base_path(
            dir.path().join("vault"),
        )));
        let account_a = format!("acc_{}", Uuid::new_v4().simple());
        let account_b = format!("acc_{}", Uuid::new_v4().simple());
        let prepare = |account: &str, name: &str| {
            let svc = service.read().unwrap();
            svc.create_account_with_id(account, name, "RF029-synthetic-password", None)
                .unwrap();
            let vault = svc.get_vault_store().unwrap();
            vault
                .save_object(&ObjectRecord {
                    id: SENTINEL_ID.into(),
                    account_id: account.into(),
                    name: name.into(),
                    type_id: "note".into(),
                    section_type: "identity".into(),
                    properties: json!({"synthetic": name}),
                    created_at: "2026-09-27T00:00:00Z".into(),
                    updated_at: "2026-09-27T00:00:00Z".into(),
                    version: 1,
                    ..Default::default()
                })
                .unwrap();
            vault
                .log_structured(
                    "rf029_seed",
                    "object",
                    Some(SENTINEL_ID),
                    None,
                    account,
                    None,
                )
                .unwrap();
            svc.get_session_key().unwrap()
        };
        let key_a = prepare(&account_a, "RF029 account A");
        let key_b = prepare(&account_b, "RF029 account B");
        service
            .read()
            .unwrap()
            .unlock_with_session_key(&account_a, &key_a)
            .unwrap();
        Self {
            service,
            published: Arc::new(AtomicUsize::new(0)),
            account_a,
            account_b,
            key_a,
            key_b,
            dir,
        }
    }

    fn session(&self) -> VaultSession {
        capture_ocr_session(&self.service).unwrap()
    }

    fn unlock_a(&self) {
        self.service
            .read()
            .unwrap()
            .unlock_with_session_key(&self.account_a, &self.key_a)
            .unwrap();
    }

    fn snapshot(&self) -> Value {
        let svc = self.service.read().unwrap();
        let vault = svc.get_vault_store().unwrap();
        json!({
            "existing": vault.load_object(SENTINEL_ID).unwrap(),
            "result": vault.load_object(RESULT_ID).unwrap(),
            "audit": vault.list_audit_log(100).unwrap(),
        })
    }

    fn both_snapshots(&self) -> [Value; 2] {
        self.unlock_a();
        let first = self.snapshot();
        self.service
            .read()
            .unwrap()
            .unlock_with_session_key(&self.account_b, &self.key_b)
            .unwrap();
        let second = self.snapshot();
        self.unlock_a();
        [first, second]
    }

    fn invalidate(&self, mode: &str) {
        // 不等待锁：若 coordinator/worker 错误跨推理持服务读锁，直接失败并释放屏障。
        let svc = self
            .service
            .try_write()
            .expect("OCR worker 等待期间不能持有 service 锁");
        match mode {
            "lock" => svc.lock(),
            "switch" => svc
                .unlock_with_session_key(&self.account_b, &self.key_b)
                .unwrap(),
            "reunlock" => {
                svc.lock();
                svc.unlock_with_session_key(&self.account_a, &self.key_a)
                    .unwrap();
            }
            _ => panic!("unknown synthetic session transition"),
        }
    }
}

#[derive(Clone, Debug)]
struct Observed {
    task_id: String,
    account_id: String,
    generation: u64,
    state: OcrJobState,
    sequence: u64,
}

#[derive(Clone, Default)]
struct Events(Arc<Mutex<Vec<Observed>>>);

impl Events {
    fn sink(&self) -> EventSink {
        let events = self.0.clone();
        Arc::new(move |event: OcrJobEvent| {
            events.lock().unwrap().push(Observed {
                task_id: event.task_id,
                account_id: event.account_id,
                generation: event.session_generation,
                state: event.state,
                sequence: event.sequence,
            });
        })
    }

    fn has(&self, id: &str, state: OcrJobState) -> bool {
        self.0
            .lock()
            .unwrap()
            .iter()
            .any(|event| event.task_id == id && event.state == state)
    }

    fn assert_terminal(&self, id: &str, account: &str, generation: u64, expected: OcrJobState) {
        let events = self.0.lock().unwrap();
        let mut rows: Vec<_> = events.iter().filter(|event| event.task_id == id).collect();
        assert!(!rows.is_empty(), "task must emit its identity");
        assert!(rows
            .iter()
            .all(|event| event.account_id == account && event.generation == generation));
        rows.sort_by_key(|event| event.sequence);
        assert!(
            rows.windows(2)
                .all(|pair| pair[0].sequence < pair[1].sequence),
            "sequence must be unique even when emission order differs"
        );
        let terminal: Vec<_> = rows
            .iter()
            .filter(|event| is_terminal(event.state))
            .collect();
        assert_eq!(terminal.len(), 1, "exactly one terminal: {rows:?}");
        assert_eq!(terminal[0].state, expected, "{rows:?}");
        assert_eq!(rows.last().unwrap().state, expected, "{rows:?}");
    }
}

fn is_terminal(state: OcrJobState) -> bool {
    matches!(
        state,
        OcrJobState::Completed | OcrJobState::Cancelled | OcrJobState::Failed | OcrJobState::Stale
    )
}

fn task_id() -> String {
    Uuid::new_v4().to_string()
}

fn launch<F, Fut>(f: &Fixture, jobs: &Arc<OcrJobs>, events: &Events, id: &str, work: F) -> Task
where
    F: FnOnce(OcrJobContext) -> Fut + Send + 'static,
    Fut: Future<Output = Result<usize, String>> + Send + 'static,
{
    let service = f.service.clone();
    let session = f.session();
    let account = session.account_id().to_owned();
    let jobs = jobs.clone();
    let emit = events.sink();
    let id = id.to_owned();
    let published = f.published.clone();
    tokio::spawn(async move {
        jobs.run(
            Some(id),
            service,
            session,
            emit,
            work,
            move |vault, value| {
                published.fetch_add(1, Ordering::SeqCst);
                vault.save_object(&ObjectRecord {
                    id: RESULT_ID.into(),
                    account_id: account.clone(),
                    name: "RF029 synthetic OCR result".into(),
                    type_id: "note".into(),
                    section_type: "identity".into(),
                    properties: json!({"value": value}),
                    created_at: "2026-09-27T00:00:00Z".into(),
                    updated_at: "2026-09-27T00:00:00Z".into(),
                    version: 1,
                    ..Default::default()
                })?;
                vault.log_structured(
                    "rf029_publish",
                    "object",
                    Some(RESULT_ID),
                    None,
                    &account,
                    None,
                )
            },
        )
        .await
    })
}

async fn settle(task: Task) -> Result<usize, String> {
    tokio::time::timeout(WATCHDOG, task)
        .await
        .expect("OCR caller did not settle")
        .expect("OCR caller must not panic")
}

async fn wait_for(label: &str, predicate: impl Fn() -> bool) {
    tokio::time::timeout(WATCHDOG, async {
        while !predicate() {
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for {label}"));
}

async fn receive<T>(rx: oneshot::Receiver<T>) -> T {
    tokio::time::timeout(WATCHDOG, rx)
        .await
        .expect("native worker did not reach barrier")
        .unwrap_or_else(|_| panic!("native worker exited before barrier"))
}

struct Release(Option<mpsc::Sender<()>>);

impl Release {
    fn now(&mut self) {
        if let Some(tx) = self.0.take() {
            let _ = tx.send(());
        }
    }
}

impl Drop for Release {
    fn drop(&mut self) {
        self.now();
    }
}

struct WorkerReady {
    workspace: PathBuf,
    thread: std::thread::ThreadId,
    cancellation: OcrCancellation,
}

/// 真 spawn_blocking 持有临时目录及不共享删除权限的文件；取消不终止模拟原生调用。
/// 每个接收均有 watchdog，Release 在测试断言 unwind 时同样解除阻塞。
fn paused_native(
    root: &Path,
    value: usize,
    result_ready: bool,
) -> (Work, Release, oneshot::Receiver<WorkerReady>) {
    let root = root.to_path_buf();
    let (entered_tx, entered_rx) = oneshot::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let work: Work = Box::new(move |context| {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                context.checkpoint()?;
                let temp = tempfile::Builder::new()
                    .prefix("rf029-native-")
                    .tempdir_in(root)
                    .map_err(|error| error.to_string())?;
                let path = temp.path().join("synthetic-page.bin");
                let mut options = std::fs::OpenOptions::new();
                options.read(true).write(true).create_new(true);
                #[cfg(windows)]
                {
                    use std::os::windows::fs::OpenOptionsExt;
                    options.share_mode(1 | 2); // FILE_SHARE_READ | FILE_SHARE_WRITE，排除 DELETE。
                }
                let mut page = options.open(path).map_err(|error| error.to_string())?;
                page.write_all(b"RF029 synthetic native resource")
                    .map_err(|error| error.to_string())?;
                let ready_value = result_ready.then(|| std::hint::black_box(value));
                entered_tx
                    .send(WorkerReady {
                        workspace: temp.path().to_owned(),
                        thread: std::thread::current().id(),
                        cancellation: context.cancellation(),
                    })
                    .map_err(|_| "native barrier receiver closed".to_string())?;
                release_rx
                    .recv_timeout(WATCHDOG)
                    .map_err(|error| format!("native watchdog: {error}"))?;
                // 特意不做结束 checkpoint：调度器必须独立拒绝迟到的成功结果。
                Ok(ready_value.unwrap_or_else(|| std::hint::black_box(value)))
            })
            .await
            .map_err(|_| "RF029 native worker join failure".to_string())?
        })
    });
    (work, Release(Some(release_tx)), entered_rx)
}

#[tokio::test(flavor = "current_thread")]
async fn rf029_bounded_queue_rejects_sixth_and_cancelled_queued_work_never_starts() {
    let f = Fixture::new();
    let jobs = Arc::new(OcrJobs::new());
    let events = Events::default();
    let generation = f.session().generation();
    let first = task_id();
    let (work, mut release, entered) = paused_native(f.dir.path(), 1, false);
    let running = launch(&f, &jobs, &events, &first, work);
    let ready = receive(entered).await;
    let starts = Arc::new(AtomicUsize::new(0));
    let mut waiting = Vec::new();
    for value in 2..=5 {
        let id = task_id();
        let count = starts.clone();
        let task = launch(&f, &jobs, &events, &id, move |_| async move {
            count.fetch_add(1, Ordering::SeqCst);
            Ok(value)
        });
        wait_for("queued task", || events.has(&id, OcrJobState::Queued)).await;
        waiting.push((id, task));
    }
    assert_eq!(starts.load(Ordering::SeqCst), 0);
    let sixth = task_id();
    let count = starts.clone();
    let rejected = launch(&f, &jobs, &events, &sixth, move |_| async move {
        count.fetch_add(1, Ordering::SeqCst);
        Ok(6)
    });
    assert_eq!(settle(rejected).await.unwrap_err(), OCR_QUEUE_FULL);
    assert_eq!(starts.load(Ordering::SeqCst), 0);

    let (cancelled_id, cancelled) = waiting.remove(1);
    assert!(jobs
        .cancel(&cancelled_id, &f.service, &f.session())
        .unwrap());
    assert_eq!(settle(cancelled).await.unwrap_err(), OCR_CANCELLED);
    events.assert_terminal(
        &cancelled_id,
        &f.account_a,
        generation,
        OcrJobState::Cancelled,
    );
    assert!(!jobs
        .cancel(&cancelled_id, &f.service, &f.session())
        .unwrap());
    assert!(ready.workspace.exists());
    assert_eq!(starts.load(Ordering::SeqCst), 0);
    release.now();
    assert_eq!(settle(running).await.unwrap(), 1);
    for (_, task) in waiting {
        settle(task).await.unwrap();
    }
    assert_eq!(starts.load(Ordering::SeqCst), 3);
    assert_eq!(f.published.load(Ordering::SeqCst), 4);
    assert!(!ready.workspace.exists());
}

#[tokio::test(flavor = "current_thread")]
async fn rf029_running_cancel_holds_execution_and_file_until_actual_native_exit() {
    let f = Fixture::new();
    let jobs = Arc::new(OcrJobs::new());
    let events = Events::default();
    let generation = f.session().generation();
    let first = task_id();
    let (work, mut release, entered) = paused_native(f.dir.path(), 7, false);
    let running = launch(&f, &jobs, &events, &first, work);
    let ready = receive(entered).await;
    assert_ne!(ready.thread, std::thread::current().id());
    let next_id = task_id();
    let starts = Arc::new(AtomicUsize::new(0));
    let count = starts.clone();
    let next = launch(&f, &jobs, &events, &next_id, move |_| async move {
        count.fetch_add(1, Ordering::SeqCst);
        Ok(8)
    });
    wait_for("second task queued", || {
        events.has(&next_id, OcrJobState::Queued)
    })
    .await;
    assert!(jobs.cancel(&first, &f.service, &f.session()).unwrap());
    jobs.cancel(&first, &f.service, &f.session()).unwrap();
    wait_for("cancel requested", || {
        events.has(&first, OcrJobState::CancelRequested)
    })
    .await;
    assert!(ready.cancellation.is_cancelled());
    assert!(
        !running.is_finished(),
        "cancel acknowledgement is not terminal"
    );
    assert!(ready.workspace.join("synthetic-page.bin").is_file());
    assert_eq!(starts.load(Ordering::SeqCst), 0);
    assert_eq!(f.published.load(Ordering::SeqCst), 0);
    assert!(!events.has(&first, OcrJobState::Cancelled));
    assert!(f.service.try_write().is_ok());
    release.now();
    assert_eq!(settle(running).await.unwrap_err(), OCR_CANCELLED);
    assert!(!ready.workspace.exists());
    assert_eq!(settle(next).await.unwrap(), 8);
    assert_eq!(starts.load(Ordering::SeqCst), 1);
    assert_eq!(f.published.load(Ordering::SeqCst), 1);
    events.assert_terminal(&first, &f.account_a, generation, OcrJobState::Cancelled);
    events.assert_terminal(&next_id, &f.account_a, generation, OcrJobState::Completed);
}

#[tokio::test(flavor = "current_thread")]
async fn rf029_aborted_invoke_keeps_background_worker_and_admission_until_settlement() {
    let f = Fixture::new();
    let jobs = Arc::new(OcrJobs::new());
    let events = Events::default();
    let generation = f.session().generation();
    let first = task_id();
    let (work, mut release, entered) = paused_native(f.dir.path(), 9, false);
    let caller = launch(&f, &jobs, &events, &first, work);
    let ready = receive(entered).await;
    caller.abort();
    assert!(tokio::time::timeout(WATCHDOG, caller)
        .await
        .unwrap()
        .unwrap_err()
        .is_cancelled());

    let starts = Arc::new(AtomicUsize::new(0));
    let mut queued = Vec::new();
    for _ in 0..4 {
        let id = task_id();
        let count = starts.clone();
        let task = launch(&f, &jobs, &events, &id, move |_| async move {
            count.fetch_add(1, Ordering::SeqCst);
            Ok(10)
        });
        wait_for("queued after caller abort", || {
            events.has(&id, OcrJobState::Queued)
        })
        .await;
        queued.push((id, task));
    }
    let id = task_id();
    assert_eq!(
        settle(launch(&f, &jobs, &events, &id, |_| async { Ok(11) }))
            .await
            .unwrap_err(),
        OCR_QUEUE_FULL
    );
    assert!(ready.workspace.exists());
    assert_eq!(starts.load(Ordering::SeqCst), 0);
    for (id, task) in queued {
        assert!(jobs.cancel(&id, &f.service, &f.session()).unwrap());
        assert_eq!(settle(task).await.unwrap_err(), OCR_CANCELLED);
    }
    release.now();
    wait_for("detached coordinator completed", || {
        events.has(&first, OcrJobState::Completed)
    })
    .await;
    wait_for("detached worker directory removed", || {
        !ready.workspace.exists()
    })
    .await;
    assert_eq!(f.published.load(Ordering::SeqCst), 1);
    events.assert_terminal(&first, &f.account_a, generation, OcrJobState::Completed);
    let id = task_id();
    assert_eq!(
        settle(launch(&f, &jobs, &events, &id, |_| async { Ok(12) }))
            .await
            .unwrap(),
        12
    );
}

#[tokio::test(flavor = "current_thread")]
async fn rf029_stale_queued_running_and_ready_results_never_publish_to_either_account() {
    let f = Fixture::new();
    let before = f.both_snapshots();
    for stage in ["queued", "running", "result-ready"] {
        for transition in ["lock", "switch", "reunlock"] {
            f.unlock_a();
            let jobs = Arc::new(OcrJobs::new());
            let events = Events::default();
            let generation = f.session().generation();
            let id = task_id();
            let (work, mut release, entered) =
                paused_native(f.dir.path(), 13, stage == "result-ready");
            let starts = Arc::new(AtomicUsize::new(0));
            let (task, leader, ready) = if stage == "queued" {
                let leader_id = task_id();
                let leader = launch(&f, &jobs, &events, &leader_id, work);
                let ready = receive(entered).await;
                let count = starts.clone();
                let task = launch(&f, &jobs, &events, &id, move |_| async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Ok(14)
                });
                wait_for("old account task queued", || {
                    events.has(&id, OcrJobState::Queued)
                })
                .await;
                (task, Some(leader), ready)
            } else {
                let task = launch(&f, &jobs, &events, &id, work);
                (task, None, receive(entered).await)
            };

            f.invalidate(transition);
            if transition != "lock" {
                assert!(jobs.cancel(&id, &f.service, &f.session()).is_err());
            }
            if stage == "queued" {
                // 取消排队任务无需等待另一不可中断 worker；也不能调用该任务的 work。
                assert_eq!(settle(task).await.unwrap_err(), OCR_SESSION_STALE);
                assert_eq!(starts.load(Ordering::SeqCst), 0);
                release.now();
                assert_eq!(
                    settle(leader.unwrap()).await.unwrap_err(),
                    OCR_SESSION_STALE
                );
            } else {
                if stage == "running" {
                    wait_for("poller marks original session stale", || {
                        ready.cancellation.is_cancelled()
                    })
                    .await;
                    assert!(!task.is_finished());
                    assert!(ready.workspace.exists());
                }
                // ready 分支不等待 50ms poll：最终提交必须自行核对会话。
                release.now();
                assert_eq!(settle(task).await.unwrap_err(), OCR_SESSION_STALE);
            }
            assert!(!ready.workspace.exists(), "{stage}/{transition}");
            events.assert_terminal(&id, &f.account_a, generation, OcrJobState::Stale);
            assert_eq!(f.published.load(Ordering::SeqCst), 0);
            assert_eq!(f.both_snapshots(), before, "{stage}/{transition}");
        }
    }
}

#[tokio::test(flavor = "current_thread")]
async fn rf029_finish_wins_cancel_and_active_or_recent_uuid_cannot_be_reused() {
    let f = Fixture::new();
    let jobs = Arc::new(OcrJobs::new());
    let events = Events::default();
    let generation = f.session().generation();
    let id = task_id();
    let (work, mut release, entered) = paused_native(f.dir.path(), 15, true);
    let first = launch(&f, &jobs, &events, &id, work);
    let ready = receive(entered).await;
    let duplicate_starts = Arc::new(AtomicUsize::new(0));
    let count = duplicate_starts.clone();
    assert_eq!(
        settle(launch(&f, &jobs, &events, &id, move |_| async move {
            count.fetch_add(1, Ordering::SeqCst);
            Ok(16)
        }))
        .await
        .unwrap_err(),
        OCR_DUPLICATE_TASK
    );
    release.now();
    assert_eq!(settle(first).await.unwrap(), 15);
    assert!(!ready.workspace.exists());
    for _ in 0..2 {
        assert!(!jobs.cancel(&id, &f.service, &f.session()).unwrap());
    }
    assert_eq!(
        settle(launch(&f, &jobs, &events, &id, |_| async { Ok(17) }))
            .await
            .unwrap_err(),
        OCR_DUPLICATE_TASK
    );
    assert_eq!(duplicate_starts.load(Ordering::SeqCst), 0);
    assert_eq!(f.published.load(Ordering::SeqCst), 1);
    events.assert_terminal(&id, &f.account_a, generation, OcrJobState::Completed);
    let snapshot = f.snapshot();
    assert_eq!(snapshot["result"]["properties"]["value"], 15);
    assert_eq!(
        snapshot["audit"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["actionType"] == "rf029_publish")
            .count(),
        1
    );
}

#[tokio::test(flavor = "current_thread")]
async fn rf029_native_panic_releases_slot_and_current_thread_runtime_keeps_advancing() {
    let f = Fixture::new();
    let jobs = Arc::new(OcrJobs::new());
    let events = Events::default();
    let generation = f.session().generation();
    let id = task_id();
    let (entered_tx, entered_rx) = oneshot::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let mut release = Release(Some(release_tx));
    let task = launch(&f, &jobs, &events, &id, move |context| async move {
        context.checkpoint()?;
        tokio::task::spawn_blocking(move || -> Result<usize, String> {
            let _ = entered_tx.send(std::thread::current().id());
            release_rx
                .recv_timeout(WATCHDOG)
                .map_err(|error| error.to_string())?;
            panic!("RF029_SYNTHETIC_NATIVE_PANIC");
        })
        .await
        .map_err(|_| "RF029 native worker failed".to_string())?
    });
    let native_thread = receive(entered_rx).await;
    assert_ne!(native_thread, std::thread::current().id());
    let heartbeat = tokio::spawn(async {
        tokio::task::yield_now().await;
        std::thread::current().id()
    });
    let runtime_thread = tokio::time::timeout(WATCHDOG, heartbeat)
        .await
        .expect("current_thread runtime stalled behind native work")
        .unwrap();
    assert_ne!(native_thread, runtime_thread);
    assert!(!task.is_finished());
    assert!(f.service.try_write().is_ok());
    release.now();
    assert!(settle(task).await.is_err());
    assert_eq!(f.published.load(Ordering::SeqCst), 0);
    events.assert_terminal(&id, &f.account_a, generation, OcrJobState::Failed);
    let next = task_id();
    assert_eq!(
        settle(launch(&f, &jobs, &events, &next, |_| async { Ok(18) }))
            .await
            .unwrap(),
        18
    );
    events.assert_terminal(&next, &f.account_a, generation, OcrJobState::Completed);
    assert_eq!(f.published.load(Ordering::SeqCst), 1);
}

#[tokio::test(flavor = "current_thread")]
async fn rf029_old_session_is_rejected_at_admission_before_work_or_publish() {
    let f = Fixture::new();
    let jobs = Arc::new(OcrJobs::new());
    let session = f.session();
    f.invalidate("reunlock");
    let starts = Arc::new(AtomicUsize::new(0));
    let count = starts.clone();
    let published = f.published.clone();
    let result = tokio::time::timeout(
        WATCHDOG,
        jobs.run(
            Some(task_id()),
            f.service.clone(),
            session,
            Events::default().sink(),
            move |_| async move {
                count.fetch_add(1, Ordering::SeqCst);
                Ok(19usize)
            },
            move |_, _| {
                published.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
        ),
    )
    .await
    .unwrap();
    assert_eq!(result.unwrap_err(), OCR_SESSION_STALE);
    assert_eq!(starts.load(Ordering::SeqCst), 0);
    assert_eq!(f.published.load(Ordering::SeqCst), 0);
    f.service.read().unwrap().lock();
    assert!(capture_ocr_session(&f.service).is_err());
}

#[tokio::test(flavor = "current_thread")]
async fn rf029_same_account_and_generation_from_another_vault_cannot_cancel() {
    let f = Fixture::new();
    let jobs = Arc::new(OcrJobs::new());
    let events = Events::default();
    let expected = f.session();
    let id = task_id();

    let other_dir = tempfile::tempdir().unwrap();
    let other_service = Arc::new(RwLock::new(VaultService::with_base_path(
        other_dir.path().join("vault"),
    )));
    let key = {
        let svc = other_service.read().unwrap();
        svc.create_account_with_id(
            &f.account_a,
            "RF029 same ID in another directory",
            "RF029-other-synthetic-password",
            None,
        )
        .unwrap();
        svc.get_session_key().unwrap()
    };
    // 配平真实会话代次，确保本例必须依赖 vault 身份，不能只靠 account/generation。
    for _ in 0..expected.generation() {
        if capture_ocr_session(&other_service).unwrap().generation() == expected.generation() {
            break;
        }
        other_service
            .read()
            .unwrap()
            .unlock_with_session_key(&f.account_a, &key)
            .unwrap();
    }
    let foreign = capture_ocr_session(&other_service).unwrap();
    assert_eq!(foreign.account_id(), expected.account_id());
    assert_eq!(foreign.generation(), expected.generation());
    assert!(!std::ptr::eq(foreign.vault(), expected.vault()));
    let (work, mut release, entered) = paused_native(f.dir.path(), 20, false);
    let task = launch(&f, &jobs, &events, &id, work);
    let ready = receive(entered).await;

    assert_eq!(
        jobs.cancel(&id, &other_service, &foreign).unwrap_err(),
        OCR_SESSION_STALE
    );
    assert!(!ready.cancellation.is_cancelled());
    assert!(ready.workspace.exists());
    release.now();
    assert_eq!(settle(task).await.unwrap(), 20);
    events.assert_terminal(
        &id,
        &f.account_a,
        expected.generation(),
        OcrJobState::Completed,
    );
    assert_eq!(f.published.load(Ordering::SeqCst), 1);
    drop(foreign);
    drop(other_service);
    drop(other_dir);
}

#[tokio::test(flavor = "current_thread")]
async fn rf029_work_future_unwind_is_failed_and_does_not_poison_later_admission() {
    let f = Fixture::new();
    let jobs = Arc::new(OcrJobs::new());
    let events = Events::default();
    let generation = f.session().generation();
    let id = task_id();
    let task = launch(&f, &jobs, &events, &id, |_| async {
        tokio::task::yield_now().await;
        panic!("RF029_SYNTHETIC_FUTURE_PANIC");
        #[allow(unreachable_code)]
        Ok(0usize)
    });
    let error = settle(task).await.unwrap_err();
    assert!(!error.contains("RF029_SYNTHETIC_FUTURE_PANIC"));
    events.assert_terminal(&id, &f.account_a, generation, OcrJobState::Failed);
    assert_eq!(f.published.load(Ordering::SeqCst), 0);
    let next = task_id();
    assert_eq!(
        settle(launch(&f, &jobs, &events, &next, |_| async { Ok(21) }))
            .await
            .unwrap(),
        21
    );
    assert_eq!(f.published.load(Ordering::SeqCst), 1);
}

#[tokio::test(flavor = "current_thread")]
async fn rf029_finish_rechecks_session_when_work_invalidates_it_in_its_final_poll() {
    let f = Fixture::new();
    let before = f.both_snapshots();
    for transition in ["lock", "switch", "reunlock"] {
        f.unlock_a();
        let jobs = Arc::new(OcrJobs::new());
        let events = Events::default();
        let generation = f.session().generation();
        let id = task_id();
        let (native_work, mut release, entered) = paused_native(f.dir.path(), 22, true);
        let service = f.service.clone();
        let account_a = f.account_a.clone();
        let account_b = f.account_b.clone();
        let key_a = f.key_a.clone();
        let key_b = f.key_b.clone();
        let invalidated = Arc::new(AtomicUsize::new(0));
        let changed = invalidated.clone();
        let task = launch(&f, &jobs, &events, &id, move |context| async move {
            let cancellation = context.cancellation();
            let value = native_work(context).await?;
            // 从这里到 Ready 不再 await：current_thread 的 monitor 无机会先设置 stale。
            // native JoinHandle 已返回，Windows 页句柄及目录 owner 也已实际析构。
            assert!(!cancellation.is_cancelled());
            {
                let svc = service
                    .try_write()
                    .map_err(|_| "RF029 final poll unexpectedly holds service lock".to_string())?;
                match transition {
                    "lock" => svc.lock(),
                    "switch" => svc.unlock_with_session_key(&account_b, &key_b)?,
                    "reunlock" => {
                        svc.lock();
                        svc.unlock_with_session_key(&account_a, &key_a)?;
                    }
                    _ => unreachable!(),
                }
            }
            assert!(
                !cancellation.is_cancelled(),
                "final session transition must precede the monitor's next poll"
            );
            changed.fetch_add(1, Ordering::SeqCst);
            Ok(value)
        });
        let ready = receive(entered).await;
        assert!(ready.workspace.exists());
        assert!(!ready.cancellation.is_cancelled());
        release.now();
        assert_eq!(settle(task).await.unwrap_err(), OCR_SESSION_STALE);
        assert_eq!(invalidated.load(Ordering::SeqCst), 1, "{transition}");
        assert!(!ready.workspace.exists(), "{transition}");
        assert!(
            !events.has(&id, OcrJobState::CancelRequested),
            "poller/worker checkpoint must not mask final-commit validation: {transition}"
        );
        events.assert_terminal(&id, &f.account_a, generation, OcrJobState::Stale);
        assert_eq!(f.published.load(Ordering::SeqCst), 0, "{transition}");
        assert_eq!(f.both_snapshots(), before, "{transition}");
    }
}
