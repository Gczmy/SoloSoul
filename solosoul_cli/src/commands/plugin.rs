//! 插件系统命令：列表与运行插件。

use std::path::PathBuf;

use color_eyre::Result;

use crate::app::{App, AppPhase};

/// 已安装插件的摘要信息（用于列表展示）。
#[derive(Debug, Clone)]
pub struct PluginSummary {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub tier: String,
}

/// /plugin 或 /plugin_list — 列出所有可用插件。
pub fn list_plugins(app: &mut App) -> Result<()> {
    let market_dir = resolve_plugin_market_dir();

    match load_registry_entries(&market_dir) {
        Ok(entries) => {
            let plugins: Vec<PluginSummary> = entries
                .into_iter()
                .map(|e| PluginSummary {
                    id: e.id,
                    name: e.name,
                    version: e.version,
                    description: e.description.unwrap_or_default(),
                    tier: e.tier.unwrap_or_else(|| "community".to_string()),
                })
                .collect();

            if plugins.is_empty() {
                app.error_message = Some("插件市场中暂无可用插件".to_string());
            } else {
                app.phase = AppPhase::PluginList {
                    plugins,
                    selected: 0,
                };
            }
        }
        Err(e) => {
            app.error_message = Some(format!("加载插件列表失败: {}", e));
        }
    }
    Ok(())
}

/// /plugin_run <plugin_id> — 运行指定插件（预览模式）。
///
/// 完整插件运行时依赖 WebAssembly 沙箱，当前版本仅展示插件清单信息。
pub fn run_plugin(app: &mut App, plugin_id: Option<&str>) -> Result<()> {
    let plugin_id = match plugin_id {
        Some(id) => id.to_string(),
        None => {
            app.error_message = Some("用法: /plugin_run <plugin_id>".to_string());
            return Ok(());
        }
    };

    let market_dir = resolve_plugin_market_dir();
    let manifest_path = market_dir
        .join("plugins")
        .join(&plugin_id)
        .join("manifest.json");

    if manifest_path.exists() {
        match std::fs::read_to_string(&manifest_path) {
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(manifest) => {
                    let name = manifest["name"].as_str().unwrap_or(&plugin_id);
                    let version = manifest["version"].as_str().unwrap_or("unknown");
                    let desc = manifest["description"]
                        .as_str()
                        .unwrap_or("无描述");
                    app.error_message = Some(format!(
                        "插件 {} v{} — {}. (完整运行时将在后续版本中支持)",
                        name, version, desc
                    ));
                }
                Err(e) => {
                    app.error_message =
                        Some(format!("解析插件清单失败: {}", e));
                }
            },
            Err(e) => {
                app.error_message =
                    Some(format!("读取插件清单失败: {}", e));
            }
        }
    } else {
        app.error_message =
            Some(format!("未找到插件 {} 的清单文件", plugin_id));
    }

    Ok(())
}

/// 解析插件市场目录路径。
fn resolve_plugin_market_dir() -> PathBuf {
    // 优先使用 SOLOSOUL_PLUGIN_DIR 环境变量
    if let Ok(dir) = std::env::var("SOLOSOUL_PLUGIN_DIR") {
        let p = PathBuf::from(&dir);
        if p.exists() {
            return p;
        }
    }

    // 尝试从 solosoul-cli 的相对路径解析
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidate = manifest_dir
        .join("..")
        .join("SoloSoul_plugin_market");
    if candidate.exists() {
        return candidate;
    }

    // 最后回退到当前目录
    PathBuf::from("./SoloSoul_plugin_market")
}

/// 从 registry.json 加载插件注册表条目。
fn load_registry_entries(
    market_dir: &PathBuf,
) -> Result<Vec<RegistryEntry>, String> {
    let registry_path = market_dir.join("registry.json");
    if !registry_path.exists() {
        return Err(format!("未找到 registry.json: {}", registry_path.display()));
    }

    let content = std::fs::read_to_string(&registry_path)
        .map_err(|e| format!("读取 registry.json 失败: {}", e))?;

    #[derive(serde::Deserialize)]
    struct RegistryFile {
        plugins: Vec<RegistryEntry>,
    }

    let registry: RegistryFile = serde_json::from_str(&content)
        .map_err(|e| format!("解析 registry.json 失败: {}", e))?;

    Ok(registry.plugins)
}

#[derive(serde::Deserialize, Debug, Clone)]
struct RegistryEntry {
    id: String,
    name: String,
    version: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    tier: Option<String>,
}
