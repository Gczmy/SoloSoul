//! 本地设备发现（mDNS）。
//!
//! 桌面端使用 `mdns-sd` 实现服务发现与广播；移动端（Android/iOS）暂不提供该功能，
//! 二期可替换为 Android NSD / iOS Bonjour。

#[cfg(desktop)]
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::Serialize;
use solosoul_sync::sha256_hex_short;
use std::sync::Arc;
#[cfg(mobile)]
use tauri::Manager;
use tokio::sync::Mutex;

use crate::state::AppState;

/// 桌面端 mDNS 服务类型（完整域名后缀）。
///
/// 注意：Android 的 `NsdManager` 必须省略 `.local.` 后缀（底层会自动补齐），因此
/// `NsdPlugin.kt` 中写作 `_solosoul._tcp`。两者在网络层等价，这里保留完整后缀
/// 以满足 `mdns-sd` 库的要求。修改本常量时需同步检查 Android 端常量。
const MDNS_SERVICE_TYPE: &str = "_solosoul._tcp.local.";
/// 恢复设备的 mDNS 服务类型（与同步类型区分，便于新设备只发现恢复主机）。
const RECOVERY_MDNS_SERVICE_TYPE: &str = "_solosoul_recovery._tcp.local.";
/// 不含域后缀的服务类型基础名，用于跨平台一致性校验。
#[cfg(test)]
const MDNS_SERVICE_TYPE_BASE: &str = "_solosoul._tcp";
const MDNS_MAX_TIMEOUT_MS: u64 = 30_000;
const MDNS_POLL_INTERVAL_MS: u64 = 200;

/// Wrapper to share ServiceDaemon across commands (prevents mem::forget leaks)
#[cfg(desktop)]
pub struct SharedDaemon(Arc<Mutex<Option<ServiceDaemon>>>);

#[cfg(mobile)]
pub struct SharedDaemon(Arc<Mutex<Option<()>>>);

/// 移动端 NSD 生命周期操作（权限申请/注册/注销）的互斥锁。
///
/// `run_mobile_plugin` 是同步 IPC，会阻塞调用线程等待 Android 主线程响应。
/// 多次快速切换“启用/禁用”时，后台注册任务与禁用注销任务可能并发执行，
/// 在 Android 主线程串行排队时互相阻塞；用一把锁串行化 NSD 生命周期操作，
/// 避免 register 与 unregister 交错导致半开启/半关闭状态。
#[cfg(mobile)]
pub(crate) static NSD_LIFECYCLE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

impl Default for SharedDaemon {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedDaemon {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(None)))
    }

    /// Get or create the daemon. The daemon lives for the app lifetime.
    #[cfg(desktop)]
    pub async fn get(&self) -> Result<Arc<Mutex<Option<ServiceDaemon>>>, String> {
        let mut guard = self.0.lock().await;
        if guard.is_none() {
            *guard = Some(ServiceDaemon::new().map_err(|e| format!("mDNS daemon: {}", e))?);
        }
        Ok(self.0.clone())
    }

    #[cfg(mobile)]
    pub async fn get(&self) -> Result<Arc<Mutex<Option<()>>>, String> {
        Ok(self.0.clone())
    }
}

/// 通过 mDNS 发现的恢复主机信息。
///
/// 安全约束：mDNS TXT 广播**不携带** PIN 与 nonce（二者仅经 QR 码/手动输入
/// 带外传递）。此前将 PIN+nonce 写入明文 TXT，局域网内任意主机浏览
/// `_solosoul_recovery._tcp.local.` 即可直接通过认证下载恢复包（完整 Vault
/// 失陷）。发现到主机后，PIN 由用户从主机屏幕/QR 手动输入。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryDiscoveredHost {
    /// 主机显示名称（由主机指纹截取生成）。
    pub name: String,
    /// 连接地址（host:port）。
    pub addr: String,
    /// 主机公钥指纹（用于 MITM 验证）。
    pub fingerprint: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredDevice {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub addresses: Vec<String>,
    /// 对端公钥指纹（mDNS TXT 广播，用于前端「已发现设备」详情与已知设备匹配）。
    /// 旧版对端/未解析时为空串。
    #[serde(default)]
    pub fingerprint: String,
    /// 对端客户端类型（macos/windows/linux/android/ios/unknown）。
    /// 优先来自 TXT 广播；旧版对端回退按 node_id 查本机 vault peer 记录；
    /// 从未同步过的设备为 unknown（前端兜底显示通用图标）。
    pub client_type: String,
}

