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

/// CLI 命令错误类型。
///
/// P230: 使用 thiserror 枚举替代 `String` 错误，便于调用方匹配特定错误。
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    /// 通用错误消息（显示给用户的文本）。
    #[error("{0}")]
    Msg(String),

    /// Vault 未解锁。
    #[error("Vault is locked")]
    VaultLocked,

    /// 没有当前账户。
    #[error("No current account")]
    NoAccount,

    /// IO 错误。
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON 序列化/反序列化错误。
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// ZIP 操作错误。
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    /// 加密操作错误。
    #[error("Cryptography error: {0}")]
    Crypto(String),

    /// 验证类错误（密码不匹配等）。
    #[error("{0}")]
    Validation(String),
}

impl From<String> for CliError {
    fn from(s: String) -> Self {
        CliError::Msg(s)
    }
}

impl From<&str> for CliError {
    fn from(s: &str) -> Self {
        CliError::Msg(s.to_string())
    }
}

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
