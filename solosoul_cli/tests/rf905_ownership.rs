//! RF905：桌面真实子进程竞争、CLI 失锁零业务写、实际 worker 生命周期。
//! 所有 root/control 均为本用例 TempDir；不访问真实账户、网络或 GUI。
#![cfg(not(any(target_os = "android", target_os = "ios")))]

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use solosoul_cli::app::{App, AppPhase};
use solosoul_cli::commands::{doctor, system};
use solosoul_cli::tasks::{TaskFailure, Tasks};
use solosoul_core::import_activity::begin_owned_root_maintenance;
use solosoul_core::VaultService;
use solosoul_vault::root_owner::VaultRootOwner;
use tempfile::TempDir;

const WAIT: Duration = Duration::from_secs(30);
const PASSWORD: &str = "RF905 Synthetic password 2026!";
const ACCOUNT: &str = "acc_rf905_cli_fixture";

struct Actor {
    child: Option<Child>,
    control: PathBuf,
}
impl Actor {
    fn spawn(kind: &str, root: &Path, control: PathBuf) -> Self {
        fs::create_dir_all(&control).unwrap();
        let child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "rf905_subprocess_fixture",
                "--ignored",
                "--nocapture",
            ])
            .env("SOLOSOUL_RF905_TEST_KIND", kind)
            .env("SOLOSOUL_RF905_TEST_ROOT", root)
            .env("SOLOSOUL_RF905_TEST_CONTROL", &control)
            .env_remove("SOLOSOUL_DATA_DIR")
            .env("RUST_BACKTRACE", "0")
            .env("RUST_LIB_BACKTRACE", "0")
            .current_dir(&control)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        Self {
            child: Some(child),
            control,
        }
    }
    fn ready(&mut self) -> Value {
        let started = Instant::now();
        loop {
            let path = self.control.join("ready.json");
            if path.exists() {
                return serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
            }
            if let Some(status) = self.child.as_mut().unwrap().try_wait().unwrap() {
                let output = self.child.take().unwrap().wait_with_output().unwrap();
                panic!(
                    "RF905 child exited before ready {status}: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            assert!(started.elapsed() < WAIT, "RF905 child ready timeout");
            thread::sleep(Duration::from_millis(10));
        }
    }
    fn release(&self) {
        fs::write(self.control.join("release"), []).unwrap();
    }
    fn finish(&mut self) -> Value {
        let started = Instant::now();
        loop {
            if self.child.as_mut().unwrap().try_wait().unwrap().is_some() {
                break;
            }
            assert!(started.elapsed() < WAIT, "RF905 child exit timeout");
            thread::sleep(Duration::from_millis(10));
        }
        let output = self.child.take().unwrap().wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "RF905 child failure: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&fs::read(self.control.join("result.json")).unwrap()).unwrap()
    }
}
impl Drop for Actor {
    fn drop(&mut self) {
        // 只收尾本测试实际 spawn 的 Child handle；不按裸 PID 查找或终止其他进程。
        let _ = fs::write(self.control.join("release"), []);
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn write_control(path: &Path, value: &Value) {
    let temporary = path.with_extension("writing");
    fs::write(&temporary, serde_json::to_vec(value).unwrap()).unwrap();
    fs::rename(temporary, path).unwrap();
}
fn wait_for(path: &Path) {
    let started = Instant::now();
    while !path.exists() {
        assert!(started.elapsed() < WAIT, "RF905 barrier timeout");
        thread::sleep(Duration::from_millis(10));
    }
}
fn control_env(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(name).expect("RF905 child-only env"))
}
fn attempt(root: &Path, control: PathBuf) -> Value {
    let mut actor = Actor::spawn("try-write", root, control);
    actor.finish()
}
fn snapshot(root: &Path) -> BTreeMap<PathBuf, (bool, Vec<u8>)> {
    fn visit(root: &Path, path: &Path, rows: &mut BTreeMap<PathBuf, (bool, Vec<u8>)>) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            let metadata = fs::symlink_metadata(&path).unwrap();
            assert!(
                !metadata.file_type().is_symlink(),
                "synthetic fixture contains no links"
            );
            let relative = path.strip_prefix(root).unwrap().to_path_buf();
            if metadata.is_dir() {
                rows.insert(relative, (true, vec![]));
                visit(root, &path, rows);
            } else {
                assert!(metadata.is_file());
                // Windows OS 排他锁阻止读取 .lock 字节；保留其存在/类型，完整比较所有业务文件。
                let bytes = if relative == Path::new(".lock") {
                    assert_eq!(metadata.len(), 0, "fixture coordination lock stays empty");
                    vec![]
                } else {
                    fs::read(path).unwrap()
                };
                rows.insert(relative, (false, bytes));
            }
        }
    }
    let mut rows = BTreeMap::new();
    visit(root, root, &mut rows);
    rows
}
fn cli_while_busy(root: &Path, expected_error: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_solosoul"))
        .arg("--data-dir")
        .arg(root)
        .env_remove("SOLOSOUL_DATA_DIR")
        .env_remove("RUST_LOG")
        .env("RUST_BACKTRACE", "0")
        .env("RUST_LIB_BACKTRACE", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let started = Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if started.elapsed() >= WAIT {
            let _ = child.kill();
            let _ = child.wait();
            panic!("actual CLI must fail before TUI while root is owned");
        }
        thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    // 必须是同一权威构造的竞争错误，不能把 DLL/TUI/参数失败当成功证据。
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(expected_error),
        "actual CLI did not report ownership failure: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

// 这是仅供父用例显式启动的独立进程 fixture，默认 harness 不运行。
#[test]
#[ignore = "RF905 child-only subprocess fixture; parent tests invoke it explicitly"]
fn rf905_subprocess_fixture() {
    let kind = std::env::var("SOLOSOUL_RF905_TEST_KIND").unwrap();
    let root = control_env("SOLOSOUL_RF905_TEST_ROOT");
    let control = control_env("SOLOSOUL_RF905_TEST_CONTROL");
    if kind == "try-write" {
        let result = match VaultService::try_with_base_path(root) {
            Err(error) => json!({"acquired": false, "error": error}),
            Ok(service) => {
                service.load_accounts();
                let account = service
                    .create_account("RF905 child", PASSWORD, None)
                    .unwrap();
                let account_id = account["id"].as_str().unwrap().to_string();
                let app = App::new(Arc::new(service)).unwrap();
                assert!(app.vault_service.root_owner().is_process_locked());
                json!({"acquired": true, "accountId": account_id})
            }
        };
        write_control(&control.join("result.json"), &result);
        return;
    }
    let service = Arc::new(VaultService::try_with_base_path(root.clone()).unwrap());
    if kind == "hold-service" {
        write_control(
            &control.join("ready.json"),
            &json!({"owned": service.root_owner().is_process_locked()}),
        );
        wait_for(&control.join("release"));
        drop(service);
        write_control(&control.join("result.json"), &json!({"released": true}));
        return;
    }
    if kind == "hold-cli" {
        let mut app = App::new(Arc::clone(&service)).unwrap();
        doctor::run(&mut app).unwrap();
        let doctor_locked = match &app.phase {
            AppPhase::Doctor { report } => report.lock_acquired,
            _ => panic!("doctor phase"),
        };
        system::about(&mut app).unwrap();
        let about_locked = match &app.phase {
            AppPhase::About { info } => info.lock_acquired,
            _ => panic!("about phase"),
        };
        write_control(
            &control.join("ready.json"),
            &json!({"doctorLocked": doctor_locked, "aboutLocked": about_locked}),
        );
        wait_for(&control.join("release"));
        drop(app);
        drop(service);
        write_control(&control.join("result.json"), &json!({"released": true}));
        return;
    }
    assert!(kind == "worker-join" || kind == "worker-drop");
    service
        .create_account_with_id(ACCOUNT, "RF905 native worker", PASSWORD, None)
        .unwrap();
    let app = App::new(Arc::clone(&service)).unwrap();
    let session = service.capture_session(ACCOUNT).unwrap();
    let mut tasks = Tasks::new(Arc::clone(&service));
    let worker_control = control.clone();
    let task = tasks
        .spawn_blocking(session, move |context, _| {
            // 故意提前释放 Context：准入必须由实际原生闭包持有，而非 UI/Context。
            drop(context);
            fs::write(worker_control.join("entered"), []).unwrap();
            wait_for(&worker_control.join("release"));
            fs::write(worker_control.join("worker-returning"), []).unwrap();
            Err(TaskFailure::Cancelled)
        })
        .unwrap();
    wait_for(&control.join("entered"));
    assert!(tasks.request_cancel(task));
    if kind == "worker-join" {
        let busy_after_cancel = begin_owned_root_maintenance(service.root_owner()).is_err();
        assert!(busy_after_cancel, "native cancellation is not native exit");
        drop(app);
        write_control(
            &control.join("ready.json"),
            &json!({"maintenanceBusyAfterCancel": busy_after_cancel}),
        );
        let report = solosoul_cli::util::shared_runtime()
            .unwrap()
            .block_on(tasks.shutdown());
        assert_eq!(report.joined, 1);
        assert_eq!(report.cancelled, 1);
        let maintenance = begin_owned_root_maintenance(service.root_owner()).unwrap();
        drop(maintenance);
        drop(tasks);
        drop(service);
        write_control(
            &control.join("result.json"),
            &json!({"joined": report.joined, "maintenanceAfterJoin": true}),
        );
    } else {
        drop(app);
        drop(tasks);
        drop(service);
        write_control(&control.join("ready.json"), &json!({"uiDropped": true}));
        wait_for(&control.join("worker-returning"));
        // Context 已提前 Drop，App/Tasks/Service 已 Drop，仍须等待实际闭包 Drop。
        let started = Instant::now();
        loop {
            if let Ok(reopened) = VaultService::try_with_base_path(root.clone()) {
                drop(reopened);
                break;
            }
            assert!(
                started.elapsed() < WAIT,
                "root remained owned after native worker returned"
            );
            thread::sleep(Duration::from_millis(10));
        }
        write_control(
            &control.join("result.json"),
            &json!({"releasedAfterNativeReturn": true}),
        );
    }
}

#[test]
fn rf905_cli_binary_loses_to_service_before_logging_or_business_writes() {
    let directory = TempDir::new().unwrap();
    let root = directory.path().join("root");
    let mut holder = Actor::spawn("hold-service", &root, directory.path().join("holder"));
    assert_eq!(holder.ready()["owned"], true);
    fs::write(
        root.join("sentinel.bin"),
        b"RF905 immutable synthetic marker",
    )
    .unwrap();
    let before = snapshot(&root);
    let refused = attempt(&root, directory.path().join("refused"));
    assert_eq!(refused["acquired"], false);
    cli_while_busy(&root, refused["error"].as_str().unwrap());
    assert_eq!(
        snapshot(&root),
        before,
        "no config/DB/profile/logs or file bytes may change"
    );
    assert!(!root.join("logs").exists());
    holder.release();
    assert_eq!(holder.finish()["released"], true);
    let recovered = attempt(&root, directory.path().join("recovered"));
    assert_eq!(recovered["acquired"], true);
    assert!(root
        .join(recovered["accountId"].as_str().unwrap())
        .join("config.json")
        .is_file());
}

#[test]
fn rf905_cli_owner_has_correct_doctor_and_refuses_other_process_then_releases() {
    let directory = TempDir::new().unwrap();
    let root = directory.path().join("root");
    let mut holder = Actor::spawn("hold-cli", &root, directory.path().join("holder"));
    let report = holder.ready();
    assert_eq!(report["doctorLocked"], true);
    assert_eq!(report["aboutLocked"], true);
    let before = snapshot(&root);
    let refused = attempt(&root, directory.path().join("refused"));
    assert_eq!(refused["acquired"], false);
    cli_while_busy(&root, refused["error"].as_str().unwrap());
    assert_eq!(snapshot(&root), before);
    holder.release();
    holder.finish();
    assert_eq!(
        attempt(&root, directory.path().join("recovered"))["acquired"],
        true
    );
}

#[test]
fn rf905_cancelled_native_worker_blocks_maintenance_until_real_join() {
    let directory = TempDir::new().unwrap();
    let root = directory.path().join("root");
    let mut holder = Actor::spawn("worker-join", &root, directory.path().join("holder"));
    assert_eq!(holder.ready()["maintenanceBusyAfterCancel"], true);
    let before = snapshot(&root);
    let refused = attempt(&root, directory.path().join("refused"));
    assert_eq!(refused["acquired"], false);
    cli_while_busy(&root, refused["error"].as_str().unwrap());
    assert_eq!(snapshot(&root), before);
    holder.release();
    let report = holder.finish();
    assert_eq!(report["joined"], 1);
    assert_eq!(report["maintenanceAfterJoin"], true);
    assert_eq!(
        attempt(&root, directory.path().join("recovered"))["acquired"],
        true
    );
}

#[test]
fn rf905_ui_and_task_manager_drop_do_not_release_running_native_worker_owner() {
    let directory = TempDir::new().unwrap();
    let root = directory.path().join("root");
    let mut holder = Actor::spawn("worker-drop", &root, directory.path().join("holder"));
    assert_eq!(holder.ready()["uiDropped"], true);
    let before = snapshot(&root);
    let refused = attempt(&root, directory.path().join("refused"));
    assert_eq!(refused["acquired"], false);
    cli_while_busy(&root, refused["error"].as_str().unwrap());
    assert_eq!(snapshot(&root), before);
    holder.release();
    assert_eq!(holder.finish()["releasedAfterNativeReturn"], true);
    assert_eq!(
        attempt(&root, directory.path().join("recovered"))["acquired"],
        true
    );
}

struct BlockingLogFile {
    file: fs::File,
    entered: mpsc::Sender<()>,
    release: Option<mpsc::Receiver<()>>,
}
impl Write for BlockingLogFile {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if let Some(release) = self.release.take() {
            self.entered
                .send(())
                .map_err(|error| io::Error::other(error.to_string()))?;
            release
                .recv_timeout(WAIT)
                .map_err(|error| io::Error::other(error.to_string()))?;
        }
        self.file.write(buffer)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}
struct LogRelease {
    release: Option<mpsc::Sender<()>>,
    root: PathBuf,
}
impl LogRelease {
    fn release(&mut self) {
        if let Some(sender) = self.release.take() {
            let _ = sender.send(());
        }
    }
    fn wait_released(&self) -> bool {
        let started = Instant::now();
        loop {
            if let Ok(owner) = VaultRootOwner::acquire(&self.root) {
                drop(owner);
                return true;
            }
            if started.elapsed() >= WAIT {
                return false;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for LogRelease {
    fn drop(&mut self) {
        // 断言展开仍放行实际 writer，再等待其 owner/文件 Drop，之后才清理 TempDir。
        self.release();
        let _ = self.wait_released();
    }
}

#[test]
fn rf905_actual_logger_worker_pins_owner_after_worker_guard_timeout() {
    let directory = TempDir::new().unwrap();
    let root = directory.path().join("root");
    let owner = VaultRootOwner::acquire(&root).unwrap();
    let file = fs::File::create(root.join("synthetic-log.bin")).unwrap();
    let (entered, ready) = mpsc::channel();
    let (release, release_rx) = mpsc::channel();
    let mut release = LogRelease {
        release: Some(release),
        root: root.clone(),
    };
    let writer = BlockingLogFile {
        file,
        entered,
        release: Some(release_rx),
    };
    let writer = solosoul_cli::util::OwnerPinnedWriter::new(writer, owner);
    let (mut sink, guard) = tracing_appender::non_blocking(writer);
    sink.write_all(b"RF905 synthetic logging\n").unwrap();
    ready.recv_timeout(WAIT).unwrap();
    // Locked 0.2.5 的 Drop 只等待有界 ack，不 join，实际 file write 仍在屏障内。
    drop(guard);
    let before = snapshot(&root);
    let refused = attempt(&root, directory.path().join("refused"));
    assert_eq!(
        refused["acquired"], false,
        "owner must be held by the actual writer, not WorkerGuard"
    );
    cli_while_busy(&root, refused["error"].as_str().unwrap());
    assert_eq!(snapshot(&root), before);
    release.release();
    drop(sink);
    assert!(
        release.wait_released(),
        "actual log writer must eventually Drop its root owner"
    );
    assert_eq!(
        fs::read(root.join("synthetic-log.bin")).unwrap(),
        b"RF905 synthetic logging\n"
    );
    assert_eq!(
        attempt(&root, directory.path().join("recovered"))["acquired"],
        true
    );
}