#[cfg(desktop)]
#[tauri::command]
pub async fn mdns_discover(
    state: tauri::State<'_, AppState>,
    daemon: tauri::State<'_, SharedDaemon>,
    timeout_ms: u64,
) -> Result<Vec<DiscoveredDevice>, String> {
    let timeout_ms = timeout_ms.min(MDNS_MAX_TIMEOUT_MS);
    // P2：发现层过滤——只展示同一账户且非本机的设备，避免用户点击
    // 其他账户的设备或本机触发无意义的 TCP 直连与英文握手报错。
    // Vault 未解锁时无法确定本地身份，退化为不过滤（保持旧行为）。
    let local_account_hash =
        crate::commands::current_account_optional(&state).map(|id| sha256_hex_short(&id));
    let local_node_id = crate::commands::vault_handle(&state)
        .ok()
        .and_then(|v| v.get_sync_node_id().ok().flatten());

    let daemon_arc = daemon.get().await?;
    let guard = daemon_arc.lock().await;
    let daemon = guard.as_ref().ok_or("mDNS daemon not initialized")?;

    let receiver = daemon
        .browse(MDNS_SERVICE_TYPE)
        .map_err(|e| format!("Browse: {}", e))?;

    let mut devices = Vec::new();
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);

    while std::time::Instant::now() < deadline {
        if let Ok(ServiceEvent::ServiceResolved(info)) =
            receiver.recv_timeout(std::time::Duration::from_millis(MDNS_POLL_INTERVAL_MS))
        {
            // P2：解析 TXT 属性做账户/本机过滤（与 SyncManager 内部 account_hash 过滤一致）。
            // P031: TXT 解析收敛到 solosoul_sync::shared::parse_mdns_txt 单一实现。
            let txt = solosoul_sync::shared::parse_mdns_txt(info.get_properties());
            let peer_account_hash = txt.account_hash;
            let peer_account_id = txt.account_id;
            let peer_node_id = txt.node_id;
            let peer_fingerprint = txt.fingerprint;
            let peer_client_type_txt = txt.client_type;
            if !should_show_device(
                &peer_account_hash,
                &peer_account_id,
                &peer_node_id,
                local_account_hash.as_deref(),
                local_node_id.as_deref(),
            ) {
                continue;
            }

            // 桌面端 addresses 统一为 ip:port 形状（与移动端一致），
            // 前端 SyncPage 直接取 addresses[0] 传给 sync_with_device，
            // 裸 IP 无法被 SyncManager 解析为 SocketAddr 导致 "Peer not discovered"。
            let addresses = format_discovered_addresses(info.get_addresses(), info.get_port());
            let display_name = discovered_display_name(&peer_fingerprint, info.get_hostname());
            // 客户端类型：优先 TXT 广播（新对端也能正确显示图标）；旧版对端无
            // client_type TXT 时按 node_id 查本机 vault peer 记录（已同步过的设备
            // 才有记录）。查询失败静默降级为 unknown。
            let client_type =
                resolve_peer_client_type(&peer_client_type_txt, &peer_node_id, &state);
            devices.push(DiscoveredDevice {
                name: display_name,
                host: info.get_hostname().to_string(),
                port: info.get_port(),
                addresses,
                fingerprint: peer_fingerprint,
                client_type,
            });
        }
    }

    // mDNS 可能对同一服务多次触发 ServiceResolved，按 host:port 去重，
    // 避免前端设备列表出现重复条目。
    let mut seen = std::collections::HashSet::new();
    devices.retain(|d| seen.insert(format!("{}:{}", d.host, d.port)));
    Ok(devices)
}

