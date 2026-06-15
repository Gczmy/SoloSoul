//! 认证相关命令。

use color_eyre::Result;

use crate::app::{App, AppPhase, UnlockStep};

/// 执行 `/account_list`：列出本地账户并切换 App 状态到展示结果。
pub fn account_list(app: &mut App) -> Result<()> {
    let accounts = app.vault_service.list_accounts();
    app.error_message = None;
    if accounts.is_empty() {
        app.error_message = Some("未发现本地账户".to_string());
    } else {
        app.previous_phase = Some(app.phase.clone());
        app.phase = AppPhase::AccountList { accounts };
    }
    Ok(())
}

/// 执行 `/unlock`：启动登录向导。
pub fn unlock(app: &mut App) -> Result<()> {
    if app.vault_service.is_unlocked() {
        app.error_message = Some("当前已登录".to_string());
        return Ok(());
    }

    let accounts = app.vault_service.list_accounts();
    if accounts.is_empty() {
        app.error_message = Some("未发现本地账户，请使用 GUI 客户端创建账户".to_string());
        return Ok(());
    }

    app.previous_phase = Some(app.phase.clone());

    if accounts.len() == 1 {
        let account_id = accounts[0].id.clone();
        app.phase = AppPhase::UnlockWizard {
            step: UnlockStep::EnterPassword { account_id },
        };
        app.password_input.clear();
    } else {
        app.phase = AppPhase::UnlockWizard {
            step: UnlockStep::SelectAccount {
                accounts,
                selected: 0,
            },
        };
    }

    Ok(())
}

/// 执行 `/lock` 或 `/logout`：锁定 Vault 并回到 Locked 状态。
pub fn lock(app: &mut App) {
    if !app.vault_service.is_unlocked() {
        app.error_message = Some("当前未登录".to_string());
        return;
    }
    app.vault_service.lock();
    app.password_input.clear();
    app.phase = AppPhase::Locked;
}
