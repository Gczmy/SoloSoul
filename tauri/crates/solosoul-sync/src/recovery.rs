//! 跨设备账户恢复（Recovery Transfer）。
//!
//! 主机端生成一个 6 位 PIN、32 字节随机恢复密码以及一次性 nonce；
//! 使用临时 Noise_XX 身份密钥对建立加密通道。
//! 新设备扫描/输入主机地址、端口、PIN 与 nonce 后连接到主机，
//! 先校验主机公钥指纹（防 MITM），再校验 PIN+nonce，最后下载加密导出包。
//! 恢复密码本身通过加密通道发送，QR 码中只包含地址、PIN、nonce 与指纹。
//!
//! 本模块只负责「安全传输临时导出文件」，不直接调用 Vault 的导出/导入逻辑。

use crate::noise::{NoiseKeys, NoiseSession};
use crate::transport::SyncTransport;
use rand::RngCore;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const PIN_LEN: usize = 6;
const RECOVERY_PASSWORD_LEN: usize = 32;
const CHUNK_SIZE: usize = 64 * 1024; // 64KB
const MAX_FILE_SIZE: u64 = 1024 * 1024 * 1024; // 1GB
const SESSION_TIMEOUT: Duration = Duration::from_secs(300); // 5 min
const ACCEPT_POLL_MS: u64 = 100;
/// 会话维度最大失败尝试次数（达到后该会话拒绝继续尝试）。
const SESSION_MAX_ATTEMPTS: u32 = 5;
/// 全局维度最大失败尝试次数。
const GLOBAL_MAX_ATTEMPTS: u32 = 10;
/// 全局失败计数窗口。
const GLOBAL_WINDOW: Duration = Duration::from_secs(60);

/// 主机端状态。
pub struct RecoveryHost {
    listener: TcpListener,
    pin: String,
    nonce: String,
    keys: NoiseKeys,
    recovery_password: String,
    export_path: PathBuf,
    account_id: String,
    started_at: Instant,
    /// 标记是否已成功服务过一次恢复请求，防止同一 PIN 被多个客户端同时下载。
    served: AtomicBool,
}

/// 恢复会话结果（客户端）。
#[derive(Debug, Clone)]
pub struct RecoveryResult {
    pub downloaded_path: PathBuf,
    pub recovery_password: String,
    pub account_id: String,
}

/// 反向恢复模式下，接收端最终获得的数据。
#[derive(Debug, Clone)]
pub struct RecoveryTransferResult {
    pub downloaded_path: PathBuf,
    pub recovery_password: String,
    pub account_id: String,
}

impl RecoveryHost {
    /// 在指定地址上启动恢复主机，并关联一个已准备好的 `.solosoul` 导出文件。
    /// `recovery_password` 由调用方生成并用于加密导出包。
    pub fn start(
        addr: &str,
        export_path: PathBuf,
        recovery_password: String,
        account_id: String,
    ) -> Result<Self, String> {
        let listener = TcpListener::bind(addr).map_err(|e| format!("bind failed: {}", e))?;
        let pin = generate_pin();
        let nonce = nanoid();
        let keys = NoiseKeys::generate();
        Ok(Self {
            listener,
            pin,
            nonce,
            keys,
            recovery_password,
            export_path,
            account_id,
            started_at: Instant::now(),
            served: AtomicBool::new(false),
        })
    }

    /// 返回供用户输入/扫码的连接信息。
    pub fn connection_info(&self) -> RecoveryConnectionInfo {
        let port = self.listener.local_addr().map(|a| a.port()).unwrap_or(0);
        RecoveryConnectionInfo {
            display_addr: format!(
                "{}:{}",
                local_display_ip().unwrap_or_else(|| "127.0.0.1".to_string()),
                port
            ),
            bind_addr: self
                .listener
                .local_addr()
                .map(|a| a.to_string())
                .unwrap_or_default(),
            pin: self.pin.clone(),
            nonce: self.nonce.clone(),
            fingerprint: self.keys.fingerprint(),
        }
    }

