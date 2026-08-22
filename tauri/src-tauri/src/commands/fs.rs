use serde::Serialize;
use std::fs::{self as fs_std};
use std::path::{Path, PathBuf};
// `Manager`（`AppHandle::try_state`/`path`）与 `AppState` 仅在非测试构建中使用：
// 测试变体的 `allowed_fs_bases` 直接返回文件系统根目录，不触碰 App 状态。
#[cfg(not(test))]
use tauri::Manager;
// P107：访问 AppState 以读取 Vault 附件目录与附件解密密钥。
// 桌面端：`allowed_fs_bases` 放行 Vault 附件目录；移动端：附件在应用数据目录内。
#[cfg(not(test))]
use crate::state::AppState;

/// Maximum file size that can be read into memory for a data URL preview (10 MiB).
const MAX_DATA_URL_SIZE: u64 = 10 * 1024 * 1024;

/// Maximum file size that may be read into memory for image preview generation (50 MiB).
///
/// 手机原图普遍超过 `MAX_DATA_URL_SIZE`（10 MiB），照片集预览必须走缩放路径；
/// 但解码超大文件会占用过多内存，超过该上限直接拒绝而非尝试解码。
const MAX_IMAGE_PREVIEW_READ_SIZE: u64 = 50 * 1024 * 1024;

/// JPEG 重编码质量（照片集缩略图 / 全屏预览共用）。
const IMAGE_PREVIEW_JPEG_QUALITY: u8 = 80;

/// 预览最长边上限（防御性钳制）：防止畸形 `max_dim` 触发病态 resize 请求。
const MAX_PREVIEW_DIM: u32 = 8192;

/// Maximum number of files returned by `fs_scan_directory`.
const MAX_SCAN_FILES: usize = 1_000;

/// Maximum recursion depth for `fs_scan_directory`.
const MAX_SCAN_DEPTH: u32 = 8;

/// Return the allowed base directories for filesystem commands.
///
/// - 桌面端：默认收窄到 **Desktop/Documents/Downloads + Vault 附件目录**（P107），
///   `SOLOSOUL_FS_BASE` 环境变量作为**额外放宽**的根目录（叠加语义，N010-②，与
///   `attachment::allowed_fs_bases` 一致）——与 OCR `is_path_in_allowed_dir` 的
///   用户目录范围一致，杜绝 XSS 经 `fs_read_file_as_text/data_url` 读取 home 下
///   任意文件（含 `~/.solosoul/**`）。
///   Vault 附件目录必须放行：附件预览（`AttachmentPreviewOverlay`）读取的是
///   `vaultPath` 指向的落库副本（`{base}/attachments/...`），不放行则预览功能失效；
///   仅放行 `attachments/` 子目录，`config.json`/`vault.db`/`accounts.json` 等
///   仍不可经 fs 命令读取。
/// - 移动端：使用 Tauri 应用私有数据目录，避免访问任意文件系统路径。
#[cfg(not(test))]
fn allowed_fs_bases<R: tauri::Runtime>(
    #[allow(unused_variables)] app: &tauri::AppHandle<R>,
) -> Result<Vec<PathBuf>, String> {
    #[cfg(mobile)]
    {
        Ok(vec![app
            .path()
            .resolve(".", tauri::path::BaseDirectory::Data)
            .map_err(|e| format!("无法解析应用数据目录: {e}"))?])
    }
    #[cfg(desktop)]
    {
        // N010-②: SOLOSOUL_FS_BASE 为「额外放宽」目录（叠加语义，与
        // `attachment::allowed_fs_bases` 一致）——若仍按旧逻辑替换默认集合，设置后
        // Vault 附件目录不在白名单内，附件预览（vaultPath 指向 {base}/attachments）
        // 将失效；同时避免同一环境变量在两处语义分叉。
        let mut bases = Vec::new();
        if let Ok(base) = std::env::var("SOLOSOUL_FS_BASE") {
            bases.push(PathBuf::from(base));
        }
        let home_key = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
        if let Ok(home) = std::env::var(home_key) {
            let home = PathBuf::from(home);
            // 在单个闭包内消费 guard 并克隆出 PathBuf，避免 guard 借用 State 逃逸。
            let vault_base = app.try_state::<AppState>().and_then(|s| {
                s.vault_service
                    .read()
                    .ok()
                    .map(|svc| svc.base_path().clone())
            });
            bases.extend(desktop_fs_bases(&home, vault_base.as_deref()));
        }
        if bases.is_empty() {
            return Err(
                "无法确定用户主目录；请设置 SOLOSOUL_FS_BASE 环境变量以指定允许的根目录"
                    .to_string(),
            );
        }
        Ok(bases)
    }
}

