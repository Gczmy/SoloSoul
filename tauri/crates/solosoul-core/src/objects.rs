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
        ignored_template_hash: None,
        created_at: now.clone(),
        updated_at: now,
        version: 1,
    };

    vault.save_object(&record)?;
    save_creation_snapshot(vault, &id, name, &serde_json::json!({}), None)?;
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
        ignored_template_hash: None,
        created_at: now.clone(),
        updated_at: now,
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

    save_creation_snapshot(
        vault,
        &id,
        &name,
        &record.properties,
        record.property_labels.as_ref(),
    )?;
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
        "propertyLabels": object.property_labels,
    }))
    .unwrap_or_default();
    let _ = vault.save_snapshot(&object.id, "user_edit", &snapshot_data, "diff_updated");
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
        "contract_type_id": record.contract_type_id,
        "template_hash": record.template_hash,
        "ignored_template_hash": record.ignored_template_hash,
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

/// 从回收站恢复单个对象，含冲突处理、级联恢复父页面、页面桩重建。
pub fn restore_from_trash(vault: &VaultStore, trash_id: &str) -> Result<RestoreResult, String> {
    restore_from_trash_with_lang(vault, trash_id, "en-US")
}

/// 带语言参数的回收站恢复入口。
pub fn restore_from_trash_with_lang(
    vault: &VaultStore,
    trash_id: &str,
    lang: &str,
) -> Result<RestoreResult, String> {
    let trash = vault
        .get_trash_item(trash_id)?
        .ok_or_else(|| "回收站项目不存在".to_string())?;

    match trash.item_type.as_str() {
        "page" => restore_page(vault, &trash, lang),
        "object" => restore_object(vault, &trash, lang),
        "template" => restore_template(vault, &trash, trash_id),
        _ => Err(format!("不支持的回收站类型: {}", trash.item_type)),
    }
}

/// 恢复页面类型回收站项，并级联恢复其下所有子对象。
fn restore_page(
    vault: &VaultStore,
    trash: &TrashItem,
    lang: &str,
) -> Result<RestoreResult, String> {
    let (page_record, _) = restore_single_object(vault, trash, lang)?;
    let page_id = page_record.id.clone();
    let page_name = page_record.name.clone();
    vault.delete_trash_item(&trash.id)?;

    let mut cascaded_count = 0u32;
    let children = find_child_objects_in_trash(vault, &page_id)?;
    for child_trash in &children {
        if let Ok((_, _)) = restore_single_object(vault, child_trash, lang) {
            vault.delete_trash_item(&child_trash.id)?;
            let _ = vault.log_structured(
                "object_restore",
                "object",
                Some(&child_trash.original_id),
                Some(&child_trash.name_snapshot),
                "user",
                Some(&format!("cascaded_from_page={}", page_id)),
            );
            cascaded_count += 1;
        }
    }

    let _ = vault.log_structured(
        "page_restore",
        "page",
        Some(&page_id),
        Some(&page_name),
        "user",
        Some(&format!("count={}", cascaded_count)),
    );

    Ok(RestoreResult {
        restored_id: page_id,
        restored_name: page_name,
        cascaded_page_name: None,
        cascaded_count,
        rebuilt_page_name: None,
    })
}