/// 已发现设备的显示名：优先用 TXT fingerprint 派生 `SoloSoul-<fp8>`（与移动端
/// NSD 注册名、桌面广播实例名一致）；旧版对端无指纹 TXT 时回退清理后的主机名
/// （剥掉 `.local.` 后缀），避免 `node_<uuid>` 全名（含 `._solosoul._tcp.local.`
/// 后缀）溢出设备卡片。纯函数，供单测覆盖。
#[cfg(desktop)]
fn discovered_display_name(fingerprint: &str, hostname: &str) -> String {
    if !fingerprint.is_empty() {
        format!("SoloSoul-{}", &fingerprint[..fingerprint.len().min(8)])
    } else {
        hostname.trim_end_matches(".local.").to_string()
    }
}

/// P044: 解析对端客户端类型——优先 TXT 广播值（新对端也能正确显示图标）；旧版对端
/// 无 client_type 时按 node_id 查本机 vault peer 记录（已同步过的设备才有记录），
/// 查询失败静默降级为 unknown。桌面/移动两端 mdns_discover 共用。
fn resolve_peer_client_type(txt_client_type: &str, peer_node_id: &str, state: &AppState) -> String {
    if !txt_client_type.is_empty() {
        return txt_client_type.to_string();
    }
    crate::commands::vault_handle(state)
        .ok()
        .and_then(|v| {
            v.load_peer_state(peer_node_id)
                .ok()
                .flatten()
                .and_then(|p| p.client_type)
        })
        .unwrap_or_else(|| "unknown".to_string())
}

/// P2：判断发现的设备是否应展示给用户——同一账户（account_hash 比对，兼容旧版
/// 明文 account_id 广播）且非本机（node_id 不相同）。本地身份未知时不过滤。
#[cfg(desktop)]
fn should_show_device(
    peer_account_hash: &str,
    peer_account_id: &str,
    peer_node_id: &str,
    local_account_hash: Option<&str>,
    local_node_id: Option<&str>,
) -> bool {
    // 账户过滤：本地账户未知（Vault 未解锁）时不过滤
    if let Some(local_hash) = local_account_hash {
        if peer_account_hash.is_empty() && peer_account_id.is_empty() {
            // Android NsdManager 广播的 TXT 属性（account_hash/account_id）存在已知的
            // 互操作限制：经常不传播到标准 mDNS 客户端（桌面 mdns-sd），表现为「安卓能
            // 发现 mac、mac 却找不到安卓」。无账户信息的服务无法按账户过滤，放行展示——
            // 会话层仍会严格校验 account_id（Account mismatch 拒绝对端），配对流程有
            // SAS 验证码兜底，安全不受影响；仅 UI 上可能出现无法连接的其他设备。
        } else if peer_account_hash.is_empty() {
            // 旧版客户端广播明文 account_id，取其哈希比对
            if sha256_hex_short(peer_account_id) != local_hash {
                return false;
            }
        } else if peer_account_hash != local_hash {
            // 直接比较 &str，避免 peer_account_hash.to_string() 多余分配
            return false;
        }
    }
    // 本机过滤：peer 有 node_id 且与本地相同则跳过
    if let (Some(local_node), peer_node) = (local_node_id, peer_node_id) {
        if !peer_node.is_empty() && peer_node == local_node {
            return false;
        }
    }
    true
}

