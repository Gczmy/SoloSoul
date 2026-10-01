//! RF215：真实 App/Tasks 管理合成阻塞引擎；不加载 ONNX、不访问网络。
//! 引擎替身刻意不检查取消，确保取消判定来自生产调度入口。

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, ThreadId};
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent};
use solosoul_core::ocr::control::OcrCancellation;
use solosoul_core::ocr::types::{MrzResult, OcrBox, OcrModelTier, OcrResult};
use solosoul_core::VaultService;
use tempfile::TempDir;
use zeroize::Zeroizing;

use super::{start_scan_with, OcrRequest, ScanEngine};
use crate::app::{App, AppPhase, UnlockStep};
use crate::commands::auth;
use crate::events::Event;
use crate::tasks::{BlockingTaskState, TaskEvent, TaskEventKind, TaskId, TaskOutput};

const WAIT: Duration = Duration::from_secs(20);
const TEXT: &str = "RF215 synthetic recognition 中文";

#[derive(Clone)]
struct Release {
    sender: Arc<Mutex<Option<mpsc::Sender<()>>>>,
    opened: Arc<AtomicBool>,
}

impl Release {
    fn open(&self) {
        self.opened.store(true, Ordering::SeqCst);
        if let Some(sender) = self.sender.lock().unwrap_or_else(|e| e.into_inner()).take() {
            let _ = sender.send(());
        }
    }
}

#[derive(Clone, Default)]
struct Probe {
    loads: Arc<AtomicUsize>,
    drops: Arc<AtomicUsize>,
    calls: Arc<Mutex<Vec<&'static str>>>,
    resource: Arc<Mutex<Option<PathBuf>>>,
}

impl Probe {
    fn resource(&self) -> PathBuf {
        self.resource
            .lock()
            .unwrap()
            .clone()
            .expect("engine resource exists after load")
    }

    fn calls(&self) -> Vec<&'static str> {
        self.calls.lock().unwrap().clone()
    }
}

#[derive(Clone, Copy)]
enum Pause {
    Scan,
    Load,
    None,
}

#[derive(Clone, Copy)]
enum Fault {
    None,
    Load,
    Scan,
    Panic,
    MrzNone,
}

struct LoadNotice {
    models: PathBuf,
    tier: OcrModelTier,
    thread: ThreadId,
}

struct ScanNotice {
    route: &'static str,
    path: PathBuf,
    cancel: OcrCancellation,
    thread: ThreadId,
}

struct FakeEngine {
    probe: Probe,
    release: Option<mpsc::Receiver<()>>,
    entered: mpsc::Sender<ScanNotice>,
    fault: Fault,
    file: Option<File>,
    directory: Option<TempDir>,
}

impl FakeEngine {
    fn enter(
        &mut self,
        route: &'static str,
        path: &Path,
        cancel: &OcrCancellation,
    ) -> Result<(), String> {
        self.probe.calls.lock().unwrap().push(route);
        self.entered
            .send(ScanNotice {
                route,
                path: path.to_path_buf(),
                cancel: cancel.clone(),
                thread: thread::current().id(),
            })
            .map_err(|e| e.to_string())?;
        if let Some(release) = self.release.take() {
            release
                .recv_timeout(WAIT)
                .map_err(|e| format!("synthetic engine barrier: {e}"))?;
        }
        match self.fault {
            Fault::Scan => Err("RF215 synthetic scan failure".into()),
            Fault::Panic => panic!("RF215 synthetic engine panic"),
            _ => Ok(()),
        }
    }
}

impl ScanEngine for FakeEngine {
    fn image(&mut self, path: &Path, cancel: &OcrCancellation) -> Result<OcrResult, String> {
        self.enter("image", path, cancel)?;
        Ok(ocr_result())
    }

    fn pdf(&mut self, path: &Path, cancel: &OcrCancellation) -> Result<OcrResult, String> {
        self.enter("pdf", path, cancel)?;
        Ok(ocr_result())
    }

    fn mrz(&mut self, path: &Path, cancel: &OcrCancellation) -> Result<Option<MrzResult>, String> {
        self.enter("mrz", path, cancel)?;
        Ok(if matches!(self.fault, Fault::MrzNone) {
            None
        } else {
            Some(mrz_result())
        })
    }
}

