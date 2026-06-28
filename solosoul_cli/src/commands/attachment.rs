//! 附件命令：管理对象关联文件（与 GUI attachment.rs 保持行为一致）。

use std::collections::HashSet;
use std::sync::Arc;

use color_eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::app::{App, AppPhase};
use crate::widgets::prompt::{self, PromptResult, PromptSpec};

/// 单对象最大活跃附件数（与 GUI 保持一致）。
const MAX_ACTIVE_ATTACHMENTS: usize = 200;

/// 附件元数据结构，字段与 GUI `AttachmentMeta` 完全一致。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub src_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault_path: Option<String>,
}

fn map_err(e: String) -> color_eyre::Report {
    color_eyre::eyre::eyre!(e)
}

/// 确保 Vault 已解锁，返回当前账户 ID。
fn require_unlocked(app: &mut App) -> Result<String> {
    if !app.vault_service.is_unlocked() {
        app.error_message = Some("请先使用 /unlock 登录".to_string());
        return Err(color_eyre::eyre::eyre!("Vault is locked"));
    }
    app.vault_service
        .get_current_account()
        .ok_or_else(|| color_eyre::eyre::eyre!("No current account"))
}

fn vault(app: &mut App) -> Result<Arc<solosoul_core::VaultStore>> {
    app.vault_service
        .get_vault_store()
        .ok_or_else(|| color_eyre::eyre::eyre!("Vault 未打开"))
}

/// 从对象 properties 中读取附件列表。
fn load_attachments(props: &Value) -> Vec<AttachmentMeta> {
    props
        .get("__attachments")
        .and_then(|v| serde_json::from_value::<Vec<AttachmentMeta>>(v.clone()).ok())
        .unwrap_or_default()
}

/// 将附件列表写回对象 properties。
fn save_attachments(props: &mut Value, atts: &[AttachmentMeta]) {
    if let Value::Object(ref mut obj) = props {
        obj.insert(
            "__attachments".to_string(),
            serde_json::to_value(atts).unwrap_or_default(),
        );
    }
}

/// 根据当前界面推断当前对象 ID。
fn current_object_id(app: &App) -> Option<String> {
    match &app.phase {
        AppPhase::ObjectDetail { object } => Some(object.id.clone()),
        AppPhase::EditObjectWizard { object_id, .. } => Some(object_id.clone()),
        AppPhase::HistoryList { object_id, .. } => Some(object_id.clone()),
        AppPhase::AttachmentList { object_id, .. } => Some(object_id.clone()),
        _ => None,
    }
}

/// 执行 `/attach <subcommand>`。
pub fn handle(app: &mut App, args: &[&str]) -> Result<()> {
    let _account_id = require_unlocked(app)?;
    match args.first().copied() {
        None | Some("help") => {
            app.error_message = Some(
                "用法: /attach list [object_id] | add <file_path> | rename <id> <new_name> | delete <id> | restore <id> | purge <id> | cleanup".to_string(),
            );
            Ok(())
        }
        Some("list") => list(app, args.get(1).copied()),
        Some("add") => add(app, args.get(1).copied()),
        Some("rename") => {
            let id = args.get(1).copied();
            let new_name = args.get(2..).map(|s| s.join(" "));
            rename(app, id, new_name.as_deref())
        }
        Some("delete") => delete(app, args.get(1).copied()),
        Some("restore") => restore(app, args.get(1).copied()),
        Some("purge") => purge(app, args.get(1).copied()),
        Some("cleanup") => cleanup(app),
        Some(other) => {
            app.error_message = Some(format!("未知子命令: {}", other));
            Ok(())
        }
    }
}

