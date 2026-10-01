//! RF905 真实独立进程/句柄/worker 生命周期回归；不把同进程第二次 acquire 当作跨进程证据。
use crate::import_activity::{begin_import_maintenance, begin_owned_root_activity};
use crate::VaultFileSystem;
use crate::{LocalVaultFileSystem, VaultService};
use solosoul_vault::root_owner::VaultRootOwner;
use solosoul_vault::{VaultConfig, VaultStore};
use std::io::Write;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

const CHILD_LIMIT: Duration = Duration::from_secs(30);
const DENIED: i32 = 23;
const CHILD_TEST: &str = "root_ownership_tests::rf905_child_process";

struct ChildGuard(Option<Child>);
impl ChildGuard {
    fn finish(mut self) -> std::process::ExitStatus {
        let started = Instant::now();
        loop {
            if let Some(status) = self.0.as_mut().unwrap().try_wait().unwrap() {
                self.0.take();
                return status;
            }
            assert!(started.elapsed() < CHILD_LIMIT, "RF905 child did not exit");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn release(&mut self) {
        self.0
            .as_mut()
            .unwrap()
            .stdin
            .take()
            .unwrap()
            .write_all(b"release\n")
            .unwrap();
    }
}
impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
fn child(root: &Path, mode: &str) -> ChildGuard {
    ChildGuard(Some(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", CHILD_TEST, "--nocapture"])
            .env("SS_RF905_CHILD_ROOT", root)
            .env("SS_RF905_CHILD_MODE", mode)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    ))
}
pub(crate) fn probe(root: &Path, mode: &str) -> i32 {
    child(root, mode).finish().code().unwrap()
}
fn wait_ready(root: &Path, mode: &str) {
    let started = Instant::now();
    while !root.join(format!("{mode}.ready")).is_file() {
        assert!(
            started.elapsed() < CHILD_LIMIT,
            "RF905 child never acquired root"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn rf905_child_process() {
    let Ok(mode) = std::env::var("SS_RF905_CHILD_MODE") else {
        return;
    };
    let root = std::path::PathBuf::from(std::env::var_os("SS_RF905_CHILD_ROOT").unwrap());
    let owned = if mode.starts_with("owned") {
        match VaultService::try_with_base_path(root.clone()) {
            Ok(svc) => Some(svc),
            Err(error) => {
                assert_eq!(error, "VAULT_DIRECTORY_BUSY");
                std::process::exit(DENIED);
            }
        }
    } else {
        None
    };
    let legacy = if mode.starts_with("legacy") {
        match crate::process_lock::ProcessLock::acquire(&root) {
            Ok(lock) => Some(lock),
            Err(_) => std::process::exit(DENIED),
        }
    } else {
        None
    };
    assert!(owned.is_some() || legacy.is_some());
    if mode.ends_with("hold") {
        std::fs::write(root.join(format!("{mode}.ready")), b"owned").unwrap();
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
        assert_eq!(line, "release\n");
    } else if let Some(svc) = owned {
        svc.create_account_with_id("acc_child", "child", "child-password", None)
            .unwrap();
        std::fs::write(root.join("owned.written"), b"owned business committed").unwrap();
    } else {
        std::fs::write(root.join("legacy.written"), b"legacy business committed").unwrap();
    }
    // 真实 process exit 释放 OS handle，父进程实际重试，而不是模拟 Err。
    std::process::exit(0);
}

#[test]
fn rf905_independent_child_std_owner_and_existing_fs2_protocol_both_start_orders() {
    for mode in ["owned_hold", "legacy_hold"] {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        let mut holder = child(&root, mode);
        wait_ready(&root, mode);
        if mode == "owned_hold" {
            assert!(crate::process_lock::ProcessLock::acquire(&root).is_err());
        } else {
            assert_eq!(
                VaultService::try_with_base_path(root.clone())
                    .err()
                    .unwrap(),
                "VAULT_DIRECTORY_BUSY"
            );
        }
        let contender = if mode == "owned_hold" {
            "legacy_try"
        } else {
            "owned_try"
        };
        assert_eq!(probe(&root, contender), DENIED);
        assert!(!root.join("owned.written").exists());
        assert!(!root.join("legacy.written").exists());
        assert!(!root.join("acc_child/vault.db").exists());
        holder.release();
        assert!(holder.finish().success());
        assert_eq!(probe(&root, contender), 0);
        assert!(root
            .join(if contender == "owned_try" {
                "owned.written"
            } else {
                "legacy.written"
            })
            .is_file());
    }
}

#[test]
fn rf905_lock_failure_precedes_database_schema_and_no_implicit_reuse() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root");
    let svc = VaultService::try_with_base_path(root.clone()).unwrap();
    assert_eq!(svc.base_path(), &root.canonicalize().unwrap());
    assert!(svc.root_owner().is_process_locked());
    assert_eq!(
        VaultService::try_with_base_path(root.join("."))
            .err()
            .unwrap(),
        "VAULT_DIRECTORY_BUSY"
    );
    let config = VaultConfig::new("standalone", root.clone()).with_data_key([0x91; 32]);
    assert_eq!(
        VaultStore::open(config.clone()).err().unwrap(),
        "VAULT_DIRECTORY_BUSY"
    );
    assert!(!root.join("vault.db").exists());
    let store = VaultStore::open_owned(config, svc.root_owner()).unwrap();
    assert!(Arc::ptr_eq(&store.root_owner(), &svc.root_owner()));
    assert!(root.join("vault.db").is_file());
}

#[test]
fn rf905_explicit_owner_reuse_and_foreign_fs_store_roots_reject_before_write() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root");
    let svc = VaultService::try_with_base_path(root.clone()).unwrap();
    let owner = svc.root_owner();
    let same = VaultService::try_with_root_owner(
        Arc::clone(&owner),
        Arc::new(LocalVaultFileSystem::new(owner.root().to_path_buf())),
    )
    .unwrap();
    assert!(Arc::ptr_eq(&same.root_owner(), &owner));
    let foreign = dir.path().join("foreign");
    std::fs::create_dir(&foreign).unwrap();
    assert_eq!(
        VaultService::try_with_root_owner(
            Arc::clone(&owner),
            Arc::new(LocalVaultFileSystem::new(foreign.clone()))
        )
        .err()
        .unwrap(),
        "VAULT_ROOT_MISMATCH"
    );
    assert_eq!(
        VaultStore::open_owned(
            VaultConfig::new("foreign", foreign.clone()).with_data_key([0x37; 32]),
            owner
        )
        .err()
        .unwrap(),
        "VAULT_ROOT_MISMATCH"
    );
    assert!(!foreign.join("vault.db").exists());
}

#[test]
fn rf905_raw_managed_account_requires_owner_without_changing_real_schema() {
    let dir = tempfile::tempdir().unwrap();
    let svc = VaultService::try_with_base_path(dir.path().join("root")).unwrap();
    svc.create_account_with_id("acc_managed", "managed", "managed-password", None)
        .unwrap();
    let account = svc.base_path().join("acc_managed");
    let config_before = std::fs::read(account.join("config.json")).unwrap();
    let conn = rusqlite::Connection::open_with_flags(
        account.join("vault.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let schema = || -> (i64, Vec<(String, Option<String>)>) {
        let version = conn
            .query_row("PRAGMA schema_version", [], |row| row.get(0))
            .unwrap();
        let mut stmt = conn
            .prepare("SELECT name, sql FROM sqlite_master ORDER BY name")
            .unwrap();
        let entries = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        (version, entries)
    };
    let before = schema();
    assert_eq!(
        VaultStore::open(
            VaultConfig::new("acc_managed", account.clone()).with_data_key([0x42; 32])
        )
        .err()
        .unwrap(),
        "VAULT_ROOT_OWNER_REQUIRED"
    );
    assert_eq!(schema(), before);
    assert_eq!(
        std::fs::read(account.join("config.json")).unwrap(),
        config_before
    );
    assert!(!account.join(".lock").exists());
    assert!(svc.get_vault_store().unwrap().stats().is_ok());
}

#[test]
fn rf905_raw_unknown_config_is_preserved_and_standalone_without_config_still_opens() {
    for config_bytes in [b"not valid JSON".as_slice(), b"{}".as_slice()] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("config.json"), config_bytes).unwrap();
        assert_eq!(
            VaultStore::open(
                VaultConfig::new("unknown", dir.path().to_path_buf()).with_data_key([0x73; 32])
            )
            .err()
            .unwrap(),
            "VAULT_ROOT_OWNER_REQUIRED"
        );
        assert_eq!(
            std::fs::read(dir.path().join("config.json")).unwrap(),
            config_bytes
        );
        assert!(!dir.path().join("vault.db").exists());
        assert!(!dir.path().join(".lock").exists());
    }
    let dir = tempfile::tempdir().unwrap();
    let store = VaultStore::open(
        VaultConfig::new("standalone", dir.path().to_path_buf()).with_data_key([0x39; 32]),
    )
    .unwrap();
    assert!(store.root_owner().is_process_locked());
    assert!(dir.path().join("vault.db").is_file());
    assert!(store.stats().is_ok());
}

#[test]
fn rf905_each_store_session_and_fs_clone_keeps_owner_after_service_drop() {
    for kind in ["store", "session", "fs"] {
        let dir = tempfile::tempdir().unwrap();
        let svc = VaultService::try_with_base_path(dir.path().join("root")).unwrap();
        svc.create_account_with_id("acc_lifetime", "lifetime", "lifetime-password", None)
            .unwrap();
        let root = svc.base_path().clone();
        let weak = Arc::downgrade(&svc.root_owner());
        let held: Box<dyn std::any::Any + Send + Sync> = match kind {
            "store" => Box::new(svc.get_vault_store().unwrap()),
            "session" => Box::new(svc.capture_session("acc_lifetime").unwrap()),
            "fs" => Box::new(svc.file_system()),
            _ => unreachable!(),
        };
        drop(svc);
        assert!(weak.upgrade().is_some(), "{kind} failed to pin root");
        assert_eq!(probe(&root, "owned_try"), DENIED, "{kind}");
        assert!(!root.join("acc_child/config.json").exists());
        drop(held);
        assert!(weak.upgrade().is_none(), "{kind} leaked root owner");
        assert_eq!(probe(&root, "owned_try"), 0, "{kind}");
    }
}

struct ReleaseOnDrop(Option<mpsc::Sender<()>>);
impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        if let Some(tx) = self.0.take() {
            let _ = tx.send(());
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rf905_cancelled_awaiter_keeps_actual_worker_owner_and_shared_maintenance_gate() {
    let dir = tempfile::tempdir().unwrap();
    let svc = VaultService::try_with_base_path(dir.path().join("root")).unwrap();
    let root = svc.base_path().clone();
    let weak = Arc::downgrade(&svc.root_owner());
    let task = begin_owned_root_activity(svc.root_owner()).unwrap();
    let fs = svc.file_system();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (done_tx, done_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release = ReleaseOnDrop(Some(release_tx));
    let waiter = tokio::spawn(async move {
        tokio::task::spawn_blocking(move || {
            let _task = task;
            entered_tx.send(()).unwrap();
            release_rx.recv_timeout(CHILD_LIMIT).unwrap();
            fs.write_file("worker.written", b"actual worker completed")
                .unwrap();
            let _ = done_tx.send(());
        })
        .await
        .unwrap();
    });
    tokio::time::timeout(CHILD_LIMIT, entered_rx)
        .await
        .unwrap()
        .unwrap();
    waiter.abort();
    assert!(waiter.await.err().unwrap().is_cancelled());
    drop(svc);
    let blocked = begin_import_maintenance(&root).err();
    let child_denied = probe(&root, "owned_try");
    drop(release);
    let completed = tokio::time::timeout(CHILD_LIMIT, done_rx).await;
    let ended = tokio::time::timeout(CHILD_LIMIT, async {
        while weak.upgrade().is_some() {
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert_eq!(blocked.as_deref(), Some("IMPORT_OPERATIONS_ACTIVE"));
    assert_eq!(child_denied, DENIED);
    assert!(completed.is_ok_and(|result| result.is_ok()));
    assert!(ended.is_ok());
    assert_eq!(
        std::fs::read(root.join("worker.written")).unwrap(),
        b"actual worker completed"
    );
    let owner = VaultRootOwner::acquire(&root).unwrap();
    let maintenance =
        crate::import_activity::begin_owned_root_maintenance(Arc::clone(&owner)).unwrap();
    assert_eq!(
        begin_owned_root_activity(Arc::clone(&owner)).err().unwrap(),
        "IMPORT_DIRECTORY_BUSY"
    );
    drop(maintenance);
    assert!(begin_owned_root_activity(owner).is_ok());
}

struct BlockingCreateFs {
    inner: LocalVaultFileSystem,
    entered: std::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    release: std::sync::Mutex<mpsc::Receiver<()>>,
}
impl VaultFileSystem for BlockingCreateFs {
    fn read_file(&self, path: &str) -> Result<Vec<u8>, String> {
        self.inner.read_file(path)
    }
    fn write_file(&self, path: &str, data: &[u8]) -> Result<(), String> {
        self.inner.write_file(path, data)
    }
    fn write_file_atomic(&self, path: &str, data: &[u8]) -> Result<(), String> {
        if path.ends_with("/config.json") {
            if let Some(entered) = self.entered.lock().unwrap().take() {
                entered.send(()).map_err(|_| "test awaiter disappeared")?;
                self.release
                    .lock()
                    .unwrap()
                    .recv_timeout(CHILD_LIMIT)
                    .map_err(|_| "test create barrier timeout")?;
            }
        }
        self.inner.write_file_atomic(path, data)
    }
    fn remove_file(&self, path: &str) -> Result<(), String> {
        self.inner.remove_file(path)
    }
    fn exists(&self, path: &str) -> Result<bool, String> {
        self.inner.exists(path)
    }
    fn create_dir_all(&self, path: &str) -> Result<(), String> {
        self.inner.create_dir_all(path)
    }
    fn remove_dir_all(&self, path: &str) -> Result<(), String> {
        self.inner.remove_dir_all(path)
    }
    fn list_dir(&self, path: &str) -> Result<Vec<String>, String> {
        self.inner.list_dir(path)
    }
    fn local_path(&self, path: &str) -> Option<std::path::PathBuf> {
        self.inner.local_path(path)
    }
}

#[test]
fn rf905_both_create_entries_refuse_maintenance_before_account_or_session_writes() {
    let dir = tempfile::tempdir().unwrap();
    let svc = VaultService::try_with_base_path(dir.path().join("root")).unwrap();
    let maintenance =
        crate::import_activity::begin_owned_root_maintenance(svc.root_owner()).unwrap();
    assert_eq!(
        svc.create_account("blocked", "synthetic-password", None)
            .unwrap_err(),
        "IMPORT_DIRECTORY_BUSY"
    );
    assert_eq!(
        svc.create_account_with_id("acc_blocked", "blocked", "synthetic-password", None)
            .unwrap_err(),
        "IMPORT_DIRECTORY_BUSY"
    );
    assert!(!svc.has_any_account());
    assert!(!svc.is_unlocked());
    assert!(!svc.base_path().join("accounts.json").exists());
    assert!(!svc.base_path().join("acc_blocked").exists());
    drop(maintenance);
    svc.create_account_with_id("acc_blocked", "created", "synthetic-password", None)
        .unwrap();
    assert!(svc.base_path().join("acc_blocked/vault.db").is_file());
    assert!(svc.get_vault_store().unwrap().stats().is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rf905_cancelled_create_waiter_keeps_core_activity_before_first_session_until_real_return()
{
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root");
    let owner = VaultRootOwner::acquire(&root).unwrap();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release = ReleaseOnDrop(Some(release_tx));
    let fs = Arc::new(BlockingCreateFs {
        inner: LocalVaultFileSystem::new(owner.root().to_path_buf()),
        entered: std::sync::Mutex::new(Some(entered_tx)),
        release: std::sync::Mutex::new(release_rx),
    });
    let svc = Arc::new(VaultService::try_with_root_owner(owner, fs).unwrap());
    let worker_svc = Arc::clone(&svc);
    let (done_tx, done_rx) = tokio::sync::oneshot::channel();
    let waiter = tokio::spawn(async move {
        tokio::task::spawn_blocking(move || {
            // 没有外围普通activity；证明许可来自真实create Core入口。
            let result = worker_svc.create_account_with_id(
                "acc_pre_session",
                "create",
                "synthetic-password",
                None,
            );
            let _ = done_tx.send(result);
        })
        .await
        .unwrap();
    });
    tokio::time::timeout(CHILD_LIMIT, entered_rx)
        .await
        .unwrap()
        .unwrap();
    waiter.abort();
    assert!(waiter.await.err().unwrap().is_cancelled());
    let blocked = begin_import_maintenance(svc.base_path()).err();
    let had_session = svc.is_unlocked();
    let config_existed = svc.base_path().join("acc_pre_session/config.json").exists();
    let manifest_existed = svc.base_path().join("accounts.json").exists();
    drop(release);
    let completed = tokio::time::timeout(CHILD_LIMIT, done_rx).await;
    assert_eq!(blocked.as_deref(), Some("IMPORT_OPERATIONS_ACTIVE"));
    assert!(!had_session);
    assert!(!config_existed);
    assert!(!manifest_existed);
    let account = completed.unwrap().unwrap().unwrap();
    assert_eq!(account["id"], "acc_pre_session");
    assert!(svc
        .base_path()
        .join("acc_pre_session/config.json")
        .is_file());
    assert!(svc.base_path().join("acc_pre_session/vault.db").is_file());
    let maintenance =
        crate::import_activity::begin_owned_root_maintenance(svc.root_owner()).unwrap();
    assert!(svc.capture_session("acc_pre_session").is_ok());
    drop(maintenance);
}
