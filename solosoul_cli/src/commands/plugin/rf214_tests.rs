//! RF214：CLI 必须读取共享 PluginManager 实际使用的映射式注册表。
//! 所有目录均来自临时根，不调用使用默认 ~/.solosoul 的构造入口。

use crate::app::{App, AppPhase, UnlockStep};
use crate::commands::auth;
use crate::events::Event;
use crate::tasks::{TaskEvent, TaskEventKind, TaskId, TaskOutput};
use crossterm::event::{KeyCode, KeyEvent};
use sha2::{Digest, Sha256};
use solosoul_core::VaultService;
use solosoul_plugin::{PluginInstallPhase, PluginManager, PluginManifest, PluginStore};
use std::ffi::OsString;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use super::load_registry_entries;

#[test]
fn rf214_cli_reads_real_registry_map_and_uses_concrete_latest_version() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let directory = tempfile::TempDir::new().expect("synthetic plugin test root");
    let service = VaultService::with_base_path(directory.path().join("vault"));
    let account = service
        .create_account("RF214 synthetic", crate::TEST_PASSWORD, None)
        .expect("create synthetic Vault account");
    let account_id = account["id"].as_str().unwrap();
    let session = service
        .capture_session(account_id)
        .expect("capture synthetic session");
    assert_eq!(session.account_id(), account_id);

    let market = directory.path().join("market");
    let plugin_data = directory.path().join("plugin-data");
    std::fs::create_dir_all(&market).unwrap();
    let plugin_id = "com.solosoul.rf214.synthetic";
    let version = serde_json::json!({
        "sha256": "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce",
        "min_app_version": "0.0.0",
        "max_app_version": "99.0.0"
    });
    let registry = serde_json::json!({
        "plugins": {
            (plugin_id): {
                "name": "RF214 合成插件",
                "description": "Only synthetic local test data",
                "tier": "p3",
                "latest_version": "1.2.3",
                "versions": { "1.0.0": version, "1.2.3": version }
            }
        }
    });
    std::fs::write(
        market.join("registry.json"),
        serde_json::to_vec_pretty(&registry).unwrap(),
    )
    .unwrap();

    // 独立的共享生产解析器先证明夹具合法，失败不能归因于自造错误 schema。
    let manager = PluginManager::new_with_dirs(market.clone(), plugin_data.clone())
        .expect("manager uses only explicitly injected temporary directories");
    let available = manager
        .list_all(None)
        .expect("shared production registry accepts map schema");
    assert_eq!(available.len(), 1);
    assert_eq!(available[0].plugin_id, plugin_id);
    assert_eq!(
        available[0].registry_entry.latest_version.as_deref(),
        Some("1.2.3")
    );
    assert!(available[0].is_compatible);
    assert!(available[0].installed_version.is_none());
    assert!(manager.list_installed().unwrap().is_empty());

    // 旧 CLI 把 plugins 当作数组，实际在这里失败；不能回退到虚构的 "latest" 版本。
    let entries = load_registry_entries(&market, &plugin_data)
        .expect("CLI must accept the same map registry as PluginManager");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, plugin_id);
    assert_eq!(entries[0].version, "1.2.3");
    assert_eq!(entries[0].name, "RF214 合成插件");
    assert_eq!(
        entries[0].description.as_deref(),
        Some("Only synthetic local test data")
    );
    assert_eq!(entries[0].tier.as_deref(), Some("p3"));
    assert_eq!(
        std::fs::read_dir(plugin_data.join("plugins"))
            .unwrap()
            .count(),
        0,
        "registry reading must not install or execute any plugin"
    );
}

const PLUGIN: &str = "com.solosoul.rf214.synthetic";
const NAME: &str = "RF214 synthetic plugin";
const DESCRIPTION: &str = "Only synthetic local test data";
const V1: &str = "1.0.0";
const V2: &str = "1.2.3";
const WASM_V1: &[u8] = b"\0asm\x01\0\0\0";
// 第二份仍是合法空模块，增加一个名为 x 的空 custom section。
const WASM_V2: &[u8] = b"\0asm\x01\0\0\0\0\x02\x01x";
const FIRST: usize = 4;
const WAIT: Duration = Duration::from_secs(20);

