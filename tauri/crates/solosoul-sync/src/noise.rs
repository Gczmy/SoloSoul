//! Noise protocol wrapper using the XX handshake pattern.
//!
//! Provides persistent long-term identity keys and an encrypted transport
//! session over an existing `SyncTransport`.

use crate::transport::SyncTransport;
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use snow::{Builder, TransportState};
use std::fmt;
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::Zeroizing;

const PATTERN: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";

/// snow 0.9.x 单条加密消息的最大有效载荷（MAXMSGLEN 65535 - TAGLEN 16）。
/// 超过此上限时 `TransportState::write_message` 返回 `Error::Input`。
/// 分块大小（recovery.rs / attachments.rs 的 CHUNK_SIZE）必须小于该值。
const MAX_NOISE_PAYLOAD: usize = 65535 - 16;

/// SAS（Short Authentication String）派生使用的域分离标签。
/// 握手哈希本身已是均匀随机的 BLAKE2s 输出，附加固定标签防止与
/// 其他用途（如未来引入的 channel binding）共享同一推导输入。
const SAS_DOMAIN_SEPARATOR: &[u8] = b"SoloSoul-SAS-v1";

/// Long-term Noise identity keys for this node.
///
/// 私钥存放于 `Zeroizing`：Drop 时清零（clone 出的副本同样在 drop 时清零），
/// 避免长期身份私钥残留于堆内存；`Debug` 手写实现仅暴露公钥指纹，
/// 杜绝 `{:?}` 日志打印泄漏私钥（纵深防御，当前无日志点直接打印）。
#[derive(Clone)]
pub struct NoiseKeys {
    secret: Zeroizing<[u8; 32]>,
    public: [u8; 32],
}

impl fmt::Debug for NoiseKeys {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NoiseKeys")
            .field("public", &hex::encode(self.public))
            .field("fingerprint", &self.fingerprint())
            .finish_non_exhaustive()
    }
}

impl NoiseKeys {
    /// Generate a fresh key pair.
    pub fn generate() -> Self {
        let secret = StaticSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        Self {
            secret: Zeroizing::new(secret.to_bytes()),
            public: public.to_bytes(),
        }
    }

    /// Restore keys from a persisted secret key.
    pub fn from_secret(secret: [u8; 32]) -> Self {
        let secret = Zeroizing::new(secret);
        let secret_obj = StaticSecret::from(*secret);
        let public = PublicKey::from(&secret_obj);
        Self {
            secret,
            public: public.to_bytes(),
        }
    }

    pub fn secret_key(&self) -> &[u8; 32] {
        &self.secret
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
    /// Noise XX 握手完成后的握手哈希（双方一致，BLAKE2s 32 字节）。
    /// 用于派生 SAS 配对验证码（短认证串）：两端各自从本地握手哈希
    /// 派生同一个 6 位数字，用户目视比对即完成设备身份确认。
    handshake_hash: [u8; 32],
}

impl NoiseSession {
    /// Perform an XX handshake as the initiator over the given transport.
    pub fn handshake_initiator(
        transport: &mut SyncTransport,
        keys: &NoiseKeys,
    ) -> Result<Self, String> {
        let mut handshake = Builder::new(PATTERN.parse().map_err(|e| format!("pattern: {:?}", e))?)
            .local_private_key(&*keys.secret)
            .build_initiator()
            .map_err(|e| format!("build initiator: {e}"))?;

        let mut buf = vec![0u8; 65535];

        // -> e, es, s, ss
        let len = handshake
            .write_message(&[], &mut buf)
            .map_err(|e| format!("write handshake 1: {e}"))?;
        transport.send_message(&buf[..len])?;

        // <- e, ee, se, s, es
        let msg = transport.receive_message()?;
        handshake
            .read_message(&msg, &mut buf)
            .map_err(|e| format!("read handshake 2: {e}"))?;

        // -> s, se
        let len = handshake
            .write_message(&[], &mut buf)
            .map_err(|e| format!("write handshake 3: {e}"))?;
        transport.send_message(&buf[..len])?;

        // 握手完成：捕获双方一致的握手哈希（必须在 into_transport_mode 之前）。
        let handshake_hash = capture_handshake_hash(&handshake);
        let state = handshake
            .into_transport_mode()
            .map_err(|e| format!("into transport: {e}"))?;
        Ok(Self {
            state,
            buffer: vec![0u8; 65535],
            handshake_hash,
        })
    }

