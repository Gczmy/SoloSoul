//! CLI 自有异步任务及原会话内的结果接纳。
//!
//! 工作 Future 只能返回数据或报告进度，不能持有 App 的可变引用。这里仅管理
//! 直接注册的异步 Future：禁止在其中分离子任务或隐藏 spawn_blocking/thread::spawn。
//! 后续阻塞任务必须有独立的受管入口；abort 不能终止已经运行的阻塞闭包。
//! 正常退出须显式 shutdown 并等待所有任务回收，Drop 仅负责请求取消。

use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use solosoul_core::{VaultService, VaultSession};
use tokio::sync::mpsc;
use tokio::task::{AbortHandle, Id, JoinError, JoinSet};
use uuid::Uuid;

const PROGRESS_CAPACITY: usize = 64;
const TASK_PANICKED: &str = "后台任务执行失败";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskIdentity {
    pub task_id: TaskId,
    pub account_id: String,
    pub session_generation: u64,
}

/// 首个调用者仅需消息结果；业务迁移时增加具体变体，不传回修改 App 的闭包。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskOutput {
    Message(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskFailure {
    Cancelled,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskEventKind {
    Progress { current: u64, total: Option<u64> },
    Completed(TaskOutput),
    Failed(String),
    Cancelled,
}

impl TaskEventKind {
    fn is_terminal(&self) -> bool {
        !matches!(self, Self::Progress { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskEvent {
    pub identity: TaskIdentity,
    pub kind: TaskEventKind,
}

/// 工作任务只持有原会话、取消标记和有界进度发送端，不拥有 UI。
pub struct TaskContext {
    identity: TaskIdentity,
    session: VaultSession,
    cancel_requested: Arc<AtomicBool>,
    progress: mpsc::Sender<TaskEvent>,
}

impl TaskContext {
    pub fn identity(&self) -> &TaskIdentity {
        &self.identity
    }

    /// 业务写入仍须使用原 VaultService::with_session，不得重新获取当前 Vault。
    pub fn session(&self) -> &VaultSession {
        &self.session
    }

    pub fn is_cancel_requested(&self) -> bool {
        self.cancel_requested.load(Ordering::Acquire)
    }

    /// 中间进度允许丢弃；终态由 JoinSet 的真实完成结果单独产生。
    pub fn report_progress(&self, current: u64, total: Option<u64>) -> bool {
        if self.is_cancel_requested() {
            return false;
        }
        self.progress
            .try_send(TaskEvent {
                identity: self.identity.clone(),
                kind: TaskEventKind::Progress { current, total },
            })
            .is_ok()
    }
}

struct TaskRecord {
    identity: TaskIdentity,
    session: VaultSession,
    cancel_requested: Arc<AtomicBool>,
    abort: AbortHandle,
    // Some 表示已真实 join，但事件尚待主循环接纳；禁止再发布进度。
    terminal: Option<TaskEventKind>,
    stale: bool,
}

/// 仅统计本次 shutdown 实际 join 的任务；未接纳的 UI 事件直接清除。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ShutdownReport {
    pub joined: usize,
    pub cancelled: usize,
    pub failed: usize,
    pub panicked: usize,
}

pub struct Tasks {
    service: Arc<VaultService>,
    jobs: JoinSet<Result<TaskOutput, TaskFailure>>,
    runtime_ids: HashMap<Id, TaskId>,
    records: HashMap<TaskId, TaskRecord>,
    progress_tx: mpsc::Sender<TaskEvent>,
    progress_rx: mpsc::Receiver<TaskEvent>,
    shutting_down: bool,
}

impl Tasks {
    pub fn new(service: Arc<VaultService>) -> Self {
        let (progress_tx, progress_rx) = mpsc::channel(PROGRESS_CAPACITY);
        Self {
            service,
            jobs: JoinSet::new(),
            runtime_ids: HashMap::new(),
            records: HashMap::new(),
            progress_tx,
            progress_rx,
            shutting_down: false,
        }
    }

    /// 使用进程现有 runtime 派发；build 自身也在工作任务中调用。
    /// Future 必须让出执行权，并依靠析构完成取消清理，不得分离子工作。
    pub fn spawn<F, Fut>(&mut self, session: VaultSession, build: F) -> Result<TaskId, String>
    where
        F: FnOnce(TaskContext) -> Fut + Send + 'static,
        Fut: Future<Output = Result<TaskOutput, TaskFailure>> + Send + 'static,
    {
        if self.shutting_down {
            return Err("任务管理器正在退出".to_string());
        }
        self.service.with_session(&session, |_| Ok(()))?;
        let runtime = crate::util::shared_runtime().map_err(|error| error.to_string())?;
        let task_id = TaskId(Uuid::new_v4());
        let identity = TaskIdentity {
            task_id,
            account_id: session.account_id().to_string(),
            session_generation: session.generation(),
        };
        let cancel_requested = Arc::new(AtomicBool::new(false));
        let context = TaskContext {
            identity: identity.clone(),
            session: session.clone(),
            cancel_requested: Arc::clone(&cancel_requested),
            progress: self.progress_tx.clone(),
        };
        let service = Arc::clone(&self.service);
        let abort = self.jobs.spawn_on(
            async move {
                // 派发后、首次执行前也可能发生锁定或同账户重登。
                if context.is_cancel_requested()
                    || service.with_session(context.session(), |_| Ok(())).is_err()
                {
                    return Err(TaskFailure::Cancelled);
                }
                build(context).await
            },
            runtime.handle(),
        );
        self.runtime_ids.insert(abort.id(), task_id);
        self.records.insert(
            task_id,
            TaskRecord {
                identity,
                session,
                cancel_requested,
                abort,
                terminal: None,
                stale: false,
            },
        );
        Ok(task_id)
    }

    /// true 仅表示仍受管的任务已收到取消请求，不代表已结束。
    pub fn request_cancel(&mut self, task_id: TaskId) -> bool {
        let Some(record) = self.records.get(&task_id) else {
            return false;
        };
        if record.terminal.is_some() {
            return false;
        }
        Self::cancel_record(record);
        true
    }

    pub fn cancel_all(&mut self) {
        for record in self.records.values() {
            if record.terminal.is_none() {
                Self::cancel_record(record);
            }
        }
    }

    /// 返回首次失效的任务 ID，供主循环立即清除旧进度；记录仍保留到真实 join。
    /// 应在每轮 poll_events 前调用，覆盖锁定、换账户和同账户重登。
    pub fn cancel_stale(&mut self) -> Vec<TaskId> {
        let mut stale = Vec::new();
        for (task_id, record) in &mut self.records {
            if !record.stale
                && self
                    .service
                    .with_session(&record.session, |_| Ok(()))
                    .is_err()
            {
                record.stale = true;
                stale.push(*task_id);
                if record.terminal.is_none() {
                    Self::cancel_record(record);
                }
            }
        }
        stale
    }

    fn cancel_record(record: &TaskRecord) {
        record.cancel_requested.store(true, Ordering::Release);
        record.abort.abort();
    }

    /// 非阻塞、有数量上限。真实终态优先于进度，进度积压不能饿死终态。
    /// 返回的每个事件须交给 apply_event；不要直接把正文写入 UI。
    pub fn poll_events(&mut self, limit: usize) -> Vec<TaskEvent> {
        if limit == 0 {
            return Vec::new();
        }
        let mut events = Vec::new();
        for _ in 0..limit {
            let Some(result) = self.jobs.try_join_next_with_id() else {
                break;
            };
            let (runtime_id, kind) = match result {
                Ok((runtime_id, outcome)) => (runtime_id, Self::outcome_kind(outcome)),
                Err(error) => (error.id(), Self::join_error_kind(&error)),
            };
            let Some(task_id) = self.runtime_ids.remove(&runtime_id) else {
                continue;
            };
            let Some(record) = self.records.get_mut(&task_id) else {
                continue;
            };
            record.terminal = Some(kind.clone());
            events.push(TaskEvent {
                identity: record.identity.clone(),
                kind,
            });
        }
        // 被丢弃的旧进度也计入扫描预算，不能因无效消息无限占用 UI 线程。
        for _ in events.len()..limit {
            let Ok(event) = self.progress_rx.try_recv() else {
                break;
            };
            if self
                .records
                .get(&event.identity.task_id)
                .is_some_and(|record| {
                    record.identity == event.identity
                        && record.terminal.is_none()
                        && !record.cancel_requested.load(Ordering::Acquire)
                })
            {
                events.push(event);
            }
        }
        events
    }

    /// 核验与 UI 发布处于同一个原会话短门闩内，避免检查后再写的窗口。
    /// apply 只同步更新已拆借的 UI 字段，不得等待、访问数据库或重入 VaultService。
    pub fn apply_event(&mut self, event: TaskEvent, apply: impl FnOnce(TaskEventKind)) -> bool {
        let task_id = event.identity.task_id;
        let Some(record) = self.records.get(&task_id) else {
            return false;
        };
        if record.identity != event.identity {
            return false;
        }
        let terminal = event.kind.is_terminal();
        if terminal {
            // 仅真实 join 后发出的终态可接纳，伪造的提前完成不能清除活跃任务。
            if record.terminal.as_ref() != Some(&event.kind) {
                return false;
            }
        } else if record.terminal.is_some() || record.cancel_requested.load(Ordering::Acquire) {
            return false;
        }
        let accepted = self
            .service
            .with_session(&record.session, |_| {
                apply(event.kind);
                Ok(())
            })
            .is_ok();
        if terminal {
            // 无论是否仍属当前会话，已 join 的任务都不再保留正文和旧 Vault 句柄。
            self.records.remove(&task_id);
        } else if !accepted {
            self.request_cancel(task_id);
        }
        accepted
    }

    /// 使用记录中的真实会话核验身份，不信任收到的事件字段。
    /// 已回收或失效时返回 false，供 UI 清除被拒绝旧事件留下的进度。
    pub fn is_current(&self, task_id: TaskId) -> bool {
        self.records.get(&task_id).is_some_and(|record| {
            !record.stale
                && self
                    .service
                    .with_session(&record.session, |_| Ok(()))
                    .is_ok()
        })
    }

    /// 活跃任务和已 join、尚未消费的终态都算受管记录。
    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty() && self.records.is_empty()
    }

    /// 退出主循环后在 shared_runtime 上等待；不会新建或关闭共享 runtime。
    /// 先撤销全部任务，再逐个 join；一个 panic 也不能阻止其他任务回收。
    pub async fn shutdown(&mut self) -> ShutdownReport {
        self.shutting_down = true;
        self.progress_rx.close();
        self.cancel_all();
        let mut report = ShutdownReport::default();
        while let Some(result) = self.jobs.join_next_with_id().await {
            report.joined += 1;
            match result {
                Ok((_, Ok(_))) => {}
                Ok((_, Err(TaskFailure::Cancelled))) => report.cancelled += 1,
                Ok((_, Err(TaskFailure::Failed(_)))) => report.failed += 1,
                Err(error) if error.is_cancelled() => report.cancelled += 1,
                Err(_) => report.panicked += 1,
            }
        }
        self.runtime_ids.clear();
        self.records.clear();
        while self.progress_rx.try_recv().is_ok() {}
        report
    }

    fn outcome_kind(outcome: Result<TaskOutput, TaskFailure>) -> TaskEventKind {
        match outcome {
            Ok(output) => TaskEventKind::Completed(output),
            Err(TaskFailure::Cancelled) => TaskEventKind::Cancelled,
            Err(TaskFailure::Failed(message)) => TaskEventKind::Failed(message),
        }
    }

    fn join_error_kind(error: &JoinError) -> TaskEventKind {
        if error.is_cancelled() {
            TaskEventKind::Cancelled
        } else {
            // 此映射只返回固定错误，不读取或转发可能含用户数据的 panic payload。
            TaskEventKind::Failed(TASK_PANICKED.to_string())
        }
    }
}

impl Drop for Tasks {
    fn drop(&mut self) {
        self.cancel_all();
        // Drop 不能等待；退出路径仍必须显式调用并等待 shutdown。
        self.jobs.abort_all();
    }
}

#[cfg(test)]
mod tests;
