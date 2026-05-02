//! Sync protocol - Noise encrypted channel and API definitions

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use snow::{Builder, HandshakeState, TransportState};

/// Noise protocol parameters — IK mode for pre-shared device keys.
const NOISE_PATTERN: &str = "Noise_IK_25519_ChaChaPoly_BLAKE2s";

/// Noise-encrypted secure channel for device-to-device sync.
///
/// Uses Noise_IK handshake: initiator pre-knows responder's static public key,
/// enabling 1-RTT authenticated encryption after initial pairing.
pub struct SecureChannel {
    transport: TransportState,
}

/// X25519 keypair for Noise handshake.
pub struct Keypair {
    pub private: [u8; 32],
    pub public: [u8; 32],
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

    /// Generate a deterministic keypair from a pairing key + device salt.
    ///
    /// Each device derives its own keypair by mixing the shared pairing key
    /// with a device-specific identifier (e.g. device name or ID).
    pub fn derive_keypair(pairing_key: &[u8], device_salt: &[u8]) -> Keypair {
        let mut hasher = Sha256::new();
        hasher.update(b"solosoul-noise-keypair-v2");
        hasher.update(pairing_key);
        hasher.update(device_salt);
        let hash = hasher.finalize();
        let mut private = [0u8; 32];
        private.copy_from_slice(&hash[..32]);

        // Derive public key from private key using X25519 scalar multiplication
        let secret = x25519_dalek::StaticSecret::from(private);
        let public = x25519_dalek::PublicKey::from(&secret);
        Keypair {
            private,
            public: *public.as_bytes(),
        }
    }

