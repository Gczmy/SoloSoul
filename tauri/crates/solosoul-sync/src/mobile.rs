//! 同步引擎移动端实现。
//!
//! 移动端不使用桌面端的 mdns-sd，发现层由 Android NSD / iOS Bonjour 插件负责。
//! 本模块仅负责启动 TCP 监听、接受入站同步连接、以及作为发起方与指定地址同步。

use crate::failure::{SyncFailure, SyncFailureKind};
use crate::session::{run_accept_loop, run_initiator_session};
use crate::shared::{
    audit_log, forget_peer_fallback, get_or_create_sync_identity, known_peers_from_vault,
    local_fingerprint_fallback, trust_peer_fallback,
};
use crate::transport::SyncTransport;
use crate::types::{PeerCallback, SessionCompletedCallback, SyncPeerInfo, SyncSessionResult};
use crate::workers::{StopCompletion, SyncWorkers};
use solosoul_core::vault_service::VaultService;
use solosoul_vault::VaultStore;
use std::net::{SocketAddr, TcpListener};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex as StdMutex, RwLock};
use std::time::Duration;
use tokio::sync::Mutex;

/// `stop()` 等待正在进行的同步会话完成的最大时长（秒）。
const STOP_GRACE_PERIOD_SECS: u64 = 30;

/// 长周期 Noise 身份密钥，与桌面端实现一致。
use crate::noise::NoiseKeys;

/// 同步服务。
pub struct SyncService {
    vault_service: Arc<std::sync::RwLock<VaultService>>,
    manager: Mutex<Option<Arc<MobileSyncManager>>>,
    lifecycle: Mutex<()>,
    stopping: StdMutex<Option<Arc<StopCompletion>>>,
    /// 入站新 peer 回调钩子（与桌面端一致，创建 manager 时注入）。
    peer_callback: Arc<RwLock<Option<PeerCallback>>>,
    /// 入站会话完成回调钩子（与桌面端一致，创建 manager 时注入）。
    session_callback: Arc<RwLock<Option<SessionCompletedCallback>>>,
}

impl SyncService {
    pub fn new(vault_service: Arc<std::sync::RwLock<VaultService>>) -> Self {
        Self {
            vault_service,
            manager: Mutex::new(None),
            lifecycle: Mutex::new(()),
            stopping: StdMutex::new(None),
            peer_callback: Arc::new(RwLock::new(None)),
            session_callback: Arc::new(RwLock::new(None)),
        }
    }

    /// 设置入站新 peer 回调钩子（GUI 装配 `sync-pairing-request` 事件推送用）。
    pub fn set_peer_callback(&self, callback: Option<PeerCallback>) {
        if let Ok(mut guard) = self.peer_callback.write() {
            *guard = callback;
        }
    }

    /// 设置入站会话完成回调钩子（GUI 装配 `sync-completed` 事件推送用）。
    pub fn set_session_callback(&self, callback: Option<SessionCompletedCallback>) {
        if let Ok(mut guard) = self.session_callback.write() {
            *guard = callback;
        }
    }