/// `/attach list [object_id]`：列出对象附件。
fn list(app: &mut App, object_id: Option<&str>) -> Result<()> {
    let account_id = require_unlocked(app)?;
    let object_id = match object_id {
        Some(id) => id.to_string(),
        None => match current_object_id(app) {
            Some(id) => id,
            None => {
                app.error_message = Some("请提供对象 ID 或在对象详情页执行".to_string());
                return Ok(());
            }
        },
    };

    let vault = vault(app)?;
    match vault.load_object(&object_id).map_err(map_err)? {
        Some(record) if record.account_id == account_id && !record.is_deleted => {
            let items: Vec<AttachmentMeta> = load_attachments(&record.properties)
                .into_iter()
                .filter(|a| a.deleted_at.is_none())
                .collect();
            app.previous_phase = Some(app.phase.clone());
            app.phase = AppPhase::AttachmentList {
                object_id,
                items,
                show_deleted: false,
                selected: 0,
            };
        }
        _ => {
            app.error_message = Some(format!("对象 '{}' 不存在或已被删除", object_id));
        }
    }
    Ok(())
}

/// `/attach add <file_path>`：复制文件并添加附件元数据。
fn add(app: &mut App, file_path: Option<&str>) -> Result<()> {
    let account_id = require_unlocked(app)?;
    let file_path = match file_path {
        Some(p) => p,
        None => {
            app.error_message =
                Some("请提供文件路径，例如 /attach add /path/to/file.pdf".to_string());
            return Ok(());
        }
    };
    let object_id = match current_object_id(app) {
        Some(id) => id,
        None => {
            app.error_message = Some("请在对象详情页执行 /attach add".to_string());
            return Ok(());
        }
    };

    let path = std::path::Path::new(file_path);
    if !path.exists() || !path.is_file() {
        app.error_message = Some(format!("文件不存在或不是普通文件: {}", file_path));
        return Ok(());
    }

    let vault = vault(app)?;
    let mut record = match vault.load_object(&object_id).map_err(map_err)? {
        Some(r) if r.account_id == account_id && !r.is_deleted => r,
        _ => {
            app.error_message = Some(format!("对象 '{}' 不存在或已被删除", object_id));
            return Ok(());
        }
    };

    let mut atts = load_attachments(&record.properties);
    let active_count = atts.iter().filter(|a| a.deleted_at.is_none()).count();
    if active_count >= MAX_ACTIVE_ATTACHMENTS {
        app.error_message = Some(format!(
            "单个对象最多保留 {} 个活跃附件",
            MAX_ACTIVE_ATTACHMENTS
        ));
        return Ok(());
    }

    let file_name = sanitize_file_name(
        path.file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unnamed".to_string())
            .as_str(),
    );
    let size_bytes = path.metadata().map(|m| m.len()).unwrap_or(0);
    let mime_type = infer_mime_type(&file_name);
    let attachment_id = format!("att_{}", uuid::Uuid::new_v4());
    let created_at = chrono::Utc::now().to_rfc3339();

    // 复制文件到 vault 附件目录。
    let vault_path = match copy_to_vault(app, file_path, &object_id, &attachment_id, &file_name) {
        Ok(p) => p,
        Err(e) => {
            app.error_message = Some(format!("复制附件失败: {}", e));
            return Ok(());
        }
    };

    let meta = AttachmentMeta {
        id: attachment_id,
        object_id: object_id.clone(),
        file_name,
        mime_type,
        size_bytes,
        created_at,
        deleted_at: None,
        src_path: Some(file_path.to_string()),
        vault_path: Some(vault_path),
    };

    atts.push(meta);
    save_attachments(&mut record.properties, &atts);
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record).map_err(map_err)?;

    let _ = vault.log_structured(
        "attachment_add",
        "attachment",
        Some(&object_id),
        Some(&record.name),
        "user",
        Some(&format!("file={}", file_path)),
    );

    app.error_message = Some(format!("已添加附件: {}", file_path));
    Ok(())
}