    /// 运行恢复会话循环：接受多个连接，完成 Noise 握手与 PIN+nonce 校验，
    /// 直到成功传输、超时或取消。
    pub fn run(self, cancel: Arc<AtomicBool>) -> Result<(), String> {
        self.listener
            .set_nonblocking(true)
            .map_err(|e| format!("set nonblocking: {}", e))?;
        let mut session_attempts = 0u32;
        loop {
            if cancel.load(Ordering::SeqCst) {
                return Err("Recovery session cancelled".to_string());
            }
            if self.started_at.elapsed() > SESSION_TIMEOUT {
                return Err("Recovery session expired".to_string());
            }
            if session_attempts >= SESSION_MAX_ATTEMPTS {
                return Err("Too many failed recovery attempts".to_string());
            }

            match self.listener.accept() {
                Ok((stream, peer_addr)) => {
                    tracing::info!("Recovery connection from {}", peer_addr);
                    match self.handle_connection(stream) {
                        Ok(()) => return Ok(()),
                        Err(e) => {
                            tracing::warn!("Recovery connection from {} failed: {}", peer_addr, e);
                            session_attempts += 1;
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(ACCEPT_POLL_MS));
                }
                Err(e) => return Err(format!("accept failed: {}", e)),
            }
        }
    }

    fn mark_served(&self) -> Result<(), String> {
        self.served
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map(|_| ())
            .map_err(|_| "Recovery data has already been served".to_string())
    }

    fn handle_connection(&self, stream: TcpStream) -> Result<(), String> {
        check_global_rate_limit()?;

        let mut transport = SyncTransport::from_stream(stream);
        let mut session = NoiseSession::handshake_responder(&mut transport, &self.keys)?;

        // 1. 接收 "nonce:pin"
        let auth = receive_text(&mut session, &mut transport)?;
        let expected = format!("{}:{}", self.nonce, self.pin);
        if auth != expected {
            record_global_failure();
            let _ = send_error(&mut session, &mut transport, "Invalid PIN or nonce");
            return Err("Invalid PIN or nonce".to_string());
        }
        // 认证成功：重置速率限制，但**不**立即标记 served。
        // served 标记延迟到文件传输完成后才设置，确保传输中途断开时
        // 用户可以重试（否则会收到 "Recovery data has already been served" 而被拒绝）。
        reset_global_rate_limit();
        send_text(&mut session, &mut transport, "OK")?;

        // 2. 发送恢复密码
        send_text(&mut session, &mut transport, &self.recovery_password)?;

        // 3. 发送 account_id
        send_text(&mut session, &mut transport, &self.account_id)?;

        // 4. 发送文件大小与内容
        let file_size = std::fs::metadata(&self.export_path)
            .map_err(|e| format!("metadata: {}", e))?
            .len();
        if file_size > MAX_FILE_SIZE {
            return Err(format!("Export file too large: {}", file_size));
        }
        send_text(&mut session, &mut transport, &file_size.to_string())?;

        let mut file =
            std::fs::File::open(&self.export_path).map_err(|e| format!("open export: {}", e))?;
        let mut buf = [0u8; CHUNK_SIZE];
        loop {
            let n = file.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            send_binary(&mut session, &mut transport, &buf[..n])?;
        }
        // 发送空块作为 EOF 标记
        send_binary(&mut session, &mut transport, &[])?;

        transport.close();

        // 文件传输完成后才标记为已服务。若传输中途失败（网络断开等），
        // served 仍为 false，用户可以重新连接并重试恢复。
        self.mark_served()?;
        tracing::info!("Recovery transfer completed");
        Ok(())
    }

    pub fn pin(&self) -> &str {
        &self.pin
    }

    pub fn local_addr(&self) -> Result<String, String> {
        self.listener
            .local_addr()
            .map(|a| a.to_string())
            .map_err(|e| e.to_string())
    }
}

/// 新设备端：连接到主机，完成恢复流程，返回下载的文件路径、恢复密码与 account_id。
pub fn recover_from_host(
    addr: &str,
    pin: &str,
    dest_dir: &Path,
    expected_fingerprint: Option<&str>,
    nonce: Option<&str>,
) -> Result<RecoveryResult, String> {
    let stream = TcpStream::connect_timeout(
        &addr.parse().map_err(|e| format!("Invalid addr: {}", e))?,
        Duration::from_secs(10),
    )
    .map_err(|e| format!("connect failed: {}", e))?;

    let mut transport = SyncTransport::from_stream(stream);
    let keys = NoiseKeys::generate();
    let mut session = NoiseSession::handshake_initiator(&mut transport, &keys)?;

    // 验证主机身份指纹
    if let Some(expected_fp) = expected_fingerprint {
        let actual_fp = session
            .remote_fingerprint()
            .ok_or("Host did not provide a static public key")?;
        if actual_fp != expected_fp {
            return Err(format!(
                "Host identity verification failed: expected {}, got {}. Possible MITM.",
                expected_fp, actual_fp
            ));
        }
    }

    // 1. 发送 "nonce:pin"（若未提供 nonce 则只发送 pin，兼容旧版手动输入）
    let auth = match nonce {
        Some(n) => format!("{}:{}", n, pin),
        None => pin.to_string(),
    };
    send_text(&mut session, &mut transport, &auth)?;

    // 2. 接收恢复密码（或错误）
    let response = receive_text(&mut session, &mut transport)?;
    if response.starts_with("__ERROR__:") {
        return Err(response.trim_start_matches("__ERROR__:").to_string());
    }
    let recovery_password = response;

    // 3. 接收 account_id
    let account_id = receive_text(&mut session, &mut transport)?;

    // 4. 接收文件大小
    let file_size_str = receive_text(&mut session, &mut transport)?;
    let file_size: u64 = file_size_str
        .parse()
        .map_err(|_| format!("Invalid file size: {}", file_size_str))?;
    if file_size > MAX_FILE_SIZE {
        return Err(format!("Export file too large: {}", file_size));
    }

    // 5. 接收文件内容
    std::fs::create_dir_all(dest_dir).map_err(|e| e.to_string())?;
    let dest_path = dest_dir.join(format!("recovery_{}.solosoul", nanoid()));
    let mut file =
        std::fs::File::create(&dest_path).map_err(|e| format!("create dest file: {}", e))?;

    let mut received: u64 = 0;
    loop {
        let chunk = receive_binary(&mut session, &mut transport)?;
        if chunk.is_empty() {
            break; // EOF
        }
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        received += chunk.len() as u64;
        if received > file_size {
            return Err("Received more data than expected".to_string());
        }
    }

    if received != file_size {
        return Err(format!(
            "Incomplete transfer: {}/{} bytes",
            received, file_size
        ));
    }

    transport.close();
    Ok(RecoveryResult {
        downloaded_path: dest_path,
        recovery_password,
        account_id,
    })
}

/// 连接信息，用于展示 QR 或提示用户手动输入。
#[derive(Debug, Clone)]
pub struct RecoveryConnectionInfo {
    /// 面向用户展示的地址（优先本地非回环 IP）。
    pub display_addr: String,
    /// 实际绑定地址（如 0.0.0.0:port）。
    pub bind_addr: String,
    pub pin: String,
    pub nonce: String,
    pub fingerprint: String,
}

fn generate_pin() -> String {
    let mut pin = String::with_capacity(PIN_LEN);
    let mut rng = rand::thread_rng();
    for _ in 0..PIN_LEN {
        let n = (rng.next_u32() % 10) as u8;
        pin.push((b'0' + n) as char);
    }
    pin
}

/// 生成随机恢复密码（Base64，用于加密导出包）。
pub fn generate_recovery_password() -> String {
    use base64::Engine;
    let mut bytes = [0u8; RECOVERY_PASSWORD_LEN];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// 尝试获取一个适合展示给用户的本地非回环 IPv4 地址。
/// 优先通过外联 UDP 获得路由选中的地址；失败或离线时枚举本地网卡。
fn local_display_ip() -> Option<String> {
    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(local) = socket.local_addr() {
                if let std::net::IpAddr::V4(v4) = local.ip() {
                    if !v4.is_loopback() {
                        return Some(v4.to_string());
                    }
                }
            }
        }
    }
    if let Ok(std::net::IpAddr::V4(v4)) = local_ip_address::local_ip() {
        if !v4.is_loopback() {
            return Some(v4.to_string());
        }
    }
    None
}