/// 插件客户端是进程级共享客户端；每个测试在首次网络操作前指定 loopback 免代理。
struct ProxyEnvironment(Vec<(&'static str, Option<OsString>)>);

impl ProxyEnvironment {
    fn local_only() -> Self {
        let mut saved = Vec::new();
        for name in ["NO_PROXY", "no_proxy"] {
            saved.push((name, std::env::var_os(name)));
            std::env::set_var(name, "127.0.0.1,localhost");
        }
        Self(saved)
    }
}

impl Drop for ProxyEnvironment {
    fn drop(&mut self) {
        for (name, value) in self.0.drain(..).rev() {
            if let Some(value) = value {
                std::env::set_var(name, value);
            } else {
                std::env::remove_var(name);
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum BodyMode {
    Complete,
    Truncated,
    WrongHash,
}

/// 只服务 manifest 与 WASM；WASM 首块之后等待显式放行，不依赖 sleep 推断进度。
struct PluginServer {
    address: SocketAddr,
    first_block: mpsc::Receiver<()>,
    release: Option<mpsc::Sender<()>>,
    released: bool,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<Result<(), String>>>,
}

impl PluginServer {
    fn start(bytes: &'static [u8], mode: BodyMode, manifest: &serde_json::Value) -> Self {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let (release, released) = mpsc::channel();
        let (started, first_block) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let manifest = serde_json::to_vec(manifest).unwrap();
        let worker = thread::spawn(move || {
            for expected in ["/manifest.json", "/plugin.wasm"] {
                let (mut stream, _) = listener.accept().map_err(|e| e.to_string())?;
                if stopping.load(Ordering::SeqCst) {
                    return Ok(());
                }
                stream
                    .set_read_timeout(Some(WAIT))
                    .map_err(|e| e.to_string())?;
                stream
                    .set_write_timeout(Some(WAIT))
                    .map_err(|e| e.to_string())?;
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    if request.len() >= 16 * 1024 {
                        return Err("synthetic HTTP header too long".to_string());
                    }
                    let mut byte = [0];
                    stream.read_exact(&mut byte).map_err(|e| e.to_string())?;
                    request.push(byte[0]);
                }
                if !request.starts_with(format!("GET {expected} HTTP/1.1\r\n").as_bytes()) {
                    return Err(format!("unexpected loopback request; expected {expected}"));
                }
                let body = if expected == "/manifest.json" {
                    manifest.as_slice()
                } else {
                    bytes
                };
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .map_err(|e| e.to_string())?;
                if expected == "/plugin.wasm" {
                    stream
                        .write_all(&body[..FIRST])
                        .map_err(|e| e.to_string())?;
                    stream.flush().map_err(|e| e.to_string())?;
                    started.send(()).map_err(|e| e.to_string())?;
                    released.recv_timeout(WAIT).map_err(|e| e.to_string())?;
                    if stopping.load(Ordering::SeqCst) || matches!(mode, BodyMode::Truncated) {
                        return Ok(());
                    }
                    stream
                        .write_all(&body[FIRST..])
                        .map_err(|e| e.to_string())?;
                } else {
                    stream.write_all(body).map_err(|e| e.to_string())?;
                }
                stream.flush().map_err(|e| e.to_string())?;
            }
            Ok(())
        });
        Self {
            address,
            first_block,
            release: Some(release),
            released: false,
            stop,
            thread: Some(worker),
        }
    }

    fn release_body(&mut self) {
        self.released = true;
        if let Some(sender) = self.release.take() {
            let _ = sender.send(());
        }
    }

    fn finish(&mut self) {
        self.release_body();
        self.join().expect("complete synthetic HTTP exchange");
    }

    fn join(&mut self) -> Result<(), String> {
        if let Some(worker) = self.thread.take() {
            worker
                .join()
                .map_err(|_| "synthetic HTTP worker panicked".to_string())?
        } else {
            Ok(())
        }
    }

    fn close_cancelled(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.release_body();
        let _ = TcpStream::connect_timeout(&self.address, Duration::from_secs(1));
        self.join()
            .expect("cancelled synthetic HTTP exchange joins");
    }
}

impl Drop for PluginServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.release_body();
        let _ = TcpStream::connect_timeout(&self.address, Duration::from_secs(1));
        let _ = self.join();
    }
}

struct Fixture {
    app: App,
    server: Option<PluginServer>,
    account_a: String,
    account_b: Option<String>,
    market: PathBuf,
    data: PathBuf,
    _environment: ProxyEnvironment,
    _directory: tempfile::TempDir,
}

impl Fixture {
    fn new(second_account: bool) -> Self {
        let environment = ProxyEnvironment::local_only();
        let directory = tempfile::TempDir::new().unwrap();
        let service = VaultService::with_base_path(directory.path().join("vault"));
        let account = service
            .create_account("RF214 A", crate::TEST_PASSWORD, None)
            .unwrap();
        let account_a = account["id"].as_str().unwrap().to_string();
        let account_b = second_account.then(|| {
            service
                .create_account("RF214 B", crate::TEST_PASSWORD, None)
                .unwrap()["id"]
                .as_str()
                .unwrap()
                .to_string()
        });
        service.lock();
        let market = directory.path().join("market");
        let data = directory.path().join("plugin-data");
        std::fs::create_dir_all(&market).unwrap();
        let mut app = App::new(Arc::new(service)).unwrap();
        app.plugin_test_dirs = Some((market.clone(), data.clone()));
        let mut fixture = Self {
            app,
            server: None,
            account_a,
            account_b,
            market,
            data,
            _environment: environment,
            _directory: directory,
        };
        unlock_account(&mut fixture.app, &fixture.account_a);
        fixture.app.i18n.set_locale("en-US");
        fixture
    }

    fn manager(&self) -> PluginManager {
        PluginManager::new_with_dirs(self.market.clone(), self.data.clone()).unwrap()
    }

    fn store(&self) -> PluginStore {
        PluginStore::new_with_data_dir(self.data.clone()).unwrap()
    }

    fn configure(&mut self, version: &str, bytes: &'static [u8], mode: BodyMode) {
        self.configure_manifest(
            version,
            bytes,
            mode,
            market_manifest(version, NAME, DESCRIPTION, &["contact.name"]),
        );
    }

    fn configure_manifest(
        &mut self,
        version: &str,
        bytes: &'static [u8],
        mode: BodyMode,
        manifest: serde_json::Value,
    ) {
        assert!(
            self.app.tasks.is_empty(),
            "previous task must be joined and consumed"
        );
        self.server.take();
        let server = PluginServer::start(bytes, mode, &manifest);
        let digest = if matches!(mode, BodyMode::WrongHash) {
            "0".repeat(64)
        } else {
            format!("{:x}", Sha256::digest(bytes))
        };
        let metadata = serde_json::json!({
            "sha256": digest, "min_app_version": "0.0.0", "max_app_version": "99.0.0",
            "download_url": format!("http://{}/plugin.wasm", server.address)
        });
        std::fs::write(
            self.market.join("registry.json"),
            serde_json::to_vec(&serde_json::json!({
                "plugins": { (PLUGIN): {
                    "name": manifest["name"], "description": manifest["description"], "tier": "p3",
                    "latest_version": version, "versions": { (version): metadata }
                }}
            }))
            .unwrap(),
        )
        .unwrap();
        self.server = Some(server);
    }

    fn begin(&mut self, updated: bool, bytes: &[u8]) -> (TaskId, TaskEvent) {
        dismiss_messages(&mut self.app);
        if updated {
            super::update_plugin(&mut self.app, Some(PLUGIN)).unwrap();
        } else {
            super::install_plugin(&mut self.app, Some(PLUGIN)).unwrap();
        }
        assert!(
            self.app.error_message.is_none(),
            "{:?}",
            self.app.error_message
        );
        let task = self
            .app
            .plugin_installs
            .get(PLUGIN)
            .expect("install task registered");
        assert_eq!(task.updated, updated);
        let id = task.task_id;
        let activity = self.app.last_activity;
        self.server
            .as_ref()
            .unwrap()
            .first_block
            .recv_timeout(WAIT)
            .expect("real WASM body reaches its first-block barrier");
        let progress = first_progress(&mut self.app, id, bytes.len() as u64);
        assert_eq!(
            self.app.last_activity, activity,
            "network progress is not user activity"
        );
        assert!(
            !self.server.as_ref().unwrap().released,
            "install returned before body release"
        );
        let task = self.app.plugin_installs.get(PLUGIN).unwrap();
        assert_eq!(task.progress.downloaded_bytes, FIRST as u64);
        assert_eq!(task.progress.total_bytes, Some(bytes.len() as u64));
        assert_eq!(task.progress.phase, PluginInstallPhase::Downloading);
        assert!(task.progress.percent < 100);
        (id, progress)
    }

    fn finish(&mut self) {
        self.server.as_mut().unwrap().finish();
        drain(&mut self.app);
    }

    fn close_cancelled(&mut self) {
        self.server.as_mut().unwrap().close_cancelled();
    }

    fn assert_installed(&self, version: &str, bytes: &[u8]) {
        let store = self.store();
        let manifest = store
            .load_manifest(PLUGIN)
            .expect("real committed manifest");
        assert_eq!(manifest.id, PLUGIN);
        assert_eq!(manifest.name, NAME);
        assert_eq!(manifest.version, version);
        assert_eq!(manifest.description, DESCRIPTION);
        assert_eq!(manifest.permissions, ["contact.name"]);
        assert!(manifest.network_policy.block_all_outbound);
        assert!(manifest.require_user_confirmation);
        assert_eq!(
            manifest.wasm_hash_sha256.as_deref(),
            Some(format!("{:x}", Sha256::digest(bytes)).as_str())
        );
        assert_eq!(store.load_wasm(PLUGIN).unwrap(), bytes);
        let installed = self.manager().list_installed().unwrap();
        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0].version, version);
        assert!(self.app.tasks.is_empty());
        assert!(self.app.plugin_installs.is_empty());
        assert!(
            self.app.error_message.is_none(),
            "{:?}",
            self.app.error_message
        );
        assert!(self
            .app
            .success_message
            .as_ref()
            .is_some_and(|(text, _)| text.contains(version)));
    }

    fn assert_not_installed(&self, before: &[(PathBuf, Vec<u8>)]) {
        assert!(self.manager().list_installed().unwrap().is_empty());
        assert_eq!(
            files(&self.data),
            before,
            "no partial install, staging file or install audit may survive"
        );
        assert!(self.app.plugin_installs.is_empty());
        assert!(self.app.tasks.is_empty());
        assert_no_completion(&self.app);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.app.shutdown_tasks();
        self.server.take();
        // 字段随后析构：任务和服务器都已回收，才恢复环境和清理临时根。
    }
}

fn key(app: &mut App, code: KeyCode) -> bool {
    app.handle_event(Event::Key(KeyEvent::from(code))).unwrap()
}

fn type_command(app: &mut App, text: &str) -> bool {
    assert!(app.command_input.is_empty());
    for character in text.chars() {
        assert!(!key(app, KeyCode::Char(character)));
    }
    key(app, KeyCode::Enter)
}

fn dismiss_messages(app: &mut App) {
    for _ in 0..2 {
        if app.error_message.is_none() && app.info_message.is_none() {
            break;
        }
        assert!(!key(app, KeyCode::Esc));
    }
    assert!(app.error_message.is_none() && app.info_message.is_none());
}

fn unlock_account(app: &mut App, account: &str) {
    auth::unlock(app).unwrap();
    if let AppPhase::UnlockWizard {
        step: UnlockStep::SelectAccount { accounts, .. },
    } = &app.phase
    {
        let selected = accounts
            .iter()
            .position(|entry| entry.id == account)
            .unwrap();
        for _ in 0..selected {
            assert!(!key(app, KeyCode::Down));
        }
        assert!(!key(app, KeyCode::Enter));
    }
    assert!(
        matches!(&app.phase, AppPhase::UnlockWizard { step: UnlockStep::EnterPassword { account_id, .. }} if account_id == account)
    );
    for character in crate::TEST_PASSWORD.chars() {
        assert!(!key(app, KeyCode::Char(character)));
    }
    assert!(!key(app, KeyCode::Enter));
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
        .expect("real task must join and be consumed");
}

fn first_progress(app: &mut App, id: TaskId, total: u64) -> TaskEvent {
    crate::util::shared_runtime().unwrap().block_on(async {
        tokio::time::timeout(WAIT, async {
            loop {
                let mut observed = None;
                for event in app.tasks.poll_events(32) {
                    if event.identity.task_id == id && matches!(&event.kind,
                        TaskEventKind::PluginProgress(p) if p.downloaded_bytes == FIRST as u64 && p.total_bytes == Some(total)) {
                        observed = Some(event.clone());
                    }
                    app.handle_event(Event::Task(event)).unwrap();
                }
                if let Some(event) = observed { return event; }
                tokio::task::yield_now().await;
            }
        }).await
    }).expect("actual plugin byte progress must arrive")
}

fn take_terminal(app: &mut App, id: TaskId) -> TaskEvent {
    crate::util::shared_runtime()
        .unwrap()
        .block_on(async {
            tokio::time::timeout(WAIT, async {
                loop {
                    for event in app.tasks.poll_events(32) {
                        if event.identity.task_id == id
                            && !matches!(
                                &event.kind,
                                TaskEventKind::PluginProgress(_) | TaskEventKind::Progress { .. }
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
        .expect("actual plugin task must join")
}

fn files(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn visit(root: &Path, path: &Path, result: &mut Vec<(PathBuf, Vec<u8>)>) {
        if !path.exists() {
            return;
        }
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                visit(root, &entry.path(), result);
            } else {
                result.push((
                    entry.path().strip_prefix(root).unwrap().to_path_buf(),
                    std::fs::read(entry.path()).unwrap(),
                ));
            }
        }
    }
    let mut result = Vec::new();
    visit(root, root, &mut result);
    result.sort_by(|a, b| a.0.cmp(&b.0));
    result
}

fn render(app: &mut App) -> String {
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(170, 38)).unwrap();
    terminal.draw(|frame| app.render(frame)).unwrap();
    terminal.backend().to_string()
}

fn assert_no_completion(app: &App) {
    if let Some((text, _)) = &app.success_message {
        assert!(
            !text.contains("installed successfully") && !text.contains("updated to"),
            "{text}"
        );
    }
}

fn assert_installed_list(app: &App, version: &str) {
    assert!(
        matches!(&app.phase, AppPhase::PluginList { plugins, installed_only: true, .. }
        if plugins.len() == 1 && plugins[0].id == PLUGIN && plugins[0].version == version
            && plugins[0].installed_version.as_deref() == Some(version))
    );
}

#[test]
fn rf214_slow_install_keeps_real_key_tick_and_progress_render_responsive() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut fixture = Fixture::new(false);
    fixture.configure(V1, WASM_V1, BodyMode::Complete);
    super::list_installed_plugins(&mut fixture.app).unwrap();
    let (id, _) = fixture.begin(false, WASM_V1);
    assert!(fixture.manager().list_installed().unwrap().is_empty());
    assert!(
        matches!(&fixture.app.phase, AppPhase::PluginList { plugins, installed_only: true, .. } if plugins.is_empty())
    );
    assert_no_completion(&fixture.app);

    // 已到真实 HTTP 首块，但服务器没有放行；普通键盘与 Tick 都必须继续执行。
    assert!(!key(&mut fixture.app, KeyCode::Char('x')));
    assert!(matches!(&fixture.app.phase, AppPhase::PluginList { filter, .. } if filter == "x"));
    assert!(!key(&mut fixture.app, KeyCode::Esc));
    let activity = fixture.app.last_activity;
    assert!(!fixture
        .app
        .handle_event_at(Event::Tick, activity + Duration::from_secs(1))
        .unwrap());
    assert_eq!(fixture.app.last_activity, activity);
    let screen = render(&mut fixture.app);
    assert!(screen.contains(PLUGIN), "{screen}");
    assert!(
        screen.contains("Downloading") && screen.contains("4 / 8 B"),
        "{screen}"
    );
    assert!(screen.contains("/plugin_cancel"), "{screen}");
    assert!(!fixture.server.as_ref().unwrap().released);

    super::install_plugin(&mut fixture.app, Some(PLUGIN)).unwrap();
    assert_eq!(
        fixture.app.plugin_installs.get(PLUGIN).unwrap().task_id,
        id,
        "same plugin cannot start a second installation while active"
    );
    dismiss_messages(&mut fixture.app);
    fixture.finish();
    fixture.assert_installed(V1, WASM_V1);
    assert_installed_list(&fixture.app, V1);
    let screen = render(&mut fixture.app);
    assert!(
        screen.contains(NAME) && screen.contains("Installed:"),
        "{screen}"
    );
    assert_eq!(fixture.manager().audit_log(Some(20)).unwrap().len(), 1);
}

#[test]
fn rf214_success_keeps_help_open_and_refreshes_the_real_back_destination() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut fixture = Fixture::new(false);
    fixture.configure(V1, WASM_V1, BodyMode::Complete);
    super::list_installed_plugins(&mut fixture.app).unwrap();
    fixture.begin(false, WASM_V1);
    assert!(!type_command(&mut fixture.app, "/help plugin_install"));
    assert!(
        matches!(&fixture.app.phase, AppPhase::Help { topic: Some(topic), .. } if topic == "plugin_install")
    );
    assert!(
        matches!(&fixture.app.previous_phase, Some(AppPhase::PluginList { plugins, installed_only: true, .. }) if plugins.is_empty())
    );
    fixture.finish();
    fixture.assert_installed(V1, WASM_V1);
    assert!(
        matches!(&fixture.app.phase, AppPhase::Help { topic: Some(topic), .. } if topic == "plugin_install")
    );
    assert!(!type_command(&mut fixture.app, "/back"));
    assert_installed_list(&fixture.app, V1);
    let screen = render(&mut fixture.app);
    assert!(screen.contains(NAME) && screen.contains(V1), "{screen}");
}

#[test]
fn rf214_real_cancel_command_retains_slot_until_join_then_allows_retry() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut fixture = Fixture::new(false);
    fixture.configure(V1, WASM_V1, BodyMode::Complete);
    super::list_installed_plugins(&mut fixture.app).unwrap();
    let before = files(&fixture.data);
    let (id, _) = fixture.begin(false, WASM_V1);
    // 在真实 PluginList 内逐键输入，不能只调用 cancel_plugin 掩盖命令不可达。
    assert!(!type_command(
        &mut fixture.app,
        &format!("/plugin_cancel {PLUGIN}")
    ));
    let task = fixture
        .app
        .plugin_installs
        .get(PLUGIN)
        .expect("slot remains until actual join is consumed");
    assert_eq!(task.task_id, id);
    assert!(task.cancelling);
    assert!(!fixture.app.tasks.is_empty());
    let screen = render(&mut fixture.app);
    assert!(
        screen.contains(PLUGIN) && screen.contains("Cancelling"),
        "{screen}"
    );
    drain(&mut fixture.app);
    assert!(
        !fixture.server.as_ref().unwrap().released,
        "cancel must finish while server is still blocked"
    );
    fixture.assert_not_installed(&before);
    assert!(fixture.app.error_message.is_none());
    fixture.close_cancelled();

    fixture.configure(V1, WASM_V1, BodyMode::Complete);
    let (retried, _) = fixture.begin(false, WASM_V1);
    assert_ne!(retried, id);
    fixture.finish();
    fixture.assert_installed(V1, WASM_V1);
    assert_installed_list(&fixture.app, V1);
}

#[test]
fn rf214_hash_or_truncated_body_failure_never_installs_and_retry_can_succeed() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    for mode in [BodyMode::WrongHash, BodyMode::Truncated] {
        let mut fixture = Fixture::new(false);
        fixture.configure(V1, WASM_V1, mode);
        super::list_installed_plugins(&mut fixture.app).unwrap();
        let before = files(&fixture.data);
        let (failed, _) = fixture.begin(false, WASM_V1);
        fixture.finish();
        assert!(
            fixture.app.error_message.is_some(),
            "{mode:?} must report actual failure"
        );
        fixture.assert_not_installed(&before);
        assert!(
            matches!(&fixture.app.phase, AppPhase::PluginList { plugins, .. } if plugins.is_empty())
        );
        assert!(fixture.manager().audit_log(Some(20)).unwrap().is_empty());

        fixture.configure(V1, WASM_V1, BodyMode::Complete);
        let (retried, _) = fixture.begin(false, WASM_V1);
        assert_ne!(failed, retried);
        fixture.finish();
        fixture.assert_installed(V1, WASM_V1);
        assert_installed_list(&fixture.app, V1);
    }
}

#[test]
fn rf214_failed_update_keeps_old_manifest_wasm_and_audit_until_verified_retry() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut fixture = Fixture::new(false);
    fixture.configure(V1, WASM_V1, BodyMode::Complete);
    super::list_installed_plugins(&mut fixture.app).unwrap();
    fixture.begin(false, WASM_V1);
    fixture.finish();
    fixture.assert_installed(V1, WASM_V1);
    let before = files(&fixture.data);
    assert_eq!(fixture.manager().audit_log(Some(20)).unwrap().len(), 1);

    fixture.configure(V2, WASM_V2, BodyMode::WrongHash);
    let (failed, _) = fixture.begin(true, WASM_V2);
    assert_installed_list(&fixture.app, V1);
    fixture.finish();
    assert!(fixture.app.error_message.is_some());
    assert_no_completion(&fixture.app);
    assert_eq!(
        files(&fixture.data),
        before,
        "failed update must preserve every old file and audit byte"
    );
    assert_eq!(fixture.store().load_manifest(PLUGIN).unwrap().version, V1);
    assert_eq!(fixture.store().load_wasm(PLUGIN).unwrap(), WASM_V1);
    assert_installed_list(&fixture.app, V1);
    assert!(fixture.app.plugin_installs.is_empty());

    fixture.configure(V2, WASM_V2, BodyMode::Complete);
    let (retried, _) = fixture.begin(true, WASM_V2);
    assert_ne!(failed, retried);
    fixture.finish();
    fixture.assert_installed(V2, WASM_V2);
    assert_installed_list(&fixture.app, V2);
    assert_eq!(fixture.manager().audit_log(Some(20)).unwrap().len(), 2);
}

