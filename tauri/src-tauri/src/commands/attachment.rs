//! File attachment commands — attach files to objects, with soft-delete support (§25.6)

#[cfg(target_os = "android")]
use crate::attachment_import_plugin::{AttachmentImportPluginHandle, OpenFilePayload};
use crate::commands::vault_handle;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
#[cfg(target_os = "android")]
use tauri::Manager;
use tauri::{AppHandle, Runtime, State};

/// 单个对象最多允许的活跃附件数量。
const MAX_ACTIVE_ATTACHMENTS: usize = 200;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentMeta {
    pub id: String,
    pub object_id: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<String>,
    /// Original source path (transient, only set at upload time)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub src_path: Option<String>,
    /// Vault storage path (persistent, survives original file deletion)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault_path: Option<String>,
}

fn load_attachments(props: &Value) -> Vec<AttachmentMeta> {
    props
        .get("__attachments")
        .and_then(|v| serde_json::from_value::<Vec<AttachmentMeta>>(v.clone()).ok())
        .unwrap_or_default()
}

fn save_attachments(props: &mut Value, atts: &[AttachmentMeta]) {
    if let Value::Object(ref mut obj) = props {
        obj.insert(
            "__attachments".to_string(),
            serde_json::to_value(atts).unwrap_or_default(),
        );
    }
}

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
/// P003: 提升为 `pub(crate)` 供 `attachment_import_plugin.rs` 复用，统一组件级判定。
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

    // Also delete the physical file from disk
    let attachments_dir = attachment_dir(svc.base_path(), &object_id, &attachment_id)?;
    std::fs::remove_dir_all(&attachments_dir)
        .map_err(|e| format!("Failed to delete attachment file: {}", e))?;

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
    // P007: 提前在块作用域内取 base 并释放非 Send 的 vault_service guard，
    // 避免后续 spawn_blocking 的 await 跨 guard 存活。
    let base = {
        let svc = state
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        svc.base_path().clone()
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
    // R2-X1: 拒绝型判定——canonicalize 失败（Android symlink 兜底）时，raw 路径须同时
    // 与非 canonical 的 `base` 比较：/data/data 与 /data/user/0 互为 symlink 的双路径场景中，
    // raw 路径与 canonical vault_base 前缀不同，只比 vault_base 会漏检库内自引用。
    let in_vault = path_within_base(&src, src_raw, src_canonicalized, &vault_base, &base);
    if in_vault {
        return Err("Source path must not be inside vault storage".to_string());
    }

    // 验证源文件在允许的用户目录内（Desktop/Documents/Downloads 或 SOLOSOUL_FS_BASE）
    let allowed_bases = allowed_fs_bases();
    if !allowed_bases.is_empty() && !allowed_bases.iter().any(|b| src.starts_with(b)) {
        return Err(
            "Source path must be within Desktop, Documents, Downloads, or SOLOSOUL_FS_BASE"
                .to_string(),
        );
    }

    let dest_dir = attachment_dir(&base, &object_id, &attachment_id)?;
    std::fs::create_dir_all(&dest_dir).map_err(|e| format!("Mkdir: {}", e))?;

    // R007: sanitize file_name to prevent path traversal; only the final component is allowed.
    let safe_name = std::path::Path::new(&file_name)
        .file_name()
        .ok_or("Invalid file name")?
        .to_string_lossy()
        .to_string();
    let dest_path = dest_dir.join(&safe_name);
    let dest_path_str = dest_path.to_string_lossy().to_string();
    // P007: 大文件复制移入阻塞线程池，避免卡住 tokio worker（路径校验已在上方完成）
    let (src, dest_path) = (src.clone(), dest_path);
    tauri::async_runtime::spawn_blocking(move || {
        std::fs::copy(&src, &dest_path).map_err(|e| format!("Copy: {}", e))
    })
    .await
    .map_err(|e| format!("Copy task panicked: {}", e))??;
    Ok(dest_path_str)
}

