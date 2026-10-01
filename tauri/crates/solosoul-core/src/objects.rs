//! Object, attachment, and trash management — core business logic.
//!
//! This module provides high-level operations for creating, updating, deleting,
//! and managing objects, attachments, and trash items. These functions are shared
//! by the CLI and Tauri GUI hosts.
//!
//! Each function takes `&VaultStore` + parameters and returns a result, leaving
//! UI concerns (prompts, state management, argument parsing) to the caller.

use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::Path;

use solosoul_vault::{ObjectRecord, PropertyType, TrashItem, VaultStore};

/// 导出附件元数据，与 GUI/CLI 共享。
pub use crate::export_import::AttachmentMeta;

/// RF-016：GUI/CLI 共用的可恢复附件清理入口。
pub use crate::attachment_cleanup::{
    purge_attachments, purge_attachments_for_session, retry_attachment_cleanup,
    retry_attachment_cleanup_for_session, CleanupReport,
};

mod create;
pub use create::{
    build_create_record, inherit_contract_type_id, inherit_property_fields,
    inherit_property_labels, inject_property_fields, inject_template_meta,
    project_template_properties, CreateRecordInput,
};

mod rollback;
pub use rollback::{rollback_object, RollbackError, RollbackErrorStage, RollbackOutcome};

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

    // P111: 仅需页面名称做重名校验，走 metadata-only 查询免全表解密。
    let pages = vault.list_object_metadata(account_id, Some("page"), None, false, false)?;
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

    let record = build_create_record(
        vault,
        account_id,
        CreateRecordInput {
            id: id.clone(),
            type_id,
            section_type: "identity".to_string(),
            name: name.clone(),
            icon_name: icon,
            parent_id: Some(page_id.to_string()),
            properties,
            template_id: template_id.map(str::to_string),
            template_type: template_id.map(|_| "user".to_string()),
        },
        &now,
    )?;

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
    let mut consumed_trash_ids = vec![trash.id.clone()];

    let mut cascaded_count = 0u32;
    let children = find_child_objects_in_trash(vault, &page_id)?;
    for child_trash in &children {
        if let Ok((_, _)) = restore_single_object(vault, child_trash, lang) {
            vault.delete_trash_item(&child_trash.id)?;
            consumed_trash_ids.push(child_trash.id.clone());
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
        consumed_trash_ids,
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
    let mut consumed_trash_ids = vec![trash.id.clone()];

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
                consumed_trash_ids.push(page_trash.id.clone());
                cascaded_page_name = Some(page_record.name.clone());
                let cascade_details = serde_json::json!({
                    "cascadedFromObject": true,
                    "pageName": page_record.name,
                });
                let _ = vault.log_structured(
                    "page_restore",
                    "page",
                    Some(&page_record.id),
                    Some(&page_record.name),
                    "user",
                    Some(&cascade_details.to_string()),
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
        consumed_trash_ids,
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
        consumed_trash_ids: vec![trash.id.clone()],
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
    // P111: 仅需 name+section 判断同名冲突，走 metadata-only 查询免全表解密。
    let objects = vault
        .list_object_metadata(account_id, None, None, false, false)
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
    pub consumed_trash_ids: Vec<String>,
}

/// 彻底删除回收站项目（含底层对象）。
pub fn purge_trash(vault: &VaultStore, trash_id: &str) -> Result<String, String> {
    let trash = vault
        .get_trash_item(trash_id)?
        .ok_or_else(|| format!("回收站项目 '{}' 不存在", trash_id))?;
    let name = trash.name_snapshot.clone();

    if trash.item_type != "template" {
        // R2-07: 底层对象删除失败必须中止（并保留 trash 记录），
        // 否则 trash 记录被删后留下无法再经回收站清理的孤儿对象行。
        vault.delete_object(&trash.original_id, false)?;
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
/// `attachment_key`：P001 附件静态加密密钥（`Some` 时加密落盘；`None` 保持旧明文行为，
/// 供无会话密钥的宿主/测试使用——读取端自动兼容两种形态）。
pub fn add_attachments(
    vault: &VaultStore,
    account_id: &str,
    object_id: &str,
    file_path: &Path,
    base_path: &Path,
    attachment_key: Option<&[u8; 32]>,
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

    // 复制文件到 vault 附件目录（P001: 提供密钥时加密落盘）
    let vault_path = copy_file_to_vault(
        file_path,
        base_path,
        object_id,
        &attachment_id,
        &file_name,
        attachment_key,
    )?;

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
        description: None,
        tags: vec![],
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

/// 兼容永久删除入口；未完成物理清理时返回固定 pending 错误。
/// 新 GUI/CLI 使用返回 CleanupReport 的会话入口，以发布已提交的元数据变化。
pub fn purge_attachment(
    vault: &VaultStore,
    account_id: &str,
    object_id: &str,
    attachment_id: &str,
    base_path: &Path,
) -> Result<(), String> {
    let record = vault
        .load_object(object_id)?
        .ok_or_else(|| "对象不存在".to_string())?;
    if record.account_id != account_id || record.is_deleted {
        return Err("对象不存在或已被删除".to_string());
    }
    let report = purge_attachments(
        vault,
        account_id,
        object_id,
        &[attachment_id.to_string()],
        base_path,
    )?;
    if report.pending != 0 {
        return Err("attachment_cleanup_pending".to_string());
    }
    Ok(())
}

// 广泛孤儿清理由 orphan_cleanup 模块统一提供原会话与排他维护入口。

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
    attachment_key: Option<&[u8; 32]>,
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
    match attachment_key {
        // P001: 附件加密落盘（SOLC 头）
        Some(key) => {
            crate::attachment_crypto::encrypt_file_stream(key, &src, &dest_path)
                .map_err(|e| format!("复制文件失败: {}", e))?;
        }
        None => {
            std::fs::copy(&src, &dest_path).map_err(|e| format!("复制文件失败: {}", e))?;
        }
    }
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

/// P017：跨 crate 共享——收集全部对象 `__attachments` 引用的附件 ID 集合。
/// 只读取当前存活对象的引用，用于显示/只读查询，不能作为孤儿删除授权；
/// 消除 solo_soul 侧的 test-only 重复版。
pub fn load_all_referenced_attachment_ids(
    vault: &VaultStore,
    account_id: &str,
) -> Result<HashSet<String>, String> {
    // P111: 只需对象 ID 集合，随后 load_objects_batch 拉全量，走 metadata-only 查询。
    let objects = vault.list_object_metadata(account_id, None, None, false, false)?;
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

// --- object property validation / template fingerprint (P031) ---

/// 校验 properties 中的 dynamic_group 字段值。
/// 要求：数组；每个元素含 id/name/type/value；type 可被解析；不超出 maxItems；类型在 allowedTypes 内。
pub fn validate_dynamic_groups(properties: &serde_json::Value) -> Result<(), String> {
    let fields = match properties.get("__fields").and_then(|v| v.as_object()) {
        Some(f) => f,
        None => return Ok(()),
    };
    let props = match properties.as_object() {
        Some(p) => p,
        None => return Ok(()),
    };

    for (key, field_def) in fields {
        if field_def.get("type").and_then(|v| v.as_str()) != Some("dynamic_group") {
            continue;
        }
        let value = match props.get(key) {
            Some(v) => v,
            None => continue,
        };
        let items = value
            .as_array()
            .ok_or_else(|| format!("字段 '{}' 是动态字段组，其值必须是数组", key))?;

        let allowed: Option<Vec<&str>> = field_def
            .get("allowedTypes")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect());
        let max_items = field_def
            .get("maxItems")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize);

        if let Some(max) = max_items {
            if items.len() > max {
                return Err(format!(
                    "字段 '{}' 最多允许 {} 个子字段，当前 {} 个",
                    key,
                    max,
                    items.len()
                ));
            }
        }

        for (idx, item) in items.iter().enumerate() {
            let obj = item
                .as_object()
                .ok_or_else(|| format!("字段 '{}' 的第 {} 个子字段必须是对象", key, idx + 1))?;
            for required in ["id", "name", "type", "value"] {
                if !obj.contains_key(required) {
                    return Err(format!(
                        "字段 '{}' 的第 {} 个子字段缺少 '{}'",
                        key,
                        idx + 1,
                        required
                    ));
                }
            }
            let child_type = obj
                .get("type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("字段 '{}' 的第 {} 个子字段 type 无效", key, idx + 1))?;
            if PropertyType::parse(child_type).is_none() {
                return Err(format!(
                    "字段 '{}' 的第 {} 个子字段类型 '{}' 不存在",
                    key,
                    idx + 1,
                    child_type
                ));
            }
            if let Some(ref allowed) = allowed {
                if !allowed.contains(&child_type) {
                    return Err(format!(
                        "字段 '{}' 的第 {} 个子字段类型 '{}' 不在允许列表 {:?} 中",
                        key,
                        idx + 1,
                        child_type,
                        allowed
                    ));
                }
            }
        }
    }
    Ok(())
}

/// 计算模板指纹，用于判断对象是否需要同步模板更新。
/// 排除 id/account_id/created_at/updated_at，按字段 id 稳定排序后序列化再取 SHA-256 前 16 位。
pub fn template_fingerprint(tpl: &solosoul_vault::UserTemplate) -> String {
    let mut props: Vec<&solosoul_vault::TemplateProperty> = tpl.properties.iter().collect();
    props.sort_by(|a, b| a.id.cmp(&b.id));
    let canonical = serde_json::json!({
        "properties": props,
    });
    let bytes = serde_json::to_vec(&canonical).unwrap_or_default();
    let hash = Sha256::digest(&bytes);
    hex::encode(&hash[..8])
}

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

    fn rf009_setup() -> (TempDir, VaultStore, String) {
        let dir = TempDir::new().unwrap();
        let account = "rf009-account".to_string();
        let config = solosoul_vault::VaultConfig::new(&account, dir.path().to_path_buf())
            .with_data_key([0x42; 32]);
        let vault = VaultStore::open(config).unwrap();
        (dir, vault, account)
    }

    fn rf009_template(account: &str) -> solosoul_vault::UserTemplate {
        serde_json::from_value(serde_json::json!({
            "id": "rf009-template",
            "accountId": account,
            "name": "旅行凭证",
            "iconId": "template-icon",
            "createdAt": "2026-09-26T00:00:00Z",
            "contractTypeId": "travel.identity",
            "properties": [
                {"id": "public", "name": "姓名", "type": "text",
                 "sensitivityLevel": "public", "contractField": true},
                {"id": "internal", "name": "类别", "type": "select",
                 "sensitivityLevel": "internal", "options": ["A", "B"],
                 "deprecatedAt": "2026-09-20T00:00:00Z", "contractField": false,
                 "allowedTypes": ["number"], "maxItems": 9},
                {"id": "sensitive", "name": "电话", "type": "phone",
                 "sensitivityLevel": "sensitive"},
                {"id": "critical", "name": "凭证", "type": "text",
                 "sensitivityLevel": "critical"},
                {"id": "unlabelled", "name": "确认", "type": "boolean"},
                {"id": "group", "name": "其他信息", "type": "dynamic_group",
                 "sensitivityLevel": "public", "allowedTypes": ["text", "phone"],
                 "maxItems": 2}
            ]
        }))
        .unwrap()
    }

    #[test]
    fn rf009_inherits_template_metadata_and_preserves_input() {
        let (_dir, vault, account) = rf009_setup();
        let page = create_page(&vault, &account, "旅行").unwrap();
        let template = rf009_template(&account);
        vault.save_user_template(&template).unwrap();
        let input = serde_json::json!({
            "public": "", "internal": "B", "sensitive": "+123",
            "critical": "user-secret", "unlabelled": false,
            "group": [{"id": "child", "name": "补充", "type": "text", "value": ""}],
            "zero": 0, "empty": null, "extra": {"nested": ["value"]},
            "__custom": "keep",
            "__fields": {"old": {"name": "旧字段", "type": "number"}},
            "__templateName": "旧名称", "__templateHash": "old-hash"
        });
        let object = create_object(
            &vault,
            &account,
            &page.id,
            "用户填写的名称",
            input.clone(),
            Some(&template.id),
            Some("user-icon"),
        )
        .unwrap();

        for (key, value) in input.as_object().unwrap() {
            if !["__fields", "__templateName", "__templateHash"].contains(&key.as_str()) {
                assert_eq!(&object.properties[key], value, "用户字段 {key} 被覆盖");
            }
        }
        assert_eq!(
            object.properties["__fields"],
            serde_json::json!({
                "public": {"name": "姓名", "type": "text", "contractField": true},
                "internal": {"name": "类别", "type": "select", "options": ["A", "B"],
                             "deprecatedAt": "2026-09-20T00:00:00Z", "contractField": false},
                "sensitive": {"name": "电话", "type": "phone"},
                "critical": {"name": "凭证", "type": "text"},
                "unlabelled": {"name": "确认", "type": "boolean"},
                "group": {"name": "其他信息", "type": "dynamic_group",
                          "allowedTypes": ["text", "phone"], "maxItems": 2}
            })
        );
        assert_eq!(
            object.property_labels,
            Some(serde_json::json!({
                "public": "public", "internal": "internal", "sensitive": "sensitive",
                "critical": "critical", "group": "public"
            }))
        );
        let fingerprint = template_fingerprint(&template);
        assert_eq!(object.template_hash.as_deref(), Some(fingerprint.as_str()));
        assert_eq!(object.properties["__templateHash"], fingerprint);
        assert_eq!(object.properties["__templateName"], template.name);
        assert_eq!(object.contract_type_id, template.contract_type_id);
        assert_eq!(object.template_id.as_deref(), Some(template.id.as_str()));
        assert_eq!(object.template_type.as_deref(), Some("user"));
        assert!(object.id.starts_with("obj_"));
        assert_eq!(object.type_id, template.id);
        assert_eq!(object.section_type, "identity");
        assert_eq!(object.parent_id.as_deref(), Some(page.id.as_str()));
        assert_eq!(object.name, "用户填写的名称");
        assert_eq!(object.icon_name, "user-icon");
        assert_eq!(object.sensitivity_level, "internal");
        assert_eq!(
            vault.load_object(&page.id).unwrap().unwrap().children_ids,
            vec![object.id.clone()]
        );
        let saved = vault.load_object(&object.id).unwrap().unwrap();
        assert_eq!(saved.properties, object.properties);
        assert_eq!(saved.property_labels, object.property_labels);
        assert_eq!(saved.template_hash, object.template_hash);
        assert_eq!(saved.contract_type_id, object.contract_type_id);
    }

    #[test]
    fn rf009_template_copy_survives_deletion_and_is_in_initial_snapshot() {
        let (_dir, vault, account) = rf009_setup();
        let page = create_page(&vault, &account, "旅行").unwrap();
        let template = rf009_template(&account);
        vault.save_user_template(&template).unwrap();
        let object = create_object(
            &vault,
            &account,
            &page.id,
            "凭证",
            serde_json::json!({"critical": "保留值"}),
            Some(&template.id),
            None,
        )
        .unwrap();
        vault.delete_user_template(&template.id).unwrap();
        assert!(vault.load_user_template(&template.id).unwrap().is_none());

        let saved = vault.load_object(&object.id).unwrap().unwrap();
        assert_eq!(saved.properties, object.properties);
        assert_eq!(saved.property_labels, object.property_labels);
        assert_eq!(saved.contract_type_id, object.contract_type_id);
        assert_eq!(saved.template_hash, object.template_hash);
        let snapshots = vault.list_snapshots(&object.id).unwrap();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0]["diffSummary"], "diff_created");
        let bytes = vault
            .get_snapshot(snapshots[0]["id"].as_str().unwrap())
            .unwrap()
            .unwrap();
        let snapshot: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(snapshot["name"], "凭证");
        assert_eq!(snapshot["properties"], object.properties);
        assert_eq!(snapshot["propertyLabels"], object.property_labels.unwrap());
    }

    #[test]
    fn rf009_absent_or_missing_template_preserves_existing_properties() {
        let (_dir, vault, account) = rf009_setup();
        let page = create_page(&vault, &account, "笔记").unwrap();
        // 无/缺失模板沿用旧行为，不借本项收紧历史自定义 __fields 的校验。
        let input = serde_json::json!({
            "group": "legacy value",
            "__fields": {"group": {"name": "旧组", "type": "dynamic_group", "maxItems": 0}},
            "__templateName": "旧名称", "__templateHash": "旧指纹",
            "false": false, "zero": 0, "empty": "", "null": null
        });
        for template_id in [None, Some("missing-template")] {
            let object = create_object(
                &vault,
                &account,
                &page.id,
                " ",
                input.clone(),
                template_id,
                None,
            )
            .unwrap();
            assert_eq!(object.properties, input);
            assert!(object.property_labels.is_none());
            assert!(object.contract_type_id.is_none());
            assert!(object.template_hash.is_none());
            assert_eq!(object.name, "未命名对象");
            assert_eq!(object.icon_name, "document");
            assert_eq!(object.type_id, template_id.unwrap_or("note"));
            assert_eq!(object.template_id.as_deref(), template_id);
            assert_eq!(object.template_type.as_deref(), template_id.map(|_| "user"));
        }
    }

    #[test]
    fn rf009_empty_and_unlabelled_templates_preserve_optional_semantics() {
        let (_dir, vault, account) = rf009_setup();
        let page = create_page(&vault, &account, "笔记").unwrap();
        let mut template = rf009_template(&account);
        template.properties.retain(|prop| prop.id == "unlabelled");
        template.contract_type_id = None;
        for empty in [false, true] {
            if empty {
                template.properties.clear();
            }
            vault.save_user_template(&template).unwrap();
            let input = serde_json::json!({
                "unlabelled": false,
                "__fields": {"local": {"name": "本地字段", "type": "text"}}
            });
            let object = create_object(
                &vault,
                &account,
                &page.id,
                "笔记",
                input.clone(),
                Some(&template.id),
                None,
            )
            .unwrap();
            assert!(object.property_labels.is_none());
            assert!(object.contract_type_id.is_none());
            assert_eq!(object.properties["unlabelled"], false);
            assert_eq!(object.properties["__templateName"], template.name);
            assert_eq!(object.template_hash, Some(template_fingerprint(&template)));
            assert_eq!(
                object.properties["__templateHash"],
                serde_json::json!(object.template_hash)
            );
            if empty {
                assert_eq!(object.properties["__fields"], input["__fields"]);
            } else {
                assert_eq!(
                    object.properties["__fields"],
                    serde_json::json!({"unlabelled": {"name": "确认", "type": "boolean"}})
                );
            }
        }
    }

    #[test]
    fn rf009_invalid_dynamic_group_is_rejected_without_writes() {
        let (_dir, vault, account) = rf009_setup();
        let page = create_page(&vault, &account, "旅行").unwrap();
        let template = rf009_template(&account);
        vault.save_user_template(&template).unwrap();
        let audit_count = vault.list_audit_log(100).unwrap().len();
        let page_snapshot_count = vault.list_snapshots(&page.id).unwrap().len();
        let child =
            serde_json::json!({"id": "child", "name": "子字段", "type": "text", "value": ""});
        for group in [
            serde_json::Value::Null,
            serde_json::json!("not-an-array"),
            serde_json::json!([{"id": "wrong", "name": "非法类型", "type": "number", "value": 1}]),
            serde_json::json!([{"id": "missing", "name": "缺少值", "type": "text"}]),
            serde_json::json!([child.clone(), child.clone(), child]),
        ] {
            let error = create_object(
                &vault,
                &account,
                &page.id,
                "不得保存",
                serde_json::json!({
                    "group": group,
                    "__fields": {"group": {"name": "伪造定义", "type": "text"}}
                }),
                Some(&template.id),
                None,
            )
            .unwrap_err();
            assert!(error.contains("group"), "{error}");
            assert_eq!(
                vault
                    .list_object_metadata(&account, None, None, false, false)
                    .unwrap()
                    .len(),
                1
            );
            let parent = vault.load_object(&page.id).unwrap().unwrap();
            assert!(parent.children_ids.is_empty());
            assert_eq!(parent.version, page.version);
            assert_eq!(parent.updated_at, page.updated_at);
            assert_eq!(
                vault.list_snapshots(&page.id).unwrap().len(),
                page_snapshot_count
            );
            assert_eq!(vault.list_audit_log(100).unwrap().len(), audit_count);
        }
    }

    #[test]
    fn rf009_template_read_error_is_not_treated_as_missing_template() {
        let (_dir, vault, account) = rf009_setup();
        let page = create_page(&vault, &account, "旅行").unwrap();
        let template = rf009_template(&account);
        vault.save_user_template(&template).unwrap();
        let audit_count = vault.list_audit_log(100).unwrap().len();

        // 仅临时测试库切换内存密钥：模板读取真实解密失败，但写入 API 仍可执行。
        // 若读取错误被吞掉，旧实现会使用错误密钥创建缺少模板安全语义的对象。
        vault.set_data_key(solosoul_vault::DataEncryptionKey::new([0x73; 32]));
        let expected_error = vault.load_user_template(&template.id).unwrap_err();
        let result = create_object(
            &vault,
            &account,
            &page.id,
            "不得保存",
            serde_json::json!({}),
            Some(&template.id),
            None,
        );
        vault.set_data_key(solosoul_vault::DataEncryptionKey::new([0x42; 32]));

        assert_eq!(result.unwrap_err(), expected_error);
        assert_eq!(
            vault
                .list_object_metadata(&account, None, None, false, false)
                .unwrap()
                .len(),
            1
        );
        let parent = vault.load_object(&page.id).unwrap().unwrap();
        assert!(parent.children_ids.is_empty());
        assert_eq!(parent.version, page.version);
        assert_eq!(parent.updated_at, page.updated_at);
        assert_eq!(vault.list_snapshots(&page.id).unwrap().len(), 1);
        assert_eq!(vault.list_audit_log(100).unwrap().len(), audit_count);
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

        let meta =
            add_attachments(&vault, &account_id, &obj.id, &file_path, dir.path(), None).unwrap();
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

        let trash_items = vault.list_trash_items(None, None).unwrap();
        let obj_trash = trash_items
            .iter()
            .find(|t| t.item_type == "object")
            .unwrap()
            .clone();
        let page_trash = trash_items
            .iter()
            .find(|t| t.item_type == "page" && t.original_id == page.id)
            .unwrap()
            .clone();

        let result = restore_from_trash(&vault, &obj_trash.id).unwrap();

        assert_eq!(result.restored_id, obj.id);
        assert_eq!(result.cascaded_page_name.as_deref(), Some("CustomPage"));
        assert!(result.rebuilt_page_name.is_none());
        assert_eq!(result.cascaded_count, 0);
        assert!(result.consumed_trash_ids.contains(&obj_trash.id));
        assert!(result.consumed_trash_ids.contains(&page_trash.id));

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
        assert!(result.consumed_trash_ids.contains(&trash.id));

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

        let trash_items = vault.list_trash_items(None, None).unwrap();
        let page_trash = trash_items
            .iter()
            .find(|t| t.item_type == "page")
            .unwrap()
            .clone();
        let obj_trash = trash_items
            .iter()
            .find(|t| t.item_type == "object")
            .unwrap()
            .clone();

        let result = restore_from_trash(&vault, &page_trash.id).unwrap();

        assert_eq!(result.restored_id, page.id);
        assert_eq!(result.cascaded_count, 1);
        assert!(result.consumed_trash_ids.contains(&page_trash.id));
        assert!(result.consumed_trash_ids.contains(&obj_trash.id));

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

    #[test]
    fn test_restored_suffix_localization() {
        assert_eq!(restored_suffix("zh-CN"), "（已恢复）");
        assert_eq!(restored_suffix("en-US"), " (restored)");
        assert_eq!(restored_suffix("ja-JP"), " (restored)");
        assert_eq!(restored_suffix(""), " (restored)");
    }
}