    /// 启用或关闭同步监听。
    pub async fn enable(&self, enable: bool) -> Result<(), String> {
        let _lifecycle = self.lifecycle.lock().await;
        self.wait_until_stopped().await?;
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
            // 注入入站新 peer 回调（配对请求事件推送）与会话完成回调（完成提醒推送）
            manager.set_peer_callback(self.peer_callback.read().ok().and_then(|g| g.clone()));
            manager.set_session_callback(self.session_callback.read().ok().and_then(|g| g.clone()));
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
            *guard = Some(Arc::new(manager));
            Ok(())
        } else {
            let old_manager = guard.take();
            drop(guard);
            if let Some(manager) = old_manager {
                manager.stop();
                let completion = StopCompletion::new();
                *self
                    .stopping
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(completion.clone());
                std::mem::drop(tokio::spawn(async move {
                    let result = manager.stop_and_wait().await;
                    // completion 的成功不能早于旧 Manager/Store 真正 Drop。
                    drop(manager);
                    completion.finish(result);
                }));
            }
            self.wait_until_stopped().await?;
            if let Ok(svc) = self.vault_service.try_read() {
                if let Some(vault) = svc.get_vault_store() {
                    audit_log(&vault, "sync_disabled", None, None);
                }
            }
            Ok(())
        }
    }

    pub async fn disable_and_wait(&self) -> Result<(), String> {
        self.enable(false).await
    }

    pub async fn wait_until_stopped(&self) -> Result<(), String> {
        let completion = self
            .stopping
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        if let Some(completion) = completion {
            let result = completion.wait().await;
            let mut stopping = self
                .stopping
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if stopping
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, &completion))
            {
                *stopping = None;
            }
            result
        } else {
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
        self.sync_with_device_typed(device_id_or_addr)
            .await
            .map_err(SyncFailure::into_legacy)
    }
    pub async fn sync_with_device_typed(
        &self,
        device_id_or_addr: String,
    ) -> Result<SyncSessionResult, SyncFailure> {
        let (node_id, account_id, keys, vault, active_sessions, running, workers) = {
            let guard = self.manager.lock().await;
            // 前端经 resolveBackendErrorMessage 翻译（settings:sync_err_not_enabled）
            let manager = guard.as_ref().ok_or_else(|| {
                SyncFailure::new(SyncFailureKind::NotEnabled, "__SYNC_ERR__:not_enabled")
            })?;
            (
                manager.node_id.clone(),
                manager.account_id.clone(),
                manager.keys.clone(),
                manager.vault.clone(),
                manager.active_sessions.clone(),
                manager.running.clone(),
                manager.workers.clone(),
            )
        };
        if !running.load(Ordering::SeqCst) {
            // 前端经 resolveBackendErrorMessage 翻译（settings:sync_err_not_running）
            return Err(SyncFailure::new(
                SyncFailureKind::NotRunning,
                "__SYNC_ERR__:not_running",
            ));
        }
        let addr: SocketAddr = device_id_or_addr.parse().map_err(|e| {
            SyncFailure::new(
                SyncFailureKind::InvalidAddress,
                format!("__SYNC_ERR__:invalid_address:{}", e),
            )
        })?;

        workers
            .spawn_session(vault.clone(), active_sessions, move |permit| {
                let stream = std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(10))
                    .map_err(SyncFailure::connection)?;
                permit
                    .track_stream(&stream)
                    .map_err(SyncFailure::activity)?;
                let mut transport = SyncTransport::from_stream(stream);
                run_initiator_session(
                    &mut transport,
                    &node_id,
                    &account_id,
                    &keys,
                    vault,
                    addr.to_string(),
                )
                .map_err(SyncFailure::from_legacy_session)
            })
            .map_err(SyncFailure::activity)?
            .await
            // worker panic 时 sender 关闭（任务 panic/abort）：前端经 resolveBackendErrorMessage 翻译
            .map_err(|e| {
                SyncFailure::new(
                    SyncFailureKind::TaskUnconfirmed,
                    format!("__SYNC_ERR__:session_failed:{}", e),
                )
            })?
    }

    /// 列出已持久化的 peers（移动端发现由上层 NSD 插件维护，这里只返回持久化列表）。
    pub async fn known_peers(&self) -> Result<Vec<SyncPeerInfo>, String> {
        let svc = self
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        let vault = svc.get_vault_store().ok_or("Vault is not unlocked")?;
        let account_id = svc.get_current_account().unwrap_or_default();
        known_peers_from_vault(&vault, &account_id)
    }

    /// 标记 peer 信任状态。
    /// `fingerprint`（可选）：配对确认时绑定握手认证指纹（P001/P103）。
    pub async fn trust_peer(
        &self,
        peer_node_id: String,
        trusted: bool,
        fingerprint: Option<String>,
    ) -> Result<(), String> {
        let guard = self.manager.lock().await;
        let result = if let Some(m) = guard.as_ref() {
            m.trust_peer(&peer_node_id, trusted, fingerprint.as_deref())
        } else {
            let svc = self
                .vault_service
                .read()
                .map_err(|_| "Vault service lock poisoned".to_string())?;
            let vault = svc.get_vault_store().ok_or("Vault is not unlocked")?;
            trust_peer_fallback(&vault, &peer_node_id, trusted, fingerprint)
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
            forget_peer_fallback(&vault, &peer_node_id)
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
            local_fingerprint_fallback(&vault)
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
    workers: Arc<SyncWorkers>,
    /// 正在进行的同步会话数量。`stop()` 会等待此计数归零后再终止 worker，
    /// 避免中途 abort 正在写入 Vault 的会话导致数据不一致。
    active_sessions: Arc<AtomicUsize>,
    /// 入站新 peer 回调钩子。
    peer_callback: Arc<RwLock<Option<PeerCallback>>>,
    /// 入站会话完成回调钩子。
    session_callback: Arc<RwLock<Option<SessionCompletedCallback>>>,
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
            workers: SyncWorkers::new(),
            active_sessions: Arc::new(AtomicUsize::new(0)),
            peer_callback: Arc::new(RwLock::new(None)),
            session_callback: Arc::new(RwLock::new(None)),
        })
    }

    fn set_peer_callback(&self, callback: Option<PeerCallback>) {
        if let Ok(mut guard) = self.peer_callback.write() {
            *guard = callback;
        }
    }

    fn set_session_callback(&self, callback: Option<SessionCompletedCallback>) {
        if let Ok(mut guard) = self.session_callback.write() {
            *guard = callback;
        }
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
            .set_nonblocking(true)
            .map_err(|e| format!("set nonblocking: {}", e))?;
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
        let peer_callback = self.peer_callback.clone();
        let session_callback = self.session_callback.clone();

        // P045: accept 循环与单连接会话处理拆分为独立函数，消除 8 层嵌套。
        let workers = self.workers.clone();
        self.workers.spawn_background(vault.clone(), move || {
            run_accept_loop(
                listener,
                running,
                node_id,
                account_id,
                keys,
                vault,
                active_sessions,
                workers,
                peer_callback,
                session_callback,
            )
        })?;

        Ok(port)
    }

    fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        self.workers.close();
    }

    async fn stop_and_wait(&self) -> Result<(), String> {
        self.stop();
        let completion = self.workers.begin_stop(
            self.active_sessions.clone(),
            Duration::from_secs(STOP_GRACE_PERIOD_SECS),
        );
        let result = completion.wait().await;
        self.listen_port.store(0, Ordering::SeqCst);
        result
    }

    fn trust_peer(
        &self,
        peer_node_id: &str,
        trusted: bool,
        fingerprint: Option<&str>,
    ) -> Result<(), String> {
        trust_peer_fallback(
            &self.vault,
            peer_node_id,
            trusted,
            fingerprint.map(str::to_string),
        )
    }

    fn forget_peer(&self, peer_node_id: &str) -> Result<(), String> {
        forget_peer_fallback(&self.vault, peer_node_id)
    }
}

impl Drop for MobileSyncManager {
    fn drop(&mut self) {
        self.stop();
        self.workers.interrupt_network();
    }
}
