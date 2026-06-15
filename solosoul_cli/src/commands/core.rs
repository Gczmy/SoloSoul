//! 核心全局命令（/exit、/back 等）。

use crate::app::{App, AppPhase};

/// 执行 `/exit`：先锁定 Vault，再标记退出。
pub fn exit(app: &mut App) {
    app.vault_service.lock();
    app.password_input.clear();
    app.phase = AppPhase::Quit;
}

/// 执行 `/back`：返回上一屏。
pub fn back(app: &mut App) {
    // 登录向导中的 EnterPassword 按 Esc 时应回到 SelectAccount（如果存在多个账户）
    // 或 Locked（单账户/无选择页）
    if let AppPhase::UnlockWizard {
        step: crate::app::UnlockStep::EnterPassword { .. },
    } = &app.phase
    {
        if let Some(AppPhase::UnlockWizard {
            step: crate::app::UnlockStep::SelectAccount { accounts, selected },
        }) = app.previous_phase.clone()
        {
            app.phase = AppPhase::UnlockWizard {
                step: crate::app::UnlockStep::SelectAccount { accounts, selected },
            };
            app.password_input.clear();
            return;
        }
    }

    if let Some(prev) = app.previous_phase.take() {
        app.phase = prev;
        app.password_input.clear();
    } else {
        app.error_message = Some("没有上一屏可返回".to_string());
    }
}
