//! Network transport layer for peer-to-peer sync.
//!
//! Uses TCP with optional Noise encryption for data transfer
//! between discovered peers.

use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const CANCEL_POLL_INTERVAL: Duration = Duration::from_millis(100);
const MAX_WRITE_CHUNK: usize = 64 * 1024;
const MAGIC_PREFIX: &[u8] = b"SOLOSOUL_SYNC_v1";
const MAX_FRAME_SIZE: usize = 64 * 1024 * 1024; // 64 MB

/// Sync transport session
#[derive(Debug)]
pub struct SyncTransport {
    pub peer_addr: String,
    stream: Option<TcpStream>,
    cancellation: Option<Arc<AtomicBool>>,
}

impl SyncTransport {
    pub fn new(peer_addr: String) -> Self {
        Self {
            peer_addr,
            stream: None,
            cancellation: None,
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
            cancellation: None,
        }
    }

    /// 受管会话不能只依赖 shutdown 唤醒已阻塞的系统调用。
    pub(crate) fn from_cancellable_stream(
        stream: TcpStream,
        cancellation: Arc<AtomicBool>,
    ) -> Result<Self, String> {
        stream
            .set_nonblocking(false)
            .and_then(|()| stream.set_read_timeout(Some(CANCEL_POLL_INTERVAL)))
            .and_then(|()| stream.set_write_timeout(Some(CANCEL_POLL_INTERVAL)))
            .map_err(|e| format!("Configure cancellable stream failed: {e}"))?;
        Ok(Self {
            peer_addr: stream
                .peer_addr()
                .map(|a| a.to_string())
                .unwrap_or_default(),
            stream: Some(stream),
            cancellation: Some(cancellation),
        })
    }

    /// Send a message with magic prefix + length framing
    pub fn send_message(&mut self, data: &[u8]) -> Result<(), String> {
        let stream = self.stream.as_mut().ok_or("Not connected")?;
        let len = (data.len() as u32).to_be_bytes();
        let mut frame = Vec::with_capacity(MAGIC_PREFIX.len() + 4 + data.len());
        frame.extend_from_slice(MAGIC_PREFIX);
        frame.extend_from_slice(&len);
        frame.extend_from_slice(data);
        let result = if let Some(cancellation) = &self.cancellation {
            transfer_all(
                frame.len(),
                cancellation,
                io::ErrorKind::WriteZero,
                |offset| {
                    let end = (offset + MAX_WRITE_CHUNK).min(frame.len());
                    stream.write(&frame[offset..end])
                },
            )
        } else {
            stream.write_all(&frame)
        };
        result.map_err(|e| format!("Send failed: {}", e))
    }

    /// Read a framed message
    pub fn receive_message(&mut self) -> Result<Vec<u8>, String> {
        let stream = self.stream.as_mut().ok_or("Not connected")?;

        // Read magic prefix
        let mut prefix = vec![0u8; MAGIC_PREFIX.len()];
        read_exact(stream, &mut prefix, self.cancellation.as_deref())
            .map_err(|e| format!("Read prefix failed: {}", e))?;
        if prefix != MAGIC_PREFIX {
            return Err("Invalid magic prefix".into());
        }

        // Read length
        let mut len_buf = [0u8; 4];
        read_exact(stream, &mut len_buf, self.cancellation.as_deref())
            .map_err(|e| format!("Read length failed: {}", e))?;
        let len = u32::from_be_bytes(len_buf) as usize;
        if len > MAX_FRAME_SIZE {
            return Err(format!("Frame too large: {} > {}", len, MAX_FRAME_SIZE));
        }

        // Read payload
        let mut payload = vec![0u8; len];
        read_exact(stream, &mut payload, self.cancellation.as_deref())
            .map_err(|e| format!("Read payload failed: {}", e))?;
        Ok(payload)
    }

    pub fn close(&mut self) {
        self.stream = None;
        self.cancellation = None;
    }
}

fn read_exact(
    stream: &mut TcpStream,
    bytes: &mut [u8],
    cancellation: Option<&AtomicBool>,
) -> io::Result<()> {
    if let Some(cancellation) = cancellation {
        transfer_all(
            bytes.len(),
            cancellation,
            io::ErrorKind::UnexpectedEof,
            |offset| stream.read(&mut bytes[offset..]),
        )
    } else {
        stream.read_exact(bytes)
    }
}

