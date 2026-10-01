//! 单一跨平台命令入口；只转发已有实现，不改变同步执行、停机与权限流程。
use super::contracts::{DiscoveredDevice, RecoveryDiscoveredHost, SyncResult};
use crate::commands::{discovery, sync, vault_directory};
use crate::state::AppState;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn sync_enable(
    app: AppHandle,
    state: State<'_, AppState>,
    enable: bool,
) -> Result<(), String> {
    let _ = &app;
    #[cfg(desktop)]
    {
        sync::sync_enable(state, enable).await
    }
    #[cfg(mobile)]
    {
        sync::sync_enable(app, state, enable).await
    }
}
#[tauri::command]
pub async fn sync_with_device(
    state: State<'_, AppState>,
    device_id: String,
) -> Result<SyncResult, String> {
    sync::sync_with_device(state, device_id).await
}
#[tauri::command]
pub async fn mdns_discover(
    app: AppHandle,
    state: State<'_, AppState>,
    daemon: State<'_, discovery::SharedDaemon>,
    timeout_ms: u64,
) -> Result<Vec<DiscoveredDevice>, String> {
    let _ = (&app, &state);
    #[cfg(desktop)]
    {
        discovery::mdns_discover(state, daemon, timeout_ms).await
    }
    #[cfg(mobile)]
    {
        discovery::mdns_discover(app, daemon, timeout_ms).await
    }
}
#[tauri::command]
pub async fn recovery_discover_hosts(
    daemon: State<'_, discovery::SharedDaemon>,
    timeout_ms: u64,
) -> Result<Vec<RecoveryDiscoveredHost>, String> {
    discovery::recovery_discover_hosts(daemon, timeout_ms).await
}
#[tauri::command]
pub async fn vault_sync_to_remote(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    vault_directory::vault_sync_to_remote(app, state).await
}
#[tauri::command]
pub async fn vault_sync_from_remote(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    vault_directory::vault_sync_from_remote(app, state).await
}
#[tauri::command]
pub async fn vault_sync_background(state: State<'_, AppState>) -> Result<(), String> {
    vault_directory::vault_sync_background(state).await
}
