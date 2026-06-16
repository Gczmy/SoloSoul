//! Sync service - bridges the VaultService and the solosoul-sync SyncManager.

use solosoul_sync::manager::SyncSessionResult;
use solosoul_sync::{NoiseKeys, SyncManager, SyncPeerInfo};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::services::vault_service::VaultService;

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

    /// Enable or disable the background sync daemon.
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
            let manager = SyncManager::new(
                node_id,
                account_id,
                keys,
                vault.clone(),
                "0.0.0.0:0", // bind to all interfaces, OS assigns port
            );
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

    /// Returns true if the sync manager is currently running.
    pub async fn is_enabled(&self) -> bool {
        self.manager.lock().await.is_some()
    }

    /// Manually sync with a discovered peer (node id) or a `host:port` address.
    pub async fn sync_with_device(
        &self,
        device_id_or_addr: String,
    ) -> Result<SyncSessionResult, String> {
        let guard = self.manager.lock().await;
        let manager = guard.as_ref().ok_or("Sync is not enabled")?;
        let result = manager.sync_with_peer(&device_id_or_addr).await?;
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

    /// List discovered and persisted peers.
    pub async fn known_peers(&self) -> Result<Vec<SyncPeerInfo>, String> {
        let guard = self.manager.lock().await;
        match guard.as_ref() {
            Some(m) => m.known_peers(),
            None => {
                // Even when the manager is off, return persisted peers.
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

    /// Mark a peer as trusted/untrusted.
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

    /// Remove a peer from persisted state.
    pub async fn forget_peer(&self, peer_node_id: String) -> Result<(), String> {
        let guard = self.manager.lock().await;
        let result = match guard.as_ref() {
            Some(m) => m.forget_peer(&peer_node_id),
            None => {
                let svc = self
                    .vault_service
                    .read()
                    .map_err(|_| "Vault service lock poisoned".to_string())?;
                let vault = svc.get_vault_store().ok_or("Vault is not unlocked")?;
                vault.delete_peer(&peer_node_id)
            }
        };
        if result.is_ok() {
            if let Ok(svc) = self.vault_service.try_read() {
                if let Some(vault) = svc.get_vault_store() {
                    audit_log(&vault, "sync_peer_forgotten", Some(&peer_node_id), None);
                }
            }
        }
        result
    }

    /// Return the local fingerprint for pairing verification.
    pub async fn local_fingerprint(&self) -> Result<String, String> {
        let guard = self.manager.lock().await;
        match guard.as_ref() {
            Some(m) => Ok(m.fingerprint()),
            None => {
                let svc = self
                    .vault_service
                    .read()
                    .map_err(|_| "Vault service lock poisoned".to_string())?;
                let vault = svc.get_vault_store().ok_or("Vault is not unlocked")?;
                let (_, keys) = get_or_create_sync_identity(&vault)?;
                Ok(keys.fingerprint())
            }
        }
    }
}

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
