use crate::state::AppState;
use serde::Serialize;
#[cfg(mobile)]
use tauri::Manager;
use tauri::{Emitter, State};

/// 记录同步相关操作日志。Vault 未解锁时静默跳过（同步服务本身不依赖 Vault）。
fn log_sync_action(
    state: &State<'_, AppState>,
    action: &str,
    entity_name: Option<&str>,
    details: Option<&str>,
) {
    let Some(account_id) = crate::commands::current_account_optional(state) else {
        return;
    };
    let Ok(vault) = crate::commands::vault_handle(state) else {
        return;
    };
    let _ = vault.log_structured(
        action,
        "sync",
        Some(&account_id),
        entity_name,
        "user",
        details,
    );
}

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
#[serde(rename_all = "camelCase")]
pub struct SyncPeer {
    pub id: String,
    pub name: String,
    pub addr: String,
    pub fingerprint: String,
    pub trusted: bool,
    pub last_seen: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub is_discovering: bool,
    pub sync_enabled: bool,
    pub auto_sync_enabled: bool,
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
        auto_sync_enabled: state.device_auto_sync.enabled(),
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
        auto_sync_enabled: state.device_auto_sync.enabled(),
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

/// 触发一次前台自动同步。
#[cfg(desktop)]
#[tauri::command]
pub async fn sync_trigger_foreground(state: State<'_, AppState>) -> Result<(), String> {
    state.device_auto_sync.trigger_foreground();
    Ok(())
}

#[cfg(mobile)]
#[tauri::command]
pub async fn sync_trigger_foreground(state: State<'_, AppState>) -> Result<(), String> {
    state.device_auto_sync.trigger_foreground();
    Ok(())
}

/// 设置是否启用设备自动同步。
#[cfg(desktop)]
#[tauri::command]
pub async fn sync_set_auto_enabled(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<bool, String> {
    state.device_auto_sync.set_enabled(enabled);
    log_sync_action(
        &state,
        if enabled {
            "auto_sync_enabled"
        } else {
            "auto_sync_disabled"
        },
        None,
        None,
    );
    Ok(enabled)
}

#[cfg(mobile)]
#[tauri::command]
pub async fn sync_set_auto_enabled(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<bool, String> {
    state.device_auto_sync.set_enabled(enabled);
    log_sync_action(
        &state,
        if enabled {
            "auto_sync_enabled"
        } else {
            "auto_sync_disabled"
        },
        None,
        None,
    );
    Ok(enabled)
}

/// 获取设备自动同步开关状态。
#[cfg(desktop)]
#[tauri::command]
pub async fn sync_get_auto_status(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.device_auto_sync.enabled())
}

#[cfg(mobile)]
#[tauri::command]
pub async fn sync_get_auto_status(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.device_auto_sync.enabled())
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

/// 同步冲突摘要，前端列表使用。
#[derive(Serialize)]
pub struct ConflictSummary {
    pub id: String,
    pub table: String,
    pub record_id: String,
    pub local_hlc: ConflictHlc,
    pub remote_hlc: ConflictHlc,
    pub winner: String,
    pub created_at: String,
}

/// 同步冲突 HLC。
#[derive(Serialize)]
pub struct ConflictHlc {
    pub wall_time_ms: u64,
    pub counter: u64,
    pub node_id: String,
}

/// 同步冲突详情，前端 Diff 使用。
#[derive(Serialize)]
pub struct ConflictDetail {
    pub id: String,
    pub table: String,
    pub record_id: String,
    pub local_hlc: ConflictHlc,
    pub remote_hlc: ConflictHlc,
    pub local_data: serde_json::Value,
    pub remote_data: serde_json::Value,
    pub remote_deleted: bool,
    pub winner: String,
    pub created_at: String,
}

fn parse_hlc_json(s: &str) -> Result<ConflictHlc, String> {
    let hlc: serde_json::Value = serde_json::from_str(s).map_err(|e| e.to_string())?;
    Ok(ConflictHlc {
        wall_time_ms: hlc["wall_time_ms"].as_u64().unwrap_or(0),
        counter: hlc["counter"].as_u64().unwrap_or(0),
        node_id: hlc["node_id"].as_str().unwrap_or("").to_string(),
    })
}

async fn list_conflicts_impl(state: State<'_, AppState>) -> Result<Vec<ConflictSummary>, String> {
    let vault = crate::commands::vault_handle(&state)?;
    let rows = vault.list_sync_conflicts()?;
    let summaries = rows
        .into_iter()
        .map(|c| {
            let local_hlc = parse_hlc_json(&c.local_hlc_json).unwrap_or(ConflictHlc {
                wall_time_ms: 0,
                counter: 0,
                node_id: String::new(),
            });
            let remote_hlc = parse_hlc_json(&c.remote_hlc_json).unwrap_or(ConflictHlc {
                wall_time_ms: 0,
                counter: 0,
                node_id: String::new(),
            });
            ConflictSummary {
                id: c.id,
                table: c.table_name,
                record_id: c.record_id,
                local_hlc,
                remote_hlc,
                winner: c.winner,
                created_at: c.created_at,
            }
        })
        .collect();
    Ok(summaries)
}

/// 获取当前所有未解决的同步冲突摘要。
#[cfg(desktop)]
#[tauri::command]
pub async fn sync_list_conflicts(
    state: State<'_, AppState>,
) -> Result<Vec<ConflictSummary>, String> {
    list_conflicts_impl(state).await
}

#[cfg(mobile)]
#[tauri::command]
pub async fn sync_list_conflicts(
    state: State<'_, AppState>,
) -> Result<Vec<ConflictSummary>, String> {
    list_conflicts_impl(state).await
}

async fn get_conflict_detail_impl(
    state: State<'_, AppState>,
    conflict_id: String,
) -> Result<ConflictDetail, String> {
    let vault = crate::commands::vault_handle(&state)?;
    let c = vault
        .get_sync_conflict(&conflict_id)?
        .ok_or("Conflict not found")?;
    let local_hlc = parse_hlc_json(&c.local_hlc_json)?;
    let remote_hlc = parse_hlc_json(&c.remote_hlc_json)?;
    let remote_data: serde_json::Value =
        serde_json::from_str(&c.remote_data_json).map_err(|e| e.to_string())?;
    let local_data: serde_json::Value =
        if !c.local_data_json.is_empty() && c.local_data_json != "{}" {
            serde_json::from_str(&c.local_data_json).map_err(|e| e.to_string())?
        } else {
            match c.table_name.as_str() {
                "profiles" => {
                    if let Some(p) = vault.load_profile(&c.record_id)? {
                        serde_json::to_value(&p).unwrap_or(serde_json::Value::Null)
                    } else {
                        serde_json::Value::Null
                    }
                }
                "objects" => {
                    if let Some(obj) = vault.load_object(&c.record_id)? {
                        serde_json::to_value(&obj).unwrap_or(serde_json::Value::Null)
                    } else {
                        serde_json::Value::Null
                    }
                }
                "user_templates" => {
                    if let Some(tpl) = vault.load_user_template(&c.record_id)? {
                        serde_json::to_value(&tpl).unwrap_or(serde_json::Value::Null)
                    } else {
                        serde_json::Value::Null
                    }
                }
                "trash_items" => {
                    if let Some(item) = vault.get_trash_item(&c.record_id)? {
                        serde_json::json!(item)
                    } else {
                        serde_json::Value::Null
                    }
                }
                _ => serde_json::Value::Null,
            }
        };
    Ok(ConflictDetail {
        id: c.id,
        table: c.table_name,
        record_id: c.record_id,
        local_hlc,
        remote_hlc,
        remote_deleted: c.remote_deleted,
        local_data,
        remote_data,
        winner: c.winner,
        created_at: c.created_at,
    })
}

/// 获取单个同步冲突详情。
#[cfg(desktop)]
#[tauri::command]
pub async fn sync_get_conflict_detail(
    state: State<'_, AppState>,
    conflict_id: String,
) -> Result<ConflictDetail, String> {
    get_conflict_detail_impl(state, conflict_id).await
}

#[cfg(mobile)]
#[tauri::command]
pub async fn sync_get_conflict_detail(
    state: State<'_, AppState>,
    conflict_id: String,
) -> Result<ConflictDetail, String> {
    get_conflict_detail_impl(state, conflict_id).await
}

async fn resolve_conflict_impl(
    state: State<'_, AppState>,
    conflict_id: String,
    strategy: String,
) -> Result<bool, String> {
    let vault = crate::commands::vault_handle(&state)?;
    let applied_remote = vault.resolve_sync_conflict(&conflict_id, &strategy)?;
    log_sync_action(
        &state,
        "sync_conflict_resolved",
        Some(&conflict_id),
        Some(&strategy),
    );
    Ok(applied_remote)
}

/// 按策略解决同步冲突。
#[cfg(desktop)]
#[tauri::command]
pub async fn sync_resolve_conflict(
    state: State<'_, AppState>,
    conflict_id: String,
    strategy: String,
) -> Result<bool, String> {
    resolve_conflict_impl(state, conflict_id, strategy).await
}

#[cfg(mobile)]
#[tauri::command]
pub async fn sync_resolve_conflict(
    state: State<'_, AppState>,
    conflict_id: String,
    strategy: String,
) -> Result<bool, String> {
    resolve_conflict_impl(state, conflict_id, strategy).await
}

#[cfg(desktop)]
#[tauri::command]
pub async fn sync_enable(state: State<'_, AppState>, enable: bool) -> Result<(), String> {
    state.sync_service.enable(enable).await?;
    log_sync_action(
        &state,
        if enable {
            "sync_enabled"
        } else {
            "sync_disabled"
        },
        None,
        None,
    );
    Ok(())
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
        // NSD 权限申请与注册可能阻塞命令（例如权限弹窗未立即返回），
        // 放在后台任务中执行，避免前端陷入永久“加载中”。
        let app = app.clone();
        // 克隆一份拥有的 AppState，使其可以在 'static 后台任务中使用。
        let app_state: AppState = app.state::<AppState>().inner().clone();
        tokio::spawn(async move {
            let handle = app.state::<crate::nsd_plugin::NsdPluginHandle<tauri::Wry>>();
            if let Err(e) = handle.request_permissions() {
                tracing::warn!("NSD request_permissions failed: {}", e);
                return;
            }
            let port = app_state.sync_service.listen_port().await;
            if port == 0 {
                return;
            }
            // 如果用户在此期间已关闭同步，则放弃注册，避免与 disable 逻辑竞合。
            if !app_state.sync_service.is_enabled().await {
                return;
            }
            let fingerprint = app_state
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
                tracing::warn!("Failed to register NSD sync service: {}", e);
                // NSD 注册失败时回滚同步状态，避免半开启。
                let _ = app_state.sync_service.enable(false).await;
                let _ = app_state
                    .handle
                    .emit("sync-nsd-failed", serde_json::json!({ "error": e }));
            }
        });
    } else {
        let handle = app.state::<crate::nsd_plugin::NsdPluginHandle<tauri::Wry>>();
        let _ = handle.unregister_service();
    }
    log_sync_action(
        &state,
        if enable {
            "sync_enabled"
        } else {
            "sync_disabled"
        },
        None,
        None,
    );
    Ok(())
}

