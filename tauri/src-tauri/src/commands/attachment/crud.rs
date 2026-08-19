//! 附件 CRUD 命令 + 元数据模型（P047 拆分）。
//! 依赖父模块的路径安全工具（path_within_base / allowed_fs_bases）与 `use super::*`。

use super::{allowed_fs_bases, path_within_base};
use crate::commands::vault_handle;
use crate::state::AppState;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::State;

/// 单个对象最多允许的活跃附件数量。
const MAX_ACTIVE_ATTACHMENTS: usize = 200;

/// 单个附件最多允许的标签数量（超出部分丢弃）。
const MAX_ATTACHMENT_TAGS: usize = 20;

/// 单个标签的最大字符数（超出部分截断）。
const MAX_ATTACHMENT_TAG_LEN: usize = 30;

/// 单个附件描述的最大字符数（超出部分截断）。
const MAX_ATTACHMENT_DESCRIPTION_LEN: usize = 500;

/// P007: 附件元数据唯一来源 = solosoul-core `export_import::AttachmentMeta`
/// （camelCase 序列化契约单一维护，GUI 与 CLI 导入导出不再双定义漂移）。
pub use solosoul_core::export_import::AttachmentMeta;

/// 附件 ID 与对象 ID 允许使用的字符集，防止路径遍历。
fn validate_attachment_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.len() > 64
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!("Invalid attachment id: {}", id));
    }
    Ok(())
}

pub(crate) fn attachment_dir(
    base: &Path,
    object_id: &str,
    attachment_id: &str,
) -> Result<PathBuf, String> {
    validate_attachment_id(object_id)?;
    validate_attachment_id(attachment_id)?;
    Ok(base.join("attachments").join(object_id).join(attachment_id))
}

pub(crate) fn load_attachments(props: &serde_json::Value) -> Vec<AttachmentMeta> {
    props
        .get("__attachments")
        .and_then(|v| serde_json::from_value::<Vec<AttachmentMeta>>(v.clone()).ok())
        .unwrap_or_default()
}

pub(crate) fn save_attachments(props: &mut serde_json::Value, atts: &[AttachmentMeta]) {
    if let serde_json::Value::Object(ref mut obj) = props {
        obj.insert(
            "__attachments".to_string(),
            serde_json::to_value(atts).unwrap_or_default(),
        );
    }
}

#[tauri::command]
pub async fn attachment_list(
    state: State<'_, AppState>,
    object_id: String,
    show_deleted: Option<bool>,
) -> Result<Vec<AttachmentMeta>, String> {
    let show = show_deleted.unwrap_or(false);
    let vault = vault_handle(&state)?;
    match vault.load_object(&object_id)? {
        Some(rec) => Ok(load_attachments(&rec.properties)
            .into_iter()
            .filter(|a| {
                if show {
                    a.deleted_at.is_some()
                } else {
                    a.deleted_at.is_none()
                }
            })
            .collect()),
        None => Ok(vec![]),
    }
}

/// Physical delete (permanent — removes metadata + deletes file from disk)
#[tauri::command]
pub async fn attachment_delete(
    state: State<'_, AppState>,
    object_id: String,
    attachment_id: String,
) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let vault = svc
        .get_vault_store()
        .ok_or_else(|| "Vault not unlocked".to_string())?;
    let mut record = vault.load_object(&object_id)?.ok_or("Object not found")?;
    let atts: Vec<AttachmentMeta> = load_attachments(&record.properties)
        .into_iter()
        .filter(|a: &AttachmentMeta| a.id != attachment_id)
        .collect();

    // Also delete the physical file from disk — NotFound 容错（与 batch 版对齐）
    let attachments_dir = attachment_dir(svc.base_path(), &object_id, &attachment_id)?;
    let _ = std::fs::remove_dir_all(&attachments_dir);

    save_attachments(&mut record.properties, &atts);
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record)?;
    state.auto_sync.trigger_debounce();
    state.device_auto_sync.trigger_data_change();
    Ok(())
}

#[tauri::command]
pub async fn attachment_restore(
    state: State<'_, AppState>,
    object_id: String,
    attachment_id: String,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let mut record = vault.load_object(&object_id)?.ok_or("Object not found")?;
    let mut atts = load_attachments(&record.properties);
    if let Some(a) = atts.iter_mut().find(|a| a.id == attachment_id) {
        a.deleted_at = None;
    }
    save_attachments(&mut record.properties, &atts);
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record)?;
    state.auto_sync.trigger_debounce();
    state.device_auto_sync.trigger_data_change();
    Ok(())
}