/// 桌面端默认允许基目录集合：Desktop/Documents/Downloads + OneDrive + Vault 附件目录。
/// 抽为纯函数便于单测；Vault 附件目录放行是为了附件预览（`vaultPath` 指向
/// `{base}/attachments/...` 落库副本），而 vault 根目录本身（config.json、
/// vault.db、accounts.json 等）不在集合内。
/// OneDrive 目录（个人版 `OneDrive`、企业版 `OneDrive - <组织名>`）放行：Windows
/// 用户默认把文件存放在 `~/OneDrive`，不放行则上传附件时 `fs_get_file_size` /
/// `attachment_copy_to_vault` 被白名单拒绝；与 Desktop/Documents/Downloads 同级
/// 的用户自有目录，属同一信任级别。目录不存在时 `resolve_within` 的 canonicalize
/// 失败即视为该基目录不可用（与静态三个目录同语义）；企业版目录名含组织名无法
/// 静态枚举，按前缀扫描 home 一级子目录收集。
#[cfg(desktop)]
fn desktop_fs_bases(home: &Path, vault_base: Option<&Path>) -> Vec<PathBuf> {
    let mut bases = vec![
        home.join("Desktop"),
        home.join("Documents"),
        home.join("Downloads"),
        home.join("OneDrive"),
    ];
    // 企业版 OneDrive：目录名形如 `OneDrive - Contoso`，静态枚举不可行，按前缀收集。
    if let Ok(entries) = std::fs::read_dir(home) {
        bases.extend(entries.flatten().map(|e| e.path()).filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("OneDrive - "))
                .unwrap_or(false)
        }));
    }
    if let Some(vault) = vault_base {
        bases.push(vault.join("attachments"));
    }
    // Phase 1 云打包：其余云盘同步目录与 attachment 白名单同源（cloud_targets 检测）。
    for t in crate::commands::cloud_targets::detect_cloud_sync_dirs(home) {
        if let Ok(canon) = PathBuf::from(&t.path).canonicalize() {
            bases.push(canon);
        }
    }
    bases
}

/// During tests, allow any absolute path by using the filesystem root as the
/// base. This keeps unit tests simple while still exercising path logic.
#[cfg(test)]
fn allowed_fs_bases<R: tauri::Runtime>(_app: &tauri::AppHandle<R>) -> Result<Vec<PathBuf>, String> {
    Ok(vec![PathBuf::from(if cfg!(windows) {
        "C:\\"
    } else {
        "/"
    })])
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
    // P017: 返回 canonical 目标路径而非字面路径——消除符号链接 TOCTOU 竞态：
    // 校验与后续文件操作使用同一已解析路径，字面路径可能在校验后被替换绕过。
    Ok(target_canon)
}

