//! Network transport layer for peer-to-peer sync.
//!
//! Uses TCP with optional Noise encryption for data transfer
//! between discovered peers.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const MAGIC_PREFIX: &[u8] = b"SOLOSOUL_SYNC_v1";

/// Sync transport session
#[derive(Debug)]
pub struct SyncTransport {
    pub peer_addr: String,
    stream: Option<TcpStream>,
}

impl SyncTransport {
    pub fn new(peer_addr: String) -> Self {
        Self { peer_addr, stream: None }
    }

    /// Connect to a peer
    pub fn connect(&mut self) -> Result<(), String> {
        let stream = TcpStream::connect_timeout(
            &self.peer_addr.parse().map_err(|e| format!("Invalid addr: {}", e))?,
            DEFAULT_TIMEOUT,
        ).map_err(|e| format!("Connect failed: {}", e))?;
        stream.set_read_timeout(Some(DEFAULT_TIMEOUT)).ok();
        stream.set_write_timeout(Some(DEFAULT_TIMEOUT)).ok();
        self.stream = Some(stream);
        Ok(())
    }

    /// Send a message with magic prefix + length framing
    pub fn send_message(&mut self, data: &[u8]) -> Result<(), String> {
        let stream = self.stream.as_mut().ok_or("Not connected")?;
        let len = (data.len() as u32).to_be_bytes();
        let mut frame = Vec::with_capacity(MAGIC_PREFIX.len() + 4 + data.len());
        frame.extend_from_slice(MAGIC_PREFIX);
        frame.extend_from_slice(&len);
        frame.extend_from_slice(data);
        stream.write_all(&frame).map_err(|e| format!("Send failed: {}", e))
    }

    /// Read a framed message
    pub fn receive_message(&mut self) -> Result<Vec<u8>, String> {
        let stream = self.stream.as_mut().ok_or("Not connected")?;

        // Read magic prefix
        let mut prefix = vec![0u8; MAGIC_PREFIX.len()];
        stream.read_exact(&mut prefix).map_err(|e| format!("Read prefix failed: {}", e))?;
        if prefix != MAGIC_PREFIX {
            return Err("Invalid magic prefix".into());
        }

        // Read length
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).map_err(|e| format!("Read length failed: {}", e))?;
        let len = u32::from_be_bytes(len_buf) as usize;

        // Read payload
        let mut payload = vec![0u8; len];
        stream.read_exact(&mut payload).map_err(|e| format!("Read payload failed: {}", e))?;
        Ok(payload)
    }

    pub fn close(&mut self) {
        self.stream = None;
    }
}

/// Listen for incoming sync connections
pub struct SyncListener {
    listener: TcpListener,
}

impl SyncListener {
    pub fn bind(addr: &str) -> Result<Self, String> {
        let listener = TcpListener::bind(addr)
            .map_err(|e| format!("Bind failed: {}", e))?;
        listener.set_nonblocking(true).ok();
        Ok(Self { listener })
    }

    /// Accept a pending connection (non-blocking)
    pub fn accept(&self) -> Result<Option<SyncTransport>, String> {
        match self.listener.accept() {
            Ok((stream, addr)) => {
                stream.set_read_timeout(Some(DEFAULT_TIMEOUT)).ok();
                stream.set_write_timeout(Some(DEFAULT_TIMEOUT)).ok();
                Ok(Some(SyncTransport {
                    peer_addr: addr.to_string(),
                    stream: Some(stream),
                }))
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(format!("Accept failed: {}", e)),
        }
    }

    pub fn local_addr(&self) -> Result<String, String> {
        self.listener.local_addr()
            .map(|a| a.to_string())
            .map_err(|e| e.to_string())
    }
}
