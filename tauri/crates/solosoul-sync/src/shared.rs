//! 同步管理器共享辅助（P139）。
//!
//! 桌面端 `SyncManager`（manager.rs）、`SyncService`（service.rs）与移动端
//! `SyncService`（mobile.rs）三处在 vault 兜底分支（known_peers / trust_peer /
//! forget_peer / local_fingerprint）以及审计日志 / 同步身份生成逻辑逐字节重复，
//! 此处统一收敛。三处仅保留 manager 已启动分支，兜底分支一律调用本模块函数，
//! 避免漂移。

use crate::noise::NoiseKeys;
use crate::types::SyncPeerInfo;
use solosoul_vault::VaultStore;

/// P1#7/#8：持久化 peer 凭 `last_addr + last_seen` 判定在线的宽限期（秒），
/// 与 manager.rs 的 mDNS `PEER_MAX_AGE_SECS` 保持一致——成功同步后 5 分钟内
/// 视为在线，即使 mDNS/NSD 发现链中断也能显示最近可达状态。
pub const PEER_ONLINE_GRACE_SECS: i64 = 300;

/// 判断持久化 peer 是否可视为在线：最近同步时间在宽限期内且留存了连接地址。
pub fn peer_last_addr_online(p: &solosoul_vault::PeerSyncState, now_ts: i64) -> Option<String> {
    match (&p.last_addr, p.last_seen) {
        (Some(addr), Some(ts)) if !addr.is_empty() && now_ts - ts <= PEER_ONLINE_GRACE_SECS => {
            Some(addr.clone())
        }
        _ => None,
    }
}

/// 记录同步相关操作日志。Vault 未解锁时静默跳过。
pub fn audit_log(vault: &VaultStore, action: &str, entity_id: Option<&str>, details: Option<&str>) {
    let _ = vault.log_structured(action, "sync", entity_id, None, "user", details);
}

/// 读取或创建同步身份（node_id + Noise 静态密钥），持久化到 vault。
pub fn get_or_create_sync_identity(vault: &VaultStore) -> Result<(String, NoiseKeys), String> {
    let node_id = match vault.get_sync_node_id()? {
        Some(id) => id,
        None => {
            let id = format!("node_{}", uuid::Uuid::new_v4().to_string().replace('-', ""));
            vault.set_sync_node_id(&id)?;
            id
        }
    };
    let keys = match vault.get_sync_secret_key()? {
        Some(secret) => NoiseKeys::from_secret(secret),
        None => {
            let keys = NoiseKeys::generate();
            vault.set_sync_secret_key(keys.secret_key())?;
            keys
        }
    };
    Ok((node_id, keys))
}

/// manager 未启动时回退：从 vault 持久化列表构造 `SyncPeerInfo`。
pub fn known_peers_from_vault(
    vault: &VaultStore,
    account_id: &str,
) -> Result<Vec<SyncPeerInfo>, String> {
    let persisted = vault.list_peers()?;
    let now_ts = chrono::Utc::now().timestamp();
    Ok(persisted
        .into_iter()
        .map(|p| {
            // P1#7/#8：在线状态心跳化——有 fresh last_addr 时填实际地址，
            // 前端据此显示「在线」，否则 addr 为空显示「未在局域网发现」。
            let online_addr = peer_last_addr_online(&p, now_ts);
            SyncPeerInfo {
                node_id: p.peer_node_id.clone(),
                account_id: account_id.to_string(),
                name: p
                    .peer_name
                    .clone()
                    .unwrap_or_else(|| p.peer_node_id.clone()),
                addr: online_addr.unwrap_or_default(),
                fingerprint: p.public_key_fingerprint.clone().unwrap_or_default(),
                trusted: p.trusted,
                last_seen: p
                    .last_seen
                    .map(|ts| format!("{}s ago", now_ts - ts))
                    .unwrap_or_default(),
                last_seen_ts: p.last_seen,
                trusted_at: p.trusted_at,
                client_type: p
                    .client_type
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
            }
        })
        .collect())
}

/// manager 未启动时回退：标记 peer 信任状态（含 P001/P103 指纹补绑）。
pub fn trust_peer_fallback(
    vault: &VaultStore,
    peer_node_id: &str,
    trusted: bool,
    fingerprint: Option<String>,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    let fp = fingerprint.filter(|f| !f.is_empty());
    let mut peer =
        vault
            .load_peer_state(peer_node_id)?
            .unwrap_or_else(|| solosoul_vault::PeerSyncState {
                peer_node_id: peer_node_id.to_string(),
                peer_name: None,
                trusted: false,
                public_key_fingerprint: fp.clone(),
                last_seen: None,
                created_at: now.clone(),
                updated_at: now.clone(),
                client_type: None,
                trusted_at: None,
                last_addr: None,
            });
    // 已有记录但无指纹时补绑（历史记录/握手期未绑定）。
    // 空串视为无指纹，避免绑定 "" 导致后续握手被 P001 拒绝。
    if trusted && peer.public_key_fingerprint.is_none() {
        peer.public_key_fingerprint = fp;
    }
    // 信任/撤销时维护 trusted_at：信任记时间戳，撤销清空。
    peer.trusted_at = if trusted {
        Some(chrono::Utc::now().timestamp())
    } else {
        None
    };
    peer.trusted = trusted;
    peer.updated_at = now;
    vault.save_peer_state(&peer)
}

/// manager 未启动时回退：移除 peer。
pub fn forget_peer_fallback(vault: &VaultStore, peer_node_id: &str) -> Result<(), String> {
    vault.delete_peer(peer_node_id)
}

/// manager 未启动时回退：从 vault 推导本地 Noise 公钥指纹。
pub fn local_fingerprint_fallback(vault: &VaultStore) -> Result<String, String> {
    let (_node_id, keys) = get_or_create_sync_identity(vault)?;
    Ok(keys.fingerprint())
}

/// P031: mDNS TXT 属性的共享解析——manager.rs（SyncManager 发现循环）与
/// src-tauri commands/discovery.rs（一次性扫描）此前各自重复解析同一组键，
/// 收敛为单一实现防漂移。
///
/// 键取值均为 `mdns_sd::TxtProperty`（Display 即字符串值），统一转为 String。
#[derive(Debug, Clone, Default)]
pub struct MdnsTxtProps {
    pub account_hash: String,
    pub account_id: String,
    pub node_id: String,
    pub fingerprint: String,
    pub client_type: String,
}

/// 从 `mdns_sd` 服务信息解析 TXT 属性（`ServiceInfo::get_properties()` 返回值）。
pub fn parse_mdns_txt(props: &mdns_sd::TxtProperties) -> MdnsTxtProps {
    MdnsTxtProps {
        account_hash: props
            .get("account_hash")
            .map(|v| v.to_string())
            .unwrap_or_default(),
        account_id: props
            .get("account_id")
            .map(|v| v.to_string())
            .unwrap_or_default(),
        node_id: props
            .get("node_id")
            .map(|v| v.to_string())
            .unwrap_or_default(),
        fingerprint: props
            .get("fingerprint")
            .map(|v| v.to_string())
            .unwrap_or_default(),
        client_type: props
            .get("client_type")
            .map(|v| v.to_string())
            .unwrap_or_default(),
    }
}