/// `/attach rename <id> <new_name>`：重命名附件。
fn rename(app: &mut App, attachment_id: Option<&str>, new_name: Option<&str>) -> Result<()> {
    let account_id = require_unlocked(app)?;
    let attachment_id = match attachment_id {
        Some(id) => id,
        None => {
            app.error_message =
                Some("请提供附件 ID，例如 /attach rename att_xxx new.pdf".to_string());
            return Ok(());
        }
    };
    let new_name = match new_name {
        Some(n) if !n.is_empty() => sanitize_file_name(n),
        _ => {
            app.error_message = Some("请提供新文件名".to_string());
            return Ok(());
        }
    };

    let object_id = match current_object_id(app) {
        Some(id) => id,
        None => {
            app.error_message = Some("请在对象详情页执行 /attach rename".to_string());
            return Ok(());
        }
    };

    let vault = vault(app)?;
    let mut record = match vault.load_object(&object_id).map_err(map_err)? {
        Some(r) if r.account_id == account_id && !r.is_deleted => r,
        _ => {
            app.error_message = Some(format!("对象 '{}' 不存在或已被删除", object_id));
            return Ok(());
        }
    };

    let mut atts = load_attachments(&record.properties);
    if let Some(a) = atts.iter_mut().find(|a| a.id == attachment_id) {
        a.file_name = new_name.clone();
    } else {
        app.error_message = Some(format!("附件 '{}' 不存在", attachment_id));
        return Ok(());
    }

    save_attachments(&mut record.properties, &atts);
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record).map_err(map_err)?;

    let _ = vault.log_structured(
        "attachment_rename",
        "attachment",
        Some(&object_id),
        Some(attachment_id),
        "user",
        Some(&format!("new_name={}", new_name)),
    );

    app.error_message = Some(format!("已重命名为: {}", new_name));
    Ok(())
}

/// `/attach delete <id>`：软删除附件。
fn delete(app: &mut App, attachment_id: Option<&str>) -> Result<()> {
    let account_id = require_unlocked(app)?;
    let attachment_id = match attachment_id {
        Some(id) => id.to_string(),
        None => {
            app.error_message = Some("请提供附件 ID，例如 /attach delete att_xxx".to_string());
            return Ok(());
        }
    };

    let object_id = match current_object_id(app) {
        Some(id) => id,
        None => {
            app.error_message = Some("请在对象详情页执行 /attach delete".to_string());
            return Ok(());
        }
    };

    let vault = vault(app)?;
    let atts = match vault.load_object(&object_id).map_err(map_err)? {
        Some(r) if r.account_id == account_id && !r.is_deleted => load_attachments(&r.properties),
        _ => {
            app.error_message = Some(format!("对象 '{}' 不存在或已被删除", object_id));
            return Ok(());
        }
    };

    if !atts
        .iter()
        .any(|a| a.id == attachment_id && a.deleted_at.is_none())
    {
        app.error_message = Some(format!("附件 '{}' 不存在或已删除", attachment_id));
        return Ok(());
    }

    prompt::open(
        app,
        PromptSpec::Confirm {
            message: format!("软删除附件 '{}'？可在回收站恢复。", attachment_id),
            default_yes: false,
        },
        Box::new(move |app, result| {
            if let PromptResult::Confirm(true) = result {
                if let Err(e) = do_delete(app, &object_id, &attachment_id) {
                    app.error_message = Some(format!("删除附件失败: {}", e));
                }
            }
        }),
    );

    Ok(())
}

fn do_delete(app: &mut App, object_id: &str, attachment_id: &str) -> Result<()> {
    let account_id = require_unlocked(app)?;
    let vault = vault(app)?;
    let mut record = match vault.load_object(object_id).map_err(map_err)? {
        Some(r) if r.account_id == account_id && !r.is_deleted => r,
        _ => {
            return Err(color_eyre::eyre::eyre!("对象不存在或已被删除"));
        }
    };

    let mut atts = load_attachments(&record.properties);
    if let Some(a) = atts.iter_mut().find(|a| a.id == attachment_id) {
        a.deleted_at = Some(chrono::Utc::now().to_rfc3339());
    } else {
        return Err(color_eyre::eyre::eyre!("附件不存在"));
    }

    save_attachments(&mut record.properties, &atts);
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record).map_err(map_err)?;

    let _ = vault.log_structured(
        "attachment_soft_delete",
        "attachment",
        Some(object_id),
        Some(attachment_id),
        "user",
        None,
    );

    app.error_message = Some(format!("已删除附件: {}", attachment_id));
    Ok(())
}

