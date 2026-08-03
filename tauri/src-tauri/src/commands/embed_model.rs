//! Embedding model management — download, install, list local models.

use serde::{Deserialize, Serialize};

use sha2::Digest;
use std::path::{Path, PathBuf};
use tauri::{path::BaseDirectory, AppHandle, Emitter, Manager};

// ── Registry ─────────────────────────────────────────────────

const REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/Gczmy/SoloSoul/main/tauri/src-tauri/resources/models/registry.json";

/// P207/N-10: 编译期固化的 Embedding 注册表 minisign 公钥（base64：2 字节算法前缀 + 8 字节
/// key_id + 32 字节 Ed25519 公钥）。
///
/// 由 `cargo tauri signer generate` 生成的**独立专用密钥对**（2026-08-03，与 Tauri updater
/// 密钥 `~/.tauri/secret.key` 隔离，避免单点信任域扩张）；对应私钥交由维护者离线保管，
/// 发布流程用 `cargo tauri signer sign`（`-p ''`，ED 预哈希签名，与 `verify(.., false)`
/// 路径兼容）对 `registry.json` 签名产出 `registry.json.minisig`，随仓库一并提交。
///
/// 公钥来源优先级：`SOLOSOUL_EMBED_REGISTRY_PUBKEY` 环境变量 > 此编译期常量。
/// 已配置公钥后校验失败即硬失败，绝不含糊。
const EMBED_REGISTRY_PUBKEY_B64: Option<&str> =
    Some("RWTemXPdgTgjPGuPgRxV+e3ng0NH2lgS8HzRbmi0XSlyjYXKI6zGkvXD");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedModelInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub disk_size: String,
    pub dimensions: u32,
    pub download_url: String,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedRegistry {
    pub models: Vec<EmbedModelInfo>,
}

/// Fetch the remote model registry.
///
/// P207: 注册表 minisign 签名校验（与插件注册表 `SOLOSOUL_REGISTRY_PUBKEY` 同模式）——
/// 注册表 JSON 与其中 `download_url`/`checksum` 同通道下发，若无独立签名则仓库被攻破时
/// 攻击者可直接替换注册表并伪造匹配的 checksum。公钥来源优先级：
/// `SOLOSOUL_EMBED_REGISTRY_PUBKEY` 环境变量 > 编译期常量 `EMBED_REGISTRY_PUBKEY_B64`。
/// 未配置时告警并继续（插件注册表先例）；配置后校验失败即硬失败，绝不含糊。
pub async fn fetch_registry() -> Result<EmbedRegistry, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Client: {}", e))?;

    let resp = client
        .get(REGISTRY_URL)
        .send()
        .await
        .map_err(|e| format!("Fetch registry: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Registry HTTP {}", resp.status()));
    }
    let registry_bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Read registry: {}", e))?;

    let pubkey_b64 = std::env::var("SOLOSOUL_EMBED_REGISTRY_PUBKEY")
        .ok()
        .or_else(|| EMBED_REGISTRY_PUBKEY_B64.map(str::to_string));
    if let Some(key_b64) = pubkey_b64 {
        let sig_url = format!("{}.minisig", REGISTRY_URL);
        let sig_text = client
            .get(&sig_url)
            .send()
            .await
            .map_err(|e| format!("Fetch registry signature: {}", e))?
            .text()
            .await
            .map_err(|e| format!("Read registry signature: {}", e))?;
        verify_registry_signature(&registry_bytes, &key_b64, &sig_text)?;
    } else {
        tracing::warn!("SOLOSOUL_EMBED_REGISTRY_PUBKEY 未配置，Embedding 注册表不校验签名");
    }

    let registry: EmbedRegistry =
        serde_json::from_slice(&registry_bytes).map_err(|e| format!("Parse registry: {}", e))?;

    Ok(registry)
}

