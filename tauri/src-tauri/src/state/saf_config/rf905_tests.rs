//! 实际 GUI 本地初始化入口与原 ProcessLock 的独立子进程竞争。
use crate::state::AppState;
use std::path::Path;
use std::process::Command;

const CHILD_ROOT: &str = "SOLOSOUL_RF905_GUI_CHILD_ROOT";
#[test]
fn rf905_gui_child_contender() {
    let Ok(path) = std::env::var(CHILD_ROOT) else {
        return;
    };
    match AppState::try_init_local_vault(Path::new(&path)) {
        Ok(service) => {
            service
                .create_account("rf905-gui-child", "synthetic-password", None)
                .unwrap();
            std::process::exit(73);
        }
        Err(error) if error.to_string().contains("VAULT_DIRECTORY_BUSY") => std::process::exit(42),
        Err(error) => {
            eprintln!("unexpected GUI factory error: {error}");
            std::process::exit(43);
        }
    }
}

fn child(path: &Path) -> std::process::Output {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .arg("--exact")
        .arg("state::saf_config::rf905_tests::rf905_gui_child_contender")
        .arg("--nocapture")
        .env(CHILD_ROOT, path);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command.output().unwrap()
}

#[test]
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn rf905_gui_state_refuses_existing_process_lock_before_account_writes_and_recovers_after_release()
{
    let root = tempfile::tempdir().unwrap();
    let lock = solosoul_core::process_lock::ProcessLock::acquire(root.path()).unwrap();
    let rejected = child(root.path());
    assert_eq!(
        rejected.status.code(),
        Some(42),
        "GUI state factory must reject actual competing process lock before account writes: {}",
        String::from_utf8_lossy(&rejected.stderr)
    );
    assert!(!root.path().join("accounts.json").exists());
    assert!(!std::fs::read_dir(root.path()).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with("acc_")));
    drop(lock);
    let accepted = child(root.path());
    assert_eq!(
        accepted.status.code(),
        Some(73),
        "GUI state factory must recover after release: {}",
        String::from_utf8_lossy(&accepted.stderr)
    );
    assert!(root.path().join("accounts.json").is_file());
}

#[test]
fn rf905_saf_fallback_checks_both_owners_before_migration_and_keeps_lock_identity() {
    let root = tempfile::tempdir().unwrap();
    let cache = root.path().join("saf_vault_temp");
    std::fs::create_dir_all(&cache).unwrap();
    std::fs::write(cache.join(".lock"), b"cache-lock").unwrap();
    std::fs::write(root.path().join(".lock"), b"target-lock").unwrap();
    std::fs::write(cache.join("vault-payload"), b"payload").unwrap();
    let source = solosoul_vault::root_owner::VaultRootOwner::acquire(&cache).unwrap();
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        assert!(AppState::try_init_fallback_vault(root.path())
            .err()
            .unwrap()
            .to_string()
            .contains("VAULT_DIRECTORY_BUSY"));
        assert!(!root.path().join("vault-payload").exists());
    }
    drop(source);
    let target = solosoul_vault::root_owner::VaultRootOwner::acquire(root.path()).unwrap();
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        assert!(AppState::try_init_fallback_vault(root.path())
            .err()
            .unwrap()
            .to_string()
            .contains("VAULT_DIRECTORY_BUSY"));
        assert!(!root.path().join("vault-payload").exists());
        assert_eq!(
            std::fs::read(cache.join("vault-payload")).unwrap(),
            b"payload"
        );
    }
    drop(target);
    let service = AppState::try_init_fallback_vault(root.path()).unwrap();
    assert_eq!(
        std::fs::read(root.path().join("vault-payload")).unwrap(),
        b"payload"
    );
    assert!(!cache.join("vault-payload").exists());
    assert_eq!(std::fs::read(cache.join(".lock")).unwrap(), b"cache-lock");
    assert_eq!(service.base_path(), &root.path().canonicalize().unwrap());
    // Windows OS 排他锁拒绝读取已锁区域；确认源/目标业务断言后真实 Drop 再读锁内容。
    drop(service);
    assert_eq!(
        std::fs::read(root.path().join(".lock")).unwrap(),
        b"target-lock"
    );
}
