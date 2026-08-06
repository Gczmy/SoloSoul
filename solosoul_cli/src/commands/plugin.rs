//! 插件系统命令：列表与运行插件。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use color_eyre::Result;
use std::time::Instant;

use crate::app::{App, AppPhase};
use crate::t;

use solosoul_plugin::{PluginEvent, PluginEventSink};

/// 终端插件事件接收器（no-op 实现）。
///
/// 插件运行结果通过 PluginManager::run() 的返回值获取，
/// 此 sink 仅满足 trait 约束，不缓冲事件。
pub struct TerminalPluginSink;

impl PluginEventSink for TerminalPluginSink {
    fn send(&self, _event: PluginEvent) -> Result<(), String> {
        Ok(())
    }
}

/// 以安装插件的摘要信息（用定列表展示）。
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
                app.error_message = Some(t!(app.i18n, "cmd-plugin-market-empty"));
            } else {
                app.phase = AppPhase::PluginList {
                    plugins,
                    selected: 0,
                    filter: String::new(),
                };
            }
        }
        Err(e) => {
            app.error_message = Some(t!(app.i18n, "cmd-plugin-search-failed", err = e));
        }
    }
    Ok(())
}

/// /plugin_run <plugin_id> [key=value ...] — 运行指定插件（后台异步执行）。
///
/// 可选的 key=value 参数将传递给插件作为运行时配置。
/// 插件在后台线程中运行，结果通过 app.error_message 异步展示。
pub fn run_plugin(app: &mut App, plugin_id: Option<&str>, raw_params: &[&str]) -> Result<()> {
    let plugin_id = match plugin_id {
        Some(id) => id.to_string(),
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-plugin-usage-run"));
            return Ok(());
        }
    };

    let account_id = match app.vault_service.get_current_account() {
        Some(id) => id,
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-plugin-need-login"));
            return Ok(());
        }
    };

    let vault = match app.vault_service.get_vault_store() {
        Some(v) => v,
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-plugin-vault-locked"));
            return Ok(());
        }
    };

    let market_dir = resolve_plugin_market_dir();
    let plugin_dir = market_dir.join("plugins").join(&plugin_id);
    if !plugin_dir.exists() {
        app.error_message = Some(t!(app.i18n, "cmd-plugin-not-found", id = plugin_id));
        return Ok(());
    }

    // 查找插件版本
    let version = match load_registry_entries(&market_dir) {
        Ok(entries) => entries
            .iter()
            .find(|e| e.id == plugin_id)
            .map(|e| e.version.clone())
            .unwrap_or_else(|| "latest".to_string()),
        Err(_) => "latest".to_string(),
    };

    // 解析 key=value 运行时参数
    let params: HashMap<String, String> = raw_params
        .iter()
        .filter_map(|p| {
            let mut parts = p.splitn(2, '=');
            let key = parts.next()?.trim().to_string();
            let value = parts.next()?.trim().to_string();
            if key.is_empty() {
                None
            } else {
                Some((key, value))
            }
        })
        .collect();

    // 共享结果容器：工作线程写入，主线程在 handle_tick 中轮询
    let result_holder: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    app.plugin_run_pending = Some(result_holder.clone());
    app.success_message = Some((
        t!(app.i18n, "cmd-plugin-running", id = plugin_id),
        Instant::now(),
    ));

    let plugin_id_clone = plugin_id;
    let market_dir_clone = market_dir;

    std::thread::spawn(move || {
        // R2-V7：运行时初始化失败优雅降级为错误消息（不再 panic）
        let rt = match crate::util::shared_runtime() {
            Ok(rt) => rt,
            Err(e) => {
                if let Ok(mut h) = result_holder.lock() {
                    *h = Some(format!("初始化共享运行时失败: {e}"));
                }
                return;
            }
        };

        let outcome = rt.block_on(async {
            let manager =
                match solosoul_plugin::PluginManager::new_with_resource_dir(&market_dir_clone) {
                    Ok(m) => m,
                    Err(e) => return format!("初始化插件管理器失败: {}", e),
                };

            // 安装插件到本地
            if let Err(e) = manager
                .install_from_registry(&plugin_id_clone, &version)
                .await
            {
                return format!("安装插件 {} 失败: {}", plugin_id_clone, e);
            }

            let sink: Arc<TerminalPluginSink> = Arc::new(TerminalPluginSink);

            match manager
                .run(
                    &plugin_id_clone,
                    params,
                    sink,
                    Some(vault),
                    Some(account_id),
                )
                .await
            {
                Ok(result) => {
                    format!(
                        "Plugin {} completed: exit_code={}, fuel={}",
                        plugin_id_clone, result.exit_code, result.fuel_consumed
                    )
                }
                Err(e) => format!("Plugin {} run failed: {}", plugin_id_clone, e),
            }
        });

        if let Ok(mut h) = result_holder.lock() {
            *h = Some(outcome);
        }
    });

    Ok(())
}

