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

/// Metadata for an attachment file to be synced.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentInfo {
    pub id: String,
    pub object_id: String,
    pub file_name: String,
    pub size_bytes: u64,
    pub sha256: String,
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
        /// 协议版本号。旧版客户端不发送此字段，反序列化时默认为 1。
        /// 发起方在 Hello 中声明自己的版本，响应方在 HelloAck 中回传其版本，
        /// 双方取 min(发起方版本, 响应方版本) 作为本次会话使用的协议版本。
        #[serde(default = "default_protocol_version")]
        protocol_version: u32,
    },
    #[serde(rename = "hello_ack")]
    HelloAck {
        node_id: String,
        account_id: String,
        public_key_fingerprint: String,
        trusted: bool,
        /// 响应方的协议版本号。旧版客户端不发送此字段，默认为 1。
        #[serde(default = "default_protocol_version")]
        protocol_version: u32,
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
    #[serde(rename = "attachment_manifest")]
    AttachmentManifest {
        object_id: String,
        attachments: Vec<AttachmentInfo>,
    },
    #[serde(rename = "attachment_manifest_done")]
    AttachmentManifestDone,
    #[serde(rename = "attachment_request")]
    AttachmentRequest {
        object_id: String,
        attachment_ids: Vec<String>,
    },
    #[serde(rename = "attachment_request_done")]
    AttachmentRequestDone,
    #[serde(rename = "attachment_chunk")]
    AttachmentChunk {
        object_id: String,
        attachment_id: String,
        chunk_index: u32,
        total_chunks: u32,
        data: Vec<u8>,
    },
    #[serde(rename = "attachment_ack")]
    AttachmentAck {
        object_id: String,
        attachment_id: String,
        chunk_index: u32,
    },
    #[serde(rename = "attachment_done")]
    AttachmentDone,
    #[serde(rename = "error")]
    Error { message: String },
}

/// 旧版客户端不发送 `protocol_version` 字段时的默认值。
/// 版本 1 是引入版本协商之前的初始协议。
fn default_protocol_version() -> u32 {
    1
}

impl SyncMessage {
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(self).map_err(|e| format!("encode: {}", e))
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        serde_json::from_slice(bytes).map_err(|e| format!("decode: {}", e))
    }
}
