//! RF211：在隔离测试进程中安装真实 panic hook，避免污染并行测试的全局 hook。

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use solosoul_core::VaultService;
use tracing_appender::non_blocking::WorkerGuard;

use super::install_panic_hook_with;
use crate::tasks::{TaskEvent, TaskEventKind, TaskOutput, Tasks};

const CHILD_MODE: &str = "SOLOSOUL_RF211_PANIC_CHILD";
const CHILD_ROOT: &str = "SOLOSOUL_RF211_PANIC_ROOT";
const CANARY: &str = "RF211_SYNTHETIC_PRIVATE_PANIC_PAYLOAD";
const WORKER_WAIT: Duration = Duration::from_secs(15);
const CHILD_WAIT: Duration = Duration::from_secs(60);
const OWNER_MESSAGE: &str = "CLI terminated after an internal error";
const BACKGROUND_MESSAGE: &str = "CLI background task panicked";

/// 使用文件捕获输出，避免 pipe 缓冲填满让子进程无法退出；超时和断言失败均回收进程。
struct ChildGuard {
    child: Child,
    reaped: bool,
}

impl ChildGuard {
    fn wait(&mut self) -> io::Result<ExitStatus> {
        let deadline = Instant::now() + CHILD_WAIT;
        loop {
            if let Some(status) = self.child.try_wait()? {
                self.reaped = true;
                return Ok(status);
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "RF211 panic child timed out",
                ));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn child_root(mode: &str) -> Option<PathBuf> {
    if std::env::var(CHILD_MODE).ok().as_deref() != Some(mode) {
        return None;
    }
    let root = PathBuf::from(std::env::var_os(CHILD_ROOT).expect("isolated child root"))
        .canonicalize()
        .expect("parent-owned temporary directory");
    let temp = std::env::temp_dir().canonicalize().unwrap();
    assert!(
        root != temp && root.starts_with(&temp),
        "panic child may only use an independent system temporary directory"
    );
    Some(root)
}

fn run_isolated(test_name: &str, mode: &str) {
    let directory = tempfile::TempDir::new().unwrap();
    let root = directory.path().canonicalize().unwrap();
    let stdout_path = root.join("stdout.txt");
    let stderr_path = root.join("stderr.txt");
    let child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            test_name,
            "--nocapture",
            "--color=never",
            "--test-threads=1",
        ])
        .current_dir(&root)
        .env(CHILD_MODE, mode)
        .env(CHILD_ROOT, &root)
        .stdout(Stdio::from(File::create(&stdout_path).unwrap()))
        .stderr(Stdio::from(File::create(&stderr_path).unwrap()))
        .spawn()
        .expect("start isolated panic-hook test process");
    let mut child = ChildGuard {
        child,
        reaped: false,
    };
    let status = child
        .wait()
        .expect("isolated panic-hook test must finish within deadline");
    let stdout = String::from_utf8_lossy(&fs::read(stdout_path).unwrap()).into_owned();
    let stderr = String::from_utf8_lossy(&fs::read(stderr_path).unwrap()).into_owned();
    let log =
        fs::read_to_string(root.join("panic.log")).expect("child must create its real tracing log");
    for (name, contents) in [("stdout", &stdout), ("stderr", &stderr), ("log", &log)] {
        assert!(
            !contents.contains(CANARY),
            "panic payload escaped into {name}"
        );
        assert!(
            !contents.contains('\u{1b}'),
            "panic hook emitted terminal escape bytes into {name}"
        );
    }
    assert!(
        status.success(),
        "isolated panic-hook child failed: {status}"
    );
    assert!(
        stdout.contains("1 passed"),
        "exact child selector must execute its test"
    );
    if mode == "worker" {
        assert!(
            stderr.is_empty(),
            "background panic must not write into the TUI stderr"
        );
        assert!(!stdout.contains(OWNER_MESSAGE));
        assert_eq!(log.matches(BACKGROUND_MESSAGE).count(), 1);
    } else {
        assert_eq!(stderr.matches(OWNER_MESSAGE).count(), 1);
        assert!(!log.contains(BACKGROUND_MESSAGE));
    }
}

fn init_child_logging(root: &Path) -> WorkerGuard {
    let file = File::create(root.join("panic.log")).unwrap();
    let (writer, guard) = tracing_appender::non_blocking(file);
    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .without_time()
        .with_max_level(tracing::Level::ERROR)
        .try_init()
        .expect("isolated child owns the tracing subscriber");
    guard
}

