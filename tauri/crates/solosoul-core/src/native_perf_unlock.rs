//! 非默认原生性能功能的线程局部解锁观察器；接口只接受固定阶段，不接受业务数据。
use std::cell::RefCell;
use std::marker::PhantomData;
use std::sync::Arc;

#[derive(Clone, Copy)]
pub enum UnlockStage {
    Recovery,
    Precheck,
    MasterConfig,
    Kdf,
    Verify,
    AccountWriteCall,
    AccountManifestSerialize,
    AccountDirectoryCreate,
    AccountDirectoryPermission,
    AccountManifestAtomicWrite,
    AccountManifestPermission,
    KdfUpgrade,
    VaultOpen,
    SessionPublish,
    PinResetCall,
}
impl UnlockStage {
    pub fn name(self) -> &'static str {
        match self {
            Self::Recovery => "recovery",
            Self::Precheck => "precheck",
            Self::MasterConfig => "master-config",
            Self::Kdf => "kdf",
            Self::Verify => "verify",
            Self::AccountWriteCall => "account-write-call",
            Self::AccountManifestSerialize => "account-manifest-serialize",
            Self::AccountDirectoryCreate => "account-directory-create",
            Self::AccountDirectoryPermission => "account-directory-permission",
            Self::AccountManifestAtomicWrite => "account-manifest-atomic-write",
            Self::AccountManifestPermission => "account-manifest-permission",
            Self::KdfUpgrade => "kdf-upgrade",
            Self::VaultOpen => "vault-open",
            Self::SessionPublish => "session-publish",
            Self::PinResetCall => "pin-reset-call",
        }
    }
}
/// 仅非默认测量功能使用；类型中不承载路径、用户名或错误正文。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PermissionTarget {
    Directory,
    File,
}
impl PermissionTarget {
    pub fn stage(self) -> UnlockStage {
        match self {
            Self::Directory => UnlockStage::AccountDirectoryPermission,
            Self::File => UnlockStage::AccountManifestPermission,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Directory => "directory",
            Self::File => "file",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PermissionOutcome {
    UsernameUnavailable,
    InvalidUsername,
    SpawnError { os_error: Option<i32> },
    Exited { code: Option<i32>, success: bool },
}
pub trait UnlockObserver: Send + Sync {
    fn begin(&self, stage: UnlockStage) -> Option<u32>;
    fn end(&self, token: u32, completed: bool);
    fn permission(&self, _target: PermissionTarget, _outcome: PermissionOutcome) {}
}
/// 只通知当前 blocking worker 的观察器；默认功能没有此模块。
pub fn permission(target: PermissionTarget, outcome: PermissionOutcome) {
    let observer = OBSERVER.with(|slot| slot.borrow().as_ref().cloned());
    if let Some(observer) = observer {
        observer.permission(target, outcome);
    }
}
thread_local! {
    static OBSERVER: RefCell<Option<Arc<dyn UnlockObserver>>> = const { RefCell::new(None) };
}
/// 不可跨线程移动；退出（包括panic）时恢复原观察器，避免blocking池复用泄漏。
pub struct ObserverScope {
    previous: Option<Arc<dyn UnlockObserver>>,
    _thread: PhantomData<*mut ()>,
}
pub fn observe(observer: Arc<dyn UnlockObserver>) -> ObserverScope {
    let previous = OBSERVER.with(|slot| slot.replace(Some(observer)));
    ObserverScope {
        previous,
        _thread: PhantomData,
    }
}
impl Drop for ObserverScope {
    fn drop(&mut self) {
        OBSERVER.with(|slot| {
            slot.replace(self.previous.take());
        });
    }
}
pub struct StageScope {
    observer: Arc<dyn UnlockObserver>,
    token: u32,
    completed: bool,
}
pub fn stage(stage: UnlockStage) -> Option<StageScope> {
    let observer = OBSERVER.with(|slot| slot.borrow().as_ref().cloned())?;
    let token = observer.begin(stage)?;
    Some(StageScope {
        observer,
        token,
        completed: false,
    })
}
pub fn complete(scope: Option<StageScope>) {
    if let Some(mut scope) = scope {
        scope.completed = true;
    }
}
impl Drop for StageScope {
    fn drop(&mut self) {
        self.observer.end(self.token, self.completed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    #[derive(Default)]
    struct Recorder(Mutex<Vec<(&'static str, bool)>>);
    impl UnlockObserver for Recorder {
        fn begin(&self, stage: UnlockStage) -> Option<u32> {
            let mut rows = self.0.lock().unwrap();
            let token = rows.len() as u32;
            rows.push((stage.name(), false));
            Some(token)
        }
        fn end(&self, token: u32, completed: bool) {
            self.0.lock().unwrap()[token as usize].1 = completed;
        }
    }
    #[test]
    fn observer_scope_is_thread_local_and_restores_after_panic() {
        assert!(stage(UnlockStage::Kdf).is_none());
        let first = Arc::new(Recorder::default());
        let second = Arc::new(Recorder::default());
        let outer = observe(first.clone());
        complete(stage(UnlockStage::Kdf));
        std::thread::spawn(|| assert!(stage(UnlockStage::Kdf).is_none()))
            .join()
            .unwrap();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _nested = observe(second.clone());
            let _stage = stage(UnlockStage::VaultOpen);
            panic!("synthetic panic");
        }));
        assert!(result.is_err());
        complete(stage(UnlockStage::SessionPublish));
        drop(outer);
        assert!(stage(UnlockStage::Kdf).is_none());
        assert_eq!(
            *first.0.lock().unwrap(),
            [("kdf", true), ("session-publish", true)]
        );
        assert_eq!(*second.0.lock().unwrap(), [("vault-open", false)]);
    }
    #[test]
    fn real_unlock_records_kdf_and_storage_without_changing_session_result() {
        use crate::VaultService;
        let dir = tempfile::tempdir().unwrap();
        let svc = VaultService::with_base_path(dir.path().join("vault"));
        svc.create_account_with_id("acc_nativeperf", "Fixture", "public-password", None)
            .unwrap();
        svc.lock();
        let recorder = Arc::new(Recorder::default());
        let before: serde_json::Value =
            serde_json::from_slice(&std::fs::read(svc.base_path().join("accounts.json")).unwrap())
                .unwrap();
        let scope = observe(recorder.clone());
        svc.unlock("acc_nativeperf", "public-password").unwrap();
        drop(scope);
        assert!(svc.get_vault_store().is_some());
        let persisted: serde_json::Value =
            serde_json::from_slice(&std::fs::read(svc.base_path().join("accounts.json")).unwrap())
                .unwrap();
        assert_eq!(persisted[0]["id"], "acc_nativeperf");
        assert_ne!(persisted[0]["last_accessed"], before[0]["last_accessed"]);
        let rows = recorder.0.lock().unwrap().clone();
        for name in [
            "recovery",
            "precheck",
            "master-config",
            "kdf",
            "verify",
            "vault-open",
            "session-publish",
            "account-manifest-serialize",
            "account-directory-create",
            "account-directory-permission",
            "account-manifest-atomic-write",
            "account-manifest-permission",
        ] {
            assert_eq!(
                rows.iter().filter(|row| row.0 == name && row.1).count(),
                1,
                "{name}"
            );
        }
        svc.lock();
        svc.unlock("acc_nativeperf", "public-password").unwrap();
        assert_eq!(*recorder.0.lock().unwrap(), rows);
    }
    #[derive(Default)]
    struct PermissionRecorder(Mutex<Vec<(PermissionTarget, PermissionOutcome)>>);
    impl UnlockObserver for PermissionRecorder {
        fn begin(&self, _stage: UnlockStage) -> Option<u32> {
            None
        }
        fn end(&self, _token: u32, _completed: bool) {}
        fn permission(&self, target: PermissionTarget, outcome: PermissionOutcome) {
            self.0.lock().unwrap().push((target, outcome));
        }
    }
    #[test]
    fn typed_permission_callbacks_remain_thread_local_and_restore_after_panic() {
        let outer = Arc::new(PermissionRecorder::default());
        let nested = Arc::new(PermissionRecorder::default());
        let guard = observe(outer.clone());
        permission(
            PermissionTarget::Directory,
            PermissionOutcome::Exited {
                code: Some(0),
                success: true,
            },
        );
        std::thread::spawn(|| {
            permission(PermissionTarget::File, PermissionOutcome::InvalidUsername)
        })
        .join()
        .unwrap();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _scope = observe(nested.clone());
            permission(
                PermissionTarget::File,
                PermissionOutcome::UsernameUnavailable,
            );
            panic!("synthetic nested scope");
        }));
        assert!(result.is_err());
        permission(
            PermissionTarget::File,
            PermissionOutcome::SpawnError { os_error: Some(5) },
        );
        drop(guard);
        permission(PermissionTarget::File, PermissionOutcome::InvalidUsername);
        assert_eq!(
            *outer.0.lock().unwrap(),
            [
                (
                    PermissionTarget::Directory,
                    PermissionOutcome::Exited {
                        code: Some(0),
                        success: true
                    }
                ),
                (
                    PermissionTarget::File,
                    PermissionOutcome::SpawnError { os_error: Some(5) }
                )
            ]
        );
        assert_eq!(
            *nested.0.lock().unwrap(),
            [(
                PermissionTarget::File,
                PermissionOutcome::UsernameUnavailable
            )]
        );
    }
}
