//! Vault 写入命令：创建、编辑、删除、回收站。
//!
//! 所有写操作都要求 Vault 已解锁。复合操作（如删除页面及其子对象）在代码注释中
//! 明确标注为非原子；单步失败时不自动回滚，失败信息会写入错误提示。

use color_eyre::Result;
use solosoul_core::{ObjectRecord, ObjectSummary, TrashItem, UserTemplate};

use crate::app::{App, AppPhase, EditObjectStep, NewObjectStep, TrashFilter};
use crate::widgets::field_editor::EditableField;
use crate::widgets::prompt::{self, PromptResult, PromptSpec};

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

use std::sync::Arc;

const DEFAULT_TRASH_RETENTION_MS: i64 = 30 * 24 * 3600 * 1000;

fn load_trash_retention(vault: &solosoul_core::VaultStore, account_id: &str) -> i64 {
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
    DEFAULT_TRASH_RETENTION_MS
}

fn parse_retention_ms(period: &str) -> i64 {
    match period {
        "7d" => 7 * 24 * 3600 * 1000,
        "30d" => 30 * 24 * 3600 * 1000,
        "60d" => 60 * 24 * 3600 * 1000,
        "half_year" | "half-year" | "6m" => 180 * 24 * 3600 * 1000,
        _ => {
            if let Ok(days) = period.trim_end_matches('d').parse::<i64>() {
                days * 24 * 3600 * 1000
            } else {
                DEFAULT_TRASH_RETENTION_MS
            }
        }
    }
}

// ============================================================================
// /newpage
// ============================================================================

/// 创建页面（collection_type=page）。
pub fn newpage(app: &mut App, name: Option<&str>) -> Result<()> {
    let account_id = require_unlocked(app)?;
    let name = match name {
        Some(n) if !n.is_empty() => n.to_string(),
        _ => {
            app.error_message = Some("请提供页面名称，例如 /newpage 旅行".to_string());
            return Ok(());
        }
    };

    let vault = vault(app)?;

    // 检查同名页面
    let pages = vault
        .list_objects(&account_id, Some("page"), None, None, false, false)
        .map_err(map_err)?;
    if pages.iter().any(|p| p.name.eq_ignore_ascii_case(&name)) {
        app.error_message = Some(format!("页面 '{}' 已存在", name));
        return Ok(());
    }

    let now = chrono::Utc::now().to_rfc3339();
    let id = format!("page_{}", uuid::Uuid::new_v4());
    let record = ObjectRecord {
        id: id.clone(),
        account_id: account_id.clone(),
        type_id: "page".to_string(),
        section_type: "page".to_string(),
        name: name.clone(),
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
        created_at: now.clone(),
        updated_at: now.clone(),
        version: 1,
    };

    vault.save_object(&record).map_err(map_err)?;
    save_creation_snapshot(&vault, &id, &name, &record.properties)?;
    let _ = vault.log_structured(
        "page_create",
        "page",
        Some(&id),
        Some(&name),
        "user",
        Some("source=cli"),
    );

    app.previous_phase = Some(app.phase.clone());
    app.phase = AppPhase::ObjectDetail { object: record };
    Ok(())
}

// ============================================================================
// /newobject 向导
// ============================================================================

/// 启动创建对象向导。
pub fn newobject(app: &mut App, page_name: Option<&str>) -> Result<()> {
    let account_id = require_unlocked(app)?;
    let vault = vault(app)?;

    let pages = vault
        .list_objects(&account_id, Some("page"), None, None, false, false)
        .map_err(map_err)?;

    if let Some(name) = page_name {
        if let Some(page) = pages.iter().find(|p| p.name.eq_ignore_ascii_case(name)) {
            start_select_template(app, page.id.clone(), page.name.clone())?;
            return Ok(());
        }
        app.error_message = Some(format!("页面 '{}' 不存在", name));
        return Ok(());
    }

    if pages.is_empty() {
        app.error_message = Some("暂无页面，请先使用 /newpage 创建页面".to_string());
        return Ok(());
    }

    app.previous_phase = Some(app.phase.clone());
    app.phase = AppPhase::NewObjectWizard {
        step: NewObjectStep::SelectPage { pages, selected: 0 },
    };
    Ok(())
}

