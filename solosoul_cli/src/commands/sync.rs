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
use color_eyre::Result;
use rand::rngs::OsRng;
use rand::RngCore;
use std::sync::Arc;

use solosoul_core::VaultService;
use solosoul_sync::manager::SyncManager;
use solosoul_sync::types::SyncPeerInfo;
use solosoul_sync::noise::NoiseKeys;

/// 处理 `/sync [subcommand] [args...]`。子命令可省略，默认 `status`。
pub fn handle(app: &mut App, argv: &[&str]) -> Result<()> {
    let sub = argv.first().copied().unwrap_or("status");
    match sub {
        "status" | "list" => {
            status(app);
            Ok(())
        }
        "with" => {
            sync_with(app, argv.get(1).copied().unwrap_or(""));
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
            app.error_message = Some(format!("未知 /sync 子命令: {}", other));
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

/// 一次性同步：构造 SyncManager → start → sync_with_peer → stop。
fn sync_with(app: &mut App, peer: &str) {
    if peer.is_empty() {
        app.error_message = Some("用法: /sync with <peer-or-host:port>".to_string());
        return;
    }
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            app.error_message = Some(format!("创建异步运行时失败: {}", e));
            return;
        }
    };

    let result = runtime.block_on(run_one_shot_sync(&app.vault_service, peer));
    match result {
        Ok(summary) => {
            tracing::info!("sync with {}: {}", peer, summary);
            app.error_message = Some(format!(
                "/sync with {} 完成：{}。详细计数在审计日志中。",
                peer, summary
            ));
        }
        Err(e) => {
            app.error_message = Some(format!("/sync with {} 失败: {}", peer, e));
        }
    }
}

async fn run_one_shot_sync(
    vault_service: &Arc<VaultService>,
    peer: &str,
) -> Result<String, String> {
    let vault = vault_service
        .get_vault_store()
        .ok_or_else(|| "Vault 未解锁，请先 /unlock".to_string())?;
    let account_id = vault_service
        .get_current_account()
        .ok_or_else(|| "无当前账户".to_string())?;

    let (node_id, keys) = sync_identity(&vault);

    let manager = SyncManager::new(
        node_id.clone(),
        account_id,
        keys,
        vault.clone(),
        "0.0.0.0:0",
    );

    manager.start().await?;
    let result = manager.sync_with_peer(peer).await;
    manager.stop();

    match result {
        Ok(sr) => Ok(format!(
            "records applied={} skipped={} examined={} attachments sent={} received={} errors={}",
            sr.data.applied,
            sr.data.skipped,
            sr.data.examined,
            sr.attachments.sent,
            sr.attachments.received,
            sr.data.errors.len()
        )),
        Err(e) => Err(e),
    }
}

fn trust_peer(app: &mut App, peer_node_id: &str, trusted: bool) {
    if peer_node_id.is_empty() {
        app.error_message = Some(format!(
            "用法: /sync {} <peer>",
            if trusted { "trust" } else { "untrust" }
        ));
        return;
    }
    let result = build_manager_for_manage(&app.vault_service)
        .and_then(|mgr| mgr.trust_peer(peer_node_id, trusted));
    match result {
        Ok(()) => {
            app.error_message = Some(format!(
                "已将 peer {} 标记为{}",
                peer_node_id,
                if trusted { "trusted" } else { "untrusted" }
            ));
        }
        Err(e) => {
            app.error_message = Some(format!("/sync trust 操作失败: {}", e));
        }
    }
}

fn forget_peer(app: &mut App, peer_node_id: &str) {
    if peer_node_id.is_empty() {
        app.error_message = Some("用法: /sync forget <peer>".to_string());
        return;
    }
    let result =
        build_manager_for_manage(&app.vault_service).and_then(|mgr| mgr.forget_peer(peer_node_id));
    match result {
        Ok(()) => {
            app.error_message = Some(format!("已从 vault 中删除 peer {}", peer_node_id));
        }
        Err(e) => {
            app.error_message = Some(format!("/sync forget 失败: {}", e));
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
