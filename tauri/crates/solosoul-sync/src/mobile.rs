//! 同步引擎移动端实现。
//!
//! 移动端不使用桌面端的 mdns-sd，发现层由 Android NSD / iOS Bonjour 插件负责。
//! 本模块仅负责启动 TCP 监听、接受入站同步连接、以及作为发起方与指定地址同步。

use crate::noise::NoiseKeys;
use crate::session::{handle_inbound, run_initiator_session};
use crate::transport::SyncTransport;
use crate::types::{SyncPeerInfo, SyncSessionResult};
use solosoul_core::vault_service::VaultService;
use solosoul_vault::{PeerSyncState, VaultStore};
use std::net::{SocketAddr, TcpListener};
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::task::{spawn_blocking, JoinHandle};

/// 长周期 Noise 身份密钥，与桌面端实现一致。
pub use crate::noise::NoiseKeys;

/// 同步服务。
pub struct SyncService {
    vault_service: Arc<std::sync::RwLock<VaultService>>,
    manager: Mutex<Option<MobileSyncManager>>,
}

impl SyncService {
    pub fn new(vault_service: Arc<std::sync::RwLock<VaultService>>) -> Self {
        Self {
            vault_service,
            manager: Mutex::new(None),
        }
    }

    /// 启用或关闭同步监听。
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
            let manager = MobileSyncManager::new(node_id, account_id, keys, vault.clone())?;
            manager.start()?;
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

    /// 返回当前是否已开启 sync。
    pub async fn is_enabled(&self) -> bool {
        self.manager.lock().await.is_some()
    }

    /// 手动同步一个 `host:port` 地址。
    pub async fn sync_with_device(
        &self,
        device_id_or_addr: String,
    ) -> Result<SyncSessionResult, String> {
        let guard = self.manager.lock().await;
        let manager = guard.as_ref().ok_or("Sync is not enabled")?;
        manager.sync_with_peer(&device_id_or_addr).await
    }

    /// 列出已持久化的 peers（移动端发现由上层 NSD 插件维护，这里只返回持久化列表）。
    pub async fn known_peers(&self) -> Result<Vec<SyncPeerInfo>, String> {
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

    /// 标记 peer 信任状态。
    pub async fn trust_peer(&self, peer_node_id: String, trusted: bool) -> Result<(), String> {
        let guard = self.manager.lock().await;
        let result = if let Some(m) = guard.as_ref() {
            m.trust_peer(&peer_node_id, trusted)
        } else {
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

    /// 移除 peer。
    pub async fn forget_peer(&self, peer_node_id: String) -> Result<(), String> {
        let guard = self.manager.lock().await;
        if let Some(m) = guard.as_ref() {
            m.forget_peer(&peer_node_id)
        } else {
            let svc = self
                .vault_service
                .read()
                .map_err(|_| "Vault service lock poisoned".to_string())?;
            let vault = svc.get_vault_store().ok_or("Vault is not unlocked")?;
            vault.delete_peer(&peer_node_id)
        }
    }

    /// 返回本地指纹。
    pub async fn local_fingerprint(&self) -> Result<String, String> {
        let guard = self.manager.lock().await;
        if let Some(m) = guard.as_ref() {
            Ok(m.fingerprint())
        } else {
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

/// 移动端同步管理器：维护 TCP 监听与 Noise 身份。
struct MobileSyncManager {
    node_id: String,
    account_id: String,
    keys: NoiseKeys,
    vault: Arc<VaultStore>,
    listen_port: AtomicU16,
    running: AtomicBool,
    worker_handles: Mutex<Vec<JoinHandle<()>>>,
}

impl MobileSyncManager {
    fn new(
        node_id: String,
        account_id: String,
        keys: NoiseKeys,
        vault: Arc<VaultStore>,
    ) -> Result<Self, String> {
        Ok(Self {
            node_id,
            account_id,
            keys,
            vault,
            listen_port: AtomicU16::new(0),
            running: AtomicBool::new(false),
            worker_handles: Mutex::new(Vec::new()),
        })
    }

    fn fingerprint(&self) -> String {
        self.keys.fingerprint()
    }

    fn start(&self) -> Result<u16, String> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(self.listen_port.load(Ordering::SeqCst));
        }
        self.running.store(true, Ordering::SeqCst);

        let listener = TcpListener::bind("0.0.0.0:0").map_err(|e| format!("bind failed: {}", e))?;
        listener
            .set_nonblocking(false)
            .map_err(|e| format!("set blocking: {}", e))?;
        let port = listener
            .local_addr()
            .map_err(|e| format!("local_addr: {}", e))?
            .port();
        self.listen_port.store(port, Ordering::SeqCst);

        let running = self.running.clone();
        let node_id = self.node_id.clone();
        let account_id = self.account_id.clone();
        let keys = self.keys.clone();
        let vault = self.vault.clone();

        let accept_handle = spawn_blocking(move || loop {
            if !running.load(Ordering::SeqCst) {
                break;
            }
            match listener.accept() {
                Ok((stream, addr)) => {
                    if !running.load(Ordering::SeqCst) {
                        break;
                    }
                    let node_id = node_id.clone();
                    let account_id = account_id.clone();
                    let keys = keys.clone();
                    let vault = vault.clone();
                    spawn_blocking(move || {
                        let mut transport = SyncTransport::from_stream(stream);
                        let _ = handle_inbound(
                            &mut transport,
                            &node_id,
                            &account_id,
                            &keys,
                            vault,
                            addr.to_string(),
                        );
                    });
                }
                Err(e) => {
                    tracing::warn!("accept error: {}", e);
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        });

        if let Ok(mut handles) = self.worker_handles.lock() {
            handles.push(accept_handle);
        }

        Ok(port)
    }

    fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        if let Ok(mut handles) = self.worker_handles.lock() {
            let port = self.listen_port.load(Ordering::SeqCst);
            if port != 0 {
                let _ = std::net::TcpStream::connect(format!("127.0.0.1:{}", port));
            }
            for h in handles.drain(..) {
                h.abort();
            }
        }
    }

    async fn sync_with_peer(&self, device_id_or_addr: &str) -> Result<SyncSessionResult, String> {
        if !self.running.load(Ordering::SeqCst) {
            return Err("Sync manager is not running".to_string());
        }
        let addr: SocketAddr = device_id_or_addr
            .parse()
            .map_err(|e| format!("Invalid address: {}", e))?;

        let node_id = self.node_id.clone();
        let account_id = self.account_id.clone();
        let keys = self.keys.clone();
        let vault = self.vault.clone();

        spawn_blocking(move || {
            let stream = std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(10))
                .map_err(|e| format!("connect: {}", e))?;
            let mut transport = SyncTransport::from_stream(stream);
            run_initiator_session(
                &mut transport,
                &node_id,
                &account_id,
                &keys,
                vault,
                addr.to_string(),
            )
        })
        .await
        .map_err(|e| format!("spawn blocking: {}", e))?
    }

    fn trust_peer(&self, peer_node_id: &str, trusted: bool) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut peer = self
            .vault
            .load_peer_state(peer_node_id)?
            .unwrap_or_else(|| PeerSyncState {
                peer_node_id: peer_node_id.to_string(),
                peer_name: None,
                trusted: false,
                public_key_fingerprint: None,
                last_seen: None,
                created_at: now.clone(),
                updated_at: now.clone(),
            });
        peer.trusted = trusted;
        peer.updated_at = now;
        self.vault.save_peer_state(&peer)
    }

    fn forget_peer(&self, peer_node_id: &str) -> Result<(), String> {
        self.vault.delete_peer(peer_node_id)
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
