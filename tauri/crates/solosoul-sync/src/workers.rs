//! RF905：同步任务在派发前登记，取消等待不提前释放真实 worker 的目录许可。

use crate::session::SessionGuard;
use solosoul_core::import_activity::{begin_owned_root_activity, RootActivityGuard};
use solosoul_vault::VaultStore;
use std::collections::HashMap;
use std::net::{Shutdown, TcpStream};
use std::sync::{Arc, Mutex};
use tokio::sync::{oneshot, Mutex as AsyncMutex, Notify};
use tokio::task::{spawn_blocking, JoinHandle};

struct WorkerState {
    accepting: bool,
    cancel_io: bool,
    next_id: u64,
    sockets: HashMap<u64, TcpStream>,
    handles: Vec<JoinHandle<()>>,
    stopping: Option<Arc<StopCompletion>>,
}

pub(crate) struct SyncWorkers {
    state: Mutex<WorkerState>,
    joining: AsyncMutex<()>,
}

impl SyncWorkers {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(WorkerState {
                accepting: true,
                cancel_io: false,
                next_id: 0,
                sockets: HashMap::new(),
                handles: Vec::new(),
                stopping: None,
            }),
            joining: AsyncMutex::new(()),
        })
    }

    /// 只允许在上一代全部 join 后重新打开；旧循环不会因新一代 running 再次复活。
    pub(crate) fn reopen(&self) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "__SYNC_ERR__:session_failed:worker_registry_poisoned")?;
        if !state.handles.is_empty()
            || !state.sockets.is_empty()
            || state
                .stopping
                .as_ref()
                .is_some_and(|completion| !completion.is_finished())
        {
            return Err("__SYNC_ERR__:session_failed:workers_not_joined".into());
        }
        state.accepting = true;
        state.cancel_io = false;
        state.stopping = None;
        Ok(())
    }

    /// 单次独立收尾；取消 Manager 的 stop awaiter 也不取消网络停止与 join。
    pub(crate) fn begin_stop(
        self: &Arc<Self>,
        counter: Arc<std::sync::atomic::AtomicUsize>,
        grace: std::time::Duration,
    ) -> Arc<StopCompletion> {
        let completion = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(completion) = &state.stopping {
                return completion.clone();
            }
            state.accepting = false;
            let completion = StopCompletion::new();
            state.stopping = Some(completion.clone());
            completion
        };
        let workers = self.clone();
        let completing = completion.clone();
        std::mem::drop(tokio::spawn(async move {
            let deadline = std::time::Instant::now() + grace;
            while counter.load(std::sync::atomic::Ordering::SeqCst) != 0
                && std::time::Instant::now() < deadline
            {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            workers.interrupt_network();
            completing.finish(workers.wait().await);
        }));
        completion
    }

    /// 新 start 必须等待上一收尾的 completion，不能抢先 join 后重新开放旧 group。
    pub(crate) async fn wait_stopping(&self) -> Result<(), String> {
        let completion = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .stopping
            .clone();
        match completion {
            Some(completion) => completion.wait().await,
            None => Ok(()),
        }
    }

    pub(crate) fn close(&self) {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .accepting = false;
    }

    /// 监听/发现只保活原 Store/owner；空闲监听不阻止维护。
    pub(crate) fn spawn_background<F>(
        self: &Arc<Self>,
        vault: Arc<VaultStore>,
        action: F,
    ) -> Result<(), String>
    where
        F: FnOnce() + Send + 'static,
    {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "__SYNC_ERR__:session_failed:worker_registry_poisoned")?;
        if !state.accepting {
            return Err("__SYNC_ERR__:not_running".into());
        }
        let permit = WorkerPermit {
            workers: self.clone(),
            id: None,
            _vault: vault,
            _session: None,
            _activity: None,
        };
        state.handles.push(spawn_blocking(move || {
            let _permit = permit;
            action();
        }));
        Ok(())
    }

    /// 准入、目录 Activity、会话计数及句柄登记都在派发前完成。
    /// Activity 使用 owner.root() 的同一 RF022 短 gate，绝不从账户路径推算根。
    pub(crate) fn spawn_session<R, F>(
        self: &Arc<Self>,
        vault: Arc<VaultStore>,
        counter: Arc<std::sync::atomic::AtomicUsize>,
        action: F,
    ) -> Result<oneshot::Receiver<R>, String>
    where
        R: Send + 'static,
        F: FnOnce(&WorkerPermit) -> R + Send + 'static,
    {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "__SYNC_ERR__:session_failed:worker_registry_poisoned")?;
        if !state.accepting {
            return Err("__SYNC_ERR__:not_running".into());
        }
        let activity = begin_owned_root_activity(vault.root_owner())?;
        let id = state
            .next_id
            .checked_add(1)
            .ok_or("__SYNC_ERR__:session_failed:worker_registry_exhausted")?;
        state.next_id = id;
        let permit = WorkerPermit {
            workers: self.clone(),
            id: Some(id),
            _vault: vault,
            _session: Some(SessionGuard::new(counter)),
            _activity: Some(activity),
        };
        let (sender, receiver) = oneshot::channel();
        state.handles.push(spawn_blocking(move || {
            let result = action(&permit);
            let _ = sender.send(result);
            // permit 在真实 closure 末尾 Drop；receiver 被取消不影响此处。
        }));
        Ok(receiver)
    }

    /// 宽限结束仅打断网络 IO，不 abort 阻塞中的数据库/附件发布。
    pub(crate) fn interrupt_network(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.cancel_io = true;
        for stream in state.sockets.values() {
            let _ = stream.shutdown(Shutdown::Both);
        }
    }

    pub(crate) async fn wait(self: &Arc<Self>) -> Result<(), String> {
        let _joining = self.joining.lock().await;
        let handles = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            std::mem::take(&mut state.handles)
        };
        let mut batch = JoinBatch {
            workers: self.clone(),
            handles,
        };
        let mut first_error = None;
        while let Some(handle) = batch.handles.last_mut() {
            if let Err(error) = handle.await {
                if first_error.is_none() {
                    first_error = Some(format!("__SYNC_ERR__:session_failed:{}", error));
                }
            }
            // 已完成的句柄立刻移除，取消后不能二次 poll 一个已消费的 JoinHandle。
            batch.handles.pop();
        }
        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

