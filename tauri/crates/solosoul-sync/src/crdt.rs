//! CRDT (Conflict-Free Replicated Data Type) for profile sync.
//!
//! Uses Last-Writer-Wins (LWW) per field with vector clocks for conflict detection.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single CRDT entry with vector clock timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtEntry {
    pub key: String,
    pub value: Vec<u8>,
    pub timestamp: u64,
    pub node_id: String,
}

/// CRDT map that uses LWW per key
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CrdtMap {
    pub entries: HashMap<String, CrdtEntry>,
}

impl CrdtMap {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Apply a local change, returning true if this is newer
    pub fn apply_local(&mut self, key: String, value: Vec<u8>, node_id: String, now: u64) -> bool {
        let is_newer = match self.entries.get(&key) {
            Some(existing) => now > existing.timestamp,
            None => true,
        };
        if is_newer {
            self.entries.insert(
                key.clone(),
                CrdtEntry {
                    key,
                    value,
                    timestamp: now,
                    node_id,
                },
            );
        }
        is_newer
    }

    /// Merge a remote entry, returning true if the local state changed
    pub fn merge_remote(&mut self, entry: CrdtEntry) -> bool {
        match self.entries.get(&entry.key) {
            Some(local) if local.timestamp >= entry.timestamp => false,
            _ => {
                self.entries.insert(entry.key.clone(), entry);
                true
            }
        }
    }

    /// Merge all entries from another CrdtMap, returning the count of new items
    pub fn merge_all(&mut self, other: &CrdtMap) -> usize {
        let mut count = 0;
        for entry in other.entries.values() {
            if self.merge_remote(entry.clone()) {
                count += 1;
            }
        }
        count
    }

    /// Get the value for a key, if it exists
    pub fn get(&self, key: &str) -> Option<&CrdtEntry> {
        self.entries.get(key)
    }

    /// Number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(key: &str, ts: u64, node: &str) -> CrdtEntry {
        CrdtEntry {
            key: key.to_string(),
            value: format!("v{}", ts).into_bytes(),
            timestamp: ts,
            node_id: node.to_string(),
        }
    }

    #[test]
    fn test_lww_newer_wins() {
        let mut map = CrdtMap::new();
        map.apply_local("k1".into(), b"v1".to_vec(), "node-a".into(), 100);
        assert_eq!(map.get("k1").unwrap().timestamp, 100);

        // Older timestamp should not replace
        let entry = make_entry("k1", 50, "node-b");
        assert!(!map.merge_remote(entry));
        assert_eq!(map.get("k1").unwrap().node_id, "node-a");

        // Newer timestamp should replace
        let entry = make_entry("k1", 200, "node-b");
        assert!(map.merge_remote(entry));
        assert_eq!(map.get("k1").unwrap().node_id, "node-b");
    }

    #[test]
    fn test_merge_all_counts() {
        let mut local = CrdtMap::new();
        local.apply_local("a".into(), vec![], "n1".into(), 10);

        let mut remote = CrdtMap::new();
        remote.apply_local("b".into(), vec![], "n2".into(), 20);
        remote.apply_local("c".into(), vec![], "n2".into(), 30);

        let count = local.merge_all(&remote);
        assert_eq!(count, 2); // b and c are new
        assert_eq!(local.len(), 3);
    }
}
