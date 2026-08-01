//! SyncManager: mDNS discovery, TCP listener, and encrypted sync sessions.

use crate::noise::NoiseKeys;
use crate::session::{handle_inbound, run_initiator_session};
use crate::transport::SyncTransport;
use crate::types::{SyncPeerInfo, SyncSessionResult};
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use solosoul_vault::{PeerSyncState, VaultStore};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::task::{spawn, spawn_blocking, JoinHandle};

/// 本机广播/发现的 mDNS 服务类型。
///
/// 注意：`mdns-sd` 库要求服务类型包含完整的 `.local.` 后缀；Android 的 `NsdManager`
/// 在 API 层面必须省略该后缀（底层会自动补齐），因此 Android 端常量写作 `_solosoul._tcp`。
/// 两者在网络层等价，这里保留完整后缀以满足 Rust 端库的要求。
const SERVICE_TYPE: &str = "_solosoul._tcp.local.";
const MDNS_TIMEOUT_MS: u64 = 200;
const PEER_MAX_AGE_SECS: u64 = 300;
/// `stop()` 等待正在进行的同步会话完成的最大时长（秒）。
/// 超时后仍会强制 abort，避免无限等待恶意/僵死的 peer。
const STOP_GRACE_PERIOD_SECS: u64 = 30;
/// `stop()` 轮询 `active_sessions` 的间隔。
const STOP_POLL_INTERVAL_MS: u64 = 100;

/// RAII guard：创建时递增 `active_sessions`，Drop 时递减。
/// 确保 `stop()` 能感知当前有多少同步会话正在进行。
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

#[derive(Debug, Clone)]
struct DiscoveredPeer {
    node_id: String,
    account_id: String,
    name: String,
    addr: SocketAddr,
    fingerprint: String,
    last_seen: Instant,
}

/// Central manager for peer-to-peer synchronization.
pub struct SyncManager {
    node_id: String,
    account_id: String,
    keys: NoiseKeys,
    vault: Arc<VaultStore>,
    listen_addr: String,
    listen_port: AtomicU16,
    running: Arc<AtomicBool>,
    discovered: Arc<Mutex<HashMap<String, DiscoveredPeer>>>,
    mdns_daemon: Mutex<Option<ServiceDaemon>>,
    /// 当前 mDNS daemon 是否为外部共享实例（GUI 复用 discovery 的 SharedDaemon）。
    /// true 时 `stop()` 只注销本节点服务注册，不调用 `shutdown()`，避免关掉
    /// 供发现/恢复命令共用的 app 生命周期 daemon（P013：进程内只保留一个）。
    shared_daemon: AtomicBool,
    worker_handles: Mutex<Vec<JoinHandle<()>>>,
    /// 正在进行的同步会话数量。`stop()` 会等待此计数归零后再终止 worker，
    /// 避免中途 abort 正在写入 Vault 的会话导致数据不一致。
    active_sessions: Arc<AtomicUsize>,
}

impl SyncManager {
    pub fn new(
        node_id: String,
        account_id: String,
        keys: NoiseKeys,
        vault: Arc<VaultStore>,
        listen_addr: &str,
    ) -> Self {
        Self {
            node_id,
            account_id,
            keys,
            vault,
            listen_addr: listen_addr.to_string(),
            listen_port: AtomicU16::new(0),
            running: Arc::new(AtomicBool::new(false)),
            discovered: Arc::new(Mutex::new(HashMap::new())),
            mdns_daemon: Mutex::new(None),
            shared_daemon: AtomicBool::new(false),
            worker_handles: Mutex::new(Vec::new()),
            active_sessions: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Returns the short public-key fingerprint for manual verification.
    pub fn fingerprint(&self) -> String {
        self.keys.fingerprint()
    }

    pub fn listen_port(&self) -> u16 {
        self.listen_port.load(Ordering::SeqCst)
    }

    /// Start the TCP listener and mDNS discovery/advertisement,
    /// creating a process-private `ServiceDaemon`（CLI / 无共享 daemon 场景）。
    pub async fn start(&self) -> Result<u16, String> {
        let mdns_daemon = ServiceDaemon::new().map_err(|e| format!("mdns daemon: {}", e))?;
        self.start_inner(mdns_daemon, false).await
    }

    /// Start using an externally owned (shared) mDNS daemon, so the whole process
    /// only ever has one `ServiceDaemon` alive.
    ///
    /// GUI 场景：`sync_enable` 把 discovery 命令共用的 `SharedDaemon` 传入，
    /// 避免 `SyncManager` 自建第二个 daemon 与发现命令同时运行导致结果不一致
    /// （P013）。`stop()` 对共享 daemon 只注销本节点服务，不调用 `shutdown()`。
    pub async fn start_with_daemon(&self, mdns_daemon: ServiceDaemon) -> Result<u16, String> {
        self.start_inner(mdns_daemon, true).await
    }

    async fn start_inner(&self, mdns_daemon: ServiceDaemon, shared: bool) -> Result<u16, String> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(self.listen_port.load(Ordering::SeqCst));
        }
        self.running.store(true, Ordering::SeqCst);
        self.shared_daemon.store(shared, Ordering::SeqCst);

        let listener =
            TcpListener::bind(&self.listen_addr).map_err(|e| format!("bind failed: {}", e))?;
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
        let active_sessions = self.active_sessions.clone();
        // TCP accept loop (blocking std listener)
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

        // mDNS
        self.register_mdns(&mdns_daemon, port)?;
        *self.mdns_daemon.lock().unwrap_or_else(|e| e.into_inner()) = Some(mdns_daemon.clone());

        let mdns_handle = self.spawn_mdns_discovery(mdns_daemon);

        if let Ok(mut handles) = self.worker_handles.lock() {
            handles.push(accept_handle);
            handles.push(mdns_handle);
        }

        Ok(port)
    }

