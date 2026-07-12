//! 同步引擎公共类型定义。
//!
//! 这些类型被 service.rs 与 manager.rs 共用，也用于移动端占位实现，
//! 因此放在独立模块中，避免条件编译导致类型不可用。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Information about a discovered or known peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPeerInfo {
    pub node_id: String,
    pub account_id: String,
    pub name: String,
    pub addr: String,
    pub fingerprint: String,
    pub trusted: bool,
    pub last_seen: String,
}

/// Attachment synchronization statistics.
#[derive(Debug, Clone, Default)]
pub struct AttachmentSyncStats {
    pub sent: u64,
    pub received: u64,
    pub bytes_transferred: u64,
    pub errors: Vec<String>,
}

/// Per-table sync statistics.
#[derive(Debug, Clone, Default)]
pub struct TableStats {
    pub examined: u64,
    pub applied: u64,
    pub skipped: u64,
}

/// Result of applying a batch of sync records.
#[derive(Debug, Clone, Default)]
pub struct ApplyStats {
    pub examined: u64,
    pub applied: u64,
    pub skipped: u64,
    pub errors: Vec<String>,
    pub conflicts: Vec<ConflictRecord>,
    pub per_table: HashMap<String, TableStats>,
}

/// A single sync conflict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictRecord {
    pub table: String,
    pub id: String,
    pub local_hlc: crate::hlc::Hlc,
    pub remote_hlc: crate::hlc::Hlc,
    pub winner: String,
}

/// Result of a full sync session, including database deltas and attachment transfers.
#[derive(Debug, Clone, Default)]
pub struct SyncSessionResult {
    pub data: ApplyStats,
    pub attachments: AttachmentSyncStats,
}
