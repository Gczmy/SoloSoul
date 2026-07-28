//! 设备间自动同步调度器（Device Auto Sync）。
//!
//! 负责在以下三类场景自动触发 Device Sync：
//! - 应用切回前台（Foreground）
//! - 本地数据发生变更（DataChange，防抖）
//! - 周期性触发（Periodic）
//!
//! 调度器保证同一时刻只有一个同步任务在执行；数据变更触发采用防抖策略，
//! 避免连续写操作产生大量同步请求。

use futures::future::BoxFuture;
use solosoul_core::VaultService;
use solosoul_sync::SyncService;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

/// 自动同步触发来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceSyncSource {
    Foreground,
    DataChange,
    Periodic,
}

/// 自动同步触发事件。
pub enum DeviceSyncEvent {
    Foreground,
    DataChange,
    Periodic,
}

impl DeviceSyncEvent {
    fn source(&self) -> DeviceSyncSource {
        match self {
            DeviceSyncEvent::Foreground => DeviceSyncSource::Foreground,
            DeviceSyncEvent::DataChange => DeviceSyncSource::DataChange,
            DeviceSyncEvent::Periodic => DeviceSyncSource::Periodic,
        }
    }
}

enum DeviceAutoSyncState {
    Idle,
    Scheduled(DeviceSyncSource, tokio::time::Instant),
    Running(DeviceSyncSource),
}

/// 设备自动同步配置。
#[derive(Clone)]
pub struct DeviceAutoSyncConfig {
    /// 是否启用自动同步。
    pub enabled: bool,
    /// 数据变更防抖延迟。
    pub debounce_delay: Duration,
    /// 周期性同步间隔。
    pub periodic_interval: Duration,
    /// 同步失败后的最大重试次数。
    pub max_retries: u32,
    /// 重试退避基准间隔。
    pub retry_delay: Duration,
}

impl DeviceAutoSyncConfig {
    pub fn production() -> Self {
        Self {
            enabled: false,
            debounce_delay: Duration::from_secs(10),
            periodic_interval: Duration::from_secs(60),
            max_retries: 2,
            retry_delay: Duration::from_millis(500),
        }
    }
}

impl Default for DeviceAutoSyncConfig {
    fn default() -> Self {
        Self::production()
    }
}

/// 可注入的同步动作，便于单元测试。
pub trait DeviceSyncAction: Send + Sync + 'static {
    fn run(&self, source: DeviceSyncSource) -> BoxFuture<'static, Result<(), String>>;
}

/// 设备自动同步管理器。
#[derive(Clone)]
pub struct DeviceAutoSyncManager {
    tx: mpsc::Sender<DeviceSyncEvent>,
    /// 运行时开关，可通过 `set_enabled` 在运行时开启/关闭自动同步。
    enabled: Arc<AtomicBool>,
}

impl DeviceAutoSyncManager {
    /// 创建并启动设备自动同步任务（生产入口）。
    pub fn new(
        sync_service: Arc<SyncService>,
        vault_service: Arc<RwLock<VaultService>>,
        app_handle: AppHandle,
    ) -> Self {
        let action = Arc::new(SyncServiceDeviceSyncAction::new(
            sync_service,
            vault_service,
            app_handle,
        ));
        Self::new_with_action(action, DeviceAutoSyncConfig::default())
    }

    /// 创建并启动设备自动同步任务（可注入同步动作与配置）。
    pub fn new_with_action(
        action: Arc<dyn DeviceSyncAction>,
        config: DeviceAutoSyncConfig,
    ) -> Self {
        let (tx, rx) = mpsc::channel(64);
        let enabled = Arc::new(AtomicBool::new(config.enabled));
        let manager = Self {
            tx,
            enabled: enabled.clone(),
        };
        manager.start_loop(rx, action, config, enabled);
        manager
    }

