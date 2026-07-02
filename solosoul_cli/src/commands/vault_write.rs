//! Vault 写入命令（CLI 薄封装）。
//!
//! 实际创建/编辑/回收站业务逻辑已下沉到 `solosoul-core::objects`，
//! 本文件只负责 Wizard 状态管理、参数解析、确认弹窗和终端输出。

use color_eyre::Result;
use solosoul_core::objects;
use solosoul_core::ObjectRecord;

use crate::app::{App, AppPhase, EditObjectStep, NewObjectStep, TrashFilter};
use crate::commands::{require_unlocked, vault};
use crate::widgets::field_editor::EditableField;
use crate::widgets::prompt::{self, PromptResult, PromptSpec};


// ============================================================================
// /newpage
// ============================================================================

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
    match objects::create_page(&vault, &account_id, &name) {
        Ok(record) => {
            app.previous_phase = Some(app.phase.clone());
            app.phase = AppPhase::ObjectDetail { object: record };
        }
        Err(e) => {
            app.error_message = Some(e);
        }
    }
    Ok(())
}

// ============================================================================
// /newobject 向导
// ============================================================================

pub fn newobject(app: &mut App, page_name: Option<&str>) -> Result<()> {
    let account_id = require_unlocked(app)?;
    let vault = vault(app)?;

    let pages = vault
        .list_objects(&account_id, Some("page"), None, None, false, false)
        .map_err(|e| color_eyre::eyre::eyre!(e))?;

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

pub fn start_select_template(app: &mut App, page_id: String, page_name: String) -> Result<()> {
    let account_id = require_unlocked(app)?;
    let vault = vault(app)?;
    let templates = vault.list_user_templates(&account_id).map_err(|e| color_eyre::eyre::eyre!(e))?;

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

pub fn start_fill_fields(
    app: &mut App,
    page_id: String,
    page_name: String,
    template: Option<solosoul_core::UserTemplate>,
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

pub fn save_new_object(
    app: &mut App,
    page_id: String,
    _page_name: String,
    template: Option<solosoul_core::UserTemplate>,
    name: String,
    fields: Vec<EditableField>,
) -> Result<()> {
    let account_id = require_unlocked(app)?;
    let vault = vault(app)?;

    let mut properties = serde_json::Map::new();
    for f in &fields {
        properties.insert(f.key.clone(), f.value.clone());
    }

    let template_id = template.as_ref().map(|t| t.id.as_str());
    let icon_name = template
        .as_ref()
        .and_then(|t| t.icon_id.clone())
        .unwrap_or_else(|| "document".to_string());

    match objects::create_object(
        &vault,
        &account_id,
        &page_id,
        &name,
        serde_json::Value::Object(properties),
        template_id,
        Some(&icon_name),
    ) {
        Ok(record) => {
            app.phase = AppPhase::ObjectDetail { object: record };
        }
        Err(e) => {
            app.error_message = Some(e);
        }
    }
    Ok(())
}

// ============================================================================
// /edit 向导
// ============================================================================

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
    match vault.load_object(id).map_err(|e| color_eyre::eyre::eyre!(e))? {
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

pub fn save_edited_object(app: &mut App, mut object: ObjectRecord) -> Result<()> {
    let vault = vault(app)?;
    match objects::update_object(&vault, &mut object) {
        Ok(()) => {
            app.phase = AppPhase::ObjectDetail { object };
        }
        Err(e) => {
            app.error_message = Some(e);
        }
    }
    Ok(())
}

// ============================================================================
// /delete
// ============================================================================

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
    let record = match vault.load_object(id).map_err(|e| color_eyre::eyre::eyre!(e))? {
        Some(r) if r.account_id == account_id && !r.is_deleted => r,
        _ => {
            app.error_message = Some(format!("对象 '{}' 不存在或已被删除", id));
            return Ok(());
        }
    };

    if record.type_id == "page" {
        let children = vault
            .list_objects(&account_id, None, Some(&record.id), None, false, false)
            .map_err(|e| color_eyre::eyre::eyre!(e))?;
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
        let retention_ms = objects::load_trash_retention(&vault, &record.account_id);
        if let Err(e) = objects::move_to_trash(&vault, &record, "object", None, retention_ms) {
            app.error_message = Some(format!("删除失败: {}", e));
        } else {
            vault.log_structured(
                "object_delete",
                "object",
                Some(&record.id),
                Some(&record.name),
                "user",
                Some(&format!("section={}", record.section_type)),
            ).ok();
            app.error_message = Some(format!("对象 '{}' 已删除", record.name));
        }
    }
    Ok(())
}

fn delete_page(app: &mut App, page: &ObjectRecord, children: &[solosoul_core::ObjectSummary]) -> Result<()> {
    let account_id = page.account_id.clone();
    let vault = vault(app)?;
    let retention_ms = objects::load_trash_retention(&vault, &account_id);

    for child_summary in children {
        if let Ok(Some(child)) = vault.load_object(&child_summary.id) {
            let _ = objects::move_to_trash(&vault, &child, "object", Some(page.id.clone()), retention_ms);
        }
    }

    // 页面本身需要先构建 ObjectRecord 再移入回收站
    if let Ok(Some(page_record)) = vault.load_object(&page.id) {
        if let Err(e) = objects::move_to_trash(&vault, &page_record, "page", None, retention_ms) {
            app.error_message = Some(format!("删除页面失败: {}", e));
            return Ok(());
        }
    }

    vault.log_structured(
        "page_delete",
        "page",
        Some(&page.id),
        Some(&page.name),
        "user",
        Some(&format!("count={}", children.len())),
    ).ok();

    app.phase = AppPhase::Home { account_id };
    app.error_message = Some(format!(
        "页面 '{}' 及 {} 个子对象已删除",
        page.name,
        children.len()
    ));
    Ok(())
}

// ============================================================================
// /trash /restore /purge
// ============================================================================

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
            if let Ok(days) = s.parse::<i64>() {
                days * 24 * 3600 * 1000
            } else {
                return None;
            }
        }
    };
    Some(now - ms)
}

