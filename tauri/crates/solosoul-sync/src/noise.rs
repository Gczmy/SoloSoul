//! Noise protocol wrapper using the XX handshake pattern.
//!
//! Provides persistent long-term identity keys and an encrypted transport
//! session over an existing `SyncTransport`.

use crate::transport::SyncTransport;
use rand::rngs::OsRng;
use snow::{Builder, TransportState};
use x25519_dalek::{PublicKey, StaticSecret};

const PATTERN: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";

/// Long-term Noise identity keys for this node.
#[derive(Debug, Clone)]
pub struct NoiseKeys {
    secret: [u8; 32],
    public: [u8; 32],
}

impl NoiseKeys {
    /// Generate a fresh key pair.
    pub fn generate() -> Self {
        let secret = StaticSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        Self {
            secret: secret.to_bytes(),
            public: public.to_bytes(),
        }
    }

    /// Restore keys from a persisted secret key.
    pub fn from_secret(secret: [u8; 32]) -> Self {
        let secret_obj = StaticSecret::from(secret);
        let public = PublicKey::from(&secret_obj);
        Self {
            secret,
            public: public.to_bytes(),
        }
    }

    pub fn secret_key(&self) -> &[u8; 32] {
        &self.secret
    }

    pub fn public_key(&self) -> &[u8; 32] {
        &self.public
    }

    /// Short hex fingerprint suitable for manual peer verification.
    pub fn fingerprint(&self) -> String {
        hex::encode(&self.public[..16])
    }
}

/// Encrypted transport session after a successful handshake.
pub struct NoiseSession {
    state: TransportState,
    buffer: Vec<u8>,
}

impl NoiseSession {
    /// Perform an XX handshake as the initiator over the given transport.
    pub fn handshake_initiator(
        transport: &mut SyncTransport,
        keys: &NoiseKeys,
    ) -> Result<Self, String> {
        let mut handshake = Builder::new(PATTERN.parse().map_err(|e| format!("pattern: {:?}", e))?)
            .local_private_key(&keys.secret)
            .build_initiator()
            .map_err(|e| format!("build initiator: {}", e))?;

        let mut buf = vec![0u8; 65535];

        // -> e, es, s, ss
        let len = handshake
            .write_message(&[], &mut buf)
            .map_err(|e| format!("write handshake 1: {}", e))?;
        transport.send_message(&buf[..len])?;

        // <- e, ee, se, s, es
        let msg = transport.receive_message()?;
        handshake
            .read_message(&msg, &mut buf)
            .map_err(|e| format!("read handshake 2: {}", e))?;

        // -> s, se
        let len = handshake
            .write_message(&[], &mut buf)
            .map_err(|e| format!("write handshake 3: {}", e))?;
        transport.send_message(&buf[..len])?;

        let state = handshake
            .into_transport_mode()
            .map_err(|e| format!("into transport: {}", e))?;
        Ok(Self {
            state,
            buffer: vec![0u8; 65535],
        })
    }

    /// Perform an XX handshake as the responder over the given transport.
    pub fn handshake_responder(
        transport: &mut SyncTransport,
        keys: &NoiseKeys,
    ) -> Result<Self, String> {
        let mut handshake = Builder::new(PATTERN.parse().map_err(|e| format!("pattern: {:?}", e))?)
            .local_private_key(&keys.secret)
            .build_responder()
            .map_err(|e| format!("build responder: {}", e))?;

        let mut buf = vec![0u8; 65535];

        // <- e, es, s, ss
        let msg = transport.receive_message()?;
        handshake
            .read_message(&msg, &mut buf)
            .map_err(|e| format!("read handshake 1: {}", e))?;

        // -> e, ee, se, s, es
        let len = handshake
            .write_message(&[], &mut buf)
            .map_err(|e| format!("write handshake 2: {}", e))?;
        transport.send_message(&buf[..len])?;

        // <- s, se
        let msg = transport.receive_message()?;
        handshake
            .read_message(&msg, &mut buf)
            .map_err(|e| format!("read handshake 3: {}", e))?;

        let state = handshake
            .into_transport_mode()
            .map_err(|e| format!("into transport: {}", e))?;
        Ok(Self {
            state,
            buffer: vec![0u8; 65535],
        })
    }

    /// Send an encrypted payload.
    pub fn send(&mut self, transport: &mut SyncTransport, payload: &[u8]) -> Result<(), String> {
        let len = self
            .state
            .write_message(payload, &mut self.buffer)
            .map_err(|e| format!("encrypt: {}", e))?;
        transport.send_message(&self.buffer[..len])
    }

    /// Receive and decrypt a payload.
    pub fn receive(&mut self, transport: &mut SyncTransport) -> Result<Vec<u8>, String> {
        let msg = transport.receive_message()?;
        let len = self
            .state
            .read_message(&msg, &mut self.buffer)
            .map_err(|e| format!("decrypt: {}", e))?;
        Ok(self.buffer[..len].to_vec())
    }

    /// 返回远端静态公钥的短指纹（16 字节 hex），用于恢复流程验证主机身份。
    pub fn remote_fingerprint(&self) -> Option<String> {
        self.state
            .get_remote_static()
            .map(|k| hex::encode(&k[..16]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn test_keys_fingerprint() {
        let keys = NoiseKeys::generate();
        assert_eq!(keys.fingerprint().len(), 32);
    }

    #[test]
    fn test_xx_handshake() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let server_keys = NoiseKeys::generate();
        let client_keys = NoiseKeys::generate();

        let server_thread = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut transport = SyncTransport::from_stream(stream);
            let mut session =
                NoiseSession::handshake_responder(&mut transport, &server_keys).unwrap();
            session.send(&mut transport, b"hello from server").unwrap();
            let received = session.receive(&mut transport).unwrap();
            assert_eq!(received, b"hello from client");
        });

        let client_thread = thread::spawn(move || {
            let stream = std::net::TcpStream::connect(&addr).unwrap();
            let mut transport = SyncTransport::from_stream(stream);
            let mut session =
                NoiseSession::handshake_initiator(&mut transport, &client_keys).unwrap();
            session.send(&mut transport, b"hello from client").unwrap();
            let received = session.receive(&mut transport).unwrap();
            assert_eq!(received, b"hello from server");
        });

        server_thread.join().unwrap();
        client_thread.join().unwrap();
    }
}
