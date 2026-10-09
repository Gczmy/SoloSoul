//! RF905：Host 的目录任务在派发前登记；许可随真实 worker 保持。
use solosoul_core::import_activity::{
    begin_owned_root_activity, begin_owned_root_maintenance, ensure_import_root_movable,
    RootMaintenanceGuard,
};
use solosoul_core::VaultService;
use solosoul_vault::root_owner::VaultRootOwner;
use std::path::Path;
use std::sync::Arc;

pub(crate) fn ownership_error(error: String) -> anyhow::Error {
    if error == "VAULT_DIRECTORY_BUSY" {
        anyhow::anyhow!("Vault 数据目录正被其他 SoloSoul GUI/CLI 使用，请关闭后重试（{error}）")
    } else {
        anyhow::Error::msg(error)
    }
}

pub(crate) fn spawn_owned_blocking<T, F>(
    owner: Arc<VaultRootOwner>,
    work: F,
) -> Result<tauri::async_runtime::JoinHandle<T>, String>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let activity = begin_owned_root_activity(owner)?;
    #[cfg(all(feature = "native-perf", target_os = "windows"))]
    let activity = {
        let activity = Arc::new(activity);
        crate::native_perf::auth_trace::register_activity(
            crate::native_perf::maintenance_trace::ActivityKind::OwnedBlockingWorker,
            &activity,
        );
        activity
    };
    Ok(tauri::async_runtime::spawn_blocking(move || {
        let _activity = activity;
        work()
    }))
}

/// 切换前同时保留源、目标准入；目标失败不修改旧服务或配置。
/// 同根只显式复用当前 owner；迁移仍拒绝所有已绑定 journal。
pub(crate) struct RootTransition {
    target_owner: Arc<VaultRootOwner>,
    _source: Arc<RootMaintenanceGuard>,
    _target: Option<Arc<RootMaintenanceGuard>>,
}
impl RootTransition {
    pub(crate) fn target_owner(&self) -> Arc<VaultRootOwner> {
        Arc::clone(&self.target_owner)
    }
}

pub(crate) fn prepare_root_transition(
    service: &VaultService,
    target: &Path,
) -> Result<Arc<RootTransition>, String> {
    let source = service.root_owner();
    let source_guard = Arc::new(begin_owned_root_maintenance(Arc::clone(&source))?);
    ensure_import_root_movable(source.root())?;
    let same_root = target
        .canonicalize()
        .is_ok_and(|path| path == source.root());
    let (target_owner, target_guard) = if same_root {
        (source, None)
    } else {
        let owner = VaultRootOwner::acquire(target)?;
        let guard = Arc::new(begin_owned_root_maintenance(Arc::clone(&owner))?);
        ensure_import_root_movable(owner.root())?;
        (owner, Some(guard))
    };
    Ok(Arc::new(RootTransition {
        target_owner,
        _source: source_guard,
        _target: target_guard,
    }))
}

#[cfg(test)]
mod rf905_tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc,
    };
    use std::time::Duration;

    struct ReleaseOnDrop(Option<mpsc::Sender<()>>);
    impl Drop for ReleaseOnDrop {
        fn drop(&mut self) {
            if let Some(tx) = self.0.take() {
                let _ = tx.send(());
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rf905_host_cancelled_waiter_keeps_real_worker_pin_and_maintenance_admission() {
        let root = tempfile::tempdir().unwrap();
        let owner = VaultRootOwner::acquire(root.path()).unwrap();
        let weak = Arc::downgrade(&owner);
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let mut release = ReleaseOnDrop(Some(release_tx));
        let output = root.path().join("worker-publication");
        let output_copy = output.clone();
        let (finished_tx, finished_rx) = tokio::sync::oneshot::channel();
        let worker = spawn_owned_blocking(Arc::clone(&owner), move || {
            let _ = entered_tx.send(());
            release_rx.recv_timeout(Duration::from_secs(10)).unwrap();
            std::fs::write(output_copy, b"published after waiter cancellation").unwrap();
            let _ = finished_tx.send(());
        })
        .unwrap();
        let waiter = tokio::spawn(async move {
            worker.await.unwrap();
        });
        tokio::time::timeout(Duration::from_secs(10), entered_rx)
            .await
            .unwrap()
            .unwrap();
        waiter.abort();
        assert!(waiter.await.unwrap_err().is_cancelled());
        drop(owner);
        assert!(weak.upgrade().is_some());
        assert_eq!(
            solosoul_core::import_activity::begin_import_maintenance(root.path())
                .err()
                .unwrap(),
            "IMPORT_OPERATIONS_ACTIVE"
        );
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        assert_eq!(
            VaultRootOwner::acquire(root.path()).err().unwrap(),
            "VAULT_DIRECTORY_BUSY"
        );
        release.0.take().unwrap().send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(10), finished_rx)
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(Duration::from_secs(10), async {
            while weak.upgrade().is_some() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            std::fs::read(output).unwrap(),
            b"published after waiter cancellation"
        );
        assert!(VaultRootOwner::acquire(root.path()).is_ok());
        assert!(solosoul_core::import_activity::begin_import_maintenance(root.path()).is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rf905_host_maintenance_rejects_before_dispatch_and_source_target_failures_preserve_service(
    ) {
        let root = tempfile::tempdir().unwrap();
        let service = crate::state::AppState::try_init_local_vault(root.path()).unwrap();
        let owner = service.root_owner();
        let counter = Arc::new(AtomicUsize::new(0));
        let seen = Arc::clone(&counter);
        let maintenance = begin_owned_root_maintenance(Arc::clone(&owner)).unwrap();
        let failure = spawn_owned_blocking(Arc::clone(&owner), move || {
            seen.fetch_add(1, Ordering::SeqCst)
        });
        assert_eq!(failure.err().unwrap(), "IMPORT_DIRECTORY_BUSY");
        assert_eq!(counter.load(Ordering::SeqCst), 0);
        drop(maintenance);
        let target = tempfile::tempdir().unwrap();
        let target_owner = VaultRootOwner::acquire(target.path()).unwrap();
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            assert_eq!(
                prepare_root_transition(&service, target.path())
                    .err()
                    .unwrap(),
                "VAULT_DIRECTORY_BUSY"
            );
            assert!(Arc::ptr_eq(&service.root_owner(), &owner));
            assert_eq!(service.base_path(), &root.path().canonicalize().unwrap());
            assert!(!target.path().join("accounts.json").exists());
            assert!(begin_owned_root_activity(Arc::clone(&owner)).is_ok());
        }
        drop(target_owner);
        let same = prepare_root_transition(&service, &root.path().join(".")).unwrap();
        assert!(Arc::ptr_eq(&same.target_owner(), &owner));
        assert_eq!(
            begin_owned_root_activity(Arc::clone(&owner))
                .err()
                .map(|e| e.to_string())
                .unwrap(),
            "IMPORT_DIRECTORY_BUSY"
        );
        drop(same);
        let transition = prepare_root_transition(&service, target.path()).unwrap();
        assert_eq!(
            transition.target_owner().root(),
            target.path().canonicalize().unwrap()
        );
        assert_eq!(
            begin_owned_root_activity(transition.target_owner())
                .err()
                .unwrap(),
            "IMPORT_DIRECTORY_BUSY"
        );
        drop(transition);
        assert!(begin_owned_root_activity(owner).is_ok());
    }
}
