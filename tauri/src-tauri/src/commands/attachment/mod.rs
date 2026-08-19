//! File attachment commands — attach files to objects, with soft-delete support (§25.6)

#[cfg(target_os = "android")]
use crate::attachment_import_plugin::{AttachmentImportPluginHandle, OpenFilePayload};
use crate::commands::vault_handle;
use crate::state::AppState;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
#[cfg(target_os = "android")]
use tauri::Manager;
use tauri::{AppHandle, Runtime, State};

/// 构建允许的源/目标文件系统基目录白名单。
///
/// - `$SOLOSOUL_FS_BASE`（若设置）
/// - 用户 Desktop / Documents / Downloads
///
/// 组件级路径前缀判定（in_vault / in_attachments 共用纯函数）。
///
/// - `resolved`: canonicalize 结果（成功时为规范路径）；`raw`: 字面路径。
/// - `canonicalized`: canonicalize 是否成功。成功时**只**用 resolved 判定，杜绝字面
///   路径以共享前缀伪造（symlink 旁路）；失败时（Android symlink 兜底）用 raw 同时
///   比较 canonical 与非 canonical 两种 base 形式，覆盖 `/data/data` ↔ `/data/user/0`
///   双路径场景——raw 路径与 canonical base 前缀不同，仅比 canonical 会漏检。
/// - `base_canon`: canonical 形式的 base；`base_raw`: 非 canonical 形式（可为同一值）。
///   P003: 提升为 `pub(crate)` 供 `attachment_import_plugin.rs` 复用，统一组件级判定。
pub(crate) fn path_within_base(
    resolved: &Path,
    raw: &Path,
    canonicalized: bool,
    base_canon: &Path,
    base_raw: &Path,
) -> bool {
    if canonicalized {
        resolved.starts_with(base_canon)
    } else {
        // P018：canonicalize 失败兜底时无法安全解析 `..`（base 可能含 symlink，词法
        // 归一不可信），任何含 ParentDir 组件的原始路径一律拒绝——杜绝
        // `base/../../secret` 这类前几段命中 base 的 `..` 逃逸。
        if raw
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return false;
        }
        raw.starts_with(base_canon) || raw.starts_with(base_raw)
    }
}

/// - 移动端：应用缓存目录（前端经 plugin-fs 中转的落盘位置）
///
/// R2-14: 从 `attachment_copy_to_vault` 与 `attachment_download` 两处近乎逐字重复的
/// 内联块中提取，消除策略漂移风险（原一处含移动端 temp_dir 分支、一处不含）。
/// P001: 提升为 `pub(crate)` 供 `export_import/export.rs` 复用（导出落盘同白名单）。
pub(crate) fn allowed_fs_bases() -> Vec<PathBuf> {
    let mut bases = Vec::new();
    if let Ok(fs_base) = std::env::var("SOLOSOUL_FS_BASE") {
        if let Ok(canon) = PathBuf::from(fs_base).canonicalize() {
            bases.push(canon);
        }
    }
    #[cfg(unix)]
    let home_var = "HOME";
    #[cfg(windows)]
    let home_var = "USERPROFILE";
    if let Ok(home) = std::env::var(home_var) {
        for dir_name in &["Desktop", "Documents", "Downloads"] {
            let p = PathBuf::from(&home).join(dir_name);
            if let Ok(canon) = p.canonicalize() {
                bases.push(canon);
            }
        }
    }
    // 移动端：文件由前端通过 plugin-fs 中转后放在应用缓存目录，需加入白名单
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        bases.push(std::env::temp_dir());
    }
    bases
}

/// Collect all attachment IDs that are currently referenced in any object's __attachments.
/// P110: Uses existing `list_object_attachment_ids` batch method instead of N+1 load_object calls.
/// 仅供测试使用（唯一生产调用方 `attachment_cleanup_orphans` 命令已删除，P020）。
#[cfg(test)]
pub(crate) fn load_all_referenced_attachment_ids(
    vault: &solosoul_vault::VaultStore,
    account_id: &str,
) -> Result<std::collections::HashSet<String>, String> {
    let batch = vault.list_object_attachment_ids(account_id)?;
    let mut active_ids = std::collections::HashSet::new();
    for (_object_id, att_ids) in &batch {
        for id in att_ids {
            active_ids.insert(id.clone());
        }
    }
    Ok(active_ids)
}