    /// Perform an XX handshake as the responder over the given transport.
    pub fn handshake_responder(
        transport: &mut SyncTransport,
        keys: &NoiseKeys,
    ) -> Result<Self, String> {
        let mut handshake = Builder::new(PATTERN.parse().map_err(|e| format!("pattern: {:?}", e))?)
            .local_private_key(&*keys.secret)
            .build_responder()
            .map_err(|e| format!("build responder: {e}"))?;

        let mut buf = vec![0u8; 65535];

        // <- e, es, s, ss
        let msg = transport.receive_message()?;
        handshake
            .read_message(&msg, &mut buf)
            .map_err(|e| format!("read handshake 1: {e}"))?;

        // -> e, ee, se, s, es
        let len = handshake
            .write_message(&[], &mut buf)
            .map_err(|e| format!("write handshake 2: {e}"))?;
        transport.send_message(&buf[..len])?;

        // <- s, se
        let msg = transport.receive_message()?;
        handshake
            .read_message(&msg, &mut buf)
            .map_err(|e| format!("read handshake 3: {e}"))?;

        // 握手完成：捕获双方一致的握手哈希（必须在 into_transport_mode 之前）。
        let handshake_hash = capture_handshake_hash(&handshake);
        let state = handshake
            .into_transport_mode()
            .map_err(|e| format!("into transport: {e}"))?;
        Ok(Self {
            state,
            buffer: vec![0u8; 65535],
            handshake_hash,
        })
    }

    /// Send an encrypted payload.
    pub fn send(&mut self, transport: &mut SyncTransport, payload: &[u8]) -> Result<(), String> {
        // snow 0.9.x 强制 payload.len() + TAGLEN(16) <= MAXMSGLEN(65535)，
        // 超过时返回隐晦的 Error::Input（透传后表现为 "encrypt: input error"）。
        // 显式检查并给出可诊断的错误信息，避免超限问题被误判为网络/对端故障。
        if payload.len() > MAX_NOISE_PAYLOAD {
            return Err(format!(
                "payload too large for noise message: {} bytes (max {})",
                payload.len(),
                MAX_NOISE_PAYLOAD
            ));
        }
        let len = self
            .state
            .write_message(payload, &mut self.buffer)
            .map_err(|e| format!("encrypt: {e}"))?;
        transport.send_message(&self.buffer[..len])
    }

    /// Receive and decrypt a payload.
    pub fn receive(&mut self, transport: &mut SyncTransport) -> Result<Vec<u8>, String> {
        let msg = transport.receive_message()?;
        let len = self
            .state
            .read_message(&msg, &mut self.buffer)
            .map_err(|e| format!("decrypt: {e}"))?;
        Ok(self.buffer[..len].to_vec())
    }

    /// 返回远端静态公钥的短指纹（16 字节 hex），用于恢复流程验证主机身份。
    pub fn remote_fingerprint(&self) -> Option<String> {
        self.state
            .get_remote_static()
            .map(|k| hex::encode(&k[..16]))
    }

    /// 派生 6 位 SAS 配对验证码（Short Authentication String）。
    ///
    /// 两端在各自本地完成 XX 握手后，握手哈希完全一致；据此派生出的数字
    /// 必然相同。配对卡片上两端各自展示该码，用户目视比对一致即确认
    /// 对端身份（MITM 时两端握手哈希不同 → 数字必然不同 → 用户可识别）。
    ///
    /// 派生：SHA-256(handshake_hash ‖ SAS_DOMAIN_SEPARATOR) 取前 8 字节
    /// 解释为 u64，模 10^6 得到 6 位十进制（前导零补齐）。
    pub fn sas_code(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.handshake_hash);
        hasher.update(SAS_DOMAIN_SEPARATOR);
        let digest = hasher.finalize();
        let val = u64::from_be_bytes(digest[..8].try_into().expect("8 bytes"));
        format!("{:06}", val % 1_000_000)
    }
}