/// N010-④/P017: 回传前端前的路径显示形态。Windows `canonicalize` 会产出
/// `\\?\` 扩展长度前缀，前端（对话框、展示、再次回传）无法直接使用——剥离之；
/// 非 Windows 或未带前缀时原样返回。
pub(crate) fn display_fs_path(path: &Path) -> String {
    // R004-②: 纯字符串处理（跨平台确定，不引入 dunce）。`\\?\` 为 Windows 扩展
    // 长度前缀：普通盘符路径 `\\?\C:\...` → `C:\...`；网络路径 `\\?\UNC\server\share`
    // 需把 `UNC\` 段还原为 `\\server\share`。dunce::simplified 在 Windows 上仅把
    // 盘符 VerbatimDisk 还原为常规路径，对网络 VerbatimUNC（`\\?\UNC\...`）按设计
    // 刻意保留原样，且非 Windows 为 no-op——本实现两类前缀都处理且跨平台行为一致，
    // 便于单测在任意平台（含 CI）稳定验证。
    let s = path.to_string_lossy();
    if let Some(stripped) = s.strip_prefix(r"\\?\") {
        if let Some(unc) = stripped.strip_prefix("UNC\\") {
            format!(r"\\{unc}")
        } else {
            stripped.to_string()
        }
    } else {
        s.to_string()
    }
}

/// P001：读取附件文件内容——SOLC 密文则用附件密钥解密，普通明文直接读。
/// 前端预览/导出读取 vault 附件（现已加密落盘）时必须走此路径；
/// 非加密文件（普通用户文件）原样读取，零影响。
#[cfg(not(test))]
pub(crate) fn read_file_with_attachment_decrypt<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    path: &Path,
    max_size: u64,
) -> Result<Vec<u8>, String> {
    if solosoul_core::attachment_crypto::is_encrypted_file(path) {
        let key_arr: [u8; 32] = {
            let state = app
                .try_state::<AppState>()
                .ok_or_else(|| "无法获取附件解密密钥".to_string())?;
            let svc = state
                .vault_service
                .read()
                .map_err(|_| "Vault service lock poisoned".to_string())?;
            let key = svc
                .attachment_encryption_key()
                .map_err(|e| format!("无法获取附件解密密钥: {}", e))?;
            key.as_slice()
                .try_into()
                .map_err(|_| "附件密钥长度错误".to_string())?
        };
        solosoul_core::attachment_crypto::read_file_decrypted(&key_arr, path, max_size)
    } else {
        std::fs::read(path).map_err(|e| format!("Read: {}", e))
    }
}

/// 测试变体：不依赖 AppState（测试环境无 Tauri 状态），非加密文件直接读。
#[cfg(test)]
pub(crate) fn read_file_with_attachment_decrypt<R: tauri::Runtime>(
    _app: &tauri::AppHandle<R>,
    path: &Path,
    max_size: u64,
) -> Result<Vec<u8>, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("Read: {}", e))?;
    if meta.len() > max_size {
        return Err(format!("File too large: {} bytes", meta.len()));
    }
    std::fs::read(path).map_err(|e| format!("Read: {}", e))
}

