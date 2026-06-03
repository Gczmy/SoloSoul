//! Sync engine - CRDT-based incremental synchronization
//!
//! Orchestrates profile sync between devices using:
//! - `SoloDoc` for CRDT conflict resolution
//! - `SecureChannel` for Noise-encrypted transport
//! - `Transport` trait for pluggable network backends
//!
//! Attachment sync (v2):
//! After CRDT sync completes, both sides exchange attachment manifests,
//! then serially transfer missing encrypted `.solo` files in 8 MiB chunks.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};

use super::crdt::SoloDoc;
use super::protocol::SecureChannel;

/// Direction of sync result
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncDirection {
    /// Local changes pushed to remote
    Pushed,
    /// Remote changes pulled to local
    Pulled,
    /// Both sides had changes, merged via CRDT
    Merged,
    /// No changes on either side
    NoChange,
}

/// Result of a sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub success: bool,
    pub direction: SyncDirection,
    pub bytes_sent: usize,
    pub bytes_received: usize,
    pub attachments_sent: usize,
    pub attachments_received: usize,
    pub attachment_bytes_sent: usize,
    pub attachment_bytes_received: usize,
    pub attachment_incomplete: bool,
    pub error: Option<String>,
}

/// Sync protocol messages exchanged between devices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncMessage {
    /// Request with sender's state vector
    StateVectorRequest {
        account_id: String,
        state_vector: Vec<u8>,
        #[serde(default)]
        supports_attachments: bool,
    },
    /// Response with receiver's state vector and optional diff
    StateVectorResponse {
        state_vector: Vec<u8>,
        diff: Option<Vec<u8>>,
        #[serde(default)]
        supports_attachments: bool,
    },
    /// Update payload (encrypted diff)
    Update { encrypted_update: Vec<u8> },
    /// Acknowledgment
    Ack { success: bool },
    /// Attachment manifest: list of file_ids and sizes
    AttachmentManifest {
        file_ids: Vec<String>,
        file_sizes: HashMap<String, u64>,
    },
    /// Request a specific attachment file
    AttachmentRequest { file_id: String },
    /// Attachment data chunk
    AttachmentChunk {
        file_id: String,
        chunk_index: u32,
        total_chunks: u32,
        data: Vec<u8>,
        is_last: bool,
    },
    /// Signal end of attachment requests
    AttachmentDone,
}

/// Transport abstraction for sync messages
pub trait Transport: Send {
    fn send(&mut self, data: &[u8]) -> Result<(), String>;
    fn recv(&mut self) -> Result<Vec<u8>, String>;
}

/// Single attachment file info for manifest exchange
#[derive(Debug, Clone)]
pub struct AttachmentManifestItem {
    pub file_id: String,
    pub size: u64,
}

/// Chunk size for attachment transfer: 8 MiB (fits within 16 MiB message limit)
const ATTACHMENT_CHUNK_SIZE: usize = 8 * 1024 * 1024;

/// Sync engine coordinating CRDT doc, encryption, and transport
pub struct SyncEngine {
    pub crdt: SoloDoc,
    pub channel: Option<SecureChannel>,
    pub transport: Box<dyn Transport>,
    /// Local attachment directory (optional — if None, attachment sync is skipped)
    pub attachments_dir: Option<String>,
    /// Local attachment manifest (file_id + encrypted .solo size)
    pub local_attachment_manifest: Vec<AttachmentManifestItem>,
}

impl SyncEngine {
    /// Create a new sync engine
    pub fn new(
        crdt: SoloDoc,
        channel: Option<SecureChannel>,
        transport: Box<dyn Transport>,
    ) -> Self {
        Self {
            crdt,
            channel,
            transport,
            attachments_dir: None,
            local_attachment_manifest: Vec::new(),
        }
    }

    /// Set attachment sync context (call before sync_initiator/sync_responder)
    pub fn with_attachments(
        mut self,
        dir: String,
        manifest: Vec<AttachmentManifestItem>,
    ) -> Self {
        self.attachments_dir = Some(dir);
        self.local_attachment_manifest = manifest;
        self
    }

