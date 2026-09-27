//! RF-029：只用于 OCR 的有界调度。锁顺序：Vault service/session → job；
//! registry 只用于登记/取 Arc/回收，绝不跨 await、推理或事件回调。
use futures::FutureExt;
use serde::Serialize;
use solosoul_core::ocr::control::{OcrCancellation, OCR_CANCELLED};
use solosoul_core::vault_service::VaultSession;
use solosoul_core::VaultService;
use solosoul_vault::VaultStore;
use std::collections::{HashMap, HashSet, VecDeque};
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tokio::sync::{oneshot, Notify, Semaphore};

pub const OCR_QUEUE_FULL: &str = "__OCR_QUEUE_FULL__";
pub const OCR_DUPLICATE_TASK: &str = "__OCR_DUPLICATE_TASK__";
pub const OCR_SESSION_STALE: &str = "__OCR_SESSION_STALE__";
const OCR_INVALID_TASK_ID: &str = "__OCR_INVALID_TASK_ID__";
const MAX_PENDING: usize = 5;
const RECENT_IDS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OcrJobState {
    Queued,
    Running,
    CancelRequested,
    Completed,
    Cancelled,
    Failed,
    Stale,
}

impl OcrJobState {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Cancelled | Self::Failed | Self::Stale
        )
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrJobEvent {
    pub task_id: String,
    pub account_id: String,
    pub session_generation: u64,
    pub state: OcrJobState,
    pub sequence: u64,
}

pub type EventSink = Arc<dyn Fn(OcrJobEvent) + Send + Sync>;

struct Progress {
    state: OcrJobState,
    stop: Option<OcrJobState>,
    sequence: u64,
}

struct Job {
    id: String,
    service: Arc<RwLock<VaultService>>,
    session: VaultSession,
    cancellation: OcrCancellation,
    progress: Mutex<Progress>,
    changed: Notify,
    emit: EventSink,
}

impl Job {
    fn event(&self, progress: &Progress) -> OcrJobEvent {
        OcrJobEvent {
            task_id: self.id.clone(),
            account_id: self.session.account_id().to_owned(),
            session_generation: self.session.generation(),
            state: progress.state,
            sequence: progress.sequence,
        }
    }

    fn emit(&self, event: OcrJobEvent) {
        // 事件只带任务身份和状态。发射失败/订阅者异常不能遗留占用的执行位。
        if std::panic::catch_unwind(AssertUnwindSafe(|| (self.emit)(event))).is_err() {
            tracing::warn!("OCR state event callback panicked");
        }
    }

    fn stop_locked(&self, progress: &mut Progress, reason: OcrJobState) -> Option<OcrJobEvent> {
        if progress.state.is_terminal() || progress.stop.is_some() {
            return None;
        }
        progress.stop = Some(reason);
        progress.state = OcrJobState::CancelRequested;
        progress.sequence += 1;
        self.cancellation.cancel();
        self.changed.notify_one();
        Some(self.event(progress))
    }

    fn stop(&self, reason: OcrJobState) {
        let event = {
            let mut progress = self.progress.lock().unwrap_or_else(|e| e.into_inner());
            self.stop_locked(&mut progress, reason)
        };
        if let Some(event) = event {
            self.emit(event);
        }
    }
}

/// 仅在短临界区内执行 action；业务 Result 作为普通返回值，避免与会话失败混淆。
fn in_session<T>(
    service: &Arc<RwLock<VaultService>>,
    session: &VaultSession,
    action: impl FnOnce(&VaultStore) -> T,
) -> Result<T, String> {
    let service = service.read().map_err(|_| OCR_SESSION_STALE.to_string())?;
    service
        .with_session(session, |vault| Ok(action(vault)))
        .map_err(|_| OCR_SESSION_STALE.to_string())
}

pub fn capture_ocr_session(service: &Arc<RwLock<VaultService>>) -> Result<VaultSession, String> {
    let service = service.read().map_err(|_| OCR_SESSION_STALE.to_string())?;
    let account = service
        .get_current_account()
        .ok_or_else(|| "No account is currently unlocked".to_string())?;
    service
        .capture_session(&account)
        .map_err(|_| OCR_SESSION_STALE.to_string())
}

#[derive(Clone)]
pub struct OcrJobContext {
    job: Arc<Job>,
}

impl OcrJobContext {
    pub fn cancellation(&self) -> OcrCancellation {
        self.job.cancellation.clone()
    }

