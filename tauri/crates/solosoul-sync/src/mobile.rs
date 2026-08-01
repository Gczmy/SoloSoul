//! 同步引擎移动端实现。
//!
//! 移动端不使用桌面端的 mdns-sd，发现层由 Android NSD / iOS Bonjour 插件负责。
//! 本模块仅负责启动 TCP 监听、接受入站同步连接、以及作为发起方与指定地址同步。

use crate::session::{handle_inbound, run_initiator_session, wrap_session_error};
use crate::transport::SyncTransport;
use crate::types::{SyncPeerInfo, SyncSessionResult};
use solosoul_core::vault_service::VaultService;
use solosoul_vault::{PeerSyncState, VaultStore};
use std::net::{SocketAddr, TcpListener};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::task::{spawn_blocking, JoinHandle};

/// `stop()` 等待正在进行的同步会话完成的最大时长（秒）。
const STOP_GRACE_PERIOD_SECS: u64 = 30;
/// `stop()` 轮询 `active_sessions` 的间隔。
const STOP_POLL_INTERVAL_MS: u64 = 100;

/// RAII guard：创建时递增 `active_sessions`，Drop 时递减。
struct SessionGuard {
    counter: Arc<AtomicUsize>,
}

impl SessionGuard {
    fn new(counter: Arc<AtomicUsize>) -> Self {
        counter.fetch_add(1, Ordering::SeqCst);
        Self { counter }
    }
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::SeqCst);
    }
}

/// 长周期 Noise 身份密钥，与桌面端实现一致。
use crate::noise::NoiseKeys;

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
            let port = manager.start()?;
            audit_log(
                &vault,
                "sync_enabled",
                None,
                Some(&format!(
                    "fingerprint={},port={}",
                    manager.fingerprint(),
                    port
                )),
            );
            *guard = Some(manager);
            Ok(())
        } else {
            let old_manager = guard.take();
            // 先释放 manager 锁，再在 blocking 线程执行 stop()：
            // stop() 可能等待活跃同步会话最多 STOP_GRACE_PERIOD_SECS（30 秒），
            // 若在此处同步调用会阻塞 async 命令线程，并让 sync_get_status 等
            // 需要 manager 锁的命令全部排队，前端表现为“禁用失败、所有按钮卡住”。
            // manager 已从锁中取出，后续 is_enabled()/sync_get_status 立即返回 false。
            drop(guard);
            if let Some(m) = old_manager {
                // 显式 drop JoinHandle 以分离任务（detach），命令立即返回
                std::mem::drop(tokio::task::spawn_blocking(move || m.stop()));
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
    ///
    /// 仅在锁内提取会话所需数据（短临界区），随后立即释放锁，并把实际会话交给
    /// blocking 线程执行：整个会话可能耗时数秒到数十秒（连接超时 10s + 数据交换）。
    /// 若在等待期间持有 manager 锁，enable(false) / sync_get_status 等命令会全部排队，
    /// 前端表现为“禁用同步失败、所有按钮卡住”。
    pub async fn sync_with_device(
        &self,
        device_id_or_addr: String,
    ) -> Result<SyncSessionResult, String> {
        let (node_id, account_id, keys, vault, active_sessions, running) = {
            let guard = self.manager.lock().await;
            let manager = guard.as_ref().ok_or("Sync is not enabled")?;
            (
                manager.node_id.clone(),
                manager.account_id.clone(),
                manager.keys.clone(),
                manager.vault.clone(),
                manager.active_sessions.clone(),
                manager.running.clone(),
            )
        };
        if !running.load(Ordering::SeqCst) {
            return Err("Sync manager is not running".to_string());
        }
        let addr: SocketAddr = device_id_or_addr
            .parse()
            .map_err(|e| format!("__SYNC_ERR__:invalid_address:{}", e))?;

        spawn_blocking(move || {
            let _guard = SessionGuard::new(active_sessions);
            let stream = std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(10))
                .map_err(|e| format!("__SYNC_ERR__:connect_failed:{}", e))?;
            let mut transport = SyncTransport::from_stream(stream);
            run_initiator_session(
                &mut transport,
                &node_id,
                &account_id,
                &keys,
                vault,
                addr.to_string(),
            )
            .map_err(wrap_session_error)
        })
        .await
        .map_err(|e| format!("spawn blocking: {}", e))?
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

    /// 返回当前监听端口（未启用时返回 0）。
    pub async fn listen_port(&self) -> u16 {
        let guard = self.manager.lock().await;
        guard.as_ref().map(|m| m.listen_port()).unwrap_or(0)
    }
}

/// 移动端同步管理器：维护 TCP 监听与 Noise 身份。
struct MobileSyncManager {
    node_id: String,
    account_id: String,
    keys: NoiseKeys,
    vault: Arc<VaultStore>,
    listen_port: AtomicU16,
    running: Arc<AtomicBool>,
    worker_handles: StdMutex<Vec<JoinHandle<()>>>,
    /// 正在进行的同步会话数量。`stop()` 会等待此计数归零后再终止 worker，
    /// 避免中途 abort 正在写入 Vault 的会话导致数据不一致。
    active_sessions: Arc<AtomicUsize>,
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
            running: Arc::new(AtomicBool::new(false)),
            worker_handles: StdMutex::new(Vec::new()),
            active_sessions: Arc::new(AtomicUsize::new(0)),
        })
    }

    fn fingerprint(&self) -> String {
        self.keys.fingerprint()
    }

    fn listen_port(&self) -> u16 {
        self.listen_port.load(Ordering::SeqCst)
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

        let running = Arc::clone(&self.running);
        let node_id = self.node_id.clone();
        let account_id = self.account_id.clone();
        let keys = self.keys.clone();
        let vault = self.vault.clone();
        let active_sessions = self.active_sessions.clone();

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
                    let guard = SessionGuard::new(active_sessions.clone());
                    spawn_blocking(move || {
                        let _guard = guard; // 持有直到会话结束
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

        // Unblock the blocking accept loop by connecting to ourselves.
        let port = self.listen_port.load(Ordering::SeqCst);
        if port != 0 {
            let _ = std::net::TcpStream::connect(format!("127.0.0.1:{}", port));
        }

        // 等待正在进行的同步会话完成，避免 abort 中断 Vault 写入。
        let deadline = Instant::now() + Duration::from_secs(STOP_GRACE_PERIOD_SECS);
        while self.active_sessions.load(Ordering::SeqCst) > 0 {
            if Instant::now() >= deadline {
                tracing::warn!(
                    "MobileSyncManager.stop(): {} session(s) still active after {}s grace period, forcing abort",
                    self.active_sessions.load(Ordering::SeqCst),
                    STOP_GRACE_PERIOD_SECS
                );
                break;
            }
            std::thread::sleep(Duration::from_millis(STOP_POLL_INTERVAL_MS));
        }

        if let Ok(mut handles) = self.worker_handles.lock() {
            for h in handles.drain(..) {
                h.abort();
            }
        }
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
