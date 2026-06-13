use crate::state::AppState;
use serde::Serialize;
use std::fs::{self as fs_std, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use tauri::State;

/// R012: reject paths that contain parent-dir references, which could escape the
/// intended directory. Used for commands that operate on user-selected files.
fn reject_traversal(path: &str) -> Result<PathBuf, String> {
    let p = Path::new(path);
    if p.components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("Path traversal is not allowed".to_string());
    }
    Ok(p.to_path_buf())
}

/// R012: resolve a path relative to a base directory and ensure the resolved
/// location stays within that base.
fn resolve_within(base: &Path, path: &str) -> Result<PathBuf, String> {
    let p = reject_traversal(path)?;
    let abs = if p.is_absolute() { p } else { base.join(p) };
    let base_canon = base.canonicalize().map_err(|e| e.to_string())?;
    let target_canon = abs.canonicalize().unwrap_or_else(|_| abs.clone());
    if !target_canon.starts_with(&base_canon) {
        return Err("Path is outside the allowed directory".to_string());
    }
    Ok(abs)
}

/// Encrypt a file using chunked AES-256-GCM (SOLO blob v3)
#[tauri::command]
pub async fn encrypt_file(
    state: State<'_, AppState>,
    src_path: String,
    dst_path: String,
) -> Result<(), String> {
    let svc = state.vault_service.read().unwrap();
    let session_key = svc.get_session_key().ok_or("Vault not unlocked")?;
    let key: [u8; 32] = session_key
        .as_slice()
        .try_into()
        .map_err(|_| "Invalid key")?;

    let base = svc.base_path();
    let src = resolve_within(base, &src_path)?;
    let dst = resolve_within(base, &dst_path)?;

    let chunk_size = 1024 * 1024; // 1MB chunks
    let mut reader = BufReader::new(File::open(&src).map_err(|e| format!("Open failed: {}", e))?);
    let mut writer =
        BufWriter::new(File::create(&dst).map_err(|e| format!("Create failed: {}", e))?);
    solosoul_crypto::aes::encrypt_chunked_stream(&key, &mut reader, &mut writer, chunk_size)
        .map_err(|e| format!("Encryption failed: {}", e))?;
    writer.flush().map_err(|e| format!("Flush failed: {}", e))?;
    Ok(())
}

/// Decrypt a SOLO blob file
#[tauri::command]
pub async fn decrypt_file(
    state: State<'_, AppState>,
    src_path: String,
    dst_path: String,
) -> Result<(), String> {
    let svc = state.vault_service.read().unwrap();
    let session_key = svc.get_session_key().ok_or("Vault not unlocked")?;
    let key: [u8; 32] = session_key
        .as_slice()
        .try_into()
        .map_err(|_| "Invalid key")?;

    let base = svc.base_path();
    let src = resolve_within(base, &src_path)?;
    let dst = resolve_within(base, &dst_path)?;

    let mut reader = BufReader::new(File::open(&src).map_err(|e| format!("Open failed: {}", e))?);
    let mut writer =
        BufWriter::new(File::create(&dst).map_err(|e| format!("Create failed: {}", e))?);
    solosoul_crypto::aes::decrypt_chunked_stream(&key, &mut reader, &mut writer)
        .map_err(|e| format!("Decryption failed: {}", e))?;
    writer.flush().map_err(|e| format!("Flush failed: {}", e))?;
    Ok(())
}

/// Create a ZIP package from a directory
#[tauri::command]
pub async fn create_zip_package(src_dir: String, dst_path: String) -> Result<(), String> {
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    let src = reject_traversal(&src_dir)?;
    let dst = reject_traversal(&dst_path)?;

    let file = File::create(&dst).map_err(|e| format!("Failed to create ZIP: {}", e))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let walkdir = walkdir::WalkDir::new(&src);

    for entry in walkdir.into_iter() {
        let entry = entry.map_err(|e| format!("WalkDir error: {}", e))?;
        let path = entry.path();
        let name = path
            .strip_prefix(&src_dir)
            .map_err(|e| format!("Path error: {}", e))?;

        if path.is_file() {
            let mut f = File::open(path).map_err(|e| format!("Open file error: {}", e))?;
            zip.start_file_from_path(name, options)
                .map_err(|e| format!("ZIP start_file error: {}", e))?;
            std::io::copy(&mut f, &mut zip).map_err(|e| format!("ZIP copy error: {}", e))?;
        }
    }
    zip.finish()
        .map_err(|e| format!("ZIP finish error: {}", e))?;
    Ok(())
}

