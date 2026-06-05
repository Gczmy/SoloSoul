use serde::Serialize;
use mdns_sd::{ServiceDaemon, ServiceInfo, ServiceEvent};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Wrapper to share ServiceDaemon across commands (prevents mem::forget leaks)
pub struct SharedDaemon(Arc<Mutex<Option<ServiceDaemon>>>);

impl Default for SharedDaemon {
    fn default() -> Self { Self::new() }
}

impl SharedDaemon {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(None)))
    }

    /// Get or create the daemon. The daemon lives for the app lifetime.
    pub async fn get(&self) -> Result<Arc<Mutex<Option<ServiceDaemon>>>, String> {
        let mut guard = self.0.lock().await;
        if guard.is_none() {
            *guard = Some(
                ServiceDaemon::new().map_err(|e| format!("mDNS daemon: {}", e))?
            );
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
    let daemon_arc = daemon.get().await?;
    let guard = daemon_arc.lock().await;
    let daemon = guard.as_ref().ok_or("mDNS daemon failed to initialize")?;

    let receiver = daemon.browse("_solosoul._tcp.local.")
        .map_err(|e| format!("Browse: {}", e))?;

    let mut devices = Vec::new();
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);

    while std::time::Instant::now() < deadline {
        if let Ok(ServiceEvent::ServiceResolved(info)) =
            receiver.recv_timeout(std::time::Duration::from_millis(200))
        {
            let addresses: Vec<String> = info.get_addresses().iter()
                .map(|a| a.to_string()).collect();
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
    let daemon = guard.as_ref().ok_or("mDNS daemon failed to initialize")?;

    let service = ServiceInfo::new(
        "_solosoul._tcp.local.", &device_name,
        &format!("{}.local.", device_name), "", port,
        std::collections::HashMap::<String, String>::new(),
    ).map_err(|e| format!("ServiceInfo: {}", e))?;

    daemon.register(service).map_err(|e| format!("Register: {}", e))?;
    Ok(())
}