#[derive(Clone, Copy, Debug)]
enum Transition {
    Locked,
    AutoLocked,
    SameAccount,
    OtherAccount,
    DirectVaultLock,
}

fn transition(fixture: &mut Fixture, change: Transition) {
    if matches!(change, Transition::AutoLocked) {
        let deadline = fixture.app.last_activity + fixture.app.auto_lock_duration;
        assert!(!fixture.app.handle_event_at(Event::Tick, deadline).unwrap());
        assert!(fixture.app.error_message.is_some());
    } else {
        auth::lock(&mut fixture.app);
    }
    assert!(matches!(fixture.app.phase, AppPhase::Locked));
    assert!(fixture.app.plugin_installs.is_empty());
    match change {
        Transition::SameAccount => unlock_account(&mut fixture.app, &fixture.account_a),
        Transition::OtherAccount => {
            unlock_account(&mut fixture.app, fixture.account_b.as_deref().unwrap())
        }
        _ => {}
    }
}

#[test]
fn rf214_inflight_lock_reunlock_switch_and_final_commit_guard_prevent_publication() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    for change in [
        Transition::Locked,
        Transition::AutoLocked,
        Transition::SameAccount,
        Transition::OtherAccount,
        Transition::DirectVaultLock,
    ] {
        let mut fixture = Fixture::new(matches!(change, Transition::OtherAccount));
        fixture.configure(V1, WASM_V1, BodyMode::Complete);
        let before = files(&fixture.data);
        let (id, progress) = fixture.begin(false, WASM_V1);
        let terminal = if matches!(change, Transition::DirectVaultLock) {
            // 不调用 App cancel_all，也不 drain.cancel_stale；完整响应后必须由原会话 commit 拒绝。
            fixture.app.vault_service.lock();
            fixture.server.as_mut().unwrap().finish();
            let terminal = take_terminal(&mut fixture.app, id);
            assert!(matches!(&terminal.kind, TaskEventKind::Cancelled));
            Some(terminal)
        } else {
            transition(&mut fixture, change);
            None
        };
        let account = fixture.app.vault_service.get_current_account();
        if let Some(account) = account.as_deref() {
            assert_ne!(
                fixture
                    .app
                    .vault_service
                    .capture_session(account)
                    .unwrap()
                    .generation(),
                progress.identity.session_generation
            );
        }
        let info = fixture.app.info_message.clone();
        let error = fixture.app.error_message.clone();
        let success = fixture.app.success_message.clone();
        if let Some(terminal) = terminal {
            fixture.app.handle_event(Event::Task(terminal)).unwrap();
        }
        drain(&mut fixture.app);
        fixture.assert_not_installed(&before);
        assert_eq!(
            fixture.app.info_message, info,
            "{change:?} must not accept old task feedback"
        );
        assert_eq!(fixture.app.error_message, error);
        assert_eq!(fixture.app.success_message, success);
        assert_eq!(fixture.app.vault_service.get_current_account(), account);
        assert!(fixture.app.task_progress.is_empty());
        match change {
            Transition::Locked | Transition::AutoLocked => {
                assert!(matches!(fixture.app.phase, AppPhase::Locked))
            }
            Transition::SameAccount | Transition::OtherAccount => assert!(
                matches!(&fixture.app.phase, AppPhase::Home { account_id } if Some(account_id) == account.as_ref())
            ),
            Transition::DirectVaultLock => {}
        }
        fixture.close_cancelled();
    }
}

