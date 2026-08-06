//! Vault 只读命令。

use color_eyre::Result;

use crate::app::{App, AppPhase, SizeReport};
use crate::commands::require_unlocked;
use crate::t;

/// `/list` 结果上限：与 `/search` 的 200 条截断对称，避免超大 Vault 一次渲染全部。
const LIST_RESULT_LIMIT: usize = 200;

/// 执行 `/list [page_name]`：列出页面或页面内对象。
pub fn list(app: &mut App, page_name: Option<&str>) -> Result<()> {
    let account_id = require_unlocked(app)?;

    let vault = app
        .vault_service
        .get_vault_store()
        .ok_or_else(|| color_eyre::eyre::eyre!("Vault not open"))?;

    match page_name {
        None => {
            let mut pages = vault
                .list_objects(&account_id, Some("page"), None, None, false, false)
                .map_err(|e| color_eyre::eyre::eyre!(e))?;
            pages.truncate(LIST_RESULT_LIMIT);
            app.previous_phase = Some(app.phase.clone());
            app.phase = AppPhase::ObjectList {
                title: t!(app.i18n, "object-list-title"),
                items: pages,
            };
        }
        Some(name) => {
            // 精确匹配页面名（忽略大小写）
            let pages = vault
                .list_objects(&account_id, Some("page"), None, None, false, false)
                .map_err(|e| color_eyre::eyre::eyre!(e))?;
            let matched = pages
                .iter()
                .find(|p| p.name.eq_ignore_ascii_case(name))
                .cloned();

            match matched {
                Some(page) => {
                    let mut objects = vault
                        .list_objects(&account_id, None, Some(&page.id), None, false, false)
                        .map_err(|e| color_eyre::eyre::eyre!(e))?;
                    objects.truncate(LIST_RESULT_LIMIT);
                    app.previous_phase = Some(app.phase.clone());
                    app.phase = AppPhase::ObjectList {
                        title: format!("{}: {}", t!(app.i18n, "object-list-title"), page.name),
                        items: objects,
                    };
                }
                None => {
                    app.error_message = Some(t!(app.i18n, "cmd-page-not-found", name = name));
                }
            }
        }
    }

    Ok(())
}

/// 执行 `/open <id>`：查看对象详情。
pub fn open(app: &mut App, object_id: Option<&str>) -> Result<()> {
    let account_id = require_unlocked(app)?;

    let id = match object_id {
        Some(id) => id,
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-provide-object-id", cmd = "/open"));
            return Ok(());
        }
    };

    let vault = app
        .vault_service
        .get_vault_store()
        .ok_or_else(|| color_eyre::eyre::eyre!("Vault not open"))?;

    match vault
        .load_object(id)
        .map_err(|e| color_eyre::eyre::eyre!(e))?
    {
        Some(record) if record.account_id == account_id && !record.is_deleted => {
            app.previous_phase = Some(app.phase.clone());
            app.phase = AppPhase::ObjectDetail { object: record };
        }
        _ => {
            app.error_message = Some(t!(app.i18n, "cmd-object-not-found", id = id));
        }
    }

    Ok(())
}

/// 执行 `/size`：账户统计。
pub fn size(app: &mut App) -> Result<()> {
    let account_id = require_unlocked(app)?;

    let vault = app
        .vault_service
        .get_vault_store()
        .ok_or_else(|| color_eyre::eyre::eyre!("Vault not open"))?;

    let stats = vault.stats().map_err(|e| color_eyre::eyre::eyre!(e))?;
    let pages = vault
        .list_objects(&account_id, Some("page"), None, None, false, false)
        .map_err(|e| color_eyre::eyre::eyre!(e))?;
    let objects = vault
        .list_objects(&account_id, None, None, None, false, false)
        .map_err(|e| color_eyre::eyre::eyre!(e))?;
    let trash = vault
        .list_objects(&account_id, None, None, None, false, true)
        .map_err(|e| color_eyre::eyre::eyre!(e))?;

    let report = SizeReport {
        page_count: pages.len(),
        object_count: objects.len().saturating_sub(pages.len()),
        trash_count: trash.len(),
        profile_count: stats.profile_count,
        total_size_bytes: stats.total_size_bytes,
    };

    app.previous_phase = Some(app.phase.clone());
    app.phase = AppPhase::Size { report };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use solosoul_core::VaultService;
    use std::sync::Arc;

    fn unlocked_app() -> (App, String, tempfile::TempDir) {
        let _guard = crate::VAULT_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::TempDir::new().unwrap();
        let vault = VaultService::with_base_path(dir.path().to_path_buf());
        let account = vault
            .create_account("Test", crate::TEST_PASSWORD, None)
            .unwrap();
        let account_id = account["id"].as_str().unwrap().to_string();
        // create_account 后已解锁
        let app = App::new(Arc::new(vault)).unwrap();
        (app, account_id, dir)
    }

    #[test]
    fn test_list_pages_empty() {
        let (mut app, _id, _dir) = unlocked_app();
        list(&mut app, None).unwrap();
        match &app.phase {
            AppPhase::ObjectList { title: _, items } => {
                assert!(items.is_empty());
            }
            _ => panic!("expected ObjectList"),
        }
    }

    #[test]
    fn test_list_unknown_page() {
        let (mut app, _id, _dir) = unlocked_app();
        list(&mut app, Some("不存在")).unwrap();
        assert!(app.error_message.is_some());
    }

    #[test]
    fn test_open_missing_id() {
        let (mut app, _id, _dir) = unlocked_app();
        open(&mut app, None).unwrap();
        assert!(app.error_message.is_some());
    }

    #[test]
    fn test_open_nonexistent_object() {
        let (mut app, _id, _dir) = unlocked_app();
        open(&mut app, Some("obj_missing")).unwrap();
        assert!(app.error_message.is_some());
    }

    #[test]
    fn test_size_empty_account() {
        let (mut app, _id, _dir) = unlocked_app();
        size(&mut app).unwrap();
        match &app.phase {
            AppPhase::Size { report } => {
                assert_eq!(report.page_count, 0);
                assert_eq!(report.object_count, 0);
                assert_eq!(report.trash_count, 0);
            }
            _ => panic!("expected Size"),
        }
    }
}
