use crate::state::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct SyncPeer {
    pub id: String,
    pub name: String,
    pub addr: String,
    pub last_seen: String,
}

#[derive(Serialize)]
pub struct SyncStatus {
    pub is_discovering: bool,
    pub connected_peers: Vec<SyncPeer>,
    pub sync_enabled: bool,
}

#[tauri::command]
pub async fn sync_discover(state: State<'_, AppState>) -> Result<SyncStatus, String> {
    let _ = state.vault_service.read().await;
    // For now, return empty status — full mDNS integration requires
    // running a background discovery service via the solosoul-sync crate.
    Ok(SyncStatus {
        is_discovering: false,
        connected_peers: vec![],
        sync_enabled: false,
    })
}

#[tauri::command]
pub async fn sync_get_status(state: State<'_, AppState>) -> Result<SyncStatus, String> {
    let _ = state.vault_service.read().await;
    Ok(SyncStatus {
        is_discovering: false,
        connected_peers: vec![],
        sync_enabled: false,
    })
}

#[tauri::command]
pub async fn sync_enable(state: State<'_, AppState>, _enable: bool) -> Result<(), String> {
    let _ = state.vault_service.read().await;
    // TODO: Start/stop background sync daemon
    Ok(())
}

#[tauri::command]
pub async fn sync_with_device(
    state: State<'_, AppState>,
    _device_id: String,
) -> Result<String, String> {
    let _ = state.vault_service.read().await;
    // TODO: Initiate Noise handshake and CRDT sync
    Ok("sync_initiated".to_string())
}