/// P207: 用 minisign 公钥校验注册表字节与签名文本（解耦自网络层，便于单测）。
/// `pubkey_b64` 为 minisign 公钥 base64（含 ED 前缀与 key_id），`sig_text` 为 `.minisig` 文件内容。
/// 仅模块内使用（fetch_registry + 同模块单测），保持私有避免 P222 类可见性过度。
fn verify_registry_signature(
    registry_bytes: &[u8],
    pubkey_b64: &str,
    sig_text: &str,
) -> Result<(), String> {
    let public_key = minisign_verify::PublicKey::from_base64(pubkey_b64)
        .map_err(|e| format!("Registry public key parse failed: {}", e))?;
    let signature = minisign_verify::Signature::decode(sig_text)
        .map_err(|e| format!("Registry signature decode failed: {}", e))?;
    public_key
        .verify(registry_bytes, &signature, false)
        .map_err(|e| format!("Registry signature verification failed: {}", e))
}

// ── Local model storage ──────────────────────────────────────

/// Get the base directory where models are stored.
/// - 桌面端：LocalData/models
/// - 移动端：Data/models（应用私有目录可写）
pub fn models_base_dir(app: &AppHandle) -> Result<PathBuf, String> {
    #[cfg(desktop)]
    {
        let dir = app
            .path()
            .resolve("models", BaseDirectory::LocalData)
            .map_err(|e| format!("Cannot resolve app_local_data_dir: {}", e))?;
        Ok(dir)
    }
    #[cfg(mobile)]
    {
        let dir = app
            .path()
            .resolve("models", BaseDirectory::Data)
            .map_err(|e| format!("Cannot resolve app_data_dir: {}", e))?;
        Ok(dir)
    }
}

/// R013: model IDs are used as directory names, so restrict them to safe
/// characters to prevent path traversal.
fn sanitize_model_id(model_id: &str) -> Result<String, String> {
    if model_id.is_empty() {
        return Err("Model ID cannot be empty".to_string());
    }
    if model_id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        Ok(model_id.to_string())
    } else {
        Err("Model ID contains invalid characters".to_string())
    }
}

/// Check if a model is installed.
pub fn is_model_installed(app: &AppHandle, model_id: &str) -> Result<bool, String> {
    let id = sanitize_model_id(model_id)?;
    let dir = models_base_dir(app)?.join(&id);
    Ok(dir.join("model.onnx").exists() && dir.join("tokenizer.json").exists())
}

/// List installed models by scanning the local directory.
pub fn list_installed_models(app: &AppHandle) -> Result<Vec<String>, String> {
    let base = models_base_dir(app)?;
    if !base.exists() {
        return Ok(vec![]);
    }

    let mut models = Vec::new();
    for entry in std::fs::read_dir(&base).map_err(|e| format!("Read dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Dir entry: {}", e))?;
        let path = entry.path();
        if path.is_dir() {
            let id = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            if !id.is_empty() && path.join("model.onnx").exists() {
                models.push(id);
            }
        }
    }
    Ok(models)
}

/// Delete an installed model.
pub fn delete_model(app: &AppHandle, model_id: &str) -> Result<(), String> {
    let id = sanitize_model_id(model_id)?;
    let dir = models_base_dir(app)?.join(&id);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| format!("Remove model dir: {}", e))?;
    }
    Ok(())
}

// ── Download ─────────────────────────────────────────────────