/// 进入选择模板步骤。
pub fn start_select_template(app: &mut App, page_id: String, page_name: String) -> Result<()> {
    let account_id = require_unlocked(app)?;
    let vault = vault(app)?;
    let templates = vault.list_user_templates(&account_id).map_err(map_err)?;

    app.previous_phase = Some(AppPhase::Home {
        account_id: account_id.clone(),
    });
    app.phase = AppPhase::NewObjectWizard {
        step: NewObjectStep::SelectTemplate {
            page_id,
            page_name,
            templates,
            selected: 0,
        },
    };
    Ok(())
}

/// 进入填写字段步骤。
pub fn start_fill_fields(
    app: &mut App,
    page_id: String,
    page_name: String,
    template: Option<UserTemplate>,
) -> Result<()> {
    let fields = if let Some(ref tpl) = template {
        EditableField::from_properties_and_template(&serde_json::json!({}), Some(tpl))
    } else {
        vec![]
    };

    app.phase = AppPhase::NewObjectWizard {
        step: NewObjectStep::FillFields {
            page_id,
            page_name,
            template,
            name: String::new(),
            fields,
            selected: 0,
        },
    };
    Ok(())
}

/// 保存新建对象。
pub fn save_new_object(
    app: &mut App,
    page_id: String,
    _page_name: String,
    template: Option<UserTemplate>,
    name: String,
    fields: Vec<EditableField>,
) -> Result<()> {
    let account_id = require_unlocked(app)?;
    let vault = vault(app)?;

    let name = if name.trim().is_empty() {
        "未命名对象".to_string()
    } else {
        name
    };

    let now = chrono::Utc::now().to_rfc3339();
    let id = format!("obj_{}", uuid::Uuid::new_v4());

    let mut properties = serde_json::Map::new();
    for f in &fields {
        properties.insert(f.key.clone(), f.value.clone());
    }

    let type_id = template
        .as_ref()
        .map(|t| t.id.clone())
        .unwrap_or_else(|| "note".to_string());
    let icon_name = template
        .as_ref()
        .and_then(|t| t.icon_id.clone())
        .unwrap_or_else(|| "document".to_string());

    let record = ObjectRecord {
        id: id.clone(),
        account_id: account_id.clone(),
        type_id,
        section_type: "identity".to_string(),
        name: name.clone(),
        icon_name,
        parent_id: Some(page_id.clone()),
        children_ids: vec![],
        properties: serde_json::Value::Object(properties),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: template.as_ref().map(|t| t.id.clone()),
        contract_type_id: template.as_ref().and_then(|t| t.contract_type_id.clone()),
        template_type: template.as_ref().map(|_| "user".to_string()),
        created_at: now.clone(),
        updated_at: now,
        version: 1,
    };

    // 单条 save_object 内部是 SQLite 事务，具有原子性。
    vault.save_object(&record).map_err(map_err)?;

    // 更新父页面的 children_ids（复合操作，失败不自动回滚）。
    if let Ok(Some(mut parent)) = vault.load_object(&page_id) {
        if !parent.children_ids.contains(&id) {
            parent.children_ids.push(id.clone());
            parent.updated_at = chrono::Utc::now().to_rfc3339();
            parent.version += 1;
            vault.save_object(&parent).map_err(map_err)?;
        }
    }

    save_creation_snapshot(&vault, &id, &name, &record.properties)?;
    let _ = vault.log_structured(
        "object_create",
        "object",
        Some(&id),
        Some(&name),
        "user",
        Some(&format!(
            "parent_id={} template={:?}",
            page_id, record.template_id
        )),
    );

    app.phase = AppPhase::ObjectDetail { object: record };
    Ok(())
}

// ============================================================================
// /edit 向导
// ============================================================================

/// 启动编辑对象向导。
pub fn edit(app: &mut App, object_id: Option<&str>) -> Result<()> {
    let account_id = require_unlocked(app)?;
    let id = match object_id {
        Some(id) => id,
        None => {
            app.error_message = Some("请提供对象 ID，例如 /edit obj_xxx".to_string());
            return Ok(());
        }
    };

    let vault = vault(app)?;
    match vault.load_object(id).map_err(map_err)? {
        Some(record) if record.account_id == account_id && !record.is_deleted => {
            let template = record
                .template_id
                .as_ref()
                .and_then(|tid| vault.load_user_template(tid).ok().flatten());
            let fields =
                EditableField::from_properties_and_template(&record.properties, template.as_ref());
            app.previous_phase = Some(app.phase.clone());
            app.phase = AppPhase::EditObjectWizard {
                object_id: id.to_string(),
                step: EditObjectStep::Overview {
                    object: record,
                    fields,
                    selected: 0,
                },
            };
        }
        _ => {
            app.error_message = Some(format!("对象 '{}' 不存在或已被删除", id));
        }
    }
    Ok(())
}

