//! 自动同步调度器。
//!
//! `AutoSyncManager` 是 Android SAF 自动同步的唯一调度入口，负责将
//! 关键里程碑同步、写操作防抖同步和切后台同步排队为单任务执行，
//! 避免多个 `sync_to_remote()` 并发运行。

use crate::attachment_import_plugin::AttachmentImportPluginHandle;
use solosoul_core::VaultService;
use std::path::Path;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;

/// 自动同步触发事件。
pub enum SyncEvent {
    /// 立即同步，取消当前防抖定时器。
    Immediate,
    /// 防抖同步，30 秒内无新事件才执行。
    Debounce,
    /// 应用切后台 / 失去焦点，立即同步。
    Background,
}

enum AutoSyncState {
    Idle,
    Scheduled(tokio::time::Instant),
    Running,
}

/// 自动同步管理器。
///
/// 通过内部 mpsc 通道接收事件，在独立任务中调度同步。
#[derive(Clone)]
pub struct AutoSyncManager {
    tx: mpsc::Sender<SyncEvent>,
}

impl AutoSyncManager {
    /// 创建并启动自动同步任务。
    pub fn new(vault_service: Arc<RwLock<VaultService>>, app_handle: AppHandle) -> Self {
        let (tx, rx) = mpsc::channel(64);
        let manager = Self { tx };
        manager.start_loop(rx, vault_service, app_handle);
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
        vault_service: Arc<RwLock<VaultService>>,
        app_handle: AppHandle,
    ) {
        const DEBOUNCE_DELAY: Duration = Duration::from_secs(30);
        const PERIODIC_INTERVAL: Duration = Duration::from_secs(30);

        tauri::async_runtime::spawn(async move {
            let mut state = AutoSyncState::Idle;
            let mut deadline: Option<tokio::time::Instant> = None;
            let mut interval = tokio::time::interval(PERIODIC_INTERVAL);

            loop {
                match state {
                    AutoSyncState::Idle => {
                        tokio::select! {
                            event = rx.recv() => match event {
                                Some(SyncEvent::Immediate | SyncEvent::Background) => {
                                    state = AutoSyncState::Running;
                                }
                                Some(SyncEvent::Debounce) => {
                                    let d = tokio::time::Instant::now() + DEBOUNCE_DELAY;
                                    deadline = Some(d);
                                    state = AutoSyncState::Scheduled(d);
                                }
                                None => break,
                            },
                            _ = interval.tick() => {
                                state = AutoSyncState::Running;
                            }
                        }
                    }
                    AutoSyncState::Scheduled(d) => {
                        tokio::select! {
                            event = rx.recv() => match event {
                                Some(SyncEvent::Immediate | SyncEvent::Background) => {
                                    deadline = None;
                                    state = AutoSyncState::Running;
                                }
                                Some(SyncEvent::Debounce) => {
                                    let new_deadline = tokio::time::Instant::now() + DEBOUNCE_DELAY;
                                    deadline = Some(new_deadline);
                                    state = AutoSyncState::Scheduled(new_deadline);
                                }
                                None => break,
                            },
                            _ = tokio::time::sleep_until(d) => {
                                deadline = None;
                                state = AutoSyncState::Running;
                            }
                            _ = interval.tick() => {
                                // Already scheduled, nothing to do.
                            }
                        }
                    }
                    AutoSyncState::Running => {
                        let _ = run_sync(&vault_service, &app_handle).await;
                        if let Some(d) = deadline.take() {
                            if d <= tokio::time::Instant::now() {
                                state = AutoSyncState::Running;
                            } else {
                                state = AutoSyncState::Scheduled(d);
                            }
                        } else {
                            state = AutoSyncState::Idle;
                        }
                    }
                }
            }
        });
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
pub async fn run_sync(
    vault_service: &Arc<RwLock<VaultService>>,
    app_handle: &AppHandle,
) -> Result<(), String> {
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

    app_handle
        .emit(
            "sync-progress",
            serde_json::json!({"phase": "sync_start", "current": 0, "total": 1}),
        )
        .ok();

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
            app_handle
                .emit(
                    "sync-progress",
                    serde_json::json!({"phase": "sync_complete", "current": 1, "total": 1}),
                )
                .ok();
            Ok(())
        }
        Err(e) => {
            app_handle
                .emit(
                    "sync-progress",
                    serde_json::json!({"phase": "error", "message": e.clone()}),
                )
                .ok();
            Err(e)
        }
    }
}