/// /plugin_install <plugin_id> — 从插件市场安装插件。
pub fn install_plugin(app: &mut App, plugin_id: Option<&str>) -> Result<()> {
    let plugin_id = match plugin_id {
        Some(id) => id.to_string(),
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-plugin-usage-install"));
            return Ok(());
        }
    };

    let Some(manager) = create_manager(app) else {
        return Ok(());
    };

    let version = match load_registry_entries(&resolve_plugin_market_dir()) {
        Ok(entries) => entries
            .iter()
            .find(|e| e.id == plugin_id)
            .map(|e| e.version.clone())
            .unwrap_or_else(|| "latest".to_string()),
        Err(_) => "latest".to_string(),
    };

    let rt = crate::util::shared_runtime()?;

    match rt.block_on(manager.install_from_registry(&plugin_id, &version)) {
        Ok(result) => {
            app.success_message = Some((
                t!(
                    app.i18n,
                    "cmd-plugin-installed",
                    id = result.plugin_id,
                    ver = result.version
                ),
                std::time::Instant::now(),
            ));
        }
        Err(e) => {
            app.error_message = Some(t!(
                app.i18n,
                "cmd-plugin-install-failed",
                id = plugin_id,
                err = e
            ));
        }
    }
    Ok(())
}

/// /plugin_update <plugin_id> — 更新已安装插件。
pub fn update_plugin(app: &mut App, plugin_id: Option<&str>) -> Result<()> {
    let plugin_id = match plugin_id {
        Some(id) => id.to_string(),
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-plugin-usage-update"));
            return Ok(());
        }
    };

    let Some(manager) = create_manager(app) else {
        return Ok(());
    };

    let rt = crate::util::shared_runtime()?;

    match rt.block_on(manager.update(&plugin_id)) {
        Ok(result) => {
            app.success_message = Some((
                t!(
                    app.i18n,
                    "cmd-plugin-updated",
                    id = result.plugin_id,
                    ver = result.version
                ),
                std::time::Instant::now(),
            ));
        }
        Err(e) => {
            app.error_message = Some(t!(
                app.i18n,
                "cmd-plugin-update-failed",
                id = plugin_id,
                err = e
            ));
        }
    }
    Ok(())
}

/// /plugin_uninstall <plugin_id> — 卸载插件。
pub fn uninstall_plugin(app: &mut App, plugin_id: Option<&str>) -> Result<()> {
    let plugin_id = match plugin_id {
        Some(id) => id.to_string(),
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-plugin-usage-uninstall"));
            return Ok(());
        }
    };

    let Some(manager) = create_manager(app) else {
        return Ok(());
    };

    match manager.uninstall(&plugin_id) {
        Ok(()) => {
            app.success_message = Some((
                t!(app.i18n, "cmd-plugin-uninstalled", id = plugin_id),
                Instant::now(),
            ));
        }
        Err(e) => {
            app.error_message = Some(t!(
                app.i18n,
                "cmd-plugin-uninstall-failed",
                id = plugin_id,
                err = e
            ));
        }
    }
    Ok(())
}

