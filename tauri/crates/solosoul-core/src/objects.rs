//! Object, attachment, and trash management — core business logic.
//!
//! This module provides high-level operations for creating, updating, deleting,
//! and managing objects, attachments, and trash items. These functions are shared
//! by the CLI and Tauri GUI hosts.
//!
//! Each function takes `&VaultStore` + parameters and returns a result, leaving
//! UI concerns (prompts, state management, argument parsing) to the caller.

use std::collections::HashSet;
use std::path::Path;

use solosoul_vault::{ObjectRecord, TrashItem, VaultStore};

/// 导出附件元数据，与 GUI/CLI 共享。
pub use crate::export_import::AttachmentMeta;

const MAX_ACTIVE_ATTACHMENTS: usize = 200;

// ── Object helpers ─────────────────────────────────────────

/// 创建页面。
pub fn create_page(
    vault: &VaultStore,
    account_id: &str,
    name: &str,
) -> Result<ObjectRecord, String> {
    if name.trim().is_empty() {
        return Err("页面名称不能为空".to_string());
    }

    let pages = vault.list_objects(account_id, Some("page"), None, None, false, false)?;
    if pages.iter().any(|p| p.name.eq_ignore_ascii_case(name)) {
        return Err(format!("页面 '{}' 已存在", name));
    }

    let now = chrono::Utc::now().to_rfc3339();
    let id = format!("page_{}", uuid::Uuid::new_v4());
    let record = ObjectRecord {
        id: id.clone(),
        account_id: account_id.to_string(),
        type_id: "page".to_string(),
        section_type: "page".to_string(),
        name: name.to_string(),
        icon_name: "folder".to_string(),
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
        template_hash: None,
        created_at: now.clone(),
        updated_at: now.clone(),
        version: 1,
    };

    vault.save_object(&record)?;
    save_creation_snapshot(vault, &id, name, &serde_json::json!({}))?;
    let _ = vault.log_structured(
        "page_create",
        "page",
        Some(&id),
        Some(name),
        "user",
        Some("source=cli"),
    );

    Ok(record)
}

/// 创建对象并更新父页面 children_ids。
pub fn create_object(
    vault: &VaultStore,
    account_id: &str,
    page_id: &str,
    name: &str,
    properties: serde_json::Value,
    template_id: Option<&str>,
    icon_name: Option<&str>,
) -> Result<ObjectRecord, String> {
    let name = if name.trim().is_empty() {
        "未命名对象".to_string()
    } else {
        name.to_string()
    };

    let now = chrono::Utc::now().to_rfc3339();
    let id = format!("obj_{}", uuid::Uuid::new_v4());

    let type_id = template_id.unwrap_or("note").to_string();
    let icon = icon_name.unwrap_or("document").to_string();

    let record = ObjectRecord {
        id: id.clone(),
        account_id: account_id.to_string(),
        type_id,
        section_type: "identity".to_string(),
        name: name.clone(),
        icon_name: icon,
        parent_id: Some(page_id.to_string()),
        children_ids: vec![],
        properties,
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: template_id.map(|s| s.to_string()),
        contract_type_id: None,
        template_type: template_id.map(|_| "user".to_string()),
        template_hash: None,
        created_at: now.clone(),
        updated_at: now.clone(),
        version: 1,
    };

    vault.save_object(&record)?;

    // 更新父页面的 children_ids
    if let Ok(Some(mut parent)) = vault.load_object(page_id) {
        if !parent.children_ids.contains(&id) {
            parent.children_ids.push(id.clone());
            parent.updated_at = chrono::Utc::now().to_rfc3339();
            parent.version += 1;
            vault.save_object(&parent)?;
        }
    }

    save_creation_snapshot(vault, &id, &name, &record.properties)?;
    let _ = vault.log_structured(
        "object_create",
        "object",
        Some(&id),
        Some(&name),
        "user",
        Some(&format!("parent_id={}", page_id)),
    );

    Ok(record)
}