#[cfg(mobile)]
#[tauri::command]
pub async fn mdns_discover(
    app: tauri::AppHandle,
    _daemon: tauri::State<'_, SharedDaemon>,
    timeout_ms: u64,
) -> Result<Vec<DiscoveredDevice>, String> {
    let timeout_ms = timeout_ms.min(MDNS_MAX_TIMEOUT_MS);

    // P2：发现层过滤——本机账户/节点身份。Vault 未解锁时无法确定本地身份，
    // 退化为不过滤（保持旧行为）。
    let app_state = app.state::<AppState>();
    let local_account_id = crate::commands::current_account_optional(&app_state);
    // 本地账户哈希：桌面端广播 account_hash，移动端 NSD 广播明文 account_id。
    // 发现过滤需要同时兼容两种来源（Android 扫 macOS / Android 互扫）。
    let local_account_hash = local_account_id.as_deref().map(sha256_hex_short);
    // 移动端广播的 node_id 是设备名（SoloSoul-{fingerprint 前 8 位}），非 vault node id，
    // 用于排除本机。指纹未知时跳过本机过滤。
    let local_device_name = match app_state.sync_service.local_fingerprint().await {
        Ok(fp) if !fp.is_empty() => Some(format!("SoloSoul-{}", &fp[..fp.len().min(8)])),
        _ => None,
    };

    // NSD 插件所有方法（request_permissions/start_discovery/get_discovered_services/
    // stop_discovery）底层都是 run_mobile_plugin 同步 IPC，会阻塞调用线程等待 Android
    // 主线程响应。若在 async 命令线程直接调用，权限弹窗未响应时 request_permissions
    // 会一直阻塞并占住 Tokio worker；多次开关同步后 worker 被占满，sync_enable /
    // sync_get_status 等命令全部排队，前端表现为“禁用同步后所有按钮卡住”。
    // 因此整个 NSD 交互移入 spawn_blocking 阻塞线程池，并用兜底超时保证命令按时返回。
    let app2 = app.clone();
    let task = tokio::task::spawn_blocking(move || -> Result<Vec<DiscoveredDevice>, String> {
        let handle = app2.state::<crate::nsd_plugin::NsdPluginHandle<tauri::Wry>>();
        // 与 sync_enable 的注册/注销共用生命周期锁，串行化权限弹窗与注册流程
        let _guard = NSD_LIFECYCLE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        handle.request_permissions()?;
        handle.start_discovery()?;

        // 轮询等待 NSD 发现结果，避免立即读取返回空列表
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
        let mut last_count = 0usize;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(MDNS_POLL_INTERVAL_MS));
            let services = handle.get_discovered_services()?;
            let count = services.len();
            // 找到新服务或超时则退出
            if count > last_count || std::time::Instant::now() >= deadline {
                break;
            }
            last_count = count;
        }

        let services = handle.get_discovered_services()?;
        handle.stop_discovery()?;
        Ok(services
            .into_iter()
            .filter(|s| {
                // P2：同一账户（本地账户已知时才过滤）+ 非本机
                let same_account = local_account_id
                    .as_deref()
                    .zip(local_account_hash.as_deref())
                    .map(|(acc, acc_hash)| {
                        // 桌面端 mDNS 广播 account_hash（无 account_id），移动端 NSD 广播
                        // 明文 account_id（无 account_hash）。两种来源都兼容：有 account_hash
                        // 按哈希比对（Android 扫 macOS），否则回退明文 account_id 比对（
                        // Android 互扫）。不匹配任一来源的旧版服务（无账户信息）会被隐藏。
                        if !s.account_hash.is_empty() {
                            s.account_hash == acc_hash
                        } else {
                            s.account_id == acc
                        }
                    })
                    .unwrap_or(true);
                let not_self = local_device_name
                    .as_deref()
                    .map(|name| s.node_id != name)
                    .unwrap_or(true);
                same_account && not_self
            })
            .map(|s| {
                // 客户端类型：优先 TXT 广播（新对端也能正确显示图标；旧版对端无
                // client_type 时回退按 node_id 查本机 vault peer 记录）。
                // State 句柄不能移入 'static 闭包，经 app2（AppHandle）重新取。
                let app_state = app2.state::<AppState>();
                let client_type = resolve_peer_client_type(&s.client_type, &s.node_id, &app_state);
                DiscoveredDevice {
                    // 显示名优先指纹派生 SoloSoul-<fp8>（与桌面端 discovered_display_name /
                    // NSD 注册名 / QR 卡片命名规则一致），其次 NSD 服务名（桌面广播实例名
                    // 即 SoloSoul-<fp8>），最后回退 TXT node_id。修复前直接用 node_id
                    // （桌面端 TXT node_id 为 node_<uuid>），导致安卓端已发现列表显示
                    // `node_f2c22bc6…` 而非可读设备名。
                    name: if !s.fingerprint.is_empty() {
                        format!("SoloSoul-{}", &s.fingerprint[..s.fingerprint.len().min(8)])
                    } else if !s.service_name.is_empty() {
                        s.service_name.clone()
                    } else {
                        s.node_id.clone()
                    },
                    host: s.host.clone(),
                    port: s.port,
                    addresses: vec![format!("{}:{}", s.host, s.port)],
                    fingerprint: s.fingerprint.clone(),
                    client_type,
                }
            })
            .collect())
    });

    // 兜底超时：即使权限弹窗未响应导致内部阻塞，命令也会按时返回。
    // 注意类型嵌套：timeout 返回 Result<_, Elapsed>，JoinHandle 输出
    // Result<_, JoinError>，闭包返回 Result<Vec<_>, String>，共三层。
    match tokio::time::timeout(std::time::Duration::from_millis(timeout_ms + 10_000), task).await {
        Ok(Ok(Ok(devices))) => Ok(devices),
        Ok(Ok(Err(e))) => Err(e),
        Ok(Err(e)) => Err(format!("discovery task failed: {e}")),
        Err(_) => Err("__SYNC_ERR__:discovery_timeout".to_string()),
    }
}