struct Resource(Arc<AtomicBool>);

impl Drop for Resource {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

/// 测试断言失败时，也先回收实际 Future，再释放本测试的 Vault。
struct ManagedTasks(Tasks);

impl Drop for ManagedTasks {
    fn drop(&mut self) {
        if let Ok(runtime) = crate::util::shared_runtime() {
            runtime.block_on(self.0.shutdown());
        }
    }
}

fn next_event(tasks: &mut Tasks) -> TaskEvent {
    crate::util::shared_runtime().unwrap().block_on(async {
        tokio::time::timeout(WORKER_WAIT, async {
            loop {
                if let Some(event) = tasks.poll_events(1).pop() {
                    return event;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("real managed worker must reach a terminal event")
    })
}

#[test]
fn rf211_managed_worker_panic_keeps_terminal_and_payload_private() {
    let Some(root) = child_root("worker") else {
        run_isolated(
            "tui::panic_tests::rf211_managed_worker_panic_keeps_terminal_and_payload_private",
            "worker",
        );
        return;
    };
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let log_guard = init_child_logging(&root);
    let restored = Arc::new(AtomicUsize::new(0));
    let hook_restored = Arc::clone(&restored);
    install_panic_hook_with(move || {
        hook_restored.fetch_add(1, Ordering::AcqRel);
        Ok(())
    });
    let owner_thread = std::thread::current().id();
    let service = Arc::new(VaultService::with_base_path(root.join("vault")));
    let account = service
        .create_account("RF211 panic synthetic", crate::TEST_PASSWORD, None)
        .unwrap();
    let account_id = account["id"].as_str().unwrap();
    let session = service.capture_session(account_id).unwrap();
    let mut tasks = ManagedTasks(Tasks::new(Arc::clone(&service)));
    let dropped = Arc::new(AtomicBool::new(false));
    let worker_dropped = Arc::clone(&dropped);
    let worker_thread = Arc::new(std::sync::Mutex::new(None));
    let recorded_thread = Arc::clone(&worker_thread);
    tasks
        .0
        .spawn(session.clone(), move |_| async move {
            let _resource = Resource(worker_dropped);
            *recorded_thread.lock().unwrap() = Some(std::thread::current().id());
            panic!("{CANARY}");
        })
        .unwrap();
    let failed = next_event(&mut tasks.0);
    assert_eq!(
        failed.kind,
        TaskEventKind::Failed("后台任务执行失败".to_string())
    );
    assert!(
        dropped.load(Ordering::Acquire),
        "terminal follows actual Future unwind and resource drop"
    );
    assert_ne!(worker_thread.lock().unwrap().unwrap(), owner_thread);
    assert_eq!(
        restored.load(Ordering::Acquire),
        0,
        "caught worker panic must never restore the running terminal"
    );
    assert!(tasks.0.apply_event(failed, |_| {}));
    tasks
        .0
        .spawn(session, |_| async {
            Ok(TaskOutput::Message(
                "synthetic follow-up completed".to_string(),
            ))
        })
        .unwrap();
    let completed = next_event(&mut tasks.0);
    assert_eq!(
        completed.kind,
        TaskEventKind::Completed(TaskOutput::Message(
            "synthetic follow-up completed".to_string()
        ))
    );
    assert!(tasks.0.apply_event(completed, |_| {}));
    assert!(tasks.0.is_empty());
    assert_eq!(restored.load(Ordering::Acquire), 0);
    drop(tasks);
    drop(service);
    drop(log_guard); // WorkerGuard 真实 flush，父进程随后读文件检查 payload。
}

#[test]
fn rf211_owner_thread_panic_restores_once_without_printing_payload() {
    let Some(root) = child_root("owner") else {
        run_isolated(
            "tui::panic_tests::rf211_owner_thread_panic_restores_once_without_printing_payload",
            "owner",
        );
        return;
    };
    let log_guard = init_child_logging(&root);
    let restored = Arc::new(AtomicUsize::new(0));
    let hook_restored = Arc::clone(&restored);
    install_panic_hook_with(move || {
        hook_restored.fetch_add(1, Ordering::AcqRel);
        Ok(())
    });
    // catch_unwind 不绕过 hook；以实际同线程 panic 验证主 TUI 分支。
    let outcome = std::panic::catch_unwind(|| panic!("{CANARY}"));
    assert!(outcome.is_err());
    assert_eq!(restored.load(Ordering::Acquire), 1);
    drop(outcome);
    drop(log_guard);
}