/// Resolve `path` within any allowed filesystem base directory. Filesystem
/// commands that operate on user-selected paths must use this helper.
///
/// 逐个允许基目录尝试 `resolve_within`；任一命中即返回规范化路径（P017：不再
/// 返回字面路径，避免符号链接 TOCTOU）。若某基目录不存在（canonicalize 失败），
/// 视为该基目录不可用并继续尝试下一个。
pub(crate) fn resolve_allowed_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    path: &str,
) -> Result<PathBuf, String> {
    let bases = allowed_fs_bases(app)?;
    let mut last_err: Option<String> = None;
    let mut any_base_exists = false;
    let mut traversal_rejected = false;
    for base in &bases {
        match resolve_within(base, path) {
            Ok(abs) => return Ok(abs),
            Err(e) => {
                // 基目录存在与否决定「越界」语义：若所有基目录都不存在
                // （如首启、目录被删），透传真实错误便于诊断。
                if base.exists() {
                    any_base_exists = true;
                }
                // R012：路径穿越是显式安全拒绝，优先透传而非被越界文案掩盖。
                if e == "Path traversal is not allowed" {
                    traversal_rejected = true;
                }
                last_err = Some(e);
            }
        }
    }
    if traversal_rejected {
        Err("Path traversal is not allowed".to_string())
    } else if any_base_exists {
        Err("Path is outside the allowed directory".to_string())
    } else {
        Err(last_err.unwrap_or_else(|| "Path is outside the allowed directory".to_string()))
    }
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
    // P024: 同步递归遍历 + 逐文件 metadata 为阻塞 IO，移入 spawn_blocking。
    tokio::task::spawn_blocking(move || {
        let mut files = Vec::new();
        scan_dir_recursive(&dir, &mut files, MAX_SCAN_DEPTH)?;
        Ok::<_, String>(files)
    })
    .await
    .map_err(|e| format!("fs_scan_directory task failed: {e}"))?
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
                path: display_fs_path(&path),
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
    let p = resolve_allowed_path(&app, &path)?;
    // P001: vault 附件密文自动解密；普通文件原样读取。
    let buf = read_file_with_attachment_decrypt(&app, &p, MAX_DATA_URL_SIZE)?;
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

/// 生成图片预览 data URL：解码 → 超过 `max_dim` 的最长边等比缩放 → JPEG 重编码 → base64。
///
/// 纯函数便于单测；调用方（`fs_read_image_preview`）负责路径白名单校验与 spawn_blocking。
/// 失败场景（文件不存在/超限/解码失败如 HEIC）均返回 Err，由前端降级为占位图。
/// `bytes` 为文件内容（P001：调用方已对 vault 附件密文解密，此处直接解码内存）。
fn generate_image_preview_from_bytes(bytes: &[u8], max_dim: u32) -> Result<String, String> {
    let max_dim = max_dim.min(MAX_PREVIEW_DIM);
    let img = image::load_from_memory(bytes).map_err(|e| format!("Decode: {e}"))?;
    let img = if max_dim > 0 && (img.width() > max_dim || img.height() > max_dim) {
        img.thumbnail(max_dim, max_dim)
    } else {
        img
    };
    let mut buf: Vec<u8> = Vec::new();
    {
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
            &mut buf,
            IMAGE_PREVIEW_JPEG_QUALITY,
        );
        encoder
            .encode_image(&img)
            .map_err(|e| format!("Encode: {e}"))?;
    }
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &buf);
    Ok(format!("data:image/jpeg;base64,{b64}"))
}

/// 读取图片并生成缩放后的预览 data URL（照片集缩略图/全屏预览用）。
///
/// - `path`：vaultPath 指向的落库副本，经 `resolve_allowed_path` 白名单校验；
/// - `max_dim`：最长边缩放上限（网格 ≈256，全屏 ≈1600）；`0` 表示不缩放；
/// - 解码在 `spawn_blocking` 中进行，避免阻塞 async 运行时。
/// - 相比 `fs_read_file_as_data_url` 的优势：手机原图常超 10 MiB data URL 上限，
///   且整文件 base64 驻留 JS 堆不可释放；本命令缩放后仅返回小尺寸 JPEG。
#[tauri::command]
pub async fn fs_read_image_preview<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    path: String,
    max_dim: u32,
) -> Result<String, String> {
    let p = resolve_allowed_path(&app, &path)?;
    // P001: vault 附件密文自动解密（图片解码改为内存解码）。
    let max_dim = max_dim.min(MAX_PREVIEW_DIM);
    let bytes = {
        let meta = std::fs::metadata(&p).map_err(|e| format!("Metadata: {e}"))?;
        if meta.len() > MAX_IMAGE_PREVIEW_READ_SIZE {
            return Err(format!(
                "File too large for image preview: {} bytes (max {})",
                meta.len(),
                MAX_IMAGE_PREVIEW_READ_SIZE
            ));
        }
        read_file_with_attachment_decrypt(&app, &p, MAX_IMAGE_PREVIEW_READ_SIZE)?
    };
    tokio::task::spawn_blocking(move || generate_image_preview_from_bytes(&bytes, max_dim))
        .await
        .map_err(|e| format!("fs_read_image_preview task failed: {e}"))?
}

