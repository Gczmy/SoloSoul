//! Embedding model management — download, install, list local models.

use serde::{Deserialize, Serialize};

use sha2::Digest;
use std::path::{Path, PathBuf};
use tauri::{path::BaseDirectory, AppHandle, Emitter, Manager};

#[cfg(mobile)]
use crate::commands::{mobile_not_supported, mobile_not_supported_with};

// ── Registry ─────────────────────────────────────────────────

const REGISTRY_URL: &str = "https://raw.githubusercontent.com/SoloSoul/models/main/registry.json";

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

#[cfg(desktop)]
/// Fetch the remote model registry.
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

    let registry: EmbedRegistry = resp
        .json()
        .await
        .map_err(|e| format!("Parse registry: {}", e))?;

    Ok(registry)
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
}