/// Move a duplicate counter suffix before the file extension.
/// e.g. "a.pdf(1)" -> "a(1).pdf"; "a (1).pdf" -> "a(1).pdf"; "a(1)" -> "a(1)".
fn sanitize_duplicate_suffix(name: &str) -> String {
    // 找到最后一个 "(num)" 模式。
    let chars: Vec<char> = name.chars().collect();
    let mut last_open = None;
    let mut last_close = None;
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '(' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
            if j < chars.len() && chars[j] == ')' {
                last_open = Some(i);
                last_close = Some(j);
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }

    let (open, close) = match (last_open, last_close) {
        (Some(o), Some(c)) => (o, c),
        _ => return name.to_string(),
    };

    let num: String = chars[open + 1..close].iter().collect();
    let before: String = chars[..open].iter().collect();
    let after: String = chars[close + 1..].iter().collect();
    let before_trimmed = before.trim_end();
    let after_trimmed = after.trim_start();

    if after_trimmed.is_empty() {
        // 如 a.pdf(1)：把 before 末尾的扩展名移到序号之后
        if let Some(dot) = before_trimmed.rfind('.') {
            let ext = &before_trimmed[dot..];
            if ext.len() > 1 && ext[1..].chars().all(|c| c.is_alphanumeric()) {
                let base = before_trimmed[..dot].trim_end();
                return format!("{}({}){}", base, num, ext);
            }
        }
        format!("{}({})", before_trimmed, num)
    } else {
        // 如 a(1).pdf 或 a (1).pdf：after 就是扩展名
        format!("{}({}){}", before_trimmed, num, after_trimmed)
    }
}

/// If `dest` already exists, append an incrementing counter before the extension.
/// e.g. `a.pdf` -> `a(1).pdf`, `a(1).pdf` -> `a(2).pdf`.
fn make_unique_dest_path(dest: &Path) -> PathBuf {
    // 某些系统保存对话框遇到同名文件会自动把序号放在扩展名之后（如 a.pdf(1)），
    // 先修正为 a(1).pdf，再判断是否存在并递增。
    let corrected = if let Some(name) = dest.file_name().and_then(|s| s.to_str()) {
        let new_name = sanitize_duplicate_suffix(name);
        if new_name != name {
            dest.with_file_name(&new_name)
        } else {
            dest.to_path_buf()
        }
    } else {
        dest.to_path_buf()
    };

    if !corrected.exists() {
        return corrected;
    }
    let parent = corrected.parent().unwrap_or_else(|| Path::new(""));
    let stem = corrected
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let ext = corrected
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| format!(".{}", s))
        .unwrap_or_default();
    let mut n = 1;
    loop {
        let candidate = parent.join(format!("{}({}){}", stem, n, ext));
        if !candidate.exists() {
            return candidate;
        }
        n += 1;
    }
}