    /// Execute sync as the initiator (sends state vector first).
    ///
    /// Protocol:
    /// 1. Send our state vector (+ attachment support flag)
    /// 2. Receive remote state vector + their diff
    /// 3. Compute our diff relative to remote SV
    /// 4. Send our diff
    /// 5. Apply remote diff
    /// 6. Sync attachments (if both sides support it)
    pub fn sync_initiator(&mut self) -> Result<SyncResult, String> {
        let mut bytes_sent = 0usize;
        let mut bytes_received = 0usize;
        let local_supports = !self.local_attachment_manifest.is_empty()
            || self.attachments_dir.is_some();

        // 1. Send our state vector
        let local_sv = self.crdt.state_vector();
        let request = SyncMessage::StateVectorRequest {
            account_id: String::new(),
            state_vector: local_sv,
            supports_attachments: local_supports,
        };
        let payload = serde_json::to_vec(&request).map_err(|e| format!("Serialize failed: {}", e))?;
        let payload = self.maybe_encrypt(&payload)?;
        self.transport.send(&payload)?;
        bytes_sent += payload.len();

        // 2. Receive remote response (SV + optional diff)
        let remote_raw = self.transport.recv()?;
        bytes_received += remote_raw.len();
        let remote_decrypted = self.maybe_decrypt(&remote_raw)?;
        let response: SyncMessage = serde_json::from_slice(&remote_decrypted)
            .map_err(|e| format!("Deserialize SV response: {}", e))?;

        let (remote_sv, remote_diff, remote_supports) = match response {
            SyncMessage::StateVectorResponse {
                state_vector,
                diff,
                supports_attachments,
            } => (state_vector, diff, supports_attachments),
            _ => return Err("Expected StateVectorResponse".to_string()),
        };

        // 3. Compute our diff relative to remote state vector
        let local_sv = self.crdt.state_vector();
        let our_diff = self.crdt.encode_diff(&remote_sv)?;
        let has_local_changes = local_sv != remote_sv;

        let direction = Self::classify_direction(&local_sv, &remote_sv, &remote_diff);

        // 4. Send our diff (only if we have changes)
        if has_local_changes {
            let update_msg = SyncMessage::Update {
                encrypted_update: our_diff,
            };
            let update_payload = serde_json::to_vec(&update_msg).map_err(|e| format!("Serialize failed: {}", e))?;
            let update_payload = self.maybe_encrypt(&update_payload)?;
            self.transport.send(&update_payload)?;
            bytes_sent += update_payload.len();
        } else {
            // Send a no-op ack so responder knows we're done
            let ack = SyncMessage::Ack { success: true };
            let ack_payload = serde_json::to_vec(&ack).map_err(|e| format!("Serialize failed: {}", e))?;
            let ack_payload = self.maybe_encrypt(&ack_payload)?;
            self.transport.send(&ack_payload)?;
            bytes_sent += ack_payload.len();
        }

        // 5. Apply remote diff
        if let Some(diff) = remote_diff {
            if !diff.is_empty() {
                self.crdt.apply_update(&diff)?;
            }
        }

        // 6. Attachment sync
        let attach_stats = if local_supports && remote_supports {
            self.sync_attachments(true)
                .map_err(|e| format!("Attachment sync failed: {}", e))?
        } else {
            AttachmentSyncStats::default()
        };

        Ok(SyncResult {
            success: true,
            direction,
            bytes_sent,
            bytes_received,
            attachments_sent: attach_stats.sent,
            attachments_received: attach_stats.received,
            attachment_bytes_sent: attach_stats.bytes_sent,
            attachment_bytes_received: attach_stats.bytes_received,
            attachment_incomplete: attach_stats.incomplete,
            error: None,
        })
    }

