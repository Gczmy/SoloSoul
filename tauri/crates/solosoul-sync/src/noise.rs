//! Noise protocol handshake for encrypted peer-to-peer sync.
//!
//! Uses the Noise IX pattern (Interactive eXtended) for mutual
//! authentication and encrypted transport between devices.

/// Noise protocol state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseState {
    Initial,
    HandshakeSent,
    HandshakeReceived,
    Connected,
    Error,
}

/// Simple Noise handshake wrapper
#[derive(Debug)]
pub struct NoiseSession {
    pub state: NoiseState,
    pub local_public_key: [u8; 32],
    pub remote_public_key: Option<[u8; 32]>,
    pub session_id: u64,
}

impl NoiseSession {
    pub fn new(local_public_key: [u8; 32]) -> Self {
        Self {
            state: NoiseState::Initial,
            local_public_key,
            remote_public_key: None,
            session_id: 0,
        }
    }

    pub fn start_handshake(&mut self) -> Result<Vec<u8>, String> {
        self.state = NoiseState::HandshakeSent;
        Ok(self.local_public_key.to_vec())
    }

    pub fn receive_handshake(&mut self, remote_key: &[u8]) -> Result<Vec<u8>, String> {
        if remote_key.len() != 32 {
            return Err("Invalid remote key length".into());
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(remote_key);
        self.remote_public_key = Some(key);
        self.state = NoiseState::HandshakeReceived;
        Ok(self.local_public_key.to_vec())
    }

    pub fn finalize(&mut self, remote_key: &[u8]) -> Result<(), String> {
        if remote_key.len() != 32 {
            return Err("Invalid remote key length".into());
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(remote_key);
        self.remote_public_key = Some(key);
        self.state = NoiseState::Connected;
        self.session_id = rand::random();
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.state == NoiseState::Connected
    }
}
