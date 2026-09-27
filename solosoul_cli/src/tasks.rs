//! CLI 自有异步任务及原会话内的结果接纳。
//!
//! 工作任务只能返回数据或报告进度，不能持有 App 的可变引用。异步入口禁止
//! 分离子任务或隐藏 spawn_blocking/thread::spawn；阻塞入口直接拥有实际闭包，
//! 以协作取消和真实 join 管理执行位，不能用 abort 冒充原生推理已停止。
//! 正常退出须显式 shutdown 并等待所有任务回收，Drop 仅负责请求取消。

use std::collections::{HashMap, VecDeque};
use std::future::Future;
#[cfg(test)]
use std::sync::atomic::AtomicBool;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use solosoul_core::ocr::control::OcrCancellation;
use solosoul_core::{VaultService, VaultSession};
use tokio::sync::mpsc;
use tokio::task::{AbortHandle, Id, JoinError, JoinSet};
use uuid::Uuid;

const PROGRESS_CAPACITY: usize = 64;
const BLOCKING_CAPACITY: usize = 5;
pub const BLOCKING_QUEUE_FULL: &str = "__BLOCKING_QUEUE_FULL__";
const TASK_ACTIVE: u8 = 0;
const TASK_CANCEL_REQUESTED: u8 = 1;
const TASK_COMMIT_CLAIMED: u8 = 2;
const TASK_PANICKED: &str = "后台任务执行失败";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskIdentity {
    pub task_id: TaskId,
    pub account_id: String,
    pub session_generation: u64,
}