impl Drop for FakeEngine {
    fn drop(&mut self) {
        drop(self.file.take());
        drop(self.directory.take());
        self.probe.drops.fetch_add(1, Ordering::SeqCst);
    }
}

struct Pending {
    id: TaskId,
    release: Release,
    loaded: mpsc::Receiver<LoadNotice>,
    entered: mpsc::Receiver<ScanNotice>,
    probe: Probe,
    path: PathBuf,
}

impl Pending {
    fn loaded(&self) -> LoadNotice {
        self.loaded
            .recv_timeout(WAIT)
            .expect("actual blocking loader must start")
    }

    fn entered(&self) -> ScanNotice {
        self.entered
            .recv_timeout(WAIT)
            .expect("actual engine must enter its barrier")
    }

    fn assert_reclaimed(&self) {
        assert_eq!(self.probe.drops.load(Ordering::SeqCst), 1);
        assert!(
            !self.probe.resource().exists(),
            "owned engine directory must be reclaimed after real join"
        );
    }
}

struct Fixture {
    app: App,
    account_a: String,
    account_b: Option<String>,
    account_a_key: Zeroizing<[u8; 32]>,
    account_b_key: Option<Zeroizing<[u8; 32]>>,
    gates: Vec<Release>,
    directory: TempDir,
}

impl Fixture {
    fn new(second_account: bool) -> Self {
        let directory = TempDir::new().unwrap();
        let service = VaultService::with_base_path(directory.path().to_path_buf());
        let account_a = service
            .create_account("RF215 A", crate::TEST_PASSWORD, None)
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();
        let second = second_account.then(|| {
            let account = service
                .create_account("RF215 B", crate::TEST_PASSWORD, None)
                .unwrap()["id"]
                .as_str()
                .unwrap()
                .to_string();
            (account, service.get_session_key().unwrap())
        });
        let (account_b, account_b_key) = match second {
            Some((account, key)) => (Some(account), Some(key)),
            None => (None, None),
        };
        service.lock();
        let mut app = App::new(Arc::new(service)).unwrap();
        app.i18n.set_locale("en-US");
        unlock_account(&mut app, &account_a);
        // key 由真实密码向导成功后采集；不会伪造认证或持久化密钥。
        let account_a_key = app.vault_service.get_session_key().unwrap();
        Self {
            app,
            account_a,
            account_b,
            account_a_key,
            account_b_key,
            gates: Vec::new(),
            directory,
        }
    }

