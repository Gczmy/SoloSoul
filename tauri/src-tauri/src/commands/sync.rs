use crate::state::AppState;
use serde::Serialize;
use tauri::{Emitter, Manager, State};

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
use solosoul_sync::types::ApplyStats;

/// 同步冲突载荷 DTO：桌面端与移动端**共用**的序列化形状（P001）。
///
/// 底层 `Hlc.node_id: [u8; 16]` 在桌面端会被 serde 序列化为 `number[]`，
/// 而移动端旧实现用本地复刻 `MobileHlc`（`node_id: String`）——同一载荷在
/// 两个平台形状不同，Android 上前端任何读取 `node_id` 的逻辑都会拿到 string。
/// 统一经 hex 编码为字符串，并删除移动端复刻结构。
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct SyncConflictDto {
    pub table: String,
    pub id: String,
    pub local_hlc: ConflictHlc,
    pub remote_hlc: ConflictHlc,
    pub winner: String,
}

/// 从底层同步 crate 的 `ConflictRecord`（含原始 `Hlc`）转换为统一 DTO。
#[cfg(desktop)]
fn conflict_to_dto(c: &solosoul_sync::types::ConflictRecord) -> SyncConflictDto {
    SyncConflictDto {
        table: c.table.clone(),
        id: c.id.clone(),
        local_hlc: ConflictHlc {
            wall_time_ms: c.local_hlc.wall_time_ms,
            counter: c.local_hlc.counter as u64,
            node_id: hex::encode(c.local_hlc.node_id),
        },
        remote_hlc: ConflictHlc {
            wall_time_ms: c.remote_hlc.wall_time_ms,
            counter: c.remote_hlc.counter as u64,
            node_id: hex::encode(c.remote_hlc.node_id),
        },
        winner: c.winner.clone(),
    }
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
    /// 最近一次同步/在线的原始 unix 秒时间戳（未格式化的相对串）。
    /// 前端据此展示精确的「最近同步时间」。
    pub last_seen_ts: Option<i64>,
    /// 最近一次信任该设备的时间（unix 秒）。从未信任/已撤销时为 None。
    pub trusted_at: Option<i64>,
    /// 客户端类型：macos / windows / linux / android / ios / unknown。
    pub client_type: String,
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
    pub conflicts: Vec<SyncConflictDto>,
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
            conflicts: stats.conflicts.iter().map(conflict_to_dto).collect(),
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

/// 发现局域网内已知的同步对端并返回状态（`sync_get_status` 内部助手）。
///
/// 历史遗留：曾声明为 `#[tauri::command]`，但从未注册进 `register_sync_commands`、
/// 不在 capabilities allowlist、前端亦无调用方——纯内部复用（P138 附带发现收尾）。
async fn sync_discover(state: State<'_, AppState>) -> Result<SyncStatus, String> {
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
                last_seen_ts: p.last_seen_ts,
                trusted_at: p.trusted_at,
                client_type: p.client_type,
            })
            .collect(),
    })
}

/// 触发一次前台自动同步。
#[tauri::command]
pub async fn sync_trigger_foreground(state: State<'_, AppState>) -> Result<(), String> {
    state.device_auto_sync.trigger_foreground();
    Ok(())
}