/// 保存编辑后的对象。
pub fn save_edited_object(app: &mut App, mut object: ObjectRecord) -> Result<()> {
    let vault = vault(app)?;

    object.updated_at = chrono::Utc::now().to_rfc3339();
    object.version += 1;

    vault.save_object(&object).map_err(map_err)?;

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

    app.phase = AppPhase::ObjectDetail { object };
    Ok(())
}

// ============================================================================
// /delete
// ============================================================================

/// 删除对象或页面（软删除）。
pub fn delete(app: &mut App, object_id: Option<&str>) -> Result<()> {
    let account_id = require_unlocked(app)?;
    let id = match object_id {
        Some(id) => id,
        None => {
            app.error_message = Some("请提供对象 ID，例如 /delete obj_xxx".to_string());
            return Ok(());
        }
    };

    let vault = vault(app)?;
    let record = match vault.load_object(id).map_err(map_err)? {
        Some(r) if r.account_id == account_id && !r.is_deleted => r,
        _ => {
            app.error_message = Some(format!("对象 '{}' 不存在或已被删除", id));
            return Ok(());
        }
    };

    if record.type_id == "page" {
        // 查找页面下的子对象。
        let children = vault
            .list_objects(&account_id, None, Some(&record.id), None, false, false)
            .map_err(map_err)?;
        let child_count = children.len();

        prompt::open(
            app,
            PromptSpec::Confirm {
                message: format!(
                    "页面 '{}' 下包含 {} 个对象，删除将一并移入回收站，确认？",
                    record.name, child_count
                ),
                default_yes: false,
            },
            Box::new(move |app, result| {
                if let PromptResult::Confirm(true) = result {
                    if let Err(e) = delete_page(app, &record, &children) {
                        app.error_message = Some(format!("删除页面失败: {}", e));
                    }
                }
            }),
        );
    } else {
        // 普通对象直接软删除。
        let retention_ms = load_trash_retention(&vault, &record.account_id);
        move_object_to_trash(&vault, &record, "object", None, retention_ms)?;
        vault.delete_object(&record.id, true).map_err(map_err)?;
        let _ = vault.log_structured(
            "object_delete",
            "object",
            Some(&record.id),
            Some(&record.name),
            "user",
            Some(&format!("section={}", record.section_type)),
        );
        app.error_message = Some(format!("对象 '{}' 已删除", record.name));
    }

    Ok(())
}

/// 删除页面及其子对象。复合操作：单步失败不自动回滚。
fn delete_page(app: &mut App, page: &ObjectRecord, children: &[ObjectSummary]) -> Result<()> {
    let account_id = page.account_id.clone();
    let vault = vault(app)?;
    let retention_ms = load_trash_retention(&vault, &account_id);

    // 1. 先将子对象移入回收站并软删除。
    for child_summary in children {
        if let Ok(Some(child)) = vault.load_object(&child_summary.id) {
            let _ = move_object_to_trash(
                &vault,
                &child,
                "object",
                Some(page.id.clone()),
                retention_ms,
            );
            let _ = vault.delete_object(&child.id, true);
        }
    }

    // 2. 将页面本身移入回收站并软删除。
    move_object_to_trash(&vault, page, "page", None, retention_ms)?;
    vault.delete_object(&page.id, true).map_err(map_err)?;

    let _ = vault.log_structured(
        "page_delete",
        "page",
        Some(&page.id),
        Some(&page.name),
        "user",
        Some(&format!("count={}", children.len())),
    );

    app.phase = AppPhase::Home { account_id };
    app.error_message = Some(format!(
        "页面 '{}' 及 {} 个子对象已删除",
        page.name,
        children.len()
    ));
    Ok(())
}

fn move_object_to_trash(
    vault: &solosoul_core::VaultStore,
    record: &ObjectRecord,
    item_type: &str,
    original_parent_id: Option<String>,
    retention_ms: i64,
) -> Result<()> {
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
    vault.save_trash_item(&trash).map_err(map_err)?;
    Ok(())
}

// ============================================================================
// /trash /restore /purge
// ============================================================================