#[cfg(mobile)]
#[tauri::command]
pub async fn sync_listen_port(state: State<'_, AppState>) -> Result<u16, String> {
    Ok(state.sync_service.listen_port().await)
}

#[cfg(desktop)]
#[tauri::command]
pub async fn sync_listen_port(state: State<'_, AppState>) -> Result<u16, String> {
    // 桌面端同样返回实际监听端口，方便用户在自动发现失败时通过 host:port 手动连接。
    Ok(state.sync_service.listen_port().await)
}

#[cfg(desktop)]
#[tauri::command]
pub async fn sync_with_device(
    state: State<'_, AppState>,
    device_id: String,
) -> Result<SyncResult, String> {
    let result = state
        .sync_service
        .sync_with_device(device_id.clone())
        .await?;
    let mut sync_result = SyncResult::from(&result.data);
    if !result.attachments.errors.is_empty() {
        sync_result.summary = format!(
            "{}; attachment errors: {}",
            sync_result.summary,
            result.attachments.errors.join("; ")
        );
    }
    let details = serde_json::json!({
        "device_id": device_id,
        "summary": sync_result.summary,
    })
    .to_string();
    log_sync_action(&state, "sync_with_device", Some(&device_id), Some(&details));

    // 当同步产生冲突时，向前端发送事件通知，让 UI 显示冲突徽章。
    if !sync_result.conflicts.is_empty() {
        let _ = state.handle.emit(
            "sync-conflicts-updated",
            serde_json::json!({ "count": sync_result.conflicts.len() }),
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
    // 移动端手动构造 SyncResult，因为 From<&ApplyStats> 实现在桌面端。
    let result = state
        .sync_service
        .sync_with_device(device_id.clone())
        .await?;
    let stats = &result.data;
    let mut sync_result = SyncResult {
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
        conflicts: vec![],
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
    };
    if !result.attachments.errors.is_empty() {
        sync_result.summary = format!(
            "{}; attachment errors: {}",
            sync_result.summary,
            result.attachments.errors.join("; ")
        );
    }
    let details = serde_json::json!({
        "device_id": device_id,
        "summary": sync_result.summary,
    })
    .to_string();
    log_sync_action(&state, "sync_with_device", Some(&device_id), Some(&details));

    // 当同步产生冲突时，向前端发送事件通知，让 UI 显示冲突徽章。
    // 移动端 sync_result.conflicts 始终为空（移动端不回传 ConflictRecord），
    // 但通过查询 Vault 中的冲突表确认是否有新冲突。
    if let Ok(vault) = crate::commands::vault_handle(&state) {
        if let Ok(conflicts) = vault.list_sync_conflicts() {
            if !conflicts.is_empty() {
                let _ = state.handle.emit(
                    "sync-conflicts-updated",
                    serde_json::json!({ "count": conflicts.len() }),
                );
            }
        }
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
    state
        .sync_service
        .trust_peer(peer_node_id.clone(), trusted)
        .await?;
    log_sync_action(
        &state,
        if trusted {
            "sync_peer_trusted"
        } else {
            "sync_peer_revoked"
        },
        Some(&peer_node_id),
        None,
    );
    Ok(())
}

#[cfg(mobile)]
#[tauri::command]
pub async fn sync_trust_peer(
    state: State<'_, AppState>,
    peer_node_id: String,
    trusted: bool,
) -> Result<(), String> {
    state
        .sync_service
        .trust_peer(peer_node_id.clone(), trusted)
        .await?;
    log_sync_action(
        &state,
        if trusted {
            "sync_peer_trusted"
        } else {
            "sync_peer_revoked"
        },
        Some(&peer_node_id),
        None,
    );
    Ok(())
}

#[cfg(desktop)]
#[tauri::command]
pub async fn sync_forget_peer(
    state: State<'_, AppState>,
    peer_node_id: String,
) -> Result<(), String> {
    state.sync_service.forget_peer(peer_node_id.clone()).await?;
    log_sync_action(&state, "sync_peer_forgotten", Some(&peer_node_id), None);
    Ok(())
}

#[cfg(mobile)]
#[tauri::command]
pub async fn sync_forget_peer(
    state: State<'_, AppState>,
    peer_node_id: String,
) -> Result<(), String> {
    state.sync_service.forget_peer(peer_node_id.clone()).await?;
    log_sync_action(&state, "sync_peer_forgotten", Some(&peer_node_id), None);
    Ok(())
}

/// 尝试获取一个适合展示给用户的本地非回环 IPv4 地址。
/// 桌面端优先通过外联 UDP 获得路由选中的地址（不发送任何数据包）；
/// 移动端跳过 UDP 连接（Android 上网络不可达时可能阻塞），仅枚举本地网卡。
async fn local_display_ip() -> Option<String> {
    // 桌面端优先通过外联 UDP 获得路由选中的地址
    #[cfg(desktop)]
    {
        if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
            if socket.connect("8.8.8.8:80").is_ok() {
                if let Ok(local) = socket.local_addr() {
                    if let std::net::IpAddr::V4(v4) = local.ip() {
                        if !v4.is_loopback() {
                            return Some(v4.to_string());
                        }
                    }
                }
            }
        }
    }

    // 移动端：跳过 UDP 连接，使用 tokio::time::timeout 防止阻塞
    #[cfg(mobile)]
    {
        let result = tokio::time::timeout(std::time::Duration::from_secs(3), async {
            // 枚举本地网卡
            if let Ok(std::net::IpAddr::V4(v4)) = local_ip_address::local_ip() {
                if !v4.is_loopback() {
                    return Some(v4.to_string());
                }
            }
            None
        })
        .await
        .unwrap_or(None);

        if result.is_some() {
            return result;
        }
    }

    // 所有平台 fallback：枚举本地网卡
    if let Ok(std::net::IpAddr::V4(v4)) = local_ip_address::local_ip() {
        if !v4.is_loopback() {
            return Some(v4.to_string());
        }
    }
    None
}

/// 生成供其他设备扫描以建立同步的二维码 payload。
/// Payload 格式：{"t":"sync","a":"host:port","f":"fingerprint","n":"deviceName"}
#[tauri::command]
pub async fn sync_generate_qr_payload(state: State<'_, AppState>) -> Result<String, String> {
    let port = state.sync_service.listen_port().await;
    if port == 0 {
        return Err("Sync is not enabled or listen port is not ready".to_string());
    }
    let fingerprint = state.sync_service.local_fingerprint().await?;
    let host = local_display_ip().await.unwrap_or_else(|| "127.0.0.1".to_string());
    let device_name = if fingerprint.is_empty() {
        format!("SoloSoul-{}", port)
    } else {
        format!("SoloSoul-{}", &fingerprint[..fingerprint.len().min(8)])
    };
    let payload = serde_json::json!({
        "t": "sync",
        "a": format!("{}:{}", host, port),
        "f": fingerprint,
        "n": device_name,
    });
    Ok(payload.to_string())
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
        // Structs serialize with camelCase keys for the frontend.
        assert!(json.contains("lastSeen"));
        assert!(json.contains("\"node-1\""));
    }

    #[test]
    fn test_sync_status_serialization() {
        let status = SyncStatus {
            is_discovering: true,
            sync_enabled: false,
            auto_sync_enabled: false,
            local_fingerprint: "fp-001".to_string(),
            connected_peers: vec![],
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("isDiscovering"));
        assert!(json.contains("syncEnabled"));
        assert!(json.contains("autoSyncEnabled"));
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
        assert!(json.contains("\"lastSeen\":\"\""));
    }

    #[test]
    fn test_sync_peer_serialization_matches_camel_case() {
        let peer = SyncPeer {
            id: "p1".to_string(),
            name: "Device".to_string(),
            addr: "0.0.0.0:0".to_string(),
            fingerprint: "fp".to_string(),
            trusted: true,
            last_seen: "now".to_string(),
        };
        let json = serde_json::to_string(&peer).unwrap();
        // SyncPeer serializes with camelCase keys for the frontend.
        assert!(json.contains("lastSeen"));
        assert!(!json.contains("last_seen"), "should not contain snake_case");
    }
}
