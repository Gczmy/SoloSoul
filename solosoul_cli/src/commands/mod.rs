//! 命令路由与执行器。

use crate::app::App;

// ---- 共享帮助函数 ----

/// 确保 Vault 已解锁，返回当前账户 ID。
pub fn require_unlocked(app: &mut App) -> color_eyre::Result<String> {
    if !app.vault_service.is_unlocked() {
        app.error_message = Some("请先使用 /unlock 登录".to_string());
        return Err(color_eyre::eyre::eyre!("Vault is locked"));
    }
    app.vault_service
        .get_current_account()
        .ok_or_else(|| color_eyre::eyre::eyre!("No current account"))
}

/// 确保 Vault 已解锁，返回 (账户 ID, VaultStore)。
pub fn require_unlocked_with_vault(app: &mut App) -> color_eyre::Result<(String, std::sync::Arc<solosoul_core::VaultStore>)> {
    let account_id = require_unlocked(app)?;
    let vault = app
        .vault_service
        .get_vault_store()
        .ok_or_else(|| color_eyre::eyre::eyre!("Vault 未打开"))?;
    Ok((account_id, vault))
}

/// 将 String 错误转换为 color_eyre::Report。
pub fn map_err(e: String) -> color_eyre::Report {
    color_eyre::eyre::eyre!(e)
}

/// 获取 VaultStore 引用。
pub fn vault(app: &mut App) -> color_eyre::Result<std::sync::Arc<solosoul_core::VaultStore>> {
    app.vault_service
        .get_vault_store()
        .ok_or_else(|| color_eyre::eyre::eyre!("Vault 未打开"))
}

// ---- CLI 命令错误类型 ----

/// CLI 命令错误类型，简化为 String。
pub type CliError = String;

pub mod attachment;
pub mod auth;
pub mod backup;
pub mod core;
pub mod doctor;
pub mod embed_model;
pub mod export_import;
pub mod history;
pub mod llm;
pub mod log;
pub mod ocr;
pub mod plugin;
pub mod profile;
pub mod search;
pub mod security;
pub mod settings;
pub mod sync;
pub mod system;
pub mod template;
pub mod vault_read;
pub mod vault_write;
