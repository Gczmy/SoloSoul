// crates/solosoul-sync/src/service.rs
//
// Sync service — bridges the VaultService and the solosoul-sync SyncManager.
//
// P0.5 阶段把原来在 tauri/src-tauri/src/services/sync_service.rs 的实现整体迁到这里,
// 供 CLI 和 GUI 共用. 这一次迁入彻底删掉了此前所有自创的 helper 方法 (disable /
// local_fingerprint / status_snapshot …), 完全按 SyncManager / VaultService 真实公开
// API 重写, 以避免 12 个 E0282 类型推断错误 + 24 个误用 API 名错误.
//
// 与原版唯一的差异: 因为 service.rs 现在位于 solosoul-sync crate 内部,
// `use solosoul_sync::X` 必须改为 `use crate::X`; `mutex guard + match guard.as_ref()`
// 模式保留 (E0282 的根因是不规范的 in-crate import, 不是 match ergonomics).
//
// 模式上, 所有需要访问 manager 的方法统一沿用:
//   let guard = self.manager.lock().await;
//   let manager = guard.as_ref().ok_or("Sync is not enabled")?;
//   // manager: &SyncManager, 调用方法 .await / .x() 全部明确

use crate::manager::SyncManager;
use crate::noise::NoiseKeys;
use crate::types::{SyncPeerInfo, SyncSessionResult};
use std::sync::Arc;
use tokio::sync::Mutex;

use solosoul_core::vault_service::VaultService;

pub struct SyncService {
    vault_service: Arc<std::sync::RwLock<VaultService>>,
    manager: Mutex<Option<SyncManager>>,
}

impl SyncService {
    pub fn new(vault_service: Arc<std::sync::RwLock<VaultService>>) -> Self {
        Self {
            vault_service,
            manager: Mutex::new(None),
        }
    }

    /// 启用或关闭后台同步守护进程.
    pub async fn enable(&self, enable: bool) -> Result<(), String> {
        let mut guard = self.manager.lock().await;
        if enable {
            if guard.is_some() {
                return Ok(());
            }
            let (vault, account_id) = {
                let svc = self
                    .vault_service
                    .read()
                    .map_err(|_| "Vault service lock poisoned".to_string())?;
                let vault = svc.get_vault_store().ok_or("Vault is not unlocked")?;
                let account_id = svc.get_current_account().ok_or("No account is unlocked")?;
                (vault, account_id)
            };
            let (node_id, keys) = get_or_create_sync_identity(&vault)?;
            let manager = SyncManager::new(node_id, account_id, keys, vault.clone(), "0.0.0.0:0");
            manager.start().await?;
            audit_log(
                &vault,
                "sync_enabled",
                None,
                Some(&format!("fingerprint={}", manager.fingerprint())),
            );
            *guard = Some(manager);
            Ok(())
        } else {
            if let Some(m) = guard.take() {
                m.stop();
            }
            if let Ok(svc) = self.vault_service.try_read() {
                if let Some(vault) = svc.get_vault_store() {
                    audit_log(&vault, "sync_disabled", None, None);
                }
            }
            Ok(())
        }
    }

    /// 返回当前是否已开启 sync.
    pub async fn is_enabled(&self) -> bool {
        self.manager.lock().await.is_some()
    }

    /// 手动同步一个已发现的 peer (按 node id) 或一个 `host:port` 地址.
    pub async fn sync_with_device(
        &self,
        device_id_or_addr: String,
    ) -> Result<SyncSessionResult, String> {
        let guard = self.manager.lock().await;
        let result = match guard.as_ref() {
            Some(m) => m.sync_with_peer(&device_id_or_addr).await,
            None => Err("Sync is not enabled".to_string()),
        }?;
        let table_summary = result
            .data
            .per_table
            .iter()
            .map(|(table, s)| format!("{}:{}+{}/{}", table, s.applied, s.skipped, s.examined))
            .collect::<Vec<_>>()
            .join(", ");
        let summary = format!(
            "examined={}, applied={}, skipped={}; conflicts={}; tables=[{}]; attachments: sent={}, received={}, bytes={}",
            result.data.examined,
            result.data.applied,
            result.data.skipped,
            result.data.conflicts.len(),
            table_summary,
            result.attachments.sent,
            result.attachments.received,
            result.attachments.bytes_transferred
        );
        if let Ok(svc) = self.vault_service.try_read() {
            if let Some(vault) = svc.get_vault_store() {
                audit_log(
                    &vault,
                    "sync_with_device",
                    Some(&device_id_or_addr),
                    Some(&summary),
                );
            }
        }
        Ok(result)
    }