/// 保存编辑后的对象（含 snapshot 和日志）。
pub fn update_object(vault: &VaultStore, object: &mut ObjectRecord) -> Result<(), String> {
    object.updated_at = chrono::Utc::now().to_rfc3339();
    object.version += 1;

    vault.save_object(object)?;

    let snapshot_data = serde_json::to_vec(&serde_json::json!({
        "name": object.name,
        "tags": object.tags_json,
        "properties": object.properties,
    }))
    .unwrap_or_default();
    let _ = vault.save_snapshot(&object.id, "user_edit", &snapshot_data, "Updated");
    let _ = vault.log_structured(
        "object_update",
        "object",
        Some(&object.id),
        Some(&object.name),
        "user",
        Some(&format!("section={}", object.section_type)),
    );

    Ok(())
}

// ── Trash helpers ──────────────────────────────────────────

/// 将对象移入回收站。组合操作：保存 TrashItem + 软删除对象。
pub fn move_to_trash(
    vault: &VaultStore,
    record: &ObjectRecord,
    item_type: &str,
    original_parent_id: Option<String>,
    retention_ms: i64,
) -> Result<(), String> {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let full_record = serde_json::json!({
        "id": record.id,
        "account_id": record.account_id,
        "type_id": record.type_id,
        "section_type": record.section_type,
        "name": record.name,
        "icon_name": record.icon_name,
        "parent_id": record.parent_id,
        "children_ids": record.children_ids,
        "properties": record.properties,
        "property_labels": record.property_labels,
        "sensitivity_level": record.sensitivity_level,
        "tags": record.tags_json,
        "created_at": record.created_at,
        "updated_at": record.updated_at,
        "version": record.version,
        "template_id": record.template_id,
        "template_type": record.template_type,
    });
    let trash = TrashItem {
        id: format!("trash_{}", uuid::Uuid::new_v4()),
        item_type: item_type.to_string(),
        original_id: record.id.clone(),
        original_parent_id,
        original_section_type: Some(record.section_type.clone()),
        original_sort_order: None,
        data: serde_json::to_vec(&full_record).unwrap_or_default(),
        deleted_at: now_ms,
        expires_at: Some(now_ms + retention_ms),
        deleted_by: "user".to_string(),
        name_snapshot: record.name.clone(),
        icon_snapshot: Some(record.icon_name.clone()),
    };
    vault.save_trash_item(&trash)?;
    vault.delete_object(&record.id, true)?;
    Ok(())
}