/// Read a text file and return its contents as a UTF-8 string.
/// Used for in-app preview of txt/md/json/xml/csv attachments.
#[tauri::command]
pub async fn fs_read_file_as_text<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    path: String,
) -> Result<String, String> {
    let p = resolve_allowed_path(&app, &path)?;
    // P001: vault 附件密文自动解密；普通文件原样读取。
    let buf = read_file_with_attachment_decrypt(&app, &p, MAX_DATA_URL_SIZE)?;
    String::from_utf8(buf).map_err(|e| format!("Text decode: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// N010-④/P017 防回归：Windows canonicalize 的 `\\?\` 前缀在回传前端前剥离；
    /// R004-②：UNC 网络路径 `\\?\UNC\server\share` → `\\server\share`；
    /// 普通路径原样返回。
    #[test]
    fn test_display_fs_path_strips_windows_extended_prefix() {
        assert_eq!(
            display_fs_path(Path::new(r"\\?\C:\Users\me\file.txt")),
            r"C:\Users\me\file.txt"
        );
        assert_eq!(
            display_fs_path(Path::new(r"\\?\UNC\server\share\file.txt")),
            r"\\server\share\file.txt"
        );
        assert_eq!(
            display_fs_path(Path::new("/Users/me/file.txt")),
            "/Users/me/file.txt"
        );
        assert_eq!(display_fs_path(Path::new("relative/path")), "relative/path");
    }

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

    // ── P107 allowed_fs_bases 收窄 ────────────────────────────────

    #[cfg(desktop)]
    #[test]
    fn test_desktop_fs_bases_includes_user_dirs_and_vault_attachments() {
        let home = Path::new("/Users/testuser");
        let bases = desktop_fs_bases(home, Some(Path::new("/Users/testuser/.solosoul")));
        assert!(bases.contains(&home.join("Desktop")));
        assert!(bases.contains(&home.join("Documents")));
        assert!(bases.contains(&home.join("Downloads")));
        // OneDrive 个人版默认目录（目录不存在时同样入集合，由 resolve_within 视为不可用）。
        assert!(bases.contains(&home.join("OneDrive")));
        // 附件预览需要 vaultPath 指向的附件目录。
        assert!(bases.contains(&Path::new("/Users/testuser/.solosoul").join("attachments")));
        // vault 根目录本身（config.json/vault.db/accounts.json）不得在集合内。
        assert!(!bases.contains(&PathBuf::from("/Users/testuser/.solosoul")));
        assert_eq!(bases.len(), 5);
    }

    #[cfg(desktop)]
    #[test]
    fn test_desktop_fs_bases_without_vault() {
        let home = Path::new("/Users/testuser");
        let bases = desktop_fs_bases(home, None);
        assert_eq!(bases.len(), 4);
        assert!(!bases
            .iter()
            .any(|b| b.to_string_lossy().contains("attachments")));
    }

    #[cfg(desktop)]
    #[test]
    fn test_desktop_fs_bases_collects_onedrive_business_dirs() {
        // 企业版 OneDrive 目录名含组织名（`OneDrive - Contoso`），无法静态枚举，
        // 应按前缀扫描 home 一级子目录收集；不匹配的目录不得误入白名单。
        let dir = TempDir::new().unwrap();
        let home = dir.path();
        fs::create_dir(home.join("OneDrive - Contoso")).unwrap();
        fs::create_dir(home.join("OneDrive - 示例公司")).unwrap();
        fs::create_dir(home.join("Projects")).unwrap();
        fs::create_dir(home.join("OneDriveBackup")).unwrap();

        let bases = desktop_fs_bases(home, None);
        assert!(bases.contains(&home.join("OneDrive - Contoso")));
        assert!(bases.contains(&home.join("OneDrive - 示例公司")));
        // 非 `OneDrive - ` 前缀目录（含 `OneDriveBackup`）不得入白名单。
        assert!(!bases.contains(&home.join("Projects")));
        assert!(!bases.contains(&home.join("OneDriveBackup")));
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

    #[test]
    fn test_fs_read_file_as_text() {
        let app = tauri::test::mock_app();
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "hello 世界").unwrap();
        let content = futures::executor::block_on(fs_read_file_as_text(
            app.handle().clone(),
            path.to_string_lossy().to_string(),
        ))
        .unwrap();
        assert_eq!(content, "hello 世界");
    }

    // ── fs_read_image_preview / generate_image_preview ────────────

    #[test]
    fn test_generate_image_preview_scales_down_longest_edge() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("big.png");
        // 2000×1000：等比缩放到最长边 256 → 256×128
        image::RgbaImage::from_pixel(2000, 1000, image::Rgba([200, 30, 40, 255]))
            .save(&path)
            .unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let url = generate_image_preview_from_bytes(&bytes, 256).unwrap();
        assert!(url.starts_with("data:image/jpeg;base64,"));

        let b64 = url.strip_prefix("data:image/jpeg;base64,").unwrap();
        let bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64).unwrap();
        let decoded = image::load_from_memory(&bytes).unwrap();
        assert_eq!(decoded.width(), 256);
        assert_eq!(decoded.height(), 128);
    }

    #[test]
    fn test_generate_image_preview_does_not_upscale_small_images() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("small.png");
        image::RgbaImage::from_pixel(100, 50, image::Rgba([10, 200, 30, 255]))
            .save(&path)
            .unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let url = generate_image_preview_from_bytes(&bytes, 256).unwrap();
        let b64 = url.strip_prefix("data:image/jpeg;base64,").unwrap();
        let bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64).unwrap();
        let decoded = image::load_from_memory(&bytes).unwrap();
        assert_eq!(decoded.width(), 100);
        assert_eq!(decoded.height(), 50);
    }

    #[test]
    fn test_generate_image_preview_max_dim_zero_keeps_size() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("big.png");
        image::RgbaImage::from_pixel(2000, 1000, image::Rgba([1, 2, 3, 255]))
            .save(&path)
            .unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let url = generate_image_preview_from_bytes(&bytes, 0).unwrap();
        let b64 = url.strip_prefix("data:image/jpeg;base64,").unwrap();
        let bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64).unwrap();
        let decoded = image::load_from_memory(&bytes).unwrap();
        assert_eq!(decoded.width(), 2000);
        assert_eq!(decoded.height(), 1000);
    }

    #[test]
    fn test_generate_image_preview_rejects_undecodable_bytes() {
        // 密文/损坏数据应返回 Err（解码失败），由前端降级为占位图。
        let url = generate_image_preview_from_bytes(b"not an image at all", 256);
        assert!(url.is_err());
    }

    #[test]
    fn test_generate_image_preview_clamps_max_dim() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("clamp.png");
        image::RgbaImage::from_pixel(100, 50, image::Rgba([1, 1, 1, 255]))
            .save(&path)
            .unwrap();
        let bytes = std::fs::read(&path).unwrap();
        // 即使传入超大的 max_dim，也按 MAX_PREVIEW_DIM 钳制后正常生成（小图不放大）
        let url = generate_image_preview_from_bytes(&bytes, u32::MAX).unwrap();
        assert!(url.starts_with("data:image/jpeg;base64,"));
    }

    // N001: `fs_read_image_preview` 内部经 `tokio::task::spawn_blocking` 解码缩略图，
    // 必须运行在 Tokio 运行时内（`futures::executor::block_on` 无 runtime 上下文会 panic）。
    #[tokio::test]
    async fn test_fs_read_image_preview_command() {
        let app = tauri::test::mock_app();
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("photo.png");
        image::RgbaImage::from_pixel(800, 600, image::Rgba([9, 8, 7, 255]))
            .save(&path)
            .unwrap();
        let url = fs_read_image_preview(
            app.handle().clone(),
            path.to_string_lossy().to_string(),
            256,
        )
        .await
        .unwrap();
        assert!(url.starts_with("data:image/jpeg;base64,"));
    }
}
