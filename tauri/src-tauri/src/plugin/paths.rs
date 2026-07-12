//! 插件市场目录定位
//!
//! Release 构建时插件市场作为 Tauri 资源打包；开发时回退到源码相对路径。

use super::PluginError;
use std::path::PathBuf;
use tauri::Manager;

/// 解析插件市场根目录（包含 `registry.json` 与 `plugins/`）
///
/// - 优先使用 Tauri 资源目录中的 `SoloSoul_plugin_market`
/// - 开发模式（`debug_assertions`）下若不存在，回退到 `CARGO_MANIFEST_DIR` 相对路径
pub fn resolve_market_dir(app_handle: Option<&tauri::AppHandle>) -> Result<PathBuf, PluginError> {
    if let Some(app) = app_handle {
        // Android 上 Tauri 的 resource_dir 返回 asset:// URL，无法被 std::fs 直接读取。
        // MainActivity 已将资源复制到 files/resources/，优先使用该私有目录。
        #[cfg(target_os = "android")]
        let resource_dir = app
            .path()
            .resolve("resources", tauri::path::BaseDirectory::Data)
            .ok();
        #[cfg(not(target_os = "android"))]
        let resource_dir = app.path().resource_dir().ok();

        if let Some(resource_dir) = resource_dir {
            let bundled = resource_dir.join("SoloSoul_plugin_market");
            if bundled.join("registry.json").exists() || bundled.join("plugins").exists() {
                return Ok(bundled);
            }
        }
    }

    #[cfg(debug_assertions)]
    {
        let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("SoloSoul_plugin_market");
        if dev.join("registry.json").exists() || dev.join("plugins").exists() {
            return Ok(dev);
        }
    }

    Err(PluginError::RegistryError(
        "无法定位插件市场目录".to_string(),
    ))
}

/// 默认市场目录（用于测试与无 app_handle 的场景）
#[cfg(debug_assertions)]
pub fn default_market_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("SoloSoul_plugin_market")
}

#[cfg(not(debug_assertions))]
pub fn default_market_dir() -> PathBuf {
    // Release builds should always provide an AppHandle and use the bundled
    // resource directory; this fallback exists only to satisfy the type system.
    PathBuf::from(".")
}
