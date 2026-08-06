//! 插件注册表在线更新集成测试
//!
//! 使用本地 TCP 服务器模拟远程注册表服务，并用 ring + blake2 构造 Minisign 签名。

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use blake2::{Blake2b512, Digest};
use ring::rand::SystemRandom;
use ring::signature::{Ed25519KeyPair, KeyPair};
use solo_soul::plugin::registry::PluginRegistry;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// 串行化涉及环境变量的异步测试，避免并行时互相覆盖。
static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn generate_minisign_keypair() -> ([u8; 8], Vec<u8>, Ed25519KeyPair) {
    let rng = SystemRandom::new();
    let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).expect("生成 Ed25519 密钥对失败");
    let keypair = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).expect("解析 PKCS#8 失败");
    let public_key = keypair.public_key().as_ref().to_vec();
    let key_id = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
    (key_id, public_key, keypair)
}

fn public_key_base64(key_id: [u8; 8], public_key: &[u8]) -> String {
    let mut buf = Vec::with_capacity(42);
    buf.extend_from_slice(&[0x45, 0x44]); // "ED" 表示预哈希模式
    buf.extend_from_slice(&key_id);
    buf.extend_from_slice(public_key);
    assert_eq!(buf.len(), 42);
    BASE64.encode(&buf)
}

fn sign_registry(data: &[u8], key_id: [u8; 8], keypair: &Ed25519KeyPair) -> String {
    // 1. 预哈希：BLAKE2b-512
    let mut hasher = Blake2b512::new();
    hasher.update(data);
    let hash = hasher.finalize();

    // 2. 对哈希签名
    let sig = keypair.sign(&hash);
    let sig_bytes = sig.as_ref();

    // 3. 构造 trusted comment（不带前缀部分参与 global signature）
    let trusted_comment = "timestamp:0\tfile:registry.json";

    // 4. global signature = sign(sig || trusted_comment)
    let mut global_msg = Vec::with_capacity(sig_bytes.len() + trusted_comment.len());
    global_msg.extend_from_slice(sig_bytes);
    global_msg.extend_from_slice(trusted_comment.as_bytes());
    let global_sig = keypair.sign(&global_msg);

    // 5. 编码签名块（74 字节）
    let mut sig_bin = Vec::with_capacity(74);
    sig_bin.extend_from_slice(&[0x45, 0x44]);
    sig_bin.extend_from_slice(&key_id);
    sig_bin.extend_from_slice(sig_bytes);
    assert_eq!(sig_bin.len(), 74);

    format!(
        "untrusted comment: test signature\n{}\ntrusted comment: {}\n{}",
        BASE64.encode(&sig_bin),
        trusted_comment,
        BASE64.encode(global_sig.as_ref()),
    )
}

async fn start_test_server(
    registry_json: Vec<u8>,
    signature: String,
) -> (tokio::task::JoinHandle<()>, String) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("绑定本地端口失败");
    let addr = listener.local_addr().unwrap();

    let handle = tokio::spawn(async move {
        loop {
            let (mut socket, _) = match listener.accept().await {
                Ok(conn) => conn,
                Err(_) => break,
            };
            let registry_json = registry_json.clone();
            let signature = signature.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                let n = match socket.read(&mut buf).await {
                    Ok(n) => n,
                    Err(_) => return,
                };
                let req = String::from_utf8_lossy(&buf[..n]);
                let path = req
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or("/");

                let (status, content_type, body): (&str, &str, Vec<u8>) = match path {
                    "/registry.json" => ("200 OK", "application/json", registry_json),
                    "/registry.json.minisig" => ("200 OK", "text/plain", signature.into_bytes()),
                    _ => ("404 Not Found", "text/plain", b"Not Found".to_vec()),
                };

                let response = format!(
                    "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    status, content_type, body.len()
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.write_all(&body).await;
            });
        }
    });

    (handle, format!("http://{}", addr))
}

fn registry_json() -> Vec<u8> {
    br#"{"plugins": {}}"#.to_vec()
}