    fn start(&mut self, extension: &str, mrz: bool, pause: Pause, fault: Fault) -> Pending {
        let path =
            self.directory
                .path()
                .join(format!("synthetic-{}.{}", self.gates.len(), extension));
        std::fs::write(&path, b"RF215 fake-engine input only").unwrap();
        let models = self.directory.path().join("models");
        let engine_parent = self.directory.path().to_path_buf();
        let (send_release, receive_release) = mpsc::channel();
        let release = Release {
            sender: Arc::new(Mutex::new(Some(send_release))),
            opened: Arc::new(AtomicBool::new(false)),
        };
        self.gates.push(release.clone());
        let (loaded, receive_loaded) = mpsc::channel();
        let (entered, receive_entered) = mpsc::channel();
        let probe = Probe::default();
        let worker_probe = probe.clone();
        let id = start_scan_with(
            &mut self.app,
            OcrRequest {
                path: path.clone(),
                models_dir: models,
                tier: OcrModelTier::Tiny,
                mrz,
            },
            move |models, tier| {
                worker_probe.loads.fetch_add(1, Ordering::SeqCst);
                if matches!(fault, Fault::Load) {
                    return Err("RF215 synthetic model load failure".into());
                }
                let directory = tempfile::Builder::new()
                    .prefix("rf215-engine-")
                    .tempdir_in(engine_parent)
                    .map_err(|e| e.to_string())?;
                let file = File::create(directory.path().join("owned-native-resource"))
                    .map_err(|e| e.to_string())?;
                *worker_probe.resource.lock().unwrap() = Some(directory.path().to_path_buf());
                let mut engine = FakeEngine {
                    probe: worker_probe,
                    release: Some(receive_release),
                    entered,
                    fault,
                    file: Some(file),
                    directory: Some(directory),
                };
                loaded
                    .send(LoadNotice {
                        models: models.to_path_buf(),
                        tier,
                        thread: thread::current().id(),
                    })
                    .map_err(|e| e.to_string())?;
                if matches!(pause, Pause::Load) {
                    engine
                        .release
                        .take()
                        .unwrap()
                        .recv_timeout(WAIT)
                        .map_err(|e| format!("synthetic loader barrier: {e}"))?;
                } else if matches!(pause, Pause::None) {
                    drop(engine.release.take());
                }
                Ok(engine)
            },
        )
        .expect("production OCR entry admits synthetic engine");
        Pending {
            id,
            release,
            loaded: receive_loaded,
            entered: receive_entered,
            probe,
            path,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // 断言失败也先放行所有阻塞资源，再等待真实 Tasks join，最后清临时 Vault。
        for gate in &self.gates {
            gate.open();
        }
        let _ = self.app.shutdown_tasks();
    }
}

fn ocr_result() -> OcrResult {
    OcrResult {
        text: TEXT.into(),
        confidence: 0.875,
        boxes: vec![OcrBox {
            text: TEXT.into(),
            confidence: 0.75,
            points: [(1.0, 2.0), (11.0, 2.0), (11.0, 12.0), (1.0, 12.0)],
        }],
    }
}

fn mrz_result() -> MrzResult {
    MrzResult {
        document_type: "P".into(),
        document_type_sub: "<".into(),
        issuing_country: "UTO".into(),
        document_number: "SYNTHETIC".into(),
        check_digit_document_number: '0',
        nationality: "UTO".into(),
        date_of_birth: "900101".into(),
        check_digit_date_of_birth: '1',
        sex: "X".into(),
        expiry_date: "300101".into(),
        check_digit_expiry: '2',
        optional_data: "RF215".into(),
        composite_check_digit: "3".into(),
        raw_lines: vec!["RF215<SYNTHETIC<MRZ".into()],
        confidence: 0.9375,
        checksum_valid: true,
    }
}

fn key(app: &mut App, code: KeyCode) -> bool {
    app.handle_event(Event::Key(KeyEvent::from(code))).unwrap()
}

fn command(app: &mut App, text: &str) -> bool {
    assert!(app.command_input.is_empty());
    for character in text.chars() {
        assert!(!key(app, KeyCode::Char(character)));
    }
    key(app, KeyCode::Enter)
}

fn dismiss(app: &mut App) {
    for _ in 0..2 {
        if app.info_message.is_none() && app.error_message.is_none() {
            break;
        }
        assert!(!key(app, KeyCode::Esc));
    }
    assert!(app.info_message.is_none() && app.error_message.is_none());
}

fn submit_account_password(app: &mut App, account: &str) {
    auth::unlock(app).unwrap();
    if let AppPhase::UnlockWizard {
        step: UnlockStep::SelectAccount { accounts, .. },
    } = &app.phase
    {
        let index = accounts
            .iter()
            .position(|entry| entry.id == account)
            .unwrap();
        for _ in 0..index {
            assert!(!key(app, KeyCode::Down));
        }
        assert!(!key(app, KeyCode::Enter));
    }
    assert!(
        matches!(&app.phase, AppPhase::UnlockWizard { step: UnlockStep::EnterPassword { account_id, .. } } if account_id == account)
    );
    for character in crate::TEST_PASSWORD.chars() {
        assert!(!key(app, KeyCode::Char(character)));
    }
    assert!(!key(app, KeyCode::Enter));
}

fn unlock_account(app: &mut App, account: &str) {
    submit_account_password(app, account);
    assert!(matches!(&app.phase, AppPhase::Home { account_id } if account_id == account));
    assert!(app.error_message.is_none(), "{:?}", app.error_message);
}

fn unlock_account_while_worker_active(
    app: &mut App,
    account: &str,
    authenticated_key: &Zeroizing<[u8; 32]>,
    original_generation: u64,
) {
    // 先走真实密码键盘向导；维护准入应拒绝，并保留锁定状态和密码页。
    submit_account_password(app, account);
    assert!(app
        .error_message
        .as_deref()
        .unwrap()
        .contains("IMPORT_OPERATIONS_ACTIVE"));
    assert!(
        matches!(&app.phase, AppPhase::UnlockWizard { step: UnlockStep::EnterPassword { account_id, .. } } if account_id == account)
    );
    assert!(app.password_input.value().is_empty());
    assert!(!app.vault_service.is_unlocked());
    assert!(app.vault_service.get_current_account().is_none());
    assert!(app.vault_service.get_session_key().is_none());
    assert!(app.vault_service.get_vault_store().is_none());
    // PIN/biometric 使用的真实会话密钥入口，然后复用 App 同一生产认证收尾。
    app.vault_service
        .unlock_with_session_key(account, authenticated_key)
        .unwrap();
    assert_eq!(
        app.vault_service
            .capture_session(account)
            .unwrap()
            .generation(),
        original_generation.wrapping_add(2)
    );
    app.error_message = None;
    app.enter_home(account);
    assert!(matches!(&app.phase, AppPhase::Home { account_id } if account_id == account));
    assert!(app.error_message.is_none(), "{:?}", app.error_message);
}

fn drain(app: &mut App) {
    crate::util::shared_runtime()
        .unwrap()
        .block_on(async {
            tokio::time::timeout(WAIT, async {
                loop {
                    app.drain_task_events(32).unwrap();
                    if app.tasks.is_empty() {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
        })
        .expect("actual OCR tasks must join and be consumed");
}

fn take_terminal(app: &mut App, id: TaskId) -> TaskEvent {
    crate::util::shared_runtime()
        .unwrap()
        .block_on(async {
            tokio::time::timeout(WAIT, async {
                loop {
                    for event in app.tasks.poll_events(32) {
                        if event.identity.task_id == id
                            && matches!(
                                &event.kind,
                                TaskEventKind::Completed(_)
                                    | TaskEventKind::Failed(_)
                                    | TaskEventKind::Cancelled
                            )
                        {
                            return event;
                        }
                        app.handle_event(Event::Task(event)).unwrap();
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
        })
        .expect("actual OCR terminal must have joined")
}

fn running_event(app: &mut App, id: TaskId) -> TaskEvent {
    let mut running = None;
    for event in app.tasks.poll_events(32) {
        if event.identity.task_id == id
            && matches!(
                event.kind,
                TaskEventKind::BlockingState(BlockingTaskState::Running)
            )
        {
            running = Some(event.clone());
        }
        app.handle_event(Event::Task(event)).unwrap();
    }
    running.expect("running state originates from actual blocking dispatch")
}

fn render(app: &mut App) -> String {
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(170, 38)).unwrap();
    terminal.draw(|frame| app.render(frame)).unwrap();
    terminal.backend().to_string()
}

fn assert_result(app: &App, pending: &Pending, mrz: bool) {
    let AppPhase::OcrResult {
        result,
        source_path,
        tiers,
        mrz: actual_mrz,
    } = &app.phase
    else {
        panic!("expected actual OCR result, got {:?}", app.phase);
    };
    assert!(tiers.is_none());
    if mrz {
        assert_eq!(source_path, &format!("{} (MRZ)", pending.path.display()));
        assert!(result.text.is_empty() && result.boxes.is_empty());
        assert_eq!(result.confidence, mrz_result().confidence);
        assert_eq!(
            serde_json::to_value(actual_mrz.as_ref().unwrap()).unwrap(),
            serde_json::to_value(mrz_result()).unwrap()
        );
    } else {
        assert_eq!(source_path, &pending.path.display().to_string());
        assert!(actual_mrz.is_none());
        assert_eq!(
            serde_json::to_value(result).unwrap(),
            serde_json::to_value(ocr_result()).unwrap()
        );
    }
    assert!(matches!(
        app.last_ocr_result,
        Some(AppPhase::OcrResult { .. })
    ));
    assert!(app.ocr_tasks.is_empty());
}

fn assert_no_ocr(app: &App) {
    assert!(app.ocr_tasks.is_empty());
    assert!(app.last_ocr_result.is_none());
    assert!(!matches!(
        app.phase,
        AppPhase::OcrResult { .. } | AppPhase::OcrTasks
    ));
    assert!(!matches!(
        app.previous_phase,
        Some(AppPhase::OcrResult { .. } | AppPhase::OcrTasks)
    ));
}

#[test]
fn rf215_blocking_image_keeps_key_tick_and_real_task_render_responsive_until_drain() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut fixture = Fixture::new(false);
    let pending = fixture.start("png", false, Pause::Scan, Fault::None);
    let loaded = pending.loaded();
    let entered = pending.entered();
    assert_eq!(loaded.models, fixture.directory.path().join("models"));
    assert_eq!(loaded.tier, OcrModelTier::Tiny);
    assert_ne!(loaded.thread, thread::current().id());
    assert_eq!(loaded.thread, entered.thread);
    assert_eq!(entered.route, "image");
    assert_eq!(entered.path, pending.path);
    assert!(!pending.release.opened.load(Ordering::SeqCst));
    running_event(&mut fixture.app, pending.id);
    assert!(matches!(fixture.app.phase, AppPhase::OcrTasks));
    assert_eq!(
        fixture.app.ocr_tasks[&pending.id].status,
        BlockingTaskState::Running
    );
    assert!(!key(&mut fixture.app, KeyCode::Char('x')));
    assert_eq!(fixture.app.command_input.value, "x");
    assert!(!key(&mut fixture.app, KeyCode::Backspace));
    let activity = fixture.app.last_activity;
    assert!(!fixture.app.handle_event(Event::Tick).unwrap());
    assert_eq!(fixture.app.last_activity, activity);
    let screen = render(&mut fixture.app);
    assert!(
        screen.contains("synthetic-0.png") && screen.contains("Running"),
        "{screen}"
    );
    assert!(!screen.contains(TEXT));
    assert!(fixture.app.last_ocr_result.is_none());
    pending.release.open();
    let terminal = take_terminal(&mut fixture.app, pending.id);
    assert!(matches!(
        terminal.kind,
        TaskEventKind::Completed(TaskOutput::OcrCompleted { .. })
    ));
    assert!(
        fixture.app.last_ocr_result.is_none(),
        "worker return cannot mutate App before event acceptance"
    );
    pending.assert_reclaimed();
    fixture.app.handle_event(Event::Task(terminal)).unwrap();
    assert_result(&fixture.app, &pending, false);
    assert_eq!(fixture.app.last_activity, activity);
    assert!(render(&mut fixture.app).contains(TEXT));
}

#[test]
fn rf215_pdf_and_mrz_routes_preserve_complete_result_dtos() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    for (extension, mrz, route) in [("PDF", false, "pdf"), ("png", true, "mrz")] {
        let mut fixture = Fixture::new(false);
        let pending = fixture.start(extension, mrz, Pause::Scan, Fault::None);
        pending.loaded();
        let entered = pending.entered();
        assert_eq!(entered.route, route);
        assert_eq!(entered.path, pending.path);
        assert_eq!(pending.probe.calls(), vec![route]);
        pending.release.open();
        drain(&mut fixture.app);
        assert_result(&fixture.app, &pending, mrz);
        pending.assert_reclaimed();
    }
}

#[test]
fn rf215_escape_requests_cancellation_but_retains_native_resources_until_real_exit() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut fixture = Fixture::new(false);
    let running = fixture.start("pdf", false, Pause::Scan, Fault::None);
    running.loaded();
    let entered = running.entered();
    let queued = fixture.start("png", false, Pause::Scan, Fault::None);
    fixture.app.drain_task_events(32).unwrap();
    assert_eq!(
        fixture.app.ocr_tasks[&queued.id].status,
        BlockingTaskState::Queued
    );
    assert!(!key(&mut fixture.app, KeyCode::Esc));
    assert!(entered.cancel.is_cancelled());
    assert!(matches!(fixture.app.phase, AppPhase::OcrTasks));
    assert!(fixture.app.ocr_tasks.contains_key(&running.id));
    assert_eq!(running.probe.drops.load(Ordering::SeqCst), 0);
    assert!(running
        .probe
        .resource()
        .join("owned-native-resource")
        .is_file());
    assert_eq!(queued.probe.loads.load(Ordering::SeqCst), 0);
    fixture.app.drain_task_events(32).unwrap();
    assert_eq!(
        fixture.app.ocr_tasks[&running.id].status,
        BlockingTaskState::CancelRequested
    );
    assert!(render(&mut fixture.app).contains("Cancel"));
    running.release.open();
    drain(&mut fixture.app);
    assert!(fixture.app.ocr_tasks.is_empty() && fixture.app.last_ocr_result.is_none());
    assert!(
        fixture.app.error_message.is_none(),
        "cancel must not be a scan failure"
    );
    running.assert_reclaimed();
    assert_eq!(queued.probe.loads.load(Ordering::SeqCst), 0);
    assert!(!render(&mut fixture.app).contains(TEXT));
}

#[test]
fn rf215_real_cancel_command_targets_full_id_and_cancel_all_preserves_running_owner() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut fixture = Fixture::new(false);
    let running = fixture.start("png", false, Pause::Scan, Fault::None);
    running.loaded();
    let entered = running.entered();
    let queued = fixture.start("png", false, Pause::Scan, Fault::None);
    assert!(!command(&mut fixture.app, "/ocr cancel not-a-uuid"));
    assert!(fixture.app.error_message.is_some());
    assert!(!entered.cancel.is_cancelled());
    dismiss(&mut fixture.app);
    assert!(!command(
        &mut fixture.app,
        &format!("/ocr cancel {}", queued.id.0)
    ));
    fixture.app.drain_task_events(32).unwrap();
    assert!(!fixture.app.ocr_tasks.contains_key(&queued.id));
    assert!(fixture.app.ocr_tasks.contains_key(&running.id));
    assert_eq!(queued.probe.loads.load(Ordering::SeqCst), 0);
    assert!(!entered.cancel.is_cancelled());
    dismiss(&mut fixture.app);
    assert!(!command(&mut fixture.app, "/ocr cancel"));
    assert!(entered.cancel.is_cancelled());
    assert!(running.probe.resource().exists());
    assert_eq!(running.probe.drops.load(Ordering::SeqCst), 0);
    running.release.open();
    drain(&mut fixture.app);
    running.assert_reclaimed();
    assert!(fixture.app.last_ocr_result.is_none());
}

#[test]
fn rf215_cancellation_during_model_load_never_calls_scan_after_loader_returns() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut fixture = Fixture::new(false);
    let pending = fixture.start("png", false, Pause::Load, Fault::None);
    pending.loaded();
    assert!(!command(&mut fixture.app, "/ocr cancel"));
    assert_eq!(pending.probe.drops.load(Ordering::SeqCst), 0);
    assert!(pending.probe.resource().exists());
    pending.release.open();
    drain(&mut fixture.app);
    assert!(pending.probe.calls().is_empty());
    pending.assert_reclaimed();
    assert!(fixture.app.last_ocr_result.is_none());
    assert!(fixture.app.error_message.is_none());
}

#[derive(Clone, Copy, Debug)]
enum Transition {
    Lock,
    AutoLock,
    SameAccount,
    OtherAccount,
    DirectLock,
}

fn transition(fixture: &mut Fixture, change: Transition, worker_active: bool) {
    let original = fixture
        .app
        .vault_service
        .capture_session(&fixture.account_a)
        .unwrap();
    match change {
        Transition::AutoLock => {
            let deadline = fixture.app.last_activity + fixture.app.auto_lock_duration;
            assert!(!fixture.app.handle_event_at(Event::Tick, deadline).unwrap());
            assert!(matches!(fixture.app.phase, AppPhase::Locked));
        }
        Transition::DirectLock => fixture.app.vault_service.lock(),
        _ => auth::lock(&mut fixture.app),
    }
    if matches!(change, Transition::SameAccount | Transition::OtherAccount) {
        let account = if matches!(change, Transition::SameAccount) {
            fixture.account_a.clone()
        } else {
            fixture.account_b.clone().unwrap()
        };
        if worker_active {
            let key = if matches!(change, Transition::SameAccount) {
                &fixture.account_a_key
            } else {
                fixture.account_b_key.as_ref().unwrap()
            };
            unlock_account_while_worker_active(
                &mut fixture.app,
                &account,
                key,
                original.generation(),
            );
        } else {
            // 已真实 join 后仍验证生产主密码向导，不把 ready-result 夹具改成旁路。
            unlock_account(&mut fixture.app, &account);
        }
    }
    assert!(fixture
        .app
        .vault_service
        .with_session(&original, |_| Ok(()))
        .is_err());
    fixture.app.drain_task_events(32).unwrap();
    assert_no_ocr(&fixture.app);
}

#[test]
fn rf215_auth_invalidation_cancels_running_and_queued_scans_without_revealing_old_text() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    for change in [
        Transition::Lock,
        Transition::AutoLock,
        Transition::SameAccount,
        Transition::OtherAccount,
        Transition::DirectLock,
    ] {
        let mut fixture = Fixture::new(matches!(change, Transition::OtherAccount));
        let running = fixture.start("pdf", false, Pause::Scan, Fault::None);
        running.loaded();
        let entered = running.entered();
        let queued = fixture.start("png", false, Pause::Scan, Fault::None);
        transition(&mut fixture, change, true);
        assert!(entered.cancel.is_cancelled(), "{change:?}");
        assert!(running.probe.resource().exists());
        let messages = (
            fixture.app.info_message.clone(),
            fixture.app.error_message.clone(),
        );
        running.release.open();
        queued.release.open();
        drain(&mut fixture.app);
        assert_no_ocr(&fixture.app);
        assert_eq!(
            (
                fixture.app.info_message.clone(),
                fixture.app.error_message.clone()
            ),
            messages
        );
        assert_eq!(queued.probe.loads.load(Ordering::SeqCst), 0);
        running.assert_reclaimed();
        assert!(!render(&mut fixture.app).contains(TEXT));
    }
}

#[test]
fn rf215_joined_result_waiting_for_app_is_rejected_after_lock_reunlock_or_switch() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    for change in [
        Transition::Lock,
        Transition::SameAccount,
        Transition::OtherAccount,
    ] {
        let mut fixture = Fixture::new(matches!(change, Transition::OtherAccount));
        let pending = fixture.start("png", false, Pause::Scan, Fault::None);
        pending.loaded();
        pending.entered();
        pending.release.open();
        let terminal = take_terminal(&mut fixture.app, pending.id);
        assert!(matches!(
            terminal.kind,
            TaskEventKind::Completed(TaskOutput::OcrCompleted { .. })
        ));
        assert!(fixture.app.last_ocr_result.is_none());
        pending.assert_reclaimed();
        transition(&mut fixture, change, false);
        let messages = (
            fixture.app.info_message.clone(),
            fixture.app.error_message.clone(),
        );
        fixture
            .app
            .handle_event(Event::Task(terminal.clone()))
            .unwrap();
        fixture.app.handle_event(Event::Task(terminal)).unwrap();
        assert_no_ocr(&fixture.app);
        assert_eq!(
            (
                fixture.app.info_message.clone(),
                fixture.app.error_message.clone()
            ),
            messages
        );
        assert!(fixture.app.tasks.is_empty());
    }
}

#[test]
fn rf215_duplicate_old_state_and_terminal_never_remove_a_new_scan_task() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut fixture = Fixture::new(false);
    let old = fixture.start("png", false, Pause::Scan, Fault::None);
    old.loaded();
    old.entered();
    let state = running_event(&mut fixture.app, old.id);
    old.release.open();
    let terminal = take_terminal(&mut fixture.app, old.id);
    fixture
        .app
        .handle_event(Event::Task(terminal.clone()))
        .unwrap();
    let new = fixture.start("png", false, Pause::Scan, Fault::None);
    new.loaded();
    let entered = new.entered();
    running_event(&mut fixture.app, new.id);
    fixture.app.handle_event(Event::Task(state)).unwrap();
    fixture.app.handle_event(Event::Task(terminal)).unwrap();
    assert_ne!(old.id, new.id);
    assert!(matches!(fixture.app.phase, AppPhase::OcrTasks));
    assert_eq!(fixture.app.ocr_tasks.len(), 1);
    assert_eq!(
        fixture.app.ocr_tasks[&new.id].status,
        BlockingTaskState::Running
    );
    assert!(!entered.cancel.is_cancelled());
    new.release.open();
    drain(&mut fixture.app);
    assert_result(&fixture.app, &new, false);
    new.assert_reclaimed();
}