    /// 设置是否启用自动同步。
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }

    /// 当前是否启用自动同步。
    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// 触发一次前台同步。
    pub fn trigger_foreground(&self) {
        let _ = self.tx.try_send(DeviceSyncEvent::Foreground);
    }

    /// 触发一次数据变更同步（防抖）。
    pub fn trigger_data_change(&self) {
        let _ = self.tx.try_send(DeviceSyncEvent::DataChange);
    }

    /// 触发一次周期性同步。
    pub fn trigger_periodic(&self) {
        let _ = self.tx.try_send(DeviceSyncEvent::Periodic);
    }

    fn start_loop(
        &self,
        mut rx: mpsc::Receiver<DeviceSyncEvent>,
        action: Arc<dyn DeviceSyncAction>,
        config: DeviceAutoSyncConfig,
        enabled: Arc<AtomicBool>,
    ) {
        let fut = async move {
            let mut state = DeviceAutoSyncState::Idle;
            let mut interval = tokio::time::interval(config.periodic_interval);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            // 消耗 interval 启动时的立即 tick，避免启动后立刻触发一次同步。
            interval.tick().await;
            let mut retry_count: u32 = 0;

            loop {
                match state {
                    DeviceAutoSyncState::Idle => {
                        tokio::select! {
                            event = rx.recv() => match event {
                                Some(DeviceSyncEvent::Foreground | DeviceSyncEvent::Periodic) => {
                                    state = DeviceAutoSyncState::Running(event.unwrap().source());
                                }
                                Some(DeviceSyncEvent::DataChange) => {
                                    let deadline = tokio::time::Instant::now() + config.debounce_delay;
                                    state = DeviceAutoSyncState::Scheduled(DeviceSyncSource::DataChange, deadline);
                                }
                                None => break,
                            },
                            _ = interval.tick() => {
                                if enabled.load(Ordering::SeqCst) {
                                    state = DeviceAutoSyncState::Running(DeviceSyncSource::Periodic);
                                }
                            }
                        }
                    }
                    DeviceAutoSyncState::Scheduled(source, d) => {
                        tokio::select! {
                            event = rx.recv() => match event {
                                Some(DeviceSyncEvent::Foreground | DeviceSyncEvent::Periodic) => {
                                    state = DeviceAutoSyncState::Running(event.unwrap().source());
                                }
                                Some(DeviceSyncEvent::DataChange) => {
                                    let new_deadline = tokio::time::Instant::now() + config.debounce_delay;
                                    state = DeviceAutoSyncState::Scheduled(source, new_deadline);
                                }
                                None => break,
                            },
                            _ = tokio::time::sleep_until(d) => {
                                state = DeviceAutoSyncState::Running(source);
                            }
                            _ = interval.tick() => {
                                // Already scheduled, nothing to do.
                            }
                        }
                    }
                    DeviceAutoSyncState::Running(source) => {
                        let result = action.run(source).await;
                        match result {
                            Ok(()) => {
                                retry_count = 0;
                                state = DeviceAutoSyncState::Idle;
                            }
                            Err(_) => {
                                if retry_count < config.max_retries {
                                    retry_count += 1;
                                    let exponent = (retry_count - 1).min(10);
                                    let backoff = config.retry_delay * 2u32.pow(exponent);
                                    tokio::time::sleep(backoff).await;
                                    state = DeviceAutoSyncState::Running(source);
                                } else {
                                    retry_count = 0;
                                    state = DeviceAutoSyncState::Idle;
                                }
                            }
                        }
                    }
                }
            }
        };

        // 生产环境使用 tauri 的全局 async runtime，测试环境使用 tokio::spawn。
        #[cfg(test)]
        tokio::spawn(fut);
        #[cfg(not(test))]
        tauri::async_runtime::spawn(fut);
    }
}

/// 生产用同步动作：与所有受信且可访问的 peer 同步。
struct SyncServiceDeviceSyncAction {
    sync_service: Arc<SyncService>,
    vault_service: Arc<RwLock<VaultService>>,
    app_handle: AppHandle,
    running: Arc<AtomicBool>,
}

