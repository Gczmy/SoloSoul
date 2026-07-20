//! 安全相关命令：/security password、/security hint、/security trash-retention、/security delete-account、/security biometric。

use color_eyre::Result;
use serde_json::Value;
use solosoul_core::biometric::BiometricManager;

use crate::app::{App, AppPhase};
use crate::commands::require_unlocked;
use crate::t;
use crate::widgets::prompt::{self, PromptResult, PromptSpec};

/// 命令入口。
pub fn handle(app: &mut App, args: &[&str]) -> Result<()> {
    let sub = args.get(1).copied().unwrap_or("");
    match sub {
        "password" => start_change_password(app),
        "hint" => handle_hint(app, args.get(2).copied()),
        "trash-retention" => handle_trash_retention(app, args.get(2).copied()),
        "delete-account" => start_delete_account(app),
        "biometric" => handle_biometric(app, args.get(2).copied(), args.get(3).copied()),
        _ => {
            app.error_message = Some(t!(app.i18n, "cmd-security-usage"));
            Ok(())
        }
    }
}

fn biometric_manager(app: &App) -> BiometricManager {
    BiometricManager::new(app.vault_service.base_path().to_path_buf())
}

/// 执行 `/security password`：通过连续提示修改主密码。
/// P119: 使用链式函数替代三层嵌套回调。
fn start_change_password(app: &mut App) -> Result<()> {
    let account_id = require_unlocked(app)?;

    prompt::open(
        app,
        PromptSpec::Text {
            label: t!(app.i18n, "prompt-current-password"),
            initial: String::new(),
            mask: true,
            allow_toggle_mask: true,
        },
        Box::new(move |app, result| on_old_password(app, result, account_id)),
    );

    Ok(())
}

/// 步骤 1: 接收旧密码，提示新密码。
fn on_old_password(app: &mut App, result: PromptResult, account_id: String) {
    if let PromptResult::Text(old_password) = result {
        prompt::open(
            app,
            PromptSpec::Text {
                label: t!(app.i18n, "prompt-new-password"),
                initial: String::new(),
                mask: true,
                allow_toggle_mask: true,
            },
            Box::new(move |app, result| on_new_password(app, result, account_id, old_password)),
        );
    }
}

/// 步骤 2: 接收新密码，提示确认密码。
fn on_new_password(app: &mut App, result: PromptResult, account_id: String, old_password: String) {
    if let PromptResult::Text(new_password) = result {
        if new_password.len() < 8 {
            app.error_message = Some(t!(app.i18n, "cmd-password-min-length"));
            return;
        }
        prompt::open(
            app,
            PromptSpec::Text {
                label: t!(app.i18n, "prompt-confirm-password"),
                initial: String::new(),
                mask: true,
                allow_toggle_mask: true,
            },
            Box::new(move |app, result| {
                on_confirm_password(app, result, account_id, old_password, new_password)
            }),
        );
    }
}

/// 步骤 3: 接收确认密码，执行修改。
fn on_confirm_password(
    app: &mut App,
    result: PromptResult,
    account_id: String,
    old_password: String,
    new_password: String,
) {
    if let PromptResult::Text(confirm_password) = result {
        if new_password != confirm_password {
            app.error_message = Some(t!(app.i18n, "cmd-password-mismatch"));
            return;
        }
        match app
            .vault_service
            .change_password(&account_id, &old_password, &new_password)
        {
            Ok(()) => app.error_message = Some(t!(app.i18n, "cmd-password-changed")),
            Err(e) => app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e)),
        }
    }
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
                    app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e));
                    color_eyre::eyre::eyre!(e)
                })?;
            app.error_message = Some(t!(app.i18n, "cmd-password-hint-updated", text = text));
        }
        None => {
            let hint = load_account_config(app, &account_id)
                .and_then(|c| c.password_hint)
                .unwrap_or_default();
            app.error_message = Some(t!(app.i18n, "cmd-password-hint-current", hint = hint));
        }
    }
    Ok(())
}

/// 执行 `/security trash-retention <days>`：设置回收站保留天数。
fn handle_trash_retention(app: &mut App, days: Option<&str>) -> Result<()> {
    let days = match days.and_then(|s| s.parse::<u64>().ok()) {
        Some(d) => d,
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-trash-retention-usage"));
            return Ok(());
        }
    };

    // 转换为毫秒，便于与 GUI 偏好保持一致。
    let ms = days.saturating_mul(24 * 60 * 60 * 1000);
    crate::commands::update_profile_preference(app, "trashRetention", Value::Number(ms.into()))?;
    app.error_message = Some(t!(
        app.i18n,
        "cmd-trash-retention-set",
        days = days.to_string()
    ));
    Ok(())
}

