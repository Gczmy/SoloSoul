//! Sync protocol - WebSocket and API definitions

use serde::{Deserialize, Serialize};

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    /// Session revoked - forced logout
    SessionRevoked { reason: String },
    /// Data changed on server
    DataChanged { new_sequence: u64 },
    /// Keepalive ping/pong
    Keepalive { ts: i64 },
}

/// API response envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(error: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.to_string()),
        }
    }
}

/// Sync status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub sequence: u64,
    pub last_modified: String,
    pub size_bytes: u64,
}

/// Conflict resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    /// Keep local changes
    KeepLocal,
    /// Use server version
    UseServer,
    /// Manual merge
    Manual,
}

/// Sync metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMetadata {
    pub sequence: u64,
    pub device_id: String,
    pub timestamp: i64,
}
