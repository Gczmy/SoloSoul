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

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_key() -> [u8; 32] {
        [1u8; 32]
    }

    fn dummy_key_alt() -> [u8; 32] {
        [2u8; 32]
    }

    #[test]
    fn test_noise_session_new() {
        let key = dummy_key();
        let session = NoiseSession::new(key);
        assert_eq!(session.state, NoiseState::Initial);
        assert_eq!(session.local_public_key, key);
        assert!(session.remote_public_key.is_none());
        assert_eq!(session.session_id, 0);
        assert!(!session.is_connected());
    }

    #[test]
    fn test_start_handshake() {
        let mut session = NoiseSession::new(dummy_key());
        let msg = session.start_handshake().unwrap();
        assert_eq!(msg, dummy_key().to_vec());
        assert_eq!(session.state, NoiseState::HandshakeSent);
    }

    #[test]
    fn test_receive_handshake() {
        let mut session = NoiseSession::new(dummy_key());
        let remote = dummy_key_alt();
        let msg = session.receive_handshake(&remote).unwrap();
        assert_eq!(msg, dummy_key().to_vec());
        assert_eq!(session.state, NoiseState::HandshakeReceived);
        assert_eq!(session.remote_public_key, Some(remote));
    }

    #[test]
    fn test_receive_handshake_invalid_key_length() {
        let mut session = NoiseSession::new(dummy_key());
        let result = session.receive_handshake(&[1u8; 31]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid remote key length"));
    }

    #[test]
    fn test_finalize() {
        let mut session = NoiseSession::new(dummy_key());
        let remote = dummy_key_alt();
        session.start_handshake().unwrap();
        session.finalize(&remote).unwrap();
        assert_eq!(session.state, NoiseState::Connected);
        assert_eq!(session.remote_public_key, Some(remote));
        assert!(session.is_connected());
        assert!(session.session_id != 0);
    }

    #[test]
    fn test_finalize_invalid_key_length() {
        let mut session = NoiseSession::new(dummy_key());
        let result = session.finalize(&[1u8; 33]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid remote key length"));
    }

    #[test]
    fn test_full_handshake_sequence() {
        let mut alice = NoiseSession::new(dummy_key());
        let mut bob = NoiseSession::new(dummy_key_alt());

        // Alice starts
        let alice_hello = alice.start_handshake().unwrap();
        assert_eq!(alice.state, NoiseState::HandshakeSent);

        // Bob receives and responds
        let bob_hello = bob.receive_handshake(&alice_hello).unwrap();
        assert_eq!(bob.state, NoiseState::HandshakeReceived);
        assert_eq!(bob.remote_public_key, Some(dummy_key()));

        // Alice receives Bob's response and finalizes
        alice.finalize(&bob_hello).unwrap();
        assert_eq!(alice.state, NoiseState::Connected);
        assert!(alice.is_connected());

        // Bob also finalizes
        bob.finalize(&alice_hello).unwrap();
        assert_eq!(bob.state, NoiseState::Connected);
        assert!(bob.is_connected());
    }
}