fn nanoid() -> String {
    uuid::Uuid::new_v4().to_string().replace("-", "")
}

fn send_text(
    session: &mut NoiseSession,
    transport: &mut SyncTransport,
    text: &str,
) -> Result<(), String> {
    send_binary(session, transport, text.as_bytes())
}

fn receive_text(
    session: &mut NoiseSession,
    transport: &mut SyncTransport,
) -> Result<String, String> {
    let bytes = receive_binary(session, transport)?;
    String::from_utf8(bytes).map_err(|e| format!("Invalid UTF-8: {}", e))
}

fn send_binary(
    session: &mut NoiseSession,
    transport: &mut SyncTransport,
    data: &[u8],
) -> Result<(), String> {
    session.send(transport, data)
}

fn receive_binary(
    session: &mut NoiseSession,
    transport: &mut SyncTransport,
) -> Result<Vec<u8>, String> {
    session.receive(transport)
}

fn send_error(
    session: &mut NoiseSession,
    transport: &mut SyncTransport,
    message: &str,
) -> Result<(), String> {
    send_text(session, transport, &format!("__ERROR__:{}", message))
}

/// 反向恢复模式：接收端服务器。
/// 与 `RecoveryHost` 不同，它不持有导出文件，而是等待主机连接并推送数据。
pub struct RecoveryReceiverServer {
    listener: TcpListener,
    pin: String,
    nonce: String,
    keys: NoiseKeys,
    started_at: Instant,
}