#[tokio::test]
async fn test_update_registry_from_remote() {
    let _guard = ENV_LOCK.lock().await;
    let (key_id, public_key, keypair) = generate_minisign_keypair();
    let data = registry_json();
    let signature = sign_registry(&data, key_id, &keypair);

    std::env::set_var("SOLOSOUL_REGISTRY_URL", "__placeholder__");
    std::env::set_var("NO_PROXY", "127.0.0.1,localhost");
    std::env::set_var(
        "SOLOSOUL_REGISTRY_PUBKEY",
        public_key_base64(key_id, &public_key),
    );

    let (_server, url) = start_test_server(data.clone(), signature).await;
    std::env::set_var("SOLOSOUL_REGISTRY_URL", format!("{}/registry.json", url));

    let dir = tempfile::tempdir().unwrap();
    let registry_path: PathBuf = dir.path().join("registry.json");
    std::fs::write(&registry_path, br#"{"plugins": {"old": {}}}"#).unwrap();

    let registry =
        PluginRegistry::new_with_dirs(dir.path().to_path_buf(), dir.path().to_path_buf());
    registry.update_from_remote().await.expect("更新注册表失败");

    let saved = std::fs::read_to_string(&registry_path).unwrap();
    assert_eq!(saved.trim(), "{\"plugins\": {}}");
}

#[tokio::test]
async fn test_update_registry_rejects_invalid_signature() {
    let _guard = ENV_LOCK.lock().await;
    let (key_id, public_key, keypair) = generate_minisign_keypair();
    let data = registry_json();
    let signature = sign_registry(&data, key_id, &keypair);
    // 破坏 global signature 的 base64：将最后一行第一个字符替换为 'A'
    let mut lines: Vec<&str> = signature.lines().collect();
    let last = lines.last_mut().unwrap();
    if !last.is_empty() {
        *last = "A";
    }
    let corrupted = lines.join("\n");

    std::env::set_var("SOLOSOUL_REGISTRY_URL", "__placeholder__");
    std::env::set_var("NO_PROXY", "127.0.0.1,localhost");
    std::env::set_var(
        "SOLOSOUL_REGISTRY_PUBKEY",
        public_key_base64(key_id, &public_key),
    );

    let (_server, url) = start_test_server(data, corrupted).await;
    std::env::set_var("SOLOSOUL_REGISTRY_URL", format!("{}/registry.json", url));

    let dir = tempfile::tempdir().unwrap();
    let registry_path: PathBuf = dir.path().join("registry.json");
    std::fs::write(&registry_path, br#"{"plugins": {}}"#).unwrap();

    let registry =
        PluginRegistry::new_with_dirs(dir.path().to_path_buf(), dir.path().to_path_buf());
    let result = registry.update_from_remote().await;
    assert!(result.is_err(), "应当拒绝无效签名");
}

#[tokio::test]
async fn test_update_registry_rejects_mismatched_key() {
    let _guard = ENV_LOCK.lock().await;
    let (key_id, _public_key, keypair) = generate_minisign_keypair();
    let data = registry_json();
    let signature = sign_registry(&data, key_id, &keypair);

    // 使用另一把公钥
    let (_, other_pk, _) = generate_minisign_keypair();

    std::env::set_var("SOLOSOUL_REGISTRY_URL", "__placeholder__");
    std::env::set_var("NO_PROXY", "127.0.0.1,localhost");
    std::env::set_var(
        "SOLOSOUL_REGISTRY_PUBKEY",
        public_key_base64(key_id, &other_pk),
    );

    let (_server, url) = start_test_server(data, signature).await;
    std::env::set_var("SOLOSOUL_REGISTRY_URL", format!("{}/registry.json", url));

    let dir = tempfile::tempdir().unwrap();
    let registry_path: PathBuf = dir.path().join("registry.json");
    std::fs::write(&registry_path, br#"{"plugins": {}}"#).unwrap();

    let registry =
        PluginRegistry::new_with_dirs(dir.path().to_path_buf(), dir.path().to_path_buf());
    let result = registry.update_from_remote().await;
    assert!(result.is_err(), "应当拒绝密钥不匹配的签名");
}
