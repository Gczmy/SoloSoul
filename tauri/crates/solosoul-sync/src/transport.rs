//! Network transport layer for peer-to-peer sync.
//!
//! Uses TCP with optional Noise encryption for data transfer
//! between discovered peers.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const MAGIC_PREFIX: &[u8] = b"SOLOSOUL_SYNC_v1";
const MAX_FRAME_SIZE: usize = 64 * 1024 * 1024; // 64 MB

/// Sync transport session
#[derive(Debug)]
pub struct SyncTransport {
    pub peer_addr: String,
    stream: Option<TcpStream>,
}

impl SyncTransport {
    pub fn new(peer_addr: String) -> Self {
        Self {
            peer_addr,
            stream: None,
        }
    }

    /// Connect to a peer
    pub fn connect(&mut self) -> Result<(), String> {
        let stream = TcpStream::connect_timeout(
            &self
                .peer_addr
                .parse()
                .map_err(|e| format!("Invalid addr: {}", e))?,
            DEFAULT_TIMEOUT,
        )
        .map_err(|e| format!("Connect failed: {}", e))?;
        stream.set_read_timeout(Some(DEFAULT_TIMEOUT)).ok();
        stream.set_write_timeout(Some(DEFAULT_TIMEOUT)).ok();
        self.stream = Some(stream);
        Ok(())
    }

    /// Wrap an existing connected stream.
    pub fn from_stream(stream: TcpStream) -> Self {
        let peer_addr = stream
            .peer_addr()
            .map(|a| a.to_string())
            .unwrap_or_default();
        // Ensure blocking mode for synchronous read/write.
        stream.set_nonblocking(false).ok();
        stream.set_read_timeout(Some(DEFAULT_TIMEOUT)).ok();
        stream.set_write_timeout(Some(DEFAULT_TIMEOUT)).ok();
        Self {
            peer_addr,
            stream: Some(stream),
        }
    }

    /// Send a message with magic prefix + length framing
    pub fn send_message(&mut self, data: &[u8]) -> Result<(), String> {
        let stream = self.stream.as_mut().ok_or("Not connected")?;
        let len = (data.len() as u32).to_be_bytes();
        let mut frame = Vec::with_capacity(MAGIC_PREFIX.len() + 4 + data.len());
        frame.extend_from_slice(MAGIC_PREFIX);
        frame.extend_from_slice(&len);
        frame.extend_from_slice(data);
        stream
            .write_all(&frame)
            .map_err(|e| format!("Send failed: {}", e))
    }

    /// Read a framed message
    pub fn receive_message(&mut self) -> Result<Vec<u8>, String> {
        let stream = self.stream.as_mut().ok_or("Not connected")?;

        // Read magic prefix
        let mut prefix = vec![0u8; MAGIC_PREFIX.len()];
        stream
            .read_exact(&mut prefix)
            .map_err(|e| format!("Read prefix failed: {}", e))?;
        if prefix != MAGIC_PREFIX {
            return Err("Invalid magic prefix".into());
        }

        // Read length
        let mut len_buf = [0u8; 4];
        stream
            .read_exact(&mut len_buf)
            .map_err(|e| format!("Read length failed: {}", e))?;
        let len = u32::from_be_bytes(len_buf) as usize;
        if len > MAX_FRAME_SIZE {
            return Err(format!("Frame too large: {} > {}", len, MAX_FRAME_SIZE));
        }

        // Read payload
        let mut payload = vec![0u8; len];
        stream
            .read_exact(&mut payload)
            .map_err(|e| format!("Read payload failed: {}", e))?;
        Ok(payload)
    }

    pub fn close(&mut self) {
        self.stream = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_sync_transport_new() {
        let transport = SyncTransport::new("127.0.0.1:9999".to_string());
        assert_eq!(transport.peer_addr, "127.0.0.1:9999");
        assert!(transport.stream.is_none());
    }

    #[test]
    fn test_send_receive_message() {
        // Use blocking std listener to avoid nonblocking accept issues in tests
        let std_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = std_listener.local_addr().unwrap().to_string();

        let server_addr = addr.clone();
        let server = thread::spawn(move || {
            let (stream, _) = std_listener.accept().unwrap();
            SyncTransport {
                peer_addr: server_addr,
                stream: Some(stream),
            }
        });

        let mut client = SyncTransport::new(addr);
        client.connect().unwrap();

        let mut server_transport = server.join().unwrap();

        // Client sends
        let payload = b"hello world";
        client.send_message(payload).unwrap();

        // Server receives
        let received = server_transport.receive_message().unwrap();
        assert_eq!(received, payload.to_vec());

        // Server echoes back
        server_transport.send_message(b"echo").unwrap();
        let echo = client.receive_message().unwrap();
        assert_eq!(echo, b"echo".to_vec());
    }

    #[test]
    fn test_send_without_connection_fails() {
        let mut transport = SyncTransport::new("127.0.0.1:9999".to_string());
        let result = transport.send_message(b"test");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Not connected"));
    }

    #[test]
    fn test_receive_without_connection_fails() {
        let mut transport = SyncTransport::new("127.0.0.1:9999".to_string());
        let result = transport.receive_message();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Not connected"));
    }

    #[test]
    fn test_connect_invalid_address_fails() {
        let mut transport = SyncTransport::new("not_an_address".to_string());
        let result = transport.connect();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid addr"));
    }

    #[test]
    fn test_close_drops_stream() {
        let std_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = std_listener.local_addr().unwrap().to_string();

        let server = thread::spawn(move || {
            let _ = std_listener.accept();
        });

        let mut client = SyncTransport::new(addr);
        client.connect().unwrap();
        assert!(client.stream.is_some());

        client.close();
        assert!(client.stream.is_none());

        server.join().unwrap();
    }

    #[test]
    fn test_message_framing() {
        // Verify that a message is framed as: MAGIC_PREFIX + 4-byte length + payload
        let mut transport = SyncTransport::new("127.0.0.1:0".to_string());
        let std_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = std_listener.local_addr().unwrap().to_string();

        let server = thread::spawn(move || {
            let (mut stream, _) = std_listener.accept().unwrap();
            let mut prefix = vec![0u8; MAGIC_PREFIX.len()];
            stream.read_exact(&mut prefix).unwrap();
            assert_eq!(prefix, MAGIC_PREFIX);

            let mut len_buf = [0u8; 4];
            stream.read_exact(&mut len_buf).unwrap();
            let len = u32::from_be_bytes(len_buf) as usize;
            assert_eq!(len, 5);

            let mut payload = vec![0u8; len];
            stream.read_exact(&mut payload).unwrap();
            assert_eq!(payload, b"hello".to_vec());
        });

        transport.peer_addr = addr;
        transport.connect().unwrap();
        transport.send_message(b"hello").unwrap();

        server.join().unwrap();
    }
}