/// `/attach restore <id>`：恢复软删除的附件。
fn restore(app: &mut App, attachment_id: Option<&str>) -> Result<()> {
    let account_id = require_unlocked(app)?;
    let attachment_id = match attachment_id {
        Some(id) => id,
        None => {
            app.error_message = Some("请提供附件 ID，例如 /attach restore att_xxx".to_string());
            return Ok(());
        }
    };

    let object_id = match current_object_id(app) {
        Some(id) => id,
        None => {
            app.error_message = Some("请在对象详情页执行 /attach restore".to_string());
            return Ok(());
        }
    };

    let vault = vault(app)?;
    let mut record = match vault.load_object(&object_id).map_err(map_err)? {
        Some(r) if r.account_id == account_id && !r.is_deleted => r,
        _ => {
            app.error_message = Some(format!("对象 '{}' 不存在或已被删除", object_id));
            return Ok(());
        }
    };

    let mut atts = load_attachments(&record.properties);
    if let Some(a) = atts.iter_mut().find(|a| a.id == attachment_id) {
        a.deleted_at = None;
    } else {
        app.error_message = Some(format!("附件 '{}' 不存在", attachment_id));
        return Ok(());
    }

    save_attachments(&mut record.properties, &atts);
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record).map_err(map_err)?;

    let _ = vault.log_structured(
        "attachment_restore",
        "attachment",
        Some(&object_id),
        Some(attachment_id),
        "user",
        None,
    );

    app.error_message = Some(format!("已恢复附件: {}", attachment_id));
    Ok(())
}

/// `/attach purge <id>`：彻底删除附件（元数据 + 文件）。
fn purge(app: &mut App, attachment_id: Option<&str>) -> Result<()> {
    let account_id = require_unlocked(app)?;
    let attachment_id = match attachment_id {
        Some(id) => id.to_string(),
        None => {
            app.error_message = Some("请提供附件 ID，例如 /attach purge att_xxx".to_string());
            return Ok(());
        }
    };

    let object_id = match current_object_id(app) {
        Some(id) => id,
        None => {
            app.error_message = Some("请在对象详情页执行 /attach purge".to_string());
            return Ok(());
        }
    };

    let vault = vault(app)?;
    let atts = match vault.load_object(&object_id).map_err(map_err)? {
        Some(r) if r.account_id == account_id && !r.is_deleted => load_attachments(&r.properties),
        _ => {
            app.error_message = Some(format!("对象 '{}' 不存在或已被删除", object_id));
            return Ok(());
        }
    };

    if !atts.iter().any(|a| a.id == attachment_id) {
        app.error_message = Some(format!("附件 '{}' 不存在", attachment_id));
        return Ok(());
    }

    prompt::open(
        app,
        PromptSpec::Confirm {
            message: format!("彻底删除附件 '{}'？此操作不可恢复。", attachment_id),
            default_yes: false,
        },
        Box::new(move |app, result| {
            if let PromptResult::Confirm(true) = result {
                if let Err(e) = do_purge(app, &object_id, &attachment_id) {
                    app.error_message = Some(format!("彻底删除附件失败: {}", e));
                }
            }
        }),
    );

    Ok(())
}

fn do_purge(app: &mut App, object_id: &str, attachment_id: &str) -> Result<()> {
    let account_id = require_unlocked(app)?;
    let vault = vault(app)?;
    let mut record = match vault.load_object(object_id).map_err(map_err)? {
        Some(r) if r.account_id == account_id && !r.is_deleted => r,
        _ => {
            return Err(color_eyre::eyre::eyre!("对象不存在或已被删除"));
        }
    };

    let atts: Vec<AttachmentMeta> = load_attachments(&record.properties)
        .into_iter()
        .filter(|a| a.id != attachment_id)
        .collect();

    // 删除物理文件。
    let attachments_dir = app
        .vault_service
        .base_path()
        .join("attachments")
        .join(object_id)
        .join(attachment_id);
    if attachments_dir.exists() {
        let _ = std::fs::remove_dir_all(&attachments_dir);
    }

    save_attachments(&mut record.properties, &atts);
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record).map_err(map_err)?;

    let _ = vault.log_structured(
        "attachment_purge",
        "attachment",
        Some(object_id),
        Some(attachment_id),
        "user",
        None,
    );

    app.error_message = Some(format!("已彻底删除附件: {}", attachment_id));
    Ok(())
}