    /// Execute sync as the responder (receives state vector first).
    ///
    /// Protocol:
    /// 1. Receive remote state vector
    /// 2. Compute diff, send our SV + diff
    /// 3. Receive remote diff
    /// 4. Apply remote diff
    /// 5. Sync attachments (if both sides support it)
    pub fn sync_responder(&mut self) -> Result<SyncResult, String> {
        let mut bytes_sent = 0usize;
        let mut bytes_received = 0usize;
        let local_supports = !self.local_attachment_manifest.is_empty()
            || self.attachments_dir.is_some();

        // 1. Receive remote state vector request
        let remote_raw = self.transport.recv()?;
        bytes_received += remote_raw.len();
        let remote_decrypted = self.maybe_decrypt(&remote_raw)?;
        let request: SyncMessage = serde_json::from_slice(&remote_decrypted)
            .map_err(|e| format!("Deserialize SV request: {}", e))?;

        let (remote_sv, remote_supports) = match request {
            SyncMessage::StateVectorRequest {
                state_vector,
                supports_attachments,
                ..
            } => (state_vector, supports_attachments),
            _ => return Err("Expected StateVectorRequest".to_string()),
        };

        // 2. Compute diff and send our SV + diff
        let local_sv = self.crdt.state_vector();
        let local_sv_copy = local_sv.clone();
        let diff = if local_sv != remote_sv {
            let d = self.crdt.encode_diff(&remote_sv)?;
            if d.len() > 2 {
                Some(d)
            } else {
                None
            }
        } else {
            None
        };

        let response = SyncMessage::StateVectorResponse {
            state_vector: local_sv,
            diff,
            supports_attachments: local_supports,
        };
        let resp_payload = serde_json::to_vec(&response).map_err(|e| format!("Serialize failed: {}", e))?;
        let resp_payload = self.maybe_encrypt(&resp_payload)?;
        self.transport.send(&resp_payload)?;
        bytes_sent += resp_payload.len();

        // 3. Receive remote diff
        let update_raw = self.transport.recv()?;
        bytes_received += update_raw.len();
        let update_decrypted = self.maybe_decrypt(&update_raw)?;
        let update_msg: SyncMessage = serde_json::from_slice(&update_decrypted)
            .map_err(|e| format!("Deserialize update: {}", e))?;

        let remote_diff = match &update_msg {
            SyncMessage::Update { encrypted_update } if !encrypted_update.is_empty() => {
                Some(encrypted_update.clone())
            }
            _ => None,
        };

        let direction = Self::classify_direction(&local_sv_copy, &remote_sv, &remote_diff);

        // 4. Apply remote diff
        if let Some(diff) = remote_diff {
            if !diff.is_empty() {
                self.crdt.apply_update(&diff)?;
            }
        }

        // 5. Attachment sync
        let attach_stats = if local_supports && remote_supports {
            self.sync_attachments(false)
                .map_err(|e| format!("Attachment sync failed: {}", e))?
        } else {
            AttachmentSyncStats::default()
        };

        Ok(SyncResult {
            success: true,
            direction,
            bytes_sent,
            bytes_received,
            attachments_sent: attach_stats.sent,
            attachments_received: attach_stats.received,
            attachment_bytes_sent: attach_stats.bytes_sent,
            attachment_bytes_received: attach_stats.bytes_received,
            attachment_incomplete: attach_stats.incomplete,
            error: None,
        })
    }

    // ========================================================================
    // Attachment sync
    // ========================================================================

