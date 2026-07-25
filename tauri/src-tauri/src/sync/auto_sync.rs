//! 自动同步调度器。
//!
//! `AutoSyncManager` 是 Android SAF 自动同步的唯一调度入口，负责将
//! 关键里程碑同步、写操作防抖同步和切后台同步排队为单任务执行，
//! 避免多个 `sync_to_remote()` 并发运行。

use crate::attachment_import_plugin::AttachmentImportPluginHandle;
use futures::future::BoxFuture;
use solosoul_core::VaultService;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;

/// 自动同步触发来源。
///
/// 用于区分周期性兜底、写操作防抖、关键里程碑等触发场景，
/// 以便前端决定是否显示提示。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncSource {
    /// 30 秒周期性兜底同步。
    Periodic,
    /// 写操作防抖同步。
    Debounce,
    /// 关键里程碑 / 立即同步。
    Immediate,
    /// 应用切后台同步。
    Background,
}

/// 自动同步触发事件。
pub enum SyncEvent {
    /// 立即同步，取消当前防抖定时器。
    Immediate,
    /// 防抖同步，30 秒内无新事件才执行。
    Debounce,
    /// 应用切后台 / 失去焦点，立即同步。
    Background,
}

impl SyncEvent {
    fn source(&self) -> SyncSource {
        match self {
            SyncEvent::Immediate => SyncSource::Immediate,
            SyncEvent::Debounce => SyncSource::Debounce,
            SyncEvent::Background => SyncSource::Background,
        }
    }
}

enum AutoSyncState {
    Idle,
    Scheduled(SyncSource, tokio::time::Instant),
    Running(SyncSource),
}

/// 可注入的同步动作。
///
/// 将具体的同步实现从调度循环中解耦，便于单元测试和生产注入。
pub trait SyncAction: Send + Sync + 'static {
    fn run(&self, source: SyncSource) -> BoxFuture<'static, Result<(), String>>;
}

/// 自动同步配置。
#[derive(Clone)]
pub struct AutoSyncConfig {
    /// 防抖同步的延迟。
    pub debounce_delay: Duration,
    /// 周期性同步的间隔。
    pub periodic_interval: Duration,
    /// 同步失败后的最大重试次数。
    pub max_retries: u32,
    /// 重试的基准退避间隔。
    pub retry_delay: Duration,
}

impl AutoSyncConfig {
    /// 生产默认配置。
    pub fn production() -> Self {
        Self {
            debounce_delay: Duration::from_secs(30),
            periodic_interval: Duration::from_secs(30),
            max_retries: 3,
            retry_delay: Duration::from_millis(500),
        }
    }
}

impl Default for AutoSyncConfig {
    fn default() -> Self {
        Self::production()
    }
}

/// 自动同步管理器。
///
/// 通过内部 mpsc 通道接收事件，在独立任务中调度同步。
#[derive(Clone)]
pub struct AutoSyncManager {
    tx: mpsc::Sender<SyncEvent>,
}

impl AutoSyncManager {
    /// 创建并启动自动同步任务（生产入口）。
    pub fn new_for_vault(vault_service: Arc<RwLock<VaultService>>, app_handle: AppHandle) -> Self {
        let action = Arc::new(VaultSyncAction::new(vault_service, app_handle));
        Self::new(action, AutoSyncConfig::default())
    }

    /// 创建并启动自动同步任务（可注入同步动作与配置）。
    pub fn new(action: Arc<dyn SyncAction>, config: AutoSyncConfig) -> Self {
        let (tx, rx) = mpsc::channel(64);
        let manager = Self { tx };
        manager.start_loop(rx, action, config);
        manager
    }

    /// 触发一次立即同步。
    pub fn trigger_immediate(&self) {
        let _ = self.tx.try_send(SyncEvent::Immediate);
    }

    /// 触发一次防抖同步。
    pub fn trigger_debounce(&self) {
        let _ = self.tx.try_send(SyncEvent::Debounce);
    }

    /// 触发一次切后台同步。
    pub fn trigger_background(&self) {
        let _ = self.tx.try_send(SyncEvent::Background);
    }