/// 执行 `/security biometric status|enable|disable|test`：管理生物识别登录。
fn handle_biometric(app: &mut App, action: Option<&str>, reason: Option<&str>) -> Result<()> {
    let account_id = require_unlocked(app)?;
    let manager = biometric_manager(app);
    let availability = manager.availability(&account_id);

    match action {
        Some("status") | None => {
            let status = if availability.available {
                "可用"
            } else {
                "不可用"
            };
            let configured = if availability.configured {
                "已启用"
            } else {
                "未启用"
            };
            let kind = availability
                .biometry_type
                .as_deref()
                .unwrap_or(t!(app.i18n, "biometric-generic-name").as_str())
                .to_string();
            app.error_message = Some(t!(
                app.i18n,
                "cmd-biometric-status",
                status = status,
                configured = configured,
                kind = kind,
                error = availability.error.as_deref().unwrap_or("")
            ));
            Ok(())
        }
        Some("enable") => {
            if !availability.available {
                app.error_message = Some(t!(app.i18n, "cmd-biometric-not-supported"));
                return Ok(());
            }
            let reason = reason
                .map(|s| s.to_string())
                .unwrap_or_else(|| "启用 SoloSoul 生物识别登录".to_string());
            prompt::open(
                app,
                PromptSpec::Text {
                    label: t!(app.i18n, "prompt-enable-biometric"),
                    initial: String::new(),
                    mask: true,
                    allow_toggle_mask: true,
                },
                Box::new(move |app, result| {
                    if let PromptResult::Text(password) = result {
                        let manager = biometric_manager(app);
                        match manager.save_credential(&account_id, &password, &reason) {
                            Ok(()) => {
                                app.error_message = Some(t!(app.i18n, "cmd-biometric-enabled"))
                            }
                            Err(e) => {
                                app.error_message =
                                    Some(t!(app.i18n, "cmd-operation-failed", err = e))
                            }
                        }
                    }
                }),
            );
            Ok(())
        }
        Some("disable") => {
            prompt::open(
                app,
                PromptSpec::Text {
                    label: t!(app.i18n, "prompt-disable-biometric"),
                    initial: String::new(),
                    mask: true,
                    allow_toggle_mask: true,
                },
                Box::new(move |app, result| {
                    if let PromptResult::Text(password) = result {
                        let manager = biometric_manager(app);
                        match manager.delete_credential(&account_id, &password) {
                            Ok(()) => {
                                app.error_message = Some(t!(app.i18n, "cmd-biometric-disabled"))
                            }
                            Err(e) => {
                                app.error_message =
                                    Some(t!(app.i18n, "cmd-operation-failed", err = e))
                            }
                        }
                    }
                }),
            );
            Ok(())
        }
        Some("test") => {
            let reason = reason
                .map(|s| s.to_string())
                .unwrap_or_else(|| "SoloSoul 生物识别测试".to_string());
            match manager.test(&reason) {
                Ok(true) => app.error_message = Some(t!(app.i18n, "cmd-biometric-test-passed")),
                Ok(false) => {
                    app.error_message = Some(t!(app.i18n, "cmd-biometric-test-unavailable"))
                }
                Err(e) => app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e)),
            }
            Ok(())
        }
        Some(other) => {
            app.error_message = Some(t!(app.i18n, "cmd-unknown-subcommand", cmd = other));
            Ok(())
        }
    }
}

/// 执行 `/security delete-account`：验证密码并确认后删除账户。
fn start_delete_account(app: &mut App) -> Result<()> {
    let account_id = require_unlocked(app)?;

    prompt::open(
        app,
        PromptSpec::Text {
            label: t!(app.i18n, "prompt-delete-account"),
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
                                message: t!(app.i18n, "prompt-delete-account-confirm"),
                                default_yes: false,
                            },
                            Box::new(move |app, result| {
                                if let PromptResult::Confirm(true) = result {
                                    match app.vault_service.delete_account(&account_id) {
                                        Ok(()) => {
                                            app.vault_service.lock();
                                            app.phase = AppPhase::Locked;
                                            app.error_message =
                                                Some(t!(app.i18n, "cmd-account-deleted"));
                                        }
                                        Err(e) => {
                                            app.error_message =
                                                Some(t!(app.i18n, "cmd-operation-failed", err = e))
                                        }
                                    }
                                }
                            }),
                        );
                    }
                    Ok(false) => {
                        app.error_message = Some(t!(app.i18n, "cmd-password-wrong-canceled"))
                    }
                    Err(e) => app.error_message = Some(t!(app.i18n, "cmd-verify-failed", err = e)),
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
        let vault = VaultService::with_base_path(dir.path().to_path_buf());
        let account = vault
            .create_account("Test", crate::TEST_PASSWORD, None)
            .unwrap();
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
            .verify_password(&account_id, crate::TEST_PASSWORD)
            .unwrap());
    }

    #[test]
    fn test_biometric_status() {
        let (mut app, _account_id, _dir) = unlocked_app();
        handle(&mut app, &["/security", "biometric", "status"]).unwrap();
        let msg = app.error_message.as_deref().unwrap_or("");
        assert!(msg.contains("生物识别"));
    }

    #[test]
    fn test_biometric_unknown_subcommand() {
        let (mut app, _account_id, _dir) = unlocked_app();
        handle(&mut app, &["/security", "biometric", "foo"]).unwrap();
        assert!(app
            .error_message
            .as_deref()
            .unwrap_or("")
            .contains("未知子命令"));
    }

    #[test]
    fn test_change_password() {
        let (mut app, account_id, _dir) = unlocked_app();
        handle(&mut app, &["/security", "password"]).unwrap();
        assert!(app.prompt.is_some());

        answer_text_prompt(&mut app, crate::TEST_PASSWORD);
        answer_text_prompt(&mut app, "newpassword");
        answer_text_prompt(&mut app, "newpassword");

        assert!(app
            .error_message
            .as_deref()
            .unwrap_or("")
            .contains("主密码已修改"));
        assert!(!app
            .vault_service
            .verify_password(&account_id, crate::TEST_PASSWORD)
            .unwrap());
        assert!(app
            .vault_service
            .verify_password(&account_id, "newpassword")
            .unwrap());
    }
}