#[test]
fn rf215_load_scan_panic_and_mrz_none_fail_without_result_and_allow_next_scan() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    for (fault, mrz, expected) in [
        (Fault::Load, false, "model load failure"),
        (Fault::Scan, false, "scan failure"),
        (Fault::Panic, false, "后台任务执行失败"),
        (Fault::MrzNone, true, "MRZ"),
    ] {
        let mut fixture = Fixture::new(false);
        let pending = fixture.start("png", mrz, Pause::None, fault);
        drain(&mut fixture.app);
        let error = fixture
            .app
            .error_message
            .as_deref()
            .expect("worker failure must reach App error");
        assert!(error.contains(expected), "{error}");
        assert!(
            !error.contains("RF215 synthetic engine panic"),
            "panic payload must not reach the user"
        );
        assert!(fixture.app.last_ocr_result.is_none() && fixture.app.ocr_tasks.is_empty());
        assert!(!matches!(fixture.app.phase, AppPhase::OcrResult { .. }));
        if !matches!(fault, Fault::Load) {
            pending.assert_reclaimed();
        }
        dismiss(&mut fixture.app);
        let retry = fixture.start("png", false, Pause::None, Fault::None);
        drain(&mut fixture.app);
        assert_result(&fixture.app, &retry, false);
        retry.assert_reclaimed();
    }
}

