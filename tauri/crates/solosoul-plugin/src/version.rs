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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::RegistryVersion;
    use semver::Version;

    fn version(min: &str, max: &str) -> RegistryVersion {
        RegistryVersion {
            sha256: "00".to_string(),
            plugin_api_version: None,
            min_app_version: min.to_string(),
            max_app_version: max.to_string(),
            download_url: None,
            raw_url: None,
            released_at: None,
            changelog: None,
        }
    }

    #[test]
    fn test_is_version_compatible_within_range() {
        let app = Version::parse("2.1.0").unwrap();
        let v = version("1.0.0", "3.0.0");
        assert!(is_version_compatible(&v, &app));
    }

    #[test]
    fn test_is_version_compatible_out_of_range() {
        let app = Version::parse("0.5.0").unwrap();
        let v = version("1.0.0", "3.0.0");
        assert!(!is_version_compatible(&v, &app));
    }

    #[test]
    fn test_parse_version_strips_v_prefix() {
        assert_eq!(
            parse_version("v1.2.3").unwrap(),
            Version::parse("1.2.3").unwrap()
        );
    }
}