impl RecoveryReceiverServer {
    /// 在指定地址上启动反向恢复接收服务器。
    pub fn start(addr: &str) -> Result<Self, String> {
        let listener = TcpListener::bind(addr).map_err(|e| format!("bind failed: {}", e))?;
        let pin = generate_pin();
        let nonce = nanoid();
        let keys = NoiseKeys::generate();
        Ok(Self {
            listener,
            pin,
            nonce,
            keys,
            started_at: Instant::now(),
        })
    }

    /// 返回供用户输入/扫码的连接信息。
    pub fn connection_info(&self) -> RecoveryConnectionInfo {
        let port = self.listener.local_addr().map(|a| a.port()).unwrap_or(0);
        RecoveryConnectionInfo {
            display_addr: format!(
                "{}:{}",
                local_display_ip().unwrap_or_else(|| "127.0.0.1".to_string()),
                port
            ),
            bind_addr: self
                .listener
                .local_addr()
                .map(|a| a.to_string())
                .unwrap_or_default(),
            pin: self.pin.clone(),
            nonce: self.nonce.clone(),
            fingerprint: self.keys.fingerprint(),
        }
    }

    /// 运行反向恢复接收循环，直到成功接收一次传输、超时或被取消。
    pub fn run(self, cancel: Arc<AtomicBool>) -> Result<RecoveryTransferResult, String> {
        self.listener
            .set_nonblocking(true)
            .map_err(|e| format!("set nonblocking: {}", e))?;
        let mut session_attempts = 0u32;
        loop {
            if cancel.load(Ordering::SeqCst) {
                return Err("Recovery session cancelled".to_string());
            }
            if self.started_at.elapsed() > SESSION_TIMEOUT {
                return Err("Recovery session expired".to_string());
            }
            if session_attempts >= SESSION_MAX_ATTEMPTS {
                return Err("Too many failed recovery attempts".to_string());
            }

            match self.listener.accept() {
                Ok((stream, peer_addr)) => {
                    tracing::info!("Reverse recovery connection from {}", peer_addr);
                    match self.handle_connection(stream) {
                        Ok(result) => return Ok(result),
                        Err(e) => {
                            tracing::warn!("Reverse recovery connection from {} failed: {}", peer_addr, e);
                            session_attempts += 1;
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(ACCEPT_POLL_MS));
                }
                Err(e) => return Err(format!("accept failed: {}", e)),
            }
        }
    }

    fn handle_connection(&self, stream: TcpStream) -> Result<RecoveryTransferResult, String> {
        check_global_rate_limit()?;

        let mut transport = SyncTransport::from_stream(stream);
        let mut session = NoiseSession::handshake_responder(&mut transport, &self.keys)?;

        // 1. 接收 "nonce:pin"
        let auth = receive_text(&mut session, &mut transport)?;
        let expected = format!("{}:{}", self.nonce, self.pin);
        if auth != expected {
            record_global_failure();
            let _ = send_error(&mut session, &mut transport, "Invalid PIN or nonce");
            return Err("Invalid PIN or nonce".to_string());
        }
        reset_global_rate_limit();

        // 2. 接收恢复密码
        let recovery_password = receive_text(&mut session, &mut transport)?;
        if recovery_password.starts_with("__ERROR__:") {
            return Err(recovery_password.trim_start_matches("__ERROR__:").to_string());
        }

        // 3. 接收 account_id
        let account_id = receive_text(&mut session, &mut transport)?;

        // 4. 接收文件大小
        let file_size_str = receive_text(&mut session, &mut transport)?;
        let file_size: u64 = file_size_str
            .parse()
            .map_err(|_| format!("Invalid file size: {}", file_size_str))?;
        if file_size > MAX_FILE_SIZE {
            return Err(format!("Export file too large: {}", file_size));
        }

        // 5. 接收文件内容
        std::fs::create_dir_all(dest_dir()).map_err(|e| e.to_string())?;
        let dest_path = dest_dir().join(format!("reverse_recovery_{}.solosoul", nanoid()));
        let mut file = std::fs::File::create(&dest_path)
            .map_err(|e| format!("create dest file: {}", e))?;

        let mut received: u64 = 0;
        loop {
            let chunk = receive_binary(&mut session, &mut transport)?;
            if chunk.is_empty() {
                break; // EOF
            }
            file.write_all(&chunk).map_err(|e| e.to_string())?;
            received += chunk.len() as u64;
            if received > file_size {
                return Err("Received more data than expected".to_string());
            }
        }

        if received != file_size {
            return Err(format!(
                "Incomplete transfer: {}/{} bytes",
                received, file_size
            ));
        }

        transport.close();

        tracing::info!("Reverse recovery transfer completed");
        Ok(RecoveryTransferResult {
            downloaded_path: dest_path,
            recovery_password,
            account_id,
        })
    }
}

fn dest_dir() -> &'static std::path::Path {
    static DEST_DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    DEST_DIR.get_or_init(|| std::env::temp_dir().join("solosoul_recovery_downloads"))
}