/// `/attach cleanup`：清理无元数据引用的孤立附件文件。
fn cleanup(app: &mut App) -> Result<()> {
    let account_id = require_unlocked(app)?;
    let vault = vault(app)?;

    let active_ids = load_all_referenced_attachment_ids(&vault, &account_id)?;
    let base_dir = app.vault_service.base_path().join("attachments");

    if !base_dir.exists() {
        app.error_message = Some("暂无附件目录".to_string());
        return Ok(());
    }

    let mut removed = 0usize;
    let mut total_freed = 0u64;

    if let Ok(object_entries) = std::fs::read_dir(&base_dir) {
        for obj_entry in object_entries.flatten() {
            let obj_path = obj_entry.path();
            if !obj_path.is_dir() {
                continue;
            }
            if let Ok(att_entries) = std::fs::read_dir(&obj_path) {
                for att_entry in att_entries.flatten() {
                    let att_path = att_entry.path();
                    let att_id = att_entry.file_name().to_string_lossy().to_string();
                    if !active_ids.contains(&att_id) {
                        if let Ok(meta) = att_path.metadata() {
                            total_freed += meta.len();
                        }
                        let _ = std::fs::remove_dir_all(&att_path);
                        removed += 1;
                    }
                }
            }
            // 删除空对象目录。
            if std::fs::read_dir(&obj_path)
                .map(|mut d| d.next().is_none())
                .unwrap_or(false)
            {
                let _ = std::fs::remove_dir(&obj_path);
            }
        }
    }

    let _ = vault.log_structured(
        "attachment_cleanup",
        "attachment",
        None,
        None,
        "system",
        Some(&format!("removed={} freed={}", removed, total_freed)),
    );

    app.error_message = Some(format!(
        "清理完成：移除 {} 个孤立附件，释放 {} 字节",
        removed, total_freed
    ));
    Ok(())
}

/// 收集所有对象元数据中引用的附件 ID（含软删除）。
fn load_all_referenced_attachment_ids(
    vault: &solosoul_core::VaultStore,
    account_id: &str,
) -> Result<HashSet<String>> {
    let objects = vault
        .list_objects(account_id, None, None, None, false, false)
        .map_err(map_err)?;
    let mut active_ids = HashSet::new();
    for summary in &objects {
        if let Ok(Some(rec)) = vault.load_object(&summary.id) {
            for a in load_attachments(&rec.properties) {
                active_ids.insert(a.id.clone());
            }
        }
    }
    Ok(active_ids)
}

/// 将文件复制到 vault 附件目录，返回目标路径。
///
/// # 安全
/// - `src_path` 会先 canonicalize 以解析相对路径遍历 (`../`)。
/// - 来源路径不能位于 vault 存储目录内（防止自引用）。
fn copy_to_vault(
    app: &App,
    src_path: &str,
    object_id: &str,
    attachment_id: &str,
    file_name: &str,
) -> Result<String> {
    let base = app.vault_service.base_path();

    // Canonicalize src_path 以解析路径遍历
    let src = std::path::Path::new(src_path)
        .canonicalize()
        .map_err(|e| map_err(format!("无效的源文件路径: {}", e)))?;

    // 拒绝源文件位于 vault 存储目录内
    // Canonicalize vault 基目录以匹配 src.canonicalize() 的符号链接解析
    let vault_base = base
        .canonicalize()
        .map_err(|e| map_err(format!("无效的 vault 基目录: {}", e)))?;
    if src.starts_with(&vault_base) {
        return Err(map_err("源文件路径不能位于 vault 存储目录内".to_string()));
    }

    let dest_dir = vault_base
        .join("attachments")
        .join(object_id)
        .join(attachment_id);
    std::fs::create_dir_all(&dest_dir).map_err(|e| map_err(format!("创建目录失败: {}", e)))?;

    let safe_name = sanitize_file_name(file_name);
    let dest_path = dest_dir.join(&safe_name);
    std::fs::copy(&src, &dest_path).map_err(|e| map_err(format!("复制文件失败: {}", e)))?;
    Ok(dest_path.to_string_lossy().to_string())
}

