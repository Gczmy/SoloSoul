//! 命令路由与执行器。

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