    /// Stop all background workers and mDNS.
    ///
    /// 先将 `running` 置为 false 阻止新会话进入，然后等待正在进行的同步会话
    /// 完成（最多 `STOP_GRACE_PERIOD_SECS` 秒），最后才 abort worker 任务。
    /// 这避免了在 `apply_sync_records` 写入 Vault 时被 `abort()` 中断而导致
    /// 数据不一致的风险。
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);

        // 立即注销/关闭 mDNS daemon：必须在会话优雅等待之前执行。
        // 共享 daemon 场景下，若等到 30s 优雅期结束才 unregister，快速「禁用→启用」
        // 时新 manager 会在同一 daemon 上注册同名实例名，触发 duplicate 错误
        // （P013 审查反馈）。daemon 的 unregister/register 经内部通道串行，先提交
        // unregister 即可消除竞态窗口。
        if let Ok(mut daemon) = self.mdns_daemon.lock() {
            if let Some(d) = daemon.take() {
                if self.shared_daemon.load(Ordering::SeqCst) {
                    // 共享 daemon 属于 discovery/恢复层，只注销本节点的同步服务注册，
                    // 保留 daemon 供 `mdns_discover` 等命令继续使用。
                    let _ = d.unregister(&format!("{}.{}", self.node_id, SERVICE_TYPE));
                } else {
                    let _ = d.shutdown();
                }
            }
        }

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
                    "SyncManager.stop(): {} session(s) still active after {}s grace period, forcing abort",
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

    /// 测试专用：模拟存在 N 个活跃同步会话，验证 `stop()` 的等待逻辑。
    #[cfg(test)]
    pub(crate) fn set_active_sessions_for_test(&self, n: usize) {
        self.active_sessions.store(n, Ordering::SeqCst);
    }

    /// 测试专用：返回活跃会话计数器，便于测试在后台 `stop()` 期间复位。
    #[cfg(test)]
    pub(crate) fn active_sessions_counter(&self) -> Arc<AtomicUsize> {
        self.active_sessions.clone()
    }

    fn register_mdns(&self, daemon: &ServiceDaemon, port: u16) -> Result<(), String> {
        let ips: Vec<IpAddr> = Self::local_ips().into_iter().map(IpAddr::V4).collect();
        if ips.is_empty() {
            return Err("No local IP address available for mDNS".to_string());
        }
        let mut txt = HashMap::<String, String>::new();
        // node_id 是随机 UUID，本身不泄露用户身份，保留明文用于 peer 标识。
        txt.insert("node_id".to_string(), self.node_id.clone());
        // account_id 直接标识用户账户，属于敏感信息。
        // 广播 SHA-256 哈希（前 16 字节 hex）而非原始值，
        // 发现方通过比较哈希过滤同一账户的 peer，无需知道对方原始 account_id。
        txt.insert(
            "account_hash".to_string(),
            sha256_hex_short(&self.account_id),
        );
        // fingerprint 是公钥指纹，用于 MITM 验证，必须明文传输。
        txt.insert("fingerprint".to_string(), self.keys.fingerprint());

        let hostname = format!("{}.local.", self.node_id);
        let service = ServiceInfo::new(
            SERVICE_TYPE,
            &self.node_id,
            &hostname,
            &ips[..],
            port,
            txt.clone(),
        )
        .map_err(|e| format!("ServiceInfo: {}", e))?;
        let fullname = format!("{}.{}", self.node_id, SERVICE_TYPE);
        match daemon.register(service) {
            Ok(_) => Ok(()),
            // 共享 daemon 场景下，快速「禁用→启用」时旧实例的 unregister 可能尚未落地，
            // mdns-sd 对同名实例重复注册会报 already registered。幂等处理：先注销再重试一次。
            Err(e) => {
                tracing::debug!("mDNS register failed ({e}), retrying after unregister");
                if let Err(ue) = daemon.unregister(&fullname) {
                    // 保留原始 register 错误，避免 pre-unregister 错误掩盖根因
                    tracing::warn!(
                        "mDNS pre-unregister failed ({ue}); original register error: {e}"
                    );
                    return Err(format!(
                        "mDNS register: {} (pre-unregister failed: {})",
                        e, ue
                    ));
                }
                let service =
                    ServiceInfo::new(SERVICE_TYPE, &self.node_id, &hostname, &ips[..], port, txt)
                        .map_err(|e| format!("ServiceInfo: {}", e))?;
                daemon
                    .register(service)
                    .map_err(|e| format!("mDNS register (retry): {}", e))
            }
        }
    }

    fn spawn_mdns_discovery(&self, daemon: ServiceDaemon) -> JoinHandle<()> {
        let running = self.running.clone();
        let discovered = self.discovered.clone();
        // 预计算本地 account_id 的哈希，用于与 mDNS TXT 中的 account_hash 比对。
        let local_account_hash = sha256_hex_short(&self.account_id);

        spawn(async move {
            let receiver = match daemon.browse(SERVICE_TYPE) {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("mDNS browse failed: {}", e);
                    return;
                }
            };
            // 记录上次清理过期 peer 的时间，避免每次迭代都扫描整个 map。
            let mut last_cleanup = Instant::now();
            const CLEANUP_INTERVAL_SECS: u64 = 60;

            while running.load(Ordering::SeqCst) {
                match receiver.recv_timeout(Duration::from_millis(MDNS_TIMEOUT_MS)) {
                    Ok(ServiceEvent::ServiceResolved(info)) => {
                        let props = info.get_properties();
                        // 比较 account_hash 而非原始 account_id，
                        // 兼容旧版客户端广播的 account_id 字段。
                        let peer_account_hash = props
                            .get("account_hash")
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| {
                                // 旧版客户端仍广播 account_id 明文，计算其哈希比较。
                                props
                                    .get("account_id")
                                    .map(|v| sha256_hex_short(&v.to_string()))
                                    .unwrap_or_default()
                            });
                        if peer_account_hash != local_account_hash {
                            continue;
                        }
                        let node_id = props
                            .get("node_id")
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| info.get_fullname().to_string());
                        let fingerprint = props
                            .get("fingerprint")
                            .map(|v| v.to_string())
                            .unwrap_or_default();
                        let addrs: Vec<IpAddr> = info.get_addresses().iter().cloned().collect();
                        if let Some(addr) = addrs.first() {
                            let socket = SocketAddr::new(*addr, info.get_port());
                            let peer = DiscoveredPeer {
                                node_id: node_id.clone(),
                                // account_id 不再从 mDNS 获取；
                                // 使用本地 account_id（同一账户的 peer 才会被发现）。
                                account_id: String::new(),
                                name: info.get_fullname().to_string(),
                                addr: socket,
                                fingerprint,
                                last_seen: Instant::now(),
                            };
                            if let Ok(mut map) = discovered.lock() {
                                map.insert(node_id, peer);
                            }
                        }
                    }
                    Ok(ServiceEvent::ServiceRemoved(_, fullname)) => {
                        if let Ok(mut map) = discovered.lock() {
                            map.retain(|_, p| p.name != fullname);
                        }
                    }
                    _ => {}
                }

                // 定期清理过期的 discovered peer 条目。
                // peer 崩溃时不发送 mDNS goodbye，导致 ServiceRemoved 事件缺失，
                // 条目会永远留在 map 中。这里每 60 秒扫描一次，移除超过
                // PEER_MAX_AGE_SECS 未更新的条目，防止 HashMap 无限增长。
                if last_cleanup.elapsed().as_secs() >= CLEANUP_INTERVAL_SECS {
                    last_cleanup = Instant::now();
                    if let Ok(mut map) = discovered.lock() {
                        let now = Instant::now();
                        map.retain(|_, p| {
                            now.duration_since(p.last_seen).as_secs() <= PEER_MAX_AGE_SECS
                        });
                    }
                }
            }
        })
    }

    /// Connect to a discovered peer or a direct `host:port` address and sync.
    pub async fn sync_with_peer(
        &self,
        device_id_or_addr: &str,
    ) -> Result<SyncSessionResult, String> {
        if !self.running.load(Ordering::SeqCst) {
            return Err("Sync manager is not running".to_string());
        }
        let addr = if let Ok(socket) = device_id_or_addr.parse::<SocketAddr>() {
            socket
        } else {
            let map = self.discovered.lock().map_err(|e| e.to_string())?;
            resolve_peer_addr(&map, device_id_or_addr)?
        };

        let node_id = self.node_id.clone();
        let account_id = self.account_id.clone();
        let keys = self.keys.clone();
        let vault = self.vault.clone();
        let active_sessions = self.active_sessions.clone();

        let result = spawn_blocking(move || {
            let _guard = SessionGuard::new(active_sessions);
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
        .map_err(|e| format!("spawn blocking: {}", e))?;

        result
    }

    /// Return discovered and persisted peers visible right now.
    pub fn known_peers(&self) -> Result<Vec<SyncPeerInfo>, String> {
        let mut out = Vec::new();
        let now = Instant::now();
        let map = self.discovered.lock().map_err(|e| e.to_string())?;
        for (_, p) in map.iter() {
            if now.duration_since(p.last_seen).as_secs() > PEER_MAX_AGE_SECS {
                continue;
            }
            out.push(SyncPeerInfo {
                node_id: p.node_id.clone(),
                account_id: p.account_id.clone(),
                name: p.name.clone(),
                addr: p.addr.to_string(),
                fingerprint: p.fingerprint.clone(),
                trusted: self.is_peer_trusted(&p.node_id)?,
                last_seen: format_duration_since(p.last_seen),
            });
        }
        // Include persisted peers even if not currently discovered.
        let persisted = self.vault.list_peers()?;
        for p in persisted {
            if out.iter().any(|i| i.node_id == p.peer_node_id) {
                continue;
            }
            out.push(SyncPeerInfo {
                node_id: p.peer_node_id.clone(),
                account_id: self.account_id.clone(),
                name: p
                    .peer_name
                    .clone()
                    .unwrap_or_else(|| p.peer_node_id.clone()),
                addr: String::new(),
                fingerprint: p.public_key_fingerprint.clone().unwrap_or_default(),
                trusted: p.trusted,
                last_seen: String::new(),
            });
        }
        Ok(out)
    }

    fn is_peer_trusted(&self, peer_node_id: &str) -> Result<bool, String> {
        match self.vault.load_peer_state(peer_node_id)? {
            Some(p) => Ok(p.trusted),
            None => Ok(false),
        }
    }

    /// Mark a peer as trusted or untrusted.
    pub fn trust_peer(&self, peer_node_id: &str, trusted: bool) -> Result<(), String> {
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

    /// Remove a peer from persisted state.
    pub fn forget_peer(&self, peer_node_id: &str) -> Result<(), String> {
        self.vault.delete_peer(peer_node_id)
    }

    fn local_ips() -> Vec<Ipv4Addr> {
        let mut ips = Vec::new();
        // Best-effort primary local IP via UDP connect to a public address.
        // 这不发送任何数据包，仅利用内核路由表确定出口 IP。
        // 但在离线局域网（无互联网连接）时此方法会失败。
        if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
            if socket.connect("8.8.8.8:80").is_ok() {
                if let Ok(local) = socket.local_addr() {
                    if let IpAddr::V4(v4) = local.ip() {
                        if !v4.is_loopback() {
                            ips.push(v4);
                        }
                    }
                }
            }
        }
        // Fallback：当 UDP 探测失败时（离线局域网、无默认路由），
        // 通过 local_ip_address crate 枚举本地网卡获取非回环 IPv4。
        if ips.is_empty() {
            if let Ok(std::net::IpAddr::V4(v4)) = local_ip_address::local_ip() {
                if !v4.is_loopback() {
                    ips.push(v4);
                }
            }
        }
        // Always include loopback for local testing.
        ips.push(Ipv4Addr::new(127, 0, 0, 1));
        ips
    }
}

/// 解析 peer 连接地址：支持 `node_id`（discovered map 按键）与裸 IP（无端口）。
///
/// 桌面端旧版 mDNS 广播裸 IP（无端口），前端把 `addresses[0]` 原样传入；
/// 裸 IP 无法解析为 SocketAddr，若直接查 discovered map 会因键不匹配报
/// "Peer not discovered"（Bug A）。这里在裸 IP 场景下按 `peer.addr.ip()`
/// 匹配 discovered map 中任意地址相同的 peer，命中则使用其完整 `addr`（含端口）。
fn resolve_peer_addr(
    discovered: &HashMap<String, DiscoveredPeer>,
    device_id_or_addr: &str,
) -> Result<SocketAddr, String> {
    // 输入是裸 IP（无端口）时，按 IP 匹配 discovered peer
    if let Ok(ip) = device_id_or_addr.parse::<IpAddr>() {
        if let Some(peer) = discovered.values().find(|p| p.addr.ip() == ip) {
            return Ok(peer.addr);
        }
    }
    discovered
        .get(device_id_or_addr)
        .map(|p| p.addr)
        .ok_or_else(|| format!("__SYNC_ERR__:peer_not_discovered:{}", device_id_or_addr))
}

/// 计算 SHA-256 哈希并返回前 16 字节的 hex 编码（32 字符）。
/// 用于在 mDNS TXT 记录中广播 account_id 的哈希值，避免泄露原始账户标识。
/// 截断为 16 字节（128 位）在局域网发现场景下碰撞风险极低且不影响安全性
///（真正的身份验证在 Noise 握手阶段完成）。
fn sha256_hex_short(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(&hasher.finalize()[..16])
}

fn format_duration_since(instant: Instant) -> String {
    let secs = instant.elapsed().as_secs();
    if secs < 60 {
        format!("{}s ago", secs)
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else {
        format!("{}h ago", secs / 3600)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer(node_id: &str, ip: [u8; 4], port: u16) -> (String, DiscoveredPeer) {
        (
            node_id.to_string(),
            DiscoveredPeer {
                node_id: node_id.to_string(),
                account_id: String::new(),
                name: node_id.to_string(),
                addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3])), port),
                fingerprint: String::new(),
                last_seen: Instant::now(),
            },
        )
    }

    fn insert_peer(
        map: &mut HashMap<String, DiscoveredPeer>,
        node_id: &str,
        ip: [u8; 4],
        port: u16,
    ) {
        let (id, p) = peer(node_id, ip, port);
        map.insert(id, p);
    }

    #[test]
    fn test_resolve_peer_addr_by_node_id() {
        let mut map = HashMap::new();
        insert_peer(&mut map, "node-a", [192, 168, 0, 33], 42069);
        insert_peer(&mut map, "node-b", [192, 168, 0, 34], 42070);

        let addr = resolve_peer_addr(&map, "node-b").unwrap();
        assert_eq!(addr, "192.168.0.34:42070".parse::<SocketAddr>().unwrap());
    }

    /// Bug A 回归：桌面端旧版广播裸 IP（无端口），应按 IP 匹配 discovered peer。
    #[test]
    fn test_resolve_peer_addr_by_bare_ip_fallback() {
        let mut map = HashMap::new();
        insert_peer(&mut map, "node-a", [192, 168, 0, 33], 42069);
        insert_peer(&mut map, "node-b", [192, 168, 0, 34], 42070);

        let addr = resolve_peer_addr(&map, "192.168.0.34").unwrap();
        assert_eq!(addr, "192.168.0.34:42070".parse::<SocketAddr>().unwrap());
    }

    #[test]
    fn test_resolve_peer_addr_unknown_returns_i18n_error() {
        let mut map = HashMap::new();
        insert_peer(&mut map, "node-a", [192, 168, 0, 33], 42069);

        let err = resolve_peer_addr(&map, "10.0.0.99").unwrap_err();
        assert!(
            err.starts_with("__SYNC_ERR__:peer_not_discovered:"),
            "got: {}",
            err
        );
        assert!(err.contains("10.0.0.99"));

        let err2 = resolve_peer_addr(&map, "no-such-node").unwrap_err();
        assert!(
            err2.starts_with("__SYNC_ERR__:peer_not_discovered:"),
            "got: {}",
            err2
        );
    }

    #[test]
    fn test_resolve_peer_addr_ipv6_bare_ip_fallback() {
        let mut map = HashMap::new();
        map.insert(
            "node-v6".to_string(),
            DiscoveredPeer {
                node_id: "node-v6".to_string(),
                account_id: String::new(),
                name: "node-v6".to_string(),
                addr: "[fe80::1]:42069".parse::<SocketAddr>().unwrap(),
                fingerprint: String::new(),
                last_seen: Instant::now(),
            },
        );

        // 裸 IPv6 按 IP 回退（ip:port 直连在 sync_with_peer 的 SocketAddr 分支处理，
        // resolve_peer_addr 只负责裸 IP / node_id）
        let addr2 = resolve_peer_addr(&map, "fe80::1").unwrap();
        assert_eq!(addr2, "[fe80::1]:42069".parse::<SocketAddr>().unwrap());
    }
}