    /// Exchange manifests and transfer missing attachments.
    ///
    /// Phase 1: initiator requests missing files, responder provides them.
    /// Phase 2: responder requests missing files, initiator provides them.
    fn sync_attachments(&mut self, is_initiator: bool) -> Result<AttachmentSyncStats, String> {
        let mut stats = AttachmentSyncStats::default();

        // Clone all data we need before any mutable borrows of self
        let dir = self.attachments_dir.clone().ok_or("Attachments dir not set")?;
        let local_manifest: Vec<AttachmentManifestItem> =
            self.local_attachment_manifest.clone();
        let local_ids: HashSet<_> =
            local_manifest.iter().map(|i| i.file_id.clone()).collect();
        let local_sizes: HashMap<_, _> =
            local_manifest.iter().map(|i| (i.file_id.clone(), i.size)).collect();

        // 1. Exchange manifests
        let manifest_msg = SyncMessage::AttachmentManifest {
            file_ids: local_ids.iter().cloned().collect(),
            file_sizes: local_sizes.clone(),
        };
        let payload = serde_json::to_vec(&manifest_msg).map_err(|e| format!("Serialize failed: {}", e))?;
        let payload = self.maybe_encrypt(&payload)?;
        self.transport.send(&payload)?;

        let remote_raw = self.transport.recv()?;
        let remote_decrypted = self.maybe_decrypt(&remote_raw)?;
        let remote_manifest = match serde_json::from_slice::<SyncMessage>(&remote_decrypted) {
            Ok(SyncMessage::AttachmentManifest { file_ids, file_sizes }) => file_sizes,
            Ok(_) | Err(_) => {
                // Unexpected message — skip attachment sync
                return Ok(stats);
            }
        };

        let remote_ids: HashSet<_> = remote_manifest.keys().cloned().collect();

        // 2. Compute missing files for each side
        let (my_missing, their_missing) = if is_initiator {
            (
                remote_ids
                    .difference(&local_ids)
                    .map(|id| AttachmentManifestItem {
                        file_id: id.clone(),
                        size: *remote_manifest.get(id).unwrap_or(&0),
                    })
                    .collect::<Vec<_>>(),
                local_ids
                    .difference(&remote_ids)
                    .map(|id| AttachmentManifestItem {
                        file_id: id.clone(),
                        size: *local_sizes.get(id).unwrap_or(&0),
                    })
                    .collect::<Vec<_>>(),
            )
        } else {
            (
                local_ids
                    .difference(&remote_ids)
                    .map(|id| AttachmentManifestItem {
                        file_id: id.clone(),
                        size: *local_sizes.get(id).unwrap_or(&0),
                    })
                    .collect::<Vec<_>>(),
                remote_ids
                    .difference(&local_ids)
                    .map(|id| AttachmentManifestItem {
                        file_id: id.clone(),
                        size: *remote_manifest.get(id).unwrap_or(&0),
                    })
                    .collect::<Vec<_>>(),
            )
        };

        // Phase 1: initiator requests, responder provides
        if is_initiator {
            for item in &my_missing {
                if let Err(e) = self.request_attachment(item, &dir, &mut stats) {
                    eprintln!("[sync] Failed to receive {}: {}", item.file_id, e);
                    stats.incomplete = true;
                }
            }
            self.send_done()?;
            self.serve_requests(&their_missing, &dir, &mut stats)?;
        } else {
            self.serve_requests(&their_missing, &dir, &mut stats)?;
            for item in &my_missing {
                if let Err(e) = self.request_attachment(item, &dir, &mut stats) {
                    eprintln!("[sync] Failed to receive {}: {}", item.file_id, e);
                    stats.incomplete = true;
                }
            }
            self.send_done()?;
        }

        Ok(stats)
    }

    fn send_done(&mut self) -> Result<(), String> {
        let done = SyncMessage::AttachmentDone;
        let payload = serde_json::to_vec(&done).map_err(|e| format!("Serialize failed: {}", e))?;
        let payload = self.maybe_encrypt(&payload)?;
        self.transport.send(&payload)
    }

