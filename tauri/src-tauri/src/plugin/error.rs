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

    /// 网络错误
    #[error("网络错误: {0}")]
    NetworkError(String),
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
