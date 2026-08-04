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
    Ok(persisted
        .into_iter()
        .map(|p| SyncPeerInfo {
            node_id: p.peer_node_id.clone(),
            account_id: account_id.to_string(),
            name: p
                .peer_name
                .clone()
                .unwrap_or_else(|| p.peer_node_id.clone()),
            addr: String::new(),
            fingerprint: p.public_key_fingerprint.clone().unwrap_or_default(),
            trusted: p.trusted,
            last_seen: p
                .last_seen
                .map(|ts| format!("{}s ago", chrono::Utc::now().timestamp() - ts))
                .unwrap_or_default(),
            last_seen_ts: p.last_seen,
            trusted_at: p.trusted_at,
            client_type: p
                .client_type
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
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