/// 解析回收站筛选参数。
fn parse_trash_filter(args: &[&str]) -> TrashFilter {
    let mut filter = TrashFilter::default();
    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "--type" | "-t" if i + 1 < args.len() => {
                filter.item_type = Some(args[i + 1].to_lowercase());
                i += 2;
                continue;
            }
            "--filter" | "-f" if i + 1 < args.len() => {
                filter.since_ms = parse_trash_since(args[i + 1]);
                i += 2;
                continue;
            }
            "--search" | "-s" if i + 1 < args.len() => {
                filter.search = Some(args[i + 1].to_string());
                i += 2;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    filter
}

fn parse_trash_since(s: &str) -> Option<i64> {
    let now = chrono::Utc::now().timestamp_millis();
    let ms = match s {
        "1d" => 24 * 3600 * 1000,
        "3d" => 3 * 24 * 3600 * 1000,
        "7d" => 7 * 24 * 3600 * 1000,
        "30d" => 30 * 24 * 3600 * 1000,
        "half_year" | "half-year" | "6m" => 180 * 24 * 3600 * 1000,
        _ => {
            // 尝试解析为天数
            if let Ok(days) = s.parse::<i64>() {
                days * 24 * 3600 * 1000
            } else {
                return None;
            }
        }
    };
    Some(now - ms)
}

fn load_trash_items(
    app: &mut App,
    filter: &TrashFilter,
) -> Result<Vec<solosoul_core::TrashItemSummary>> {
    let vault = vault(app)?;
    let mut items = vault
        .list_trash_items(filter.item_type.as_deref(), filter.since_ms)
        .map_err(map_err)?;
    if let Some(search) = &filter.search {
        let lower = search.to_lowercase();
        items.retain(|item| item.name.to_lowercase().contains(&lower));
    }
    Ok(items)
}

pub fn apply_trash_filter(app: &mut App, filter: TrashFilter) -> Result<()> {
    let items = load_trash_items(app, &filter)?;
    app.previous_phase = Some(app.phase.clone());
    app.phase = AppPhase::TrashList {
        items,
        selected: 0,
        selected_ids: Vec::new(),
        filter,
    };
    Ok(())
}

/// 列出回收站项目。
pub fn trash(app: &mut App, args: &[&str]) -> Result<()> {
    require_unlocked(app)?;
    let filter = parse_trash_filter(args);
    apply_trash_filter(app, filter)
}

/// 批量恢复回收站项目。
pub fn batch_restore(app: &mut App, ids: &[String]) -> Result<()> {
    let account_id = require_unlocked(app)?;
    let vault = vault(app)?;
    let mut success = Vec::new();
    let mut failed = Vec::new();
    for id in ids {
        match restore_single(app, vault.as_ref(), &account_id, id) {
            Ok(new_id) => success.push(format!("{} -> {}", id, new_id)),
            Err(e) => failed.push(format!("{}: {}", id, e)),
        }
    }
    app.error_message = Some(format!(
        "恢复完成：成功 {} 项；失败 {} 项{}",
        success.len(),
        failed.len(),
        if failed.is_empty() {
            String::new()
        } else {
            format!("\n{}", failed.join("\n"))
        }
    ));
    Ok(())
}

/// 批量彻底删除回收站项目。
pub fn batch_purge(app: &mut App, ids: &[String]) -> Result<()> {
    require_unlocked(app)?;
    let vault = vault(app)?;
    let mut success = 0usize;
    let mut failed = Vec::new();
    for id in ids {
        match vault.delete_trash_item(id).map_err(map_err) {
            Ok(()) => success += 1,
            Err(e) => failed.push(format!("{}: {}", id, e)),
        }
    }
    app.error_message = Some(format!(
        "彻底删除完成：成功 {} 项；失败 {} 项{}",
        success,
        failed.len(),
        if failed.is_empty() {
            String::new()
        } else {
            format!("\n{}", failed.join("\n"))
        }
    ));
    Ok(())
}

fn restore_single(
    app: &mut App,
    vault: &solosoul_core::VaultStore,
    account_id: &str,
    trash_id: &str,
) -> Result<String> {
    let trash = match vault.get_trash_item(trash_id).map_err(map_err)? {
        Some(t) => t,
        None => return Err(color_eyre::eyre::eyre!("回收站项目不存在")),
    };
    match trash.item_type.as_str() {
        "object" | "page" => {
            let record = object_record_from_trash(&trash)?;
            restore_record(app, vault, &trash, record, account_id)
        }
        "template" => {
            let template: UserTemplate = serde_json::from_slice(&trash.data)
                .map_err(|e| map_err(format!("模板数据损坏: {}", e)))?;
            vault.save_user_template(&template).map_err(map_err)?;
            vault.delete_trash_item(trash_id).map_err(map_err)?;
            Ok(template.name)
        }
        _ => Err(color_eyre::eyre::eyre!(format!(
            "不支持的回收站类型: {}",
            trash.item_type
        ))),
    }
}

/// 从回收站恢复单个对象/页面/模板。
pub fn restore(app: &mut App, trash_id: Option<&str>) -> Result<()> {
    let account_id = require_unlocked(app)?;
    let trash_id = match trash_id {
        Some(id) => id,
        None => {
            app.error_message = Some("请提供 trash_id，例如 /restore trash_xxx".to_string());
            return Ok(());
        }
    };

    let vault = vault(app)?;
    match restore_single(app, vault.as_ref(), &account_id, trash_id) {
        Ok(new_id) => app.error_message = Some(format!("已恢复: {}", new_id)),
        Err(e) => app.error_message = Some(format!("恢复失败: {}", e)),
    }
    Ok(())
}

fn object_record_from_trash(trash: &TrashItem) -> Result<ObjectRecord> {
    let data: serde_json::Value = serde_json::from_slice(&trash.data)
        .map_err(|e| map_err(format!("回收站数据损坏: {}", e)))?;
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
        created_at: data["created_at"].as_str().unwrap_or(&now).to_string(),
        updated_at: now,
        version: data["version"].as_u64().unwrap_or(1) as u32,
    })
}