/// 恢复对象类型回收站项。若所属自定义页面缺失：在回收站则级联恢复，已永久删除则重建页面桩。
fn restore_object(
    vault: &VaultStore,
    trash: &TrashItem,
    lang: &str,
) -> Result<RestoreResult, String> {
    let record_data: serde_json::Value =
        serde_json::from_slice(&trash.data).map_err(|e| format!("回收站数据损坏: {}", e))?;

    let account_id = read_str(&record_data, "account_id", "accountId").unwrap_or("imported");
    let target_section = trash
        .original_section_type
        .as_deref()
        .or_else(|| record_data["sectionType"].as_str())
        .or_else(|| record_data["section_type"].as_str())
        .unwrap_or("identity")
        .to_string();

    let mut cascaded_page_name: Option<String> = None;
    let mut rebuilt_page_name: Option<String> = None;

    if !is_built_in_section(&target_section) && uuid::Uuid::parse_str(&target_section).is_ok() {
        let page_exists = vault
            .load_object(&target_section)
            .ok()
            .flatten()
            .map(|o| !o.is_deleted)
            .unwrap_or(false);

        if !page_exists {
            if let Ok(Some(page_trash)) = find_page_in_trash(vault, &target_section) {
                let (page_record, _) = restore_single_object(vault, &page_trash, lang)?;
                vault.delete_trash_item(&page_trash.id)?;
                cascaded_page_name = Some(page_record.name.clone());
                let _ = vault.log_structured(
                    "page_restore",
                    "page",
                    Some(&page_record.id),
                    Some(&page_record.name),
                    "user",
                    Some("cascaded_from_object_restore"),
                );
            } else {
                let raw_name = record_data["parentPageName"].as_str().unwrap_or("");
                let page_name = if raw_name.is_empty() {
                    recovered_page_name(lang).to_string()
                } else {
                    raw_name.to_string()
                };
                let page_icon = record_data["parentPageIcon"].as_str().unwrap_or("folder");
                let stub =
                    rebuild_page_stub(vault, &target_section, account_id, &page_name, page_icon)?;
                rebuilt_page_name = Some(stub.name.clone());
                let _ = vault.log_structured(
                    "page_create",
                    "page",
                    Some(&stub.id),
                    Some(&stub.name),
                    "user",
                    Some("rebuilt_stub_from_object_restore"),
                );
            }
        }
    }

    let (record, was_conflict) = restore_single_object(vault, trash, lang)?;
    vault.delete_trash_item(&trash.id)?;
    let _ = vault.log_structured(
        "object_restore",
        "object",
        Some(&trash.original_id),
        Some(&trash.name_snapshot),
        "user",
        Some(&format!(
            "section={} was_conflict={}",
            target_section, was_conflict
        )),
    );

    Ok(RestoreResult {
        restored_id: record.id,
        restored_name: record.name,
        cascaded_page_name,
        cascaded_count: 0,
        rebuilt_page_name,
    })
}

fn restore_template(
    vault: &VaultStore,
    trash: &TrashItem,
    trash_id: &str,
) -> Result<RestoreResult, String> {
    let template: solosoul_vault::UserTemplate =
        serde_json::from_slice(&trash.data).map_err(|e| format!("模板数据损坏: {}", e))?;
    let name = template.name.clone();
    vault.save_user_template(&template)?;
    vault.delete_trash_item(trash_id)?;
    Ok(RestoreResult {
        restored_id: template.id,
        restored_name: name,
        cascaded_page_name: None,
        cascaded_count: 0,
        rebuilt_page_name: None,
    })
}

// ── Restore helpers ────────────────────────────────────────

const BUILT_IN_SECTIONS: &[&str] = &["identity", "travel", "financial", "professional"];

fn is_built_in_section(section_type: &str) -> bool {
    BUILT_IN_SECTIONS.contains(&section_type)
}

fn read_str<'a>(data: &'a serde_json::Value, snake: &str, camel: &str) -> Option<&'a str> {
    data[camel].as_str().or_else(|| data[snake].as_str())
}

fn read_array(data: &serde_json::Value, snake: &str, camel: &str) -> Option<Vec<String>> {
    data[camel]
        .as_array()
        .or_else(|| data[snake].as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
}

fn restored_suffix(lang: &str) -> &'static str {
    match lang {
        "zh-CN" => "（已恢复）",
        _ => " (restored)",
    }
}

fn recovered_page_name(lang: &str) -> &'static str {
    match lang {
        "zh-CN" => "已恢复的页面",
        _ => "Recovered Page",
    }
}

fn inherit_contract_type_id(vault: &VaultStore, template_id: Option<&str>) -> Option<String> {
    template_id.and_then(|tid| {
        vault
            .load_user_template(tid)
            .ok()
            .flatten()
            .and_then(|t| t.contract_type_id)
    })
}

fn find_page_in_trash(vault: &VaultStore, page_id: &str) -> Result<Option<TrashItem>, String> {
    let all = vault.list_trash_items(None, None)?;
    for item in &all {
        if item.item_type == "page" && item.original_id == page_id {
            return vault.get_trash_item(&item.id);
        }
    }
    Ok(None)
}

