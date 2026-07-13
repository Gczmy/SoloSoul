use crate::state::AppState;
use serde::Serialize;
use std::fs::{self as fs_std};
use std::io::Read;
use std::path::{Path, PathBuf};
#[cfg(mobile)]
use tauri::Manager;
use tauri::State;

/// Maximum file size that can be read into memory for a data URL preview (10 MiB).
const MAX_DATA_URL_SIZE: u64 = 10 * 1024 * 1024;

/// Maximum number of files returned by `fs_scan_directory`.
const MAX_SCAN_FILES: usize = 1_000;

/// Maximum recursion depth for `fs_scan_directory`.
const MAX_SCAN_DEPTH: u32 = 8;

/// Number of objects sampled from a backup for the type-id preview.
const BACKUP_PREVIEW_SAMPLE: usize = 30;

/// Return the allowed base directory for filesystem commands.
/// - 桌面端：优先使用 `SOLOSOUL_FS_BASE` 环境变量，否则使用用户 home 目录。
/// - 移动端：使用 Tauri 应用私有数据目录，避免访问任意文件系统路径。
#[cfg(not(test))]
fn allowed_fs_base<R: tauri::Runtime>(
    #[allow(unused_variables)] app: &tauri::AppHandle<R>,
) -> Result<PathBuf, String> {
    #[cfg(mobile)]
    {
        app.path()
            .resolve(".", tauri::path::BaseDirectory::Data)
            .map_err(|e| format!("无法解析应用数据目录: {e}"))
    }
    #[cfg(desktop)]
    {
        if let Ok(base) = std::env::var("SOLOSOUL_FS_BASE") {
            return Ok(PathBuf::from(base));
        }
        let home_key = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
        std::env::var(home_key).map(PathBuf::from).map_err(|_| {
            "Could not determine user home directory; set SOLOSOUL_FS_BASE".to_string()
        })
    }
}

/// During tests, allow any absolute path by using the filesystem root as the
/// base. This keeps unit tests simple while still exercising path logic.
#[cfg(test)]
fn allowed_fs_base<R: tauri::Runtime>(_app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    Ok(PathBuf::from(if cfg!(windows) { "C:\\" } else { "/" }))
}

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

/// R012: canonicalize a path by resolving symlinks on the deepest existing
/// prefix and appending any non-existing trailing components.
fn canonicalize_existing(path: &Path) -> Result<PathBuf, String> {
    if path.exists() {
        return path.canonicalize().map_err(|e| e.to_string());
    }
    let parent = path.parent().ok_or("Invalid path")?;
    let mut canon = canonicalize_existing(parent)?;
    if let Some(name) = path.file_name() {
        canon.push(name);
    }
    Ok(canon)
}

/// R012: resolve a path relative to a base directory and ensure the resolved
/// location stays within that base. Symlinks are resolved on existing path
/// components; if resolution fails, the path is rejected rather than falling
/// back to a textual comparison.
fn resolve_within(base: &Path, path: &str) -> Result<PathBuf, String> {
    let p = reject_traversal(path)?;
    let abs = if p.is_absolute() { p } else { base.join(p) };
    let base_canon = base.canonicalize().map_err(|e| e.to_string())?;
    let target_canon = canonicalize_existing(&abs)?;
    if !target_canon.starts_with(&base_canon) {
        return Err("Path is outside the allowed directory".to_string());
    }
    Ok(abs)
}

/// Resolve `path` within the allowed filesystem base directory. Filesystem
/// commands that operate on user-selected paths must use this helper.
fn resolve_allowed_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    path: &str,
) -> Result<PathBuf, String> {
    let base = allowed_fs_base(app)?;
    resolve_within(&base, path)
}