/// 反向恢复模式：主机端主动连接接收端并推送导出包。
/// 当前账户必须先解锁，调用方负责生成导出包与恢复密码。
pub fn push_to_receiver(
    addr: &str,
    pin: &str,
    expected_fingerprint: Option<&str>,
    nonce: Option<&str>,
    export_path: &std::path::Path,
    recovery_password: String,
    account_id: String,
) -> Result<(), String> {
    let stream = TcpStream::connect_timeout(
        &addr.parse().map_err(|e| format!("Invalid addr: {}", e))?,
        Duration::from_secs(10),
    )
    .map_err(|e| format!("connect failed: {}", e))?;

    let mut transport = SyncTransport::from_stream(stream);
    let keys = NoiseKeys::generate();
    let mut session = NoiseSession::handshake_initiator(&mut transport, &keys)?;

    // 验证接收端身份指纹
    if let Some(expected_fp) = expected_fingerprint {
        let actual_fp = session
            .remote_fingerprint()
            .ok_or("Receiver did not provide a static public key")?;
        if actual_fp != expected_fp {
            return Err(format!(
                "Receiver identity verification failed: expected {}, got {}. Possible MITM.",
                expected_fp, actual_fp
            ));
        }
    }

    // 1. 发送 "nonce:pin"
    let auth = match nonce {
        Some(n) => format!("{}:{}", n, pin),
        None => pin.to_string(),
    };
    send_text(&mut session, &mut transport, &auth)?;

    // 2. 读取认证结果
    let auth_response = receive_text(&mut session, &mut transport)?;
    if auth_response.starts_with("__ERROR__:") {
        return Err(auth_response.trim_start_matches("__ERROR__:").to_string());
    }
    if auth_response != "OK" {
        return Err(format!("Unexpected auth response: {}", auth_response));
    }

    // 3. 发送恢复密码
    send_text(&mut session, &mut transport, &recovery_password)?;

    // 3. 发送 account_id
    send_text(&mut session, &mut transport, &account_id)?;

    // 4. 发送文件大小和内容
    let file_size = std::fs::metadata(export_path)
        .map_err(|e| format!("metadata: {}", e))?
        .len();
    if file_size > MAX_FILE_SIZE {
        return Err(format!("Export file too large: {}", file_size));
    }
    send_text(&mut session, &mut transport, &file_size.to_string())?;

    let mut file = std::fs::File::open(export_path).map_err(|e| format!("open export: {}", e))?;
    let mut buf = [0u8; CHUNK_SIZE];
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        send_binary(&mut session, &mut transport, &buf[..n])?;
    }
    // 发送空块作为 EOF 标记
    send_binary(&mut session, &mut transport, &[])?;

    transport.close();

    tracing::info!("Reverse recovery push completed");
    Ok(())
}

