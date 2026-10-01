//! /sync 设备同步命令。
//!
//! 一次性会话实现 —— 每次调用都构造一个 `SyncManager`、单次
//! `start() → sync_with_peer() → stop()`。CLI 不维持后台常驻的 mDNS/TCP
//! listener 守护进程（"始终在线"同步请使用 GUI）。
//!
//! 子命令：
//! - `/sync status | list` —— 列出 vault 已持久化的 peers
//! - `/sync with <peer-or-host:port>` —— 一次性同步
//! - `/sync trust <peer>` / `/sync untrust <peer>` —— 修改 vault trust 标记
//! - `/sync forget <peer>` —— 删除 vault 中的 peer
//! - `/sync help` —— 帮助

use crate::app::App;
use crate::t;
use crate::tasks::{SyncStage, TaskContext, TaskFailure, TaskId, TaskOutput};
use color_eyre::Result;
use rand::rngs::OsRng;
use rand::RngCore;
use solosoul_core::VaultSession;
use std::sync::Arc;
use std::time::Instant;

use solosoul_core::VaultService;
use solosoul_sync::manager::SyncManager;
use solosoul_sync::noise::NoiseKeys;
use solosoul_sync::types::SyncPeerInfo;

/// 处理 `/sync [subcommand] [args...]`。子命令可省略，默认 `status`。
pub fn handle(app: &mut App, argv: &[&str]) -> Result<()> {
    let sub = argv.first().copied().unwrap_or("status");
    match sub {
        "status" | "list" | "jobs" => {
            status(app);
            Ok(())
        }
        "with" => {
            sync_with(app, argv.get(1).copied().unwrap_or(""));
            Ok(())
        }
        "cancel" => {
            cancel(app, argv.get(1).copied());
            Ok(())
        }
        "trust" => {
            trust_peer(app, argv.get(1).copied().unwrap_or(""), true);
            Ok(())
        }
        "untrust" => {
            trust_peer(app, argv.get(1).copied().unwrap_or(""), false);
            Ok(())
        }
        "forget" => {
            forget_peer(app, argv.get(1).copied().unwrap_or(""));
            Ok(())
        }
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => {
            app.error_message = Some(t!(app.i18n, "cmd-unknown-subcommand", cmd = other));
            Ok(())
        }
    }
}

fn print_help() {
    println!("用法: /sync <subcommand> [args]");
    println!("  status | list             列出 vault 已持久化的 peers");
    println!("  with <peer|host:port>     与指定 peer 一次性同步");
    println!("  trust <peer>              将 peer 标记为受信任");
    println!("  untrust <peer>            取消 peer 的受信任状态");
    println!("  forget <peer>             从 vault 中删除 peer 记录");
    println!("  jobs                      查看同步任务 ID 与阶段");
    println!("  cancel [task-id]           请求取消同步并等待实际收尾");
    println!("  help                      显示本帮助");
}

/// 列出当前账户 vault 中已持久化的 peers。
fn status(app: &mut App) {
    let vault_service: Arc<VaultService> = app.vault_service.clone();
    let items = list_persisted_peers(&vault_service);
    app.previous_phase = Some(app.phase.clone());
    app.phase = crate::app::AppPhase::SyncStatus {
        peers: items,
        info: "vault 中已持久化的 peer（来自历史同步会话；不包含当前 mDNS 实时发现）".to_string(),
    };
}

/// 同一 CLI 只保留一项未真实收尾的同步；取消请求不会提前释放占位。
pub(crate) struct SyncTask {
    pub peer: String,
    pub stage: SyncStage,
    pub cancelling: bool,
}

