use crate::state::AppState;
use serde::Serialize;
use tauri::State;

#[cfg(mobile)]
use crate::commands::{mobile_not_supported, mobile_not_supported_with};
#[cfg(desktop)]
use solosoul_sync::types::{ApplyStats, ConflictRecord};

// 移动端：提供与桌面端字段兼容的本地 ConflictRecord，使 SyncResult 签名不变。
#[cfg(mobile)]
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct ConflictRecord {
    pub table: String,
    pub id: String,
    pub local_hlc: MobileHlc,
    pub remote_hlc: MobileHlc,
    pub winner: String,
}

#[cfg(mobile)]
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct MobileHlc {
    pub wall_time_ms: u64,
    pub counter: u64,
    pub node_id: String,
}

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

#[derive(Serialize)]
pub struct SyncResult {
    pub summary: String,
    pub examined: u64,
    pub applied: u64,
    pub skipped: u64,
    pub conflicts: Vec<ConflictRecord>,
    pub per_table: Vec<TableResult>,
}

#[derive(Serialize)]
pub struct TableResult {
    pub table: String,
    pub examined: u64,
    pub applied: u64,
    pub skipped: u64,
}

#[cfg(desktop)]
impl From<&ApplyStats> for SyncResult {
    fn from(stats: &ApplyStats) -> Self {
        Self {
            summary: format!(
                "examined={}, applied={}, skipped={}, conflicts={}",
                stats.examined,
                stats.applied,
                stats.skipped,
                stats.conflicts.len()
            ),
            examined: stats.examined,
            applied: stats.applied,
            skipped: stats.skipped,
            conflicts: stats.conflicts.clone(),
            per_table: stats
                .per_table
                .iter()
                .map(|(table, s)| TableResult {
                    table: table.clone(),
                    examined: s.examined,
                    applied: s.applied,
                    skipped: s.skipped,
                })
                .collect(),
        }
    }
}

#[cfg(desktop)]
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

#[cfg(mobile)]
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

#[cfg(desktop)]
#[tauri::command]
pub async fn sync_get_status(state: State<'_, AppState>) -> Result<SyncStatus, String> {
    sync_discover(state).await
}

#[cfg(mobile)]
#[tauri::command]
pub async fn sync_get_status(state: State<'_, AppState>) -> Result<SyncStatus, String> {
    sync_discover(state).await
}

#[cfg(desktop)]
#[tauri::command]
pub async fn sync_enable(state: State<'_, AppState>, enable: bool) -> Result<(), String> {
    state.sync_service.enable(enable).await
}

#[cfg(mobile)]
#[tauri::command]
pub async fn sync_enable(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    enable: bool,
) -> Result<(), String> {
    state.sync_service.enable(enable).await?;

    // 移动端：启用同步后自动注册 NSD 服务，让桌面端可以发现本机；
    // 关闭同步时注销 NSD 服务。
    if enable {
        let handle = app.state::<crate::nsd_plugin::NsdPluginHandle<tauri::Wry>>();
        handle.request_permissions()?;
        let port = state.sync_service.listen_port().await;
        if port != 0 {
            let fingerprint = state
                .sync_service
                .local_fingerprint()
                .await
                .unwrap_or_default();
            let device_name = if fingerprint.is_empty() {
                format!("SoloSoul-{}", port)
            } else {
                format!("SoloSoul-{}", &fingerprint[..fingerprint.len().min(8)])
            };
            if let Err(e) =
                crate::commands::discovery::register_sync_service(&app, device_name, port).await
            {
                // NSD 注册失败时回滚同步状态，避免半开启。
                let _ = state.sync_service.enable(false).await;
                return Err(format!(
                    "Failed to enable sync: NSD advertise failed: {}",
                    e
                ));
            }
        }
    } else {
        let handle = app.state::<crate::nsd_plugin::NsdPluginHandle<tauri::Wry>>();
        let _ = handle.unregister_service();
    }
    Ok(())
}

#[cfg(mobile)]
#[tauri::command]
pub async fn sync_listen_port(state: State<'_, AppState>) -> Result<u16, String> {
    Ok(state.sync_service.listen_port().await)
}

#[cfg(desktop)]
#[tauri::command]
pub async fn sync_listen_port(_state: State<'_, AppState>) -> Result<u16, String> {
    // 桌面端监听端口由 mDNS 服务信息直接提供，无需单独暴露。
    Ok(0)
}

#[cfg(desktop)]
#[tauri::command]
pub async fn sync_with_device(
    state: State<'_, AppState>,
    device_id: String,
) -> Result<SyncResult, String> {
    let result = state.sync_service.sync_with_device(device_id).await?;
    let mut sync_result = SyncResult::from(&result.data);
    if !result.attachments.errors.is_empty() {
        sync_result.summary = format!(
            "{}; attachment errors: {}",
            sync_result.summary,
            result.attachments.errors.join("; ")
        );
    }
    Ok(sync_result)
}

#[cfg(mobile)]
#[tauri::command]
pub async fn sync_with_device(
    state: State<'_, AppState>,
    device_id: String,
) -> Result<SyncResult, String> {
    let result = state.sync_service.sync_with_device(device_id).await?;
    let mut sync_result = SyncResult::from(&result.data);
    if !result.attachments.errors.is_empty() {
        sync_result.summary = format!(
            "{}; attachment errors: {}",
            sync_result.summary,
            result.attachments.errors.join("; ")
        );
    }
    Ok(sync_result)
}

