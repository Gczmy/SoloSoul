//! 认证相关命令。

use color_eyre::Result;

use crate::app::App;

/// 执行 `/account_list`：列出本地账户并切换 App 状态到展示结果。
pub fn account_list(app: &mut App) -> Result<()> {
    let accounts = app.vault_service.list_accounts();
    app.error_message = None;
    if accounts.is_empty() {
        app.error_message = Some("未发现本地账户".to_string());
    } else {
        app.previous_phase = Some(app.phase.clone());
        app.phase = crate::app::AppPhase::AccountList { accounts };
    }
    Ok(())
}