/// 清理文件名，仅保留最终路径分量，防止路径遍历。
fn sanitize_file_name(file_name: &str) -> String {
    std::path::Path::new(file_name)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unnamed".to_string())
}

/// 根据扩展名推断 MIME 类型。
fn infer_mime_type(file_name: &str) -> String {
    let ext = std::path::Path::new(file_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    match ext.to_lowercase().as_str() {
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        "md" => "text/markdown",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "mp4" => "video/mp4",
        "mp3" => "audio/mpeg",
        "json" => "application/json",
        "csv" => "text/csv",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use solosoul_core::{ObjectRecord, VaultService};
    use std::sync::Arc;

    use crate::app::{App, AppPhase};

    fn unlocked_app() -> (App, String, tempfile::TempDir) {
        let _guard = crate::VAULT_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::TempDir::new().unwrap();
        std::env::set_var("SOLOSOUL_DATA_DIR", dir.path());
        let vault = VaultService::new();
        let account = vault.create_account("Test", crate::TEST_PASSWORD, None).unwrap();
        let account_id = account["id"].as_str().unwrap().to_string();
        let app = App::new(Arc::new(vault)).unwrap();
        (app, account_id, dir)
    }

    fn create_test_object(app: &mut App, account_id: &str) -> String {
        let vault = app.vault_service.get_vault_store().unwrap();
        let id = format!("obj_{}", uuid::Uuid::new_v4());
        let record = ObjectRecord {
            id: id.clone(),
            account_id: account_id.to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "测试对象".to_string(),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({}),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            contract_type_id: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
        };
        vault.save_object(&record).unwrap();
        app.phase = AppPhase::ObjectDetail { object: record };
        id
    }

    #[test]
    fn test_add_and_list() {
        let (mut app, account_id, _dir) = unlocked_app();
        let obj_id = create_test_object(&mut app, &account_id);

        // Use a temp dir outside vault base to avoid self-reference check
        let files_dir = tempfile::TempDir::new().unwrap();
        let file_path = files_dir.path().join("test.txt");
        std::fs::write(&file_path, "hello").unwrap();

        super::add(&mut app, Some(file_path.to_str().unwrap())).unwrap();

        let vault = app.vault_service.get_vault_store().unwrap();
        let record = vault.load_object(&obj_id).unwrap().unwrap();
        let atts = super::load_attachments(&record.properties);
        assert_eq!(atts.len(), 1);
        assert_eq!(atts[0].file_name, "test.txt");
        assert_eq!(atts[0].size_bytes, 5);
        assert!(atts[0].vault_path.is_some());

        super::list(&mut app, None).unwrap();
        assert!(matches!(app.phase, AppPhase::AttachmentList { .. }));
    }

    #[test]
    fn test_rename() {
        let (mut app, account_id, _dir) = unlocked_app();
        let obj_id = create_test_object(&mut app, &account_id);
        let files_dir = tempfile::TempDir::new().unwrap();
        let file_path = files_dir.path().join("test.txt");
        std::fs::write(&file_path, "hello").unwrap();
        super::add(&mut app, Some(file_path.to_str().unwrap())).unwrap();

        let vault = app.vault_service.get_vault_store().unwrap();
        let record = vault.load_object(&obj_id).unwrap().unwrap();
        let att_id = super::load_attachments(&record.properties)[0].id.clone();

        super::rename(&mut app, Some(&att_id), Some("new name.txt")).unwrap();

        let record = vault.load_object(&obj_id).unwrap().unwrap();
        let atts = super::load_attachments(&record.properties);
        assert_eq!(atts[0].file_name, "new name.txt");
    }

    #[test]
    fn test_delete_restore_and_purge() {
        let (mut app, account_id, _dir) = unlocked_app();
        let obj_id = create_test_object(&mut app, &account_id);
        let files_dir = tempfile::TempDir::new().unwrap();
        let file_path = files_dir.path().join("test.txt");
        std::fs::write(&file_path, "hello").unwrap();
        super::add(&mut app, Some(file_path.to_str().unwrap())).unwrap();

        let vault = app.vault_service.get_vault_store().unwrap();
        let record = vault.load_object(&obj_id).unwrap().unwrap();
        let att_id = super::load_attachments(&record.properties)[0].id.clone();
        let vault_file = app
            .vault_service
            .base_path()
            .join("attachments")
            .join(&obj_id)
            .join(&att_id)
            .join("test.txt");
        assert!(vault_file.exists());

        // 软删除
        super::delete(&mut app, Some(&att_id)).unwrap();
        assert!(app.prompt.is_some());
        super::do_delete(&mut app, &obj_id, &att_id).unwrap();

        let record = vault.load_object(&obj_id).unwrap().unwrap();
        let atts = super::load_attachments(&record.properties);
        assert!(atts[0].deleted_at.is_some());
        // 软删除保留文件
        assert!(vault_file.exists());

        // 恢复
        super::restore(&mut app, Some(&att_id)).unwrap();
        let record = vault.load_object(&obj_id).unwrap().unwrap();
        let atts = super::load_attachments(&record.properties);
        assert!(atts[0].deleted_at.is_none());

        // 彻底删除
        super::purge(&mut app, Some(&att_id)).unwrap();
        assert!(app.prompt.is_some());
        super::do_purge(&mut app, &obj_id, &att_id).unwrap();

        let record = vault.load_object(&obj_id).unwrap().unwrap();
        let atts = super::load_attachments(&record.properties);
        assert!(atts.is_empty());
        assert!(!vault_file.exists());
    }

    #[test]
    fn test_cleanup() {
        let (mut app, account_id, _dir) = unlocked_app();
        let obj_id = create_test_object(&mut app, &account_id);
        let files_dir = tempfile::TempDir::new().unwrap();
        let file_path = files_dir.path().join("test.txt");
        std::fs::write(&file_path, "hello").unwrap();
        super::add(&mut app, Some(file_path.to_str().unwrap())).unwrap();

        let vault = app.vault_service.get_vault_store().unwrap();
        let record = vault.load_object(&obj_id).unwrap().unwrap();
        let att_id = super::load_attachments(&record.properties)[0].id.clone();

        // 制造孤立文件：保留文件但删除元数据。
        let mut record = vault.load_object(&obj_id).unwrap().unwrap();
        record.properties = serde_json::json!({});
        vault.save_object(&record).unwrap();

        let orphan_dir = app
            .vault_service
            .base_path()
            .join("attachments")
            .join(&obj_id)
            .join(&att_id);
        assert!(orphan_dir.exists());

        super::cleanup(&mut app).unwrap();
        assert!(!orphan_dir.exists());
    }

    #[test]
    fn test_sanitize_file_name_prevents_traversal() {
        assert_eq!(super::sanitize_file_name("../../../etc/passwd"), "passwd");
        assert_eq!(super::sanitize_file_name("/tmp/file.txt"), "file.txt");
        assert_eq!(super::sanitize_file_name("normal.txt"), "normal.txt");
    }

    #[test]
    fn test_handle_unknown_subcommand() {
        let (mut app, _id, _dir) = unlocked_app();
        super::handle(&mut app, &["unknown"]).unwrap();
        assert!(app.error_message.is_some());
    }
}
