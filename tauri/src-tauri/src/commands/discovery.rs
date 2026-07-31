//! 本地设备发现（mDNS）。
//!
//! 桌面端使用 `mdns-sd` 实现服务发现与广播；移动端（Android/iOS）暂不提供该功能，
//! 二期可替换为 Android NSD / iOS Bonjour。

#[cfg(desktop)]
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::Serialize;
use std::sync::Arc;
#[cfg(mobile)]
use tauri::Manager;
use tokio::sync::Mutex;

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
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryDiscoveredHost {
    /// 主机显示名称（由主机指纹截取生成）。
    pub name: String,
    /// 连接地址（host:port）。
    pub addr: String,
    /// 6 位数字 PIN。
    pub pin: String,
    /// 主机公钥指纹（用于 MITM 验证）。
    pub fingerprint: String,
    /// 一次性 nonce（用于认证）。
    pub nonce: String,
}

#[derive(Serialize)]
pub struct DiscoveredDevice {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub addresses: Vec<String>,
}

#[cfg(desktop)]
#[tauri::command]
pub async fn mdns_discover(
    daemon: tauri::State<'_, SharedDaemon>,
    timeout_ms: u64,
) -> Result<Vec<DiscoveredDevice>, String> {
    let timeout_ms = timeout_ms.min(MDNS_MAX_TIMEOUT_MS);
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
            let addresses: Vec<String> =
                info.get_addresses().iter().map(|a| a.to_string()).collect();
            devices.push(DiscoveredDevice {
                name: info.get_fullname().to_string(),
                host: info.get_hostname().to_string(),
                port: info.get_port(),
                addresses,
            });
        }
    }

    // mDNS 可能对同一服务多次触发 ServiceResolved，按 host:port 去重，
    // 避免前端设备列表出现重复条目。
    let mut seen = std::collections::HashSet::new();
    devices.retain(|d| seen.insert(format!("{}:{}", d.host, d.port)));
    Ok(devices)
}

#[cfg(mobile)]
#[tauri::command]
pub async fn mdns_discover(
    app: tauri::AppHandle,
    _daemon: tauri::State<'_, SharedDaemon>,
    timeout_ms: u64,
) -> Result<Vec<DiscoveredDevice>, String> {
    let timeout_ms = timeout_ms.min(MDNS_MAX_TIMEOUT_MS);

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
            .map(|s| DiscoveredDevice {
                name: s.node_id.clone(),
                host: s.host.clone(),
                port: s.port,
                addresses: vec![format!("{}:{}", s.host, s.port)],
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

#[cfg(desktop)]
#[tauri::command]
pub async fn mdns_advertise(
    daemon: tauri::State<'_, SharedDaemon>,
    device_name: String,
    port: u16,
) -> Result<(), String> {
    let daemon_arc = daemon.get().await?;
    let guard = daemon_arc.lock().await;
    let daemon = guard.as_ref().ok_or("mDNS daemon not initialized")?;

    let service = ServiceInfo::new(
        MDNS_SERVICE_TYPE,
        &device_name,
        &format!("{}.local.", device_name),
        "",
        port,
        std::collections::HashMap::<String, String>::new(),
    )
    .map_err(|e| format!("ServiceInfo: {}", e))?;

    daemon
        .register(service)
        .map_err(|e| format!("Register: {}", e))?;
    Ok(())
}

// ── 恢复 mDNS 广告与发现 ────────────────────────────────────────────────────

/// 注册恢复主机的 mDNS 广告（让新设备能通过局域网发现本机）。
///
/// 广告的服务类型为 `_solosoul_recovery._tcp.local.`，与同步服务区分。
/// TXT 记录包含 addr、PIN、fingerprint 和 nonce，供接收端直接连接。
#[cfg(desktop)]
pub fn recovery_advertise(
    daemon: &ServiceDaemon,
    instance_name: &str,
    port: u16,
    pin: &str,
    fingerprint: &str,
    nonce: &str,
    addr: &str,
) -> Result<(), String> {
    let mut txt = std::collections::HashMap::<String, String>::new();
    txt.insert("pin".to_string(), pin.to_string());
    txt.insert("fp".to_string(), fingerprint.to_string());
    txt.insert("nonce".to_string(), nonce.to_string());
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
/// 返回按名称排序的 `RecoveryDiscoveredHost` 列表，每个元素包含从 TXT 记录解析的
/// addr、PIN、fingerprint 和 nonce，接收端可直接用于 `recovery_restore_from_host`。
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
            let pin = props.get("pin").map(|v| v.to_string()).unwrap_or_default();
            let fingerprint = props.get("fp").map(|v| v.to_string()).unwrap_or_default();
            let nonce = props
                .get("nonce")
                .map(|v| v.to_string())
                .unwrap_or_default();
            let addr = props.get("addr").map(|v| v.to_string()).unwrap_or_default();

            if pin.is_empty() || addr.is_empty() {
                continue; // 不完整的信息，跳过
            }

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
                pin,
                fingerprint,
                nonce,
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
    })
}

/// 移动端注册 NSD 服务（供 sync_enable 内部调用，避免命令间重复逻辑）。
#[cfg(mobile)]
pub async fn register_sync_service(
    app: &tauri::AppHandle,
    device_name: String,
    port: u16,
) -> Result<(), String> {
    use crate::commands::current_account_optional;
    use tauri::Manager;

    let (account_id, fingerprint) = {
        let state = app.state::<crate::state::AppState>();
        let account_id = current_account_optional(&state).unwrap_or_default();
        let fp = state
            .sync_service
            .local_fingerprint()
            .await
            .unwrap_or_default();
        (account_id, fp)
    };

    if account_id.is_empty() {
        return Err("Cannot advertise sync service: no unlocked account".to_string());
    }

    let handle = app.state::<crate::nsd_plugin::NsdPluginHandle<tauri::Wry>>();
    handle.register_service(crate::nsd_plugin::RegisterServicePayload {
        port,
        node_id: device_name,
        account_id,
        fingerprint,
    })?;
    Ok(())
}

#[cfg(mobile)]
#[tauri::command]
pub async fn mdns_advertise(
    app: tauri::AppHandle,
    _daemon: tauri::State<'_, SharedDaemon>,
    device_name: String,
    port: u16,
) -> Result<(), String> {
    register_sync_service(&app, device_name, port).await
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
            addresses: vec!["192.168.1.5".to_string(), "fe80::1".to_string()],
        };
        let json = serde_json::to_string(&device).unwrap();
        assert!(json.contains("\"Alice-MacBook\""));
        assert!(json.contains("\"192.168.1.5\""));
        // Verify camelCase field naming
        assert!(json.contains("\"name\":\"Alice-MacBook\""));
        assert!(json.contains("\"addresses\""));
    }

    #[test]
    fn test_discovered_device_empty_addresses() {
        let device = DiscoveredDevice {
            name: "Headless".to_string(),
            host: String::new(),
            port: 0,
            addresses: vec![],
        };
        let json = serde_json::to_string(&device).unwrap();
        // Should still serialize correctly with empty arrays
        assert!(json.contains("\"addresses\":[]"));
        assert!(json.contains("\"port\":0"));
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