// ── 恢复 mDNS 广告与发现 ────────────────────────────────────────────────────

/// 注册恢复主机的 mDNS 广告（让新设备能通过局域网发现本机）。
///
/// 广告的服务类型为 `_solosoul_recovery._tcp.local.`，与同步服务区分。
/// TXT 记录**只包含 addr 与 fingerprint**——PIN 与 nonce 属于认证凭证，
/// 仅经 QR 码/手动输入带外传递。此前把 pin+nonce 明文广播到局域网，
/// 任何浏览 `_solosoul_recovery._tcp.local.` 的主机都能直接通过认证并
/// 下载整个 Vault 的恢复包（P001）。
#[cfg(desktop)]
pub fn recovery_advertise(
    daemon: &ServiceDaemon,
    instance_name: &str,
    port: u16,
    fingerprint: &str,
    addr: &str,
) -> Result<(), String> {
    let mut txt = std::collections::HashMap::<String, String>::new();
    txt.insert("fp".to_string(), fingerprint.to_string());
    txt.insert("addr".to_string(), addr.to_string());

    let service = ServiceInfo::new(
        RECOVERY_MDNS_SERVICE_TYPE,
        instance_name,
        &format!("{}.local.", instance_name),
        "",
        port,
        txt,
    )
    .map_err(|e| format!("Recovery ServiceInfo: {}", e))?;

    daemon
        .register(service)
        .map_err(|e| format!("Recovery mDNS register: {}", e))?;
    Ok(())
}

/// 取消恢复主机的 mDNS 广告。
#[cfg(desktop)]
pub fn recovery_stop_advertise(daemon: &ServiceDaemon, instance_name: &str) -> Result<(), String> {
    daemon
        .unregister(&format!("{}.{}", instance_name, RECOVERY_MDNS_SERVICE_TYPE))
        .map_err(|e| format!("Recovery mDNS unregister: {}", e))?;
    Ok(())
}