fn load_trash_items(app: &mut App, filter: &TrashFilter) -> Result<Vec<solosoul_core::TrashItemSummary>> {
    let vault = vault(app)?;
    let mut items = vault
        .list_trash_items(filter.item_type.as_deref(), filter.since_ms)
        .map_err(|e| color_eyre::eyre::eyre!(e))?;
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

pub fn trash(app: &mut App, args: &[&str]) -> Result<()> {
    require_unlocked(app)?;
    let filter = parse_trash_filter(args);
    apply_trash_filter(app, filter)
}

pub fn batch_restore(app: &mut App, ids: &[String]) -> Result<()> {
    let account_id = require_unlocked(app)?;
    let vault = vault(app)?;
    let mut success = Vec::new();
    let mut failed = Vec::new();
    for id in ids {
        match objects::restore_from_trash(&vault, &account_id, id) {
            Ok(result) => success.push(format!("{} -> {}", id, result.new_id)),
            Err(e) => failed.push(format!("{}: {}", id, e)),
        }
    }
    app.error_message = Some(format!(
        "恢复完成：成功 {} 项；失败 {} 项{}",
        success.len(),
        failed.len(),
        if failed.is_empty() { String::new() } else { format!("\n{}", failed.join("\n")) }
    ));
    Ok(())
}

pub fn batch_purge(app: &mut App, ids: &[String]) -> Result<()> {
    require_unlocked(app)?;
    let vault = vault(app)?;
    let mut success = 0usize;
    let mut failed = Vec::new();
    for id in ids {
        match objects::purge_trash(&vault, id) {
            Ok(_) => success += 1,
            Err(e) => failed.push(format!("{}: {}", id, e)),
        }
    }
    app.error_message = Some(format!(
        "彻底删除完成：成功 {} 项；失败 {} 项{}",
        success,
        failed.len(),
        if failed.is_empty() { String::new() } else { format!("\n{}", failed.join("\n")) }
    ));
    Ok(())
}

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
    match objects::restore_from_trash(&vault, &account_id, trash_id) {
        Ok(result) => app.error_message = Some(format!("已恢复: {}", result.new_id)),
        Err(e) => app.error_message = Some(format!("恢复失败: {}", e)),
    }
    Ok(())
}

