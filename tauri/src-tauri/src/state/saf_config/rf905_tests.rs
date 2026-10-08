//! 实际 GUI 本地初始化入口与原 ProcessLock 的独立子进程竞争。
use crate::state::AppState;
use std::path::Path;
use std::process::Command;

const CHILD_ROOT: &str = "SOLOSOUL_RF905_GUI_CHILD_ROOT";

#[test]
fn fe2_mobile_cold_start_restores_app_private_accounts_but_keeps_them_locked() {
    let root = tempfile::tempdir().unwrap();
    let original = AppState::try_init_local_vault(root.path()).unwrap();
    let account = original
        .create_account("fe2-cold-synthetic", "synthetic-password", None)
        .unwrap();
    drop(original);

    // 实际重建服务，不复用内存账户缓存，也没有 SAF 配置。
    assert!(AppState::load_saved_saf_uri(root.path()).is_none());
    let restored = AppState::try_init_mobile_without_saf(root.path()).unwrap();
    assert_eq!(restored.base_path(), &root.path().canonicalize().unwrap());
    assert_eq!(restored.list_accounts().len(), 1);
    assert_eq!(
        restored.list_accounts()[0].id,
        account["id"].as_str().unwrap()
    );
    assert!(!restored.is_unlocked());
    assert!(!root.path().join(".uninitialized_vault").exists());
}

#[test]
fn fe2_mobile_first_launch_still_uses_placeholder_without_creating_accounts() {
    let root = tempfile::tempdir().unwrap();
    // UI 偏好、资源和锁文件均不构成用户已建立账户的证据。
    std::fs::write(root.path().join("ui_preferences.json"), b"{}").unwrap();
    std::fs::write(root.path().join(".lock"), b"").unwrap();
    let service = AppState::try_init_mobile_without_saf(root.path()).unwrap();
    assert_eq!(
        service.base_path(),
        &root
            .path()
            .join(".uninitialized_vault")
            .canonicalize()
            .unwrap()
    );
    assert!(!service.has_any_account());
    assert!(!root.path().join("accounts.json").exists());
}

#[test]
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn fe2_mobile_restored_root_busy_does_not_fall_back_to_an_empty_placeholder() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("accounts.json"), b"[]").unwrap();
    let lock = solosoul_core::process_lock::ProcessLock::acquire(root.path()).unwrap();
    assert!(AppState::try_init_mobile_without_saf(root.path())
        .err()
        .unwrap()
        .to_string()
        .contains("VAULT_DIRECTORY_BUSY"));
    assert!(!root.path().join(".uninitialized_vault").exists());
    drop(lock);
}
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