/// Extract a ZIP package to a directory
#[tauri::command]
pub async fn extract_zip_package(zip_path: String, dst_dir: String) -> Result<Vec<String>, String> {
    use std::io::copy;
    use zip::ZipArchive;

    let zip = reject_traversal(&zip_path)?;
    let dst = reject_traversal(&dst_dir)?;

    let file = File::open(&zip).map_err(|e| format!("Failed to open ZIP: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("Failed to read ZIP: {}", e))?;

    fs_std::create_dir_all(&dst).map_err(|e| format!("Create dir error: {}", e))?;

    let mut extracted = Vec::new();
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("ZIP index error: {}", e))?;
        let outpath = dst.join(file.mangled_name());

        if file.name().ends_with('/') {
            fs_std::create_dir_all(&outpath).map_err(|e| format!("Create dir error: {}", e))?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs_std::create_dir_all(p).map_err(|e| format!("Create parent error: {}", e))?;
                }
            }
            let mut outfile =
                File::create(&outpath).map_err(|e| format!("Create file error: {}", e))?;
            copy(&mut file, &mut outfile).map_err(|e| format!("Extract error: {}", e))?;
            extracted.push(outpath.to_string_lossy().to_string());
        }
    }
    Ok(extracted)
}

/// Inspect a backup file and return metadata about its contents
#[tauri::command]
pub async fn inspect_backup(
    state: State<'_, AppState>,
    backup_path: String,
) -> Result<String, String> {
    let svc = state.vault_service.read().unwrap();
    let session_key = svc.get_session_key().ok_or("Vault not unlocked")?;
    let key: [u8; 32] = session_key
        .as_slice()
        .try_into()
        .map_err(|_| "Invalid key")?;

    let base = svc.base_path();
    let backup = resolve_within(base, &backup_path)?;

    let encrypted = fs_std::read(&backup).map_err(|e| format!("Read backup failed: {}", e))?;
    let plaintext = solosoul_crypto::aes::decrypt_blob(&key, &encrypted)
        .map_err(|e| format!("Decryption failed: {}", e))?;
    let json_str =
        String::from_utf8(plaintext.to_vec()).map_err(|e| format!("Invalid UTF-8: {}", e))?;

    let mut obj_count = 0;
    let mut type_ids: Vec<String> = Vec::new();

    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_str) {
        if let Some(arr) = val
            .get("unified_objects")
            .and_then(|u| u.get("objects"))
            .and_then(|o| o.as_array())
        {
            obj_count = arr.len();
            for obj in arr.iter().take(30) {
                if let Some(tid) = obj.get("typeId").and_then(|t| t.as_str()) {
                    type_ids.push(tid.to_string());
                }
            }
        }
    }

    Ok(format!(
        "BACKUP: len={}, obj_count={}, type_ids={:?}",
        json_str.len(),
        obj_count,
        type_ids
    ))
}

// ── Directory scanning for local import ────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ScannedFile {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub ext: String,
}

#[tauri::command]
pub async fn fs_scan_directory(path: String) -> Result<Vec<ScannedFile>, String> {
    let dir = reject_traversal(&path)?;
    if !dir.is_dir() {
        return Err("Not a directory".to_string());
    }
    let mut files = Vec::new();
    scan_dir_recursive(&dir, &mut files, 3)?; // max depth 3
    Ok(files)
}