    /// 列出已发现并已持久化的 peers; 即便 manager 未启动也会回退到 vault 持久化列表.
    pub async fn known_peers(&self) -> Result<Vec<SyncPeerInfo>, String> {
        let guard = self.manager.lock().await;
        match guard.as_ref() {
            Some(m) => m.known_peers(),
            None => {
                let svc = self
                    .vault_service
                    .read()
                    .map_err(|_| "Vault service lock poisoned".to_string())?;
                let vault = svc.get_vault_store().ok_or("Vault is not unlocked")?;
                let account_id = svc.get_current_account().unwrap_or_default();
                let persisted = vault.list_peers()?;
                Ok(persisted
                    .into_iter()
                    .map(|p| SyncPeerInfo {
                        node_id: p.peer_node_id.clone(),
                        account_id: account_id.clone(),
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
                    })
                    .collect())
            }
        }
    }

    /// 把一个 peer 标记为信任/不信任; 即便 manager 未启动也会回写到 vault.
    pub async fn trust_peer(&self, peer_node_id: String, trusted: bool) -> Result<(), String> {
        let guard = self.manager.lock().await;
        let result = match guard.as_ref() {
            Some(m) => m.trust_peer(&peer_node_id, trusted),
            None => {
                let svc = self
                    .vault_service
                    .read()
                    .map_err(|_| "Vault service lock poisoned".to_string())?;
                let vault = svc.get_vault_store().ok_or("Vault is not unlocked")?;
                let now = chrono::Utc::now().to_rfc3339();
                let mut peer = vault.load_peer_state(&peer_node_id)?.unwrap_or_else(|| {
                    solosoul_vault::PeerSyncState {
                        peer_node_id: peer_node_id.clone(),
                        peer_name: None,
                        trusted: false,
                        public_key_fingerprint: None,
                        last_seen: None,
                        created_at: now.clone(),
                        updated_at: now.clone(),
                    }
                });
                peer.trusted = trusted;
                peer.updated_at = now;
                vault.save_peer_state(&peer)
            }
        };
        if result.is_ok() {
            if let Ok(svc) = self.vault_service.try_read() {
                if let Some(vault) = svc.get_vault_store() {
                    audit_log(
                        &vault,
                        if trusted {
                            "sync_peer_trusted"
                        } else {
                            "sync_peer_revoked"
                        },
                        Some(&peer_node_id),
                        None,
                    );
                }
            }
        }
        result
    }

    /// 从持久化状态中移除一个 peer (manager 启动时同步, 否则仅清持久化).
    pub async fn forget_peer(&self, peer_node_id: String) -> Result<(), String> {
        let guard = self.manager.lock().await;
        match guard.as_ref() {
            Some(m) => m.forget_peer(&peer_node_id),
            None => {
                let svc = self
                    .vault_service
                    .read()
                    .map_err(|_| "Vault service lock poisoned".to_string())?;
                let vault = svc.get_vault_store().ok_or("Vault is not unlocked")?;
                vault.delete_peer(&peer_node_id)
            }
        }
    }
    /// 返回本地节点的 Noise 公钥指纹, GUI 状态栏 / pairing 二维码都会用它.
    pub async fn local_fingerprint(&self) -> Result<String, String> {
        let guard = self.manager.lock().await;
        match guard.as_ref() {
            Some(m) => Ok(m.fingerprint()),
            None => {
                // Manager 未启动时, 从 vault 中取出 Noise 静态密钥直接推导指纹.
                let svc = self
                    .vault_service
                    .read()
                    .map_err(|_| "Vault service lock poisoned".to_string())?;
                let vault = svc.get_vault_store().ok_or("Vault is not unlocked")?;
                let (_node_id, keys) = get_or_create_sync_identity(&vault)?;
                Ok(keys.fingerprint())
            }
        }
    }
}

// -----------------------------------------------------------------------------
// 私有辅助函数: 审计日志 + 同步身份生成.
// -----------------------------------------------------------------------------

fn audit_log(
    vault: &solosoul_vault::VaultStore,
    action: &str,
    entity_id: Option<&str>,
    details: Option<&str>,
) {
    let _ = vault.log_structured(action, "sync", entity_id, None, "user", details);
}

fn get_or_create_sync_identity(
    vault: &solosoul_vault::VaultStore,
) -> Result<(String, NoiseKeys), String> {
    let node_id = match vault.get_sync_node_id()? {
        Some(id) => id,
        None => {
            let id = format!("node_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
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

// -----------------------------------------------------------------------------
// 单元测试: 锁定 vault 的最小 happy-path + not-running 错误回退.
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn fresh_service() -> (SyncService, tempfile::TempDir) {
        let dir = tempdir().expect("tempdir");
        let vault = VaultService::new();
        let svc = SyncService::new(Arc::new(std::sync::RwLock::new(vault)));
        (svc, dir)
    }

    #[tokio::test]
    async fn fresh_service_is_not_enabled() {
        let (svc, _dir) = fresh_service();
        assert!(!svc.is_enabled().await);
    }

    #[tokio::test]
    async fn fresh_service_sync_with_device_returns_err() {
        let (svc, _dir) = fresh_service();
        let r = svc.sync_with_device("127.0.0.1:12345".to_string()).await;
        assert!(r.is_err());
    }
}