    /// Perform a Noise_IK handshake (convenience: single-process testing).
    ///
    /// In production, `initiator_ik` and `responder_ik` are called separately
    /// on each device. This method combines both sides for testing.
    pub fn handshake_ik(
        initiator_key: &Keypair,
        responder_key: &Keypair,
    ) -> Result<(Self, Self), String> {
        let mut initiator: HandshakeState =
            Builder::new(NOISE_PATTERN.parse().unwrap())
                .local_private_key(&initiator_key.private)
                .remote_public_key(&responder_key.public)
                .build_initiator()
                .map_err(|e| format!("Initiator build: {}", e))?;

        let mut responder: HandshakeState =
            Builder::new(NOISE_PATTERN.parse().unwrap())
                .local_private_key(&responder_key.private)
                .build_responder()
                .map_err(|e| format!("Responder build: {}", e))?;

        let mut buf = vec![0u8; 65535];

        // Message 1: initiator → responder (carries initiator's static key)
        let len1 = initiator
            .write_message(&[], &mut buf)
            .map_err(|e| format!("Msg1 write: {}", e))?;
        responder
            .read_message(&buf[..len1], &mut vec![0u8; 65535])
            .map_err(|e| format!("Msg1 read: {}", e))?;

        // Message 2: responder → initiator (final, carries responder's static key)
        let len2 = responder
            .write_message(&[], &mut buf)
            .map_err(|e| format!("Msg2 write: {}", e))?;
        initiator
            .read_message(&buf[..len2], &mut vec![0u8; 65535])
            .map_err(|e| format!("Msg2 read: {}", e))?;

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

    /// Build the initiator side of a Noise_IK handshake.
    ///
    /// Call `write_message` / `read_message` manually to complete the handshake,
    /// then call `into_transport_mode()` to get the encrypted channel.
    pub fn build_initiator(
        local_private: &[u8; 32],
        remote_public: &[u8; 32],
    ) -> Result<HandshakeState, String> {
        Builder::new(NOISE_PATTERN.parse().unwrap())
            .local_private_key(local_private)
            .remote_public_key(remote_public)
            .build_initiator()
            .map_err(|e| format!("Initiator build: {}", e))
    }

    /// Build the responder side of a Noise_IK handshake.
    pub fn build_responder(
        local_private: &[u8; 32],
    ) -> Result<HandshakeState, String> {
        Builder::new(NOISE_PATTERN.parse().unwrap())
            .local_private_key(local_private)
            .build_responder()
            .map_err(|e| format!("Responder build: {}", e))
    }

    /// Wrap a completed transport state into a SecureChannel.
    pub fn from_transport(transport: TransportState) -> Self {
        Self { transport }
    }

    /// Legacy XX handshake — kept for backward compatibility tests only.
    #[deprecated(note = "Use handshake_ik() for production")]
    pub fn handshake(pairing_key: &[u8]) -> Result<(Self, Self), String> {
        let private_key = Self::derive_private_key(pairing_key);

        let xx_pattern: snow::params::NoiseParams = "Noise_XX_25519_ChaChaPoly_BLAKE2s"
            .parse()
            .unwrap();

        let mut initiator: HandshakeState =
            Builder::new(xx_pattern.clone())
                .local_private_key(&private_key)
                .prologue(pairing_key)
                .build_initiator()
                .map_err(|e| format!("Initiator build: {}", e))?;

        let mut responder: HandshakeState =
            Builder::new(xx_pattern)
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

    #[test]
    fn test_ik_handshake_roundtrip() {
        let pairing_key = b"ik-test-pairing-key";
        let initiator = SecureChannel::derive_keypair(pairing_key, b"device-a");
        let responder = SecureChannel::derive_keypair(pairing_key, b"device-b");

        let (mut alice, mut bob) = SecureChannel::handshake_ik(&initiator, &responder).unwrap();

        let msg = b"hello via IK";
        let encrypted = alice.encrypt(msg);
        assert_ne!(encrypted, msg, "ciphertext should differ from plaintext");

        let decrypted = bob.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, msg);
    }

    #[test]
    fn test_ik_bidirectional() {
        let pairing_key = b"ik-bidir-key";
        let key_a = SecureChannel::derive_keypair(pairing_key, b"alice-device");
        let key_b = SecureChannel::derive_keypair(pairing_key, b"bob-device");

        let (mut alice, mut bob) = SecureChannel::handshake_ik(&key_a, &key_b).unwrap();

        // alice → bob
        let msg_a = b"alice says hi via IK";
        let enc_a = alice.encrypt(msg_a);
        assert_eq!(bob.decrypt(&enc_a).unwrap(), msg_a);

        // bob → alice
        let msg_b = b"bob says hi back via IK";
        let enc_b = bob.encrypt(msg_b);
        assert_eq!(alice.decrypt(&enc_b).unwrap(), msg_b);
    }

    #[test]
    fn test_ik_tampered_ciphertext_fails() {
        let pairing_key = b"ik-tamper-key";
        let key_a = SecureChannel::derive_keypair(pairing_key, b"a");
        let key_b = SecureChannel::derive_keypair(pairing_key, b"b");

        let (mut alice, mut bob) = SecureChannel::handshake_ik(&key_a, &key_b).unwrap();

        let mut encrypted = alice.encrypt(b"secret message");
        if !encrypted.is_empty() {
            encrypted[0] ^= 0xFF;
        }
        assert!(bob.decrypt(&encrypted).is_err(), "tampered ciphertext should fail");
    }

    #[test]
    fn test_ik_wrong_remote_public_key_fails() {
        let pairing_key = b"correct-key";
        let wrong_key = b"wrong-key";

        let initiator_kp = SecureChannel::derive_keypair(pairing_key, b"initiator");
        let responder_kp = SecureChannel::derive_keypair(pairing_key, b"responder");
        let wrong_responder_kp = SecureChannel::derive_keypair(wrong_key, b"responder");

        // Build initiator with WRONG responder public key (from different pairing key)
        let mut initiator = SecureChannel::build_initiator(
            &initiator_kp.private,
            &wrong_responder_kp.public, // wrong key!
        ).unwrap();
        let mut responder = SecureChannel::build_responder(&responder_kp.private).unwrap();

        let mut buf = vec![0u8; 65535];

        // Message 1: initiator → responder (encrypted to wrong key)
        let len1 = initiator.write_message(&[], &mut buf).unwrap();
        let result = responder.read_message(&buf[..len1], &mut vec![0u8; 65535]);
        assert!(result.is_err(), "wrong remote public key should fail at message 1");
    }

    #[test]
    fn test_derive_keypair_deterministic() {
        let pairing_key = b"deterministic-test";
        let device_salt = b"my-macbook";

        let kp1 = SecureChannel::derive_keypair(pairing_key, device_salt);
        let kp2 = SecureChannel::derive_keypair(pairing_key, device_salt);

        assert_eq!(kp1.private, kp2.private);
        assert_eq!(kp1.public, kp2.public);
    }

    #[test]
    fn test_derive_keypair_different_devices() {
        let pairing_key = b"same-key";
        let kp_a = SecureChannel::derive_keypair(pairing_key, b"device-a");
        let kp_b = SecureChannel::derive_keypair(pairing_key, b"device-b");

        assert_ne!(kp_a.private, kp_b.private, "different salts should produce different keys");
        assert_ne!(kp_a.public, kp_b.public, "different salts should produce different public keys");
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