    /// 引擎准备前后使用；不能在持有 PDFium guard 时调用此会话检查。
    pub fn checkpoint(&self) -> Result<(), String> {
        if in_session(&self.job.service, &self.job.session, |_| ()).is_err() {
            self.job.stop(OcrJobState::Stale);
            return Err(OCR_SESSION_STALE.to_string());
        }
        self.job.cancellation.check()
    }
}

#[derive(Default)]
struct Registry {
    pending: HashMap<String, Arc<Job>>,
    recent: HashSet<String>,
    recent_order: VecDeque<String>,
}

pub struct OcrJobs {
    registry: Mutex<Registry>,
    execution: Arc<Semaphore>,
}

impl Default for OcrJobs {
    fn default() -> Self {
        Self::new()
    }
}

impl OcrJobs {
    pub fn new() -> Self {
        Self {
            registry: Mutex::new(Registry::default()),
            execution: Arc::new(Semaphore::new(1)),
        }
    }

    pub async fn run<T, F, Fut, P>(
        self: &Arc<Self>,
        task_id: Option<String>,
        service: Arc<RwLock<VaultService>>,
        session: VaultSession,
        emit: EventSink,
        work: F,
        publish: P,
    ) -> Result<T, String>
    where
        T: Send + 'static,
        F: FnOnce(OcrJobContext) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, String>> + Send + 'static,
        P: FnOnce(&VaultStore, &T) -> Result<(), String> + Send + 'static,
    {
        let id = match task_id {
            Some(id) => uuid::Uuid::parse_str(&id)
                .map_err(|_| OCR_INVALID_TASK_ID.to_string())?
                .to_string(),
            None => uuid::Uuid::new_v4().to_string(),
        };
        let job = Arc::new(Job {
            id: id.clone(),
            service,
            session,
            emit,
            cancellation: OcrCancellation::default(),
            progress: Mutex::new(Progress {
                state: OcrJobState::Queued,
                stop: None,
                sequence: 1,
            }),
            changed: Notify::new(),
        });
        let queued = OcrJobEvent {
            task_id: job.id.clone(),
            account_id: job.session.account_id().to_owned(),
            session_generation: job.session.generation(),
            state: OcrJobState::Queued,
            sequence: 1,
        };
        // 准入在同一 registry 临界区内立即决定，Semaphore 前没有无限等待者。
        in_session(&job.service, &job.session, |_| {
            let mut registry = self.registry.lock().unwrap_or_else(|e| e.into_inner());
            if registry.pending.contains_key(&id) || registry.recent.contains(&id) {
                return Err(OCR_DUPLICATE_TASK.to_string());
            }
            if registry.pending.len() >= MAX_PENDING {
                return Err(OCR_QUEUE_FULL.to_string());
            }
            registry.pending.insert(id, job.clone());
            Ok(())
        })??;
        job.emit(queued);
        let (reply, result) = oneshot::channel();
        let manager = self.clone();
        // 调用方停止等待不取消本任务所有权；运行位由实际 worker 完成后释放。
        tokio::spawn(async move {
            let context = OcrJobContext { job };
            let output = manager.execute(&context, work).await;
            let output = manager.finish(&context, output, publish);
            let _ = reply.send(output);
        });
        result
            .await
            .map_err(|_| "OCR coordinator stopped unexpectedly".to_string())?
    }

    /// 只有同一原会话能够取消其任务；未知/已结束返回 false，不表示正在执行的任务结束。
    pub fn cancel(
        &self,
        task_id: &str,
        service: &Arc<RwLock<VaultService>>,
        caller: &VaultSession,
    ) -> Result<bool, String> {
        let task_id = uuid::Uuid::parse_str(task_id)
            .map_err(|_| OCR_INVALID_TASK_ID.to_string())?
            .to_string();
        let job = {
            self.registry
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .pending
                .get(&task_id)
                .cloned()
        };
        let outcome = in_session(service, caller, |_| {
            let Some(job) = job else {
                return Ok((false, None));
            };
            if caller.account_id() != job.session.account_id()
                || caller.generation() != job.session.generation()
                || !std::ptr::eq(caller.vault(), job.session.vault())
            {
                return Err(OCR_SESSION_STALE.to_string());
            }
            let mut progress = job.progress.lock().unwrap_or_else(|e| e.into_inner());
            if progress.state.is_terminal() {
                return Ok((false, None));
            }
            let event = job.stop_locked(&mut progress, OcrJobState::Cancelled);
            drop(progress);
            Ok((true, event.map(|event| (job, event))))
        })??;
        if let Some((job, event)) = outcome.1 {
            job.emit(event);
        }
        Ok(outcome.0)
    }