#[test]
fn rf215_completion_preserves_help_and_result_command_reopens_latest_result() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut fixture = Fixture::new(false);
    let pending = fixture.start("png", false, Pause::Scan, Fault::None);
    pending.loaded();
    pending.entered();
    assert!(!command(&mut fixture.app, "/help ocr"));
    assert!(matches!(fixture.app.phase, AppPhase::Help { .. }));
    pending.release.open();
    drain(&mut fixture.app);
    assert!(matches!(fixture.app.phase, AppPhase::Help { .. }));
    assert!(matches!(
        fixture.app.last_ocr_result,
        Some(AppPhase::OcrResult { .. })
    ));
    assert!(!render(&mut fixture.app).contains(TEXT));
    assert!(!command(&mut fixture.app, "/ocr result"));
    assert_result(&fixture.app, &pending, false);
    assert!(render(&mut fixture.app).contains(TEXT));
    assert!(!command(&mut fixture.app, "/ocr jobs"));
    assert!(matches!(fixture.app.phase, AppPhase::OcrTasks));
    assert!(!render(&mut fixture.app).contains(TEXT));
    pending.assert_reclaimed();
}

#[test]
fn rf215_idle_result_current_previous_and_cache_clear_on_direct_lock_or_new_generation() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    for relogin in [false, true] {
        for cached in [false, true] {
            let mut fixture = Fixture::new(false);
            let pending = fixture.start("png", false, Pause::None, Fault::None);
            drain(&mut fixture.app);
            assert_result(&fixture.app, &pending, false);
            assert!(fixture.app.tasks.is_empty());
            if cached {
                assert!(!command(&mut fixture.app, "/help ocr"));
            }
            fixture.app.vault_service.lock();
            if relogin {
                fixture
                    .app
                    .vault_service
                    .unlock(&fixture.account_a, crate::TEST_PASSWORD)
                    .unwrap();
            }
            // 没有活动 Tasks 可提供 stale id，OCR 自己的会话票据仍须失效。
            fixture.app.drain_task_events(32).unwrap();
            assert_no_ocr(&fixture.app);
            assert!(!render(&mut fixture.app).contains(TEXT));
            if relogin {
                assert!(!command(&mut fixture.app, "/ocr result"));
                assert!(fixture
                    .app
                    .info_message
                    .as_deref()
                    .is_some_and(|message| message.contains("No OCR result")));
                assert_no_ocr(&fixture.app);
            }
        }
    }
}