impl SyncServiceDeviceSyncAction {
    fn new(
        sync_service: Arc<SyncService>,
        vault_service: Arc<RwLock<VaultService>>,
        app_handle: AppHandle,
    ) -> Self {
        Self {
            sync_service,
            vault_service,
            app_handle,
            running: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl DeviceSyncAction for SyncServiceDeviceSyncAction {
    fn run(&self, source: DeviceSyncSource) -> BoxFuture<'static, Result<(), String>> {
        let is_unlocked = self
            .vault_service
            .read()
            .ok()
            .and_then(|svc| svc.get_vault_store())
            .is_some();
        if !is_unlocked {
            tracing::debug!("[DeviceAutoSync] vault is locked, skipping sync");
            return Box::pin(async { Ok(()) });
        }

        if self
            .running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Box::pin(async { Ok(()) });
        }

        let sync_service = self.sync_service.clone();
        let app_handle = self.app_handle.clone();
        let running = self.running.clone();
        let source_str = match source {
            DeviceSyncSource::Foreground => "foreground",
            DeviceSyncSource::DataChange => "data_change",
            DeviceSyncSource::Periodic => "periodic",
        };

        Box::pin(async move {
            let result = run_device_sync(&sync_service, &app_handle, source_str).await;
            running.store(false, Ordering::SeqCst);
            result
        })
    }
}

async fn run_device_sync(
    sync_service: &Arc<SyncService>,
    app_handle: &AppHandle,
    source: &str,
) -> Result<(), String> {
    // 仅在 device sync 已启用时执行。
    if !sync_service.is_enabled().await {
        return Ok(());
    }

    let peers = match sync_service.known_peers().await {
        Ok(peers) => peers,
        Err(e) => {
            tracing::warn!("[DeviceAutoSync] failed to list peers: {}", e);
            return Err(e);
        }
    };

    let targets: Vec<_> = peers
        .into_iter()
        .filter(|p| p.trusted && !p.addr.is_empty())
        .collect();

    if targets.is_empty() {
        tracing::debug!("[DeviceAutoSync] no trusted peers with known address, skipping");
        return Ok(());
    }

    app_handle
        .emit(
            "device-sync-auto-status",
            serde_json::json!({
                "phase": "sync_start",
                "source": source,
                "peer_count": targets.len(),
            }),
        )
        .ok();

    let mut last_error: Option<String> = None;
    for peer in targets {
        let peer_id = peer.node_id.clone();
        if let Err(e) = sync_service.sync_with_device(peer_id.clone()).await {
            tracing::warn!(
                "[DeviceAutoSync] sync with {} failed: {}",
                peer_id,
                e
            );
            last_error = Some(e);
            continue;
        }
        tracing::info!("[DeviceAutoSync] sync with {} completed", peer_id);
    }

    app_handle
        .emit(
            "device-sync-auto-status",
            serde_json::json!({
                "phase": last_error.as_ref().map(|_| "error").unwrap_or("sync_complete"),
                "source": source,
                "message": last_error,
            }),
        )
        .ok();

    if let Some(e) = last_error {
        return Err(e);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockDeviceSyncAction {
        calls: Arc<AtomicUsize>,
    }

    impl DeviceSyncAction for MockDeviceSyncAction {
        fn run(&self, _source: DeviceSyncSource) -> BoxFuture<'static, Result<(), String>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Ok(()) })
        }
    }

    fn test_config() -> DeviceAutoSyncConfig {
        DeviceAutoSyncConfig {
            enabled: true,
            debounce_delay: Duration::from_millis(50),
            periodic_interval: Duration::from_secs(3600),
            max_retries: 0,
            retry_delay: Duration::from_millis(1),
        }
    }

    #[tokio::test]
    async fn test_data_change_debounce() {
        let calls = Arc::new(AtomicUsize::new(0));
        let action = Arc::new(MockDeviceSyncAction {
            calls: calls.clone(),
        });
        let manager = DeviceAutoSyncManager::new_with_action(action, test_config());

        for _ in 0..5 {
            manager.trigger_data_change();
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        tokio::time::sleep(Duration::from_millis(120)).await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_foreground_triggers_immediately() {
        let calls = Arc::new(AtomicUsize::new(0));
        let action = Arc::new(MockDeviceSyncAction {
            calls: calls.clone(),
        });
        let manager = DeviceAutoSyncManager::new_with_action(action, test_config());

        manager.trigger_foreground();
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
