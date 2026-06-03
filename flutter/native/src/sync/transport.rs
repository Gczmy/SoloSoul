//! TCP transport for sync protocol.
//!
//! Provides a blocking TCP-based implementation of the `Transport` trait
//! for device-to-device synchronization over a local network.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

use super::protocol::Transport;

/// Length-prefix framing: 4-byte big-endian length header.
const LEN_HEADER_SIZE: usize = 4;
const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024; // 16 MiB

/// TCP-based transport using length-prefix framing.
pub struct TcpTransport {
    stream: TcpStream,
}

impl TcpTransport {
    /// Wrap an existing `TcpStream`.
    pub fn new(stream: TcpStream) -> Self {
        Self { stream }
    }

    /// Connect to a remote peer with a 5-second timeout.
    pub fn connect(addr: &str) -> Result<Self, String> {
        let socket_addr = addr.parse().map_err(|e| format!("Invalid address {}: {}", addr, e))?;
        let stream = TcpStream::connect_timeout(&socket_addr, Duration::from_secs(10))
            .map_err(|e| format!("TCP connect to {} failed: {}", addr, e))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .map_err(|e| format!("set_read_timeout: {}", e))?;
        stream
            .set_write_timeout(Some(Duration::from_secs(30)))
            .map_err(|e| format!("set_write_timeout: {}", e))?;
        Ok(Self { stream })
    }

    /// Bind a TCP listener on the given address.
    pub fn listen(addr: &str) -> Result<TcpListener, String> {
        TcpListener::bind(addr)
            .map_err(|e| format!("TCP bind {} failed: {}", addr, e))
    }

    /// Accept an incoming connection with a timeout.
    pub fn accept(listener: &TcpListener, timeout: Duration) -> Result<Self, String> {
        listener
            .set_nonblocking(true)
            .map_err(|e| format!("set_nonblocking: {}", e))?;
        let start = std::time::Instant::now();
        loop {
            match listener.accept() {
                Ok((stream, _)) => {
                    listener
                        .set_nonblocking(false)
                        .map_err(|e| format!("restore blocking: {}", e))?;
                    stream
                        .set_read_timeout(Some(Duration::from_secs(30)))
                        .map_err(|e| format!("set_read_timeout: {}", e))?;
                    stream
                        .set_write_timeout(Some(Duration::from_secs(30)))
                        .map_err(|e| format!("set_write_timeout: {}", e))?;
                    return Ok(Self::new(stream));
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if start.elapsed() >= timeout {
                        listener
                            .set_nonblocking(false)
                            .map_err(|e| format!("restore blocking: {}", e))?;
                        return Err(format!(
                            "TCP accept timed out after {}s",
                            timeout.as_secs()
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    listener
                        .set_nonblocking(false)
                        .map_err(|e| format!("restore blocking: {}", e))?;
                    return Err(format!("TCP accept failed: {}", e));
                }
            }
        }
    }

    /// Get the local address of the socket.
    pub fn local_addr(&self) -> std::net::SocketAddr {
        self.stream
            .local_addr()
            .unwrap_or_else(|_| "0.0.0.0:0".parse().unwrap())
    }
}

impl Transport for TcpTransport {
    fn send(&mut self, data: &[u8]) -> Result<(), String> {
        if data.len() > MAX_MESSAGE_SIZE {
            return Err(format!("Message too large: {} bytes", data.len()));
        }
        let len_bytes = (data.len() as u32).to_be_bytes();
        self.stream
            .write_all(&len_bytes)
            .map_err(|e| format!("send header: {}", e))?;
        self.stream
            .write_all(data)
            .map_err(|e| format!("send body: {}", e))?;
        self.stream
            .flush()
            .map_err(|e| format!("send flush: {}", e))?;
        Ok(())
    }

    fn recv(&mut self) -> Result<Vec<u8>, String> {
        let mut header = [0u8; LEN_HEADER_SIZE];
        self.stream
            .read_exact(&mut header)
            .map_err(|e| format!("recv header: {}", e))?;
        let len = u32::from_be_bytes(header) as usize;
        if len > MAX_MESSAGE_SIZE {
            return Err(format!("Message too large: {} bytes", len));
        }
        let mut buf = vec![0u8; len];
        self.stream
            .read_exact(&mut buf)
            .map_err(|e| format!("recv body: {}", e))?;
        Ok(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn test_tcp_transport_roundtrip() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut transport = TcpTransport::new(stream);
            let msg = transport.recv().unwrap();
            transport.send(&msg).unwrap(); // echo back
        });

        let mut client = TcpTransport::connect(&addr.to_string()).unwrap();
        let payload = b"hello over TCP";
        client.send(payload).unwrap();
        let echoed = client.recv().unwrap();
        assert_eq!(echoed, payload);

        handle.join().unwrap();
    }

    #[test]
    fn test_tcp_transport_multiple_messages() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut transport = TcpTransport::new(stream);
            for _ in 0..5 {
                let msg = transport.recv().unwrap();
                transport.send(&msg).unwrap();
            }
        });

        let mut client = TcpTransport::connect(&addr.to_string()).unwrap();
        for i in 0..5 {
            let payload = format!("message {}", i);
            client.send(payload.as_bytes()).unwrap();
            let echoed = client.recv().unwrap();
            assert_eq!(echoed, payload.as_bytes());
        }

        handle.join().unwrap();
    }

    #[test]
    fn test_tcp_transport_empty_message() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut transport = TcpTransport::new(stream);
            let msg = transport.recv().unwrap();
            transport.send(&msg).unwrap();
        });

        let mut client = TcpTransport::connect(&addr.to_string()).unwrap();
        client.send(b"").unwrap();
        let echoed = client.recv().unwrap();
        assert!(echoed.is_empty());

        handle.join().unwrap();
    }
}