/// Download a model zip, verify checksum, and extract.
pub async fn download_model(app: &AppHandle, model: &EmbedModelInfo) -> Result<(), String> {
    let base_dir = models_base_dir(app)?;
    std::fs::create_dir_all(&base_dir).map_err(|e| format!("Create models dir: {}", e))?;

    let id = sanitize_model_id(&model.id)?;
    let model_dir = base_dir.join(&id);
    let zip_path = base_dir.join(format!("{}.zip", id));

    // Clean up any previous incomplete download
    let _ = std::fs::remove_file(&zip_path);
    let _ = std::fs::remove_dir_all(&model_dir);

    // Download
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| format!("Client: {}", e))?;

    let mut resp = client
        .get(&model.download_url)
        .send()
        .await
        .map_err(|e| format!("Download request: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Download HTTP {}", resp.status()));
    }

    let total = resp.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut file =
        std::fs::File::create(&zip_path).map_err(|e| format!("Create zip file: {}", e))?;

    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| format!("Download chunk: {}", e))?
    {
        use std::io::Write;
        file.write_all(&chunk)
            .map_err(|e| format!("Write chunk: {}", e))?;
        downloaded += chunk.len() as u64;

        // Emit progress event
        if total > 0 {
            let pct = (downloaded as f64 / total as f64 * 100.0) as u32;
            let _ = app.emit(
                "embed-download-progress",
                serde_json::json!({
                    "modelId": model.id,
                    "progress": pct,
                    "downloaded": downloaded,
                    "total": total
                }),
            );
        }
    } // Verify checksum (SHA256) — 流式读取，避免大文件 OOM
    let hash = {
        use std::io::Read;
        let mut file = std::fs::File::open(&zip_path).map_err(|e| format!("Open zip: {}", e))?;
        let mut hasher = sha2::Sha256::new();
        let mut buf = [0u8; 8192];
        loop {
            let n = file
                .read(&mut buf)
                .map_err(|e| format!("Read zip chunk: {}", e))?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        format!("{:x}", hasher.finalize())
    };
    let expected = model
        .checksum
        .strip_prefix("sha256:")
        .unwrap_or(&model.checksum);
    if hash != expected {
        let _ = std::fs::remove_file(&zip_path);
        return Err(format!(
            "Checksum mismatch: expected {}, got {}",
            expected, hash
        ));
    }

    // Extract
    std::fs::create_dir_all(&model_dir).map_err(|e| format!("Create model dir: {}", e))?;
    extract_zip(&zip_path, &model_dir)?;

    // Clean up zip
    let _ = std::fs::remove_file(&zip_path);

    // Emit completion event
    let _ = app.emit(
        "embed-download-complete",
        serde_json::json!({
            "modelId": model.id,
            "success": true
        }),
    );

    Ok(())
}

fn extract_zip(zip_path: &PathBuf, dest: &Path) -> Result<(), String> {
    let file = std::fs::File::open(zip_path).map_err(|e| format!("Open zip: {}", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Read zip: {}", e))?;
    let dest_canon = dest
        .canonicalize()
        .map_err(|e| format!("Canonicalize dest: {}", e))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Zip entry: {}", e))?;
        let outpath = dest_canon.join(file.mangled_name());
        if outpath
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
            || !outpath.starts_with(&dest_canon)
        {
            return Err(format!("ZIP entry escapes destination: {}", file.name()));
        }

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath).map_err(|e| format!("Create dir: {}", e))?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p).map_err(|e| format!("Create parent dir: {}", e))?;
                }
            }
            let mut outfile =
                std::fs::File::create(&outpath).map_err(|e| format!("Create file: {}", e))?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| format!("Extract file: {}", e))?;
        }
    }

    Ok(())
}

// ── Tauri Commands ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct EmbedModelWithStatus {
    #[serde(flatten)]
    pub info: EmbedModelInfo,
    pub installed: bool,
}

/// Fetch the remote registry and mark which models are locally installed.
#[tauri::command]
pub async fn llm_get_embed_models(app: AppHandle) -> Result<Vec<EmbedModelWithStatus>, String> {
    let registry = fetch_registry().await?;
    let installed = list_installed_models(&app)?;
    let mut result = Vec::with_capacity(registry.models.len());
    for m in registry.models {
        let installed_flag = installed.contains(&m.id);
        result.push(EmbedModelWithStatus {
            info: m,
            installed: installed_flag,
        });
    }
    Ok(result)
}

/// Download and install an embedding model by ID.
#[tauri::command]
pub async fn llm_download_embed_model(app: AppHandle, model_id: String) -> Result<(), String> {
    let registry = fetch_registry().await?;
    let model = registry
        .models
        .into_iter()
        .find(|m| m.id == model_id)
        .ok_or_else(|| format!("Model {} not found in registry", model_id))?;
    download_model(&app, &model).await
}