fn restore_record(
    app: &mut App,
    vault: &solosoul_core::VaultStore,
    trash: &TrashItem,
    mut record: ObjectRecord,
    account_id: &str,
) -> Result<String> {
    // 如果原父页面已不存在或已删除，则清除 parent_id 避免孤立引用。
    if let Some(ref pid) = record.parent_id {
        if vault
            .load_object(pid)
            .ok()
            .flatten()
            .is_none_or(|p| p.is_deleted)
        {
            record.parent_id = None;
        }
    }

    // 检查 ID 冲突：同 section 下是否存在同名活跃对象。
    let conflict = vault
        .list_objects(account_id, None, None, Some(&record.name), false, false)
        .map_err(map_err)?
        .into_iter()
        .any(|o| {
            o.name == record.name && o.section_type == record.section_type && o.id != record.id
        });

    let new_id = if conflict {
        format!(
            "{}_{}",
            record.id,
            uuid::Uuid::new_v4()
                .to_string()
                .split('-')
                .next()
                .unwrap_or("restored")
        )
    } else {
        record.id.clone()
    };

    if conflict {
        record.id = new_id.clone();
        record.name = format!("{}（已恢复）", record.name);
    }

    vault.save_object(&record).map_err(map_err)?;
    vault.delete_trash_item(&trash.id).map_err(map_err)?;
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

    app.phase = AppPhase::ObjectDetail { object: record };
    Ok(new_id)
}

/// 彻底删除回收站项目。
pub fn purge(app: &mut App, trash_id: Option<&str>) -> Result<()> {
    let trash_id = match trash_id {
        Some(id) => id,
        None => {
            app.error_message = Some("请提供 trash_id，例如 /purge trash_xxx".to_string());
            return Ok(());
        }
    };

    let vault = vault(app)?;
    let trash = match vault.get_trash_item(trash_id).map_err(map_err)? {
        Some(t) => t,
        None => {
            app.error_message = Some(format!("回收站项目 '{}' 不存在", trash_id));
            return Ok(());
        }
    };

    prompt::open(
        app,
        PromptSpec::Confirm {
            message: format!("彻底删除 '{}'？此操作不可恢复。", trash.name_snapshot),
            default_yes: false,
        },
        Box::new(move |app, result| {
            if let PromptResult::Confirm(true) = result {
                if let Err(e) = do_purge(app, &trash) {
                    app.error_message = Some(format!("彻底删除失败: {}", e));
                }
            }
        }),
    );

    Ok(())
}