fn sync_with(app: &mut App, peer: &str) {
    if peer.is_empty() {
        app.error_message = Some(t!(app.i18n, "cmd-sync-with-usage"));
        return;
    }
    let Ok(account) = super::require_unlocked(app) else {
        return;
    };
    let session = match app.vault_service.capture_session(&account) {
        Ok(session) => session,
        Err(_) => {
            app.error_message = Some(t!(app.i18n, "cmd-need-unlock"));
            return;
        }
    };
    if let Err(error) = app.drain_task_events(32) {
        app.error_message = Some(error.to_string());
        return;
    }
    if !app.sync_tasks.is_empty() {
        app.info_message = Some(t!(app.i18n, "cmd-sync-in-progress"));
        return;
    }
    // Arc 在原会话门槛内捕获；异步阶段不能重新读取当前 Vault 或当前账户。
    let service = Arc::clone(&app.vault_service);
    let vault = match service.with_session(&session, |_| {
        service
            .get_vault_store()
            .ok_or_else(|| "Vault 未解锁".to_string())
    }) {
        Ok(vault) => vault,
        Err(_) => {
            app.error_message = Some(t!(app.i18n, "cmd-need-unlock"));
            return;
        }
    };
    let target = peer.to_owned();
    match app.tasks.spawn_cooperative(session, move |context| {
        run_one_shot_sync(context, service, vault, target)
    }) {
        Ok(id) => {
            app.sync_tasks.insert(
                id,
                SyncTask {
                    peer: peer.to_owned(),
                    stage: SyncStage::Starting,
                    cancelling: false,
                },
            );
            app.info_message = Some(t!(app.i18n, "cmd-sync-started", id = id.0.to_string()));
        }
        Err(error) => {
            app.error_message = Some(t!(
                app.i18n,
                "cmd-sync-with-failure",
                peer = peer,
                err = error
            ))
        }
    }
}

fn cancel(app: &mut App, value: Option<&str>) {
    let ids: Vec<_> = match value {
        Some(value) => match uuid::Uuid::parse_str(value) {
            Ok(id) if app.sync_tasks.contains_key(&TaskId(id)) => vec![TaskId(id)],
            _ => {
                app.error_message = Some(t!(app.i18n, "cmd-sync-cancel-usage"));
                return;
            }
        },
        None => app.sync_tasks.keys().copied().collect(),
    };
    if ids.is_empty() {
        app.info_message = Some(t!(app.i18n, "cmd-sync-no-task"));
        return;
    }
    for id in ids {
        if app.tasks.request_cancel(id) {
            if let Some(task) = app.sync_tasks.get_mut(&id) {
                task.cancelling = true;
            }
            app.info_message = Some(t!(app.i18n, "cmd-sync-cancelling"));
        }
    }
}

async fn run_one_shot_sync(
    context: TaskContext,
    service: Arc<VaultService>,
    vault: Arc<solosoul_vault::VaultStore>,
    peer: String,
) -> Result<TaskOutput, TaskFailure> {
    if context.is_cancel_requested() {
        return Err(TaskFailure::Cancelled);
    }
    let manager =
        prepare_manager(&service, context.session(), vault).map_err(|_| TaskFailure::Cancelled)?;
    context.report_sync_progress(SyncStage::Starting);
    let started = tokio::select! {
        biased;
        _ = context.cancelled() => Err(TaskFailure::Cancelled),
        result = manager.start() => result.map_err(TaskFailure::Failed),
    };
    let result = match started {
        Ok(_) => {
            context.report_sync_progress(SyncStage::Synchronizing);
            tokio::select! {
                biased;
                _ = context.cancelled() => Err(TaskFailure::Cancelled),
                result = manager.sync_with_peer(&peer) => result.map_err(TaskFailure::Failed),
            }
        }
        Err(error) => Err(error),
    };
    // 不论 start 失败、配对失败或取消，终态必须等真实 registry 收尾。
    context.report_sync_progress(SyncStage::Stopping);
    let shutdown = manager.stop_and_wait().await;
    match result {
        Ok(sr) => {
            shutdown.map_err(TaskFailure::Failed)?;
            let summary = format!(
                "records applied={} skipped={} examined={} attachments sent={} received={} errors={}",
                sr.data.applied, sr.data.skipped, sr.data.examined,
                sr.attachments.sent, sr.attachments.received, sr.data.errors.len()
            );
            context.commit(|| Ok(TaskOutput::SyncCompleted { peer, summary }))
        }
        Err(error) => {
            if shutdown.is_err() {
                tracing::warn!("CLI sync shutdown also failed");
            }
            Err(error)
        }
    }
}

fn prepare_manager(
    service: &VaultService,
    session: &VaultSession,
    vault: Arc<solosoul_vault::VaultStore>,
) -> Result<SyncManager, String> {
    service.with_session(session, |_| {
        let (node_id, keys) = sync_identity(&vault);
        Ok(SyncManager::new(
            node_id,
            session.account_id().to_owned(),
            keys,
            vault,
            "0.0.0.0:0",
        ))
    })
}

