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
use rand::rngs::OsRng;
use rand::RngCore;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const PIN_LEN: usize = 6;
const RECOVERY_PASSWORD_LEN: usize = 32;
/// 单次加密消息的最大有效载荷。
///
/// snow 0.9.x 的 `TransportState::write_message` 强制
/// `payload.len() + TAGLEN(16) > MAXMSGLEN(65535)` 时返回 `Error::Input`，
/// 因此分块必须小于 `65535 - 16 = 65519`。此前 64KB（65536 字节）分块
/// 加上 MAC 后 65552 > 65535，超出上限 17 字节：导出包 ≥64KB 时，主机发送
/// 首个分块即报 "encrypt: input error" 并关闭连接，客户端表现为
/// "read prefix failed: failed to fill whole buffer"（单测文件只有 22 字节
/// 所以从未触发）。
const CHUNK_SIZE: usize = 32 * 1024; // 32KB（低于 snow MAXMSGLEN - TAGLEN 上限）
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
/// P015: recovery_password 用 Zeroizing 管理——恢复密码在整个主机会话期驻留内存
/// （最长 5 分钟），普通 String 会被交换分区/内存转储还原，进而解密已导出的备份包。
pub struct RecoveryHost {
    listener: TcpListener,
    pin: String,
    nonce: String,
    keys: NoiseKeys,
    recovery_password: zeroize::Zeroizing<String>,
    export_path: PathBuf,
    account_id: String,
    account_name: String,
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
    pub account_name: String,
}