// ── 全局速率限制 ───────────────────────────────────────────────────────

struct RateLimiter {
    failures: u32,
    last_failure: Option<Instant>,
}

static GLOBAL_RATE_LIMITER: Mutex<RateLimiter> = Mutex::new(RateLimiter {
    failures: 0,
    last_failure: None,
});

fn check_global_rate_limit() -> Result<(), String> {
    let mut rl = GLOBAL_RATE_LIMITER.lock().map_err(|e| e.to_string())?;
    if let Some(last) = rl.last_failure {
        if last.elapsed() > GLOBAL_WINDOW {
            rl.failures = 0;
            rl.last_failure = None;
        }
    }
    if rl.failures >= GLOBAL_MAX_ATTEMPTS {
        return Err("Too many failed recovery attempts. Please try again later.".to_string());
    }
    Ok(())
}

fn record_global_failure() {
    if let Ok(mut rl) = GLOBAL_RATE_LIMITER.lock() {
        if let Some(last) = rl.last_failure {
            if last.elapsed() > GLOBAL_WINDOW {
                rl.failures = 0;
            }
        }
        rl.failures += 1;
        rl.last_failure = Some(Instant::now());
    }
}

fn reset_global_rate_limit() {
    if let Ok(mut rl) = GLOBAL_RATE_LIMITER.lock() {
        rl.failures = 0;
        rl.last_failure = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_generate_pin_length() {
        let pin = generate_pin();
        assert_eq!(pin.len(), PIN_LEN);
        assert!(pin.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_recovery_transfer() {
        let tmp = tempfile::tempdir().unwrap();
        let export_path = tmp.path().join("export.solosoul");
        std::fs::write(&export_path, b"hello recovery payload").unwrap();

        let host = RecoveryHost::start(
            "127.0.0.1:0",
            export_path.clone(),
            generate_recovery_password(),
            "acc_host".to_string(),
        )
        .unwrap();
        let info = host.connection_info();
        let addr = host.local_addr().unwrap();
        let cancel = Arc::new(AtomicBool::new(false));

        thread::spawn(move || {
            host.run(cancel).unwrap();
        });

        let dest_dir = tmp.path().join("dest");
        let result = recover_from_host(
            &addr,
            &info.pin,
            &dest_dir,
            Some(&info.fingerprint),
            Some(&info.nonce),
        )
        .unwrap();

        let received = std::fs::read_to_string(&result.downloaded_path).unwrap();
        assert_eq!(received, "hello recovery payload");
        assert!(!result.recovery_password.is_empty());
        assert_eq!(result.account_id, "acc_host");
    }

    #[test]
    fn test_recovery_wrong_pin_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let export_path = tmp.path().join("export.solosoul");
        std::fs::write(&export_path, b"hello recovery payload").unwrap();

        let host = RecoveryHost::start(
            "127.0.0.1:0",
            export_path.clone(),
            generate_recovery_password(),
            "acc_host".to_string(),
        )
        .unwrap();
        let info = host.connection_info();
        let addr = host.local_addr().unwrap();
        let cancel = Arc::new(AtomicBool::new(false));

        thread::spawn(move || {
            let _ = host.run(cancel);
        });

        let dest_dir = tmp.path().join("dest");
        let result = recover_from_host(&addr, "000000", &dest_dir, Some(&info.fingerprint), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_recovery_cancel() {
        let tmp = tempfile::tempdir().unwrap();
        let export_path = tmp.path().join("export.solosoul");
        std::fs::write(&export_path, b"hello").unwrap();

        let host = RecoveryHost::start(
            "127.0.0.1:0",
            export_path.clone(),
            generate_recovery_password(),
            "acc_host".to_string(),
        )
        .unwrap();
        let cancel = Arc::new(AtomicBool::new(true));
        let result = host.run(cancel);
        assert!(result.is_err());
    }
}