/// 从 TrashItem 恢复 ObjectRecord。用于从回收站还原对象。
pub fn object_record_from_trash(trash: &TrashItem) -> Result<ObjectRecord, String> {
    let data: serde_json::Value =
        serde_json::from_slice(&trash.data).map_err(|e| format!("回收站数据损坏: {}", e))?;
    let now = chrono::Utc::now().to_rfc3339();
    Ok(ObjectRecord {
        id: data["id"]
            .as_str()
            .unwrap_or(&trash.original_id)
            .to_string(),
        account_id: data["account_id"]
            .as_str()
            .unwrap_or("imported")
            .to_string(),
        type_id: data["type_id"].as_str().unwrap_or("note").to_string(),
        section_type: trash
            .original_section_type
            .as_deref()
            .or(data["section_type"].as_str())
            .unwrap_or("identity")
            .to_string(),
        name: data["name"]
            .as_str()
            .unwrap_or(&trash.name_snapshot)
            .to_string(),
        icon_name: data["icon_name"].as_str().unwrap_or("document").to_string(),
        parent_id: data["parent_id"].as_str().map(String::from),
        children_ids: data["children_ids"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        properties: data["properties"].clone(),
        property_labels: if data["property_labels"].is_null() {
            None
        } else {
            Some(data["property_labels"].clone())
        },
        sensitivity_level: data["sensitivity_level"]
            .as_str()
            .unwrap_or("internal")
            .to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: data["tags"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        template_id: data["template_id"].as_str().map(String::from),
        contract_type_id: data["contract_type_id"].as_str().map(String::from),
        template_type: data["template_type"].as_str().map(String::from),
        template_hash: data["template_hash"].as_str().map(String::from),
        created_at: data["created_at"].as_str().unwrap_or(&now).to_string(),
        updated_at: now,
        version: data["version"].as_u64().unwrap_or(1) as u32,
    })
}

/// 从回收站恢复单个对象，含冲突处理。
pub fn restore_from_trash(
    vault: &VaultStore,
    account_id: &str,
    trash_id: &str,
) -> Result<RestoreResult, String> {
    let trash = vault
        .get_trash_item(trash_id)?
        .ok_or_else(|| "回收站项目不存在".to_string())?;

    let new_id = match trash.item_type.as_str() {
        "object" | "page" => {
            let mut record = object_record_from_trash(&trash)?;

            // 如果原父页面不存在或已删除，清除 parent_id
            if let Some(ref pid) = record.parent_id.clone() {
                if vault
                    .load_object(pid)
                    .ok()
                    .flatten()
                    .is_none_or(|p| p.is_deleted)
                {
                    record.parent_id = None;
                }
            }

            // 检查冲突：同 section 下同名
            let conflict = vault
                .list_objects(account_id, None, None, Some(&record.name), false, false)
                .map_err(|e| e.to_string())?
                .into_iter()
                .any(|o| {
                    o.name == record.name
                        && o.section_type == record.section_type
                        && o.id != record.id
                });

            let new_id = if conflict {
                let suffix = uuid::Uuid::new_v4()
                    .to_string()
                    .split('-')
                    .next()
                    .unwrap_or("restored")
                    .to_string();
                format!("{}_{}", record.id, suffix)
            } else {
                record.id.clone()
            };

            if conflict {
                record.id = new_id.clone();
                record.name = format!("{}（已恢复）", record.name);
            }

            vault.save_object(&record)?;
            vault.delete_trash_item(trash_id)?;
            let _ = vault.log_structured(
                "object_restore",
                if record.type_id == "page" {
                    "page"
                } else {
                    "object"
                },
                Some(&trash.original_id),
                Some(&trash.name_snapshot),
                "user",
                Some(&format!(
                    "section={} was_conflict={}",
                    record.section_type, conflict
                )),
            );

            new_id
        }
        "template" => {
            // 模板恢复：直接还原数据
            let template: solosoul_vault::UserTemplate =
                serde_json::from_slice(&trash.data).map_err(|e| format!("模板数据损坏: {}", e))?;
            vault.save_user_template(&template)?;
            vault.delete_trash_item(trash_id)?;
            template.name
        }
        _ => return Err(format!("不支持的回收站类型: {}", trash.item_type)),
    };

    Ok(RestoreResult { new_id })
}

/// 恢复结果。
pub struct RestoreResult {
    pub new_id: String,
}

/// 彻底删除回收站项目（含底层对象）。
pub fn purge_trash(vault: &VaultStore, trash_id: &str) -> Result<String, String> {
    let trash = vault
        .get_trash_item(trash_id)?
        .ok_or_else(|| format!("回收站项目 '{}' 不存在", trash_id))?;
    let name = trash.name_snapshot.clone();

    if trash.item_type != "template" {
        let _ = vault.delete_object(&trash.original_id, false);
    }
    vault.delete_trash_item(trash_id)?;
    let _ = vault.log_structured(
        "trash_permanent_delete",
        "trash_item",
        Some(trash_id),
        Some(&name),
        "user",
        Some(&format!("original_id={}", trash.original_id)),
    );

    Ok(name)
}

// ── Attachment helpers ─────────────────────────────────────

/// 从对象 properties 中读取附件列表。
pub fn load_attachments(props: &serde_json::Value) -> Vec<AttachmentMeta> {
    props
        .get("__attachments")
        .and_then(|v| serde_json::from_value::<Vec<AttachmentMeta>>(v.clone()).ok())
        .unwrap_or_default()
}

/// 将附件列表写回对象 properties。
pub fn save_attachments(props: &mut serde_json::Value, atts: &[AttachmentMeta]) {
    if let serde_json::Value::Object(ref mut obj) = props {
        obj.insert(
            "__attachments".to_string(),
            serde_json::to_value(atts).unwrap_or_default(),
        );
    }
}

/// 添加附件（复制文件 + 更新元数据）。
pub fn add_attachments(
    vault: &VaultStore,
    account_id: &str,
    object_id: &str,
    file_path: &Path,
    base_path: &Path,
) -> Result<AttachmentMeta, String> {
    let mut record = vault
        .load_object(object_id)?
        .ok_or_else(|| format!("对象 '{}' 不存在", object_id))?;
    if record.account_id != account_id || record.is_deleted {
        return Err("对象不存在或已被删除".to_string());
    }

    let mut atts = load_attachments(&record.properties);
    let active_count = atts.iter().filter(|a| a.deleted_at.is_none()).count();
    if active_count >= MAX_ACTIVE_ATTACHMENTS {
        return Err(format!(
            "单个对象最多保留 {} 个活跃附件",
            MAX_ACTIVE_ATTACHMENTS
        ));
    }

    if !file_path.exists() || !file_path.is_file() {
        return Err(format!("文件不存在或不是普通文件: {}", file_path.display()));
    }

    let file_name = sanitize_file_name(
        file_path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unnamed".to_string())
            .as_str(),
    );
    let size_bytes = file_path.metadata().map(|m| m.len()).unwrap_or(0);
    let mime_type = infer_mime_type(&file_name);
    let attachment_id = format!("att_{}", uuid::Uuid::new_v4());
    let created_at = chrono::Utc::now().to_rfc3339();

    // 复制文件到 vault 附件目录
    let vault_path =
        copy_file_to_vault(file_path, base_path, object_id, &attachment_id, &file_name)?;

    let meta = AttachmentMeta {
        id: attachment_id,
        object_id: object_id.to_string(),
        file_name,
        mime_type,
        size_bytes,
        created_at,
        deleted_at: None,
        src_path: Some(file_path.to_string_lossy().to_string()),
        vault_path: Some(vault_path),
    };

    atts.push(meta.clone());
    save_attachments(&mut record.properties, &atts);
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record)?;

    let _ = vault.log_structured(
        "attachment_add",
        "attachment",
        Some(object_id),
        Some(&record.name),
        "user",
        Some(&format!("file={}", file_path.display())),
    );

    Ok(meta)
}

/// 重命名附件。
pub fn rename_attachment(
    vault: &VaultStore,
    account_id: &str,
    object_id: &str,
    attachment_id: &str,
    new_name: &str,
) -> Result<(), String> {
    let mut record = vault
        .load_object(object_id)?
        .ok_or_else(|| format!("对象 '{}' 不存在", object_id))?;
    if record.account_id != account_id || record.is_deleted {
        return Err("对象不存在或已被删除".to_string());
    }

    let safe_name = sanitize_file_name(new_name);
    let mut atts = load_attachments(&record.properties);
    if let Some(a) = atts.iter_mut().find(|a| a.id == attachment_id) {
        a.file_name = safe_name.clone();
    } else {
        return Err(format!("附件 '{}' 不存在", attachment_id));
    }

    save_attachments(&mut record.properties, &atts);
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record)?;

    let _ = vault.log_structured(
        "attachment_rename",
        "attachment",
        Some(object_id),
        Some(attachment_id),
        "user",
        Some(&format!("new_name={}", safe_name)),
    );

    Ok(())
}