#[test]
fn rf214_old_real_progress_and_terminal_cannot_replace_retried_plugin_task() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut fixture = Fixture::new(false);
    fixture.configure(V1, WASM_V1, BodyMode::Complete);
    let (old, progress) = fixture.begin(false, WASM_V1);
    super::cancel_plugin(&mut fixture.app, Some(PLUGIN)).unwrap();
    let terminal = take_terminal(&mut fixture.app, old);
    assert!(matches!(&terminal.kind, TaskEventKind::Cancelled));
    fixture
        .app
        .handle_event(Event::Task(terminal.clone()))
        .unwrap();
    fixture.close_cancelled();
    fixture.configure(V1, WASM_V1, BodyMode::Complete);
    let (new, _) = fixture.begin(false, WASM_V1);
    assert_ne!(old, new);
    let current_progress = fixture
        .app
        .plugin_installs
        .get(PLUGIN)
        .unwrap()
        .progress
        .clone();
    let info = fixture.app.info_message.clone();
    let error = fixture.app.error_message.clone();
    let success = fixture.app.success_message.clone();
    for stale in [progress, terminal] {
        fixture.app.handle_event(Event::Task(stale)).unwrap();
        let task = fixture.app.plugin_installs.get(PLUGIN).unwrap();
        assert_eq!(task.task_id, new);
        assert!(!task.cancelling && !task.updated);
        assert_eq!(task.progress, current_progress);
        assert_eq!(fixture.app.info_message, info);
        assert_eq!(fixture.app.error_message, error);
        assert_eq!(fixture.app.success_message, success);
    }
    fixture.finish();
    fixture.assert_installed(V1, WASM_V1);
}