/// Collect all attachment IDs that are currently referenced in any object's __attachments.
/// P110: Uses existing `list_object_attachment_ids` batch method instead of N+1 load_object calls.
/// 仅供测试使用（唯一生产调用方 `attachment_cleanup_orphans` 命令已删除，P020）。
#[cfg(test)]
fn load_all_referenced_attachment_ids(
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

// ── Types for global attachment tree ────────────────────────────

/// One object in the attachment tree, containing its attachments.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentTreeObject {
    pub object_id: String,
    pub object_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<AttachmentMeta>,
}

/// One page (section type or custom page) in the attachment tree.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentTreePage {
    #[serde(default)]
    pub page_id: Option<String>,
    pub page_name: String,
    #[serde(default)]
    pub page_icon: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objects: Vec<AttachmentTreeObject>,
}

/// Result of listing all attachments across all objects, grouped by page.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentListAllResult {
    /// Pages with active (non-deleted) attachments.
    pub pages: Vec<AttachmentTreePage>,
    /// Pages with deleted attachments (for trash view).
    pub trash_pages: Vec<AttachmentTreePage>,
}

/// List all attachments across all objects, grouped by page.
/// Custom pages use parent_id to find child objects;
/// remaining objects are grouped by section_type (built-in sections).
#[tauri::command]
pub async fn attachment_list_all(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<AttachmentListAllResult, String> {
    let vault = vault_handle(&state)?;
    // P112: 单次 list_objects 已解密全部 properties 并返回 summary.properties，
    // 下方直接复用，不再 load_objects_batch / 逐页 list_objects 重复解密。
    // P114: 全表 AES 解密 + 附件树构建移入 spawn_blocking，避免阻塞 tokio worker。
    tokio::task::spawn_blocking(move || {
        let objects = vault.list_objects(&account_id, None, None, None, false, false)?;

        // Separate page objects from other objects
        let (page_objects, section_groups, children_by_parent) =
            group_objects_for_attachment_tree(&objects);

        let pages = build_attachment_tree_pages(
            &vault,
            &page_objects,
            &section_groups,
            &children_by_parent,
            false,
        )?;
        let trash_pages = build_attachment_tree_pages(
            &vault,
            &page_objects,
            &section_groups,
            &children_by_parent,
            true,
        )?;

        Ok(AttachmentListAllResult { pages, trash_pages })
    })
    .await
    .map_err(|e| format!("attachment_list_all task failed: {e}"))?
}

/// P112: 附件树分组结果——页面对象、按 section_type 分组的内置区段对象、
/// 按 parent_id 分组的子对象（单次 list_objects 解密后一次成型，替代每页面 N+1 次查询）。
type AttachmentTreeGroups = (
    Vec<solosoul_vault::ObjectSummary>,
    std::collections::BTreeMap<String, Vec<solosoul_vault::ObjectSummary>>,
    HashMap<String, Vec<solosoul_vault::ObjectSummary>>,
);

/// P112: 单次 list_objects 已解密全部 properties，这里按 parent_id 一次性预分组子对象
/// （替代每页面 N+1 次解密查询），并分离页面对象与按 section_type 分组的内置区段对象。
fn group_objects_for_attachment_tree(
    objects: &[solosoul_vault::ObjectSummary],
) -> AttachmentTreeGroups {
    let mut page_objects: Vec<solosoul_vault::ObjectSummary> = Vec::new();
    let mut section_groups: std::collections::BTreeMap<String, Vec<solosoul_vault::ObjectSummary>> =
        std::collections::BTreeMap::new();
    let mut children_by_parent: HashMap<String, Vec<solosoul_vault::ObjectSummary>> =
        HashMap::new();

    for obj in objects {
        if obj.collection_type == "page" {
            page_objects.push(obj.clone());
        } else {
            section_groups
                .entry(obj.section_type.clone())
                .or_default()
                .push(obj.clone());
        }
        if let Some(pid) = &obj.parent_id {
            children_by_parent
                .entry(pid.clone())
                .or_default()
                .push(obj.clone());
        }
    }

    (page_objects, section_groups, children_by_parent)
}

/// Build attachment tree pages for a given filter (active vs trash).
/// P112: 直接复用已解密的 `summary.properties` 解析附件（不再 load_objects_batch 重复解密）；
/// 子对象由调用方按 parent_id 一次性预分组传入（不再每页面 N+1 次解密查询）。
fn build_attachment_tree_pages(
    vault: &solosoul_vault::VaultStore,
    page_objects: &[solosoul_vault::ObjectSummary],
    section_groups: &std::collections::BTreeMap<String, Vec<solosoul_vault::ObjectSummary>>,
    children_by_parent: &HashMap<String, Vec<solosoul_vault::ObjectSummary>>,
    only_deleted: bool,
) -> Result<Vec<AttachmentTreePage>, String> {
    let template_cache: std::cell::RefCell<std::collections::HashMap<String, Option<String>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
    let build_objects_with_attachments = |objs: &[solosoul_vault::ObjectSummary],
                                          only_del: bool|
     -> Vec<AttachmentTreeObject> {
        objs.iter()
            .filter_map(|summary| {
                let all_atts = load_attachments(&summary.properties);
                let filtered: Vec<AttachmentMeta> = all_atts
                    .into_iter()
                    .filter(|a| {
                        if only_del {
                            a.deleted_at.is_some()
                        } else {
                            a.deleted_at.is_none()
                        }
                    })
                    .collect();
                if filtered.is_empty() {
                    None
                } else {
                    let template_name = summary.template_id.as_ref().and_then(|tid| {
                        let mut cache = template_cache.borrow_mut();
                        cache.get(tid).cloned().unwrap_or_else(|| {
                            let name = vault.load_user_template(tid).ok().flatten().map(|t| t.name);
                            cache.insert(tid.clone(), name.clone());
                            name
                        })
                    });
                    Some(AttachmentTreeObject {
                        object_id: summary.id.clone(),
                        object_name: summary.name.clone(),
                        template_name,
                        attachments: filtered,
                    })
                }
            })
            .collect()
    };

    let mut pages: Vec<AttachmentTreePage> = Vec::new();
    let mut child_ids_assigned: std::collections::HashSet<String> =
        std::collections::HashSet::new();

    // For custom pages: find children via pre-grouped parent map
    for page_obj in page_objects {
        let children = children_by_parent
            .get(&page_obj.id)
            .cloned()
            .unwrap_or_default();
        for child in &children {
            child_ids_assigned.insert(child.id.clone());
        }
        let objects_with_attachments = build_objects_with_attachments(&children, only_deleted);
        if !objects_with_attachments.is_empty() {
            pages.push(AttachmentTreePage {
                page_id: Some(page_obj.id.clone()),
                page_name: page_obj.name.clone(),
                page_icon: Some(page_obj.icon_name.clone()),
                objects: objects_with_attachments,
            });
        }
    }

    // For remaining objects: group by section_type (built-in sections)
    for (section, objs) in section_groups {
        let unassigned: Vec<_> = objs
            .iter()
            .filter(|o| !child_ids_assigned.contains(&o.id))
            .cloned()
            .collect();
        if unassigned.is_empty() {
            continue;
        }
        let objects_with_attachments = build_objects_with_attachments(&unassigned, only_deleted);
        if !objects_with_attachments.is_empty() {
            pages.push(AttachmentTreePage {
                page_id: None,
                page_name: section.clone(),
                page_icon: Some(section.clone()),
                objects: objects_with_attachments,
            });
        }
    }

    Ok(pages)
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
    // P007: 提前在块作用域内取 vault_base 并释放非 Send 的 vault_service guard，
    // 避免后续 spawn_blocking 的 await 跨 guard 存活。
    let vault_base = {
        let svc = state
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        svc.base_path()
            .canonicalize()
            .map_err(|_| "Invalid vault base path".to_string())?
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
    let allowed_bases = allowed_fs_bases();

    // If we have allowed bases, verify dest is within one of them.
    if !allowed_bases.is_empty() {
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
            return Err(
                "Destination must be within Desktop, Documents, Downloads, or SOLOSOUL_FS_BASE"
                    .to_string(),
            );
        }
    }

    // Resolve duplicate file names: a.pdf -> a(1).pdf -> a(2).pdf
    let dest = make_unique_dest_path(dest);

    // P007: 建目录 + 大文件复制移入阻塞线程池，避免卡住 tokio worker
    let (src, dest) = (src.clone(), dest.to_path_buf());
    tauri::async_runtime::spawn_blocking(move || {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create destination directory: {}", e))?;
        }
        std::fs::copy(&src, &dest).map_err(|e| format!("Failed to copy file: {}", e))?;
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| format!("Copy task panicked: {}", e))??;

    Ok(())
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
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let vault = svc
        .get_vault_store()
        .ok_or_else(|| "Vault not unlocked".to_string())?;

    let record = vault.load_object(&object_id)?.ok_or("Object not found")?;
    let att = load_attachments(&record.properties)
        .into_iter()
        .find(|a| a.id == attachment_id)
        .ok_or("Attachment not found")?;

    let path_str = att
        .vault_path
        .as_ref()
        .or(att.src_path.as_ref())
        .ok_or("Attachment has no file path")?;

    // P203: 移除残留调试日志——此前以 error! 记录完整 vault 路径/object_id/mime，
    // 属敏感数据泄漏面；错误路径仅保留脱敏日志（不含路径/文件名/对象 ID）。
    let vault_base = svc
        .base_path()
        .canonicalize()
        .map_err(|_| "Invalid vault base path".to_string())?;
    let attachments_dir = vault_base.join("attachments");

    // R2-W1: 与 attachment_download 同款 src_canonicalized 模式——跟踪 canonicalize
    // 是否成功；字面路径仅在 canonicalize 失败（Android symlink 兜底）时参与判定，
    // 成功时只用 canonicalize 结果，杜绝字面前缀绕过 symlink 旁路。
    let (path, path_canonicalized) = std::path::Path::new(path_str)
        .canonicalize()
        .map(|p| (p, true))
        .or_else(|_| {
            let p = std::path::PathBuf::from(path_str);
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
            tracing::error!("attachment_open: failed to resolve attachment file: {}", e);
            format!("Cannot access attachment file: {}", e)
        })?;
    let attachments_canon = attachments_dir
        .canonicalize()
        .unwrap_or_else(|_| attachments_dir.clone());
    // R2-01: 与 attachment_download 一致——拒绝 `..`、组件级 starts_with，
    // 移除字符串前缀回退分支（共享前缀兄弟目录可绕过）。
    let path_raw = std::path::Path::new(path_str);
    if path_raw
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        tracing::error!("attachment_open: attachment path contains '..'");
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
        tracing::error!("attachment_open: attachment path is outside vault storage");
        return Err("Attachment path is outside vault storage".to_string());
    }

    #[cfg(target_os = "android")]
    {
        let handle = app.state::<AttachmentImportPluginHandle<R>>();
        handle.open_file(OpenFilePayload {
            path: path.to_string_lossy().to_string(),
            mime_type: att.mime_type.clone(),
        })
    }

    #[cfg(not(target_os = "android"))]
    {
        opener::open(&path).map_err(|e| format!("Failed to open file: {}", e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solosoul_vault::{ObjectRecord, VaultConfig, VaultStore};
    use tempfile::TempDir;

    fn setup_vault() -> (VaultStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let config =
            VaultConfig::new("test_account", dir.path().to_path_buf()).with_data_key([0x42u8; 32]);
        let vault = VaultStore::open(config).unwrap();
        (vault, dir)
    }

    /// R2-X1: 路径判定纯函数防回归测试（symlink 旁路 / Android 双路径 / 边界）。
    #[test]
    fn test_path_within_base_canonical_only() {
        // canonicalize 成功：只用 resolved 判定，字面路径共享前缀不能绕过。
        let base_canon = Path::new("/vault");
        let base_raw = Path::new("/vault");
        // resolved 在 base 内 → true
        assert!(path_within_base(
            Path::new("/vault/attachments/a.bin"),
            Path::new("/vault/attachments/a.bin"),
            true,
            base_canon,
            base_raw,
        ));
        // resolved 在 base 外，但 raw 字面前缀命中 → 仍 false（旁路封死）
        assert!(!path_within_base(
            Path::new("/real_outside/a.bin"),
            Path::new("/vault/attachments/a.bin"),
            true,
            base_canon,
            base_raw,
        ));
        // 完全无关 → false
        assert!(!path_within_base(
            Path::new("/etc/passwd"),
            Path::new("/etc/passwd"),
            true,
            base_canon,
            base_raw,
        ));
    }

    #[test]
    fn test_path_within_base_raw_fallback_canonical() {
        // canonicalize 失败（Android 兜底）：raw 命中 canonical base → true
        let base_canon = Path::new("/vault");
        let base_raw = Path::new("/vault");
        assert!(path_within_base(
            Path::new("/vault/attachments/a.bin"),
            Path::new("/vault/attachments/a.bin"),
            false,
            base_canon,
            base_raw,
        ));
        // 与 base 完全无关 → false
        assert!(!path_within_base(
            Path::new("/etc/passwd"),
            Path::new("/etc/passwd"),
            false,
            base_canon,
            base_raw,
        ));
    }

    #[test]
    fn test_path_within_base_rejects_parent_dir_escape() {
        // P018：兜底分支拒绝含 `..` 的逃逸路径（前几段命中 base 但实际越出）
        let base_canon = Path::new("/vault");
        let base_raw = Path::new("/vault");
        for bad in [
            "/vault/../../etc/passwd",
            "/vault/attachments/../..//etc/passwd",
            "/vault/../vault_evil/secret",
        ] {
            assert!(
                !path_within_base(
                    Path::new(bad),
                    Path::new(bad),
                    false,
                    base_canon,
                    base_raw,
                ),
                "should reject parent-dir escape: {bad}"
            );
        }
        // 无 `..` 的正常库内路径仍放行
        assert!(path_within_base(
            Path::new("/vault/attachments/a.bin"),
            Path::new("/vault/attachments/a.bin"),
            false,
            base_canon,
            base_raw,
        ));
    }

    #[test]
    fn test_path_within_base_raw_fallback_dual_path() {
        // Android 双路径：raw 前缀是 /data/data（非 canonical），canonical base 是
        // /data/user/0——raw 仅命中 base_raw 时也应判定为库内（copy_to_vault 拒绝型）。
        let base_canon = Path::new("/data/user/0/com.solosoul");
        let base_raw = Path::new("/data/data/com.solosoul");
        // raw 命中 base_raw → true（此前 a||a 恒等会漏检）
        assert!(path_within_base(
            Path::new("/data/user/0/com.solosoul/attachments/a.bin"),
            Path::new("/data/data/com.solosoul/attachments/a.bin"),
            false,
            base_canon,
            base_raw,
        ));
        // 与 base 完全无关 → false
        assert!(!path_within_base(
            Path::new("/storage/emulated/0/Download/x.bin"),
            Path::new("/storage/emulated/0/Download/x.bin"),
            false,
            base_canon,
            base_raw,
        ));
        // canonicalize 成功时双路径也覆盖（resolved 命中 canonical）
        assert!(path_within_base(
            Path::new("/data/user/0/com.solosoul/attachments/a.bin"),
            Path::new("/data/data/com.solosoul/attachments/a.bin"),
            true,
            base_canon,
            base_raw,
        ));
    }

    #[test]
    fn test_attachment_meta_serde_roundtrip() {
        let original = AttachmentMeta {
            id: "att-1".to_string(),
            object_id: "obj-1".to_string(),
            file_name: "test.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            size_bytes: 1024,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            deleted_at: Some("2024-02-01T00:00:00Z".to_string()),
            src_path: Some("/tmp/test.pdf".to_string()),
            vault_path: Some("/vault/test.pdf".to_string()),
        };
        let json = serde_json::to_string(&original).unwrap();
        let restored: AttachmentMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, original.id);
        assert_eq!(restored.object_id, original.object_id);
        assert_eq!(restored.file_name, original.file_name);
        assert_eq!(restored.mime_type, original.mime_type);
        assert_eq!(restored.size_bytes, original.size_bytes);
        assert_eq!(restored.created_at, original.created_at);
        assert_eq!(restored.deleted_at, original.deleted_at);
        assert_eq!(restored.src_path, original.src_path);
        assert_eq!(restored.vault_path, original.vault_path);
    }

    #[test]
    fn test_load_attachments_empty() {
        let props = serde_json::json!({"title": "hello"});
        let atts = load_attachments(&props);
        assert!(atts.is_empty());
    }

    #[test]
    fn test_load_attachments_some() {
        let atts = vec![AttachmentMeta {
            id: "att-1".to_string(),
            object_id: "obj-1".to_string(),
            file_name: "a.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            size_bytes: 100,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            deleted_at: None,
            src_path: None,
            vault_path: None,
        }];
        let props = serde_json::json!({"title": "hello", "__attachments": atts});
        let loaded = load_attachments(&props);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "att-1");
    }

    #[test]
    fn test_save_and_load_attachments() {
        let mut props = serde_json::json!({"title": "hello"});
        let atts = vec![AttachmentMeta {
            id: "att-1".to_string(),
            object_id: "obj-1".to_string(),
            file_name: "a.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            size_bytes: 100,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            deleted_at: None,
            src_path: None,
            vault_path: None,
        }];
        save_attachments(&mut props, &atts);
        let loaded = load_attachments(&props);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "att-1");
        assert_eq!(loaded[0].file_name, "a.pdf");
    }

    #[test]
    fn test_load_all_referenced_attachment_ids() {
        let (vault, _dir) = setup_vault();
        let account_id = "acc-1";

        let record1 = ObjectRecord {
            contract_type_id: None,
            id: "obj-1".to_string(),
            account_id: account_id.to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Note 1".to_string(),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({
                "__attachments": [
                    AttachmentMeta {
                        id: "att-1".to_string(),
                        object_id: "obj-1".to_string(),
                        file_name: "a.pdf".to_string(),
                        mime_type: "application/pdf".to_string(),
                        size_bytes: 100,
                        created_at: "2024-01-01T00:00:00Z".to_string(),
                        deleted_at: None,
                        src_path: None,
                        vault_path: None,
                    },
                    AttachmentMeta {
                        id: "att-2".to_string(),
                        object_id: "obj-1".to_string(),
                        file_name: "b.pdf".to_string(),
                        mime_type: "application/pdf".to_string(),
                        size_bytes: 200,
                        created_at: "2024-01-01T00:00:00Z".to_string(),
                        deleted_at: Some("2024-02-01T00:00:00Z".to_string()),
                        src_path: None,
                        vault_path: None,
                    },
                ]
            }),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            template_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            ..Default::default()
        };
        vault.save_object(&record1).unwrap();

        let record2 = ObjectRecord {
            contract_type_id: None,
            id: "obj-2".to_string(),
            account_id: account_id.to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Note 2".to_string(),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({
                "__attachments": [
                    AttachmentMeta {
                        id: "att-3".to_string(),
                        object_id: "obj-2".to_string(),
                        file_name: "c.pdf".to_string(),
                        mime_type: "application/pdf".to_string(),
                        size_bytes: 300,
                        created_at: "2024-01-01T00:00:00Z".to_string(),
                        deleted_at: None,
                        src_path: None,
                        vault_path: None,
                    },
                ]
            }),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            template_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            ..Default::default()
        };
        vault.save_object(&record2).unwrap();

        let ids = load_all_referenced_attachment_ids(&vault, account_id).unwrap();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains("att-1"));
        assert!(ids.contains("att-2"));
        assert!(ids.contains("att-3"));
    }

    #[test]
    fn test_vault_attachment_filtering() {
        let (vault, _dir) = setup_vault();
        let mut record = ObjectRecord {
            contract_type_id: None,
            id: "obj-1".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Note".to_string(),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({
                "__attachments": [
                    AttachmentMeta {
                        id: "att-1".to_string(),
                        object_id: "obj-1".to_string(),
                        file_name: "active.pdf".to_string(),
                        mime_type: "application/pdf".to_string(),
                        size_bytes: 100,
                        created_at: "2024-01-01T00:00:00Z".to_string(),
                        deleted_at: None,
                        src_path: None,
                        vault_path: None,
                    },
                    AttachmentMeta {
                        id: "att-2".to_string(),
                        object_id: "obj-1".to_string(),
                        file_name: "deleted.pdf".to_string(),
                        mime_type: "application/pdf".to_string(),
                        size_bytes: 200,
                        created_at: "2024-01-01T00:00:00Z".to_string(),
                        deleted_at: Some("2024-02-01T00:00:00Z".to_string()),
                        src_path: None,
                        vault_path: None,
                    },
                ]
            }),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            template_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            ..Default::default()
        };
        vault.save_object(&record).unwrap();

        let rec = vault.load_object("obj-1").unwrap().unwrap();
        let atts = load_attachments(&rec.properties);
        let active: Vec<_> = atts.iter().filter(|a| a.deleted_at.is_none()).collect();
        let deleted: Vec<_> = atts.iter().filter(|a| a.deleted_at.is_some()).collect();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "att-1");
        assert_eq!(deleted.len(), 1);
        assert_eq!(deleted[0].id, "att-2");

        // Test soft-delete helper logic inline
        let mut atts_mut = load_attachments(&rec.properties);
        if let Some(a) = atts_mut.iter_mut().find(|a| a.id == "att-1") {
            a.deleted_at = Some("2024-03-01T00:00:00Z".to_string());
        }
        save_attachments(&mut record.properties, &atts_mut);
        vault.save_object(&record).unwrap();

        let rec2 = vault.load_object("obj-1").unwrap().unwrap();
        let atts2 = load_attachments(&rec2.properties);
        assert_eq!(atts2.iter().filter(|a| a.deleted_at.is_none()).count(), 0);
        assert_eq!(atts2.iter().filter(|a| a.deleted_at.is_some()).count(), 2);
    }

    /// P112 回归：`attachment_list_all` 数据流不再重复解密——子对象按 parent_id 预分组
    /// 一次完成（替代每页面 N+1 次解密查询），且 `build_attachment_tree_pages` 直接复用
    /// 已解密的 `summary.properties`（不再 load_objects_batch 二次全量解密）。
    /// 覆盖：页面含子对象附件、无附件对象不出现在树中、独立对象按 section 分组、
    /// 回收站视图只含已删除附件、分组 map 幂等（活动视图与回收站视图共享同一分组）。
    #[test]
    fn test_attachment_list_all_groups_children_and_reuses_summary_properties() {
        let (vault, _dir) = setup_vault();
        let account_id = "acc-1";

        let mk_meta = |id: &str, obj_id: &str, deleted: bool| AttachmentMeta {
            id: id.to_string(),
            object_id: obj_id.to_string(),
            file_name: format!("{}.pdf", id),
            mime_type: "application/pdf".to_string(),
            size_bytes: 100,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            deleted_at: deleted.then(|| "2024-02-01T00:00:00Z".to_string()),
            src_path: None,
            vault_path: None,
        };
        let mk_record = |id: &str,
                         type_id: &str,
                         section_type: &str,
                         parent: Option<&str>,
                         atts: Vec<AttachmentMeta>|
         -> ObjectRecord {
            ObjectRecord {
                contract_type_id: None,
                id: id.to_string(),
                account_id: account_id.to_string(),
                type_id: type_id.to_string(),
                section_type: section_type.to_string(),
                name: id.to_string(),
                icon_name: "document".to_string(),
                parent_id: parent.map(String::from),
                children_ids: vec![],
                properties: serde_json::json!({ "__attachments": atts }),
                property_labels: None,
                sensitivity_level: "internal".to_string(),
                is_deleted: false,
                deleted_at: None,
                tags_json: vec![],
                template_id: None,
                template_type: None,
                template_hash: None,
                ignored_template_hash: None,
                created_at: "2024-01-01T00:00:00Z".to_string(),
                updated_at: "2024-01-01T00:00:00Z".to_string(),
                version: 1,
            }
        };

        // 自定义页面 + 子对象（带附件）
        vault
            .save_object(&mk_record("page-1", "page", "", None, vec![]))
            .unwrap();
        vault
            .save_object(&mk_record(
                "obj-child",
                "note",
                "custom",
                Some("page-1"),
                vec![mk_meta("att-child", "obj-child", false)],
            ))
            .unwrap();
        // 独立对象（内置 section，带一个活动附件 + 一个已删除附件）
        vault
            .save_object(&mk_record(
                "obj-standalone",
                "note",
                "identity",
                None,
                vec![
                    mk_meta("att-act", "obj-standalone", false),
                    mk_meta("att-del", "obj-standalone", true),
                ],
            ))
            .unwrap();
        // 无附件的独立对象（不应出现在树中）
        vault
            .save_object(&mk_record("obj-empty", "note", "identity", None, vec![]))
            .unwrap();

        let objects = vault
            .list_objects(account_id, None, None, None, false, false)
            .unwrap();
        let (page_objects, section_groups, children_by_parent) =
            group_objects_for_attachment_tree(&objects);

        // 子对象按 parent_id 一次性分组（每页面无需再查）
        assert_eq!(children_by_parent.len(), 1);
        let child_summaries = children_by_parent.get("page-1").unwrap();
        assert_eq!(child_summaries.len(), 1);
        assert_eq!(child_summaries[0].id, "obj-child");

        // 活动视图：页面树含子对象附件，section 树含独立对象活动附件（无附件对象被过滤）
        let pages = build_attachment_tree_pages(
            &vault,
            &page_objects,
            &section_groups,
            &children_by_parent,
            false,
        )
        .unwrap();
        let page_tree = pages
            .iter()
            .find(|p| p.page_id.as_deref() == Some("page-1"))
            .expect("page-1 tree exists");
        assert_eq!(page_tree.objects.len(), 1);
        assert_eq!(page_tree.objects[0].object_id, "obj-child");
        assert_eq!(page_tree.objects[0].attachments.len(), 1);
        assert_eq!(page_tree.objects[0].attachments[0].id, "att-child");

        let section_tree = pages
            .iter()
            .find(|p| p.page_id.is_none())
            .expect("section tree exists");
        assert_eq!(section_tree.objects.len(), 1);
        assert_eq!(section_tree.objects[0].object_id, "obj-standalone");
        assert_eq!(section_tree.objects[0].attachments.len(), 1);
        assert_eq!(section_tree.objects[0].attachments[0].id, "att-act");

        // 回收站视图：只含已删除附件
        let trash_pages = build_attachment_tree_pages(
            &vault,
            &page_objects,
            &section_groups,
            &children_by_parent,
            true,
        )
        .unwrap();
        let trash_tree = trash_pages
            .iter()
            .find(|p| p.page_id.is_none())
            .expect("trash section tree exists");
        assert_eq!(trash_tree.objects[0].attachments.len(), 1);
        assert_eq!(trash_tree.objects[0].attachments[0].id, "att-del");
    }

    #[test]
    fn test_make_unique_dest_path_no_conflict() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("a.pdf");
        assert_eq!(make_unique_dest_path(&dest), dest);
    }

    #[test]
    fn test_make_unique_dest_path_with_conflict() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("a.pdf");
        std::fs::write(&dest, b"").unwrap();
        let r1 = make_unique_dest_path(&dest);
        assert_eq!(r1, tmp.path().join("a(1).pdf"));
        std::fs::write(&r1, b"").unwrap();
        let r2 = make_unique_dest_path(&dest);
        assert_eq!(r2, tmp.path().join("a(2).pdf"));
    }

    #[test]
    fn test_make_unique_dest_path_fixes_system_suffix() {
        let tmp = tempfile::tempdir().unwrap();
        // 系统保存对话框可能自动返回 a.pdf(1)，需要修正为 a(1).pdf
        let dest = tmp.path().join("a.pdf(1)");
        let result = make_unique_dest_path(&dest);
        assert_eq!(result, tmp.path().join("a(1).pdf"));
    }

    #[test]
    fn test_sanitize_duplicate_suffix_variants() {
        assert_eq!(sanitize_duplicate_suffix("a.pdf(1)"), "a(1).pdf");
        assert_eq!(sanitize_duplicate_suffix("a (1).pdf"), "a(1).pdf");
        assert_eq!(sanitize_duplicate_suffix("a(1)"), "a(1)");
        assert_eq!(sanitize_duplicate_suffix("a.pdf"), "a.pdf");
    }
}
