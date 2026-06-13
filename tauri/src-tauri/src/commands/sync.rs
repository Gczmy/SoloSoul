use crate::state::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct SyncPeer {
    pub id: String,
    pub name: String,
    pub addr: String,
    pub fingerprint: String,
    pub trusted: bool,
    pub last_seen: String,
}

#[derive(Serialize)]
pub struct SyncStatus {
    pub is_discovering: bool,
    pub sync_enabled: bool,
    pub local_fingerprint: String,
    pub connected_peers: Vec<SyncPeer>,
}

#[tauri::command]
pub async fn sync_discover(state: State<'_, AppState>) -> Result<SyncStatus, String> {
    let peers = state.sync_service.known_peers().await?;
    let local_fingerprint = state.sync_service.local_fingerprint().await?;
    Ok(SyncStatus {
        is_discovering: state.sync_service.is_enabled().await,
        sync_enabled: state.sync_service.is_enabled().await,
        local_fingerprint,
        connected_peers: peers
            .into_iter()
            .map(|p| SyncPeer {
                id: p.node_id,
                name: p.name,
                addr: p.addr,
                fingerprint: p.fingerprint,
                trusted: p.trusted,
                last_seen: p.last_seen,
            })
            .collect(),
    })
}

#[tauri::command]
pub async fn sync_get_status(state: State<'_, AppState>) -> Result<SyncStatus, String> {
    sync_discover(state).await
}

#[tauri::command]
pub async fn sync_enable(state: State<'_, AppState>, enable: bool) -> Result<(), String> {
    state.sync_service.enable(enable).await
}

#[tauri::command]
pub async fn sync_with_device(
    state: State<'_, AppState>,
    device_id: String,
) -> Result<String, String> {
    state.sync_service.sync_with_device(device_id).await
}

#[tauri::command]
pub async fn sync_trust_peer(
    state: State<'_, AppState>,
    peer_node_id: String,
    trusted: bool,
) -> Result<(), String> {
    state.sync_service.trust_peer(peer_node_id, trusted).await
}

#[tauri::command]
pub async fn sync_forget_peer(
    state: State<'_, AppState>,
    peer_node_id: String,
) -> Result<(), String> {
    state.sync_service.forget_peer(peer_node_id).await
}
