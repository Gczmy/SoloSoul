//! Sync protocol messages exchanged between peers.

use crate::hlc::Hlc;
use serde::{Deserialize, Serialize};

/// A single record to be synchronized.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRecord {
    pub id: String,
    pub table: String,
    pub data: serde_json::Value,
    pub hlc: Hlc,
    #[serde(default)]
    pub deleted: bool,
}

/// Top-level messages exchanged over a sync session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SyncMessage {
    #[serde(rename = "hello")]
    Hello {
        node_id: String,
        account_id: String,
        public_key_fingerprint: String,
    },
    #[serde(rename = "hello_ack")]
    HelloAck {
        node_id: String,
        account_id: String,
        public_key_fingerprint: String,
        trusted: bool,
    },
    #[serde(rename = "batch")]
    Batch {
        table: String,
        records: Vec<SyncRecord>,
        finished: bool,
    },
    #[serde(rename = "ack")]
    Ack { table: String, count: u64 },
    #[serde(rename = "done")]
    Done,
    #[serde(rename = "error")]
    Error { message: String },
}

impl SyncMessage {
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(self).map_err(|e| format!("encode: {}", e))
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        serde_json::from_slice(bytes).map_err(|e| format!("decode: {}", e))
    }
}