/// /plugin_sessions — 查看活跃插件会话。
pub fn list_sessions(app: &mut App) -> Result<()> {
    let Some(manager) = create_manager(app) else {
        return Ok(());
    };

    match manager.list_sessions() {
        Ok(sessions) => {
            if sessions.is_empty() {
                app.error_message = Some(t!(app.i18n, "cmd-plugin-no-sessions"));
            } else {
                let lines: Vec<String> = sessions
                    .iter()
                    .map(|s| {
                        format!(
                            "- {} (plugin: {}, created: {})",
                            s.id, s.plugin_id, s.created_at
                        )
                    })
                    .collect();
                app.error_message = Some(
                    t!(
                        app.i18n,
                        "cmd-plugin-sessions-header",
                        count = sessions.len().to_string()
                    ) + "\n"
                        + &lines.join("\n"),
                );
            }
        }
        Err(e) => {
            app.error_message = Some(t!(app.i18n, "cmd-plugin-list-sessions-failed", err = e));
        }
    }
    Ok(())
}

/// /plugin_list_installed — 列出本地已安装插件。
pub fn list_installed_plugins(app: &mut App) -> Result<()> {
    let Some(manager) = create_manager(app) else {
        return Ok(());
    };

    match manager.list_installed() {
        Ok(installed) => {
            if installed.is_empty() {
                app.error_message = Some(t!(app.i18n, "cmd-plugin-none-installed"));
            } else {
                let lines: Vec<String> = installed
                    .iter()
                    .map(|p| format!("- {} v{} ({})", p.name, p.version, p.description))
                    .collect();
                app.error_message = Some(
                    t!(
                        app.i18n,
                        "cmd-plugin-installed-header",
                        count = installed.len().to_string()
                    ) + "\n"
                        + &lines.join("\n"),
                );
            }
        }
        Err(e) => {
            app.error_message = Some(t!(app.i18n, "cmd-plugin-list-installed-failed", err = e));
        }
    }
    Ok(())
}

/// /plugin_audit_log [limit] — 查看插件审计日志。
pub fn audit_log(app: &mut App, limit: Option<&str>) -> Result<()> {
    let Some(manager) = create_manager(app) else {
        return Ok(());
    };

    let limit_num: Option<usize> = match limit.and_then(|s| s.parse().ok()) {
        Some(n) if n > 0 => Some(n),
        Some(_) => {
            app.error_message = Some(t!(app.i18n, "cmd-plugin-limit-must-be-positive"));
            return Ok(());
        }
        None if limit.is_some() => {
            app.error_message = Some(t!(app.i18n, "cmd-plugin-limit-must-be-number"));
            return Ok(());
        }
        None => Some(20), // 默认 20 条
    };

    match manager.audit_log(limit_num) {
        Ok(entries) => {
            if entries.is_empty() {
                app.error_message = Some(t!(app.i18n, "cmd-plugin-no-audit-logs"));
            } else {
                let lines: Vec<String> = entries
                    .iter()
                    .map(|e| {
                        let session = e.session_id.as_deref().unwrap_or("-");
                        format!(
                            "[{}] {} @ {} — {:?}",
                            e.timestamp, e.plugin_id, session, e.action
                        )
                    })
                    .collect();
                app.error_message = Some(
                    t!(
                        app.i18n,
                        "cmd-plugin-audit-header",
                        count = entries.len().to_string()
                    ) + "\n"
                        + &lines.join("\n"),
                );
            }
        }
        Err(e) => {
            app.error_message = Some(t!(app.i18n, "cmd-plugin-audit-failed", err = e));
        }
    }
    Ok(())
}

/// 从插件市场加载指定插件的清单。
pub fn load_manifest(plugin_id: &str) -> Option<solosoul_plugin::PluginManifest> {
    let market_dir = resolve_plugin_market_dir();
    let manifest_path = market_dir
        .join("plugins")
        .join(plugin_id)
        .join("manifest.json");

    let content = std::fs::read_to_string(&manifest_path).ok()?;
    serde_json::from_str::<solosoul_plugin::PluginManifest>(&content).ok()
}

