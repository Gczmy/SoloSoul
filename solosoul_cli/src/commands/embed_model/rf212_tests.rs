//! RF212 基线：实际 HTTP 首块屏障与真实模型目录扫描。

use std::ffi::OsString;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent};
use sha2::{Digest, Sha256};
use solosoul_core::VaultService;

use super::{handle, scan_local_models};
use crate::app::{App, AppPhase, UnlockStep};
use crate::commands::auth;
use crate::events::Event;
use crate::tasks::{TaskEvent, TaskEventKind, TaskId};

const MODEL_ID: &str = "rf212-synthetic";
const MODEL_BYTES: &[u8] = b"RF212 first block | synthetic remaining model bytes";
const FIRST_BLOCK: usize = 18;
const IO_GUARD: Duration = Duration::from_secs(15);
const DEADLOCK_GUARD: Duration = Duration::from_secs(5);

struct RegistryEnvironment(Vec<(&'static str, Option<OsString>)>);

impl RegistryEnvironment {
    fn set(url: &str) -> Self {
        let mut saved = Vec::new();
        // loopback 不能因宿主机代理配置而走外部代理；所有值在两个线程回收后恢复。
        for (name, value) in [
            ("SOLOSOUL_EMBED_REGISTRY", url),
            ("NO_PROXY", "127.0.0.1,localhost"),
            ("no_proxy", "127.0.0.1,localhost"),
        ] {
            saved.push((name, std::env::var_os(name)));
            std::env::set_var(name, value);
        }
        Self(saved)
    }
}

impl Drop for RegistryEnvironment {
    fn drop(&mut self) {
        for (name, previous) in self.0.drain(..).rev() {
            if let Some(previous) = previous {
                std::env::set_var(name, previous);
            } else {
                std::env::remove_var(name);
            }
        }
    }
}

/// 两个线程的退出 guard 都能放行；顺序标志是断言依据，超时不是性能标准。
#[derive(Clone)]
struct ReleaseGate {
    opened: Arc<AtomicBool>,
    sender: Arc<Mutex<Option<mpsc::Sender<()>>>>,
}

impl ReleaseGate {
    fn open(&self) {
        self.opened.store(true, Ordering::SeqCst);
        let sender = self
            .sender
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        if let Some(sender) = sender {
            let _ = sender.send(());
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum ResponseMode {
    Complete,
    Truncated,
    WrongHash,
}

struct LoopbackDownload {
    address: SocketAddr,
    first_block: mpsc::Receiver<()>,
    release: ReleaseGate,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<Result<(), String>>>,
}

impl LoopbackDownload {
    fn start() -> Self {
        Self::with_mode(ResponseMode::Complete)
    }

    fn with_mode(mode: ResponseMode) -> Self {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .expect("bind synthetic loopback server");
        let address = listener.local_addr().unwrap();
        let (release, released) = mpsc::channel();
        let (first_block, observed) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let thread = thread::spawn(move || {
            let registry = serde_json::to_vec(&serde_json::json!({
                "models": [{
                    "id": MODEL_ID,
                    "name": "RF212 synthetic model",
                    "size_mb": 0.001,
                    "sha256": if matches!(mode, ResponseMode::WrongHash) {
                        "0".repeat(64)
                    } else {
                        format!("{:x}", Sha256::digest(MODEL_BYTES))
                    },
                    "download_url": format!("http://{address}/model.bin")
                }]
            }))
            .map_err(|error| error.to_string())?;
            for expected in ["/registry.json", "/model.bin"] {
                let (mut stream, _) = listener.accept().map_err(|error| error.to_string())?;
                if worker_stop.load(Ordering::SeqCst) {
                    return Ok(());
                }
                stream
                    .set_read_timeout(Some(IO_GUARD))
                    .map_err(|error| error.to_string())?;
                stream
                    .set_write_timeout(Some(IO_GUARD))
                    .map_err(|error| error.to_string())?;
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    if request.len() >= 16 * 1024 {
                        return Err("synthetic HTTP header exceeds test limit".to_string());
                    }
                    let mut byte = [0];
                    stream
                        .read_exact(&mut byte)
                        .map_err(|error| error.to_string())?;
                    request.push(byte[0]);
                }
                if !request.starts_with(format!("GET {expected} HTTP/1.1\r\n").as_bytes()) {
                    return Err(format!("unexpected synthetic request for {expected}"));
                }
                let body = if expected == "/registry.json" {
                    registry.as_slice()
                } else {
                    MODEL_BYTES
                };
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .map_err(|error| error.to_string())?;
                if expected == "/model.bin" {
                    stream
                        .write_all(&body[..FIRST_BLOCK])
                        .map_err(|error| error.to_string())?;
                    stream.flush().map_err(|error| error.to_string())?;
                    first_block.send(()).map_err(|error| error.to_string())?;
                    released
                        .recv_timeout(IO_GUARD)
                        .map_err(|error| error.to_string())?;
                    if worker_stop.load(Ordering::SeqCst) || matches!(mode, ResponseMode::Truncated)
                    {
                        // 取消场景显式关流；截断场景保留完整 Content-Length，真实客户端必须报错。
                        return Ok(());
                    }
                    stream
                        .write_all(&body[FIRST_BLOCK..])
                        .map_err(|error| error.to_string())?;
                } else {
                    stream.write_all(body).map_err(|error| error.to_string())?;
                }
                stream.flush().map_err(|error| error.to_string())?;
            }
            Ok(())
        });
        Self {
            address,
            first_block: observed,
            release: ReleaseGate {
                opened: Arc::new(AtomicBool::new(false)),
                sender: Arc::new(Mutex::new(Some(release))),
            },
            stop,
            thread: Some(thread),
        }
    }

    fn registry_url(&self) -> String {
        format!("http://{}/registry.json", self.address)
    }

    fn close_response(&mut self) -> Result<(), String> {
        self.stop.store(true, Ordering::SeqCst);
        self.release.open();
        let _ = TcpStream::connect_timeout(&self.address, Duration::from_secs(1));
        if self.thread.is_some() {
            self.join()
        } else {
            Ok(())
        }
    }

    fn join(&mut self) -> Result<(), String> {
        self.thread
            .take()
            .expect("server join is single use")
            .join()
            .map_err(|_| "synthetic server panicked".to_string())?
    }
}

impl Drop for LoopbackDownload {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.release.open();
        // 唤醒尚在 accept 的测试线程；没有请求时也可回收。
        let _ = TcpStream::connect_timeout(&self.address, Duration::from_secs(1));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct LocalApp(App);

impl Drop for LocalApp {
    fn drop(&mut self) {
        let _ = self.0.shutdown_tasks();
    }
}

#[derive(Debug)]
struct InstallObservation {
    returned_before_release: bool,
    key_before_release: bool,
    input: String,
    error: Option<String>,
    handle_result: Result<(), String>,
}

struct InstallCaller {
    release: ReleaseGate,
    finish: Option<mpsc::Sender<()>>,
    observation: mpsc::Receiver<InstallObservation>,
    thread: Option<JoinHandle<Result<(), String>>>,
}

impl InstallCaller {
    fn start(directory: PathBuf, release: ReleaseGate) -> Self {
        let (observed, observation) = mpsc::channel();
        let (finish, finished) = mpsc::channel();
        let worker_release = release.clone();
        // App / FluentBundle 不跨线程传递，只传入临时目录和同步信号。
        let thread = thread::spawn(move || {
            let service = VaultService::with_base_path(directory);
            let account = service.create_account("RF212 synthetic", crate::TEST_PASSWORD, None)?;
            let account_id = account["id"].as_str().unwrap().to_string();
            let mut app = LocalApp(App::new(Arc::new(service)).map_err(|error| error.to_string())?);
            app.0.phase = AppPhase::Home { account_id };
            let handle_result =
                handle(&mut app.0, &["install", MODEL_ID]).map_err(|error| error.to_string());
            let returned_before_release = !worker_release.opened.load(Ordering::SeqCst);
            let error = app.0.error_message.clone();
            let exited = app
                .0
                .handle_event(Event::Key(KeyEvent::from(KeyCode::Char('x'))))
                .map_err(|error| error.to_string())?;
            if exited {
                return Err("typing must not exit the CLI".to_string());
            }
            observed
                .send(InstallObservation {
                    returned_before_release,
                    key_before_release: !worker_release.opened.load(Ordering::SeqCst),
                    input: app.0.command_input.value.clone(),
                    error,
                    handle_result,
                })
                .map_err(|error| error.to_string())?;
            // 保持 App / 任务存活，直到主线程完成首块与调用顺序观察。
            finished
                .recv_timeout(IO_GUARD)
                .map_err(|error| error.to_string())?;
            app.0.shutdown_tasks().map_err(|error| error.to_string())?;
            Ok(())
        });
        Self {
            release,
            finish: Some(finish),
            observation,
            thread: Some(thread),
        }
    }

    fn finish_and_join(&mut self) -> Result<(), String> {
        if let Some(finish) = self.finish.take() {
            let _ = finish.send(());
        }
        self.thread
            .take()
            .expect("caller join is single use")
            .join()
            .map_err(|_| "synthetic App caller panicked".to_string())?
    }
}

impl Drop for InstallCaller {
    fn drop(&mut self) {
        self.release.open();
        if let Some(finish) = self.finish.take() {
            let _ = finish.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[test]
fn rf212_install_returns_and_handles_keys_before_model_stream_is_released() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let directory = tempfile::TempDir::new().unwrap();
    let mut server = LoopbackDownload::start();
    let _environment = RegistryEnvironment::set(&server.registry_url());
    let mut caller = InstallCaller::start(directory.path().to_path_buf(), server.release.clone());
    server
        .first_block
        .recv_timeout(IO_GUARD)
        .expect("real HTTP model response reaches first block");

    // 旧实现的 block_on 与首块屏障形成因果死锁。超时只负责解除死锁，
    // 最终检查在放行前还是放行后调用返回/按键处理，而非墙钟耗时。
    let early_observation = caller.observation.recv_timeout(DEADLOCK_GUARD);
    server.release.open();
    let server_result = server.join();
    let caller_result = caller.finish_and_join();
    server_result.expect("synthetic HTTP transfer must complete without errors");
    caller_result.expect("synthetic App caller must be joined successfully");
    let observation = match early_observation {
        Ok(observation) => observation,
        Err(mpsc::RecvTimeoutError::Timeout) => caller
            .observation
            .recv_timeout(IO_GUARD)
            .expect("old blocking caller returns after explicit release"),
        Err(error) => panic!("App observation channel failed: {error}"),
    };
    assert!(observation.handle_result.is_ok(), "{observation:?}");
    assert!(observation.error.is_none(), "{observation:?}");
    assert!(
        observation.returned_before_release,
        "handle install waited for the model body release: {observation:?}"
    );
    assert!(
        observation.key_before_release,
        "keyboard handling waited for download: {observation:?}"
    );
    assert_eq!(observation.input, "x");
}

#[test]
fn rf212_scan_excludes_incomplete_directories_but_keeps_complete_model() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let directory = tempfile::TempDir::new().unwrap();
    let models = directory.path().join("embed_models");
    for id in ["empty-model", "only-part-model", "complete-model"] {
        std::fs::create_dir_all(models.join(id)).unwrap();
    }
    std::fs::write(models.join("only-part-model/model.bin.part"), b"unfinished").unwrap();
    std::fs::write(models.join("complete-model/model.bin"), MODEL_BYTES).unwrap();

    let mut entries = scan_local_models(&models);
    entries.sort_by(|left, right| left.id.cmp(&right.id));
    let ids: Vec<_> = entries.iter().map(|entry| entry.id.as_str()).collect();
    assert_eq!(
        ids,
        ["complete-model"],
        "incomplete directories must not appear as installed"
    );
    assert!(entries[0].installed);
    assert_eq!(
        std::fs::read(models.join("complete-model/model.bin")).unwrap(),
        MODEL_BYTES
    );
}

/// 每次只安装一个小型合成 HTTP 响应；先回收服务器，再恢复环境变量。
struct HttpAttempt {
    server: LoopbackDownload,
    _environment: RegistryEnvironment,
}

impl HttpAttempt {
    fn new(mode: ResponseMode) -> Self {
        let server = LoopbackDownload::with_mode(mode);
        let environment = RegistryEnvironment::set(&server.registry_url());
        Self {
            server,
            _environment: environment,
        }
    }
}

impl Drop for HttpAttempt {
    fn drop(&mut self) {
        let _ = self.server.close_response();
    }
}

struct DownloadFixture {
    app: App,
    attempt: Option<HttpAttempt>,
    account_a: String,
    account_b: Option<String>,
    directory: tempfile::TempDir,
}

impl DownloadFixture {
    fn new(with_second_account: bool) -> Self {
        let directory = tempfile::TempDir::new().unwrap();
        let service = VaultService::with_base_path(directory.path().to_path_buf());
        let account = service
            .create_account("RF212 download A", crate::TEST_PASSWORD, None)
            .unwrap();
        let account_a = account["id"].as_str().unwrap().to_string();
        let account_b = with_second_account.then(|| {
            service
                .create_account("RF212 download B", crate::TEST_PASSWORD, None)
                .unwrap()["id"]
                .as_str()
                .unwrap()
                .to_string()
        });
        service.lock();
        let app = App::new(Arc::new(service)).unwrap();
        let mut fixture = Self {
            app,
            attempt: None,
            account_a,
            account_b,
            directory,
        };
        unlock_download_account(&mut fixture.app, &fixture.account_a);
        fixture.app.i18n.set_locale("en-US");
        fixture
    }

    fn models_root(&self) -> PathBuf {
        self.directory.path().join("embed_models")
    }

    fn target(&self) -> PathBuf {
        self.models_root().join(MODEL_ID).join("model.bin")
    }

    /// 只有真实客户端字节进度已由 App 接纳才返回，服务端仍停在首块之后。
    fn start(&mut self, mode: ResponseMode) -> (TaskId, TaskEvent) {
        assert!(
            self.app.tasks.is_empty(),
            "previous attempt must be joined first"
        );
        self.attempt.take();
        self.attempt = Some(HttpAttempt::new(mode));
        assert!(!command(
            &mut self.app,
            &format!("/embed_model install {MODEL_ID}")
        ));
        assert!(
            self.app.error_message.is_none(),
            "{:?}",
            self.app.error_message
        );
        let id = self
            .app
            .embed_downloads
            .get(MODEL_ID)
            .expect("download is registered")
            .task_id;
        let activity = self.app.last_activity;
        self.attempt
            .as_ref()
            .unwrap()
            .server
            .first_block
            .recv_timeout(IO_GUARD)
            .expect("real model stream reaches first-block barrier");
        let progress = wait_first_progress(&mut self.app, id);
        assert_eq!(
            self.app.last_activity, activity,
            "real download progress is not user activity"
        );
        assert_eq!(
            self.app.task_progress.get(&id),
            Some(&(FIRST_BLOCK as u64, Some(MODEL_BYTES.len() as u64)))
        );
        assert!(
            !self.target().exists(),
            "first block is never an installed model"
        );
        (id, progress)
    }

    fn finish_response(&mut self) {
        let server = &mut self.attempt.as_mut().unwrap().server;
        server.release.open();
        server.join().expect("synthetic response completes");
        drain_until(&mut self.app, |app| app.tasks.is_empty());
    }

    fn close_cancelled_response(&mut self) {
        self.attempt
            .as_mut()
            .unwrap()
            .server
            .close_response()
            .expect("cancelled synthetic response closes and joins");
    }

    fn assert_published(&self) {
        assert_eq!(std::fs::read(self.target()).unwrap(), MODEL_BYTES);
        let files = model_files(&self.models_root());
        assert_eq!(
            files,
            vec![(
                PathBuf::from(MODEL_ID).join("model.bin"),
                MODEL_BYTES.to_vec()
            )],
            "success must not retain staging files"
        );
        assert!(self.app.embed_downloads.is_empty());
        assert!(self.app.task_progress.is_empty());
        assert!(self.app.tasks.is_empty());
        assert!(
            self.app.error_message.is_none(),
            "{:?}",
            self.app.error_message
        );
        assert!(self
            .app
            .success_message
            .as_ref()
            .is_some_and(|(text, _)| text.contains(MODEL_ID)));
    }
}

impl Drop for DownloadFixture {
    fn drop(&mut self) {
        // 任务 Future 真实析构完成后才回收 HTTP 线程、恢复环境和删除临时 Vault。
        let _ = self.app.shutdown_tasks();
        self.attempt.take();
    }
}

fn command(app: &mut App, text: &str) -> bool {
    app.command_input.set_value(text.to_string());
    app.handle_event(Event::Key(KeyEvent::from(KeyCode::Enter)))
        .unwrap()
}

fn unlock_download_account(app: &mut App, account_id: &str) {
    auth::unlock(app).expect("start real unlock wizard");
    if let AppPhase::UnlockWizard {
        step: UnlockStep::SelectAccount { accounts, .. },
    } = &app.phase
    {
        let index = accounts
            .iter()
            .position(|account| account.id == account_id)
            .unwrap();
        for _ in 0..index {
            assert!(!app
                .handle_event(Event::Key(KeyEvent::from(KeyCode::Down)))
                .unwrap());
        }
        assert!(!app
            .handle_event(Event::Key(KeyEvent::from(KeyCode::Enter)))
            .unwrap());
    }
    assert!(matches!(&app.phase, AppPhase::UnlockWizard {
        step: UnlockStep::EnterPassword { account_id: selected, .. }
    } if selected == account_id));
    for character in crate::TEST_PASSWORD.chars() {
        assert!(!app
            .handle_event(Event::Key(KeyEvent::from(KeyCode::Char(character))))
            .unwrap());
    }
    assert!(!app
        .handle_event(Event::Key(KeyEvent::from(KeyCode::Enter)))
        .unwrap());
    assert!(matches!(&app.phase, AppPhase::Home { account_id: current } if current == account_id));
    assert!(app.error_message.is_none(), "{:?}", app.error_message);
}

fn drain_until(app: &mut App, ready: impl Fn(&App) -> bool) {
    crate::util::shared_runtime()
        .unwrap()
        .block_on(async {
            tokio::time::timeout(IO_GUARD, async {
                loop {
                    app.drain_task_events(32).unwrap();
                    if ready(app) {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
        })
        .expect("actual task drain must reach the requested state");
}

/// 不伪造进度或终态；事件均从真实 Tasks poll 取得，然后交给真实 App。
fn wait_first_progress(app: &mut App, id: TaskId) -> TaskEvent {
    crate::util::shared_runtime()
        .unwrap()
        .block_on(async {
            tokio::time::timeout(IO_GUARD, async {
            loop {
                let mut first = None;
                for event in app.tasks.poll_events(32) {
                    if event.identity.task_id == id && matches!(event.kind,
                        TaskEventKind::Progress { current, total: Some(total) }
                        if current == FIRST_BLOCK as u64 && total == MODEL_BYTES.len() as u64
                    ) {
                        first = Some(event.clone());
                    }
                    app.handle_event(Event::Task(event)).unwrap();
                }
                if let Some(event) = first { return event; }
                tokio::task::yield_now().await;
            }
        }).await
        })
        .expect("real first-block byte progress must arrive")
}

fn take_terminal(app: &mut App, id: TaskId) -> TaskEvent {
    crate::util::shared_runtime()
        .unwrap()
        .block_on(async {
            tokio::time::timeout(IO_GUARD, async {
                loop {
                    for event in app.tasks.poll_events(32) {
                        if event.identity.task_id == id
                            && !matches!(event.kind, TaskEventKind::Progress { .. })
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
        .expect("actual worker must join and produce a terminal event")
}

fn model_files(root: &std::path::Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn visit(root: &std::path::Path, path: &std::path::Path, files: &mut Vec<(PathBuf, Vec<u8>)>) {
        if !path.exists() {
            return;
        }
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                visit(root, &entry.path(), files);
            } else {
                files.push((
                    entry.path().strip_prefix(root).unwrap().to_path_buf(),
                    std::fs::read(entry.path()).unwrap(),
                ));
            }
        }
    }
    let mut files = Vec::new();
    visit(root, root, &mut files);
    files.sort_by(|left, right| left.0.cmp(&right.0));
    files
}

fn rendered(app: &mut App) -> String {
    let backend = ratatui::backend::TestBackend::new(160, 36);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.render(frame)).unwrap();
    terminal.backend().to_string()
}

#[test]
fn rf212_real_http_progress_renders_then_success_refreshes_installed_list() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut fixture = DownloadFixture::new(false);
    let (id, _) = fixture.start(ResponseMode::Complete);
    assert!(
        matches!(&fixture.app.phase, AppPhase::EmbedModelList { models, .. } if models.is_empty())
    );
    let activity = fixture.app.last_activity;
    let screen = rendered(&mut fixture.app);
    assert!(screen.contains(MODEL_ID), "{screen}");
    assert!(
        screen.contains(&format!("{FIRST_BLOCK} / {} B", MODEL_BYTES.len())),
        "real byte counts must render: {screen}"
    );
    assert!(
        screen.contains("Downloading"),
        "active download state must render: {screen}"
    );
    assert!(
        screen.contains("/embed_model cancel"),
        "download cancellation hint must render: {screen}"
    );
    assert_eq!(
        fixture.app.last_activity, activity,
        "progress/render must not count as keyboard activity"
    );
    assert_eq!(
        model_files(&fixture.models_root()).len(),
        1,
        "only this task's staging file exists"
    );

    // 首块仍挂起时重复安装不得创建第二个同模型任务。
    assert!(!command(
        &mut fixture.app,
        &format!("/embed_model install {MODEL_ID}")
    ));
    assert_eq!(
        fixture.app.embed_downloads.get(MODEL_ID).unwrap().task_id,
        id
    );
    // 真实 Esc 关闭重复安装的信息框，让完成后的断言读取实际列表行。
    assert!(!fixture
        .app
        .handle_event(Event::Key(KeyEvent::from(KeyCode::Esc)))
        .unwrap());
    fixture.finish_response();
    fixture.assert_published();
    assert!(
        matches!(&fixture.app.phase, AppPhase::EmbedModelList { models, .. }
        if models.len() == 1 && models[0].id == MODEL_ID && models[0].installed)
    );
    let screen = rendered(&mut fixture.app);
    assert!(
        screen.contains(MODEL_ID),
        "installed model remains visible: {screen}"
    );
}

#[test]
fn rf212_success_does_not_navigate_away_from_the_page_selected_during_download() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut fixture = DownloadFixture::new(false);
    fixture.start(ResponseMode::Complete);
    assert!(!command(&mut fixture.app, "/help embed_model"));
    assert!(
        matches!(&fixture.app.phase, AppPhase::Help { topic: Some(topic), .. } if topic == "embed_model")
    );
    assert!(
        matches!(&fixture.app.previous_phase, Some(AppPhase::EmbedModelList { models, .. }) if models.is_empty())
    );
    fixture.finish_response();
    fixture.assert_published();
    assert!(
        matches!(&fixture.app.phase, AppPhase::Help { topic: Some(topic), .. } if topic == "embed_model"),
        "completion must preserve the page selected during download"
    );
    // /back 真正恢复缓存页面；不能靠重新 /embed_model list 扫描来掩盖旧列表。
    assert!(!command(&mut fixture.app, "/back"));
    assert!(
        matches!(&fixture.app.phase, AppPhase::EmbedModelList { models, .. }
        if models.len() == 1 && models[0].id == MODEL_ID && models[0].installed)
    );
    let screen = rendered(&mut fixture.app);
    assert!(
        screen.contains(MODEL_ID),
        "returning from help must show the installed row: {screen}"
    );
}

#[test]
fn rf212_cancel_and_real_exit_reclaim_http_task_and_staging_files() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut fixture = DownloadFixture::new(false);
    let (id, _) = fixture.start(ResponseMode::Complete);
    assert!(!command(
        &mut fixture.app,
        &format!("/embed_model cancel {MODEL_ID}")
    ));
    let pending = fixture
        .app
        .embed_downloads
        .get(MODEL_ID)
        .expect("cancel retains placeholder until join is consumed");
    assert_eq!(pending.task_id, id);
    assert!(pending.cancelling);
    assert!(!fixture.app.tasks.is_empty());
    let screen = rendered(&mut fixture.app);
    assert!(
        screen.contains(MODEL_ID),
        "cancelled task stays visible until join: {screen}"
    );
    assert!(
        screen.contains("Cancelling"),
        "cancellation state must render before join: {screen}"
    );
    drain_until(&mut fixture.app, |app| app.tasks.is_empty());
    assert!(
        !fixture
            .attempt
            .as_ref()
            .unwrap()
            .server
            .release
            .opened
            .load(Ordering::SeqCst),
        "cancel must reclaim the client while server is still blocked"
    );
    assert!(fixture.app.embed_downloads.is_empty());
    assert!(fixture.app.task_progress.is_empty());
    assert!(model_files(&fixture.models_root()).is_empty());
    assert!(fixture.app.success_message.is_none());
    assert!(fixture.app.error_message.is_none());
    fixture.close_cancelled_response();

    // 同一模型在实际回收后能再次启动；退出命令及真实 shutdown 都走 App 入口。
    let (next, _) = fixture.start(ResponseMode::Complete);
    assert_ne!(next, id);
    assert!(command(&mut fixture.app, "/exit"));
    assert!(matches!(fixture.app.phase, AppPhase::Quit));
    fixture.app.shutdown_tasks().unwrap();
    assert!(fixture.app.tasks.is_empty());
    assert!(fixture.app.embed_downloads.is_empty());
    assert!(fixture.app.task_progress.is_empty());
    assert!(model_files(&fixture.models_root()).is_empty());
    assert!(!fixture.app.vault_service.is_unlocked());
    fixture.close_cancelled_response();
}

#[test]
fn rf212_truncated_or_bad_hash_stream_leaves_no_model_and_can_retry() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    for mode in [ResponseMode::Truncated, ResponseMode::WrongHash] {
        let mut fixture = DownloadFixture::new(false);
        let (failed, _) = fixture.start(mode);
        fixture.finish_response();
        assert!(
            fixture.app.error_message.is_some(),
            "{mode:?} must report a real download failure"
        );
        if matches!(mode, ResponseMode::WrongHash) {
            assert!(fixture
                .app
                .error_message
                .as_ref()
                .unwrap()
                .contains("sha256"));
        }
        assert!(fixture.app.success_message.is_none());
        assert!(fixture.app.embed_downloads.is_empty());
        assert!(fixture.app.task_progress.is_empty());
        assert!(
            model_files(&fixture.models_root()).is_empty(),
            "{mode:?} leaves no temporary file"
        );
        assert!(scan_local_models(&fixture.models_root()).is_empty());

        let (retried, _) = fixture.start(ResponseMode::Complete);
        assert_ne!(failed, retried);
        fixture.finish_response();
        fixture.assert_published();
    }
}

#[derive(Clone, Copy, Debug)]
enum SessionChange {
    Lock,
    AutoLock,
    SameAccount,
    OtherAccount,
    DirectVaultLock,
}

#[test]
fn rf212_lock_reunlock_and_account_switch_reject_old_download_publication() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    for change in [
        SessionChange::Lock,
        SessionChange::AutoLock,
        SessionChange::SameAccount,
        SessionChange::OtherAccount,
        SessionChange::DirectVaultLock,
    ] {
        let mut fixture = DownloadFixture::new(matches!(change, SessionChange::OtherAccount));
        let (id, progress) = fixture.start(ResponseMode::Complete);
        if matches!(change, SessionChange::DirectVaultLock) {
            // 不走 App cancel_all：让真实网络完整返回，提交必须自行拒绝原会话。
            fixture.app.vault_service.lock();
            let server = &mut fixture.attempt.as_mut().unwrap().server;
            server.release.open();
            server.join().unwrap();
            let terminal = take_terminal(&mut fixture.app, id);
            assert!(matches!(terminal.kind, TaskEventKind::Cancelled));
            assert!(model_files(&fixture.models_root()).is_empty());
            fixture.app.handle_event(Event::Task(terminal)).unwrap();
        } else {
            if matches!(change, SessionChange::AutoLock) {
                let deadline = fixture.app.last_activity + fixture.app.auto_lock_duration;
                assert!(!fixture.app.handle_event_at(Event::Tick, deadline).unwrap());
                assert!(matches!(fixture.app.phase, AppPhase::Locked));
                assert!(
                    fixture.app.error_message.is_some(),
                    "real idle-lock feedback must remain"
                );
            } else {
                auth::lock(&mut fixture.app);
            }
            assert!(fixture.app.embed_downloads.is_empty());
            assert!(fixture.app.task_progress.is_empty());
            match change {
                SessionChange::SameAccount => {
                    unlock_download_account(&mut fixture.app, &fixture.account_a)
                }
                SessionChange::OtherAccount => {
                    unlock_download_account(&mut fixture.app, fixture.account_b.as_deref().unwrap())
                }
                SessionChange::Lock | SessionChange::AutoLock => {}
                SessionChange::DirectVaultLock => unreachable!(),
            }
            if let Some(account) = fixture.app.vault_service.get_current_account() {
                let current = fixture.app.vault_service.capture_session(&account).unwrap();
                assert_ne!(current.generation(), progress.identity.session_generation);
            }
        }
        let info = fixture.app.info_message.clone();
        let error = fixture.app.error_message.clone();
        let account = fixture.app.vault_service.get_current_account();
        drain_until(&mut fixture.app, |app| app.tasks.is_empty());
        assert_eq!(
            fixture.app.info_message, info,
            "{change:?} must not publish old completion/cancellation"
        );
        assert_eq!(fixture.app.error_message, error);
        assert!(fixture.app.success_message.is_none());
        assert!(fixture.app.embed_downloads.is_empty());
        assert!(fixture.app.task_progress.is_empty());
        assert!(
            model_files(&fixture.models_root()).is_empty(),
            "{change:?} must not publish or retain staging files"
        );
        assert_eq!(fixture.app.vault_service.get_current_account(), account);
        match change {
            SessionChange::Lock | SessionChange::AutoLock => {
                assert!(matches!(fixture.app.phase, AppPhase::Locked))
            }
            SessionChange::SameAccount | SessionChange::OtherAccount => {
                assert!(
                    matches!(&fixture.app.phase, AppPhase::Home { account_id } if Some(account_id) == account.as_ref())
                );
            }
            SessionChange::DirectVaultLock => {}
        }
        fixture.close_cancelled_response();
    }
}

#[test]
fn rf212_old_real_progress_and_joined_terminal_cannot_clear_retried_download() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut fixture = DownloadFixture::new(false);
    let (old_id, old_progress) = fixture.start(ResponseMode::Complete);
    assert!(!command(
        &mut fixture.app,
        &format!("/embed_model cancel {MODEL_ID}")
    ));
    let old_terminal = take_terminal(&mut fixture.app, old_id);
    assert!(matches!(old_terminal.kind, TaskEventKind::Cancelled));
    fixture
        .app
        .handle_event(Event::Task(old_terminal.clone()))
        .unwrap();
    assert!(fixture.app.tasks.is_empty());
    fixture.close_cancelled_response();
    let (new_id, _) = fixture.start(ResponseMode::Complete);
    assert_ne!(old_id, new_id);
    let progress = fixture.app.task_progress.clone();
    let info = fixture.app.info_message.clone();
    let error = fixture.app.error_message.clone();
    for event in [old_progress, old_terminal] {
        assert!(!fixture.app.handle_event(Event::Task(event)).unwrap());
        assert_eq!(
            fixture.app.embed_downloads.get(MODEL_ID).unwrap().task_id,
            new_id
        );
        assert!(
            !fixture
                .app
                .embed_downloads
                .get(MODEL_ID)
                .unwrap()
                .cancelling
        );
        assert_eq!(fixture.app.task_progress, progress);
        assert_eq!(fixture.app.info_message, info);
        assert_eq!(fixture.app.error_message, error);
        assert!(fixture.app.success_message.is_none());
    }
    fixture.finish_response();
    fixture.assert_published();
}

#[test]
fn rf212_invalid_ids_and_already_installed_target_never_dispatch_or_overwrite() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut fixture = DownloadFixture::new(false);
    // 若错误地派发下载，也只可能接触本机合成服务器；Drop 会取消并回收。
    fixture.attempt = Some(HttpAttempt::new(ResponseMode::Complete));
    std::fs::create_dir_all(fixture.target().parent().unwrap()).unwrap();
    let sentinel = b"RF212 existing complete local model";
    std::fs::write(fixture.target(), sentinel).unwrap();
    let before = model_files(&fixture.models_root());
    for id in [
        "",
        ".",
        "..",
        "../escape",
        r"..\escape",
        "CON",
        "LPT1",
        "bad/id",
    ] {
        fixture.app.error_message = None;
        handle(&mut fixture.app, &["install", id]).unwrap();
        assert!(
            fixture.app.error_message.is_some(),
            "invalid model id {id:?} must be rejected"
        );
        assert!(fixture.app.tasks.is_empty());
        assert!(fixture.app.embed_downloads.is_empty());
        assert_eq!(model_files(&fixture.models_root()), before);
    }
    assert!(!command(
        &mut fixture.app,
        &format!("/embed_model install {MODEL_ID}")
    ));
    assert!(fixture.app.error_message.is_none());
    assert!(fixture
        .app
        .info_message
        .as_ref()
        .is_some_and(|text| text.contains(MODEL_ID)));
    assert!(fixture.app.tasks.is_empty());
    assert!(fixture.app.embed_downloads.is_empty());
    assert_eq!(std::fs::read(fixture.target()).unwrap(), sentinel);
    assert_eq!(model_files(&fixture.models_root()), before);
}

#[test]
fn rf212_target_created_while_downloading_is_not_overwritten_at_publication() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut fixture = DownloadFixture::new(false);
    fixture.start(ResponseMode::Complete);
    // 在首次预检之后放置合法已有模型，覆盖最终发布而非仅 install 的快速检查。
    std::fs::create_dir_all(fixture.target().parent().unwrap()).unwrap();
    let sentinel = b"RF212 another writer published this model";
    std::fs::write(fixture.target(), sentinel).unwrap();
    fixture.finish_response();
    assert!(fixture.app.error_message.is_some());
    assert!(fixture.app.success_message.is_none());
    assert!(fixture.app.embed_downloads.is_empty());
    assert!(fixture.app.task_progress.is_empty());
    assert_eq!(std::fs::read(fixture.target()).unwrap(), sentinel);
    assert_eq!(
        model_files(&fixture.models_root()),
        vec![(PathBuf::from(MODEL_ID).join("model.bin"), sentinel.to_vec())]
    );
    let installed = scan_local_models(&fixture.models_root());
    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].id, MODEL_ID);
    assert!(installed[0].installed);
}