/// Download an attachment file to a user-chosen destination path.
/// Copies the file from vault storage to a destination path that is verified
/// to be within the user's allowed download area (desktop, documents, downloads,
/// or the SOLOSOUL_FS_BASE directory if set).
#[tauri::command]
pub async fn attachment_download(
    state: State<'_, AppState>,
    src_path: String,
    dest_path: String,
) -> Result<(), String> {
    // P007: 提前在块作用域内取 vault_base + 附件密钥并释放非 Send 的 vault_service guard，
    // 避免后续 spawn_blocking 的 await 跨 guard 存活。
    // P001: 附件在 vault 内加密存储，下载时解密复制（旧明文自动兼容）。
    let (vault_base, att_key) = {
        let svc = state
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        let key = svc
            .attachment_encryption_key()
            .map_err(|e| format!("无法获取附件密钥: {}", e))?;
        let key_arr: [u8; 32] = key
            .as_slice()
            .try_into()
            .map_err(|_| "附件密钥长度错误".to_string())?;
        (
            svc.base_path()
                .canonicalize()
                .map_err(|_| "Invalid vault base path".to_string())?,
            key_arr,
        )
    };

    // Security: ensure the source path is within vault storage.
    // 在 Android 上 /data/data/... 与 /data/user/0/... 可能互为 symlink，
    // canonicalize 失败但文件存在时保留原始路径做前缀比较。
    let (src, src_canonicalized) = std::path::Path::new(&src_path)
        .canonicalize()
        .map(|p| (p, true))
        .or_else(|_| {
            let p = std::path::PathBuf::from(&src_path);
            if p.exists() {
                Ok((p, false))
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "source path does not exist",
                ))
            }
        })
        .map_err(|e| format!("Invalid source path: {}", e))?;

    // R2-01: 拒绝 `..` 组件；回退分支改用组件级 Path::starts_with，
    // 避免共享前缀的兄弟目录（如 ~/.solosoul_evil/）绕过 in_vault 检查。
    let src_raw = std::path::Path::new(&src_path);
    if src_raw
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("Source path must not contain '..'".to_string());
    }

    let attachments_dir = vault_base.join("attachments");
    let attachments_canon = attachments_dir
        .canonicalize()
        .unwrap_or_else(|_| attachments_dir.clone());
    // R2-V8/X1: `src_raw`（字面路径）仅当 canonicalize 失败（Android symlink 兜底）时
    // 参与判定——成功时只用 canonicalize 结果，杜绝字面前缀绕过 symlink 旁路。
    let in_attachments = path_within_base(
        &src,
        src_raw,
        src_canonicalized,
        &attachments_canon,
        &attachments_dir,
    );
    let in_vault = path_within_base(&src, src_raw, src_canonicalized, &vault_base, &vault_base);

    if !in_attachments && !in_vault {
        return Err("Source path must be within vault storage".to_string());
    }

    // Security: validate dest_path is in an allowed download directory.
    // Reject path traversal components.
    let dest = std::path::Path::new(&dest_path);
    if dest
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("Destination path must not contain '..'".to_string());
    }

    // Determine allowed base directories for downloads.
    // P015: 白名单为空时 fail-closed 拒绝（而非放行任意路径）
    let allowed_bases = allowed_fs_bases();
    if allowed_bases.is_empty() {
        tracing::warn!("[attachment] allowed FS bases empty — rejecting download (fail-closed)");
        // N010-③: 中文 + 自救提示（与复制路径文案一致）。
        return Err(
            "允许的文件白名单为空（Desktop/Documents/Downloads 与 SOLOSOUL_FS_BASE 均不可解析）。请确认存在用户目录或设置 SOLOSOUL_FS_BASE 环境变量"
                .to_string(),
        );
    }

    {
        let dest_canon = if dest.exists() {
            dest.canonicalize()
                .map_err(|e| format!("Invalid destination: {}", e))?
        } else if let Some(parent) = dest.parent() {
            if parent.exists() {
                parent
                    .canonicalize()
                    .map_err(|_| "Cannot resolve destination parent".to_string())?
            } else {
                return Err("Destination parent directory does not exist".to_string());
            }
        } else {
            return Err("Invalid destination path".to_string());
        };

        let in_allowed_dir = allowed_bases.iter().any(|base| {
            if dest_canon.starts_with(base) {
                return true;
            }
            // Also allow the destination's parent directory itself to be an allowed dir
            if let Some(parent) = dest_canon.parent() {
                parent.starts_with(base)
            } else {
                false
            }
        });

        if !in_allowed_dir {
            // R004-①: 中文 + 自救提示（与复制路径文案一致，N010 漏改此处）。
            return Err(
                "目标位置必须在 Desktop、Documents、Downloads 或 SOLOSOUL_FS_BASE 目录内（如需其他位置，可设置 SOLOSOUL_FS_BASE 环境变量）"
                    .to_string(),
            );
        }
    }

    // Resolve duplicate file names: a.pdf -> a(1).pdf -> a(2).pdf
    let dest = make_unique_dest_path(dest);

    // P007: 建目录 + 大文件复制移入阻塞线程池，避免卡住 tokio worker
    // P001: 解密复制（源为 SOLC 密文则解密，旧明文直接复制）。
    let (src, dest) = (src.clone(), dest.to_path_buf());
    tauri::async_runtime::spawn_blocking(move || {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create destination directory: {}", e))?;
        }
        solosoul_core::attachment_crypto::copy_decrypt_file(&att_key, &src, &dest)
            .map_err(|e| format!("Failed to copy file: {}", e))?;
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| format!("Copy task panicked: {}", e))??;

    Ok(())
}