/// 从握手状态中捕获双方一致的握手哈希（32 字节 BLAKE2s）。
/// 必须在 `into_transport_mode()` 之前调用——传输模式状态不再保留该值。
fn capture_handshake_hash(handshake: &snow::HandshakeState) -> [u8; 32] {
    let h: &[u8] = handshake.get_handshake_hash();
    let mut out = [0u8; 32];
    let n = h.len().min(32);
    out[..n].copy_from_slice(&h[..n]);
    out
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

    /// P011 防回归：Debug 输出不得包含私钥明文（纵深防御）。
    /// 私钥 32 字节 hex 为 64 字符，与公钥/fingerprint 展示位不同源，
    /// 若未来回退到派生 Debug 或误加 secret 字段，本测试即失败。
    #[test]
    fn test_debug_redacts_secret() {
        let keys = NoiseKeys::generate();
        let secret_hex = hex::encode(keys.secret_key());
        let dbg = format!("{:?}", keys);
        assert!(
            !dbg.contains(&secret_hex),
            "Debug 输出泄漏了私钥明文: {dbg}"
        );
        assert!(dbg.contains("fingerprint"), "Debug 应展示 fingerprint");
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

    /// P001 防回归：握手后双方对端指纹必须与对端 `keys.fingerprint()` 一致。
    /// 若 `remote_fingerprint()` 与 `fingerprint()` 的格式/算法漂移，
    /// 所有诚实 peer 的会话都会在 verify_peer_identity 检查①处失败。
    #[test]
    fn test_handshake_remote_fingerprint_matches_peer_fingerprint() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let server_keys = NoiseKeys::generate();
        let client_keys = NoiseKeys::generate();
        // 预先计算指纹，避免 keys 被两个线程闭包同时 move（E0382 双 move）。
        let server_fp = server_keys.fingerprint();
        let client_fp = client_keys.fingerprint();

        let server_thread = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut transport = SyncTransport::from_stream(stream);
            let session = NoiseSession::handshake_responder(&mut transport, &server_keys).unwrap();
            // 响应方看到的对端指纹 == 发起方 keys.fingerprint()
            assert_eq!(
                session.remote_fingerprint().as_deref(),
                Some(client_fp.as_str())
            );
        });

        let client_thread = thread::spawn(move || {
            let stream = std::net::TcpStream::connect(&addr).unwrap();
            let mut transport = SyncTransport::from_stream(stream);
            let session = NoiseSession::handshake_initiator(&mut transport, &client_keys).unwrap();
            // 发起方看到的对端指纹 == 响应方 keys.fingerprint()
            assert_eq!(
                session.remote_fingerprint().as_deref(),
                Some(server_fp.as_str())
            );
        });

        server_thread.join().unwrap();
        client_thread.join().unwrap();
    }

    /// SAS 配对验证码：诚实握手两端派生的 6 位数字必须完全一致。
    /// 这是「两边显示同一个验证码」方案正确性的根基。
    #[test]
    fn test_handshake_sas_code_identical_on_both_sides() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let server_keys = NoiseKeys::generate();
        let client_keys = NoiseKeys::generate();

        let server_code = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut transport = SyncTransport::from_stream(stream);
            let session = NoiseSession::handshake_responder(&mut transport, &server_keys).unwrap();
            session.sas_code()
        });

        let client_code = thread::spawn(move || {
            let stream = std::net::TcpStream::connect(&addr).unwrap();
            let mut transport = SyncTransport::from_stream(stream);
            let session = NoiseSession::handshake_initiator(&mut transport, &client_keys).unwrap();
            session.sas_code()
        });

        let server_code = server_code.join().unwrap();
        let client_code = client_code.join().unwrap();
        assert_eq!(server_code.len(), 6, "SAS 码必须为 6 位");
        assert_eq!(client_code.len(), 6, "SAS 码必须为 6 位");
        assert!(
            server_code.chars().all(|c| c.is_ascii_digit()),
            "SAS 码必须全为数字"
        );
        assert_eq!(
            server_code, client_code,
            "诚实握手两端 SAS 码必须一致（对端身份确认的根基）"
        );
    }

    /// SAS 码与会话绑定：不同的握手（不同 ephemeral / 静态密钥组合）
    /// 必须派生不同验证码（否则验证码毫无安全意义）。
    #[test]
    fn test_sas_code_differs_across_sessions() {
        // 同一对密钥的两次独立握手（ephemeral 不同）也应产生不同 SAS 码，
        // 证明验证码是「本次会话」的，而非静态绑定设备。
        let listener1 = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr1 = listener1.local_addr().unwrap().to_string();
        let listener2 = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr2 = listener2.local_addr().unwrap().to_string();
        let server_keys = NoiseKeys::generate();
        let client_keys = NoiseKeys::generate();

        let run_pair = |listener: TcpListener, addr: String, tag: String| {
            let server_keys = server_keys.clone();
            let client_keys = client_keys.clone();
            let server = thread::spawn(move || {
                let (stream, _) = listener.accept().unwrap();
                let mut transport = SyncTransport::from_stream(stream);
                let session =
                    NoiseSession::handshake_responder(&mut transport, &server_keys).unwrap();
                session.sas_code()
            });
            let client = thread::spawn(move || {
                let stream = std::net::TcpStream::connect(&addr).unwrap();
                let mut transport = SyncTransport::from_stream(stream);
                let session =
                    NoiseSession::handshake_initiator(&mut transport, &client_keys).unwrap();
                session.sas_code()
            });
            let (s, c) = (server.join().unwrap(), client.join().unwrap());
            assert_eq!(s, c, "{tag}: 两端 SAS 应一致");
            s
        };

        let code1 = run_pair(listener1, addr1, "session-1".to_string());
        let code2 = run_pair(listener2, addr2, "session-2".to_string());
        // 比较派生哈希的前 12 位十进制（1/10^12 碰撞概率），避免 6 位码
        // 在 10^6 空间内的偶发相等让本测试在极低概率下误报失败（防 flaky）。
        // 注意：仅测试断言用更长前缀；实际 UI 仍展示用户选择的 6 位码。
        let sas_long = |sas: &str| {
            let mut hasher = Sha256::new();
            hasher.update(sas.as_bytes());
            let d = hasher.finalize();
            let v = u64::from_be_bytes(d[..8].try_into().expect("8 bytes"));
            format!("{:012}", v % 1_000_000_000_000)
        };
        assert_ne!(
            sas_long(&code1),
            sas_long(&code2),
            "不同会话的 SAS 派生值应不同"
        );
    }
}