pub fn purge(app: &mut App, trash_id: Option<&str>) -> Result<()> {
    let trash_id = match trash_id {
        Some(id) => id.to_string(),
        None => {
            app.error_message = Some("请提供 trash_id，例如 /purge trash_xxx".to_string());
            return Ok(());
        }
    };

    let vault = vault(app)?;
    let trash = match vault.get_trash_item(&trash_id).map_err(|e| color_eyre::eyre::eyre!(e))? {
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
                let vault = match crate::commands::vault(app) {
                    Ok(v) => v,
                    Err(_) => return,
                };
                match objects::purge_trash(&vault, &trash_id) {
                    Ok(name) => app.error_message = Some(format!("已彻底删除 '{}'", name)),
                    Err(e) => app.error_message = Some(format!("彻底删除失败: {}", e)),
                }
            }
        }),
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use solosoul_core::{objects, VaultService};
    use std::sync::Arc;

    use crate::app::{App, AppPhase};

    fn unlocked_app() -> (App, String, tempfile::TempDir) {
        let _guard = crate::VAULT_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::TempDir::new().unwrap();
        let vault = VaultService::with_base_path(dir.path().to_path_buf());
        let account = vault.create_account("Test", crate::TEST_PASSWORD, None).unwrap();
        let account_id = account["id"].as_str().unwrap().to_string();
        let app = App::new(Arc::new(vault)).unwrap();
        (app, account_id, dir)
    }

    #[test]
    fn test_newpage_success() {
        let (mut app, account_id, _dir) = unlocked_app();
        super::newpage(&mut app, Some("旅行")).unwrap();

        let vault = app.vault_service.get_vault_store().unwrap();
        let pages = vault.list_objects(&account_id, Some("page"), None, None, false, false).unwrap();
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
    fn test_delete_restore_purge_object_lifecycle() {
        let (mut app, account_id, _dir) = unlocked_app();
        let vault = app.vault_service.get_vault_store().unwrap();

        // 创建页面和对象
        let page = objects::create_page(&vault, &account_id, "旅行").unwrap();
        let obj = objects::create_object(
            &vault, &account_id, &page.id, "待删除",
            serde_json::json!({}), None, None,
        ).unwrap();

        // 删除
        super::delete(&mut app, Some(&obj.id)).unwrap();
        assert!(vault.load_object(&obj.id).unwrap().unwrap().is_deleted);

        // 回收站
        let trash = vault.list_trash_items(None, None).unwrap();
        assert_eq!(trash.len(), 1);
        let trash_id = trash[0].id.clone();

        // 恢复
        super::restore(&mut app, Some(&trash_id)).unwrap();
        assert!(vault.list_trash_items(None, None).unwrap().is_empty());
        let restored = vault.load_object(&obj.id).unwrap().unwrap();
        assert!(!restored.is_deleted);
        assert_eq!(restored.name, "待删除");

        // 再次删除并彻底删除
        super::delete(&mut app, Some(&obj.id)).unwrap();
        let trash = vault.list_trash_items(None, None).unwrap();
        let trash_id = trash[0].id.clone();
        // require_unlocked check 需要 App 引用
        let vault_ref = app.vault_service.get_vault_store().unwrap();
        // 直接调用 core 的 purge_trash
        objects::purge_trash(&vault_ref, &trash_id).unwrap();
        assert!(vault.load_object(&obj.id).unwrap().is_none());
    }

    #[test]
    fn test_edit_and_save() {
        let (mut app, account_id, _dir) = unlocked_app();
        let vault = app.vault_service.get_vault_store().unwrap();
        let page = objects::create_page(&vault, &account_id, "旅行").unwrap();
        let mut obj = objects::create_object(
            &vault, &account_id, &page.id, "原名称",
            serde_json::json!({"title": "old"}), None, None,
        ).unwrap();

        obj.name = "新名称".to_string();
        super::save_edited_object(&mut app, obj).unwrap();

        // 检查是否更新
        let objects_list = vault.list_objects(&account_id, None, Some(&page.id), None, false, false).unwrap();
        assert_eq!(objects_list[0].name, "新名称");
    }
}