fn find_child_objects_in_trash(
    vault: &VaultStore,
    section_type: &str,
) -> Result<Vec<TrashItem>, String> {
    let all = vault.list_trash_items(None, None)?;
    let mut out = Vec::new();
    for item in &all {
        if item.item_type == "object" {
            if let Ok(Some(full)) = vault.get_trash_item(&item.id) {
                if full.original_section_type.as_deref() == Some(section_type) {
                    out.push(full);
                }
            }
        }
    }
    Ok(out)
}

fn rebuild_page_stub(
    vault: &VaultStore,
    page_id: &str,
    account_id: &str,
    page_name: &str,
    icon_name: &str,
) -> Result<ObjectRecord, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let record = ObjectRecord {
        id: page_id.to_string(),
        account_id: account_id.to_string(),
        type_id: "page".to_string(),
        section_type: page_id.to_string(),
        name: page_name.to_string(),
        icon_name: icon_name.to_string(),
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
        template_hash: None,
        ignored_template_hash: None,
        created_at: now.clone(),
        updated_at: now,
        version: 1,
        contract_type_id: None,
    };
    vault.save_object(&record)?;
    Ok(record)
}

/// Restore a single non-deleted object from a trash item.
/// Returns the restored ObjectRecord and whether the ID was changed due to a name conflict.
fn restore_single_object(
    vault: &VaultStore,
    trash: &TrashItem,
    lang: &str,
) -> Result<(ObjectRecord, bool), String> {
    let record_data: serde_json::Value =
        serde_json::from_slice(&trash.data).map_err(|e| format!("Invalid trash data: {}", e))?;

    let target_section = trash
        .original_section_type
        .as_deref()
        .or_else(|| record_data["sectionType"].as_str())
        .or_else(|| record_data["section_type"].as_str())
        .unwrap_or("identity");

    let account_id = read_str(&record_data, "account_id", "accountId").unwrap_or("imported");
    let objects = vault
        .list_objects(
            account_id,
            None,
            None,
            Some(&trash.name_snapshot),
            false,
            false,
        )
        .unwrap_or_default();
    let exists = objects
        .iter()
        .any(|o| o.name == trash.name_snapshot && o.section_type == target_section);

    let suffix = restored_suffix(lang);

    let new_id = if exists {
        format!(
            "{}_{}",
            trash.original_id,
            uuid::Uuid::new_v4()
                .to_string()
                .split('-')
                .next()
                .unwrap_or("restored")
        )
    } else {
        trash.original_id.clone()
    };

    let new_name = if exists {
        format!("{}{}", trash.name_snapshot, suffix)
    } else {
        trash.name_snapshot.clone()
    };

    let contract_type_id =
        inherit_contract_type_id(vault, read_str(&record_data, "template_id", "templateId"));

    let now = chrono::Utc::now().to_rfc3339();
    let record = ObjectRecord {
        contract_type_id,
        id: new_id.clone(),
        account_id: read_str(&record_data, "account_id", "accountId")
            .unwrap_or("imported")
            .to_string(),
        type_id: read_str(&record_data, "type_id", "typeId")
            .unwrap_or("note")
            .to_string(),
        section_type: target_section.to_string(),
        name: new_name,
        icon_name: read_str(&record_data, "icon_name", "iconName")
            .unwrap_or("document")
            .to_string(),
        parent_id: read_str(&record_data, "parent_id", "parentId").map(String::from),
        children_ids: read_array(&record_data, "children_ids", "childrenIds").unwrap_or_default(),
        properties: record_data["properties"].clone(),
        property_labels: if record_data["propertyLabels"].is_null() {
            if record_data["property_labels"].is_null() {
                None
            } else {
                Some(record_data["property_labels"].clone())
            }
        } else {
            Some(record_data["propertyLabels"].clone())
        },
        sensitivity_level: read_str(&record_data, "sensitivity_level", "sensitivityLevel")
            .unwrap_or("internal")
            .to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: record_data["tags"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        template_id: read_str(&record_data, "template_id", "templateId").map(String::from),
        template_type: read_str(&record_data, "template_type", "templateType").map(String::from),
        template_hash: read_str(&record_data, "template_hash", "templateHash").map(String::from),
        ignored_template_hash: read_str(
            &record_data,
            "ignored_template_hash",
            "ignoredTemplateHash",
        )
        .map(String::from),
        created_at: read_str(&record_data, "created_at", "createdAt")
            .unwrap_or(&now)
            .to_string(),
        updated_at: now.clone(),
        version: record_data["version"].as_u64().unwrap_or(1) as u32,
    };

    vault.save_object(&record)?;
    if new_id != trash.original_id {
        let _ = vault.copy_snapshots(&trash.original_id, &new_id);
    }

    Ok((record, exists))
}

/// 恢复结果。
/// 同时供 GUI 与 CLI 使用；Tauri 命令会把它映射到 camelCase 的 RestoreOutcome。
pub struct RestoreResult {
    pub restored_id: String,
    pub restored_name: String,
    pub cascaded_page_name: Option<String>,
    pub cascaded_count: u32,
    pub rebuilt_page_name: Option<String>,
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
    property_labels: Option<&serde_json::Value>,
) -> Result<(), String> {
    let snapshot_data = serde_json::to_vec(&serde_json::json!({
        "name": name,
        "tags": Vec::<String>::new(),
        "properties": properties,
        "propertyLabels": property_labels,
    }))
    .unwrap_or_default();
    let _ = vault.save_snapshot(object_id, "user_edit", &snapshot_data, "diff_created");
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
        let result = restore_from_trash(&vault, &trash_items[0].id).unwrap();
        assert_eq!(result.restored_id, obj.id);
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

    /// 构造一个自定义页面：id 与 section_type 均为 page_id，更贴近真实应用。
    fn make_custom_page(
        vault: &VaultStore,
        account_id: &str,
        page_id: &str,
        name: &str,
    ) -> ObjectRecord {
        let now = chrono::Utc::now().to_rfc3339();
        let page = ObjectRecord {
            id: page_id.to_string(),
            account_id: account_id.to_string(),
            type_id: "page".to_string(),
            section_type: page_id.to_string(),
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
            template_hash: None,
            ignored_template_hash: None,
            created_at: now.clone(),
            updated_at: now,
            version: 1,
            contract_type_id: None,
        };
        vault.save_object(&page).unwrap();
        page
    }

    #[test]
    fn test_restore_object_cascades_parent_page() {
        let (vault, account_id, _dir) = test_setup();
        let page_id = uuid::Uuid::new_v4().to_string();
        let page = make_custom_page(&vault, &account_id, &page_id, "CustomPage");

        let mut obj = create_object(
            &vault,
            &account_id,
            &page.id,
            "MyObject",
            serde_json::json!({}),
            None,
            None,
        )
        .unwrap();
        obj.section_type = page_id.clone();
        vault.save_object(&obj).unwrap();

        move_to_trash(&vault, &obj, "object", Some(page.id.clone()), 3600000).unwrap();
        move_to_trash(&vault, &page, "page", None, 3600000).unwrap();

        let obj_trash = vault
            .list_trash_items(None, None)
            .unwrap()
            .into_iter()
            .find(|t| t.item_type == "object")
            .unwrap();

        let result = restore_from_trash(&vault, &obj_trash.id).unwrap();

        assert_eq!(result.restored_id, obj.id);
        assert_eq!(result.cascaded_page_name.as_deref(), Some("CustomPage"));
        assert!(result.rebuilt_page_name.is_none());
        assert_eq!(result.cascaded_count, 0);

        assert!(!vault.load_object(&page.id).unwrap().unwrap().is_deleted);
        assert!(!vault.load_object(&obj.id).unwrap().unwrap().is_deleted);
    }

    #[test]
    fn test_restore_object_rebuilds_page_stub() {
        let (vault, account_id, _dir) = test_setup();
        let page_id = uuid::Uuid::new_v4().to_string();

        let mut obj = create_object(
            &vault,
            &account_id,
            &page_id,
            "OrphanObject",
            serde_json::json!({}),
            None,
            None,
        )
        .unwrap();
        obj.section_type = page_id.clone();
        vault.save_object(&obj).unwrap();

        // 手动构造带 parentPageName 的 TrashItem（模拟 page_delete 写入的格式）
        let full_record = serde_json::json!({
            "id": obj.id,
            "accountId": obj.account_id,
            "typeId": obj.type_id,
            "sectionType": obj.section_type,
            "name": obj.name,
            "iconName": obj.icon_name,
            "parentId": obj.parent_id,
            "childrenIds": obj.children_ids,
            "properties": obj.properties,
            "propertyLabels": obj.property_labels,
            "sensitivityLevel": obj.sensitivity_level,
            "tags": obj.tags_json,
            "createdAt": obj.created_at,
            "updatedAt": obj.updated_at,
            "version": obj.version,
            "templateId": obj.template_id,
            "templateType": obj.template_type,
            "contractTypeId": obj.contract_type_id,
            "templateHash": obj.template_hash,
            "parentPageName": "My Lost Page",
        });
        let trash = TrashItem {
            id: format!("trash_{}", uuid::Uuid::new_v4()),
            item_type: "object".to_string(),
            original_id: obj.id.clone(),
            original_parent_id: None,
            original_section_type: Some(obj.section_type.clone()),
            original_sort_order: None,
            data: serde_json::to_vec(&full_record).unwrap_or_default(),
            deleted_at: chrono::Utc::now().timestamp_millis(),
            expires_at: Some(chrono::Utc::now().timestamp_millis() + 3600000),
            deleted_by: "user".to_string(),
            name_snapshot: obj.name.clone(),
            icon_snapshot: Some(obj.icon_name.clone()),
        };
        vault.save_trash_item(&trash).unwrap();
        vault.delete_object(&obj.id, true).unwrap();

        let result = restore_from_trash(&vault, &trash.id).unwrap();

        assert_eq!(result.restored_id, obj.id);
        assert_eq!(result.rebuilt_page_name.as_deref(), Some("My Lost Page"));
        assert!(result.cascaded_page_name.is_none());
        assert_eq!(result.cascaded_count, 0);

        let stub = vault.load_object(&page_id).unwrap().unwrap();
        assert_eq!(stub.name, "My Lost Page");
        assert_eq!(stub.type_id, "page");
        assert!(!stub.is_deleted);
    }

    #[test]
    fn test_restore_page_cascades_objects() {
        let (vault, account_id, _dir) = test_setup();
        let page_id = uuid::Uuid::new_v4().to_string();
        let page = make_custom_page(&vault, &account_id, &page_id, "ParentPage");

        let mut obj = create_object(
            &vault,
            &account_id,
            &page.id,
            "ChildObj",
            serde_json::json!({}),
            None,
            None,
        )
        .unwrap();
        obj.section_type = page_id.clone();
        vault.save_object(&obj).unwrap();

        move_to_trash(&vault, &obj, "object", Some(page.id.clone()), 3600000).unwrap();
        move_to_trash(&vault, &page, "page", None, 3600000).unwrap();

        let page_trash = vault
            .list_trash_items(None, None)
            .unwrap()
            .into_iter()
            .find(|t| t.item_type == "page")
            .unwrap();

        let result = restore_from_trash(&vault, &page_trash.id).unwrap();

        assert_eq!(result.restored_id, page.id);
        assert_eq!(result.cascaded_count, 1);

        assert!(!vault.load_object(&page.id).unwrap().unwrap().is_deleted);
        assert!(!vault.load_object(&obj.id).unwrap().unwrap().is_deleted);
    }

    #[test]
    fn test_parse_retention_ms() {
        assert_eq!(parse_retention_ms("7d"), 7 * 24 * 3600 * 1000);
        assert_eq!(parse_retention_ms("30d"), 30 * 24 * 3600 * 1000);
        assert_eq!(parse_retention_ms("6m"), 180 * 24 * 3600 * 1000);
        assert_eq!(parse_retention_ms("half_year"), 180 * 24 * 3600 * 1000);
    }
}
