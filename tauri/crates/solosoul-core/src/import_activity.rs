//! RF022 的导入任务与目录维护准入。只协调已接入的本进程任务，不替代 RF905。
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

#[derive(Default)]
struct RootActivity {
    imports: usize,
    maintenance: bool,
}
static ROOTS: OnceLock<Mutex<HashMap<PathBuf, RootActivity>>> = OnceLock::new();
fn roots() -> &'static Mutex<HashMap<PathBuf, RootActivity>> {
    ROOTS.get_or_init(|| Mutex::new(HashMap::new()))
}
fn root_key(root: &Path) -> Result<PathBuf, String> {
    root.canonicalize()
        .map_err(|_| "IMPORT_DIRECTORY_UNAVAILABLE".to_string())
}

/// worker 真实持有，移入 blocking closure；取消等待不能提前释放此 guard。
pub struct ImportActivityGuard {
    root: PathBuf,
}
impl Drop for ImportActivityGuard {
    fn drop(&mut self) {
        // poison 后仍释放计数；新准入保持 fail-closed。
        let mut state = roots()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(entry) = state.get_mut(&self.root) {
            entry.imports = entry.imports.saturating_sub(1);
            if entry.imports == 0 && !entry.maintenance {
                state.remove(&self.root);
            }
        }
    }
}

/// 同 root 的多个正常任务可并行；同操作的写许可由数据库 epoch 决定。
pub fn begin_import_activity(root: &Path) -> Result<ImportActivityGuard, String> {
    let root = root_key(root)?;
    let mut states = roots()
        .lock()
        .map_err(|_| "IMPORT_DIRECTORY_BUSY".to_string())?;
    let state = states.entry(root.clone()).or_default();
    if state.maintenance {
        return Err("IMPORT_DIRECTORY_BUSY".into());
    }
    state.imports = state
        .imports
        .checked_add(1)
        .ok_or("IMPORT_DIRECTORY_BUSY")?;
    Ok(ImportActivityGuard { root })
}

pub struct ImportMaintenanceGuard {
    root: PathBuf,
}
impl Drop for ImportMaintenanceGuard {
    fn drop(&mut self) {
        let mut states = roots()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(state) = states.get_mut(&self.root) {
            state.maintenance = false;
            if state.imports == 0 {
                states.remove(&self.root);
            }
        }
    }
}

/// 先获得真实排他准入，再检查 journal；检查与后续维护期间不接受新导入。
pub fn begin_import_maintenance(root: &Path) -> Result<ImportMaintenanceGuard, String> {
    let root = root_key(root)?;
    let mut states = roots()
        .lock()
        .map_err(|_| "IMPORT_DIRECTORY_BUSY".to_string())?;
    let state = states.entry(root.clone()).or_default();
    if state.imports != 0 || state.maintenance {
        return Err("IMPORT_OPERATIONS_ACTIVE".into());
    }
    state.maintenance = true;
    Ok(ImportMaintenanceGuard { root })
}

/// 只读本机 journal 的不敏感阶段；无需保存/解密其他账户凭证。
/// 与 begin_import_maintenance 配对使用；单独调用仅是状态查询，不提供排他。
pub fn has_pending_imports(root: &Path, account_id: Option<&str>) -> Result<bool, String> {
    has_import_records(root, account_id, true)
}

/// 已完成记录也绑定 Native root；迁移协议交给 RF905，本项禁止静默搬走已绑定目录。
pub fn has_bound_imports(root: &Path) -> Result<bool, String> {
    has_import_records(root, None, false)
}