    fn start_loop(
        &self,
        mut rx: mpsc::Receiver<SyncEvent>,
        action: Arc<dyn SyncAction>,
        config: AutoSyncConfig,
    ) {
        let fut = async move {
            let mut state = AutoSyncState::Idle;
            let mut interval = tokio::time::interval(config.periodic_interval);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            // tokio::time::interval 第一次 tick 会立即触发；
            // 先消耗掉这次 tick，避免启动时误触发一次同步。
            interval.tick().await;
            let mut retry_count: u32 = 0;

            loop {
                match state {
                    AutoSyncState::Idle => {
                        tokio::select! {
                            event = rx.recv() => match event {
                                Some(SyncEvent::Immediate | SyncEvent::Background) => {
                                    state = AutoSyncState::Running(event.unwrap().source());
                                }
                                Some(SyncEvent::Debounce) => {
                                    let deadline = tokio::time::Instant::now() + config.debounce_delay;
                                    state = AutoSyncState::Scheduled(SyncSource::Debounce, deadline);
                                }
                                None => break,
                            },
                            _ = interval.tick() => {
                                state = AutoSyncState::Running(SyncSource::Periodic);
                            }
                        }
                    }
                    AutoSyncState::Scheduled(source, d) => {
                        tokio::select! {
                            event = rx.recv() => match event {
                                Some(SyncEvent::Immediate | SyncEvent::Background) => {
                                    state = AutoSyncState::Running(event.unwrap().source());
                                }
                                Some(SyncEvent::Debounce) => {
                                    let new_deadline = tokio::time::Instant::now() + config.debounce_delay;
                                    state = AutoSyncState::Scheduled(source, new_deadline);
                                }
                                None => break,
                            },
                            _ = tokio::time::sleep_until(d) => {
                                state = AutoSyncState::Running(source);
                            }
                            _ = interval.tick() => {
                                // Already scheduled, nothing to do.
                            }
                        }
                    }
                    AutoSyncState::Running(source) => {
                        let result = action.run(source).await;
                        match result {
                            Ok(()) => {
                                retry_count = 0;
                                state = AutoSyncState::Idle;
                            }
                            Err(_) => {
                                if retry_count < config.max_retries {
                                    retry_count += 1;
                                    let exponent = (retry_count - 1).min(10);
                                    let backoff = config.retry_delay * 2u32.pow(exponent);
                                    tokio::time::sleep(backoff).await;
                                    state = AutoSyncState::Running(source);
                                } else {
                                    retry_count = 0;
                                    state = AutoSyncState::Idle;
                                }
                            }
                        }
                    }
                }
            }
        };

        // 生产环境使用 tauri 的全局 async runtime，确保在无 Tokio 上下文（如
        // Android 主线程）中也能成功 spawn。测试环境下使用 tokio::spawn 即可，
        // 因为 #[tokio::test] 已建立运行时上下文。
        #[cfg(test)]
        tokio::spawn(fut);
        #[cfg(not(test))]
        tauri::async_runtime::spawn(fut);
    }
}

/// 生产用同步动作：调用 VaultService.sync_to_remote()。
struct VaultSyncAction {
    vault_service: Arc<RwLock<VaultService>>,
    app_handle: AppHandle,
}

impl VaultSyncAction {
    fn new(vault_service: Arc<RwLock<VaultService>>, app_handle: AppHandle) -> Self {
        Self {
            vault_service,
            app_handle,
        }
    }
}

impl SyncAction for VaultSyncAction {
    fn run(&self, source: SyncSource) -> BoxFuture<'static, Result<(), String>> {
        let vault_service = self.vault_service.clone();
        let app_handle = self.app_handle.clone();
        Box::pin(async move { run_sync(&vault_service, &app_handle, source).await })
    }
}

/// 从应用数据目录读取保存的 SAF tree URI。
fn load_saved_saf_uri(data_dir: &Path) -> Option<String> {
    let path = data_dir.join("app_config.json");
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&path).ok()?;
    let config: serde_json::Value = serde_json::from_str(&content).ok()?;
    config
        .get("saf_tree_uri")
        .and_then(|v| v.as_str())
        .map(String::from)
}

/// 检查 SAF tree URI 是否仍然可访问。
fn check_saf_access(app_handle: &AppHandle, tree_uri: &str) -> bool {
    let handle = app_handle.state::<AttachmentImportPluginHandle<tauri::Wry>>();
    handle.check_vault_dir_access(tree_uri).unwrap_or(false)
}

