//! mDNS service discovery for peer-to-peer sync.
//!
//! Discovers other SoloSoul devices on the local network using
//! multicast DNS (mDNS) with the `_solosoul._tcp` service type.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

const SERVICE_TYPE: &str = "_solosoul._tcp";
const DISCOVERY_PORT: u16 = 42069;

/// A discovered peer device
#[derive(Debug, Clone)]
pub struct DiscoveredPeer {
    pub id: String,
    pub name: String,
    pub addr: SocketAddr,
    pub last_seen: Instant,
}

/// Simple service discovery tracker
#[derive(Debug, Default)]
pub struct DiscoveryManager {
    peers: HashMap<String, DiscoveredPeer>,
}

impl DiscoveryManager {
    pub fn new() -> Self {
        Self {
            peers: HashMap::new(),
        }
    }

    /// Register or update a discovered peer
    pub fn add_peer(&mut self, id: String, name: String, addr: SocketAddr) {
        self.peers.insert(
            id.clone(),
            DiscoveredPeer {
                id,
                name,
                addr,
                last_seen: Instant::now(),
            },
        );
    }

    /// Get all currently visible peers (not expired)
    pub fn visible_peers(&self, max_age: Duration) -> Vec<&DiscoveredPeer> {
        let now = Instant::now();
        self.peers
            .values()
            .filter(|p| now.duration_since(p.last_seen) < max_age)
            .collect()
    }

    /// Remove stale peers
    pub fn prune(&mut self, max_age: Duration) {
        let now = Instant::now();
        self.peers
            .retain(|_, p| now.duration_since(p.last_seen) < max_age);
    }

    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    pub fn service_type(&self) -> &str {
        SERVICE_TYPE
    }

    pub fn discovery_port(&self) -> u16 {
        DISCOVERY_PORT
    }
}