#[test]
fn rf214_committed_but_unconsumed_success_never_refills_a_new_session_ui() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    for change in [
        Transition::Locked,
        Transition::SameAccount,
        Transition::OtherAccount,
    ] {
        let mut fixture = Fixture::new(matches!(change, Transition::OtherAccount));
        fixture.configure(V1, WASM_V1, BodyMode::Complete);
        let (id, _) = fixture.begin(false, WASM_V1);
        fixture.server.as_mut().unwrap().finish();
        let completed = take_terminal(&mut fixture.app, id);
        assert!(
            matches!(&completed.kind, TaskEventKind::Completed(TaskOutput::PluginInstalled {
            plugin_id, version, updated: false, ..
        }) if plugin_id == PLUGIN && version == V1)
        );
        assert_eq!(fixture.store().load_wasm(PLUGIN).unwrap(), WASM_V1);
        let committed = files(&fixture.data);
        assert_no_completion(&fixture.app);
        transition(&mut fixture, change);
        let account = fixture.app.vault_service.get_current_account();
        let info = fixture.app.info_message.clone();
        let error = fixture.app.error_message.clone();
        let success = fixture.app.success_message.clone();
        fixture.app.handle_event(Event::Task(completed)).unwrap();
        drain(&mut fixture.app);
        assert!(fixture.app.plugin_installs.is_empty());
        assert_eq!(fixture.app.info_message, info);
        assert_eq!(fixture.app.error_message, error);
        assert_eq!(fixture.app.success_message, success);
        assert_eq!(fixture.app.vault_service.get_current_account(), account);
        assert!(!matches!(fixture.app.phase, AppPhase::PluginList { .. }));
        assert_eq!(
            files(&fixture.data),
            committed,
            "a file already committed before lock is retained; only stale UI is rejected"
        );
    }
}

