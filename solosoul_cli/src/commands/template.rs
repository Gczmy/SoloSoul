//! 模板管理命令：/template、/template show <id>、/template delete <id>。

use color_eyre::Result;
use solosoul_core::template_service::SystemTemplateRegistry;
use solosoul_core::UserTemplate;

use crate::app::{App, AppPhase};
use crate::commands::{map_err, require_unlocked_with_vault};

/// 命令入口。
pub fn handle(app: &mut App, args: &[&str]) -> Result<()> {
    let sub = args.get(1).copied().unwrap_or("");
    match sub {
        "" => list_templates(app),
        "show" => show_template(app, args.get(2).copied()),
        "delete" => delete_template(app, args.get(2).copied()),
        _ => {
            app.error_message =
                Some("用法: /template | /template show <id> | /template delete <id>".to_string());
            Ok(())
        }
    }
}

/// 加载系统模板注册表，使用当前 UI 语言。
fn load_system_templates(app: &App) -> Result<SystemTemplateRegistry, String> {
    let locale = crate::commands::settings::load_ui_prefs(app)
        .get("language")
        .and_then(|v| v.as_str())
        .unwrap_or("en-US")
        .to_string();
    Ok(SystemTemplateRegistry::load_for_locale(&locale)?)
}

/// 执行 `/template`：打开模板列表屏幕。
fn list_templates(app: &mut App) -> Result<()> {
    let (account_id, vault) = require_unlocked_with_vault(app)?;
    let user_templates = vault.list_user_templates(&account_id).map_err(map_err)?;
    let system_registry = match load_system_templates(app) {
        Ok(r) => r,
        Err(e) => {
            app.error_message = Some(format!("加载系统模板失败: {}", e));
            return Ok(());
        }
    };
    let system_templates: Vec<solosoul_core::template_service::SystemTemplate> =
        system_registry.list_all();

    app.previous_phase = Some(app.phase.clone());
    app.phase = AppPhase::TemplateList {
        user_templates,
        system_templates,
        selected: 0,
    };
    Ok(())
}

/// 查找模板：先用户模板，再系统模板。
fn find_template<'a>(
    user_templates: &'a [UserTemplate],
    system_templates: &'a [solosoul_core::template_service::SystemTemplate],
    id: &str,
) -> TemplateView<'a> {
    if let Some(t) = user_templates.iter().find(|t| t.id == id) {
        return TemplateView::User(t);
    }
    if let Some(t) = system_templates.iter().find(|t| t.key == id) {
        return TemplateView::System(t);
    }
    TemplateView::None
}

enum TemplateView<'a> {
    User(&'a UserTemplate),
    System(&'a solosoul_core::template_service::SystemTemplate),
    None,
}

/// 执行 `/template show <id>`：打开模板详情屏幕。
fn show_template(app: &mut App, id: Option<&str>) -> Result<()> {
    let id = match id {
        Some(id) => id,
        None => {
            app.error_message = Some("用法: /template show <id>".to_string());
            return Ok(());
        }
    };

    let (account_id, vault) = require_unlocked_with_vault(app)?;
    let user_templates = vault.list_user_templates(&account_id).map_err(map_err)?;
    let system_registry = match load_system_templates(app) {
        Ok(r) => r,
        Err(e) => {
            app.error_message = Some(format!("加载系统模板失败: {}", e));
            return Ok(());
        }
    };
    let system_templates = system_registry.list_all();

    match find_template(&user_templates, &system_templates, id) {
        TemplateView::User(t) => {
            app.previous_phase = Some(app.phase.clone());
            app.phase = AppPhase::TemplateDetail {
                template_id: id.to_string(),
                name: t.name.clone(),
                source: "用户".to_string(),
                json: serde_json::to_string_pretty(t).unwrap_or_default(),
            };
        }
        TemplateView::System(t) => {
            app.previous_phase = Some(app.phase.clone());
            app.phase = AppPhase::TemplateDetail {
                template_id: id.to_string(),
                name: t.name_fallback.clone(),
                source: "系统".to_string(),
                json: serde_json::to_string_pretty(t).unwrap_or_default(),
            };
        }
        TemplateView::None => {
            app.error_message = Some(format!("模板 '{}' 不存在", id));
        }
    }
    Ok(())
}

/// 执行 `/template delete <id>`：删除用户模板。
fn delete_template(app: &mut App, id: Option<&str>) -> Result<()> {
    let id = match id {
        Some(id) => id,
        None => {
            app.error_message = Some("用法: /template delete <id>".to_string());
            return Ok(());
        }
    };

    let (_account_id, vault) = require_unlocked_with_vault(app)?;
    match vault.delete_user_template(id).map_err(map_err) {
        Ok(()) => {
            app.error_message = Some(format!("已删除用户模板: {}", id));
            list_templates(app)?;
        }
        Err(e) => {
            app.error_message = Some(format!("删除失败: {}", e));
        }
    }
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
        let app = App::new(Arc::new(vault)).unwrap();
        app.vault_service
            .unlock(&account_id, crate::TEST_PASSWORD)
            .unwrap();
        (app, account_id, dir)
    }

    #[test]
    fn test_list_templates_opens_screen() {
        let (mut app, _account_id, _dir) = unlocked_app();
        handle(&mut app, &["/template"]).unwrap();
        assert!(matches!(app.phase, AppPhase::TemplateList { .. }));
    }

    #[test]
    fn test_show_system_template() {
        let (mut app, _account_id, _dir) = unlocked_app();
        handle(&mut app, &["/template", "show", "passport"]).unwrap();
        assert!(matches!(app.phase, AppPhase::TemplateDetail { .. }));
    }

    #[test]
    fn test_show_unknown_template() {
        let (mut app, _account_id, _dir) = unlocked_app();
        handle(&mut app, &["/template", "show", "nonexistent"]).unwrap();
        assert!(app
            .error_message
            .as_deref()
            .unwrap_or("")
            .contains("不存在"));
    }

    #[test]
    fn test_delete_missing_id() {
        let (mut app, _account_id, _dir) = unlocked_app();
        handle(&mut app, &["/template", "delete"]).unwrap();
        assert!(app.error_message.as_deref().unwrap_or("").contains("用法"));
    }
}
