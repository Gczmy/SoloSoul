//! PluginStore - 管理插件的独立安装目录
//!
//! 插件存储在 ~/.solosoul/plugins/ 下，与 Vault 数据完全隔离。
//! 目录权限 0700，每个插件拥有独立的子目录（manifest.json + plugin.wasm）。

use std::path::{Path, PathBuf};

use super::manifest::PluginManifest;

/// 插件存储管理器
pub struct PluginStore {
    base_dir: PathBuf,
}

impl PluginStore {
    /// 创建 PluginStore，自动初始化目录并设置权限
    pub fn new() -> Result<Self, String> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| "No home directory".to_string())?;
        let base_dir = PathBuf::from(home).join(".solosoul").join("plugins");
        Self::with_dir(base_dir)
    }

    /// 使用指定目录创建 PluginStore（用于测试）
    pub fn with_dir(base_dir: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&base_dir)
            .map_err(|e| format!("Failed to create plugin dir: {}", e))?;

        // 设置目录权限 0700
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o700);
            std::fs::set_permissions(&base_dir, perms)
                .map_err(|e| format!("Failed to set permissions: {}", e))?;
        }

        Ok(Self { base_dir })
    }

    /// 获取插件基础目录（供 Dart FFI 调用）
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// 获取指定插件的安装目录
    pub fn plugin_dir(&self, plugin_id: &str) -> PathBuf {
        self.base_dir.join(plugin_id)
    }

    /// 读取插件 wasm 字节码
    pub fn load_wasm(&self, plugin_id: &str) -> Result<Vec<u8>, String> {
        let path = self.plugin_dir(plugin_id).join("plugin.wasm");
        std::fs::read(&path)
            .map_err(|e| format!("Failed to read wasm for {}: {}", plugin_id, e))
    }

    /// 读取插件清单
    pub fn load_manifest(&self, plugin_id: &str) -> Result<PluginManifest, String> {
        let path = self.plugin_dir(plugin_id).join("manifest.json");
        let data = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read manifest for {}: {}", plugin_id, e))?;
        let manifest: PluginManifest = serde_json::from_str(&data)
            .map_err(|e| format!("Failed to parse manifest for {}: {}", plugin_id, e))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// 保存插件到独立目录
    pub fn save_plugin(
        &self,
        plugin_id: &str,
        wasm: &[u8],
        manifest: &PluginManifest,
    ) -> Result<(), String> {
        let dir = self.plugin_dir(plugin_id);
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create plugin dir: {}", e))?;

        std::fs::write(dir.join("plugin.wasm"), wasm)
            .map_err(|e| format!("Failed to write wasm: {}", e))?;

        let manifest_json = serde_json::to_string_pretty(manifest)
            .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
        std::fs::write(dir.join("manifest.json"), manifest_json)
            .map_err(|e| format!("Failed to write manifest: {}", e))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let file_perms = std::fs::Permissions::from_mode(0o600);
            let _ = std::fs::set_permissions(dir.join("plugin.wasm"), file_perms.clone());
            let _ = std::fs::set_permissions(dir.join("manifest.json"), file_perms);
        }

        Ok(())
    }

    /// 删除插件目录
    pub fn remove_plugin(&self, plugin_id: &str) -> Result<(), String> {
        let dir = self.plugin_dir(plugin_id);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)
                .map_err(|e| format!("Failed to remove plugin {}: {}", plugin_id, e))?;
        }
        Ok(())
    }

    /// 列出所有已安装插件（仅返回通过有效性校验的插件）
    pub fn list_installed(&self) -> Result<Vec<String>, String> {
        let mut plugins = Vec::new();
        let entries = std::fs::read_dir(&self.base_dir)
            .map_err(|e| format!("Failed to read plugin dir: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            if !entry.file_type().map_err(|e| e.to_string())?.is_dir() {
                continue;
            }
            if let Some(name) = entry.file_name().to_str() {
                // 排除系统文件/目录，并校验目录下包含合法的 manifest + wasm
                if !name.starts_with('.') && self.is_valid_plugin_dir(name) {
                    plugins.push(name.to_string());
                }
            }
        }
        Ok(plugins)
    }

    /// 校验插件目录是否包含完整的 manifest.json 和 plugin.wasm
    fn is_valid_plugin_dir(&self, plugin_id: &str) -> bool {
        let dir = self.plugin_dir(plugin_id);
        dir.join("manifest.json").is_file() && dir.join("plugin.wasm").is_file()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_manifest(plugin_id: &str) -> PluginManifest {
        PluginManifest {
            plugin_id: plugin_id.to_string(),
            name: "Test".to_string(),
            version: "1.0.0".to_string(),
            plugin_api_version: "1.0".to_string(),
            min_app_version: "1.0.0".to_string(),
            max_app_version: "2.0.0".to_string(),
            description: "Test plugin".to_string(),
            publisher: "Test".to_string(),
            homepage: None,
            signature: None,
            required_fields: vec!["identity.full_name".to_string()],
            optional_fields: vec![],
            network_policy: None,
            data_ttl_seconds: 300,
            require_user_confirmation: true,
            consent_validity_hours: 24,
            i18n: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_plugin_store_lifecycle() {
        let temp = tempfile::tempdir().unwrap();
        let store = PluginStore::with_dir(temp.path().to_path_buf()).unwrap();

        // 保存插件
        let manifest = test_manifest("com.test.plugin");
        store.save_plugin("com.test.plugin", b"fake wasm", &manifest).unwrap();

        // 读取 manifest
        let loaded = store.load_manifest("com.test.plugin").unwrap();
        assert_eq!(loaded.plugin_id, "com.test.plugin");

        // 读取 wasm
        let wasm = store.load_wasm("com.test.plugin").unwrap();
        assert_eq!(wasm, b"fake wasm");

        // 列出已安装
        let list = store.list_installed().unwrap();
        assert_eq!(list, vec!["com.test.plugin"]);

        // 删除插件
        store.remove_plugin("com.test.plugin").unwrap();
        let list = store.list_installed().unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn test_list_installed_filters_invalid() {
        let temp = tempfile::tempdir().unwrap();
        let store = PluginStore::with_dir(temp.path().to_path_buf()).unwrap();

        // 创建完整插件
        let manifest = test_manifest("com.valid.plugin");
        store.save_plugin("com.valid.plugin", b"wasm", &manifest).unwrap();

        // 创建空目录（不应被列出）
        std::fs::create_dir(temp.path().join("empty_dir")).unwrap();

        // 创建只有 manifest 没有 wasm 的目录（不应被列出）
        std::fs::create_dir(temp.path().join("no_wasm")).unwrap();
        std::fs::write(temp.path().join("no_wasm/manifest.json"), "{}").unwrap();

        let list = store.list_installed().unwrap();
        assert_eq!(list, vec!["com.valid.plugin"]);
    }
}
