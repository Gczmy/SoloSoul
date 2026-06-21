use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;

const MDNS_SERVICE_TYPE: &str = "_solosoul._tcp.local.";
const MDNS_MAX_TIMEOUT_MS: u64 = 30_000;
const MDNS_POLL_INTERVAL_MS: u64 = 200;

/// Wrapper to share ServiceDaemon across commands (prevents mem::forget leaks)
pub struct SharedDaemon(Arc<Mutex<Option<ServiceDaemon>>>);

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
    pub async fn get(&self) -> Result<Arc<Mutex<Option<ServiceDaemon>>>, String> {
        let mut guard = self.0.lock().await;
        if guard.is_none() {
            *guard = Some(ServiceDaemon::new().map_err(|e| format!("mDNS daemon: {}", e))?);
        }
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
    fn test_shared_daemon_default_constructs() {
        let daemon = SharedDaemon::default();
        drop(daemon); // verify no panic on drop
        let daemon2 = SharedDaemon::new();
        drop(daemon2);
    }
}