/// Inspect a backup file and return metadata about its contents
///
/// # OOM protection
/// 使用流式解密（`decrypt_blob_stream`）逐块处理，避免将整个备份文件读入内存。
/// 同时限制预览样本数为 `BACKUP_PREVIEW_SAMPLE`（30 条）。
#[tauri::command]
pub async fn inspect_backup<R: tauri::Runtime>(
    #[allow(unused_variables)] app: tauri::AppHandle<R>,
    state: State<'_, AppState>,
    backup_path: String,
) -> Result<String, String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let session_key = svc.get_session_key().ok_or("Vault not unlocked")?;
    let key: [u8; 32] = session_key
        .as_slice()
        .try_into()
        .map_err(|_| "Invalid key")?;

    let base = svc.base_path();
    let backup = resolve_within(base, &backup_path)?;

    // 文件大小上限检查（500MB 安全阈值）
    let meta = backup.metadata().map_err(|e| format!("Metadata: {}", e))?;
    if meta.len() > 500 * 1024 * 1024 {
        return Err("Backup file too large (> 500 MB)".to_string());
    }

    // ── 流式读取并解密，避免将备份文件全量读入内存 ──
    let max_read = std::cmp::min(meta.len(), 50 * 1024 * 1024) as u64;
    let mut encrypted_reader = {
        let file =
            std::fs::File::open(&backup).map_err(|e| format!("Open backup failed: {}", e))?;
        std::io::BufReader::new(file).take(max_read)
    };

    // 使用流式解密降低内存峰值。backup 的体积通常远小于 50 MB。
    let mut decrypted_buf = Vec::new();
    solosoul_crypto::aes::decrypt_chunked_stream(&key, &mut encrypted_reader, &mut decrypted_buf)
        .map_err(|e| format!("Decryption failed: {}", e))?;
    let json_str = String::from_utf8(decrypted_buf).map_err(|e| format!("Invalid UTF-8: {}", e))?;

    let mut obj_count = 0;
    let mut type_ids: Vec<String> = Vec::new();

    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_str) {
        if let Some(arr) = val
            .get("unified_objects")
            .and_then(|u| u.get("objects"))
            .and_then(|o| o.as_array())
        {
            obj_count = arr.len();
            for obj in arr.iter().take(BACKUP_PREVIEW_SAMPLE) {
                if let Some(tid) = obj.get("typeId").and_then(|t| t.as_str()) {
                    type_ids.push(tid.to_string());
                }
            }
        }
    }

    Ok(format!(
        "BACKUP: file_size={}, obj_count={}, type_ids={:?}",
        meta.len(),
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
pub async fn fs_scan_directory<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    path: String,
) -> Result<Vec<ScannedFile>, String> {
    let dir = resolve_allowed_path(&app, &path)?;
    if !dir.is_dir() {
        return Err("Not a directory".to_string());
    }
    let mut files = Vec::new();
    scan_dir_recursive(&dir, &mut files, MAX_SCAN_DEPTH)?;
    Ok(files)
}

fn scan_dir_recursive(
    dir: &Path,
    files: &mut Vec<ScannedFile>,
    max_depth: u32,
) -> Result<(), String> {
    if max_depth == 0 || files.len() >= MAX_SCAN_FILES {
        return Ok(());
    }
    let entries = fs_std::read_dir(dir).map_err(|e| e.to_string())?;
    for entry in entries {
        if files.len() >= MAX_SCAN_FILES {
            break;
        }
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
pub async fn fs_get_file_size<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    path: String,
) -> Result<u64, String> {
    let p = resolve_allowed_path(&app, &path)?;
    let meta = std::fs::metadata(&p).map_err(|e| format!("Read: {}", e))?;
    Ok(meta.len())
}

#[tauri::command]
pub async fn fs_is_dir<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    path: String,
) -> Result<bool, String> {
    let p = resolve_allowed_path(&app, &path)?;
    let meta = std::fs::metadata(&p).map_err(|e| format!("Read: {}", e))?;
    Ok(meta.is_dir())
}

#[tauri::command]
pub async fn fs_read_file_as_data_url<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    path: String,
) -> Result<String, String> {
    use std::io::Read;
    let p = resolve_allowed_path(&app, &path)?;
    let mut file = std::fs::File::open(&p).map_err(|e| format!("Open: {}", e))?;
    let meta = file.metadata().map_err(|e| format!("Metadata: {}", e))?;
    if meta.len() > MAX_DATA_URL_SIZE {
        return Err(format!(
            "File too large for preview: {} bytes (max {})",
            meta.len(),
            MAX_DATA_URL_SIZE
        ));
    }
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
        let app = tauri::test::mock_app();
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.bin");
        fs::write(&path, vec![0u8; 1234]).unwrap();
        let size = futures::executor::block_on(fs_get_file_size(
            app.handle().clone(),
            path.to_string_lossy().to_string(),
        ))
        .unwrap();
        assert_eq!(size, 1234);
    }

    #[test]
    fn test_fs_read_file_as_data_url() {
        let app = tauri::test::mock_app();
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.png");
        fs::write(&path, vec![0u8; 100]).unwrap();
        let url = futures::executor::block_on(fs_read_file_as_data_url(
            app.handle().clone(),
            path.to_string_lossy().to_string(),
        ))
        .unwrap();
        assert!(url.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn test_fs_read_file_as_data_url_unknown_ext() {
        let app = tauri::test::mock_app();
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.xyz");
        fs::write(&path, "hello").unwrap();
        let url = futures::executor::block_on(fs_read_file_as_data_url(
            app.handle().clone(),
            path.to_string_lossy().to_string(),
        ))
        .unwrap();
        assert!(url.starts_with("data:application/octet-stream;base64,"));
    }
}