/// 工作任务返回具体数据，不传回修改 App 的闭包。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskOutput {
    Message(String),
    OcrCompleted {
        result_json: String,
        source_path: String,
        mrz_json: Option<String>,
    },
    EmbedModelInstalled {
        model_id: String,
        bytes: u64,
    },
    PluginInstalled {
        plugin_id: String,
        version: String,
        name: String,
        description: String,
        tier: String,
        // prepare 阶段已校验的完整快照；用于刷新详情，不在会话门闩内重新读文件。
        manifest_json: String,
        updated: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskFailure {
    Cancelled,
    Failed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockingTaskState {
    Queued,
    Running,
    CancelRequested,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskEventKind {
    BlockingState(BlockingTaskState),
    Progress { current: u64, total: Option<u64> },
    PluginProgress(solosoul_plugin::PluginInstallProgress),
    Completed(TaskOutput),
    Failed(String),
    Cancelled,
}

impl TaskEventKind {
    fn is_terminal(&self) -> bool {
        !matches!(
            self,
            Self::Progress { .. } | Self::PluginProgress(_) | Self::BlockingState(_)
        )
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
    task_state: Arc<AtomicU8>,
    progress: mpsc::Sender<TaskEvent>,
    service: Arc<VaultService>,
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
        self.task_state.load(Ordering::Acquire) == TASK_CANCEL_REQUESTED
    }

    /// 原会话内的一次性最终提交。取消先取得许可时不执行 publish；提交先取得
    /// 许可后取消不再 abort，真实成功或失败仍由 JoinSet 返回。
    /// 网络、校验、flush 和文件关闭须在调用前完成。publish 仅做短同步发布，
    /// 不得重入 VaultService；调用本方法后必须直接返回，不再 await 或追加业务步骤。
    pub fn commit(
        self,
        publish: impl FnOnce() -> Result<TaskOutput, String>,
    ) -> Result<TaskOutput, TaskFailure> {
        self.service
            .with_session(&self.session, |_| {
                if self
                    .task_state
                    .compare_exchange(
                        TASK_ACTIVE,
                        TASK_COMMIT_CLAIMED,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    )
                    .is_err()
                {
                    return Ok(Err(TaskFailure::Cancelled));
                }
                // 发布失败同样封住取消，保留实际失败，不能改报取消。
                Ok(publish().map_err(TaskFailure::Failed))
            })
            .map_err(|_| TaskFailure::Cancelled)?
    }

    /// RF214：沿用有界队列与原任务身份，不允许进度回调直接修改 App。
    pub fn report_plugin_progress(&self, progress: solosoul_plugin::PluginInstallProgress) -> bool {
        if self.is_cancel_requested() {
            return false;
        }
        self.progress
            .try_send(TaskEvent {
                identity: self.identity.clone(),
                kind: TaskEventKind::PluginProgress(progress),
            })
            .is_ok()
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
    task_state: Arc<AtomicU8>,
    abort: Option<AbortHandle>,
    blocking: Option<BlockingRecord>,
    // Some 表示已真实 join 或排队闭包已析构，终态尚待主循环接纳。
    terminal: Option<TaskEventKind>,
    stale: bool,
}

struct BlockingRecord {
    cancellation: OcrCancellation,
    state: BlockingTaskState,
}

struct PendingBlocking {
    task_id: TaskId,
    work: Box<dyn FnOnce() -> Result<TaskOutput, TaskFailure> + Send>,
    runtime: tokio::runtime::Handle,
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
    blocking_queue: VecDeque<PendingBlocking>,
    blocking_active: Option<TaskId>,
    // 独立于可丢弃进度的控制事件；最多五条阻塞记录，各状态最多入队一次。
    blocking_states: VecDeque<TaskEvent>,
    blocking_terminals: VecDeque<TaskEvent>,
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
            blocking_queue: VecDeque::new(),
            blocking_active: None,
            blocking_states: VecDeque::new(),
            blocking_terminals: VecDeque::new(),
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
        let task_state = Arc::new(AtomicU8::new(TASK_ACTIVE));
        let context = TaskContext {
            identity: identity.clone(),
            session: session.clone(),
            task_state: Arc::clone(&task_state),
            progress: self.progress_tx.clone(),
            service: Arc::clone(&self.service),
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
                task_state,
                abort: Some(abort),
                blocking: None,
                terminal: None,
                stale: false,
            },
        );
        Ok(task_id)
    }

    /// 最多一个实际阻塞闭包和四个待派发闭包；未消费的终态仍计入准入上限。
    /// work 不得再派生隐藏任务。取消只设置 token，执行位直到真实 join 才释放。
    pub fn spawn_blocking<F>(&mut self, session: VaultSession, work: F) -> Result<TaskId, String>
    where
        F: FnOnce(TaskContext, OcrCancellation) -> Result<TaskOutput, TaskFailure> + Send + 'static,
    {
        if self.shutting_down {
            return Err("任务管理器正在退出".to_string());
        }
        self.service.with_session(&session, |_| Ok(()))?;
        if self
            .records
            .values()
            .filter(|record| record.blocking.is_some())
            .count()
            >= BLOCKING_CAPACITY
        {
            return Err(BLOCKING_QUEUE_FULL.to_string());
        }
        let runtime = crate::util::shared_runtime().map_err(|error| error.to_string())?;
        let task_id = TaskId(Uuid::new_v4());
        let identity = TaskIdentity {
            task_id,
            account_id: session.account_id().to_string(),
            session_generation: session.generation(),
        };
        let task_state = Arc::new(AtomicU8::new(TASK_ACTIVE));
        let cancellation = OcrCancellation::default();
        let context = TaskContext {
            identity: identity.clone(),
            session: session.clone(),
            task_state: Arc::clone(&task_state),
            progress: self.progress_tx.clone(),
            service: Arc::clone(&self.service),
        };
        let service = Arc::clone(&self.service);
        let worker_cancellation = cancellation.clone();
        self.records.insert(
            task_id,
            TaskRecord {
                identity: identity.clone(),
                session,
                task_state,
                abort: None,
                blocking: Some(BlockingRecord {
                    cancellation,
                    state: BlockingTaskState::Queued,
                }),
                terminal: None,
                stale: false,
            },
        );
        self.blocking_states.push_back(TaskEvent {
            identity,
            kind: TaskEventKind::BlockingState(BlockingTaskState::Queued),
        });
        self.blocking_queue.push_back(PendingBlocking {
            task_id,
            runtime: runtime.handle().clone(),
            work: Box::new(move || {
                // 工作尚未开始时，锁定、同账户重登、排队取消都不能加载模型。
                if context.is_cancel_requested()
                    || service.with_session(context.session(), |_| Ok(())).is_err()
                {
                    return Err(TaskFailure::Cancelled);
                }
                work(context, worker_cancellation)
            }),
        });
        self.dispatch_next_blocking();
        Ok(task_id)
    }

    fn dispatch_next_blocking(&mut self) {
        if self.shutting_down || self.blocking_active.is_some() {
            return;
        }
        let Some(pending) = self.blocking_queue.pop_front() else {
            return;
        };
        let record = self
            .records
            .get_mut(&pending.task_id)
            .expect("queued task owns its record");
        record
            .blocking
            .as_mut()
            .expect("queued blocking task")
            .state = BlockingTaskState::Running;
        self.blocking_states
            .retain(|event| event.identity.task_id != pending.task_id);
        self.blocking_states.push_back(TaskEvent {
            identity: record.identity.clone(),
            kind: TaskEventKind::BlockingState(BlockingTaskState::Running),
        });
        // JoinSet 直接持有原生闭包，不持有可先被 abort 的异步等待包装。
        let abort = self.jobs.spawn_blocking_on(pending.work, &pending.runtime);
        self.runtime_ids.insert(abort.id(), pending.task_id);
        record.abort = Some(abort);
        self.blocking_active = Some(pending.task_id);
    }

    /// true 仅表示仍受管的任务已收到取消请求，不代表已结束。
    /// 已取得最终提交许可或已 join 时返回 false，不再 abort。
    pub fn request_cancel(&mut self, task_id: TaskId) -> bool {
        let Some(record) = self.records.get_mut(&task_id) else {
            return false;
        };
        if record.terminal.is_some() || !Self::cancel_record(record) {
            return false;
        }
        if let Some(blocking) = &mut record.blocking {
            if blocking.state != BlockingTaskState::CancelRequested {
                blocking.state = BlockingTaskState::CancelRequested;
                self.blocking_states
                    .retain(|event| event.identity.task_id != task_id);
                self.blocking_states.push_back(TaskEvent {
                    identity: record.identity.clone(),
                    kind: TaskEventKind::BlockingState(BlockingTaskState::CancelRequested),
                });
            }
            if let Some(index) = self
                .blocking_queue
                .iter()
                .position(|pending| pending.task_id == task_id)
            {
                // 先析构所有捕获资源，再产生无需 native join 的排队取消终态。
                drop(self.blocking_queue.remove(index));
                record.terminal = Some(TaskEventKind::Cancelled);
                self.blocking_terminals.push_back(TaskEvent {
                    identity: record.identity.clone(),
                    kind: TaskEventKind::Cancelled,
                });
            }
        }
        true
    }

    pub fn cancel_all(&mut self) {
        let ids: Vec<_> = self.records.keys().copied().collect();
        for task_id in ids {
            self.request_cancel(task_id);
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
            }
        }
        for task_id in &stale {
            self.request_cancel(*task_id);
        }
        stale
    }

    fn cancel_record(record: &TaskRecord) -> bool {
        match record.task_state.compare_exchange(
            TASK_ACTIVE,
            TASK_CANCEL_REQUESTED,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) | Err(TASK_CANCEL_REQUESTED) => {
                if let Some(blocking) = &record.blocking {
                    blocking.cancellation.cancel();
                } else if let Some(abort) = &record.abort {
                    abort.abort();
                }
                true
            }
            // 最终发布已取得许可：等待真实 join，不能把已提交工作改报取消。
            Err(_) => false,
        }
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
            if self.blocking_active == Some(task_id) {
                self.blocking_active = None;
            }
            let Some(record) = self.records.get_mut(&task_id) else {
                continue;
            };
            let kind = if record.blocking.is_some()
                && record.task_state.load(Ordering::Acquire) == TASK_CANCEL_REQUESTED
            {
                TaskEventKind::Cancelled
            } else {
                kind
            };
            record.terminal = Some(kind.clone());
            events.push(TaskEvent {
                identity: record.identity.clone(),
                kind,
            });
        }
        self.dispatch_next_blocking();
        while events.len() < limit {
            let Some(event) = self.blocking_terminals.pop_front() else {
                break;
            };
            events.push(event);
        }
        // 状态独立于高频进度，取消请求不会因通道满而丢失；旧状态计入扫描预算。
        let mut scanned = events.len();
        while scanned < limit {
            let Some(event) = self.blocking_states.pop_front() else {
                break;
            };
            scanned += 1;
            if self.accepts_nonterminal(&event) {
                events.push(event);
            }
        }
        // 被丢弃的旧进度也计入扫描预算，不能因无效消息无限占用 UI 线程。
        for _ in scanned..limit {
            let Ok(event) = self.progress_rx.try_recv() else {
                break;
            };
            if self.accepts_nonterminal(&event) {
                events.push(event);
            }
        }
        events
    }

    fn accepts_nonterminal(&self, event: &TaskEvent) -> bool {
        self.records
            .get(&event.identity.task_id)
            .is_some_and(|record| {
                if record.identity != event.identity || record.terminal.is_some() {
                    return false;
                }
                if let TaskEventKind::BlockingState(state) = &event.kind {
                    return record
                        .blocking
                        .as_ref()
                        .is_some_and(|blocking| blocking.state == *state);
                }
                record.task_state.load(Ordering::Acquire) != TASK_CANCEL_REQUESTED
            })
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
            // 仅实际工作已结束（或排队闭包已析构）的终态可接纳，不能伪造提前完成。
            if record.terminal.as_ref() != Some(&event.kind) {
                return false;
            }
        } else if !self.accepts_nonterminal(&event) {
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
            // 无论是否仍属当前会话，已结束任务都不再保留正文和旧 Vault 句柄。
            self.records.remove(&task_id);
            self.blocking_states
                .retain(|pending| pending.identity.task_id != task_id);
            self.blocking_terminals
                .retain(|pending| pending.identity.task_id != task_id);
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
        self.jobs.is_empty() && self.records.is_empty() && self.blocking_queue.is_empty()
    }

    /// 退出主循环后在 shared_runtime 上等待；不会新建或关闭共享 runtime。
    /// 撤销可取消任务，等待已取得提交许可的任务，再逐个 join；panic 不阻止回收。
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
        self.blocking_active = None;
        self.blocking_queue.clear();
        self.blocking_states.clear();
        self.blocking_terminals.clear();
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

#[cfg(test)]
mod rf212_tests;

#[cfg(test)]
mod rf215_tests;