/// 正常/取消/恐慌都先清理 socket 与 Store pin，Activity 最后释放。
pub(crate) struct WorkerPermit {
    workers: Arc<SyncWorkers>,
    id: Option<u64>,
    _vault: Arc<VaultStore>,
    _session: Option<SessionGuard>,
    _activity: Option<RootActivityGuard>,
}

impl WorkerPermit {
    pub(crate) fn track_stream(&self, stream: &TcpStream) -> Result<(), String> {
        let mut state = self
            .workers
            .state
            .lock()
            .map_err(|_| "__SYNC_ERR__:session_failed:worker_registry_poisoned")?;
        if state.cancel_io {
            let _ = stream.shutdown(Shutdown::Both);
            return Err("__SYNC_ERR__:not_running".into());
        }
        let copy = stream
            .try_clone()
            .map_err(|e| format!("__SYNC_ERR__:session_failed:{}", e))?;
        state.sockets.insert(
            self.id
                .ok_or("__SYNC_ERR__:session_failed:missing_session")?,
            copy,
        );
        Ok(())
    }
}

impl Drop for WorkerPermit {
    fn drop(&mut self) {
        if let Some(id) = self.id {
            self.workers
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .sockets
                .remove(&id);
        }
    }
}

/// wait future 被取消时，把尚未完成/消费的句柄归还，后续等待仍覆盖原 worker。
struct JoinBatch {
    workers: Arc<SyncWorkers>,
    handles: Vec<JoinHandle<()>>,
}
impl Drop for JoinBatch {
    fn drop(&mut self) {
        self.workers
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .handles
            .append(&mut self.handles);
    }
}

/// Service 的独立收尾结果；调用方取消只撤销等待，不取消持 manager 的收尾任务。
pub(crate) struct StopCompletion {
    result: Mutex<Option<Result<(), String>>>,
    notify: Notify,
}
impl StopCompletion {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self {
            result: Mutex::new(None),
            notify: Notify::new(),
        })
    }
    fn is_finished(&self) -> bool {
        self.result
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_some()
    }
    pub(crate) fn finish(&self, result: Result<(), String>) {
        *self
            .result
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(result);
        self.notify.notify_waiters();
    }
    pub(crate) async fn wait(&self) -> Result<(), String> {
        loop {
            let notified = self.notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if let Some(result) = self
                .result
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone()
            {
                return result;
            }
            notified.await;
        }
    }
}

#[cfg(test)]
#[path = "workers_tests.rs"]
mod tests;