fn has_import_records(
    root: &Path,
    account_id: Option<&str>,
    pending_only: bool,
) -> Result<bool, String> {
    let root = root_key(root)?;
    if let Some(id) = account_id {
        if id.is_empty()
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err("IMPORT_ACCOUNT_MISMATCH".into());
        }
        return solosoul_vault::inspect_import_journal_at(&root, &root.join(id), pending_only);
    }
    for item in std::fs::read_dir(&root).map_err(|_| "IMPORT_DIRECTORY_CHECK_FAILED")? {
        let item = item.map_err(|_| "IMPORT_DIRECTORY_CHECK_FAILED")?;
        let kind = item
            .file_type()
            .map_err(|_| "IMPORT_DIRECTORY_CHECK_FAILED")?;
        if kind.is_symlink() {
            return Err("IMPORT_DIRECTORY_CHECK_FAILED".into());
        }
        if kind.is_dir()
            && solosoul_vault::inspect_import_journal_at(&root, &item.path(), pending_only)?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn ensure_imports_idle(root: &Path, account_id: Option<&str>) -> Result<(), String> {
    if has_pending_imports(root, account_id)? {
        Err("IMPORT_OPERATIONS_PENDING".into())
    } else {
        Ok(())
    }
}

/// 与维护 guard 配对，先保留 pending 的明确错误，再拒绝已有记录的 root 迁移。
pub fn ensure_import_root_movable(root: &Path) -> Result<(), String> {
    ensure_imports_idle(root, None)?;
    if has_bound_imports(root)? {
        Err("IMPORT_DIRECTORY_BOUND".into())
    } else {
        Ok(())
    }
}

/// 首次初始化只允许真实占位目录，不能作为已有 Vault 的热替换旁路。
pub fn begin_initial_import_setup(
    root: &Path,
    data_dir: &Path,
) -> Result<ImportMaintenanceGuard, String> {
    let root = root_key(root)?;
    let expected = root_key(&data_dir.join(".uninitialized_vault"))?;
    if root != expected {
        return Err("VAULT_DIRECTORY_ALREADY_INITIALIZED".into());
    }
    let guard = begin_import_maintenance(&root)?;
    ensure_import_root_movable(&root)?;
    Ok(guard)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn rf022_maintenance_waits_for_actual_owned_worker_and_blocks_new_admission() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let worker_root = root.clone();
        let worker = std::thread::spawn(move || {
            let _guard = begin_import_activity(&worker_root).unwrap();
            entered_tx.send(()).unwrap();
            release_rx.recv().unwrap();
        });
        entered_rx.recv().unwrap();
        assert_eq!(
            begin_import_maintenance(&root).err().unwrap(),
            "IMPORT_OPERATIONS_ACTIVE"
        );
        release_tx.send(()).unwrap();
        worker.join().unwrap();
        let maintenance = begin_import_maintenance(&root.join(".")).unwrap();
        assert_eq!(
            begin_import_activity(&root).err().unwrap(),
            "IMPORT_DIRECTORY_BUSY"
        );
        drop(maintenance);
        assert!(begin_import_activity(&root).is_ok());
    }

    #[test]
    fn rf022_pending_reader_preserves_legacy_complete_and_other_account_boundaries() {
        let dir = tempfile::tempdir().unwrap();
        for account in ["acc_a", "acc_b"] {
            let account_dir = dir.path().join(account);
            std::fs::create_dir(&account_dir).unwrap();
            let db = rusqlite::Connection::open(account_dir.join("vault.db")).unwrap();
            db.execute_batch("CREATE TABLE import_operations(phase TEXT); INSERT INTO import_operations VALUES('complete'),('abandoned');").unwrap();
        }
        let maintenance = begin_import_maintenance(dir.path()).unwrap();
        assert!(!has_pending_imports(dir.path(), None).unwrap());
        let db = rusqlite::Connection::open(dir.path().join("acc_b/vault.db")).unwrap();
        db.execute("INSERT INTO import_operations VALUES('attachments')", [])
            .unwrap();
        assert!(has_pending_imports(dir.path(), None).unwrap());
        assert!(!has_pending_imports(dir.path(), Some("acc_a")).unwrap());
        assert_eq!(
            ensure_imports_idle(dir.path(), Some("acc_b")).unwrap_err(),
            "IMPORT_OPERATIONS_PENDING"
        );
        assert!(has_pending_imports(dir.path(), Some("../acc_b")).is_err());
        drop(db);
        drop(maintenance);
        let legacy = dir.path().join("legacy");
        std::fs::create_dir(&legacy).unwrap();
        drop(rusqlite::Connection::open(legacy.join("vault.db")).unwrap());
        assert!(!has_pending_imports(dir.path(), Some("legacy")).unwrap());
    }

    #[test]
    fn rf022_worker_panic_releases_only_its_own_root_and_directory_errors_fail_closed() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let _active = begin_import_activity(second.path()).unwrap();
        let root = first.path().to_path_buf();
        assert!(std::thread::spawn(move || {
            let _guard = begin_import_activity(&root).unwrap();
            panic!("synthetic worker exit");
        })
        .join()
        .is_err());
        assert!(begin_import_maintenance(first.path()).is_ok());
        assert_eq!(
            begin_import_maintenance(second.path()).err().unwrap(),
            "IMPORT_OPERATIONS_ACTIVE"
        );
        let account = first.path().join("acc_bad");
        std::fs::create_dir(&account).unwrap();
        std::fs::write(account.join("vault.db"), b"invalid sqlite").unwrap();
        assert_eq!(
            has_pending_imports(first.path(), Some("acc_bad")).unwrap_err(),
            "IMPORT_DIRECTORY_CHECK_FAILED"
        );
    }
    #[test]
    fn rf022_completed_binding_blocks_relocation_but_not_key_maintenance_and_initialization_is_first_only(
    ) {
        let data = tempfile::tempdir().unwrap();
        let placeholder = data.path().join(".uninitialized_vault");
        std::fs::create_dir(&placeholder).unwrap();
        let initial = begin_initial_import_setup(&placeholder, data.path()).unwrap();
        assert_eq!(
            begin_import_activity(&placeholder).err().unwrap(),
            "IMPORT_DIRECTORY_BUSY"
        );
        drop(initial);
        let ordinary = data.path().join("existing");
        std::fs::create_dir(&ordinary).unwrap();
        assert_eq!(
            begin_initial_import_setup(&ordinary, data.path())
                .err()
                .unwrap(),
            "VAULT_DIRECTORY_ALREADY_INITIALIZED"
        );
        let account = ordinary.join("acc_one");
        std::fs::create_dir(&account).unwrap();
        let db = rusqlite::Connection::open(account.join("vault.db")).unwrap();
        db.execute_batch("CREATE TABLE import_operations(phase TEXT); INSERT INTO import_operations VALUES('complete');").unwrap();
        assert!(!has_pending_imports(&ordinary, None).unwrap());
        assert!(has_bound_imports(&ordinary).unwrap());
        assert_eq!(
            ensure_import_root_movable(&ordinary).unwrap_err(),
            "IMPORT_DIRECTORY_BOUND"
        );
        let maintenance = begin_import_maintenance(&ordinary).unwrap();
        ensure_imports_idle(&ordinary, None).unwrap();
        drop(maintenance);
        db.execute("DELETE FROM import_operations", []).unwrap();
        ensure_import_root_movable(&ordinary).unwrap();
    }
}