#[tauri::command]
pub async fn attachment_save(
    state: State<'_, AppState>,
    object_id: String,
    meta: AttachmentMeta,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let mut record = vault.load_object(&object_id)?.ok_or("Object not found")?;
    let mut atts = load_attachments(&record.properties);
    // §10.4.4: maximum active attachments per object
    let active_count = atts.iter().filter(|a| a.deleted_at.is_none()).count();
    if active_count >= MAX_ACTIVE_ATTACHMENTS {
        return Err(format!(
            "Maximum {} active attachments per object reached",
            MAX_ACTIVE_ATTACHMENTS
        ));
    }
    atts.push(meta);
    save_attachments(&mut record.properties, &atts);
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record)?;
    state.auto_sync.trigger_debounce();
    state.device_auto_sync.trigger_data_change();
    Ok(())
}

#[tauri::command]
pub async fn attachment_rename(
    state: State<'_, AppState>,
    object_id: String,
    attachment_id: String,
    new_name: String,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let mut record = vault.load_object(&object_id)?.ok_or("Object not found")?;
    let mut atts = load_attachments(&record.properties);
    if let Some(a) = atts.iter_mut().find(|a| a.id == attachment_id) {
        // P207: sanitize file_name to prevent path traversal
        let safe_name = std::path::Path::new(&new_name)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or(new_name.clone());
        a.file_name = safe_name;
    }
    save_attachments(&mut record.properties, &atts);
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record)?;
    state.auto_sync.trigger_debounce();
    state.device_auto_sync.trigger_data_change();
    Ok(())
}

/// 更新附件元数据（描述 + 标签）。
///
/// - `description`: `None` 表示不修改；`Some` 时按「空串清除」语义处理（trim 后
///   为空则置 None），超长截断。
/// - `tags`: `None` 表示不修改；`Some` 时整体替换（逐项 trim、去空、去重、限长）。
///
/// 与 `attachment_rename` 同构：加载对象 → 定位附件 → 修改 → 落盘 + 触发同步。
#[tauri::command]
pub async fn attachment_update_meta(
    state: State<'_, AppState>,
    object_id: String,
    attachment_id: String,
    description: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let mut record = vault.load_object(&object_id)?.ok_or("Object not found")?;
    let mut atts = load_attachments(&record.properties);
    let att = atts
        .iter_mut()
        .find(|a| a.id == attachment_id)
        .ok_or("Attachment not found")?;
    if let Some(desc) = description {
        let trimmed = desc.trim();
        if trimmed.is_empty() {
            att.description = None;
        } else {
            // 字符级截断：String::truncate 按字节截断，多字节 UTF-8（中文等）落在字符
            // 中间会 panic——用 chars().take() 保证字符边界安全
            att.description = Some(
                trimmed
                    .chars()
                    .take(MAX_ATTACHMENT_DESCRIPTION_LEN)
                    .collect(),
            );
        }
    }
    if let Some(tags) = tags {
        // 与前端一致：大小写不敏感去重 + 逐标签长度上限（均字符级，UTF-8 安全）
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let cleaned: Vec<String> = tags
            .into_iter()
            .filter_map(|s| {
                let trimmed = s.trim().to_string();
                let normalized = trimmed.to_lowercase();
                if trimmed.is_empty() || !seen.insert(normalized) {
                    None
                } else {
                    Some(trimmed.chars().take(MAX_ATTACHMENT_TAG_LEN).collect())
                }
            })
            .take(MAX_ATTACHMENT_TAGS)
            .collect();
        att.tags = cleaned;
    }
    save_attachments(&mut record.properties, &atts);
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record)?;
    state.auto_sync.trigger_debounce();
    state.device_auto_sync.trigger_data_change();
    Ok(())
}

#[tauri::command]
pub async fn attachment_soft_delete(
    state: State<'_, AppState>,
    object_id: String,
    attachment_id: String,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let mut record = vault.load_object(&object_id)?.ok_or("Object not found")?;
    let mut atts = load_attachments(&record.properties);
    if let Some(a) = atts.iter_mut().find(|a| a.id == attachment_id) {
        a.deleted_at = Some(chrono::Utc::now().to_rfc3339());
    }

    save_attachments(&mut record.properties, &atts);
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record)?;
    state.auto_sync.trigger_debounce();
    state.device_auto_sync.trigger_data_change();
    Ok(())
}

