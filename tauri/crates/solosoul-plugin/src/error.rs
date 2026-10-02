//! 插件系统错误类型
//!
//! 所有插件相关错误统一收敛到 `PluginError`，便于前端展示与日志记录。

use thiserror::Error;

/// 插件错误类型
#[derive(Debug, Error)]
pub enum PluginError {
    /// 插件未找到
    #[error("插件未找到: {0}")]
    NotFound(String),

    /// manifest 解析失败或字段缺失
    #[error("无效的插件 manifest: {0}")]
    InvalidManifest(String),

    /// Wasm 文件超过大小限制
    #[error("Wasm 文件过大: {0} 字节")]
    WasmTooLarge(usize),

    /// 校验和不匹配
    #[error("Wasm SHA-256 校验和不匹配")]
    ChecksumMismatch,

    /// 与当前应用版本不兼容
    #[error("插件版本不兼容: {0}")]
    IncompatibleVersion(String),

    /// Wasm 执行失败
    #[error("插件执行失败: {0}")]
    ExecutionFailed(String),

    /// 用户拒绝授权
    #[error("用户拒绝授权")]
    ConsentDenied,

    /// 非法字段
    #[error("非法字段: {0}")]
    InvalidField(String),

    /// 非法参数
    #[error("非法参数: {0}")]
    InvalidArgument(String),

    /// 频率超限
    #[error("频率超限")]
    RateLimited,

    /// 插件存储错误
    #[error("插件存储错误: {0}")]
    StoreError(String),

    /// 注册表错误
    #[error("插件注册表错误: {0}")]
    RegistryError(String),

    /// 网络错误（远程下载失败）
    #[error("网络错误: {0}")]
    NetworkError(String),
    /// 与原 ExecutionFailed 的 Display 相同，提供新 Host 可用的固定类别。
    #[error("插件执行失败: {0}")]
    SessionExpired(String),
    #[error("插件执行失败: {0}")]
    VaultLocked(String),
    #[error("插件执行失败: {0}")]
    TaskUnconfirmed(String),
}

impl From<std::io::Error> for PluginError {
    fn from(e: std::io::Error) -> Self {
        PluginError::StoreError(e.to_string())
    }
}

impl From<serde_json::Error> for PluginError {
    fn from(e: serde_json::Error) -> Self {
        PluginError::InvalidManifest(e.to_string())
    }
}

impl From<wasmtime::Error> for PluginError {
    fn from(e: wasmtime::Error) -> Self {
        PluginError::ExecutionFailed(e.to_string())
    }
}

impl From<hex::FromHexError> for PluginError {
    fn from(_: hex::FromHexError) -> Self {
        PluginError::InvalidManifest("非法的十六进制哈希".to_string())
    }
}

impl PluginError {
    /// 固定机器类别；不含字段、路径、密钥或运行时正文。
    pub fn safe_code(&self) -> &'static str {
        match self {
            Self::NotFound(..) => "PLUGIN_NOT_FOUND",
            Self::InvalidManifest(..) => "PLUGIN_MANIFEST_INVALID",
            Self::WasmTooLarge(..) => "PLUGIN_WASM_TOO_LARGE",
            Self::ChecksumMismatch => "PLUGIN_CHECKSUM_MISMATCH",
            Self::IncompatibleVersion(..) => "PLUGIN_VERSION_INCOMPATIBLE",
            Self::ExecutionFailed(..) => "PLUGIN_EXECUTION_FAILED",
            Self::ConsentDenied => "PLUGIN_CONSENT_DENIED",
            Self::InvalidField(..) => "PLUGIN_INVALID_FIELD",
            Self::InvalidArgument(..) => "PLUGIN_INVALID_ARGUMENT",
            Self::RateLimited => "PLUGIN_RATE_LIMITED",
            Self::StoreError(cause)
                if matches!(
                    cause.as_str(),
                    "IMPORT_DIRECTORY_BUSY" | "IMPORT_OPERATIONS_ACTIVE"
                ) =>
            {
                "VAULT_BUSY"
            }
            Self::StoreError(..) => "PLUGIN_STORE_FAILED",
            Self::RegistryError(..) => "PLUGIN_REGISTRY_FAILED",
            Self::NetworkError(..) => "PLUGIN_NETWORK_FAILED",
            Self::SessionExpired(..) => "PLUGIN_SESSION_EXPIRED",
            Self::TaskUnconfirmed(..) => "PLUGIN_TASK_UNCONFIRMED",
            Self::VaultLocked(..) => "VAULT_LOCKED",
        }
    }
}