/// 短系统等待仅用于取消轮询；每次实际进展仍重新开始原30秒等待。
fn transfer_all(
    length: usize,
    cancellation: &AtomicBool,
    empty: io::ErrorKind,
    mut transfer: impl FnMut(usize) -> io::Result<usize>,
) -> io::Result<()> {
    let mut offset = 0;
    let mut progress = Instant::now();
    loop {
        if cancellation.load(Ordering::SeqCst) {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "Sync cancelled"));
        }
        if offset == length {
            return Ok(());
        }
        let result = transfer(offset);
        if cancellation.load(Ordering::SeqCst) {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "Sync cancelled"));
        }
        match result {
            Ok(0) => return Err(io::Error::from(empty)),
            Ok(count) => {
                offset += count;
                progress = Instant::now();
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                ) && progress.elapsed() < DEFAULT_TIMEOUT => {}
            Err(error) => return Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn cancellable_receive_preserves_partial_frame_across_poll_timeouts() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut server, _) = listener.accept().unwrap();
        let writer = thread::spawn(move || {
            server.write_all(&MAGIC_PREFIX[..3]).unwrap();
            thread::sleep(Duration::from_millis(250));
            server.write_all(&MAGIC_PREFIX[3..]).unwrap();
            server.write_all(&5u32.to_be_bytes()).unwrap();
            server.write_all(b"he").unwrap();
            thread::sleep(Duration::from_millis(250));
            server.write_all(b"llo").unwrap();
        });
        let mut transport =
            SyncTransport::from_cancellable_stream(client, Arc::new(AtomicBool::new(false)))
                .unwrap();
        assert_eq!(transport.receive_message().unwrap(), b"hello");
        writer.join().unwrap();
    }

    #[tokio::test]
    async fn cancellable_receive_keeps_thirty_second_idle_limit() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (server, _) = listener.accept().unwrap();
        let mut transport =
            SyncTransport::from_cancellable_stream(client, Arc::new(AtomicBool::new(false)))
                .unwrap();
        let (done_tx, done_rx) = tokio::sync::oneshot::channel();
        let started = Instant::now();
        let worker = tokio::task::spawn_blocking(move || {
            let _ = done_tx.send(transport.receive_message());
        });
        let result = tokio::time::timeout(Duration::from_secs(40), done_rx).await;
        let elapsed = started.elapsed();
        drop(server);
        worker.await.unwrap();
        println!("RF1120 network idle limit elapsed: {elapsed:?}");
        assert!(result
            .unwrap()
            .unwrap()
            .unwrap_err()
            .contains("Read prefix failed"));
        assert!(elapsed >= Duration::from_secs(29));
        assert!(elapsed < Duration::from_secs(40));
    }

    #[tokio::test]
    async fn cancellation_interrupts_write_to_nonreading_peer() {
        let listener_socket = tokio::net::TcpSocket::new_v4().unwrap();
        listener_socket.set_recv_buffer_size(4096).unwrap();
        listener_socket
            .bind("127.0.0.1:0".parse().unwrap())
            .unwrap();
        let listener = listener_socket.listen(1).unwrap();
        let socket = tokio::net::TcpSocket::new_v4().unwrap();
        socket.set_send_buffer_size(4096).unwrap();
        let client = socket
            .connect(listener.local_addr().unwrap())
            .await
            .unwrap()
            .into_std()
            .unwrap();
        let (server, _) = listener.accept().await.unwrap();
        let cancellation = Arc::new(AtomicBool::new(false));
        let mut transport =
            SyncTransport::from_cancellable_stream(client, cancellation.clone()).unwrap();
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (done_tx, mut done_rx) = tokio::sync::oneshot::channel();
        let worker = tokio::task::spawn_blocking(move || {
            let payload = vec![0; 32 * 1024 * 1024];
            entered_tx.send(()).unwrap();
            let _ = done_tx.send(transport.send_message(&payload));
        });
        entered_rx.await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        match done_rx.try_recv() {
            Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {}
            result => {
                drop(server);
                worker.await.unwrap();
                panic!("Expected pending socket write before cancellation, got {result:?}");
            }
        }
        let started = Instant::now();
        cancellation.store(true, Ordering::SeqCst);
        let result = tokio::time::timeout(Duration::from_secs(2), &mut done_rx).await;
        let elapsed = started.elapsed();
        drop(server); // 失败路径也关闭本测试对端，等待真实 blocking worker 退出。
        worker.await.unwrap();
        println!("RF1120 network write interrupt elapsed: {elapsed:?}");
        assert!(result
            .expect("Network write did not stop within 2s")
            .unwrap()
            .unwrap_err()
            .contains("Sync cancelled"));
    }

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
                cancellation: None,
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