#[cfg(desktop)]
#[tauri::command]
pub async fn sync_trust_peer(
    state: State<'_, AppState>,
    peer_node_id: String,
    trusted: bool,
) -> Result<(), String> {
    state.sync_service.trust_peer(peer_node_id, trusted).await
}

#[cfg(mobile)]
#[tauri::command]
pub async fn sync_trust_peer(
    state: State<'_, AppState>,
    peer_node_id: String,
    trusted: bool,
) -> Result<(), String> {
    state.sync_service.trust_peer(peer_node_id, trusted).await
}

#[cfg(desktop)]
#[tauri::command]
pub async fn sync_forget_peer(
    state: State<'_, AppState>,
    peer_node_id: String,
) -> Result<(), String> {
    state.sync_service.forget_peer(peer_node_id).await
}

#[cfg(mobile)]
#[tauri::command]
pub async fn sync_forget_peer(
    state: State<'_, AppState>,
    peer_node_id: String,
) -> Result<(), String> {
    state.sync_service.forget_peer(peer_node_id).await
}

#[cfg(all(test, desktop))]
mod tests {
    use super::*;
    use solosoul_sync::types::{ApplyStats, TableStats};
    use std::collections::HashMap;

    #[test]
    fn test_sync_peer_serialization() {
        let peer = SyncPeer {
            id: "node-1".to_string(),
            name: "My Mac".to_string(),
            addr: "192.168.1.5:42069".to_string(),
            fingerprint: "ab:cd:ef:01".to_string(),
            trusted: true,
            last_seen: "2024-06-01T12:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&peer).unwrap();
        // Structs use default serde (snake_case, no rename_all)
        assert!(json.contains("last_seen"));
        assert!(json.contains("\"node-1\""));
    }

    #[test]
    fn test_sync_status_serialization() {
        let status = SyncStatus {
            is_discovering: true,
            sync_enabled: false,
            local_fingerprint: "fp-001".to_string(),
            connected_peers: vec![],
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("is_discovering"));
        assert!(json.contains("sync_enabled"));
    }

    #[test]
    fn test_table_result_serialization() {
        let tr = TableResult {
            table: "objects".to_string(),
            examined: 100,
            applied: 50,
            skipped: 50,
        };
        let json = serde_json::to_string(&tr).unwrap();
        assert!(json.contains("\"examined\":100"));
        assert!(json.contains("\"table\":\"objects\""));
    }

    #[test]
    fn test_sync_result_from_apply_stats_empty() {
        let stats = ApplyStats {
            examined: 0,
            applied: 0,
            skipped: 0,
            errors: vec![],
            conflicts: vec![],
            per_table: HashMap::new(),
        };
        let result = SyncResult::from(&stats);
        assert_eq!(result.examined, 0);
        assert_eq!(result.applied, 0);
        assert!(result.conflicts.is_empty());
        assert!(result.per_table.is_empty());
        assert!(result.summary.contains("examined=0"));
    }

    #[test]
    fn test_sync_result_from_apply_stats_with_data() {
        let mut per_table = HashMap::new();
        per_table.insert(
            "profiles".to_string(),
            TableStats {
                examined: 10,
                applied: 5,
                skipped: 5,
            },
        );
        let stats = ApplyStats {
            examined: 10,
            applied: 5,
            skipped: 5,
            errors: vec![],
            conflicts: vec![ConflictRecord {
                table: "profiles".to_string(),
                id: "obj-1".to_string(),
                local_hlc: solosoul_sync::hlc::Hlc::new(0, 0, "test"),
                remote_hlc: solosoul_sync::hlc::Hlc::new(1, 0, "test"),
                winner: "local".to_string(),
            }],
            per_table,
        };
        let result = SyncResult::from(&stats);
        assert_eq!(result.examined, 10);
        assert_eq!(result.applied, 5);
        assert_eq!(result.conflicts.len(), 1);
        assert_eq!(result.per_table.len(), 1);
        assert_eq!(result.per_table[0].table, "profiles");
        assert_eq!(result.per_table[0].applied, 5);
        assert!(result.summary.contains("conflicts=1"));
    }

    #[test]
    fn test_sync_result_serialization() {
        let result = SyncResult {
            summary: "examined=1, applied=1, skipped=0, conflicts=0".to_string(),
            examined: 1,
            applied: 1,
            skipped: 0,
            conflicts: vec![],
            per_table: vec![TableResult {
                table: "objects".to_string(),
                examined: 1,
                applied: 1,
                skipped: 0,
            }],
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"examined\":1"));
        assert!(json.contains("\"summary\""));
        assert!(json.contains("per_table"));
    }

    #[test]
    fn test_sync_peer_untrusted_serialization() {
        let peer = SyncPeer {
            id: "n2".to_string(),
            name: "Phone".to_string(),
            addr: "10.0.0.2:42069".to_string(),
            fingerprint: "02:03:04".to_string(),
            trusted: false,
            last_seen: String::new(),
        };
        let json = serde_json::to_string(&peer).unwrap();
        assert!(json.contains("\"trusted\":false"));
        assert!(json.contains("\"last_seen\":\"\""));
    }

    #[test]
    fn test_sync_peer_serialization_matches_snake_case() {
        let peer = SyncPeer {
            id: "p1".to_string(),
            name: "Device".to_string(),
            addr: "0.0.0.0:0".to_string(),
            fingerprint: "fp".to_string(),
            trusted: true,
            last_seen: "now".to_string(),
        };
        let json = serde_json::to_string(&peer).unwrap();
        // Default serde uses snake_case (no rename_all on these structs)
        assert!(json.contains("last_seen"));
        assert!(!json.contains("lastSeen"), "should not contain camelCase");
    }
}
