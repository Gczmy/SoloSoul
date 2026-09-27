//! RF211：真实认证入口与 Tick 不应接纳旧会话的后台消息。
//!
//! 首组用合成 worker 和旧 plugin_run_pending 复现迟到消息；后续走真实 Tasks。
//! 所有用例只使用临时 Vault，不安装插件、不访问网络。

use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent};
use solosoul_core::VaultService;

use super::{App, AppPhase, PluginRunMessage, UnlockStep};
use crate::commands::auth;
use crate::events::Event;
use crate::tasks::{TaskEvent, TaskEventKind, TaskFailure, TaskId, TaskOutput};
use tokio::sync::oneshot;

const WORKER_TIMEOUT: Duration = Duration::from_secs(30);

/// 单次屏障：显式完成或断言展开时均会放行并 join，避免遗留后台线程。
struct PendingWorker {
    release: Option<mpsc::Sender<()>>,
    handle: Option<JoinHandle<Result<(), String>>>,
}

impl PendingWorker {
    fn start(app: &mut App, result: PluginRunMessage) -> Self {
        let holder = Arc::new(Mutex::new(None));
        app.plugin_run_pending = Some(Arc::clone(&holder));
        let (release, released) = mpsc::channel();
        let (started, ready) = mpsc::channel();
        let handle = thread::spawn(move || {
            started.send(()).map_err(|error| error.to_string())?;
            released
                .recv_timeout(WORKER_TIMEOUT)
                .map_err(|error| format!("synthetic worker release: {error}"))?;
            *holder
                .lock()
                .map_err(|_| "synthetic result holder poisoned".to_string())? = Some(result);
            Ok(())
        });
        // 先建立 RAII 所有者，再等待 ready；等待失败也一定回收线程。
        let worker = Self {
            release: Some(release),
            handle: Some(handle),
        };
        ready
            .recv_timeout(WORKER_TIMEOUT)
            .expect("synthetic worker must reach its barrier");
        worker
    }

    fn complete(&mut self) -> Result<(), String> {
        let send_result = self
            .release
            .take()
            .ok_or_else(|| "synthetic worker was already released".to_string())?
            .send(())
            .map_err(|error| error.to_string());
        let worker_result = self
            .handle
            .take()
            .ok_or_else(|| "synthetic worker was already joined".to_string())?
            .join()
            .map_err(|_| "synthetic worker panicked".to_string())?;
        send_result?;
        worker_result
    }
}