#[test]
fn rf214_real_exit_waits_for_download_cleanup_without_server_release() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut fixture = Fixture::new(false);
    fixture.configure(V1, WASM_V1, BodyMode::Complete);
    let before = files(&fixture.data);
    fixture.begin(false, WASM_V1);
    assert!(!fixture.server.as_ref().unwrap().released);
    assert!(type_command(&mut fixture.app, "/exit"));
    assert!(matches!(fixture.app.phase, AppPhase::Quit));
    fixture.app.shutdown_tasks().unwrap();
    assert!(
        !fixture.server.as_ref().unwrap().released,
        "shutdown cannot require the blocked source to finish"
    );
    fixture.assert_not_installed(&before);
    assert!(!fixture.app.vault_service.is_unlocked());
    assert!(fixture.app.task_progress.is_empty());
    fixture.close_cancelled();
}

#[test]
fn rf214_invalid_ids_and_locked_install_never_dispatch_or_touch_plugin_files() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut fixture = Fixture::new(false);
    fixture.configure(V1, WASM_V1, BodyMode::Complete);
    let before = files(&fixture.data);
    for id in [
        None,
        Some(""),
        Some(".."),
        Some("../escape"),
        Some(r"..\escape"),
        Some("bad/id"),
    ] {
        dismiss_messages(&mut fixture.app);
        super::install_plugin(&mut fixture.app, id).unwrap();
        assert!(fixture.app.error_message.is_some(), "invalid id {id:?}");
        assert!(fixture.app.tasks.is_empty());
        assert!(fixture.app.plugin_installs.is_empty());
        assert_eq!(files(&fixture.data), before);
    }
    dismiss_messages(&mut fixture.app);
    auth::lock(&mut fixture.app);
    super::install_plugin(&mut fixture.app, Some(PLUGIN)).unwrap();
    assert!(fixture.app.error_message.is_some());
    assert!(fixture.app.tasks.is_empty());
    assert!(fixture.app.plugin_installs.is_empty());
    assert_eq!(files(&fixture.data), before);
    assert_no_completion(&fixture.app);
}

fn market_manifest(
    version: &str,
    name: &str,
    description: &str,
    required: &[&str],
) -> serde_json::Value {
    serde_json::json!({
        "plugin_id": PLUGIN, "version": version, "name": name,
        "description": description, "tier": "p3", "required_fields": required,
        "network_policy": {"blockAllOutbound": true, "allowedDomains": []},
        "require_user_confirmation": true
    })
}

const FILTER_A: &str = "com.solosoul.rf214.a";
const FILTER_C: &str = "com.solosoul.rf214.z";
const FILTER: &str = "matchme";
const A_NAME: &str = "MatchMe Alpha";

/// 使用真实 PluginStore 创建初始安装，不伪造 App 列表或成功事件。
fn save_initial_plugin(fixture: &Fixture, id: &str, name: &str, description: &str) {
    let manifest: PluginManifest = serde_json::from_value(serde_json::json!({
        "id": id, "name": name, "version": V1, "description": description,
        "permissions": ["contact.name"], "tier": "p3", "requireUserConfirmation": true,
        "wasmHashSha256": format!("{:x}", Sha256::digest(WASM_V1)),
        "networkPolicy": {"blockAllOutbound": true, "allowedDomains": []}
    }))
    .unwrap();
    fixture.store().save_plugin(&manifest, WASM_V1).unwrap();
    assert_eq!(fixture.store().load_wasm(id).unwrap(), WASM_V1);
}