/// 解析并校验附件文件路径（`attachment_open` 与 `attachment_share` 共享的安全关键路径）。
///
/// 校验链：Vault 解锁 → 对象/附件存在 → 取 vault_path/src_path → canonicalize（失败时
/// Android symlink 兜底）→ 拒绝 `..` → `path_within_base` 组件级前缀校验。返回校验
/// 通过的 canonical 路径与附件元数据；错误路径仅保留脱敏日志（不含路径/文件名/对象 ID）。
fn resolve_verified_attachment_path(
    svc: &solosoul_core::vault_service::VaultService,
    object_id: &str,
    attachment_id: &str,
) -> Result<(PathBuf, AttachmentMeta), String> {
    let vault = svc
        .get_vault_store()
        .ok_or_else(|| "Vault not unlocked".to_string())?;

    let record = vault.load_object(object_id)?.ok_or("Object not found")?;
    let att = load_attachments(&record.properties)
        .into_iter()
        .find(|a| a.id == attachment_id)
        .ok_or("Attachment not found")?;

    let path_str = att
        .vault_path
        .as_ref()
        .or(att.src_path.as_ref())
        .ok_or("Attachment has no file path")?;

    let vault_base = svc
        .base_path()
        .canonicalize()
        .map_err(|_| "Invalid vault base path".to_string())?;
    let attachments_dir = vault_base.join("attachments");

    // R2-W1: 与 attachment_download 同款 src_canonicalized 模式——跟踪 canonicalize
    // 是否成功；字面路径仅在 canonicalize 失败（Android symlink 兜底）时参与判定，
    // 成功时只用 canonicalize 结果，杜绝字面前缀绕过 symlink 旁路。
    let (path, path_canonicalized) = Path::new(path_str)
        .canonicalize()
        .map(|p| (p, true))
        .or_else(|_| {
            let p = PathBuf::from(path_str);
            if p.exists() {
                Ok((p, false))
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "source path does not exist",
                ))
            }
        })
        .map_err(|e| {
            // 脱敏：不记录 path_str（可能含 vault 绝对路径）
            tracing::error!("attachment: failed to resolve attachment file: {}", e);
            format!("Cannot access attachment file: {}", e)
        })?;
    let attachments_canon = attachments_dir
        .canonicalize()
        .unwrap_or_else(|_| attachments_dir.clone());
    // R2-01: 与 attachment_download 一致——拒绝 `..`、组件级 starts_with，
    // 移除字符串前缀回退分支（共享前缀兄弟目录可绕过）。
    let path_raw = Path::new(path_str);
    if path_raw
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        tracing::error!("attachment: attachment path contains '..'");
        return Err("Attachment path must not contain '..'".to_string());
    }
    // R2-W1/X1: 字面路径仅在 canonicalize 失败时参与判定（同 download）。
    let in_vault = path_within_base(
        &path,
        path_raw,
        path_canonicalized,
        &attachments_canon,
        &attachments_dir,
    );
    if !in_vault {
        tracing::error!("attachment: attachment path is outside vault storage");
        return Err("Attachment path is outside vault storage".to_string());
    }

    Ok((path, att))
}