/// Batch soft-delete multiple attachments on the same object in a single transaction.
/// Significantly faster than N sequential `attachment_soft_delete` calls.
#[tauri::command]
pub async fn attachment_batch_soft_delete(
    state: State<'_, AppState>,
    object_id: String,
    attachment_ids: Vec<String>,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let mut record = vault.load_object(&object_id)?.ok_or("Object not found")?;
    let mut atts = load_attachments(&record.properties);
    let ids_set: std::collections::HashSet<&str> =
        attachment_ids.iter().map(|s| s.as_str()).collect();
    let now = chrono::Utc::now().to_rfc3339();
    for att in atts.iter_mut().filter(|a| ids_set.contains(a.id.as_str())) {
        att.deleted_at = Some(now.clone());
    }
    save_attachments(&mut record.properties, &atts);
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record)?;
    state.auto_sync.trigger_debounce();
    state.device_auto_sync.trigger_data_change();
    Ok(())
}

/// Batch restore multiple soft-deleted attachments on the same object in a single transaction.
/// Significantly faster than N sequential `attachment_restore` calls.
#[tauri::command]
pub async fn attachment_batch_restore(
    state: State<'_, AppState>,
    object_id: String,
    attachment_ids: Vec<String>,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let mut record = vault.load_object(&object_id)?.ok_or("Object not found")?;
    let mut atts = load_attachments(&record.properties);
    let ids_set: std::collections::HashSet<&str> =
        attachment_ids.iter().map(|s| s.as_str()).collect();
    for att in atts.iter_mut().filter(|a| ids_set.contains(a.id.as_str())) {
        att.deleted_at = None;
    }
    save_attachments(&mut record.properties, &atts);
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record)?;
    state.auto_sync.trigger_debounce();
    state.device_auto_sync.trigger_data_change();
    Ok(())
}

/// Batch permanent-delete multiple attachments on the same object in a single transaction.
/// Removes metadata entries AND deletes physical files from disk.
#[tauri::command]
pub async fn attachment_batch_delete(
    state: State<'_, AppState>,
    object_id: String,
    attachment_ids: Vec<String>,
) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let vault = svc
        .get_vault_store()
        .ok_or_else(|| "Vault not unlocked".to_string())?;
    let mut record = vault.load_object(&object_id)?.ok_or("Object not found")?;
    let ids_set: std::collections::HashSet<&str> =
        attachment_ids.iter().map(|s| s.as_str()).collect();

    // Collect physical paths before removing metadata
    let paths_to_remove: Vec<std::path::PathBuf> = load_attachments(&record.properties)
        .iter()
        .filter(|a| ids_set.contains(a.id.as_str()))
        .filter_map(|a| attachment_dir(svc.base_path(), &object_id, &a.id).ok())
        .collect();

    let atts: Vec<AttachmentMeta> = load_attachments(&record.properties)
        .into_iter()
        .filter(|a| !ids_set.contains(a.id.as_str()))
        .collect();

    // Delete physical files
    for dir in &paths_to_remove {
        let _ = std::fs::remove_dir_all(dir);
    }

    save_attachments(&mut record.properties, &atts);
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record)?;
    state.auto_sync.trigger_debounce();
    state.device_auto_sync.trigger_data_change();
    Ok(())
}

#[tauri::command]
pub async fn attachment_count_batch(
    state: State<'_, AppState>,
    object_ids: Vec<String>,
) -> Result<HashMap<String, usize>, String> {
    let vault = vault_handle(&state)?;
    // P110: Use batch load instead of N+1 individual load_object calls
    let objects = vault.load_objects_batch(&object_ids)?;
    let mut result = HashMap::new();
    for (id, rec) in &objects {
        let count = load_attachments(&rec.properties)
            .iter()
            .filter(|a| a.deleted_at.is_none())
            .count();
        result.insert(id.clone(), count);
    }
    Ok(result)
}

