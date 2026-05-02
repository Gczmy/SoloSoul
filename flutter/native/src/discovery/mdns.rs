//! mDNS device discovery for local network sync.
//!
//! Uses mDNS/DNS-SD to advertise and discover SoloSoul devices on the
//! same local network. Devices advertise a `_solosoul._tcp.local.` service.

use mdns_sd::{ServiceDaemon, ServiceEvent};

/// Service type for SoloSoul sync
const SERVICE_TYPE: &str = "_solosoul._tcp.local.";

/// Discovered device info
#[derive(Debug, Clone)]
pub struct DiscoveredDevice {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub addresses: Vec<std::net::SocketAddr>,
}

/// mDNS-based device discovery.
///
/// Wraps `mdns_sd::ServiceDaemon` to advertise the local device and
/// browse for other SoloSoul devices on the network.
pub struct MdnsDiscovery {
    daemon: ServiceDaemon,
    registered: bool,
}

impl MdnsDiscovery {
    /// Create a new mDNS discovery instance.
    pub fn new() -> Result<Self, String> {
        let daemon =
            ServiceDaemon::new().map_err(|e| format!("mDNS daemon creation failed: {}", e))?;
        Ok(Self {
            daemon,
            registered: false,
        })
    }

    /// Advertise this device on the network.
    ///
    /// `device_name` should be a unique identifier (e.g., account ID or device name).
    /// `port` is the TCP port the sync server is listening on.
    pub fn advertise(&mut self, device_name: &str, port: u16) -> Result<(), String> {
        let service = mdns_sd::ServiceInfo::new(
            SERVICE_TYPE,
            device_name,
            &format!("{}.local.", device_name),
            "",
            port,
            None,
        )
        .map_err(|e| format!("ServiceInfo creation failed: {}", e))?;

        self.daemon
            .register(service)
            .map_err(|e| format!("mDNS register failed: {}", e))?;
        self.registered = true;
        Ok(())
    }

    /// Browse for other SoloSoul devices on the network.
    ///
    /// Returns a list of discovered devices. This is a blocking call that
    /// waits for a short period to collect responses.
    pub fn browse(&self, timeout_ms: u64) -> Result<Vec<DiscoveredDevice>, String> {
        let receiver = self
            .daemon
            .browse(SERVICE_TYPE)
            .map_err(|e| format!("mDNS browse failed: {}", e))?;

        let mut devices = Vec::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);

        while std::time::Instant::now() < deadline {
            match receiver.recv_deadline(deadline) {
                Ok(ServiceEvent::ServiceResolved(info)) => {
                    let addresses: Vec<std::net::SocketAddr> = info
                        .get_addresses()
                        .iter()
                        .map(|ip| std::net::SocketAddr::new(*ip, info.get_port()))
                        .collect();

                    if !addresses.is_empty() {
                        devices.push(DiscoveredDevice {
                            name: info.get_hostname().to_string(),
                            host: info.get_fullname().to_string(),
                            port: info.get_port(),
                            addresses,
                        });
                    }
                }
                Ok(_) => {}      // Other events (Discovered, Removed, etc.)
                Err(_) => break, // Timeout or disconnected
            }
        }

        // Stop browsing
        self.daemon
            .stop_browse(SERVICE_TYPE)
            .map_err(|e| format!("stop_browse failed: {}", e))?;

        Ok(devices)
    }

    /// Stop advertising and clean up.
    pub fn shutdown(&mut self) -> Result<(), String> {
        if self.registered {
            // mdns_sd ServiceDaemon cleans up on drop
            self.registered = false;
        }
        Ok(())
    }
}

impl Drop for MdnsDiscovery {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
