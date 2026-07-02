//! 附件命令（CLI 薄封装）。
//!
//! 实际附件业务逻辑已下沉到 `solosoul-core::objects`，
//! 本文件只负责命令分发、参数解析、对象 ID 推断和终端输出。

use color_eyre::Result;
use solosoul_core::objects;

use crate::app::{App, AppPhase};
use crate::commands::require_unlocked_with_vault;

/// 命令入口。
pub fn handle(app: &mut App, args: &[&str]) -> Result<()> {
    let _account_id = require_unlocked_with_vault(app)?;
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

fn current_object_id(app: &App) -> Option<String> {
    match &app.phase {
        AppPhase::ObjectDetail { object } => Some(object.id.clone()),
        AppPhase::EditObjectWizard { object_id, .. } => Some(object_id.clone()),
        AppPhase::HistoryList { object_id, .. } => Some(object_id.clone()),
        AppPhase::AttachmentList { object_id, .. } => Some(object_id.clone()),
        _ => None,
    }
}

fn list(app: &mut App, object_id: Option<&str>) -> Result<()> {
    let (account_id, vault) = require_unlocked_with_vault(app)?;
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

    let record = match vault.load_object(&object_id).map_err(|e| color_eyre::eyre::eyre!(e))? {
        Some(r) if r.account_id == account_id && !r.is_deleted => r,
        _ => {
            app.error_message = Some(format!("对象 '{}' 不存在或已被删除", object_id));
            return Ok(());
        }
    };

    let items: Vec<objects::AttachmentMeta> = objects::load_attachments(&record.properties)
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
    Ok(())
}

fn add(app: &mut App, file_path: Option<&str>) -> Result<()> {
    let (account_id, vault) = require_unlocked_with_vault(app)?;
    let file_path = match file_path {
        Some(p) => p,
        None => {
            app.error_message = Some("请提供文件路径，例如 /attach add /path/to/file.pdf".to_string());
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

    let base = app.vault_service.base_path().to_path_buf();
    match objects::add_attachments(
        &vault,
        &account_id,
        &object_id,
        std::path::Path::new(file_path),
        &base,
    ) {
        Ok(_) => app.error_message = Some(format!("已添加附件: {}", file_path)),
        Err(e) => app.error_message = Some(format!("添加附件失败: {}", e)),
    }
    Ok(())
}

fn rename(app: &mut App, attachment_id: Option<&str>, new_name: Option<&str>) -> Result<()> {
    let (account_id, vault) = require_unlocked_with_vault(app)?;
    let attachment_id = match attachment_id {
        Some(id) => id,
        None => {
            app.error_message = Some("请提供附件 ID，例如 /attach rename att_xxx new.pdf".to_string());
            return Ok(());
        }
    };
    let new_name = match new_name {
        Some(n) if !n.is_empty() => n,
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

    match objects::rename_attachment(&vault, &account_id, &object_id, attachment_id, new_name) {
        Ok(()) => app.error_message = Some(format!("已重命名为: {}", new_name)),
        Err(e) => app.error_message = Some(format!("重命名失败: {}", e)),
    }
    Ok(())
}

fn delete(app: &mut App, attachment_id: Option<&str>) -> Result<()> {
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

    crate::widgets::prompt::open(
        app,
        crate::widgets::prompt::PromptSpec::Confirm {
            message: format!("软删除附件 '{}'？可在回收站恢复。", attachment_id),
            default_yes: false,
        },
        Box::new(move |app, result| {
            if let crate::widgets::prompt::PromptResult::Confirm(true) = result {
                let (account_id, vault) = match require_unlocked_with_vault(app) {
                    Ok(v) => v,
                    Err(_) => return,
                };
                match objects::soft_delete_attachment(&vault, &account_id, &object_id, &attachment_id) {
                    Ok(()) => app.error_message = Some(format!("已删除附件: {}", attachment_id)),
                    Err(e) => app.error_message = Some(format!("删除附件失败: {}", e)),
                }
            }
        }),
    );
    Ok(())
}

fn restore(app: &mut App, attachment_id: Option<&str>) -> Result<()> {
    let (account_id, vault) = require_unlocked_with_vault(app)?;
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

    match objects::restore_attachment(&vault, &account_id, &object_id, attachment_id) {
        Ok(()) => app.error_message = Some(format!("已恢复附件: {}", attachment_id)),
        Err(e) => app.error_message = Some(format!("恢复附件失败: {}", e)),
    }
    Ok(())
}

fn purge(app: &mut App, attachment_id: Option<&str>) -> Result<()> {
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

    crate::widgets::prompt::open(
        app,
        crate::widgets::prompt::PromptSpec::Confirm {
            message: format!("彻底删除附件 '{}'？此操作不可恢复。", attachment_id),
            default_yes: false,
        },
        Box::new(move |app, result| {
            if let crate::widgets::prompt::PromptResult::Confirm(true) = result {
                let (_account_id, vault) = match require_unlocked_with_vault(app) {
                    Ok(v) => v,
                    Err(_) => return,
                };
                let base = app.vault_service.base_path().to_path_buf();
                // 需要 account_id 但已通过 require_unlocked_with_vault 获取，从 vault service 重取
                let account_id = match app.vault_service.get_current_account() {
                    Some(id) => id,
                    None => return,
                };
                match objects::purge_attachment(&vault, &account_id, &object_id, &attachment_id, &base) {
                    Ok(()) => app.error_message = Some(format!("已彻底删除附件: {}", attachment_id)),
                    Err(e) => app.error_message = Some(format!("彻底删除附件失败: {}", e)),
                }
            }
        }),
    );
    Ok(())
}

fn cleanup(app: &mut App) -> Result<()> {
    let (account_id, vault) = require_unlocked_with_vault(app)?;
    let base = app.vault_service.base_path().to_path_buf();

    match objects::cleanup_orphan_attachments(&vault, &account_id, &base) {
        Ok((removed, freed)) => {
            app.error_message = Some(format!(
                "清理完成：移除 {} 个孤立附件，释放 {} 字节",
                removed, freed
            ));
        }
        Err(e) => {
            app.error_message = Some(format!("清理附件失败: {}", e));
        }
    }
    Ok(())
}


