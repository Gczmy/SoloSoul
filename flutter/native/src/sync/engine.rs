//! Sync engine - CRDT-based incremental synchronization
//!
//! Orchestrates profile sync between devices using:
//! - `SoloDoc` for CRDT conflict resolution
//! - `SecureChannel` for Noise-encrypted transport
//! - `Transport` trait for pluggable network backends

use serde::{Deserialize, Serialize};

use super::crdt::{DocMeta, SoloDoc};
use super::protocol::SecureChannel;
use crate::vault::ProfileData;

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
    pub error: Option<String>,
}

/// Sync protocol messages exchanged between devices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncMessage {
    /// Request with sender's state vector
    StateVectorRequest {
        account_id: String,
        state_vector: Vec<u8>,
    },
    /// Response with receiver's state vector and optional diff
    StateVectorResponse {
        state_vector: Vec<u8>,
        diff: Option<Vec<u8>>,
    },
    /// Update payload (encrypted diff)
    Update { encrypted_update: Vec<u8> },
    /// Acknowledgment
    Ack { success: bool },
}

/// Transport abstraction for sync messages
pub trait Transport: Send {
    fn send(&mut self, data: &[u8]) -> Result<(), String>;
    fn recv(&mut self) -> Result<Vec<u8>, String>;
}

/// Sync engine coordinating CRDT doc, encryption, and transport
pub struct SyncEngine {
    pub crdt: SoloDoc,
    pub channel: Option<SecureChannel>,
    pub transport: Box<dyn Transport>,
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
        }
    }

    /// Execute sync as the initiator (sends state vector first).
    ///
    /// Protocol:
    /// 1. Send our state vector
    /// 2. Receive remote state vector + their diff
    /// 3. Compute our diff relative to remote SV
    /// 4. Send our diff
    /// 5. Apply remote diff
    pub fn sync_initiator(&mut self) -> Result<SyncResult, String> {
        let mut bytes_sent = 0usize;
        let mut bytes_received = 0usize;

        // 1. Send our state vector
        let local_sv = self.crdt.state_vector();
        let request = SyncMessage::StateVectorRequest {
            account_id: String::new(),
            state_vector: local_sv,
        };
        let payload = self.maybe_encrypt(&serde_json::to_vec(&request).unwrap());
        self.transport.send(&payload)?;
        bytes_sent += payload.len();

        // 2. Receive remote response (SV + optional diff)
        let remote_raw = self.transport.recv()?;
        bytes_received += remote_raw.len();
        let remote_decrypted = self.maybe_decrypt(&remote_raw)?;
        let response: SyncMessage = serde_json::from_slice(&remote_decrypted)
            .map_err(|e| format!("Deserialize SV response: {}", e))?;

        let (remote_sv, remote_diff) = match response {
            SyncMessage::StateVectorResponse { state_vector, diff } => (state_vector, diff),
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
            let update_payload = self.maybe_encrypt(&serde_json::to_vec(&update_msg).unwrap());
            self.transport.send(&update_payload)?;
            bytes_sent += update_payload.len();
        } else {
            // Send a no-op ack so responder knows we're done
            let ack = SyncMessage::Ack { success: true };
            let ack_payload = self.maybe_encrypt(&serde_json::to_vec(&ack).unwrap());
            self.transport.send(&ack_payload)?;
            bytes_sent += ack_payload.len();
        }

        // 5. Apply remote diff
        if let Some(diff) = remote_diff {
            if !diff.is_empty() {
                self.crdt.apply_update(&diff)?;
            }
        }

        Ok(SyncResult {
            success: true,
            direction,
            bytes_sent,
            bytes_received,
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
    pub fn sync_responder(&mut self) -> Result<SyncResult, String> {
        let mut bytes_sent = 0usize;
        let mut bytes_received = 0usize;

        // 1. Receive remote state vector request
        let remote_raw = self.transport.recv()?;
        bytes_received += remote_raw.len();
        let remote_decrypted = self.maybe_decrypt(&remote_raw)?;
        let request: SyncMessage = serde_json::from_slice(&remote_decrypted)
            .map_err(|e| format!("Deserialize SV request: {}", e))?;

        let remote_sv = match request {
            SyncMessage::StateVectorRequest { state_vector, .. } => state_vector,
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
        };
        let resp_payload = self.maybe_encrypt(&serde_json::to_vec(&response).unwrap());
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

        Ok(SyncResult {
            success: true,
            direction,
            bytes_sent,
            bytes_received,
            error: None,
        })
    }

    fn classify_direction(
        local_sv: &[u8],
        remote_sv: &[u8],
        remote_diff: &Option<Vec<u8>>,
    ) -> SyncDirection {
        let sv_match = local_sv == remote_sv;
        let has_remote = remote_diff.as_ref().map_or(false, |d| d.len() > 2);
        match (sv_match, has_remote) {
            (true, false) => SyncDirection::NoChange,
            (false, false) => SyncDirection::Pushed,
            (true, true) => SyncDirection::Pulled,
            (false, true) => SyncDirection::Merged,
        }
    }

    fn maybe_encrypt(&mut self, data: &[u8]) -> Vec<u8> {
        match self.channel.as_mut() {
            Some(ch) => ch.encrypt(data),
            None => data.to_vec(),
        }
    }

    fn maybe_decrypt(&mut self, data: &[u8]) -> Result<Vec<u8>, String> {
        match self.channel.as_mut() {
            Some(ch) => ch.decrypt(data),
            None => Ok(data.to_vec()),
        }
    }
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
        (Self { tx: tx_a, rx: rx_b }, Self { tx: tx_b, rx: rx_a })
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
}
