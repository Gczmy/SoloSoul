//! 插件本地存储
//!
//! 插件数据保存在 `~/.solosoul/plugins/{plugin_id}/`，目录权限 `0700`，文件权限 `0600`。

use super::{PluginError, PluginManifest};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// Wasm 文件最大 10 MiB
const MAX_WASM_SIZE: usize = 10 * 1024 * 1024;

/// 插件 ID 允许字符集，防止通过 ID 构造路径遍历。
fn validate_plugin_id(id: &str) -> Result<(), PluginError> {
    if id.is_empty()
        || id.len() > 64
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err(PluginError::StoreError(format!(
            "Invalid plugin id: {}",
            id
        )));
    }
    Ok(())
}

/// 插件本地存储
pub struct PluginStore {
    base_dir: PathBuf,
}

impl PluginStore {
    /// 创建插件存储，同时确保基础目录存在
    pub fn new() -> Result<Self, PluginError> {
        let base_dir = Self::plugins_dir()?;
        Self::ensure_dir(&base_dir)?;
        Ok(Self { base_dir })
    }

    /// 获取当前用户的插件基础目录
    fn plugins_dir() -> Result<PathBuf, PluginError> {
        Ok(Self::data_dir()?.join("plugins"))
    }

    /// 获取 SoloSoul 数据根目录（~/.solosoul）
    pub fn data_dir() -> Result<PathBuf, PluginError> {
        let home = dirs::home_dir()
            .ok_or_else(|| PluginError::StoreError("无法获取主目录".to_string()))?;
        Ok(home.join(".solosoul"))
    }

    /// 确保目录存在并设置权限
    fn ensure_dir(path: &Path) -> Result<(), PluginError> {
        if !path.exists() {
            fs::create_dir_all(path)?;
            #[cfg(unix)]
            fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
        }
        Ok(())
    }

    fn plugin_dir(&self, plugin_id: &str) -> Result<PathBuf, PluginError> {
        validate_plugin_id(plugin_id)?;
        Ok(self.base_dir.join(plugin_id))
    }

    fn manifest_path(&self, plugin_id: &str) -> Result<PathBuf, PluginError> {
        Ok(self.plugin_dir(plugin_id)?.join("manifest.json"))
    }

    fn wasm_path(&self, plugin_id: &str) -> Result<PathBuf, PluginError> {
        Ok(self.plugin_dir(plugin_id)?.join("plugin.wasm"))
    }

    /// 保存插件 manifest 与 wasm 到本地
    pub fn save_plugin(
        &self,
        manifest: &PluginManifest,
        wasm_bytes: &[u8],
    ) -> Result<(), PluginError> {
        if wasm_bytes.len() > MAX_WASM_SIZE {
            return Err(PluginError::WasmTooLarge(wasm_bytes.len()));
        }

        let dir = self.plugin_dir(&manifest.id)?;
        Self::ensure_dir(&dir)?;

        let manifest_json = serde_json::to_string_pretty(manifest)?;
        let manifest_path = dir.join("manifest.json");
        let mut file = fs::File::create(&manifest_path)?;
        file.write_all(manifest_json.as_bytes())?;
        #[cfg(unix)]
        file.set_permissions(fs::Permissions::from_mode(0o600))?;

        let wasm_path = dir.join("plugin.wasm");
        let mut wasm_file = fs::File::create(&wasm_path)?;
        wasm_file.write_all(wasm_bytes)?;
        #[cfg(unix)]
        wasm_file.set_permissions(fs::Permissions::from_mode(0o600))?;

        Ok(())
    }

    /// 加载插件 manifest
    pub fn load_manifest(&self, plugin_id: &str) -> Result<PluginManifest, PluginError> {
        let path = self.manifest_path(plugin_id)?;
        if !path.exists() {
            return Err(PluginError::NotFound(plugin_id.to_string()));
        }
        let content = fs::read_to_string(path)?;
        let manifest: PluginManifest = serde_json::from_str(&content)?;
        Ok(manifest)
    }

    /// 加载插件 wasm 并校验 SHA-256（manifest 中提供时）
    pub fn load_wasm(&self, plugin_id: &str) -> Result<Vec<u8>, PluginError> {
        let manifest = self.load_manifest(plugin_id)?;
        let path = self.wasm_path(plugin_id)?;
        if !path.exists() {
            return Err(PluginError::NotFound(plugin_id.to_string()));
        }
        let bytes = fs::read(path)?;
        if bytes.len() > MAX_WASM_SIZE {
            return Err(PluginError::WasmTooLarge(bytes.len()));
        }
        if let Some(expected_hash) = manifest.wasm_hash_sha256 {
            let actual = compute_sha256(&bytes);
            if actual != expected_hash {
                return Err(PluginError::ChecksumMismatch);
            }
        }
        Ok(bytes)
    }

    /// 删除已安装的插件
    pub fn delete_plugin(&self, plugin_id: &str) -> Result<(), PluginError> {
        let dir = self.plugin_dir(plugin_id)?;
        if dir.exists() {
            fs::remove_dir_all(dir)?;
        }
        Ok(())
    }

    /// 列出所有已安装插件的 manifest
    pub fn installed_manifests(&self) -> Result<Vec<PluginManifest>, PluginError> {
        let mut manifests = Vec::new();
        if !self.base_dir.exists() {
            return Ok(manifests);
        }
        for entry in fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let id = entry.file_name().to_string_lossy().to_string();
                if let Ok(manifest) = self.load_manifest(&id) {
                    manifests.push(manifest);
                }
            }
        }
        Ok(manifests)
    }
}

/// 计算 SHA-256 十六进制字符串
pub fn compute_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_sha256_known_value() {
        let hash = compute_sha256(b"hello");
        assert_eq!(
            hash,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }
}