    async fn execute<T, F, Fut>(&self, context: &OcrJobContext, work: F) -> Result<T, String>
    where
        F: FnOnce(OcrJobContext) -> Fut,
        Fut: Future<Output = Result<T, String>>,
    {
        context.checkpoint()?;
        let acquire = self.execution.clone().acquire_owned();
        tokio::pin!(acquire);
        let mut monitor = tokio::time::interval(Duration::from_millis(50));
        monitor.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let permit = loop {
            tokio::select! {
                permit = &mut acquire => break permit.map_err(|_| "OCR executor closed".to_string())?,
                _ = context.job.changed.notified() => { context.checkpoint()?; }
                _ = monitor.tick() => { context.checkpoint()?; }
            }
        };
        context.checkpoint()?;
        let running = {
            let mut progress = context
                .job
                .progress
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if progress.stop.is_some() {
                return Err(OCR_CANCELLED.to_string());
            }
            progress.state = OcrJobState::Running;
            progress.sequence += 1;
            context.job.event(&progress)
        };
        context.job.emit(running);
        let worker_context = context.clone();
        let operation = AssertUnwindSafe(async move { work(worker_context).await }).catch_unwind();
        tokio::pin!(operation);
        let output = loop {
            tokio::select! {
                result = &mut operation => break result.unwrap_or_else(|_| Err("OCR worker panicked".to_string())),
                _ = context.job.changed.notified() => { let _ = context.checkpoint(); }
                _ = monitor.tick() => { let _ = context.checkpoint(); }
            }
        };
        // operation 返回才证明 native/ONNX/ML Kit 工作及 RF028 owner 已退出。
        drop(permit);
        output
    }

    fn finish<T, P>(
        &self,
        context: &OcrJobContext,
        output: Result<T, String>,
        publish: P,
    ) -> Result<T, String>
    where
        P: FnOnce(&VaultStore, &T) -> Result<(), String>,
    {
        let job = &context.job;
        let completed = in_session(&job.service, &job.session, |vault| {
            let mut progress = job.progress.lock().unwrap_or_else(|e| e.into_inner());
            let result = match progress.stop {
                Some(OcrJobState::Stale) => Err(OCR_SESSION_STALE.to_string()),
                Some(_) => Err(OCR_CANCELLED.to_string()),
                None => match output {
                    Ok(value) => {
                        std::panic::catch_unwind(AssertUnwindSafe(|| publish(vault, &value)))
                            .unwrap_or_else(|_| Err("OCR result publication panicked".to_string()))
                            .map(|()| value)
                    }
                    Err(error) => Err(error),
                },
            };
            progress.state = match (&result, progress.stop) {
                (_, Some(OcrJobState::Stale)) => OcrJobState::Stale,
                (_, Some(_)) => OcrJobState::Cancelled,
                (Ok(_), _) => OcrJobState::Completed,
                (Err(error), _) if error == OCR_CANCELLED => OcrJobState::Cancelled,
                (Err(error), _) if error == OCR_SESSION_STALE => OcrJobState::Stale,
                (Err(_), _) => OcrJobState::Failed,
            };
            progress.sequence += 1;
            (result, job.event(&progress))
        });
        // with_session 可能在进入闭包前拒绝；仍须发布一次无正文 stale 终态并回收。
        let (result, event) = completed.unwrap_or_else(|_| {
            let mut progress = job.progress.lock().unwrap_or_else(|e| e.into_inner());
            job.cancellation.cancel();
            progress.state = OcrJobState::Stale;
            progress.sequence += 1;
            (Err(OCR_SESSION_STALE.to_string()), job.event(&progress))
        });
        {
            let mut registry = self.registry.lock().unwrap_or_else(|e| e.into_inner());
            registry.pending.remove(&job.id);
            registry.recent.insert(job.id.clone());
            registry.recent_order.push_back(job.id.clone());
            while registry.recent_order.len() > RECENT_IDS {
                if let Some(old) = registry.recent_order.pop_front() {
                    registry.recent.remove(&old);
                }
            }
        }
        job.emit(event);
        result
    }
}

#[cfg(test)]
mod rf029_tests;
