//! Sync protocol - Noise encrypted channel and API definitions

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use snow::{Builder, HandshakeState, TransportState};

/// Noise protocol parameters
const NOISE_PATTERN: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";

/// Noise-encrypted secure channel for device-to-device sync.
///
/// Uses Noise_XX handshake with deterministic keypairs derived from a
/// pairing key. After handshake completes, provides authenticated
/// encryption for all sync messages.
pub struct SecureChannel {
    transport: TransportState,
}

impl SecureChannel {
    /// Derive a deterministic X25519 private key (32 bytes) from a pairing key.
    fn derive_private_key(pairing_key: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"solosoul-noise-keypair-v1");
        hasher.update(pairing_key);
        let hash = hasher.finalize();
        let mut private = [0u8; 32];
        private.copy_from_slice(&hash[..32]);
        private
    }

    /// Perform a full Noise_XX handshake between initiator and responder.
    ///
    /// This is a convenience method for local testing and single-process use.
    /// Returns both transport states.
    pub fn handshake(pairing_key: &[u8]) -> Result<(Self, Self), String> {
        let private_key = Self::derive_private_key(pairing_key);

        let mut initiator: HandshakeState =
            Builder::new(NOISE_PATTERN.parse().unwrap())
                .local_private_key(&private_key)
                .prologue(pairing_key)
                .build_initiator()
                .map_err(|e| format!("Initiator build: {}", e))?;

        let mut responder: HandshakeState =
            Builder::new(NOISE_PATTERN.parse().unwrap())
                .local_private_key(&private_key)
                .prologue(pairing_key)
                .build_responder()
                .map_err(|e| format!("Responder build: {}", e))?;

        let mut buf = vec![0u8; 65535];

        // Message 1: initiator → responder
        let len1 = initiator
            .write_message(&[], &mut buf)
            .map_err(|e| format!("Msg1 write: {}", e))?;
        responder
            .read_message(&buf[..len1], &mut vec![0u8; 65535])
            .map_err(|e| format!("Msg1 read: {}", e))?;

        // Message 2: responder → initiator
        let len2 = responder
            .write_message(&[], &mut buf)
            .map_err(|e| format!("Msg2 write: {}", e))?;
        initiator
            .read_message(&buf[..len2], &mut vec![0u8; 65535])
            .map_err(|e| format!("Msg2 read: {}", e))?;

        // Message 3: initiator → responder (final)
        let len3 = initiator
            .write_message(&[], &mut buf)
            .map_err(|e| format!("Msg3 write: {}", e))?;
        responder
            .read_message(&buf[..len3], &mut vec![0u8; 65535])
            .map_err(|e| format!("Msg3 read: {}", e))?;

        let transport_i = initiator
            .into_transport_mode()
            .map_err(|e| format!("Initiator transport: {}", e))?;
        let transport_r = responder
            .into_transport_mode()
            .map_err(|e| format!("Responder transport: {}", e))?;

        Ok((
            Self { transport: transport_i },
            Self { transport: transport_r },
        ))
    }

    /// Encrypt a plaintext message.
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Vec<u8> {
        let mut buf = vec![0u8; plaintext.len() + 16]; // +16 for AEAD tag
        let len = self
            .transport
            .write_message(plaintext, &mut buf)
            .expect("encrypt failed");
        buf.truncate(len);
        buf
    }

    /// Decrypt a ciphertext message.
    pub fn decrypt(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>, String> {
        let mut buf = vec![0u8; ciphertext.len() + 16];
        let len = self
            .transport
            .read_message(ciphertext, &mut buf)
            .map_err(|e| format!("decrypt failed: {}", e))?;
        buf.truncate(len);
        Ok(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handshake_roundtrip() {
        let pairing_key = b"test-pairing-key-2026";
        let (mut alice, mut bob) = SecureChannel::handshake(pairing_key).unwrap();

        let msg = b"hello from alice";
        let encrypted = alice.encrypt(msg);
        assert_ne!(encrypted, msg, "ciphertext should differ from plaintext");

        let decrypted = bob.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, msg);
    }

    #[test]
    fn test_bidirectional_encrypt_decrypt() {
        let pairing_key = b"bidir-test-key";
        let (mut alice, mut bob) = SecureChannel::handshake(pairing_key).unwrap();

        // alice → bob
        let msg_a = b"alice says hi";
        let enc_a = alice.encrypt(msg_a);
        assert_eq!(bob.decrypt(&enc_a).unwrap(), msg_a);

        // bob → alice
        let msg_b = b"bob says hi back";
        let enc_b = bob.encrypt(msg_b);
        assert_eq!(alice.decrypt(&enc_b).unwrap(), msg_b);
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let pairing_key = b"tamper-test-key";
        let (mut alice, mut bob) = SecureChannel::handshake(pairing_key).unwrap();

        let mut encrypted = alice.encrypt(b"secret message");
        // Flip a bit in the ciphertext
        if !encrypted.is_empty() {
            encrypted[0] ^= 0xFF;
        }
        assert!(bob.decrypt(&encrypted).is_err(), "tampered ciphertext should fail");
    }

    #[test]
    fn test_wrong_pairing_key_fails() {
        let key_a = b"pairing-key-alpha";
        let key_b = b"pairing-key-beta";

        let (mut alice, _) = SecureChannel::handshake(key_a).unwrap();
        let (_, mut bob) = SecureChannel::handshake(key_b).unwrap();

        let encrypted = alice.encrypt(b"secret");
        // Bob with different key cannot decrypt
        assert!(
            bob.decrypt(&encrypted).is_err(),
            "different pairing keys should produce incompatible channels"
        );
    }

    #[test]
    fn test_multiple_messages() {
        let pairing_key = b"multi-msg-key";
        let (mut alice, mut bob) = SecureChannel::handshake(pairing_key).unwrap();

        for i in 0..10 {
            let msg = format!("message {}", i);
            let encrypted = alice.encrypt(msg.as_bytes());
            let decrypted = bob.decrypt(&encrypted).unwrap();
            assert_eq!(decrypted, msg.as_bytes());
        }
    }

    #[test]
    fn test_empty_message() {
        let pairing_key = b"empty-msg-key";
        let (mut alice, mut bob) = SecureChannel::handshake(pairing_key).unwrap();

        let encrypted = alice.encrypt(b"");
        let decrypted = bob.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, b"");
    }
}

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