/// 执行一次 `sync_to_remote()` 并发射进度事件。
///
/// 非 SAF 模式下会快速返回，不执行任何 I/O。
///
/// - `source` 标识触发来源；`Periodic` 同步会标记为 `silent`，
///   前端可据此不显示提示。
/// - 若本地无脏数据，直接跳过并返回 Ok，不发射任何事件。
pub async fn run_sync(
    vault_service: &Arc<RwLock<VaultService>>,
    app_handle: &AppHandle,
    source: SyncSource,
) -> Result<(), String> {
    let silent = source == SyncSource::Periodic;

    // SAF 未启用时直接跳过，避免桌面端无意义开销。
    {
        let guard = vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        if !guard.is_remote_storage() {
            return Ok(());
        }
    }

    // 校验 SAF 授权是否仍然有效；若已失效则广播事件并跳过同步，
    // 避免在授权撤销后继续尝试写入 SAF 导致崩溃或误导性错误。
    if let Ok(data_dir) = app_handle
        .path()
        .resolve(".", tauri::path::BaseDirectory::Data)
    {
        if let Some(tree_uri) = load_saved_saf_uri(&data_dir) {
            if !check_saf_access(app_handle, &tree_uri) {
                let _ = app_handle.emit("saf-auth-revoked", ());
                return Err("SAF 目录访问权限已失效".to_string());
            }
        }
    }

    // 无脏数据时静默跳过，避免无意义的同步和提示。
    {
        let guard = vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        if !guard.is_dirty() {
            return Ok(());
        }
    }

    let event = || {
        serde_json::json!({
            "phase": "sync_start",
            "current": 0,
            "total": 1,
            "source": source_string(source),
            "silent": silent,
        })
    };

    if !silent {
        app_handle.emit("sync-progress", event()).ok();
    }

    let svc = vault_service.clone();
    let result = tokio::task::spawn_blocking(move || {
        let guard = svc
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        guard.sync_to_remote()
    })
    .await
    .map_err(|e| format!("同步任务 panic: {e}"))?;

    match result {
        Ok(()) => {
            if !silent {
                app_handle
                    .emit(
                        "sync-progress",
                        serde_json::json!({
                            "phase": "sync_complete",
                            "current": 1,
                            "total": 1,
                            "source": source_string(source),
                            "silent": silent,
                        }),
                    )
                    .ok();
            }
            Ok(())
        }
        Err(e) => {
            app_handle
                .emit(
                    "sync-progress",
                    serde_json::json!({
                        "phase": "error",
                        "message": e.clone(),
                        "source": source_string(source),
                        "silent": silent,
                    }),
                )
                .ok();
            Err(e)
        }
    }
}

fn source_string(source: SyncSource) -> &'static str {
    match source {
        SyncSource::Periodic => "periodic",
        SyncSource::Debounce => "debounce",
        SyncSource::Immediate => "immediate",
        SyncSource::Background => "background",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// 用于测试的同步动作。
    struct MockSyncAction {
        /// 总调用次数。
        calls: Arc<AtomicUsize>,
        /// 前 N 次调用返回失败。
        fail_before_success: usize,
    }

    impl MockSyncAction {
        fn new(fail_before_success: usize) -> (Arc<AtomicUsize>, Self) {
            let calls = Arc::new(AtomicUsize::new(0));
            let action = Self {
                calls: calls.clone(),
                fail_before_success,
            };
            (calls, action)
        }
    }

    impl SyncAction for MockSyncAction {
        fn run(&self, _source: SyncSource) -> BoxFuture<'static, Result<(), String>> {
            let count = self.calls.fetch_add(1, Ordering::SeqCst);
            if count < self.fail_before_success {
                Box::pin(async move { Err("mock failure".to_string()) })
            } else {
                Box::pin(async move { Ok(()) })
            }
        }
    }

    fn test_config() -> AutoSyncConfig {
        AutoSyncConfig {
            debounce_delay: Duration::from_millis(50),
            periodic_interval: Duration::from_secs(3600),
            max_retries: 0,
            retry_delay: Duration::from_millis(1),
        }
    }

    /// 防抖：多次触发 Debounce，最终只执行一次同步。
    #[tokio::test]
    async fn test_debounce_triggers_only_once() {
        let (calls, action) = MockSyncAction::new(0);
        let manager = AutoSyncManager::new(Arc::new(action), test_config());

        for _ in 0..5 {
            manager.trigger_debounce();
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // 等待防抖窗口过去（50ms + 缓冲）
        tokio::time::sleep(Duration::from_millis(120)).await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    /// Immediate 事件应取消当前防抖并立即执行。
    #[tokio::test]
    async fn test_immediate_cancels_debounce() {
        let (calls, action) = MockSyncAction::new(0);
        let manager = AutoSyncManager::new(Arc::new(action), test_config());

        manager.trigger_debounce();
        tokio::time::sleep(Duration::from_millis(25)).await;
        manager.trigger_immediate();

        // 立即同步应在很短时间内执行
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        // 等待原防抖窗口过去，不应再触发
        tokio::time::sleep(Duration::from_millis(80)).await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    /// 同步失败 2 次，第 3 次成功。
    #[tokio::test]
    async fn test_retry_until_success() {
        let (calls, action) = MockSyncAction::new(2);
        let config = AutoSyncConfig {
            debounce_delay: Duration::from_millis(50),
            periodic_interval: Duration::from_secs(3600),
            max_retries: 3,
            retry_delay: Duration::from_millis(5),
        };
        let manager = AutoSyncManager::new(Arc::new(action), config);

        manager.trigger_immediate();

        // 初始 + 2 次重试，最多等待约 75ms
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }
}
