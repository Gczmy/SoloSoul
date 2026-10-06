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
            Self::KdfUpgrade => "kdf-upgrade",
            Self::VaultOpen => "vault-open",
            Self::SessionPublish => "session-publish",
            Self::PinResetCall => "pin-reset-call",
        }
    }
}
pub trait UnlockObserver: Send + Sync {
    fn begin(&self, stage: UnlockStage) -> Option<u32>;
    fn end(&self, token: u32, completed: bool);
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
        let scope = observe(recorder.clone());
        svc.unlock("acc_nativeperf", "public-password").unwrap();
        drop(scope);
        assert!(svc.get_vault_store().is_some());
        let rows = recorder.0.lock().unwrap().clone();
        for name in [
            "recovery",
            "precheck",
            "master-config",
            "kdf",
            "verify",
            "vault-open",
            "session-publish",
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
}