/// Open an attachment with the system's default application.
/// The path is resolved from the attachment metadata and verified to be inside
/// the vault's `attachments` directory before opening.
/// On Android, uses the native FileProvider plugin so that external PDF viewers
/// can read the app-private vault file.
#[tauri::command]
pub async fn attachment_open<R: Runtime>(
    #[allow(unused_variables)] app: AppHandle<R>,
    state: State<'_, AppState>,
    object_id: String,
    attachment_id: String,
) -> Result<(), String> {
    let (path, _att, att_key) = {
        let svc = state
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        let key = svc
            .attachment_encryption_key()
            .map_err(|e| format!("无法获取附件密钥: {}", e))?;
        let key_arr: [u8; 32] = key
            .as_slice()
            .try_into()
            .map_err(|_| "附件密钥长度错误".to_string())?;
        let (p, a) = resolve_verified_attachment_path(&svc, &object_id, &attachment_id)?;
        (p, a, key_arr)
    };

    #[cfg(target_os = "android")]
    {
        // P001: 外部应用无法读取 vault 密文——先解密到临时明文再交给 FileProvider。
        let temp_dir = std::env::temp_dir().join(format!("solosoul_open_{}", object_id));
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| format!("Failed to prepare open dir: {}", e))?;
        let safe_name = solosoul_core::path_util::sanitize_file_name(&_att.file_name)?;
        let temp_path = temp_dir.join(&safe_name);
        solosoul_core::attachment_crypto::copy_decrypt_file(&att_key, &path, &temp_path)
            .map_err(|e| format!("Failed to decrypt for open: {}", e))?;
        let handle = app.state::<AttachmentImportPluginHandle<R>>();
        handle.open_file(OpenFilePayload {
            path: temp_path.to_string_lossy().to_string(),
            mime_type: _att.mime_type.clone(),
        })
    }

    #[cfg(not(target_os = "android"))]
    {
        // P001: 解密到临时明文再交给系统默认应用（外部应用无法读取 vault 密文）。
        let temp_dir = std::env::temp_dir().join(format!("solosoul_open_{}", object_id));
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| format!("Failed to prepare open dir: {}", e))?;
        let safe_name = solosoul_core::path_util::sanitize_file_name(&_att.file_name)?;
        let temp_path = temp_dir.join(&safe_name);
        solosoul_core::attachment_crypto::copy_decrypt_file(&att_key, &path, &temp_path)
            .map_err(|e| format!("Failed to decrypt for open: {}", e))?;
        opener::open(&temp_path).map_err(|e| format!("Failed to open file: {}", e))?;
        Ok(())
    }
}

// ── Sub-modules（P047 拆分）──────────────────────────────────
// 命令子模块用 `pub mod`：tauri 宏（generate_handler）要求命令的 `__cmd__xxx`
// 辅助符号与命令定义同模块——lib.rs 以 `attachment::crud::attachment_list`、
// `attachment::tree::attachment_list_all`、`attachment::share::attachment_share`
// 定义处路径注册（re-export 不携带辅助符号）。
pub mod crud;
pub mod share;
#[cfg(test)]
mod tests;
pub mod tree;

// 保持既有模块路径 `commands::attachment::xxx` 对外可用：
// - crud.rs：AttachmentMeta（export_import 以 `super::attachment::AttachmentMeta` 引用）、
//   attachment_dir / path_within_base / allowed_fs_bases（attachment_import_plugin 引用）
// - tree.rs：附件树类型
// - share.rs：attachment_share 命令（lib.rs 注册）
pub use crud::AttachmentMeta;
pub use share::attachment_share;
pub use tree::{
    attachment_list_all, AttachmentListAllResult, AttachmentTreeObject, AttachmentTreePage,
};
// 子模块与外部 crate 内引用：attachment_dir（attachment_import_plugin 用
// `crate::commands::attachment::{attachment_dir, path_within_base}`）、
// 元数据工具（tree/share 与 mod.rs 的 resolve_verified_attachment_path 共用）。
pub(crate) use crud::{attachment_dir, load_attachments};
// 测试模块（attachment::tests）经 `use super::*` 使用 save_attachments。
#[cfg(test)]
pub(crate) use crud::save_attachments;