fn seed_filtered_list(fixture: &mut Fixture, match_in_description: bool) {
    save_initial_plugin(fixture, FILTER_A, A_NAME, "unchanged A");
    save_initial_plugin(
        fixture,
        PLUGIN,
        if match_in_description {
            "Original Beta"
        } else {
            "MatchMe Beta"
        },
        if match_in_description {
            "MatchMe old description"
        } else {
            "old description"
        },
    );
    save_initial_plugin(fixture, FILTER_C, "Unrelated Gamma", "unchanged C");
    super::list_installed_plugins(&mut fixture.app).unwrap();
    assert!(
        matches!(&fixture.app.phase, AppPhase::PluginList { plugins, installed_only: true, .. } if plugins.len() == 3)
    );
    for character in FILTER.chars() {
        assert!(!key(&mut fixture.app, KeyCode::Char(character)));
    }
    assert!(!key(&mut fixture.app, KeyCode::Down));
    assert!(
        matches!(&fixture.app.phase, AppPhase::PluginList { selected: 1, filter, .. } if filter == FILTER)
    );
}

fn assert_filtered_a_is_selected_and_opens(fixture: &mut Fixture, total: usize) {
    assert!(
        matches!(&fixture.app.phase, AppPhase::PluginList { plugins, selected: 0, filter, installed_only: true }
        if plugins.len() == total && filter == FILTER),
        "selection is a visible-list index; one visible row must select index zero"
    );
    let screen = render(&mut fixture.app);
    assert!(screen.contains(A_NAME), "{screen}");
    assert!(
        !screen.contains("MatchMe Beta")
            && !screen.contains("Original Beta")
            && !screen.contains("Replacement Beta"),
        "{screen}"
    );
    assert!(!key(&mut fixture.app, KeyCode::Enter));
    assert!(
        matches!(&fixture.app.phase, AppPhase::PluginDetail { manifest }
        if manifest.id == FILTER_A && manifest.version == V1 && manifest.name == A_NAME),
        "Enter must open the remaining visible plugin, not a hidden row or no-op"
    );
}

#[test]
fn rf214_filtered_uninstall_clamps_current_and_cached_selection_before_enter() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    for cached in [false, true] {
        let mut fixture = Fixture::new(false);
        seed_filtered_list(&mut fixture, false);
        if cached {
            assert!(!type_command(&mut fixture.app, "/help plugin_uninstall"));
        }
        // 实际命令卸载被选中的 B；同样覆盖 /help 中缓存的过滤列表。
        assert!(!type_command(
            &mut fixture.app,
            &format!("/plugin_uninstall {PLUGIN}")
        ));
        assert!(
            fixture.app.error_message.is_none(),
            "{:?}",
            fixture.app.error_message
        );
        assert!(fixture.store().load_manifest(PLUGIN).is_err());
        assert_eq!(fixture.store().load_wasm(FILTER_A).unwrap(), WASM_V1);
        assert_eq!(fixture.store().load_wasm(FILTER_C).unwrap(), WASM_V1);
        if cached {
            assert!(
                matches!(&fixture.app.phase, AppPhase::Help { topic: Some(topic), .. } if topic == "plugin_uninstall")
            );
            assert!(!type_command(&mut fixture.app, "/back"));
        }
        assert_filtered_a_is_selected_and_opens(&mut fixture, 2);
    }
}

#[test]
fn rf214_http_update_that_leaves_filter_reselects_visible_row_and_refreshes_back_cache() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    // 名称匹配覆盖当前页，描述匹配覆盖导航缓存；两者都由真实 HTTP 更新减少可见项。
    for match_in_description in [false, true] {
        let mut fixture = Fixture::new(false);
        seed_filtered_list(&mut fixture, match_in_description);
        let manifest = market_manifest(
            V2,
            "Replacement Beta",
            "replacement description",
            &["contact.email"],
        );
        fixture.configure_manifest(V2, WASM_V2, BodyMode::Complete, manifest);
        fixture.begin(true, WASM_V2);
        assert!(
            matches!(&fixture.app.phase, AppPhase::PluginList { selected: 1, filter, .. } if filter == FILTER)
        );
        if match_in_description {
            assert!(!type_command(&mut fixture.app, "/help plugin_update"));
        }
        fixture.finish();
        let updated = fixture.store().load_manifest(PLUGIN).unwrap();
        assert_eq!(updated.version, V2);
        assert_eq!(updated.name, "Replacement Beta");
        assert_eq!(updated.description, "replacement description");
        assert_eq!(updated.permissions, ["contact.email"]);
        assert_eq!(fixture.store().load_wasm(PLUGIN).unwrap(), WASM_V2);
        assert!(
            fixture.app.error_message.is_none(),
            "{:?}",
            fixture.app.error_message
        );
        assert!(fixture.app.plugin_installs.is_empty());
        if match_in_description {
            assert!(
                matches!(&fixture.app.phase, AppPhase::Help { topic: Some(topic), .. } if topic == "plugin_update")
            );
            assert!(!type_command(&mut fixture.app, "/back"));
        }
        assert_filtered_a_is_selected_and_opens(&mut fixture, 3);
    }
}

