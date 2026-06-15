//! 安全相关命令：/security password、/security hint、/security trash-retention、/security delete-account。

use color_eyre::Result;
use serde_json::{Map, Value};

use crate::app::{App, AppPhase};
use crate::widgets::prompt::{self, PromptResult, PromptSpec};

fn map_err(e: String) -> color_eyre::Report {
    color_eyre::eyre::eyre!(e)
}

/// 命令入口。
pub fn handle(app: &mut App, args: &[&str]) -> Result<()> {
    let sub = args.get(1).copied().unwrap_or("");
    match sub {
        "password" => start_change_password(app),
        "hint" => handle_hint(app, args.get(2).copied()),
        "trash-retention" => handle_trash_retention(app, args.get(2).copied()),
        "delete-account" => start_delete_account(app),
        _ => {
            app.error_message =
                Some("用法: /security password|hint|trash-retention|delete-account".to_string());
            Ok(())
        }
    }
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

/// 更新当前账户加密偏好中的单个键值。
fn update_profile_preference(app: &mut App, key: &str, value: Value) -> Result<()> {
    let account_id = require_unlocked(app)?;

    let vault = app
        .vault_service
        .get_vault_store()
        .ok_or_else(|| color_eyre::eyre::eyre!("Vault 未打开"))?;

    let mut profile = match vault.load_profile(&account_id).map_err(map_err)? {
        Some(p) => p,
        None => solosoul_core::Profile::new_with_id(&account_id, &account_id, Vec::new()),
    };

    let mut data: Value = if profile.data.is_empty() {
        Value::Object(Map::new())
    } else {
        serde_json::from_slice(&profile.data)
            .map_err(|e| color_eyre::eyre::eyre!("解析 profile 数据失败: {}", e))?
    };

    if let Some(obj) = data.as_object_mut() {
        let prefs = obj
            .entry("preferences")
            .or_insert_with(|| Value::Object(Map::new()));
        if let Some(p) = prefs.as_object_mut() {
            p.insert(key.to_string(), value);
        }
    }

    profile.data = serde_json::to_vec(&data).map_err(|e| {
        app.error_message = Some(format!("序列化 profile 数据失败: {}", e));
        color_eyre::eyre::eyre!(e)
    })?;
    profile.updated_at = chrono::Utc::now();
    profile.version += 1;

    vault.save_profile(&profile).map_err(map_err)?;
    Ok(())
}

/// 执行 `/security password`：通过连续提示修改主密码。
fn start_change_password(app: &mut App) -> Result<()> {
    let account_id = require_unlocked(app)?;

    prompt::open(
        app,
        PromptSpec::Text {
            label: "当前主密码".to_string(),
            initial: String::new(),
            mask: true,
            allow_toggle_mask: true,
        },
        Box::new(move |app, result| {
            if let PromptResult::Text(old_password) = result {
                let account_id = account_id.clone();
                prompt::open(
                    app,
                    PromptSpec::Text {
                        label: "新主密码".to_string(),
                        initial: String::new(),
                        mask: true,
                        allow_toggle_mask: true,
                    },
                    Box::new(move |app, result| {
                        if let PromptResult::Text(new_password) = result {
                            if new_password.len() < 8 {
                                app.error_message = Some("主密码至少需要 8 位".to_string());
                                return;
                            }
                            let account_id = account_id.clone();
                            let old_password = old_password.clone();
                            prompt::open(
                                app,
                                PromptSpec::Text {
                                    label: "确认新主密码".to_string(),
                                    initial: String::new(),
                                    mask: true,
                                    allow_toggle_mask: true,
                                },
                                Box::new(move |app, result| {
                                    if let PromptResult::Text(confirm_password) = result {
                                        if new_password != confirm_password {
                                            app.error_message =
                                                Some("两次输入的新密码不一致".to_string());
                                            return;
                                        }
                                        match app.vault_service.change_password(
                                            &account_id,
                                            &old_password,
                                            &new_password,
                                        ) {
                                            Ok(()) => {
                                                app.error_message = Some("主密码已修改".to_string())
                                            }
                                            Err(e) => {
                                                app.error_message = Some(format!("修改失败: {}", e))
                                            }
                                        }
                                    }
                                }),
                            );
                        }
                    }),
                );
            }
        }),
    );

    Ok(())
}

/// 读取账户配置，用于获取密码提示等无需解锁的信息。
fn load_account_config(app: &App, account_id: &str) -> Option<solosoul_core::AccountConfig> {
    let path = app
        .vault_service
        .base_path()
        .join(account_id)
        .join("config.json");
    if !path.exists() {
        return None;
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|c| serde_json::from_str::<solosoul_core::AccountConfig>(&c).ok())
}

/// 执行 `/security hint [text]`：获取或设置密码提示词。
fn handle_hint(app: &mut App, text: Option<&str>) -> Result<()> {
    let account_id = app
        .vault_service
        .get_current_account()
        .ok_or_else(|| color_eyre::eyre::eyre!("No current account"))?;

    match text {
        Some(text) => {
            app.vault_service
                .update_password_hint(&account_id, text)
                .map_err(|e| {
                    app.error_message = Some(format!("更新密码提示失败: {}", e));
                    color_eyre::eyre::eyre!(e)
                })?;
            app.error_message = Some(format!("密码提示已更新为: {}", text));
        }
        None => {
            let hint = load_account_config(app, &account_id)
                .and_then(|c| c.password_hint)
                .unwrap_or_default();
            app.error_message = Some(format!("当前密码提示: {}", hint));
        }
    }
    Ok(())
}

/// 执行 `/security trash-retention <days>`：设置回收站保留天数。
fn handle_trash_retention(app: &mut App, days: Option<&str>) -> Result<()> {
    let days = match days.and_then(|s| s.parse::<u64>().ok()) {
        Some(d) => d,
        None => {
            app.error_message = Some("用法: /security trash-retention <天数>".to_string());
            return Ok(());
        }
    };

    // 转换为毫秒，便于与 GUI 偏好保持一致。
    let ms = days.saturating_mul(24 * 60 * 60 * 1000);
    update_profile_preference(app, "trashRetention", Value::Number(ms.into()))?;
    app.error_message = Some(format!("回收站保留天数已设置为: {}", days));
    Ok(())
}

/// 执行 `/security delete-account`：验证密码并确认后删除账户。
fn start_delete_account(app: &mut App) -> Result<()> {
    let account_id = require_unlocked(app)?;

    prompt::open(
        app,
        PromptSpec::Text {
            label: "输入当前主密码以确认删除账户".to_string(),
            initial: String::new(),
            mask: true,
            allow_toggle_mask: true,
        },
        Box::new(move |app, result| {
            if let PromptResult::Text(password) = result {
                match app.vault_service.verify_password(&account_id, &password) {
                    Ok(true) => {
                        let account_id = account_id.clone();
                        prompt::open(
                            app,
                            PromptSpec::Confirm {
                                message: "! 删除账户将永久清除所有数据，是否继续？".to_string(),
                                default_yes: false,
                            },
                            Box::new(move |app, result| {
                                if let PromptResult::Confirm(true) = result {
                                    match app.vault_service.delete_account(&account_id) {
                                        Ok(()) => {
                                            app.vault_service.lock();
                                            app.phase = AppPhase::Locked;
                                            app.error_message = Some("账户已删除".to_string());
                                        }
                                        Err(e) => {
                                            app.error_message = Some(format!("删除失败: {}", e))
                                        }
                                    }
                                }
                            }),
                        );
                    }
                    Ok(false) => app.error_message = Some("密码错误，账户删除已取消".to_string()),
                    Err(e) => app.error_message = Some(format!("验证失败: {}", e)),
                }
            }
        }),
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent};
    use solosoul_core::VaultService;
    use std::sync::Arc;

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

    fn answer_text_prompt(app: &mut App, text: &str) {
        if let Some(state) = app.prompt.as_mut() {
            state.value = text.to_string();
            state.cursor = state.value.chars().count();
        }
        app.handle_event(crate::events::Event::Key(KeyEvent::from(KeyCode::Enter)))
            .unwrap();
    }

    #[test]
    fn test_hint_get_and_set() {
        let (mut app, account_id, _dir) = unlocked_app();

        // 设置提示
        handle(&mut app, &["/security", "hint", "my favorite color"]).unwrap();
        assert!(app
            .error_message
            .as_deref()
            .unwrap_or("")
            .contains("my favorite color"));

        // 读取提示
        handle(&mut app, &["/security", "hint"]).unwrap();
        assert!(app
            .error_message
            .as_deref()
            .unwrap_or("")
            .contains("my favorite color"));

        let cfg = load_account_config(&app, &account_id).unwrap();
        assert_eq!(cfg.password_hint, Some("my favorite color".to_string()));
    }

    #[test]
    fn test_trash_retention() {
        let (mut app, account_id, _dir) = unlocked_app();
        handle(&mut app, &["/security", "trash-retention", "30"]).unwrap();

        let vault = app.vault_service.get_vault_store().unwrap();
        let profile = vault.load_profile(&account_id).unwrap().unwrap();
        let data: Value = serde_json::from_slice(&profile.data).unwrap();
        assert_eq!(
            data["preferences"]["trashRetention"],
            30_u64 * 24 * 60 * 60 * 1000
        );
    }

    #[test]
    fn test_delete_account_rejects_wrong_password() {
        let (mut app, account_id, _dir) = unlocked_app();
        handle(&mut app, &["/security", "delete-account"]).unwrap();
        assert!(app.prompt.is_some());

        answer_text_prompt(&mut app, "wrongpassword");

        assert!(app
            .error_message
            .as_deref()
            .unwrap_or("")
            .contains("密码错误"));
        assert!(app
            .vault_service
            .verify_password(&account_id, "password123")
            .unwrap());
    }

    #[test]
    fn test_change_password() {
        let (mut app, account_id, _dir) = unlocked_app();
        handle(&mut app, &["/security", "password"]).unwrap();
        assert!(app.prompt.is_some());

        answer_text_prompt(&mut app, "password123");
        answer_text_prompt(&mut app, "newpassword");
        answer_text_prompt(&mut app, "newpassword");

        assert!(app
            .error_message
            .as_deref()
            .unwrap_or("")
            .contains("主密码已修改"));
        assert!(!app
            .vault_service
            .verify_password(&account_id, "password123")
            .unwrap());
        assert!(app
            .vault_service
            .verify_password(&account_id, "newpassword")
            .unwrap());
    }
}