    /// Request a single attachment from remote and write it to disk.
    fn request_attachment(
        &mut self,
        item: &AttachmentManifestItem,
        dir: &str,
        stats: &mut AttachmentSyncStats,
    ) -> Result<(), String> {
        // Send request
        let req = SyncMessage::AttachmentRequest {
            file_id: item.file_id.clone(),
        };
        let payload = serde_json::to_vec(&req).map_err(|e| format!("Serialize failed: {}", e))?;
        let payload = self.maybe_encrypt(&payload)?;
        self.transport.send(&payload)?;

        // Receive chunks
        let part_path = format!("{}/{}.solo.part", dir, item.file_id);
        let final_path = format!("{}/{}.solo", dir, item.file_id);

        let mut file =
            std::fs::File::create(&part_path).map_err(|e| format!("Create part file: {}", e))?;
        let mut total_received = 0u64;

        loop {
            let chunk_raw = self.transport.recv()?;
            let chunk_decrypted = self.maybe_decrypt(&chunk_raw)?;
            let chunk_msg: SyncMessage = serde_json::from_slice(&chunk_decrypted)
                .map_err(|e| format!("Deserialize chunk: {}", e))?;

            match chunk_msg {
                SyncMessage::AttachmentChunk {
                    file_id,
                    data,
                    is_last,
                    ..
                } => {
                    if file_id != item.file_id {
                        return Err("File ID mismatch".to_string());
                    }
                    file.write_all(&data)
                        .map_err(|e| format!("Write chunk: {}", e))?;
                    total_received += data.len() as u64;
                    stats.bytes_received += data.len();
                    if is_last {
                        break;
                    }
                }
                _ => return Err("Expected AttachmentChunk".to_string()),
            }
        }

        drop(file);

        // Verify size
        if total_received != item.size {
            std::fs::remove_file(&part_path).ok();
            return Err(format!(
                "Size mismatch for {}: expected {} got {}",
                item.file_id, item.size, total_received
            ));
        }

        // Rename .part -> final
        std::fs::rename(&part_path, &final_path)
            .map_err(|e| format!("Rename {}: {}", item.file_id, e))?;

        stats.received += 1;
        Ok(())
    }

    /// Serve attachment requests from remote until AttachmentDone is received.
    fn serve_requests(
        &mut self,
        available: &[AttachmentManifestItem],
        dir: &str,
        stats: &mut AttachmentSyncStats,
    ) -> Result<(), String> {
        let available_map: HashMap<_, _> = available.iter().map(|i| (&i.file_id, i.size)).collect();

        loop {
            let req_raw = self.transport.recv()?;
            let req_decrypted = self.maybe_decrypt(&req_raw)?;
            let req: SyncMessage = serde_json::from_slice(&req_decrypted)
                .map_err(|e| format!("Deserialize request: {}", e))?;

            match req {
                SyncMessage::AttachmentDone => break,
                SyncMessage::AttachmentRequest { file_id } => {
                    if let Some(&expected_size) = available_map.get(&file_id) {
                        if let Err(e) = self.send_attachment_chunks(&file_id, dir, expected_size, stats) {
                            eprintln!("[sync] Failed to send {}: {}", file_id, e);
                            stats.incomplete = true;
                        }
                    } else {
                        // File not available — send empty done signal per file
                        // (simplified: just log and continue)
                        eprintln!("[sync] Requested unknown attachment: {}", file_id);
                        stats.incomplete = true;
                    }
                }
                _ => return Err("Expected AttachmentRequest or AttachmentDone".to_string()),
            }
        }

        Ok(())
    }

    /// Send a single attachment file in chunks.
    fn send_attachment_chunks(
        &mut self,
        file_id: &str,
        dir: &str,
        _expected_size: u64,
        stats: &mut AttachmentSyncStats,
    ) -> Result<(), String> {
        let file_path = format!("{}/{}.solo", dir, file_id);
        let mut file = std::fs::File::open(&file_path)
            .map_err(|e| format!("Open attachment {}: {}", file_id, e))?;

        let file_size = file.metadata().map(|m| m.len()).unwrap_or(0);
        let total_chunks =
            ((file_size + ATTACHMENT_CHUNK_SIZE as u64 - 1) / ATTACHMENT_CHUNK_SIZE as u64) as u32;

        let mut buffer = vec![0u8; ATTACHMENT_CHUNK_SIZE];
        let mut chunk_index = 0u32;

        loop {
            let n = file
                .read(&mut buffer)
                .map_err(|e| format!("Read chunk {}: {}", file_id, e))?;
            if n == 0 {
                break;
            }

            chunk_index += 1;
            let is_last = n < ATTACHMENT_CHUNK_SIZE || chunk_index == total_chunks;

            let chunk = SyncMessage::AttachmentChunk {
                file_id: file_id.to_string(),
                chunk_index,
                total_chunks,
                data: buffer[..n].to_vec(),
                is_last,
            };

            let payload = serde_json::to_vec(&chunk).map_err(|e| format!("Serialize failed: {}", e))?;
            let payload = self.maybe_encrypt(&payload)?;
            self.transport.send(&payload)?;
            stats.bytes_sent += n;

            if is_last {
                break;
            }
        }

        stats.sent += 1;
        Ok(())
    }