fn scan_dir_recursive(
    dir: &Path,
    files: &mut Vec<ScannedFile>,
    max_depth: u32,
) -> Result<(), String> {
    if max_depth == 0 {
        return Ok(());
    }
    let entries = fs_std::read_dir(dir).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            scan_dir_recursive(&path, files, max_depth - 1)?;
        } else if path.is_file() {
            let metadata = fs_std::metadata(&path).map_err(|e| e.to_string())?;
            files.push(ScannedFile {
                path: path.to_string_lossy().to_string(),
                name: path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                size: metadata.len(),
                ext: path
                    .extension()
                    .map(|e| e.to_string_lossy().to_string())
                    .unwrap_or_default(),
            });
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn fs_get_file_size(path: String) -> Result<u64, String> {
    let p = reject_traversal(&path)?;
    let meta = std::fs::metadata(&p).map_err(|e| format!("Read: {}", e))?;
    Ok(meta.len())
}

#[tauri::command]
pub async fn fs_read_file_as_data_url(path: String) -> Result<String, String> {
    use std::io::Read;
    let p = reject_traversal(&path)?;
    let mut file = std::fs::File::open(&p).map_err(|e| format!("Open: {}", e))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .map_err(|e| format!("Read: {}", e))?;
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
    let mime = match ext.to_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        "md" => "text/markdown",
        "json" => "application/json",
        "xml" => "application/xml",
        "csv" => "text/csv",
        _ => "application/octet-stream",
    };
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &buf);
    Ok(format!("data:{};base64,{}", mime, b64))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_scanned_file_serde() {
        let file = ScannedFile {
            path: "/tmp/test.txt".to_string(),
            name: "test.txt".to_string(),
            size: 42,
            ext: "txt".to_string(),
        };
        let json = serde_json::to_string(&file).unwrap();
        assert!(json.contains("test.txt"));
        assert!(json.contains("42"));
    }

    #[test]
    fn test_scan_dir_recursive() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(dir.path().join("a.txt"), "hello").unwrap();
        fs::write(sub.join("b.txt"), "world").unwrap();

        let mut files = Vec::new();
        scan_dir_recursive(dir.path(), &mut files, 3).unwrap();
        assert_eq!(files.len(), 2);
        let names: Vec<_> = files.iter().map(|f| f.name.clone()).collect();
        assert!(names.contains(&"a.txt".to_string()));
        assert!(names.contains(&"b.txt".to_string()));
    }

    #[test]
    fn test_scan_dir_recursive_max_depth() {
        let dir = TempDir::new().unwrap();
        let level1 = dir.path().join("level1");
        fs::create_dir(&level1).unwrap();
        let level2 = level1.join("level2");
        fs::create_dir(&level2).unwrap();
        fs::write(level2.join("deep.txt"), "deep").unwrap();

        let mut files = Vec::new();
        scan_dir_recursive(dir.path(), &mut files, 1).unwrap();
        assert_eq!(files.len(), 0); // level2 is beyond max_depth 1

        let mut files2 = Vec::new();
        scan_dir_recursive(dir.path(), &mut files2, 2).unwrap();
        assert_eq!(files2.len(), 0); // level2 is beyond max_depth 2

        let mut files3 = Vec::new();
        scan_dir_recursive(dir.path(), &mut files3, 3).unwrap();
        assert_eq!(files3.len(), 1);
        assert_eq!(files3[0].name, "deep.txt");
    }

    #[test]
    fn test_scan_dir_recursive_not_directory() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("not_a_dir.txt");
        fs::write(&file_path, "data").unwrap();
        let mut files = Vec::new();
        let result = scan_dir_recursive(&file_path, &mut files, 3);
        assert!(result.is_err());
    }

    #[test]
    fn test_fs_get_file_size() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.bin");
        fs::write(&path, vec![0u8; 1234]).unwrap();
        let size =
            futures::executor::block_on(fs_get_file_size(path.to_string_lossy().to_string()))
                .unwrap();
        assert_eq!(size, 1234);
    }

    #[test]
    fn test_fs_read_file_as_data_url() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.png");
        fs::write(&path, vec![0u8; 100]).unwrap();
        let url = futures::executor::block_on(fs_read_file_as_data_url(
            path.to_string_lossy().to_string(),
        ))
        .unwrap();
        assert!(url.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn test_fs_read_file_as_data_url_unknown_ext() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.xyz");
        fs::write(&path, "hello").unwrap();
        let url = futures::executor::block_on(fs_read_file_as_data_url(
            path.to_string_lossy().to_string(),
        ))
        .unwrap();
        assert!(url.starts_with("data:application/octet-stream;base64,"));
    }
}