/// 发现局域网中的恢复主机。
///
/// 返回按名称排序的 `RecoveryDiscoveredHost` 列表。TXT 记录只携带
/// addr 与 fingerprint（P001：不再广播 PIN/nonce），因此接收端需要
/// 用户手动输入主机屏幕/QR 上的 6 位 PIN 后才能发起 `recovery_restore_from_host`。
#[cfg(desktop)]
#[tauri::command]
pub async fn recovery_discover_hosts(
    daemon: tauri::State<'_, SharedDaemon>,
    timeout_ms: u64,
) -> Result<Vec<RecoveryDiscoveredHost>, String> {
    let timeout_ms = timeout_ms.min(MDNS_MAX_TIMEOUT_MS);
    let daemon_arc = daemon.get().await?;
    let guard = daemon_arc.lock().await;
    let daemon = guard.as_ref().ok_or("mDNS daemon not initialized")?;

    let receiver = daemon
        .browse(RECOVERY_MDNS_SERVICE_TYPE)
        .map_err(|e| format!("Browse recovery: {}", e))?;

    let mut hosts: Vec<RecoveryDiscoveredHost> = Vec::new();
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);

    while std::time::Instant::now() < deadline {
        if let Ok(ServiceEvent::ServiceResolved(info)) =
            receiver.recv_timeout(std::time::Duration::from_millis(MDNS_POLL_INTERVAL_MS))
        {
            let props = info.get_properties();
            let fingerprint = props.get("fp").map(|v| v.to_string()).unwrap_or_default();
            let addr = props.get("addr").map(|v| v.to_string()).unwrap_or_default();

            let addresses: Vec<String> =
                info.get_addresses().iter().map(|a| a.to_string()).collect();
            let connect_addr = if !addr.is_empty() {
                addr.clone()
            } else if let Some(first_addr) = addresses.first() {
                format!("{}:{}", first_addr, info.get_port())
            } else {
                continue;
            };

            // 从全名中提取简短易读的显示名称
            let display_name = if !fingerprint.is_empty() {
                format!("SoloSoul-{}", &fingerprint[..fingerprint.len().min(8)])
            } else {
                info.get_hostname().to_string()
            };

            hosts.push(RecoveryDiscoveredHost {
                name: display_name,
                addr: connect_addr,
                fingerprint,
            });
        }
    }

    // 按名称去重并排序（mDNS 可能多次广播同一主机）
    hosts.sort_by(|a, b| a.name.cmp(&b.name));
    hosts.dedup_by(|a, b| a.name == b.name && a.addr == b.addr);

    Ok(hosts)
}

/// 移动端注册恢复 NSD 服务——暂不支持（恢复发现仅限于桌面端作为主机，
/// 移动端可通过 QR 码或手动输入连接）。
#[cfg(mobile)]
#[tauri::command]
pub async fn recovery_discover_hosts(
    _daemon: tauri::State<'_, SharedDaemon>,
    _timeout_ms: u64,
) -> Result<Vec<RecoveryDiscoveredHost>, String> {
    Ok(Vec::new()) // 移动端暂不支持
}

/// 把 mDNS 解析出的地址列表格式化为 `ip:port`（IPv6 自动带方括号）。
/// 桌面端与移动端（discovery.rs 移动版）保持同一形状，供前端直接使用。
#[cfg(desktop)]
fn format_discovered_addresses(
    addrs: &std::collections::HashSet<std::net::IpAddr>,
    port: u16,
) -> Vec<String> {
    addrs
        .iter()
        .map(|a| std::net::SocketAddr::new(*a, port).to_string())
        .collect()
}