    // ========================================================================
    // Helpers
    // ========================================================================

    fn classify_direction(
        local_sv: &[u8],
        remote_sv: &[u8],
        remote_diff: &Option<Vec<u8>>,
    ) -> SyncDirection {
        let sv_match = local_sv == remote_sv;
        let has_remote = remote_diff.as_ref().is_some_and(|d| d.len() > 2);
        match (sv_match, has_remote) {
            (true, false) => SyncDirection::NoChange,
            (false, false) => SyncDirection::Pushed,
            (true, true) => SyncDirection::Pulled,
            (false, true) => SyncDirection::Merged,
        }
    }

    /// Maximum payload size before encryption (60 KB, leaving headroom under Noise ~65535 limit).
    const MAX_PAYLOAD_SIZE: usize = 60_000;

    fn maybe_encrypt(&mut self, data: &[u8]) -> Result<Vec<u8>, String> {
        if data.len() > Self::MAX_PAYLOAD_SIZE {
            return Err(format!(
                "Sync message too large ({} bytes), exceeding safe transfer limit. Try reducing attachments or emptying trash before sync.",
                data.len()
            ));
        }
        match self.channel.as_mut() {
            Some(ch) => ch.encrypt(data),
            None => Ok(data.to_vec()),
        }
    }

    fn maybe_decrypt(&mut self, data: &[u8]) -> Result<Vec<u8>, String> {
        match self.channel.as_mut() {
            Some(ch) => ch.decrypt(data),
            None => Ok(data.to_vec()),
        }
    }
}

/// Statistics returned by attachment sync phase.
#[derive(Debug, Default)]
struct AttachmentSyncStats {
    sent: usize,
    received: usize,
    bytes_sent: usize,
    bytes_received: usize,
    incomplete: bool,
}