/// 软删除附件（标记 deleted_at）。
pub fn soft_delete_attachment(
    vault: &VaultStore,
    account_id: &str,
    object_id: &str,
    attachment_id: &str,
) -> Result<(), String> {
    let mut record = vault
        .load_object(object_id)?
        .ok_or_else(|| "对象不存在".to_string())?;
    if record.account_id != account_id || record.is_deleted {
        return Err("对象不存在或已被删除".to_string());
    }

    let mut atts = load_attachments(&record.properties);
    if let Some(a) = atts.iter_mut().find(|a| a.id == attachment_id) {
        a.deleted_at = Some(chrono::Utc::now().to_rfc3339());
    } else {
        return Err("附件不存在".to_string());
    }

    save_attachments(&mut record.properties, &atts);
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record)?;

    let _ = vault.log_structured(
        "attachment_soft_delete",
        "attachment",
        Some(object_id),
        Some(attachment_id),
        "user",
        None,
    );

    Ok(())
}

/// 恢复软删除的附件。
pub fn restore_attachment(
    vault: &VaultStore,
    account_id: &str,
    object_id: &str,
    attachment_id: &str,
) -> Result<(), String> {
    let mut record = vault
        .load_object(object_id)?
        .ok_or_else(|| "对象不存在".to_string())?;
    if record.account_id != account_id || record.is_deleted {
        return Err("对象不存在或已被删除".to_string());
    }

    let mut atts = load_attachments(&record.properties);
    if let Some(a) = atts.iter_mut().find(|a| a.id == attachment_id) {
        a.deleted_at = None;
    } else {
        return Err("附件不存在".to_string());
    }

    save_attachments(&mut record.properties, &atts);
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record)?;

    let _ = vault.log_structured(
        "attachment_restore",
        "attachment",
        Some(object_id),
        Some(attachment_id),
        "user",
        None,
    );

    Ok(())
}

