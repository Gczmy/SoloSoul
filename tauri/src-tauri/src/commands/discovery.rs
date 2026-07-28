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
    Ok(devices)
}

#[cfg(mobile)]
#[tauri::command]
pub async fn mdns_discover(
    app: tauri::AppHandle,
    daemon: tauri::State<'_, SharedDaemon>,
    timeout_ms: u64,
) -> Result<Vec<DiscoveredDevice>, String> {
    let _ = daemon;
    let timeout_ms = timeout_ms.min(MDNS_MAX_TIMEOUT_MS);
    let handle = app.state::<crate::nsd_plugin::NsdPluginHandle<tauri::Wry>>();
    handle.request_permissions()?;
    handle.start_discovery()?;

    // 轮询等待 NSD 发现结果，避免立即读取返回空列表
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
    let mut last_count = 0usize;
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(MDNS_POLL_INTERVAL_MS)).await;
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
