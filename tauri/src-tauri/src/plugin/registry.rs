//! 插件市场注册表
//!
//! 解析 `SoloSoul_plugin_market/registry.json`，提供与当前应用版本的兼容性判断。

use super::{MarketPluginInfo, PluginError, PluginManifest, RegistryEntry, RegistryVersion};
use semver::Version;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 注册表文件顶层结构
#[derive(Debug, Deserialize)]
struct RegistryFile {
    plugins: HashMap<String, RegistryEntry>,
}

/// 插件注册表
pub struct PluginRegistry {
    path: PathBuf,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    /// 创建注册表加载器（开发模式回退到源码路径）
    pub fn new() -> Self {
        let path = super::paths::dev_market_dir().join("registry.json");
        Self { path }
    }

    /// 使用 Tauri 应用句柄创建注册表加载器（Release 优先读取资源目录）
    pub fn new_with_app_handle(app_handle: &tauri::AppHandle) -> Result<Self, PluginError> {
        let market_dir = super::paths::resolve_market_dir(Some(app_handle))?;
        Ok(Self {
            path: market_dir.join("registry.json"),
        })
    }

    /// 从指定路径加载注册表
    pub fn from_path(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    /// 加载注册表并转换为前端可用的市场插件信息列表
    pub fn load(&self, installed: &[PluginManifest]) -> Result<Vec<MarketPluginInfo>, PluginError> {
        let content = std::fs::read_to_string(&self.path)
            .map_err(|e| PluginError::RegistryError(format!("读取注册表失败: {}", e)))?;
        let file: RegistryFile = serde_json::from_str(&content)?;
        let app_version = current_app_version()?;

        let installed_map: HashMap<String, PluginManifest> = installed
            .iter()
            .map(|m| (m.id.clone(), m.clone()))
            .collect();

        let mut infos = Vec::new();
        for (plugin_id, entry) in file.plugins {
            let installed_version = installed_map.get(&plugin_id).map(|m| m.version.clone());
            let latest = entry.latest_version.clone();
            let has_update = match (&installed_version, &latest) {
                (Some(inst), Some(latest)) => {
                    parse_version(latest).is_ok_and(|l| parse_version(inst).is_ok_and(|i| l > i))
                }
                _ => false,
            };
            let is_compatible = latest
                .as_ref()
                .and_then(|v| entry.versions.get(v))
                .map(|ver| is_version_compatible(ver, &app_version))
                .unwrap_or(false);

            infos.push(MarketPluginInfo {
                plugin_id,
                installed_version,
                has_update,
                is_compatible,
                tier: entry.tier,
                category: entry.category.clone(),
                registry_entry: entry,
            });
        }
        Ok(infos)
    }

    /// 获取某个插件的注册表条目
    pub fn get_entry(&self, plugin_id: &str) -> Result<RegistryEntry, PluginError> {
        let content = std::fs::read_to_string(&self.path)
            .map_err(|e| PluginError::RegistryError(format!("读取注册表失败: {}", e)))?;
        let file: RegistryFile = serde_json::from_str(&content)?;
        file.plugins
            .get(plugin_id)
            .cloned()
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))
    }
}

/// 当前应用版本
fn current_app_version() -> Result<Version, PluginError> {
    parse_version(env!("CARGO_PKG_VERSION"))
        .map_err(|e| PluginError::RegistryError(format!("应用版本解析失败: {}", e)))
}

/// 解析 semver 版本，忽略可能的前缀 `v`
fn parse_version(s: &str) -> Result<Version, PluginError> {
    let s = s.strip_prefix('v').unwrap_or(s);
    Version::parse(s).map_err(|e| PluginError::InvalidManifest(format!("版本解析失败: {}", e)))
}

/// 判断注册表版本是否与当前应用版本兼容
fn is_version_compatible(version: &RegistryVersion, app_version: &Version) -> bool {
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