/// 创建 PluginManager 实例（提取公共代码）。/// 创建 PluginManager 实例（提取公共代码）。
/// /plugin_registry_update — 异步刷新远程插件注册表。
pub fn update_registry(app: &mut App) -> Result<()> {
    let Some(manager) = create_manager(app) else {
        return Ok(());
    };

    let rt = crate::util::shared_runtime()?;

    app.success_message = Some((t!(app.i18n, "cmd-plugin-updating-registry"), Instant::now()));

    let result_holder: std::sync::Arc<std::sync::Mutex<Option<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));
    let holder = result_holder.clone();

    std::thread::spawn(move || {
        rt.block_on(async {
            match manager.update_registry().await {
                Ok(()) => {
                    if let Ok(mut h) = holder.lock() {
                        *h = Some("Plugin registry updated.".to_string());
                    }
                }
                Err(e) => {
                    if let Ok(mut h) = holder.lock() {
                        *h = Some(format!("Failed to update registry: {}", e));
                    }
                }
            }
        });
    });

    app.plugin_run_pending = Some(result_holder);
    Ok(())
}

/// /plugin_search <keyword> — 在插件市场中按关键词搜索。
pub fn search_plugins(app: &mut App, keyword: Option<&str>) -> Result<()> {
    let keyword = match keyword {
        Some(k) => k.to_lowercase(),
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-plugin-usage-search"));
            return Ok(());
        }
    };

    let market_dir = resolve_plugin_market_dir();

    match load_registry_entries(&market_dir) {
        Ok(entries) => {
            let matched: Vec<PluginSummary> = entries
                .into_iter()
                .filter(|e| {
                    e.name.to_lowercase().contains(&keyword)
                        || e.description
                            .as_deref()
                            .unwrap_or("")
                            .to_lowercase()
                            .contains(&keyword)
                })
                .map(|e| PluginSummary {
                    id: e.id,
                    name: e.name,
                    version: e.version,
                    description: e.description.unwrap_or_default(),
                    tier: e.tier.unwrap_or_else(|| "community".to_string()),
                })
                .collect();

            if matched.is_empty() {
                app.error_message = Some(t!(
                    app.i18n,
                    "cmd-plugin-search-no-match",
                    keyword = keyword
                ));
            } else {
                app.phase = AppPhase::PluginList {
                    plugins: matched,
                    selected: 0,
                    filter: String::new(),
                };
            }
        }
        Err(e) => {
            app.error_message = Some(t!(app.i18n, "cmd-plugin-search-failed", err = e));
        }
    }
    Ok(())
}

/// 创建 PluginManager 实例（提取公共代码）。
fn create_manager(app: &mut App) -> Option<solosoul_plugin::PluginManager> {
    let market_dir = resolve_plugin_market_dir();
    match solosoul_plugin::PluginManager::new_with_resource_dir(&market_dir) {
        Ok(m) => Some(m),
        Err(e) => {
            app.error_message = Some(t!(app.i18n, "cmd-plugin-init-failed", err = e));
            None
        }
    }
}

/// 解析插件市场目录路径。
fn resolve_plugin_market_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("SOLOSOUL_PLUGIN_DIR") {
        let p = PathBuf::from(&dir);
        if p.exists() {
            return p;
        }
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidate = manifest_dir.join("..").join("SoloSoul_plugin_market");
    if candidate.exists() {
        return candidate;
    }

    PathBuf::from("./SoloSoul_plugin_market")
}

/// 克 registry.json 加载插件注册表条目。
fn load_registry_entries(market_dir: &Path) -> Result<Vec<RegistryEntry>, String> {
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

    let registry: RegistryFile =
        serde_json::from_str(&content).map_err(|e| format!("解析 registry.json 失败: {}", e))?;

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