#[test]
fn rf214_http_update_refreshes_complete_manifest_in_current_and_cached_detail() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    for cached in [false, true] {
        let mut fixture = Fixture::new(false);
        save_initial_plugin(&fixture, PLUGIN, NAME, DESCRIPTION);
        super::list_installed_plugins(&mut fixture.app).unwrap();
        assert!(!key(&mut fixture.app, KeyCode::Enter));
        assert!(
            matches!(&fixture.app.phase, AppPhase::PluginDetail { manifest }
            if manifest.id == PLUGIN && manifest.version == V1 && manifest.permissions == ["contact.name"])
        );

        let mut remote = market_manifest(
            V2,
            "RF214 updated detail",
            "updated detailed description",
            &["contact.email"],
        );
        remote["optional_fields"] = serde_json::json!(["contact.nickname"]);
        remote["data_ttl_seconds"] = serde_json::json!(42);
        remote["publisher"] = serde_json::json!("Synthetic Publisher");
        remote["category"] = serde_json::json!("rf214-updated-category");
        fixture.configure_manifest(V2, WASM_V2, BodyMode::Complete, remote);
        fixture.begin(true, WASM_V2);
        assert!(
            matches!(&fixture.app.phase, AppPhase::PluginDetail { manifest } if manifest.version == V1)
        );
        if cached {
            assert!(!type_command(&mut fixture.app, "/help plugin_update"));
        }
        fixture.finish();
        assert!(
            fixture.app.error_message.is_none(),
            "{:?}",
            fixture.app.error_message
        );
        assert!(fixture.app.plugin_installs.is_empty());
        let stored = fixture.store().load_manifest(PLUGIN).unwrap();
        assert_eq!(stored.version, V2);
        assert_eq!(stored.permissions, ["contact.email", "contact.nickname"]);
        assert_eq!(stored.data_ttl_seconds, 42);
        assert_eq!(fixture.store().load_wasm(PLUGIN).unwrap(), WASM_V2);
        if cached {
            assert!(
                matches!(&fixture.app.phase, AppPhase::Help { topic: Some(topic), .. } if topic == "plugin_update")
            );
            assert!(!type_command(&mut fixture.app, "/back"));
        }
        let AppPhase::PluginDetail { manifest } = &fixture.app.phase else {
            panic!("completion/back must preserve the selected plugin detail page");
        };
        assert_eq!(manifest.id, PLUGIN);
        assert_eq!(manifest.version, V2);
        assert_eq!(manifest.name, "RF214 updated detail");
        assert_eq!(manifest.permissions, ["contact.email", "contact.nickname"]);
        assert_eq!(manifest.author.as_deref(), Some("Synthetic Publisher"));
        assert_eq!(manifest.category, "rf214-updated-category");
        assert_eq!(manifest.data_ttl_seconds, 42);
        assert_eq!(serde_json::to_value(manifest).unwrap(), serde_json::to_value(&stored).unwrap(),
            "the UI must receive the entire verified manifest, including permission and policy fields");
        let screen = render(&mut fixture.app);
        assert!(
            screen.contains("RF214 updated detail") && screen.contains(V2),
            "{screen}"
        );
        assert!(
            screen.contains("contact.email") && screen.contains("contact.nickname"),
            "{screen}"
        );
        assert!(
            !screen.contains("contact.name"),
            "old permissions must not remain visible: {screen}"
        );
    }
}

#[test]
fn rf214_cli_registry_uses_same_cache_and_bundled_fallback_as_manager() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let directory = tempfile::TempDir::new().expect("synthetic registry cache test root");
    let market = directory.path().join("market");
    let plugin_data = directory.path().join("plugin-data");
    std::fs::create_dir_all(&market).unwrap();
    std::fs::create_dir_all(&plugin_data).unwrap();
    let plugin_id = "com.solosoul.rf214.cache-priority";
    let version = serde_json::json!({
        "sha256": "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce",
        "min_app_version": "0.0.0",
        "max_app_version": "99.0.0"
    });
    let bundled = serde_json::json!({
        "plugins": {
            (plugin_id): {
                "name": "RF214 bundled synthetic plugin",
                "description": "Synthetic bundled registry entry",
                "tier": "p3",
                "latest_version": "1.0.0",
                "versions": { "1.0.0": version.clone() }
            }
        }
    });
    let mut cached = bundled.clone();
    cached["plugins"][plugin_id]["latest_version"] = serde_json::json!("2.0.0");
    cached["plugins"][plugin_id]["versions"]["2.0.0"] = version;
    cached["plugins"][plugin_id]["name"] = serde_json::json!("RF214 cached synthetic plugin");
    let bundled_bytes = serde_json::to_vec_pretty(&bundled).unwrap();
    let cached_bytes = serde_json::to_vec_pretty(&cached).unwrap();
    let cache_path = plugin_data.join("registry.json");
    std::fs::write(market.join("registry.json"), &bundled_bytes).unwrap();
    std::fs::write(&cache_path, &cached_bytes).unwrap();

    // 独立的真实 Manager 持有同一市场/缓存目录；缓存保留 1.0.0，
    // 因而错误选旧版会是可执行的降级，而非夹具缺版本造成的无关错误。
    let manager = PluginManager::new_with_dirs(market.clone(), plugin_data.clone())
        .expect("manager uses only explicitly injected temporary directories");
    let available = manager.list_all(None).expect("read shared cached registry");
    let entries = load_registry_entries(&market, &plugin_data)
        .expect("CLI must read the same cache as its installation manager");
    assert_eq!(available.len(), 1);
    assert_eq!(entries.len(), 1);
    assert_eq!(available[0].plugin_id, plugin_id);
    assert_eq!(entries[0].id, plugin_id);
    assert_eq!(
        available[0].registry_entry.latest_version.as_deref(),
        Some("2.0.0")
    );
    assert!(available[0].registry_entry.versions.contains_key("1.0.0"));
    assert_eq!(entries[0].version, "2.0.0");
    assert_eq!(entries[0].name, "RF214 cached synthetic plugin");
    assert_eq!(entries[0].name, available[0].registry_entry.name);
    assert!(available[0].installed_version.is_none());
    assert_eq!(std::fs::read(&cache_path).unwrap(), cached_bytes);

    // 同一 Manager 不缓存旧选择；仅删除本测试的合成缓存后，两入口都回落 bundled。
    std::fs::remove_file(&cache_path).unwrap();
    let available = manager
        .list_all(None)
        .expect("fall back to bundled registry");
    let entries = load_registry_entries(&market, &plugin_data)
        .expect("CLI falls back only when the same cache is absent");
    assert_eq!(available.len(), 1);
    assert_eq!(entries.len(), 1);
    assert_eq!(available[0].plugin_id, plugin_id);
    assert_eq!(entries[0].id, plugin_id);
    assert_eq!(
        available[0].registry_entry.latest_version.as_deref(),
        Some("1.0.0")
    );
    assert_eq!(entries[0].version, "1.0.0");
    assert_eq!(entries[0].name, "RF214 bundled synthetic plugin");
    assert_eq!(entries[0].name, available[0].registry_entry.name);
    assert!(manager.list_installed().unwrap().is_empty());
    assert_eq!(
        std::fs::read(market.join("registry.json")).unwrap(),
        bundled_bytes
    );
    assert_eq!(
        std::fs::read_dir(plugin_data.join("plugins"))
            .unwrap()
            .count(),
        0,
        "registry projection must not install or execute a plugin"
    );
}