/// Delete an installed embedding model.
#[tauri::command]
pub fn llm_delete_embed_model(app: AppHandle, model_id: String) -> Result<(), String> {
    delete_model(&app, &model_id)
}

#[cfg(all(test, desktop))]
mod tests {
    use super::*;

    #[test]
    fn test_embed_model_info_serde() {
        let info = EmbedModelInfo {
            id: "all-MiniLM-L6-v2".to_string(),
            name: "MiniLM".to_string(),
            description: "Lightweight embedding model".to_string(),
            disk_size: "80MB".to_string(),
            dimensions: 384,
            download_url: "https://example.com/model.zip".to_string(),
            checksum: "sha256:abc123".to_string(),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"all-MiniLM-L6-v2\""));
        assert!(json.contains("\"download_url\""));
        let restored: EmbedModelInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, info.id);
        assert_eq!(restored.dimensions, 384);
    }

    #[test]
    fn test_embed_registry_serde() {
        let registry = EmbedRegistry {
            models: vec![EmbedModelInfo {
                id: "m1".to_string(),
                name: "M1".to_string(),
                description: String::new(),
                disk_size: "10MB".to_string(),
                dimensions: 128,
                download_url: String::new(),
                checksum: String::new(),
            }],
        };
        let json = serde_json::to_string(&registry).unwrap();
        let restored: EmbedRegistry = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.models.len(), 1);
        assert_eq!(restored.models[0].id, "m1");
    }

    #[test]
    fn test_embed_model_with_status_serde() {
        let status = EmbedModelWithStatus {
            info: EmbedModelInfo {
                id: "m1".to_string(),
                name: "M1".to_string(),
                description: String::new(),
                disk_size: String::new(),
                dimensions: 0,
                download_url: String::new(),
                checksum: String::new(),
            },
            installed: true,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"installed\":true"));
        // flatten embeds EmbedModelInfo fields at top level
        assert!(json.contains("\"id\":\"m1\""));
    }

    #[test]
    fn test_sanitize_model_id_accepts_valid() {
        assert_eq!(
            sanitize_model_id("all-MiniLM-L6-v2").unwrap(),
            "all-MiniLM-L6-v2"
        );
        assert_eq!(sanitize_model_id("model_v1.0").unwrap(), "model_v1.0");
        assert_eq!(sanitize_model_id("a").unwrap(), "a");
    }

    #[test]
    fn test_sanitize_model_id_rejects_empty() {
        assert!(sanitize_model_id("").is_err());
        assert_eq!(
            sanitize_model_id("").unwrap_err(),
            "Model ID cannot be empty"
        );
    }

    #[test]
    fn test_sanitize_model_id_rejects_special_chars() {
        assert!(sanitize_model_id("../models").is_err());
        assert!(sanitize_model_id("model@2").is_err());
        assert!(sanitize_model_id("space model").is_err());
        assert!(sanitize_model_id("a/b").is_err());
    }

    #[test]
    fn test_sanitize_model_id_rejects_path_traversal() {
        assert!(sanitize_model_id("../../etc/passwd").is_err());
        assert!(sanitize_model_id("~/models").is_err());
    }

    #[test]
    fn test_is_model_installed_checks_files() {
        let dir = tempfile::TempDir::new().unwrap();
        let model_dir = dir.path().join("test-model");
        std::fs::create_dir_all(&model_dir).unwrap();

        // Without files, not installed
        assert!(!is_model_installed_inner(&model_dir));

        // With only one file, not installed
        std::fs::write(model_dir.join("model.onnx"), "data").unwrap();
        assert!(!is_model_installed_inner(&model_dir));

        // With both files, installed
        std::fs::write(model_dir.join("tokenizer.json"), "{}").unwrap();
        assert!(is_model_installed_inner(&model_dir));
    }

    fn is_model_installed_inner(dir: &std::path::Path) -> bool {
        dir.join("model.onnx").exists() && dir.join("tokenizer.json").exists()
    }

    #[test]
    fn test_list_installed_models_scans_directory() {
        let dir = tempfile::TempDir::new().unwrap();

        // Empty dir
        let models = list_installed_models_inner(dir.path());
        assert!(models.is_empty());

        // Create a valid model
        let m1 = dir.path().join("model-a");
        std::fs::create_dir_all(&m1).unwrap();
        std::fs::write(m1.join("model.onnx"), "x").unwrap();
        std::fs::write(m1.join("tokenizer.json"), "{}").unwrap();

        // Create dir without model.onnx (incomplete install)
        let m2 = dir.path().join("model-b");
        std::fs::create_dir_all(&m2).unwrap();
        std::fs::write(m2.join("tokenizer.json"), "{}").unwrap();

        let models = list_installed_models_inner(dir.path());
        assert_eq!(models, vec!["model-a"]);
    }

    fn list_installed_models_inner(base: &std::path::Path) -> Vec<String> {
        if !base.exists() {
            return vec![];
        }
        let mut models = Vec::new();
        if let Ok(dir) = std::fs::read_dir(base) {
            for entry in dir.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let id = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_string();
                    if !id.is_empty() && path.join("model.onnx").exists() {
                        models.push(id);
                    }
                }
            }
        }
        models
    }

    #[test]
    fn test_delete_removes_directory() {
        let dir = tempfile::TempDir::new().unwrap();
        let model_dir = dir.path().join("to-delete");
        std::fs::create_dir_all(&model_dir).unwrap();
        std::fs::write(model_dir.join("model.onnx"), "x").unwrap();

        assert!(model_dir.exists());
        delete_model_inner(&model_dir).unwrap();
        assert!(!model_dir.exists());
    }

    #[test]
    fn test_delete_nonexistent_is_ok() {
        let dir = tempfile::TempDir::new().unwrap();
        let nonexistent = dir.path().join("does-not-exist");
        assert!(!nonexistent.exists());
        delete_model_inner(&nonexistent).unwrap();
    }

    fn delete_model_inner(dir: &std::path::Path) -> Result<(), String> {
        if dir.exists() {
            std::fs::remove_dir_all(dir).map_err(|e| format!("Remove: {}", e))?;
        }
        Ok(())
    }

    // ── P207 注册表签名校验（纯函数单测，无网络） ──────────────
    // 用 ring + blake2 构造 Minisign 预哈希签名，与插件注册表测试同模式。
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;
    use blake2::{Blake2b512, Digest};
    use ring::rand::SystemRandom;
    use ring::signature::{Ed25519KeyPair, KeyPair};

    fn gen_minisign_keypair() -> ([u8; 8], Vec<u8>, Ed25519KeyPair) {
        let rng = SystemRandom::new();
        let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).expect("生成 Ed25519 密钥对失败");
        let keypair = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).expect("解析 PKCS#8 失败");
        let public_key = keypair.public_key().as_ref().to_vec();
        let key_id = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        (key_id, public_key, keypair)
    }

    fn pubkey_b64(key_id: [u8; 8], public_key: &[u8]) -> String {
        let mut buf = Vec::with_capacity(42);
        buf.extend_from_slice(&[0x45, 0x44]); // "ED" 预哈希模式
        buf.extend_from_slice(&key_id);
        buf.extend_from_slice(public_key);
        assert_eq!(buf.len(), 42);
        BASE64.encode(&buf)
    }

    fn sign_data(data: &[u8], key_id: [u8; 8], keypair: &Ed25519KeyPair) -> String {
        let mut hasher = Blake2b512::new();
        hasher.update(data);
        let hash = hasher.finalize();
        let sig = keypair.sign(&hash);
        let sig_bytes = sig.as_ref();

        let trusted_comment = "timestamp:0\tfile:registry.json";
        let mut global_msg = Vec::with_capacity(sig_bytes.len() + trusted_comment.len());
        global_msg.extend_from_slice(sig_bytes);
        global_msg.extend_from_slice(trusted_comment.as_bytes());
        let global_sig = keypair.sign(&global_msg);

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

    fn sample_registry_bytes() -> Vec<u8> {
        br#"{"models":[{"id":"all-MiniLM-L6-v2","download_url":"https://example.com/m.zip","checksum":"sha256:abc"}]}"#
            .to_vec()
    }

    #[test]
    fn test_verify_registry_signature_accepts_valid() {
        let (key_id, public_key, keypair) = gen_minisign_keypair();
        let data = sample_registry_bytes();
        let sig = sign_data(&data, key_id, &keypair);
        verify_registry_signature(&data, &pubkey_b64(key_id, &public_key), &sig).unwrap();
    }

    #[test]
    fn test_verify_registry_signature_rejects_corrupted_sig() {
        let (key_id, public_key, keypair) = gen_minisign_keypair();
        let data = sample_registry_bytes();
        let sig = sign_data(&data, key_id, &keypair);
        // 破坏 global signature（最后一行）——无论解码失败还是校验失败都必须被拒绝
        let mut lines: Vec<&str> = sig.lines().collect();
        *lines.last_mut().unwrap() = "A";
        let corrupted = lines.join("\n");
        assert!(
            verify_registry_signature(&data, &pubkey_b64(key_id, &public_key), &corrupted).is_err()
        );
    }
    #[test]
    fn test_verify_registry_signature_rejects_tampered_data() {
        let (key_id, public_key, keypair) = gen_minisign_keypair();
        let data = sample_registry_bytes();
        let sig = sign_data(&data, key_id, &keypair);
        // 篡改注册表内容（如替换 checksum）
        let mut tampered = data.clone();
        let needle: &[u8] = b"sha256:abc";
        let pos = tampered
            .windows(needle.len())
            .position(|w| w == needle)
            .expect("needle in fixture");
        tampered[pos + 7] = b'x'; // abc -> xbc
        assert!(
            verify_registry_signature(&tampered, &pubkey_b64(key_id, &public_key), &sig).is_err()
        );
    }

    #[test]
    fn test_verify_registry_signature_rejects_mismatched_key() {
        let (key_id, _public_key, keypair) = gen_minisign_keypair();
        let data = sample_registry_bytes();
        let sig = sign_data(&data, key_id, &keypair);
        let (_, other_pk, _) = gen_minisign_keypair();
        assert!(verify_registry_signature(&data, &pubkey_b64(key_id, &other_pk), &sig).is_err());
    }

    #[test]
    fn test_verify_registry_signature_rejects_bad_pubkey() {
        let (key_id, _public_key, keypair) = gen_minisign_keypair();
        let data = sample_registry_bytes();
        let sig = sign_data(&data, key_id, &keypair);
        let err = verify_registry_signature(&data, "not-base64!!!", &sig).unwrap_err();
        assert!(err.contains("public key parse failed"));
    }

    #[test]
    fn test_verify_registry_signature_rejects_bad_sig_text() {
        let (key_id, public_key, _keypair) = gen_minisign_keypair();
        let data = sample_registry_bytes();
        let err = verify_registry_signature(&data, &pubkey_b64(key_id, &public_key), "garbage")
            .unwrap_err();
        assert!(err.contains("signature decode failed"));
    }

    #[test]
    fn test_verify_registry_signature_no_pubkey_path() {
        // 未配置公钥时 fetch_registry 不校验（走 env 分支逻辑）；此处直接验证
        // verify_registry_signature 在空字符串公钥下报 parse 失败，不会误通过。
        let (key_id, _public_key, keypair) = gen_minisign_keypair();
        let data = sample_registry_bytes();
        let sig = sign_data(&data, key_id, &keypair);
        assert!(verify_registry_signature(&data, "", &sig).is_err());
    }

    // ── N-10（P207 闭环）真实发布链路端到端验证 ──────────────
    // 使用仓库内实际下发的 registry.json 与 registry.json.minisig（由专用私钥以
    // `cargo tauri signer sign -p ''` 签名，ED 预哈希模式）与编译期常量公钥做真实验证。
    // 这是发布链路的防漂移测试：任何一侧（公钥 / 签名 / 注册表内容）不一致即失败。
    #[test]
    fn test_real_registry_signature_end_to_end() {
        let registry_bytes = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/models/registry.json"
        ))
        .expect("registry.json 应随仓库存在");
        let sig_text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/models/registry.json.minisig"
        ))
        .expect("registry.json.minisig 应随仓库存在");
        let embedded_pubkey = EMBED_REGISTRY_PUBKEY_B64.expect("编译期公钥已配置");

        // 合法签名：必须通过
        verify_registry_signature(&registry_bytes, embedded_pubkey, &sig_text)
            .expect("真实注册表签名必须验证通过");

        // 篡改注册表内容（替换 checksum 一字节）：必须拒绝
        let mut tampered = registry_bytes.clone();
        let needle: &[u8] = b"2d07de441b5288be";
        let pos = tampered
            .windows(needle.len())
            .position(|w| w == needle)
            .expect("checksum 前缀应存在于注册表中");
        tampered[pos] = b'0'; // 破坏 sha256 前缀
        assert!(
            verify_registry_signature(&tampered, embedded_pubkey, &sig_text).is_err(),
            "篡改后的注册表必须被拒绝"
        );

        // 错误公钥：必须拒绝
        let (_, other_pk, _) = gen_minisign_keypair();
        assert!(
            verify_registry_signature(
                &registry_bytes,
                &pubkey_b64([0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11], &other_pk),
                &sig_text,
            )
            .is_err(),
            "不匹配的公钥必须被拒绝"
        );

        // 篡改签名文本（第 2 行签名 base64 改坏）：必须拒绝
        let mut sig_lines: Vec<&str> = sig_text.lines().collect();
        sig_lines[1] = "AAAA";
        assert!(
            verify_registry_signature(&registry_bytes, embedded_pubkey, &sig_lines.join("\n"))
                .is_err()
        );
    }
    #[test]
    fn test_compiled_public_key_is_valid_minisign() {
        // 编译期公钥必须能被 minisign_verify 解析（防拼写错误 / 格式漂移）。
        let pubkey_b64 = EMBED_REGISTRY_PUBKEY_B64.expect("编译期公钥已配置");
        let _pk =
            minisign_verify::PublicKey::from_base64(pubkey_b64).expect("编译期公钥必须可解析");
        // 签名也必须可解析（格式合法）
        let sig_text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/models/registry.json.minisig"
        ))
        .expect("registry.json.minisig 应随仓库存在");
        let _sig = minisign_verify::Signature::decode(&sig_text).expect("签名必须可解析");
    }

    #[test]
    fn test_committed_zip_checksum_matches_registry() {
        // 评审建议：提交的 zip 的 sha256 必须与 registry.json 声明的 checksum 一致，
        // 防止「签名有效但 zip/registry 漂移」在用户下载时才暴露。
        let registry_bytes = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/models/registry.json"
        ))
        .expect("registry.json 应随仓库存在");
        let registry: EmbedRegistry = serde_json::from_slice(&registry_bytes).unwrap();
        assert!(!registry.models.is_empty(), "注册表至少应有一个模型");
        for model in &registry.models {
            let zip_path = concat!(env!("CARGO_MANIFEST_DIR"), "/resources/models/").to_string()
                + &format!("{}.zip", model.id);
            let zip_bytes = std::fs::read(&zip_path)
                .unwrap_or_else(|_| panic!("模型 zip 应随仓库存在: {}", zip_path));
            let hash = {
                use sha2::Digest;
                let mut hasher = sha2::Sha256::new();
                hasher.update(&zip_bytes);
                format!("{:x}", hasher.finalize())
            };
            let expected = model
                .checksum
                .strip_prefix("sha256:")
                .unwrap_or(&model.checksum);
            assert_eq!(
                hash, expected,
                "模型 {} 的 zip sha256 与 registry checksum 不一致（zip 或 registry 已漂移）",
                model.id
            );
        }
    }
}
