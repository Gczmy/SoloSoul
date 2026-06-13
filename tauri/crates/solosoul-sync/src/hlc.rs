//! Hybrid Logical Clock (HLC) for sync ordering.
//!
//! Combines physical wall-clock time (ms) with a logical counter and node id
//! to produce causality-tracking timestamps that are resilient to clock skew.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;
use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

static GLOBAL_COUNTER: AtomicU32 = AtomicU32::new(0);
static LAST_TIME_MS: Mutex<u64> = Mutex::new(0);

/// A Hybrid Logical Clock timestamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hlc {
    pub wall_time_ms: u64,
    pub counter: u32,
    pub node_id: [u8; 16],
}

impl Hlc {
    /// Create a new HLC from raw components.
    pub fn new(wall_time_ms: u64, counter: u32, node_id: &str) -> Self {
        Self {
            wall_time_ms,
            counter,
            node_id: Self::parse_node_id(node_id),
        }
    }

    /// Generate a fresh HLC timestamp for the local node.
    pub fn now(node_id: &str) -> Self {
        let physical = physical_now_ms();
        let mut last = LAST_TIME_MS.lock().unwrap();
        let (wall, counter) = if physical > *last {
            *last = physical;
            (physical, 0)
        } else {
            *last += 1;
            (*last, GLOBAL_COUNTER.fetch_add(1, AtomicOrdering::Relaxed))
        };
        Self {
            wall_time_ms: wall,
            counter,
            node_id: Self::parse_node_id(node_id),
        }
    }

    /// Parse an HLC from a compact string representation.
    /// Format: `<wall_time_ms>-<counter>-<16-byte-hex-node-id>`
    pub fn parse(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.splitn(3, '-').collect();
        if parts.len() != 3 {
            return Err(format!("Invalid HLC string: {}", s));
        }
        let wall = parts[0]
            .parse::<u64>()
            .map_err(|e| format!("Invalid wall time: {}", e))?;
        let counter = parts[1]
            .parse::<u32>()
            .map_err(|e| format!("Invalid counter: {}", e))?;
        let mut node_id = [0u8; 16];
        let bytes = hex::decode(parts[2]).map_err(|e| format!("Invalid node id: {}", e))?;
        if bytes.len() != 16 {
            return Err("node id must be 16 bytes".to_string());
        }
        node_id.copy_from_slice(&bytes);
        Ok(Self {
            wall_time_ms: wall,
            counter,
            node_id,
        })
    }

    fn parse_node_id(node_id: &str) -> [u8; 16] {
        Self::parse_node_id_bytes(node_id)
    }

    pub fn parse_node_id_bytes(node_id: &str) -> [u8; 16] {
        let mut out = [0u8; 16];
        let bytes = if node_id.len() == 32 {
            hex::decode(node_id).unwrap_or_default()
        } else {
            let mut buf = [0u8; 16];
            let src = node_id.as_bytes();
            let len = src.len().min(16);
            buf[..len].copy_from_slice(&src[..len]);
            buf.to_vec()
        };
        let len = bytes.len().min(16);
        out[..len].copy_from_slice(&bytes[..len]);
        out
    }

    pub fn node_id_string(&self) -> String {
        hex::encode(self.node_id)
    }
}

impl PartialOrd for Hlc {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Hlc {
    fn cmp(&self, other: &Self) -> Ordering {
        self.wall_time_ms
            .cmp(&other.wall_time_ms)
            .then_with(|| self.counter.cmp(&other.counter))
            .then_with(|| self.node_id.cmp(&other.node_id))
    }
}

impl fmt::Display for Hlc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}-{}-{}",
            self.wall_time_ms,
            self.counter,
            hex::encode(self.node_id)
        )
    }
}

impl FromStr for Hlc {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

fn physical_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Watermark used to track how far a peer has synced for a given table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncWatermark {
    pub wall_time_ms: u64,
    pub counter: u32,
    pub node_id: [u8; 16],
}

impl SyncWatermark {
    pub fn zero() -> Self {
        Self {
            wall_time_ms: 0,
            counter: 0,
            node_id: [0u8; 16],
        }
    }

    pub fn from_hlc(hlc: &Hlc) -> Self {
        Self {
            wall_time_ms: hlc.wall_time_ms,
            counter: hlc.counter,
            node_id: hlc.node_id,
        }
    }
}

impl PartialOrd for SyncWatermark {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SyncWatermark {
    fn cmp(&self, other: &Self) -> Ordering {
        self.wall_time_ms
            .cmp(&other.wall_time_ms)
            .then_with(|| self.counter.cmp(&other.counter))
            .then_with(|| self.node_id.cmp(&other.node_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hlc_ordering() {
        let a = Hlc::new(100, 0, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let b = Hlc::new(100, 1, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let c = Hlc::new(101, 0, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        assert!(a < b);
        assert!(b < c);
    }

    #[test]
    fn test_hlc_tie_breaker() {
        let a = Hlc::new(100, 0, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let b = Hlc::new(100, 0, "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        assert!(a < b);
    }

    #[test]
    fn test_hlc_roundtrip() {
        let hlc = Hlc::new(12345, 7, "node-a");
        let s = hlc.to_string();
        let parsed = Hlc::parse(&s).unwrap();
        assert_eq!(hlc, parsed);
    }
}