/// Extract attachment file IDs and encrypted sizes from Profile JSON.
///
/// Scans `unifiedObjects.objects[].attachments[]` and
/// `unifiedObjects.objects[].properties` for `"type": "attachment"` entries.
/// Then verifies each file exists in `attachments_dir` and records its actual size.
pub fn extract_attachment_manifest(profile_json: &str, attachments_dir: &str) -> Vec<AttachmentManifestItem> {
    let value: serde_json::Value = serde_json::from_str(profile_json).unwrap_or_default();
    let mut file_ids = Vec::new();

    // Scan unifiedObjects.objects[].attachments[]
    if let Some(objects) = value
        .get("unifiedObjects")
        .and_then(|v| v.get("objects"))
        .and_then(|v| v.as_array())
    {
        for obj in objects {
            // Top-level attachments array
            if let Some(attachments) = obj.get("attachments").and_then(|v| v.as_array()) {
                for att in attachments {
                    if let Some(file_id) = att.get("fileId").and_then(|v| v.as_str()) {
                        file_ids.push(file_id.to_string());
                    }
                }
            }
            // Properties with type == "attachment"
            if let Some(properties) = obj.get("properties").and_then(|v| v.as_object()) {
                for (_, prop) in properties {
                    if let Some(prop_type) = prop.get("type").and_then(|v| v.as_str()) {
                        if prop_type == "attachment" {
                            if let Some(file_id) = prop.get("fileId").and_then(|v| v.as_str()) {
                                file_ids.push(file_id.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    // Deduplicate and verify files exist on disk
    let mut seen = HashSet::new();
    let mut manifest = Vec::new();
    for file_id in file_ids {
        if !seen.insert(file_id.clone()) {
            continue;
        }
        let path = format!("{}/{}.solo", attachments_dir, file_id);
        if let Ok(meta) = std::fs::metadata(&path) {
            manifest.push(AttachmentManifestItem {
                file_id,
                size: meta.len(),
            });
        }
    }

    manifest
}

// ============================================================================
// Mock transport for testing
// ============================================================================

/// In-memory transport using channels, simulating a network connection.
pub struct MockTransport {
    tx: std::sync::mpsc::Sender<Vec<u8>>,
    rx: std::sync::mpsc::Receiver<Vec<u8>>,
}

impl MockTransport {
    /// Create a pair of connected mock transports.
    pub fn pair() -> (Self, Self) {
        let (tx_a, rx_a) = std::sync::mpsc::channel();
        let (tx_b, rx_b) = std::sync::mpsc::channel();
        (
            Self { tx: tx_a, rx: rx_b },
            Self { tx: tx_b, rx: rx_a },
        )
    }
}

impl Transport for MockTransport {
    fn send(&mut self, data: &[u8]) -> Result<(), String> {
        self.tx
            .send(data.to_vec())
            .map_err(|e| format!("send failed: {}", e))
    }

    fn recv(&mut self) -> Result<Vec<u8>, String> {
        self.rx.recv().map_err(|e| format!("recv failed: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::super::crdt::DocMeta;
    use super::*;
    use crate::vault::*;

    fn make_profile_a() -> ProfileData {
        ProfileData {
            identity: Some(IdentityData {
                full_name: Some("Alice Smith".to_string()),
                given_name: Some("Alice".to_string()),
                family_name: Some("Smith".to_string()),
                date_of_birth: Some("1990-01-15".to_string()),
                gender: Some("female".to_string()),
                nationality: Some("US".to_string()),
                id_cards: vec![],
                contact: None,
                addresses: vec![],
            }),
            travel: None,
            financial: None,
            professional: None,
            preferences: None,
        }
    }

    fn make_profile_b() -> ProfileData {
        ProfileData {
            identity: None,
            travel: Some(TravelData {
                passports: vec![PassportData {
                    number: Some("AB123456".to_string()),
                    country: Some("US".to_string()),
                    issue_date: Some("2020-01-01".to_string()),
                    expiry_date: Some("2030-06-01".to_string()),
                    holder_name: Some("Bob Jones".to_string()),
                    is_deleted: false,
                    deleted_at: None,
                }],
                visas: vec![],
                travel_history: vec![],
            }),
            financial: None,
            professional: None,
            preferences: None,
        }
    }

    fn make_meta() -> DocMeta {
        DocMeta {
            profile_id: "test-profile".to_string(),
            version: 1,
            last_modified: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    /// Helper: run sync between two engines in parallel, return results + profiles.
    fn run_sync(
        mut engine_a: SyncEngine,
        mut engine_b: SyncEngine,
    ) -> (SyncResult, ProfileData, SyncResult, ProfileData) {
        let handle_a = std::thread::spawn(move || {
            let r = engine_a.sync_initiator();
            (r, engine_a.crdt.to_profile().unwrap())
        });
        let result_b = engine_b.sync_responder();
        let profile_b = engine_b.crdt.to_profile().unwrap();
        let (result_a, profile_a) = handle_a.join().unwrap();
        (result_a.unwrap(), profile_a, result_b.unwrap(), profile_b)
    }

    #[test]
    fn test_sync_bidirectional_changes() {
        // A has identity, B has travel — after sync both should have both
        let meta = make_meta();
        let doc_a = SoloDoc::from_profile(&make_profile_a(), &meta);
        let doc_b = SoloDoc::from_profile(&make_profile_b(), &meta);

        let (transport_a, transport_b) = MockTransport::pair();
        let engine_a = SyncEngine::new(doc_a, None, Box::new(transport_a));
        let engine_b = SyncEngine::new(doc_b, None, Box::new(transport_b));

        let (_, profile_a, _, profile_b) = run_sync(engine_a, engine_b);

        assert!(profile_a.identity.is_some(), "A should have identity");
        assert!(
            profile_a.travel.is_some(),
            "A should have travel after sync"
        );
        assert!(
            profile_b.identity.is_some(),
            "B should have identity after sync"
        );
        assert!(profile_b.travel.is_some(), "B should have travel");
    }

    #[test]
    fn test_sync_no_changes() {
        // Create one doc and derive a second from its update — both will have
        // identical state vectors, simulating "already synced" devices.
        let meta = make_meta();
        let profile = make_profile_a();

        let doc_source = SoloDoc::from_profile(&profile, &meta);
        let full_update = doc_source.encode_state_as_update();
        let doc_copy = SoloDoc::from_update(&full_update).unwrap();

        // Verify state vectors match
        let sv1 = doc_source.state_vector();
        let sv2 = doc_copy.state_vector();
        assert_eq!(sv1, sv2, "State vectors should match for no-change test");

        let (transport_a, transport_b) = MockTransport::pair();
        let engine_a = SyncEngine::new(doc_source, None, Box::new(transport_a));
        let engine_b = SyncEngine::new(doc_copy, None, Box::new(transport_b));

        let (result_a, _, result_b, _) = run_sync(engine_a, engine_b);

        assert_eq!(result_a.direction, SyncDirection::NoChange);
        assert_eq!(result_b.direction, SyncDirection::NoChange);
    }

    #[test]
    fn test_sync_push_only() {
        // A has changes, B is empty — A pushes, B pulls
        let meta = make_meta();
        let doc_a = SoloDoc::from_profile(&make_profile_a(), &meta);
        let empty = ProfileData {
            identity: None,
            travel: None,
            financial: None,
            professional: None,
            preferences: None,
        };
        let doc_b = SoloDoc::from_profile(&empty, &meta);

        let (transport_a, transport_b) = MockTransport::pair();
        let engine_a = SyncEngine::new(doc_a, None, Box::new(transport_a));
        let engine_b = SyncEngine::new(doc_b, None, Box::new(transport_b));

        let (_, _, _, profile_b) = run_sync(engine_a, engine_b);

        assert!(
            profile_b.identity.is_some(),
            "B should have identity after sync"
        );
        assert_eq!(
            profile_b.identity.unwrap().full_name,
            Some("Alice Smith".to_string())
        );
    }

    #[test]
    fn test_sync_with_encrypted_channel() {
        let pairing_key = b"test-pairing-key-sync";
        let key_a = SecureChannel::derive_keypair(pairing_key, b"device-a");
        let key_b = SecureChannel::derive_keypair(pairing_key, b"device-b");
        let (ch_a, ch_b) = SecureChannel::handshake_ik(&key_a, &key_b).unwrap();

        let meta = make_meta();
        let doc_a = SoloDoc::from_profile(&make_profile_a(), &meta);
        let doc_b = SoloDoc::from_profile(&make_profile_b(), &meta);

        let (transport_a, transport_b) = MockTransport::pair();
        let engine_a = SyncEngine::new(doc_a, Some(ch_a), Box::new(transport_a));
        let engine_b = SyncEngine::new(doc_b, Some(ch_b), Box::new(transport_b));

        let (_, profile_a, _, profile_b) = run_sync(engine_a, engine_b);

        assert!(profile_a.travel.is_some(), "A should have travel");
        assert!(profile_b.identity.is_some(), "B should have identity");
    }

    #[test]
    fn test_extract_attachment_manifest_empty() {
        let json = r#"{"identity":null,"travel":null}"#;
        let manifest = extract_attachment_manifest(json, "/tmp");
        assert!(manifest.is_empty());
    }

    #[test]
    fn test_extract_attachment_manifest_from_attachments_array() {
        let json = r#"{
            "unifiedObjects": {
                "objects": [{
                    "attachments": [
                        {"fileId": "aaa", "size": 123},
                        {"fileId": "bbb", "size": 456}
                    ]
                }]
            }
        }"#;
        // Without actual files on disk, manifest should be empty
        let manifest = extract_attachment_manifest(json, "/tmp/nonexistent");
        assert!(manifest.is_empty());
    }
}