impl Drop for PendingWorker {
    fn drop(&mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn key(app: &mut App, code: KeyCode) {
    assert!(
        !app.handle_event(Event::Key(KeyEvent::from(code)))
            .expect("synthetic key must be handled"),
        "unlock input must not exit the CLI"
    );
}

/// 复用真实 /unlock 和密码键盘处理；多账户时通过列表按键选择目标。
fn unlock_account(app: &mut App, account_id: &str) {
    auth::unlock(app).expect("start real unlock wizard");
    if let AppPhase::UnlockWizard {
        step: UnlockStep::SelectAccount { accounts, .. },
    } = &app.phase
    {
        let index = accounts
            .iter()
            .position(|account| account.id == account_id)
            .expect("requested synthetic account must be listed");
        for _ in 0..index {
            key(app, KeyCode::Down);
        }
        key(app, KeyCode::Enter);
    }
    assert!(matches!(
        &app.phase,
        AppPhase::UnlockWizard {
            step: UnlockStep::EnterPassword { account_id: selected, .. }
        } if selected == account_id
    ));
    for character in crate::TEST_PASSWORD.chars() {
        key(app, KeyCode::Char(character));
    }
    key(app, KeyCode::Enter);
    assert!(matches!(
        &app.phase,
        AppPhase::Home { account_id: current } if current == account_id
    ));
    assert_eq!(
        app.vault_service.get_current_account().as_deref(),
        Some(account_id)
    );
    assert!(app.error_message.is_none());
}

#[derive(Clone, Copy)]
enum Transition {
    Locked,
    SameAccountUnlocked,
    OtherAccountUnlocked,
}

fn assert_stale_plugin_result_ignored(transition: Transition, is_error: bool) {
    // 锁的生命周期覆盖账户创建、真实登录、worker 回收与 TempDir 清理。
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let directory = tempfile::TempDir::new().expect("synthetic Vault directory");
    let service = VaultService::with_base_path(directory.path().to_path_buf());
    let account_a = service
        .create_account("RF211 A", crate::TEST_PASSWORD, None)
        .expect("create synthetic account A");
    let account_a = account_a["id"].as_str().unwrap().to_string();
    let account_b = if matches!(transition, Transition::OtherAccountUnlocked) {
        let account = service
            .create_account("RF211 B", crate::TEST_PASSWORD, None)
            .expect("create synthetic account B");
        Some(account["id"].as_str().unwrap().to_string())
    } else {
        None
    };
    service.lock();
    let mut app = App::new(Arc::new(service)).expect("real CLI App");
    unlock_account(&mut app, &account_a);
    let original_generation = app
        .vault_service
        .capture_session(&account_a)
        .expect("capture original session")
        .generation();
    let stale_text = if is_error {
        "RF211 OLD ACCOUNT synthetic plugin failure"
    } else {
        "RF211 OLD ACCOUNT synthetic plugin success"
    };
    let mut worker = PendingWorker::start(&mut app, (is_error, stale_text.to_string()));

    // worker 已启动，但结果尚未产生；真实命令先改变认证状态。
    auth::lock(&mut app);
    assert!(matches!(app.phase, AppPhase::Locked));
    assert!(!app.vault_service.is_unlocked());
    match transition {
        Transition::Locked => {}
        Transition::SameAccountUnlocked => unlock_account(&mut app, &account_a),
        Transition::OtherAccountUnlocked => {
            unlock_account(&mut app, account_b.as_deref().unwrap());
        }
    }
    let expected_account = app.vault_service.get_current_account();
    if let Some(account_id) = expected_account.as_deref() {
        let generation = app
            .vault_service
            .capture_session(account_id)
            .expect("capture replacement session")
            .generation();
        assert_ne!(generation, original_generation);
    }
    let expected_error = app.error_message.clone();
    let expected_info = app.info_message.clone();
    let expected_activity = app.last_activity;

    // 确定旧结果已经写完再 Tick；不靠 sleep 或调度速度推断迟到顺序。
    worker
        .complete()
        .expect("release and join synthetic worker");
    assert!(
        !app.handle_event(Event::Tick).expect("poll real Tick"),
        "late result must not exit the CLI"
    );
    match transition {
        Transition::Locked => assert!(matches!(app.phase, AppPhase::Locked)),
        Transition::SameAccountUnlocked | Transition::OtherAccountUnlocked => {
            assert!(matches!(
                &app.phase,
                AppPhase::Home { account_id } if Some(account_id) == expected_account.as_ref()
            ));
        }
    }
    assert_eq!(app.vault_service.get_current_account(), expected_account);
    assert_eq!(
        app.last_activity, expected_activity,
        "Tick is not user activity"
    );
    assert_eq!(
        app.info_message, expected_info,
        "old successful plugin result must not replace current-session UI"
    );
    assert_eq!(
        app.error_message, expected_error,
        "old failed plugin result must not replace current-session UI"
    );

    // 后续 Tick 也不能再次投递此前丢弃的终态。
    assert!(!app
        .handle_event(Event::Tick)
        .expect("poll second real Tick"));
    assert_eq!(app.info_message, expected_info);
    assert_eq!(app.error_message, expected_error);
}

#[test]
fn rf211_locked_tick_ignores_old_plugin_success() {
    assert_stale_plugin_result_ignored(Transition::Locked, false);
}

#[test]
fn rf211_locked_tick_ignores_old_plugin_failure() {
    assert_stale_plugin_result_ignored(Transition::Locked, true);
}

#[test]
fn rf211_same_account_reunlock_ignores_old_plugin_success() {
    assert_stale_plugin_result_ignored(Transition::SameAccountUnlocked, false);
}

#[test]
fn rf211_account_switch_ignores_old_plugin_failure() {
    assert_stale_plugin_result_ignored(Transition::OtherAccountUnlocked, true);
}

/// 新任务入口的真实 App；失败展开时先由 TaskProbe 放行，再等待任务实际回收。
struct ManagedFixture {
    app: App,
    account_a: String,
    account_b: Option<String>,
    _directory: tempfile::TempDir,
}

impl ManagedFixture {
    fn new(with_second_account: bool) -> Self {
        let directory = tempfile::TempDir::new().expect("managed-task Vault directory");
        let service = VaultService::with_base_path(directory.path().to_path_buf());
        let account = service
            .create_account("RF211 managed A", crate::TEST_PASSWORD, None)
            .expect("create managed-task account A");
        let account_a = account["id"].as_str().unwrap().to_string();
        let account_b = if with_second_account {
            let account = service
                .create_account("RF211 managed B", crate::TEST_PASSWORD, None)
                .expect("create managed-task account B");
            Some(account["id"].as_str().unwrap().to_string())
        } else {
            None
        };
        service.lock();
        let app = App::new(Arc::new(service)).expect("managed-task App");
        let mut fixture = Self {
            app,
            account_a,
            account_b,
            _directory: directory,
        };
        unlock_account(&mut fixture.app, &fixture.account_a);
        fixture
    }
}

impl Drop for ManagedFixture {
    fn drop(&mut self) {
        let _ = self.app.shutdown_tasks();
    }
}

/// 实际 Future 析构确认，不把“已请求取消”误当作 worker 已退出。
struct TaskResource(mpsc::Sender<()>);

impl Drop for TaskResource {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}

struct TaskProbe {
    task_id: TaskId,
    release: Option<oneshot::Sender<()>>,
    dropped: mpsc::Receiver<()>,
}

impl TaskProbe {
    fn start(app: &mut App, outcome: Result<TaskOutput, TaskFailure>) -> Self {
        let account = app.vault_service.get_current_account().unwrap();
        let session = app.vault_service.capture_session(&account).unwrap();
        let (release, released) = oneshot::channel();
        let (started, ready) = mpsc::channel();
        let (dropped, observed) = mpsc::channel();
        let task_id = app
            .tasks
            .spawn(session, move |context| async move {
                let _resource = TaskResource(dropped);
                let accepted = context.report_progress(3, Some(9));
                let _ = started.send(accepted);
                // Tokio worker 只等待异步屏障；同步 recv 仅在测试主线程使用。
                let _ = released.await;
                outcome
            })
            .expect("spawn on the real shared runtime");
        let probe = Self {
            task_id,
            release: Some(release),
            dropped: observed,
        };
        assert!(
            ready
                .recv_timeout(WORKER_TIMEOUT)
                .expect("task reaches async barrier"),
            "real progress must have been queued"
        );
        probe
    }

    fn finish(&mut self) {
        self.release
            .take()
            .expect("task release is single use")
            .send(())
            .expect("task is still waiting at its async barrier");
        self.wait_for_drop();
    }

    fn wait_for_drop(&self) {
        self.dropped
            .recv_timeout(WORKER_TIMEOUT)
            .expect("actual Future resource must be released");
    }
}

impl Drop for TaskProbe {
    fn drop(&mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
    }
}

fn drain_until_reclaimed(app: &mut App) {
    let result = crate::util::shared_runtime()
        .expect("existing shared runtime")
        .block_on(async {
            tokio::time::timeout(WORKER_TIMEOUT, async {
                while !app.tasks.is_empty() {
                    app.drain_task_events(16)?;
                    tokio::task::yield_now().await;
                }
                Ok::<(), color_eyre::Report>(())
            })
            .await
        });
    result
        .expect("task drain must finish")
        .expect("real App drain succeeds");
}

/// 保存生产 manager 真正 join 后产生的事件，精确控制其到达 App 的时点。
fn take_completed_event(app: &mut App) -> TaskEvent {
    crate::util::shared_runtime()
        .expect("existing shared runtime")
        .block_on(async {
            tokio::time::timeout(WORKER_TIMEOUT, async {
                loop {
                    if let Some(event) = app.tasks.poll_events(1).into_iter().next() {
                        return event;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
        })
        .expect("completed task must produce a real event")
}

fn assert_managed_output_requires_app_drain(is_error: bool) {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut fixture = ManagedFixture::new(false);
    let message = "RF211 managed synthetic result";
    let outcome = if is_error {
        Err(TaskFailure::Failed(message.to_string()))
    } else {
        Ok(TaskOutput::Message(message.to_string()))
    };
    let mut task = TaskProbe::start(&mut fixture.app, outcome);
    let activity = fixture.app.last_activity;

    // worker 已报告进度，但没有 App 事件消费就不能自行修改 UI。
    assert!(fixture.app.task_progress.is_empty());
    assert!(fixture.app.info_message.is_none());
    assert!(fixture.app.error_message.is_none());
    fixture
        .app
        .drain_task_events(1)
        .expect("drain queued progress");
    assert_eq!(
        fixture.app.task_progress.get(&task.task_id),
        Some(&(3, Some(9)))
    );
    assert_eq!(fixture.app.last_activity, activity);
    assert!(fixture.app.info_message.is_none());
    assert!(fixture.app.error_message.is_none());

    task.finish();
    assert!(
        fixture.app.info_message.is_none(),
        "worker must not publish success itself"
    );
    assert!(
        fixture.app.error_message.is_none(),
        "worker must not publish failure itself"
    );
    assert_eq!(
        fixture.app.task_progress.get(&task.task_id),
        Some(&(3, Some(9)))
    );

    drain_until_reclaimed(&mut fixture.app);
    assert!(fixture.app.task_progress.is_empty());
    assert!(fixture.app.tasks.is_empty());
    assert_eq!(fixture.app.last_activity, activity);
    if is_error {
        assert_eq!(fixture.app.error_message.as_deref(), Some(message));
        assert!(fixture.app.info_message.is_none());
    } else {
        assert_eq!(fixture.app.info_message.as_deref(), Some(message));
        assert!(fixture.app.error_message.is_none());
    }
}

#[test]
fn rf211_managed_success_and_progress_only_reach_app_through_drain() {
    assert_managed_output_requires_app_drain(false);
}

#[test]
fn rf211_managed_failure_routes_to_error_only_after_drain() {
    assert_managed_output_requires_app_drain(true);
}

fn assert_joined_event_rejected_after_session_change(transition: Transition) {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut fixture = ManagedFixture::new(matches!(transition, Transition::OtherAccountUnlocked));
    let mut task = TaskProbe::start(
        &mut fixture.app,
        Ok(TaskOutput::Message(
            "RF211 completed OLD SESSION body".to_string(),
        )),
    );
    fixture
        .app
        .drain_task_events(1)
        .expect("drain old progress");
    assert_eq!(
        fixture.app.task_progress.get(&task.task_id),
        Some(&(3, Some(9)))
    );
    task.finish();
    let completed = take_completed_event(&mut fixture.app);
    assert_eq!(completed.identity.task_id, task.task_id);
    assert_eq!(completed.identity.account_id, fixture.account_a);
    assert!(matches!(
        &completed.kind,
        TaskEventKind::Completed(TaskOutput::Message(_))
    ));
    assert!(fixture.app.info_message.is_none());

    auth::lock(&mut fixture.app);
    assert!(matches!(fixture.app.phase, AppPhase::Locked));
    match transition {
        Transition::Locked => {}
        Transition::SameAccountUnlocked => {
            unlock_account(&mut fixture.app, &fixture.account_a);
        }
        Transition::OtherAccountUnlocked => {
            unlock_account(&mut fixture.app, fixture.account_b.as_deref().unwrap());
        }
    }
    let expected_account = fixture.app.vault_service.get_current_account();
    if let Some(account_id) = expected_account.as_deref() {
        let session = fixture
            .app
            .vault_service
            .capture_session(account_id)
            .unwrap();
        assert_ne!(session.generation(), completed.identity.session_generation);
    }
    let info = fixture.app.info_message.clone();
    let error = fixture.app.error_message.clone();
    let activity = fixture.app.last_activity;

    assert!(!fixture
        .app
        .handle_event(Event::Task(completed))
        .expect("dispatch real task event"));
    assert_eq!(
        fixture.app.info_message, info,
        "joined old body must not be published"
    );
    assert_eq!(fixture.app.error_message, error);
    assert_eq!(fixture.app.last_activity, activity);
    assert_eq!(
        fixture.app.vault_service.get_current_account(),
        expected_account
    );
    assert!(fixture.app.task_progress.is_empty());
    assert!(fixture.app.tasks.is_empty());
    match transition {
        Transition::Locked => assert!(matches!(fixture.app.phase, AppPhase::Locked)),
        Transition::SameAccountUnlocked | Transition::OtherAccountUnlocked => {
            assert!(matches!(
                &fixture.app.phase,
                AppPhase::Home { account_id } if Some(account_id) == expected_account.as_ref()
            ));
        }
    }
}

#[test]
fn rf211_already_joined_task_event_is_rejected_after_real_lock() {
    assert_joined_event_rejected_after_session_change(Transition::Locked);
}

#[test]
fn rf211_already_joined_task_event_is_rejected_after_same_account_reunlock() {
    assert_joined_event_rejected_after_session_change(Transition::SameAccountUnlocked);
}

#[test]
fn rf211_already_joined_task_event_is_rejected_after_account_switch() {
    assert_joined_event_rejected_after_session_change(Transition::OtherAccountUnlocked);
}

#[test]
fn rf211_drain_removes_stale_progress_after_direct_vault_lock() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut fixture = ManagedFixture::new(false);
    let task = TaskProbe::start(
        &mut fixture.app,
        Ok(TaskOutput::Message(
            "RF211 direct-lock stale body".to_string(),
        )),
    );
    fixture
        .app
        .drain_task_events(1)
        .expect("drain active progress");
    assert_eq!(
        fixture.app.task_progress.get(&task.task_id),
        Some(&(3, Some(9)))
    );
    let activity = fixture.app.last_activity;

    // 不经过 App 的 clear_sensitive_state，检验每轮 drain 自身的会话失效清理。
    fixture.app.vault_service.lock();
    assert!(matches!(fixture.app.phase, AppPhase::Home { .. }));
    assert_eq!(
        fixture.app.task_progress.get(&task.task_id),
        Some(&(3, Some(9)))
    );
    fixture
        .app
        .drain_task_events(1)
        .expect("detect stale task and remove its progress");

    assert!(fixture.app.task_progress.is_empty());
    assert!(fixture.app.info_message.is_none());
    assert!(fixture.app.error_message.is_none());
    assert_eq!(fixture.app.last_activity, activity);
    task.wait_for_drop();
    drain_until_reclaimed(&mut fixture.app);
    assert!(fixture.app.tasks.is_empty());
    assert!(fixture.app.info_message.is_none());
    assert!(fixture.app.error_message.is_none());
}