/// 移动端同步注册 NSD 服务的 blocking 版（供 sync_enable 的 spawn_blocking 任务调用，
/// 避免在 async 上下文里执行 run_mobile_plugin 同步 IPC 占用 Tokio worker）。
/// 与 async 版 `register_sync_service` 等价，但由调用方负责在获取参数后再调用。
#[cfg(mobile)]
pub fn register_sync_service_blocking(
    app: &tauri::AppHandle,
    device_name: String,
    port: u16,
    account_id: String,
    fingerprint: String,
    client_type: String,
) -> Result<(), String> {
    if account_id.is_empty() {
        return Err("Cannot advertise sync service: no unlocked account".to_string());
    }
    let handle = app.state::<crate::nsd_plugin::NsdPluginHandle<tauri::Wry>>();
    handle.register_service(crate::nsd_plugin::RegisterServicePayload {
        port,
        node_id: device_name,
        account_id,
        fingerprint,
        client_type,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovered_device_serialization() {
        let device = DiscoveredDevice {
            name: "Alice-MacBook".to_string(),
            host: "Alice-MacBook.local.".to_string(),
            port: 42069,
            addresses: vec![
                "192.168.1.5:42069".to_string(),
                "[fe80::1]:42069".to_string(),
            ],
            fingerprint: "a1b2c3d4e5f6".to_string(),
            client_type: "macos".to_string(),
        };
        let json = serde_json::to_string(&device).unwrap();
        assert!(json.contains("\"Alice-MacBook\""));
        assert!(json.contains("\"192.168.1.5:42069\""));
        // Verify camelCase field naming
        assert!(json.contains("\"name\":\"Alice-MacBook\""));
        assert!(json.contains("\"addresses\""));
        // 新增字段序列化
        assert!(json.contains("\"fingerprint\":\"a1b2c3d4e5f6\""));
        assert!(json.contains("\"clientType\":\"macos\""));
    }

    /// 显示名：有指纹 → SoloSoul-<fp8>；无指纹 → 主机名剥 .local. 后缀。
    #[test]
    #[cfg(desktop)]
    fn test_discovered_display_name_uses_fingerprint() {
        assert_eq!(
            discovered_display_name("a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6", "ignored"),
            "SoloSoul-a1b2c3d4"
        );
        // 无指纹：主机名剥掉 .local. 后缀
        assert_eq!(
            discovered_display_name("", "node_abc123.local."),
            "node_abc123"
        );
        assert_eq!(discovered_display_name("", "macbook.local."), "macbook");
    }

    #[test]
    #[cfg(desktop)]
    fn test_format_discovered_addresses_includes_port() {
        let mut addrs = std::collections::HashSet::new();
        addrs.insert(std::net::IpAddr::V4(std::net::Ipv4Addr::new(
            192, 168, 1, 5,
        )));
        addrs.insert(std::net::IpAddr::V6(std::net::Ipv6Addr::new(
            0xfe80, 0, 0, 0, 0, 0, 0, 1,
        )));
        let mut formatted = format_discovered_addresses(&addrs, 42069);
        formatted.sort();
        assert_eq!(
            formatted,
            vec![
                "192.168.1.5:42069".to_string(),
                "[fe80::1]:42069".to_string()
            ]
        );
    }

    #[test]
    #[cfg(desktop)]
    fn test_should_show_device_same_account_not_self() {
        let local_hash = sha256_hex_short("acc-1");
        // 同账户 + 非本机 → 展示
        assert!(should_show_device(
            &local_hash,
            "",
            "node-other",
            Some(&local_hash),
            Some("node-self")
        ));
    }

    #[test]
    #[cfg(desktop)]
    fn test_should_show_device_filters_other_account() {
        let local_hash = sha256_hex_short("acc-1");
        let other_hash = sha256_hex_short("acc-2");
        // 其他账户 → 隐藏
        assert!(!should_show_device(
            &other_hash,
            "",
            "node-other",
            Some(&local_hash),
            None
        ));
    }

    #[test]
    #[cfg(desktop)]
    fn test_should_show_device_filters_self() {
        let local_hash = sha256_hex_short("acc-1");
        // 本机（node_id 相同）→ 隐藏
        assert!(!should_show_device(
            &local_hash,
            "",
            "node-self",
            Some(&local_hash),
            Some("node-self")
        ));
    }

    #[test]
    #[cfg(desktop)]
    fn test_should_show_device_legacy_account_id_fallback() {
        let local_hash = sha256_hex_short("acc-1");
        // 旧版客户端广播明文 account_id（无 account_hash）→ 取其哈希比对
        assert!(should_show_device(
            "",
            "acc-1",
            "node-other",
            Some(&local_hash),
            None
        ));
        assert!(!should_show_device(
            "",
            "acc-2",
            "node-other",
            Some(&local_hash),
            None
        ));
    }

    /// Android NsdManager TXT 属性经常不传播到标准 mDNS 客户端：服务无任何账户信息
    /// （无 account_hash 也无 account_id）时放行展示（会话层仍校验 account_id），
    /// 否则 mac 端永远找不到安卓端。
    #[test]
    #[cfg(desktop)]
    fn test_should_show_device_shows_account_less_android_peer() {
        let local_hash = sha256_hex_short("acc-1");
        // 无账户信息的服务（Android TXT 未传播）→ 放行（同账户概率高，跨账户由会话层拒绝）
        assert!(should_show_device(
            "",
            "",
            "SoloSoul-abcdef12",
            Some(&local_hash),
            Some("node-self")
        ));
        // 有账户信息时仍严格过滤（回归：跨账户必须隐藏）
        assert!(!should_show_device(
            "",
            "acc-2",
            "node-other",
            Some(&local_hash),
            None
        ));
        // 本机过滤不因放宽账户而失效
        assert!(!should_show_device(
            "",
            "",
            "node-self",
            Some(&local_hash),
            Some("node-self")
        ));
    }

    #[test]
    #[cfg(desktop)]
    fn test_should_show_device_unknown_local_identity_passes() {
        // 本地账户未知（Vault 未解锁）→ 不过滤账户；本机 node_id 已知时仍排除本机
        assert!(should_show_device(
            "",
            "",
            "node-other",
            None,
            Some("node-self")
        ));
        assert!(!should_show_device(
            "",
            "",
            "node-self",
            None,
            Some("node-self")
        ));
        // 本地身份完全未知 → 全部展示（保持旧行为）
        assert!(should_show_device("", "", "any-node", None, None));
    }

    #[test]
    fn test_discovered_device_empty_addresses() {
        let device = DiscoveredDevice {
            name: "Headless".to_string(),
            host: String::new(),
            port: 0,
            addresses: vec![],
            fingerprint: String::new(),
            client_type: "unknown".to_string(),
        };
        let json = serde_json::to_string(&device).unwrap();
        // Should still serialize correctly with empty arrays
        assert!(json.contains("\"addresses\":[]"));
        assert!(json.contains("\"port\":0"));
        // client_type 序列化为 unknown
        assert!(json.contains("\"clientType\":\"unknown\""));
    }

    #[test]
    fn test_mdns_constants_defined() {
        assert_eq!(MDNS_SERVICE_TYPE, "_solosoul._tcp.local.");
        assert_eq!(MDNS_MAX_TIMEOUT_MS, 30_000);
        assert_eq!(MDNS_POLL_INTERVAL_MS, 200);
    }

    #[test]
    fn test_mdns_service_type_matches_android_base() {
        // Android 端 NsdPlugin 使用的基础服务类型（不含 .local. 后缀）。
        const ANDROID_SERVICE_TYPE: &str = "_solosoul._tcp";
        let base = MDNS_SERVICE_TYPE.trim_end_matches(".local.");
        assert_eq!(base, ANDROID_SERVICE_TYPE);
        assert_eq!(MDNS_SERVICE_TYPE_BASE, ANDROID_SERVICE_TYPE);
    }

    #[test]
    fn test_shared_daemon_default_constructs() {
        let daemon = SharedDaemon::default();
        drop(daemon); // verify no panic on drop
        let daemon2 = SharedDaemon::new();
        drop(daemon2);
    }
}
