//! 版本兼容性工具函数
//!
//! 提供应用版本解析与注册表版本兼容性判断，由 `manager.rs` 和 `registry.rs` 共享使用。

use semver::Version;

use crate::PluginError;

/// 当前应用版本
pub fn current_app_version() -> Result<Version, PluginError> {
    let s = env!("CARGO_PKG_VERSION");
    Version::parse(s).map_err(|e| PluginError::RegistryError(format!("版本解析失败: {}", e)))
}

/// 解析 semver 版本，忽略可能的前缀 `v`
pub fn parse_version(s: &str) -> Result<Version, PluginError> {
    let s = s.strip_prefix('v').unwrap_or(s);
    Version::parse(s).map_err(|e| PluginError::InvalidManifest(format!("版本解析失败: {}", e)))
}

/// 判断注册表版本是否与当前应用版本兼容
pub fn is_version_compatible(version: &crate::RegistryVersion, app_version: &Version) -> bool {
    let min = match parse_version(&version.min_app_version) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let max = match parse_version(&version.max_app_version) {
        Ok(v) => v,
        Err(_) => return false,
    };
    app_version >= &min && app_version <= &max
}