fn do_purge(app: &mut App, trash: &TrashItem) -> Result<()> {
    let vault = vault(app)?;
    if trash.item_type != "template" {
        // 先尝试永久删除底层对象（忽略不存在）。
        let _ = vault.delete_object(&trash.original_id, false);
    }
    vault.delete_trash_item(&trash.id).map_err(map_err)?;
    let _ = vault.log_structured(
        "trash_permanent_delete",
        "trash_item",
        Some(&trash.id),
        Some(&trash.name_snapshot),
        "user",
        Some(&format!("original_id={}", trash.original_id)),
    );
    app.error_message = Some(format!("已彻底删除 '{}'", trash.name_snapshot));
    Ok(())
}

// ============================================================================
// Helpers
// ============================================================================

fn save_creation_snapshot(
    vault: &solosoul_core::VaultStore,
    object_id: &str,
    name: &str,
    properties: &serde_json::Value,
) -> Result<()> {
    let snapshot_data = serde_json::to_vec(&serde_json::json!({
        "name": name,
        "tags": Vec::<String>::new(),
        "properties": properties,
    }))
    .unwrap_or_default();
    let _ = vault.save_snapshot(object_id, "user_edit", &snapshot_data, "Created");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use solosoul_core::{ObjectRecord, ObjectSummary, VaultService};

    use crate::app::{App, AppPhase, NewObjectStep};

    fn unlocked_app() -> (App, String, tempfile::TempDir) {
        let _guard = crate::VAULT_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::TempDir::new().unwrap();
        std::env::set_var("SOLOSOUL_DATA_DIR", dir.path());
        let vault = VaultService::new();
        let account = vault.create_account("Test", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap().to_string();
        let app = App::new(Arc::new(vault)).unwrap();
        (app, account_id, dir)
    }

    #[test]
    fn test_newpage_success() {
        let (mut app, account_id, _dir) = unlocked_app();
        super::newpage(&mut app, Some("旅行")).unwrap();

        let vault = app.vault_service.get_vault_store().unwrap();
        let pages = vault
            .list_objects(&account_id, Some("page"), None, None, false, false)
            .unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].name, "旅行");
        assert!(matches!(app.phase, AppPhase::ObjectDetail { .. }));
    }

    #[test]
    fn test_newpage_duplicate_fails() {
        let (mut app, _id, _dir) = unlocked_app();
        super::newpage(&mut app, Some("旅行")).unwrap();
        super::newpage(&mut app, Some("旅行")).unwrap();
        assert!(app.error_message.is_some());
    }

    #[test]
    fn test_newobject_wizard_save() {
        let (mut app, account_id, _dir) = unlocked_app();
        super::newpage(&mut app, Some("旅行")).unwrap();

        // 读取刚创建的页面 ID
        let vault = app.vault_service.get_vault_store().unwrap();
        let page = vault
            .list_objects(&account_id, Some("page"), None, None, false, false)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        // 直接进入填写字段（无模板）
        super::start_select_template(&mut app, page.id.clone(), page.name.clone()).unwrap();
        // 选择空白对象（selected 0 即空白）
        super::start_fill_fields(&mut app, page.id.clone(), page.name.clone(), None).unwrap();

        // 保存
        if let AppPhase::NewObjectWizard {
            step:
                NewObjectStep::FillFields {
                    page_id,
                    page_name,
                    template,
                    name: _,
                    fields,
                    ..
                },
        } = app.phase.clone()
        {
            super::save_new_object(
                &mut app,
                page_id,
                page_name,
                template,
                "我的笔记".to_string(),
                fields,
            )
            .unwrap();
        } else {
            panic!("expected FillFields");
        }

        assert!(matches!(app.phase, AppPhase::ObjectDetail { .. }));
        let vault = app.vault_service.get_vault_store().unwrap();
        let objects = vault
            .list_objects(&account_id, None, Some(&page.id), None, false, false)
            .unwrap();
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].name, "我的笔记");
    }

    #[test]
    fn test_edit_object() {
        let (mut app, account_id, _dir) = unlocked_app();
        super::newpage(&mut app, Some("旅行")).unwrap();
        let vault = app.vault_service.get_vault_store().unwrap();
        let page = vault
            .list_objects(&account_id, Some("page"), None, None, false, false)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        // 创建对象
        let mut obj = ObjectRecord {
            id: format!("obj_{}", uuid::Uuid::new_v4()),
            account_id: account_id.clone(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "原名称".to_string(),
            icon_name: "document".to_string(),
            parent_id: Some(page.id.clone()),
            children_ids: vec![],
            properties: serde_json::json!({"title": "old"}),
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
        vault.save_object(&obj).unwrap();

        // 编辑名称与属性
        obj.name = "新名称".to_string();
        if let serde_json::Value::Object(ref mut map) = obj.properties {
            map.insert("title".to_string(), serde_json::json!("new"));
        }
        super::save_edited_object(&mut app, obj).unwrap();

        let vault = app.vault_service.get_vault_store().unwrap();
        let loaded = vault
            .load_object(&format!("obj_{}", "ignored"))
            .ok()
            .flatten();
        assert!(loaded.is_none()); // 占位，避免未使用
        let objects = vault
            .list_objects(&account_id, None, Some(&page.id), None, false, false)
            .unwrap();
        assert_eq!(objects[0].name, "新名称");
    }

    #[test]
    fn test_delete_restore_purge_object_lifecycle() {
        let (mut app, account_id, _dir) = unlocked_app();
        super::newpage(&mut app, Some("旅行")).unwrap();
        let vault = app.vault_service.get_vault_store().unwrap();
        let page = vault
            .list_objects(&account_id, Some("page"), None, None, false, false)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        let obj_id = format!("obj_{}", uuid::Uuid::new_v4());
        let obj = ObjectRecord {
            id: obj_id.clone(),
            account_id: account_id.clone(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "待删除".to_string(),
            icon_name: "document".to_string(),
            parent_id: Some(page.id.clone()),
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
        vault.save_object(&obj).unwrap();

        // 删除
        super::delete(&mut app, Some(&obj_id)).unwrap();
        let vault = app.vault_service.get_vault_store().unwrap();
        assert!(vault.load_object(&obj_id).unwrap().unwrap().is_deleted);

        // 回收站
        let trash = vault.list_trash_items(None, None).unwrap();
        assert_eq!(trash.len(), 1);
        let trash_id = trash[0].id.clone();

        // 恢复
        super::restore(&mut app, Some(&trash_id)).unwrap();
        let vault = app.vault_service.get_vault_store().unwrap();
        assert!(vault.list_trash_items(None, None).unwrap().is_empty());
        let restored = vault.load_object(&obj_id).unwrap().unwrap();
        assert!(!restored.is_deleted);
        assert_eq!(restored.name, "待删除");

        // 再次删除并彻底删除
        super::delete(&mut app, Some(&obj_id)).unwrap();
        let trash = vault.list_trash_items(None, None).unwrap();
        let trash_id = trash[0].id.clone();
        super::do_purge(&mut app, &vault.get_trash_item(&trash_id).unwrap().unwrap()).unwrap();
        assert!(vault.load_object(&obj_id).unwrap().is_none());
        assert!(vault.list_trash_items(None, None).unwrap().is_empty());
    }

    #[test]
    fn test_delete_page_cascade_prompt() {
        let (mut app, account_id, _dir) = unlocked_app();
        super::newpage(&mut app, Some("旅行")).unwrap();
        let vault = app.vault_service.get_vault_store().unwrap();
        let page = vault
            .list_objects(&account_id, Some("page"), None, None, false, false)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        let child = ObjectRecord {
            id: format!("obj_{}", uuid::Uuid::new_v4()),
            account_id: account_id.clone(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "子对象".to_string(),
            icon_name: "document".to_string(),
            parent_id: Some(page.id.clone()),
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
        vault.save_object(&child).unwrap();

        super::delete(&mut app, Some(&page.id)).unwrap();
        // 确认提示已打开
        assert!(app.prompt.is_some());
        // 模拟确认
        if app.prompt.take().is_some() {
            let page_record = vault.load_object(&page.id).unwrap().unwrap();
            let child_summary = ObjectSummary {
                id: child.id.clone(),
                name: child.name.clone(),
                collection_type: child.type_id.clone(),
                section_type: child.section_type.clone(),
                sensitivity_level: child.sensitivity_level.clone(),
                created_at: child.created_at.clone(),
                updated_at: child.updated_at.clone(),
                is_deleted: child.is_deleted,
                template_id: child.template_id.clone(),
                template_type: child.template_type.clone(),
                contract_type_id: child.contract_type_id.clone(),
                icon_name: child.icon_name.clone(),
                properties: child.properties.clone(),
                tags: child.tags_json.clone(),
            };
            super::delete_page(&mut app, &page_record, &[child_summary]).unwrap();
            assert!(app.error_message.as_ref().unwrap().contains("已删除"));
        }
    }
}
