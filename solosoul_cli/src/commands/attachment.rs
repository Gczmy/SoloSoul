//! 附件命令（CLI 薄封装）。
//!
//! 实际附件业务逻辑已下沉到 `solosoul-core::objects`，
//! 本文件只负责命令分发、参数解析、对象 ID 推断和终端输出。

use color_eyre::Result;
use solosoul_core::objects;
use std::time::Instant;

use crate::app::{App, AppPhase};
use crate::commands::require_unlocked_with_vault;
use crate::t;

/// 命令入口。
pub fn handle(app: &mut App, args: &[&str]) -> Result<()> {
    let _account_id = require_unlocked_with_vault(app)?;
    match args.first().copied() {
        None | Some("help") => {
            app.error_message = Some(t!(app.i18n, "cmd-attachment-usage"));
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
            app.error_message = Some(t!(app.i18n, "cmd-unknown-subcommand", cmd = other));
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
                app.error_message = Some(t!(app.i18n, "cmd-provide-object-id-or-detail"));
                return Ok(());
            }
        },
    };

    let record = match vault
        .load_object(&object_id)
        .map_err(|e| color_eyre::eyre::eyre!(e))?
    {
        Some(r) if r.account_id == account_id && !r.is_deleted => r,
        _ => {
            app.error_message = Some(t!(app.i18n, "cmd-object-not-found", id = object_id));
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
            app.error_message = Some(t!(app.i18n, "cmd-provide-file-path"));
            return Ok(());
        }
    };
    let object_id = match current_object_id(app) {
        Some(id) => id,
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-execute-in-detail", cmd = "/attach add"));
            return Ok(());
        }
    };

    let base = app.vault_service.base_path().to_path_buf();
    // P001: 附件加密落盘——从已解锁会话派生附件静态加密密钥。
    let att_key: Option<[u8; 32]> = app
        .vault_service
        .attachment_encryption_key()
        .ok()
        .and_then(|k| k.as_slice().try_into().ok());
    match objects::add_attachments(
        &vault,
        &account_id,
        &object_id,
        std::path::Path::new(file_path),
        &base,
        att_key.as_ref(),
    ) {
        Ok(_) => {
            app.success_message = Some((
                t!(app.i18n, "attachment-added", path = file_path),
                Instant::now(),
            ))
        }
        Err(e) => app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e)),
    }
    Ok(())
}

fn rename(app: &mut App, attachment_id: Option<&str>, new_name: Option<&str>) -> Result<()> {
    let (account_id, vault) = require_unlocked_with_vault(app)?;
    let attachment_id = match attachment_id {
        Some(id) => id,
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-provide-attachment-id-example"));
            return Ok(());
        }
    };
    let new_name = match new_name {
        Some(n) if !n.is_empty() => n,
        _ => {
            app.error_message = Some(t!(app.i18n, "cmd-provide-filename"));
            return Ok(());
        }
    };
    let object_id = match current_object_id(app) {
        Some(id) => id,
        None => {
            app.error_message = Some(t!(
                app.i18n,
                "cmd-execute-in-detail",
                cmd = "/attach rename"
            ));
            return Ok(());
        }
    };

    match objects::rename_attachment(&vault, &account_id, &object_id, attachment_id, new_name) {
        Ok(()) => {
            app.success_message = Some((
                t!(app.i18n, "attachment-renamed", name = new_name),
                Instant::now(),
            ))
        }
        Err(e) => app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e)),
    }
    Ok(())
}

fn delete(app: &mut App, attachment_id: Option<&str>) -> Result<()> {
    let attachment_id = match attachment_id {
        Some(id) => id.to_string(),
        None => {
            app.error_message = Some(t!(
                app.i18n,
                "cmd-provide-attachment-id",
                cmd = "/attach delete"
            ));
            return Ok(());
        }
    };
    let object_id = match current_object_id(app) {
        Some(id) => id,
        None => {
            app.error_message = Some(t!(
                app.i18n,
                "cmd-execute-in-detail",
                cmd = "/attach delete"
            ));
            return Ok(());
        }
    };

    crate::widgets::prompt::open(
        app,
        crate::widgets::prompt::PromptSpec::Confirm {
            message: t!(
                app.i18n,
                "cmd-prompt-soft-delete-attachment",
                id = &attachment_id
            ),
            default_yes: false,
        },
        Box::new(move |app, result| {
            if let crate::widgets::prompt::PromptResult::Confirm(true) = result {
                let (account_id, vault) = match require_unlocked_with_vault(app) {
                    Ok(v) => v,
                    Err(_) => return,
                };
                match objects::soft_delete_attachment(
                    &vault,
                    &account_id,
                    &object_id,
                    &attachment_id,
                ) {
                    Ok(()) => {
                        app.success_message = Some((
                            t!(app.i18n, "attachment-deleted", id = attachment_id),
                            Instant::now(),
                        ))
                    }
                    Err(e) => {
                        app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e))
                    }
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
            app.error_message = Some(t!(
                app.i18n,
                "cmd-provide-attachment-id",
                cmd = "/attach restore"
            ));
            return Ok(());
        }
    };
    let object_id = match current_object_id(app) {
        Some(id) => id,
        None => {
            app.error_message = Some(t!(
                app.i18n,
                "cmd-execute-in-detail",
                cmd = "/attach restore"
            ));
            return Ok(());
        }
    };

    match objects::restore_attachment(&vault, &account_id, &object_id, attachment_id) {
        Ok(()) => {
            app.success_message = Some((
                t!(app.i18n, "attachment-restored", id = attachment_id),
                Instant::now(),
            ))
        }
        Err(e) => app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e)),
    }
    Ok(())
}

fn purge(app: &mut App, attachment_id: Option<&str>) -> Result<()> {
    let attachment_id = match attachment_id {
        Some(id) => id.to_string(),
        None => {
            app.error_message = Some(t!(
                app.i18n,
                "cmd-provide-attachment-id",
                cmd = "/attach purge"
            ));
            return Ok(());
        }
    };
    let object_id = match current_object_id(app) {
        Some(id) => id,
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-execute-in-detail", cmd = "/attach purge"));
            return Ok(());
        }
    };

    crate::widgets::prompt::open(
        app,
        crate::widgets::prompt::PromptSpec::Confirm {
            message: t!(app.i18n, "cmd-prompt-purge-attachment", id = &attachment_id),
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
                match objects::purge_attachment(
                    &vault,
                    &account_id,
                    &object_id,
                    &attachment_id,
                    &base,
                ) {
                    Ok(()) => {
                        app.success_message = Some((
                            t!(app.i18n, "attachment-purged", id = attachment_id),
                            Instant::now(),
                        ))
                    }
                    Err(e) => {
                        app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e))
                    }
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
            app.success_message = Some((
                t!(
                    app.i18n,
                    "cmd-cleanup-result",
                    count = removed.to_string(),
                    bytes = freed.to_string()
                ),
                Instant::now(),
            ));
        }
        Err(e) => {
            app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e));
        }
    }
    Ok(())
}