#[test]
fn rf215_real_exit_then_shutdown_waits_for_blocking_engine_resources_to_drop() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut fixture = Fixture::new(false);
    let pending = fixture.start("pdf", false, Pause::Scan, Fault::None);
    pending.loaded();
    let entered = pending.entered();
    let resource = pending.probe.resource();
    let drops = Arc::clone(&pending.probe.drops);
    let release = pending.release.clone();
    thread::scope(|scope| {
        // 模拟原生函数只能自行退出：观察协作取消后才放行；超时仅防测试死锁。
        let native_exit = scope.spawn(move || {
            let deadline = Instant::now() + WAIT;
            while !entered.cancel.is_cancelled() && Instant::now() < deadline {
                thread::yield_now();
            }
            let observed = (
                entered.cancel.is_cancelled(),
                resource.exists(),
                drops.load(Ordering::SeqCst),
            );
            release.open();
            observed
        });
        assert!(command(&mut fixture.app, "/exit"));
        assert!(matches!(fixture.app.phase, AppPhase::Quit));
        fixture.app.shutdown_tasks().unwrap();
        assert_eq!(native_exit.join().unwrap(), (true, true, 0));
    });
    assert!(fixture.app.tasks.is_empty() && fixture.app.ocr_tasks.is_empty());
    assert!(fixture.app.last_ocr_result.is_none());
    pending.assert_reclaimed();
}