impl RecoveryHost {
    /// 在指定地址上启动恢复主机，并关联一个已准备好的 `.solosoul` 导出文件。
    /// `recovery_password` 由调用方生成并用于加密导出包。
    pub fn start(
        addr: &str,
        export_path: PathBuf,
        recovery_password: zeroize::Zeroizing<String>,
        account_id: String,
        account_name: String,
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
            account_name,
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
        let mut transport = SyncTransport::from_stream(stream);
        // 先完成 Noise 握手，使被拒绝时（如全局限流）能通过加密会话发送明确的
        // `__ERROR__` 帧；若在握手前拒绝，客户端只会收到 EOF 而显示晦涩的
        // "read prefix failed: failed to fill whole buffer"。
        let mut session = NoiseSession::handshake_responder(&mut transport, &self.keys)?;

        // 全局限流检查（握手之后）：达到上限时发送错误帧而非静默关闭连接。
        if let Err(e) = check_global_rate_limit() {
            let _ = send_error(&mut session, &mut transport, &e);
            return Err(e);
        }

        // 1. 接收 "nonce:pin"（P001 之后 PIN/nonce 不再经 mDNS 广播，
        //    只可能来自 QR 扫码或用户手动输入，认证闸门仍然有效）
        let auth = receive_text(&mut session, &mut transport)?;
        let expected = format!("{}:{}", self.nonce, self.pin);
        // 兼容两种认证形态：
        // - `nonce:pin`：扫码/带 nonce 的客户端（标准路径）
        // - 裸 `pin`：手动输入模式（客户端 `recover_from_host` 在 nonce=None 时只发 PIN，
        //   见下方调用方注释"兼容旧版手动输入"）。P001 移除 mDNS 中的 nonce 后，
        //   局域网发现的主机只会填充 addr+fingerprint，用户手动输入 PIN 时即走此路径。
        if !constant_time_eq(&auth, &expected) && !constant_time_eq(&auth, &self.pin) {
            record_global_failure();
            let _ = send_error(&mut session, &mut transport, "Invalid PIN or nonce");
            return Err("Invalid PIN or nonce".to_string());
        }
        // 认证成功：重置速率限制，但**不**立即标记 served。
        // served 标记延迟到文件传输完成后才设置，确保传输中途断开时
        // 用户可以重试（否则会收到 "Recovery data has already been served" 而被拒绝）。
        reset_global_rate_limit();
        send_text(&mut session, &mut transport, "OK")?;

        // 2. 发送恢复密码（Zeroizing 自动 Deref 为 &str）
        send_text(&mut session, &mut transport, &self.recovery_password)?;

        // 3. 发送 account_id
        send_text(&mut session, &mut transport, &self.account_id)?;

        // 4. 发送 account_name
        send_text(&mut session, &mut transport, &self.account_name)?;

        // 5. 发送文件大小与内容
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
///
/// `on_progress` 可选进度回调：下载期间按已接收字节数报告 0-100 的整数百分比（至少触发一次）。
pub fn recover_from_host(
    addr: &str,
    pin: &str,
    dest_dir: &Path,
    expected_fingerprint: Option<&str>,
    nonce: Option<&str>,
    on_progress: Option<Box<dyn Fn(u8) + Send>>,
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

    // 2. 接收 OK 或错误
    let response = receive_text(&mut session, &mut transport)?;
    if response.starts_with("__ERROR__:") {
        return Err(response.trim_start_matches("__ERROR__:").to_string());
    }
    if response != "OK" {
        return Err(format!("Unexpected auth response: {}", response));
    }

    // 3. 接收恢复密码
    let recovery_password = receive_text(&mut session, &mut transport)?;

    // 4. 接收 account_id
    let account_id = receive_text(&mut session, &mut transport)?;

    // 5. 接收 account_name
    let account_name = receive_text(&mut session, &mut transport)?;

    // 6. 接收文件大小
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
        if let Some(cb) = &on_progress {
            // 下载进度按字节数换算为 0-100 百分比（file_size 上限 1GB，received * 100 不会溢出）
            let pct = ((received * 100 / file_size.max(1)) as u8).min(100);
            cb(pct);
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
        account_name,
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
    let mut rng = OsRng;
    for _ in 0..PIN_LEN {
        // P017: OsRng + 拒绝采样——2^32 不能被 10 整除（余 6），直接 % 10 会让
        // 0-5 六个数字多一次映射产生轻微偏差；只接受落在完整 10 块内的值。
        let n = loop {
            let v = rng.next_u32();
            if v < 4_294_967_290 {
                break (v % 10) as u8;
            }
        };
        pin.push((b'0' + n) as char);
    }
    pin
}

/// 常数时间字符串比较（P029：避免 PIN+nonce 校验的计时侧信道）。
///
/// 长度不相等时提前返回（长度本身非机密，且协议固定为 `nonce:pin`），
/// 相等长度时逐字节 XOR 累加，不因首个不同字节提前退出。
fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let a = a.as_bytes();
    let b = b.as_bytes();
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// 生成随机恢复密码（Base64，用于加密导出包）。
pub fn generate_recovery_password() -> String {
    use base64::Engine;
    let mut bytes = [0u8; RECOVERY_PASSWORD_LEN];
    // P017: 恢复密码改用 OsRng（操作系统 CSPRNG）直取随机字节。
    OsRng.fill_bytes(&mut bytes);
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// 尝试获取一个适合展示给用户的本地非回环 IPv4 地址。
/// 优先通过外联 UDP 获得路由选中的地址；失败或离线时枚举本地网卡。
/// P015: 本地非回环 IPv4 展示地址（跨 crate 唯一实现）。
///
/// 优先通过外联 UDP 获得路由选中的地址（`connect` 仅设置对端，不发送任何数据包，
/// 纯本地内核操作不会阻塞）；回退枚举本地网卡。src-tauri 同步命令（sync_listen_addr /
/// sync_generate_qr_payload）经 lib.rs 根 re-export 复用本实现，不再各自维护副本。
pub fn local_display_ip() -> Option<String> {
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
            zeroize::Zeroizing::new(generate_recovery_password()),
            "acc_host".to_string(),
            "Host Account".to_string(),
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
            None,
        )
        .unwrap();

        let received = std::fs::read_to_string(&result.downloaded_path).unwrap();
        assert_eq!(received, "hello recovery payload");
        assert!(!result.recovery_password.is_empty());
        assert_eq!(result.account_id, "acc_host");
        assert_eq!(result.account_name, "Host Account");
    }

    #[test]
    fn test_recovery_transfer_large_file() {
        // 回归测试：此前 CHUNK_SIZE = 64KB（65536 字节）恰好超过 snow
        // MAXMSGLEN(65535) - TAGLEN(16) 上限，导出包 ≥64KB 时主机发送首个分块
        // 即报 "encrypt: input error" 并关闭连接，客户端表现为
        // "read prefix failed: failed to fill whole buffer"（小文件单测无法触发）。
        let tmp = tempfile::tempdir().unwrap();
        let export_path = tmp.path().join("export_big.solosoul");
        let big_payload = vec![b'x'; 300 * 1024]; // 300KB > 64KB
        std::fs::write(&export_path, &big_payload).unwrap();

        let host = RecoveryHost::start(
            "127.0.0.1:0",
            export_path.clone(),
            zeroize::Zeroizing::new(generate_recovery_password()),
            "acc_host".to_string(),
            "Host Account".to_string(),
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
            None,
        )
        .unwrap();

        let received = std::fs::read(&result.downloaded_path).unwrap();
        assert_eq!(received.len(), big_payload.len());
        assert_eq!(received, big_payload);
    }

    #[test]
    fn test_recovery_transfer_progress_callback() {
        // 回归：下载进度回调应随字节数上报 0-100 百分比，至少触发一次且单调不减。
        let tmp = tempfile::tempdir().unwrap();
        let export_path = tmp.path().join("export_progress.solosoul");
        let big_payload = vec![b'p'; 200 * 1024];
        std::fs::write(&export_path, &big_payload).unwrap();

        let host = RecoveryHost::start(
            "127.0.0.1:0",
            export_path.clone(),
            zeroize::Zeroizing::new(generate_recovery_password()),
            "acc_host".to_string(),
            "Host Account".to_string(),
        )
        .unwrap();
        let info = host.connection_info();
        let addr = host.local_addr().unwrap();
        let cancel = Arc::new(AtomicBool::new(false));

        thread::spawn(move || {
            host.run(cancel).unwrap();
        });

        let dest_dir = tmp.path().join("dest_progress");
        let progress = Arc::new(Mutex::new(Vec::new()));
        let cb_progress = progress.clone();
        let result = recover_from_host(
            &addr,
            &info.pin,
            &dest_dir,
            Some(&info.fingerprint),
            Some(&info.nonce),
            Some(Box::new(move |pct: u8| {
                cb_progress.lock().unwrap().push(pct);
            })),
        )
        .unwrap();
        let received = std::fs::read(&result.downloaded_path).unwrap();
        assert_eq!(received.len(), big_payload.len());

        let events = progress.lock().unwrap();
        assert!(!events.is_empty(), "progress callback never invoked");
        assert_eq!(*events.last().unwrap(), 100, "final progress should be 100");
        assert!(
            events.windows(2).all(|w| w[0] <= w[1]),
            "progress must be monotonic non-decreasing"
        );
    }

    #[test]
    fn test_recovery_wrong_pin_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let export_path = tmp.path().join("export.solosoul");
        std::fs::write(&export_path, b"hello recovery payload").unwrap();

        let host = RecoveryHost::start(
            "127.0.0.1:0",
            export_path.clone(),
            zeroize::Zeroizing::new(generate_recovery_password()),
            "acc_host".to_string(),
            "Host Account".to_string(),
        )
        .unwrap();
        let info = host.connection_info();
        let addr = host.local_addr().unwrap();
        let cancel = Arc::new(AtomicBool::new(false));

        thread::spawn(move || {
            let _ = host.run(cancel);
        });

        let dest_dir = tmp.path().join("dest");
        let result = recover_from_host(
            &addr,
            "000000",
            &dest_dir,
            Some(&info.fingerprint),
            None,
            None,
        );
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
            zeroize::Zeroizing::new(generate_recovery_password()),
            "acc_host".to_string(),
            "Host Account".to_string(),
        )
        .unwrap();
        let cancel = Arc::new(AtomicBool::new(true));
        let result = host.run(cancel);
        assert!(result.is_err());
    }
}