/// 彻底删除附件（元数据 + 物理文件）。
pub fn purge_attachment(
    vault: &VaultStore,
    account_id: &str,
    object_id: &str,
    attachment_id: &str,
    base_path: &Path,
) -> Result<(), String> {
    let mut record = vault
        .load_object(object_id)?
        .ok_or_else(|| "对象不存在".to_string())?;
    if record.account_id != account_id || record.is_deleted {
        return Err("对象不存在或已被删除".to_string());
    }

    let atts: Vec<AttachmentMeta> = load_attachments(&record.properties)
        .into_iter()
        .filter(|a| a.id != attachment_id)
        .collect();

    // 删除物理文件
    let attachments_dir = base_path
        .join("attachments")
        .join(object_id)
        .join(attachment_id);
    if attachments_dir.exists() {
        let _ = std::fs::remove_dir_all(&attachments_dir);
    }

    save_attachments(&mut record.properties, &atts);
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;
    vault.save_object(&record)?;

    let _ = vault.log_structured(
        "attachment_purge",
        "attachment",
        Some(object_id),
        Some(attachment_id),
        "user",
        None,
    );

    Ok(())
}

/// 清理孤立附件文件（无元数据引用的附件）。
pub fn cleanup_orphan_attachments(
    vault: &VaultStore,
    account_id: &str,
    base_path: &Path,
) -> Result<(usize, u64), String> {
    let active_ids = load_all_referenced_attachment_ids(vault, account_id)?;
    let base_dir = base_path.join("attachments");

    if !base_dir.exists() {
        return Ok((0, 0));
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
            // 删除空对象目录
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

    Ok((removed, total_freed))
}

// ── Internal helpers ───────────────────────────────────────

fn save_creation_snapshot(
    vault: &VaultStore,
    object_id: &str,
    name: &str,
    properties: &serde_json::Value,
) -> Result<(), String> {
    let snapshot_data = serde_json::to_vec(&serde_json::json!({
        "name": name,
        "tags": Vec::<String>::new(),
        "properties": properties,
    }))
    .unwrap_or_default();
    let _ = vault.save_snapshot(object_id, "user_edit", &snapshot_data, "Created");
    Ok(())
}

fn copy_file_to_vault(
    src_path: &Path,
    base_path: &Path,
    object_id: &str,
    attachment_id: &str,
    file_name: &str,
) -> Result<String, String> {
    let src = src_path
        .canonicalize()
        .map_err(|e| format!("无效的源文件路径: {}", e))?;

    let vault_base = base_path
        .canonicalize()
        .map_err(|e| format!("无效的 vault 基目录: {}", e))?;

    if src.starts_with(&vault_base) {
        return Err("源文件路径不能位于 vault 存储目录内".to_string());
    }

    let dest_dir = vault_base
        .join("attachments")
        .join(object_id)
        .join(attachment_id);
    std::fs::create_dir_all(&dest_dir).map_err(|e| format!("创建目录失败: {}", e))?;

    let safe_name = sanitize_file_name(file_name);
    let dest_path = dest_dir.join(&safe_name);
    std::fs::copy(&src, &dest_path).map_err(|e| format!("复制文件失败: {}", e))?;
    Ok(dest_path.to_string_lossy().to_string())
}

fn sanitize_file_name(file_name: &str) -> String {
    Path::new(file_name)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unnamed".to_string())
}

fn infer_mime_type(file_name: &str) -> String {
    let ext = Path::new(file_name)
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

fn load_all_referenced_attachment_ids(
    vault: &VaultStore,
    account_id: &str,
) -> Result<HashSet<String>, String> {
    let objects = vault.list_objects(account_id, None, None, None, false, false)?;
    let ids: Vec<String> = objects.iter().map(|s| s.id.clone()).collect();
    let loaded = vault.load_objects_batch(&ids)?;

    let mut active_ids = HashSet::new();
    for rec in loaded.values() {
        for a in load_attachments(&rec.properties) {
            active_ids.insert(a.id.clone());
        }
    }
    Ok(active_ids)
}

/// 解析保留期字符串（如 \"30d\"、\"6m\"）为毫秒值。
pub fn parse_retention_ms(period: &str) -> i64 {
    match period {
        "7d" => 7 * 24 * 3600 * 1000,
        "30d" => 30 * 24 * 3600 * 1000,
        "60d" => 60 * 24 * 3600 * 1000,
        "half_year" | "half-year" | "6m" => 180 * 24 * 3600 * 1000,
        _ => {
            if let Ok(days) = period.trim_end_matches('d').parse::<i64>() {
                days * 24 * 3600 * 1000
            } else {
                30 * 24 * 3600 * 1000 // default 30d
            }
        }
    }
}

/// 从 profile 加载回收站保留期。
pub fn load_trash_retention(vault: &VaultStore, account_id: &str) -> i64 {
    if let Ok(Some(profile)) = vault.load_profile(account_id) {
        if !profile.data.is_empty() {
            if let Ok(data) = serde_json::from_slice::<serde_json::Value>(&profile.data) {
                if let Some(ret) = data
                    .pointer("/preferences/trashRetention")
                    .and_then(|v| v.as_str())
                {
                    return parse_retention_ms(ret);
                }
            }
        }
    }
    30 * 24 * 3600 * 1000 // default 30d
}

// ════════════════════════════════════════════════════════════════
// Tests
// ════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VaultService;
    use std::sync::Arc;
    use tempfile::TempDir;

    static CORE_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn test_setup() -> (Arc<VaultStore>, String, TempDir) {
        let _guard = CORE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = TempDir::new().unwrap();
        let vault = VaultService::with_base_path(dir.path().to_path_buf());
        let account = vault.create_account("Test", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap().to_string();
        let vault_store = vault.get_vault_store().unwrap();
        (vault_store, account_id, dir)
    }

    #[test]
    fn test_create_page() {
        let (vault, account_id, _dir) = test_setup();
        let page = create_page(&vault, &account_id, "旅行").unwrap();
        assert_eq!(page.name, "旅行");
        assert_eq!(page.type_id, "page");

        let pages = vault
            .list_objects(&account_id, Some("page"), None, None, false, false)
            .unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].name, "旅行");
    }

    #[test]
    fn test_create_duplicate_page_fails() {
        let (vault, account_id, _dir) = test_setup();
        create_page(&vault, &account_id, "旅行").unwrap();
        let result = create_page(&vault, &account_id, "旅行");
        assert!(result.is_err());
    }

    #[test]
    fn test_create_object_with_page_update() {
        let (vault, account_id, _dir) = test_setup();
        let page = create_page(&vault, &account_id, "旅行").unwrap();

        let obj = create_object(
            &vault,
            &account_id,
            &page.id,
            "我的笔记",
            serde_json::json!({"content": "hello"}),
            None,
            None,
        )
        .unwrap();

        assert_eq!(obj.name, "我的笔记");
        assert!(obj.id.starts_with("obj_"));

        // 验证父页面 children_ids 已更新
        let updated_page = vault.load_object(&page.id).unwrap().unwrap();
        assert!(updated_page.children_ids.contains(&obj.id));
    }

    #[test]
    fn test_update_object() {
        let (vault, account_id, _dir) = test_setup();
        let page = create_page(&vault, &account_id, "旅行").unwrap();
        let mut obj = create_object(
            &vault,
            &account_id,
            &page.id,
            "旧名称",
            serde_json::json!({"title": "old"}),
            None,
            None,
        )
        .unwrap();

        obj.name = "新名称".to_string();
        update_object(&vault, &mut obj).unwrap();

        let loaded = vault.load_object(&obj.id).unwrap().unwrap();
        assert_eq!(loaded.name, "新名称");
        assert_eq!(loaded.version, 2);
    }

    #[test]
    fn test_move_to_trash_and_restore() {
        let (vault, account_id, _dir) = test_setup();
        let page = create_page(&vault, &account_id, "旅行").unwrap();
        let obj = create_object(
            &vault,
            &account_id,
            &page.id,
            "待删除",
            serde_json::json!({}),
            None,
            None,
        )
        .unwrap();

        // 移入回收站
        move_to_trash(&vault, &obj, "object", None, 3600000).unwrap();
        let loaded = vault.load_object(&obj.id).unwrap().unwrap();
        assert!(loaded.is_deleted);

        // 验证存在回收站记录
        let trash_items = vault.list_trash_items(None, None).unwrap();
        assert_eq!(trash_items.len(), 1);

        // 恢复
        let result = restore_from_trash(&vault, &account_id, &trash_items[0].id).unwrap();
        assert_eq!(result.new_id, obj.id);
        let restored = vault.load_object(&obj.id).unwrap().unwrap();
        assert!(!restored.is_deleted);
    }

    #[test]
    fn test_attachment_add_and_rename() {
        let (vault, account_id, dir) = test_setup();
        let page = create_page(&vault, &account_id, "测试").unwrap();
        let obj = create_object(
            &vault,
            &account_id,
            &page.id,
            "测试对象",
            serde_json::json!({}),
            None,
            None,
        )
        .unwrap();

        // 源文件必须在 vault 目录之外
        let src_dir = tempfile::TempDir::new().unwrap();
        let file_path = src_dir.path().join("test.txt");
        std::fs::write(&file_path, "hello").unwrap();

        let meta = add_attachments(&vault, &account_id, &obj.id, &file_path, dir.path()).unwrap();
        assert_eq!(meta.file_name, "test.txt");
        assert_eq!(meta.size_bytes, 5);

        let record = vault.load_object(&obj.id).unwrap().unwrap();
        let atts = load_attachments(&record.properties);
        assert_eq!(atts.len(), 1);

        // 重命名
        rename_attachment(&vault, &account_id, &obj.id, &meta.id, "newname.txt").unwrap();
        let record = vault.load_object(&obj.id).unwrap().unwrap();
        let atts = load_attachments(&record.properties);
        assert_eq!(atts[0].file_name, "newname.txt");
    }

    #[test]
    fn test_sanitize_file_name() {
        assert_eq!(sanitize_file_name("../../../etc/passwd"), "passwd");
        assert_eq!(sanitize_file_name("/tmp/file.txt"), "file.txt");
        assert_eq!(sanitize_file_name("normal.txt"), "normal.txt");
    }

    #[test]
    fn test_parse_retention_ms() {
        assert_eq!(parse_retention_ms("7d"), 7 * 24 * 3600 * 1000);
        assert_eq!(parse_retention_ms("30d"), 30 * 24 * 3600 * 1000);
        assert_eq!(parse_retention_ms("6m"), 180 * 24 * 3600 * 1000);
        assert_eq!(parse_retention_ms("half_year"), 180 * 24 * 3600 * 1000);
    }
}