fn trust_peer(app: &mut App, peer_node_id: &str, trusted: bool) {
    if peer_node_id.is_empty() {
        app.error_message = Some(if trusted {
            t!(app.i18n, "cmd-sync-trust-usage")
        } else {
            t!(app.i18n, "cmd-sync-untrust-usage")
        });
        return;
    }
    let result = build_manager_for_manage(&app.vault_service)
        .and_then(|mgr| mgr.trust_peer(peer_node_id, trusted, None));
    match result {
        Ok(()) => {
            app.success_message = Some((
                if trusted {
                    t!(app.i18n, "cmd-sync-trusted", id = peer_node_id)
                } else {
                    t!(app.i18n, "cmd-sync-untrusted", id = peer_node_id)
                },
                Instant::now(),
            ));
        }
        Err(e) => {
            app.error_message = Some(t!(app.i18n, "cmd-sync-trust-operation-failed", err = e));
        }
    }
}

fn forget_peer(app: &mut App, peer_node_id: &str) {
    if peer_node_id.is_empty() {
        app.error_message = Some(t!(app.i18n, "cmd-sync-forget-usage"));
        return;
    }
    let result =
        build_manager_for_manage(&app.vault_service).and_then(|mgr| mgr.forget_peer(peer_node_id));
    match result {
        Ok(()) => {
            app.success_message = Some((
                t!(app.i18n, "cmd-sync-forgotten", id = peer_node_id),
                Instant::now(),
            ));
        }
        Err(e) => {
            app.error_message = Some(t!(app.i18n, "cmd-sync-forget-operation-failed", err = e));
        }
    }
}

/// 构造一个 SyncManager（仅用于 trust/forget 等管理类调用，不启动 listener）。
fn build_manager_for_manage(vault_service: &Arc<VaultService>) -> Result<SyncManager, String> {
    let vault = vault_service
        .get_vault_store()
        .ok_or_else(|| "Vault 未解锁".to_string())?;
    let account_id = vault_service
        .get_current_account()
        .ok_or_else(|| "无当前账户".to_string())?;
    let (node_id, keys) = sync_identity(&vault);
    Ok(SyncManager::new(
        node_id,
        account_id,
        keys,
        vault,
        "0.0.0.0:0",
    ))
}

/// 读取 vault 中持久化的 peer 列表（不启动 mDNS）。
pub fn list_persisted_peers(vault_service: &Arc<VaultService>) -> Vec<SyncPeerInfo> {
    let vault = match vault_service.get_vault_store() {
        Some(v) => v,
        None => return Vec::new(),
    };
    let peers = match vault.list_peers() {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };
    peers
        .into_iter()
        .map(|p| SyncPeerInfo {
            node_id: p.peer_node_id.clone(),
            account_id: vault_service.get_current_account().unwrap_or_default(),
            name: p
                .peer_name
                .clone()
                .unwrap_or_else(|| p.peer_node_id.clone()),
            addr: String::new(),
            fingerprint: p.public_key_fingerprint.clone().unwrap_or_default(),
            trusted: p.trusted,
            last_seen: String::new(),
            // v24（客户端类型/信任时间/最近同步时间）字段：直接透传持久化值
            last_seen_ts: p.last_seen,
            trusted_at: p.trusted_at,
            client_type: p.client_type.unwrap_or_else(|| "unknown".to_string()),
        })
        .collect()
}

/// 与 Tauri `sync_service` 同款的 identity 持久化逻辑。vault 以
/// 原始 `[u8;32]` 存储 secret key，无需 hex 编解码。
fn sync_identity(vault: &Arc<solosoul_vault::VaultStore>) -> (String, NoiseKeys) {
    let node_id = if let Ok(Some(existing)) = vault.get_sync_node_id() {
        existing
    } else {
        let mut bytes = [0u8; 16];
        OsRng.fill_bytes(&mut bytes);
        let id = format!(
            "node_{}",
            bytes
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>()
        );
        let _ = vault.set_sync_node_id(&id);
        id
    };

    let keys = match vault.get_sync_secret_key() {
        Ok(Some(existing)) => NoiseKeys::from_secret(existing),
        _ => {
            let k = NoiseKeys::generate();
            let _ = vault.set_sync_secret_key(k.secret_key());
            k
        }
    };

    (node_id, keys)
}

#[cfg(test)]
mod rf213_tests;