/// Copy a file into vault-managed attachment storage.
/// Returns the vault path that should be stored as `vault_path` on the attachment meta.
///
/// # Security
/// - `src_path` is canonicalized to resolve relative path traversal (`../`).
/// - Source path must NOT be inside vault storage itself (prevents self-referencing).
/// - `file_name` is sanitized to only the final path component.
#[tauri::command]
pub async fn attachment_copy_to_vault(
    state: State<'_, AppState>,
    src_path: String,
    object_id: String,
    attachment_id: String,
    file_name: String,
) -> Result<String, String> {
    // P007: 提前在块作用域内取 base + 附件密钥并释放非 Send 的 vault_service guard，
    // 避免后续 spawn_blocking 的 await 跨 guard 存活。
    // P001: 附件以加密形式落盘（at-rest），复制时用附件密钥加密写入。
    let (base, att_key) = {
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
        (svc.base_path().clone(), key_arr)
    };

    // Canonicalize src_path to resolve relative path traversal.
    // 在 Android 上文件选择器可能返回缓存路径（或 content:// URI，已由前端中转为本地路径），
    // 如果 canonicalize 失败但路径确实存在，则降级使用原始路径。
    // R2-W1: 与 attachment_download 同款 src_canonicalized 模式——canonicalize 成功
    // 时仅用 canonicalize 结果判定；字面路径仅在 Android symlink 兜底（canonicalize
    // 失败但文件存在）时参与，统一 raw/canonical 混用。
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

    // Reject if source path is within vault storage (self-referencing)
    // Canonicalize the vault base to match src.canonicalize() symlink resolution.
    let vault_base = base
        .canonicalize()
        .map_err(|_| "Invalid vault base path".to_string())?;
    let src_raw = std::path::Path::new(&src_path);
    // P014: 与 attachment_download 对齐，入口拒绝 `..` 组件——兜底分支（canonicalize
    // 失败但文件存在）用字面路径做 `starts_with` 前缀判定，`..` 组件可让字面前缀
    // 通过检查却解析到白名单外（Android symlink 场景可达）。
    if src_raw
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("Source path must not contain '..'".to_string());
    }
    // R2-X1: 拒绝型判定——canonicalize 失败（Android symlink 兜底）时，raw 路径须同时
    // 与非 canonical 的 `base` 比较：/data/data 与 /data/user/0 互为 symlink 的双路径场景中，
    // raw 路径与 canonical vault_base 前缀不同，只比 vault_base 会漏检库内自引用。
    let in_vault = path_within_base(&src, src_raw, src_canonicalized, &vault_base, &base);
    if in_vault {
        return Err("Source path must not be inside vault storage".to_string());
    }

    // 验证源文件在允许的用户目录内（Desktop/Documents/Downloads 或 SOLOSOUL_FS_BASE）
    // P015: 白名单为空时 fail-closed 拒绝（而非放行任意路径）
    let allowed_bases = allowed_fs_bases();
    if allowed_bases.is_empty() {
        tracing::warn!("[attachment] allowed FS bases empty — rejecting copy (fail-closed)");
        // N010-③: 中文 + 自救提示（P015：极简 Linux 无 Desktop/Documents/Downloads 时
        // 告知可设置 SOLOSOUL_FS_BASE 白名单目录）。
        return Err(
            "允许的文件白名单为空（Desktop/Documents/Downloads 与 SOLOSOUL_FS_BASE 均不可解析）。请确认存在用户目录或设置 SOLOSOUL_FS_BASE 环境变量"
                .to_string(),
        );
    }
    if !allowed_bases.iter().any(|b| src.starts_with(b)) {
        return Err(
            "源文件必须在 Desktop、Documents、Downloads 或 SOLOSOUL_FS_BASE 目录内（如需其他位置，可设置 SOLOSOUL_FS_BASE 环境变量）"
                .to_string(),
        );
    }

    let dest_dir = attachment_dir(&base, &object_id, &attachment_id)?;
    std::fs::create_dir_all(&dest_dir).map_err(|e| format!("Mkdir: {}", e))?;

    // R007 + P023: sanitize file_name to prevent path traversal.
    // P023: 统一走 solosoul_core::path_util::sanitize_file_name（平台无关拒绝
    // `/` `\\` 分隔符 + 取末段兜底 + 拒绝空/`.`/`..`），修复旧实现仅取末段
    // 无法剥离 `..\\..\\evil.txt` 反斜杠的强度缺口。
    let safe_name = solosoul_core::path_util::sanitize_file_name(&file_name)?;
    let dest_path = dest_dir.join(&safe_name);
    let dest_path_str = dest_path.to_string_lossy().to_string();
    // P007: 大文件复制移入阻塞线程池，避免卡住 tokio worker（路径校验已在上方完成）
    // P001: 加密写入（encrypt_chunked_stream，SOLC 头）——附件不再明文落盘。
    let (src, dest_path) = (src.clone(), dest_path);
    tauri::async_runtime::spawn_blocking(move || {
        solosoul_core::attachment_crypto::encrypt_file_stream(&att_key, &src, &dest_path)
            .map_err(|e| format!("Encrypt copy: {}", e))
    })
    .await
    .map_err(|e| format!("Copy task panicked: {}", e))??;
    Ok(dest_path_str)
}
