//! 插件市场注册表
//!
//! 解析 `SoloSoul_plugin_market/registry.json`，提供与当前应用版本的兼容性判断。

use super::{MarketPluginInfo, PluginError, PluginManifest, RegistryEntry, RegistryVersion};
use minisign_verify::{PublicKey, Signature};
use semver::Version;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// 默认远程注册表 URL
const DEFAULT_REGISTRY_URL: &str = "https://plugins.solosoul.app/registry.json";

/// 注册表文件顶层结构
#[derive(Debug, Deserialize)]
struct RegistryFile {
    plugins: HashMap<String, RegistryEntry>,
}

/// 插件注册表
pub struct PluginRegistry {
    /// 打包的原始注册表路径（Release 时为只读资源目录）
    bundled_path: PathBuf,
    /// 可写的缓存注册表路径（应用数据目录）
    cache_path: PathBuf,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    /// 创建注册表加载器（开发模式回退到源码路径）
    pub fn new() -> Self {
        let bundled = super::paths::default_market_dir().join("registry.json");
        let cache = super::PluginStore::data_dir()
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
            .join("registry.json");
        Self {
            bundled_path: bundled,
            cache_path: cache,
        }
    }

    /// 使用 Tauri 应用句柄创建注册表加载器（Release 优先读取资源目录）
    pub fn new_with_app_handle(app_handle: &tauri::AppHandle) -> Result<Self, PluginError> {
        let market_dir = super::paths::resolve_market_dir(Some(app_handle))?;
        let cache = super::PluginStore::data_dir()?.join("registry.json");
        Ok(Self {
            bundled_path: market_dir.join("registry.json"),
            cache_path: cache,
        })
    }

    /// 从指定路径加载注册表（测试用）
    pub fn from_path(path: impl AsRef<Path>) -> Self {
        let p = path.as_ref().to_path_buf();
        Self {
            bundled_path: p.clone(),
            cache_path: p,
        }
    }

    /// 获取当前实际使用的注册表路径（优先缓存，其次 bundled）
    fn active_path(&self) -> &PathBuf {
        if self.cache_path.exists() {
            &self.cache_path
        } else {
            &self.bundled_path
        }
    }

    /// 从远程 URL 拉取并更新本地注册表
    ///
    /// 1. 读取环境变量 `SOLOSOUL_REGISTRY_URL`（默认 `DEFAULT_REGISTRY_URL`）
    /// 2. 读取环境变量 `SOLOSOUL_REGISTRY_PUBKEY`（必需）
    /// 3. 下载注册表文件与对应的 `.minisig` 签名
    /// 4. 使用 Minisign 验证签名
    /// 5. 校验 JSON 结构后原子写入本地 `registry.json`
    pub async fn update_from_remote(&self) -> Result<(), PluginError> {
        let url = std::env::var("SOLOSOUL_REGISTRY_URL")
            .unwrap_or_else(|_| DEFAULT_REGISTRY_URL.to_string());
        let pubkey_b64 = match std::env::var("SOLOSOUL_REGISTRY_PUBKEY") {
            Ok(k) => k,
            Err(_) => {
                tracing::warn!(
                    "SOLOSOUL_REGISTRY_PUBKEY 未配置，跳过注册表远程更新，使用本地 bundled 注册表"
                );
                return Ok(());
            }
        };
        let public_key = PublicKey::from_base64(&pubkey_b64)
            .map_err(|e| PluginError::RegistryError(format!("注册表公钥解析失败: {}", e)))?;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| PluginError::RegistryError(format!("HTTP 客户端创建失败: {}", e)))?;

        let registry_bytes = client
            .get(&url)
            .send()
            .await
            .map_err(|e| PluginError::RegistryError(format!("下载注册表失败: {}", e)))?
            .bytes()
            .await
            .map_err(|e| PluginError::RegistryError(format!("读取注册表响应失败: {}", e)))?;

        let sig_url = format!("{}.minisig", url);
        let sig_text = client
            .get(&sig_url)
            .send()
            .await
            .map_err(|e| PluginError::RegistryError(format!("下载注册表签名失败: {}", e)))?
            .text()
            .await
            .map_err(|e| PluginError::RegistryError(format!("读取签名响应失败: {}", e)))?;

        let signature = Signature::decode(&sig_text)
            .map_err(|e| PluginError::RegistryError(format!("签名解码失败: {}", e)))?;

        public_key
            .verify(&registry_bytes, &signature, false)
            .map_err(|e| PluginError::RegistryError(format!("注册表签名验证失败: {}", e)))?;

        // 校验 JSON 结构合法
        let _: RegistryFile = serde_json::from_slice(&registry_bytes)
            .map_err(|e| PluginError::RegistryError(format!("注册表 JSON 非法: {}", e)))?;

        // 原子写入缓存路径（数据目录，可写）
        if let Some(parent) = self.cache_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let tmp_path = self.cache_path.with_extension("tmp");
        std::fs::write(&tmp_path, &registry_bytes)
            .map_err(|e| PluginError::StoreError(format!("写入注册表临时文件失败: {}", e)))?;
        std::fs::rename(&tmp_path, &self.cache_path)
            .map_err(|e| PluginError::StoreError(format!("替换注册表文件失败: {}", e)))?;

        Ok(())
    }

    /// 加载注册表并转换为前端可用的市场插件信息列表
    pub fn load(&self, installed: &[PluginManifest]) -> Result<Vec<MarketPluginInfo>, PluginError> {
        let path = self.active_path();
        let content = std::fs::read_to_string(path)
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
        let path = self.active_path();
        let content = std::fs::read_to_string(path)
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