/// 设置是否启用设备自动同步。
#[tauri::command]
pub async fn sync_set_auto_enabled(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<bool, String> {
    state.device_auto_sync.set_enabled(enabled);
    // P0#1: 开关持久化——AtomicBool 仅内存，重启即丢（用户感知"已打开"实际失效）。
    // 写入 ui_preferences.json（明文非敏感偏好，随 Vault 目录可移植），
    // AppState 启动时据此恢复。失败仅记录日志，不阻断开关操作。
    if let Ok(svc) = state.vault_service.read() {
        if let Err(e) =
            crate::commands::settings::write_auto_sync_pref(&state.handle, &svc, enabled)
        {
            tracing::warn!("[sync] persist auto_sync_enabled failed: {}", e);
        }
    }
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
#[tauri::command]
pub async fn sync_get_auto_status(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.device_auto_sync.enabled())
}

/// 获取「账户设置偏好（主题、主题色等 UI 外观）是否随设备同步」开关状态。
///
/// 由本机 VaultService 的原子开关驱动（Vault 未解锁时也可读）；
/// 该开关同步引擎在发送侧剥离 / 接收侧保留本机偏好。
#[tauri::command]
pub async fn sync_get_ui_prefs_sync(state: State<'_, AppState>) -> Result<bool, String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    Ok(svc.ui_prefs_sync_enabled())
}

/// 设置「账户设置偏好（主题、主题色等 UI 外观）是否随设备同步」开关。
///
/// 写入 VaultService 原子开关 + 持久化到 ui_preferences.json，
/// AppState 启动时据此恢复（与 auto_sync_enabled 同模式）。
#[tauri::command]
pub async fn sync_set_ui_prefs_sync(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<bool, String> {
    // set_ui_prefs_sync_enabled 是 &self（AtomicBool），读锁足够，
    // 避免写锁短暂阻塞正在进行的 vault 操作。
    {
        let svc = state
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        svc.set_ui_prefs_sync_enabled(enabled);
    }
    if let Ok(svc) = state.vault_service.read() {
        if let Err(e) =
            crate::commands::settings::write_ui_prefs_sync_pref(&state.handle, &svc, enabled)
        {
            tracing::warn!("[sync] persist ui_prefs_sync_enabled failed: {}", e);
        }
    }
    log_sync_action(
        &state,
        if enabled {
            "ui_prefs_sync_enabled"
        } else {
            "ui_prefs_sync_disabled"
        },
        None,
        None,
    );
    Ok(enabled)
}

/// 获取同步状态（发现对端 + 开关状态）。
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

/// 同步冲突 HLC（统一 DTO，桌面/移动共用）。
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
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
    // P040：字段缺失/类型错误不再静默吞掉——畸形 HLC 不应得到 wall_time_ms=0 的假值。
    let hlc: serde_json::Value = serde_json::from_str(s).map_err(|e| e.to_string())?;
    let wall_time_ms = hlc["wall_time_ms"]
        .as_u64()
        .ok_or_else(|| format!("HLC wall_time_ms 缺失或类型错误: {}", s))?;
    let counter = hlc["counter"]
        .as_u64()
        .ok_or_else(|| format!("HLC counter 缺失或类型错误: {}", s))?;
    let node_id = hlc["node_id"]
        .as_str()
        .ok_or_else(|| format!("HLC node_id 缺失或类型错误: {}", s))?
        .to_string();
    Ok(ConflictHlc {
        wall_time_ms,
        counter,
        node_id,
    })
}

async fn list_conflicts_impl(state: State<'_, AppState>) -> Result<Vec<ConflictSummary>, String> {
    let vault = crate::commands::vault_handle(&state)?;
    let rows = vault.list_sync_conflicts()?;
    let summaries = rows
        .into_iter()
        .map(|c| {
            // P040：列表兜底保留，但畸形数据必须留痕（warn），不静默
            let local_hlc = parse_hlc_json(&c.local_hlc_json).unwrap_or_else(|e| {
                tracing::warn!("[sync] 冲突 {} local HLC 解析失败: {}", c.id, e);
                ConflictHlc {
                    wall_time_ms: 0,
                    counter: 0,
                    node_id: String::new(),
                }
            });
            let remote_hlc = parse_hlc_json(&c.remote_hlc_json).unwrap_or_else(|e| {
                tracing::warn!("[sync] 冲突 {} remote HLC 解析失败: {}", c.id, e);
                ConflictHlc {
                    wall_time_ms: 0,
                    counter: 0,
                    node_id: String::new(),
                }
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
    // P013：进程内只保留一个 mDNS daemon。把 discovery 命令共用的 SharedDaemon
    // 注入 SyncService，避免 SyncManager 再自建一个 daemon 造成双 daemon 并存。
    let shared_daemon = if enable {
        let daemon_state = state
            .handle
            .state::<crate::commands::discovery::SharedDaemon>();
        let daemon_arc = daemon_state.get().await?;
        let guard = daemon_arc.lock().await;
        guard.as_ref().cloned()
    } else {
        None
    };
    state
        .sync_service
        .enable_with_daemon(enable, shared_daemon)
        .await?;
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
        // NSD 权限申请与注册都是 run_mobile_plugin 同步 IPC，会阻塞调用线程等待
        // Android 主线程响应；权限弹窗未响应时可能长时间阻塞。若在 tokio::spawn 的
        // async 任务里执行会占住 Tokio worker，多次开关同步后 worker 被占满，
        // sync_enable/sync_get_status 等命令全部排队，前端表现为“禁用后所有按钮卡住”。
        // 因此这里先在 async 上下文收集注册所需参数（短暂持 manager 锁，不会阻塞），
        // 再把同步 IPC 移入 spawn_blocking 阻塞线程池执行，命令本身立即返回。
        let app_state: AppState = app.state::<AppState>().inner().clone();
        let port = app_state.sync_service.listen_port().await;
        if port == 0 {
            return Ok(());
        }
        // 如果用户在此期间已关闭同步，则放弃注册，避免与 disable 逻辑竞合。
        if !app_state.sync_service.is_enabled().await {
            return Ok(());
        }
        let fingerprint = app_state
            .sync_service
            .local_fingerprint()
            .await
            .unwrap_or_default();
        let account_id = crate::commands::current_account_optional(&app_state).unwrap_or_default();
        let device_name = if fingerprint.is_empty() {
            format!("SoloSoul-{}", port)
        } else {
            format!("SoloSoul-{}", &fingerprint[..fingerprint.len().min(8)])
        };

        let app2 = app.clone();
        // 显式 drop JoinHandle 以分离任务（detach），命令立即返回
        std::mem::drop(tokio::task::spawn_blocking(move || {
            // 与 mdns_discover / 注销任务共用生命周期锁，串行化 NSD 操作
            let _guard = crate::commands::discovery::NSD_LIFECYCLE_LOCK
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            // 持锁后复查：快速「禁用→启用」连点时，本注册任务可能排在注销任务
            // 之后执行，若此时同步已被禁用则放弃注册，避免「已禁用但仍注册」。
            let still_enabled = tauri::async_runtime::block_on(async {
                app2.state::<crate::state::AppState>()
                    .sync_service
                    .is_enabled()
                    .await
            });
            if !still_enabled {
                return;
            }
            let handle = app2.state::<crate::nsd_plugin::NsdPluginHandle<tauri::Wry>>();
            if let Err(e) = handle.request_permissions() {
                tracing::warn!("NSD request_permissions failed: {}", e);
                return;
            }
            if let Err(e) = crate::commands::discovery::register_sync_service_blocking(
                &app2,
                device_name,
                port,
                account_id,
                fingerprint,
                solosoul_sync::local_client_type().to_string(),
            ) {
                tracing::warn!("Failed to register NSD sync service: {}", e);
                // NSD 注册失败时回滚同步状态，避免半开启。
                let app3 = app2.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app3.state::<crate::state::AppState>();
                    let _ = state.sync_service.enable(false).await;
                    let _ = state
                        .handle
                        .emit("sync-nsd-failed", serde_json::json!({ "error": e }));
                });
            }
        }));
    } else {
        // unregister_service 走 run_mobile_plugin，是同步 IPC，主线程繁忙（例如
        // 后台 NSD 注册任务正在申请权限）时可能阻塞；移入 blocking 线程执行，
        // 避免禁用同步的命令卡住前端。与注册任务共用生命周期锁，避免交错。
        let app2 = app.clone();
        // 显式 drop JoinHandle 以分离任务（detach），命令立即返回
        std::mem::drop(tokio::task::spawn_blocking(move || {
            let _guard = crate::commands::discovery::NSD_LIFECYCLE_LOCK
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            // 持锁后复查：快速「禁用→启用」连点时，本注销任务可能排在注册任务
            // 之后执行，若此时同步已重新启用则放弃注销，避免「已启用但未注册」。
            let still_disabled = !tauri::async_runtime::block_on(async {
                app2.state::<crate::state::AppState>()
                    .sync_service
                    .is_enabled()
                    .await
            });
            if !still_disabled {
                return;
            }
            let handle = app2.state::<crate::nsd_plugin::NsdPluginHandle<tauri::Wry>>();
            let _ = handle.unregister_service();
        }));
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

/// 返回本地监听地址（`host:port`），与移动端形状一致，供前端状态卡完整展示。
/// 未启用（端口 0）时返回空串，前端据此隐藏地址行。
#[tauri::command]
pub async fn sync_listen_addr(state: State<'_, AppState>) -> Result<String, String> {
    let port = state.sync_service.listen_port().await;
    if port == 0 {
        return Ok(String::new());
    }
    let host = local_display_ip()
        .await
        .unwrap_or_else(|| "127.0.0.1".to_string());
    Ok(format!("{}:{}", host, port))
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
    // P001：移动端 conflicts 与桌面端共用统一 DTO（Hlc.node_id 为 [u8;16]，
    // hex 编码为字符串），供前端冲突 UI 跨平台一致消费。
    let conflicts = stats
        .conflicts
        .iter()
        .map(|c| SyncConflictDto {
            table: c.table.clone(),
            id: c.id.clone(),
            local_hlc: ConflictHlc {
                wall_time_ms: c.local_hlc.wall_time_ms,
                counter: c.local_hlc.counter as u64,
                node_id: hex::encode(c.local_hlc.node_id),
            },
            remote_hlc: ConflictHlc {
                wall_time_ms: c.remote_hlc.wall_time_ms,
                counter: c.remote_hlc.counter as u64,
                node_id: hex::encode(c.remote_hlc.node_id),
            },
            winner: c.winner.clone(),
        })
        .collect::<Vec<_>>();
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
        conflicts,
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
    // 移动端 conflicts 现随会话回传（P1#10），直接以此为准。
    if !sync_result.conflicts.is_empty() {
        let _ = state.handle.emit(
            "sync-conflicts-updated",
            serde_json::json!({ "count": sync_result.conflicts.len() }),
        );
    }

    Ok(sync_result)
}

/// 信任/撤销信任一个同步对端。
#[tauri::command]
pub async fn sync_trust_peer(
    state: State<'_, AppState>,
    peer_node_id: String,
    trusted: bool,
    fingerprint: Option<String>,
) -> Result<(), String> {
    state
        .sync_service
        .trust_peer(peer_node_id.clone(), trusted, fingerprint)
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

/// 忘记一个同步对端。
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
        // 错误码供前端通过 resolveI18nPrefix 国际化（settings:sync_err_not_enabled）
        return Err("__SYNC_ERR__:not_enabled".to_string());
    }
    let fingerprint = state.sync_service.local_fingerprint().await?;
    let host = local_display_ip()
        .await
        .unwrap_or_else(|| "127.0.0.1".to_string());
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
    use solosoul_sync::types::{ApplyStats, ConflictRecord, TableStats};
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
            last_seen_ts: Some(1_717_240_000),
            trusted_at: Some(1_717_000_000),
            client_type: "macos".to_string(),
        };
        let json = serde_json::to_string(&peer).unwrap();
        // Structs serialize with camelCase keys for the frontend.
        assert!(json.contains("lastSeen"));
        assert!(json.contains("lastSeenTs"));
        assert!(json.contains("trustedAt"));
        assert!(json.contains("clientType"));
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
            last_seen_ts: None,
            trusted_at: None,
            client_type: "unknown".to_string(),
        };
        let json = serde_json::to_string(&peer).unwrap();
        assert!(json.contains("\"trusted\":false"));
        assert!(json.contains("\"lastSeen\":\"\""));
        assert!(json.contains("\"clientType\":\"unknown\""));
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
            last_seen_ts: Some(0),
            trusted_at: Some(0),
            client_type: "windows".to_string(),
        };
        let json = serde_json::to_string(&peer).unwrap();
        // SyncPeer serializes with camelCase keys for the frontend.
        assert!(json.contains("lastSeen"));
        assert!(!json.contains("last_seen"), "should not contain snake_case");
        assert!(!json.contains("last_seen_ts"));
    }

    /// P040：合法 HLC 正常解析，畸形数据必须报错而非得到 0 假值。
    #[test]
    fn test_parse_hlc_json() {
        // 正常
        let ok =
            parse_hlc_json(r#"{"wall_time_ms": 1000, "counter": 5, "node_id": "node-a"}"#).unwrap();
        assert_eq!(ok.wall_time_ms, 1000);
        assert_eq!(ok.counter, 5);
        assert_eq!(ok.node_id, "node-a");

        // 缺字段 / 类型错误 / 非法 JSON 均报错，不再静默生成 wall_time_ms=0 假 HLC
        for bad in [
            r#"{"wall_time_ms": 1000, "counter": 5}"#, // 缺 node_id
            r#"{"wall_time_ms": "x", "counter": 5, "node_id": "n"}"#, // 类型错误
            r#"{"wall_time_ms": 1000, "counter": "x", "node_id": "n"}"#,
            "not json",
            "{}",
        ] {
            assert!(parse_hlc_json(bad).is_err(), "畸形 HLC 应报错: {}", bad);
        }
    }
}
