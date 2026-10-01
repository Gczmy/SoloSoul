//! 插件系统命令：列表与运行插件。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use color_eyre::Result;
use std::time::Instant;

use crate::app::{App, AppPhase, PluginRunMessage};
use crate::t;

use solosoul_plugin::{PluginEvent, PluginEventSink};

pub(crate) mod install;
pub(crate) use install::PluginInstallTask;

/// P035：插件 ID 白名单字符校验。
/// 插件 ID 会直接拼接入本地路径（market_dir/plugins/<id>），必须拒绝路径分隔符与
/// `.`/`..`，杜绝 `../`、绝对路径等路径逃逸。允许 `[a-zA-Z0-9_.-]`。
fn is_valid_plugin_id(id: &str) -> bool {
    !id.is_empty()
        && id != "."
        && id != ".."
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

/// 非法插件 ID 时置错误消息并返回 true（调用方应立即 return Ok(()))。
fn reject_invalid_plugin_id(app: &mut App, id: &str) -> bool {
    if is_valid_plugin_id(id) {
        return false;
    }
    app.error_message = Some(t!(app.i18n, "cmd-plugin-invalid-id", id = id));
    true
}

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
    pub installed_version: Option<String>,
}

/// /plugin 或 /plugin_list — 列出所有可用插件。
pub fn list_plugins(app: &mut App) -> Result<()> {
    if let Some(plugins) = market_summaries(app) {
        app.phase = AppPhase::PluginList {
            plugins,
            selected: 0,
            filter: String::new(),
            installed_only: false,
        };
    }
    Ok(())
}

fn market_summaries(app: &mut App) -> Option<Vec<PluginSummary>> {
    let manager = create_manager(app)?;
    match manager.list_all(None) {
        Ok(entries) => {
            let mut plugins: Vec<_> = entries
                .into_iter()
                .map(|entry| {
                    let installed_version = entry.installed_version.clone();
                    let entry = RegistryEntry::from(entry);
                    PluginSummary {
                        id: entry.id,
                        name: entry.name,
                        version: entry.version,
                        description: entry.description.unwrap_or_default(),
                        tier: entry.tier.unwrap_or_default(),
                        installed_version,
                    }
                })
                .collect();
            plugins.sort_by(|a, b| a.id.cmp(&b.id));
            Some(plugins)
        }
        Err(error) => {
            app.error_message = Some(t!(
                app.i18n,
                "cmd-plugin-list-installed-failed",
                err = error
            ));
            None
        }
    }
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
    if reject_invalid_plugin_id(app, &plugin_id) {
        return Ok(());
    }
    app.drain_task_events(32)?;
    if app.plugin_installs.contains_key(&plugin_id) {
        app.info_message = Some(t!(app.i18n, "cmd-plugin-task-active", id = &plugin_id));
        return Ok(());
    }

    let account_id = match app.vault_service.get_current_account() {
        Some(id) => id,
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-plugin-need-login"));
            return Ok(());
        }
    };

    let native_activity = match solosoul_core::import_activity::begin_owned_root_activity(
        app.vault_service.root_owner(),
    ) {
        Ok(activity) => activity,
        Err(error) => {
            app.error_message = Some(error);
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
    // P001: 附件静态加密密钥——插件复制附件到工作区前解密。
    let attachment_key: Option<[u8; 32]> = app
        .vault_service
        .attachment_encryption_key()
        .ok()
        .and_then(|k| k.as_slice().try_into().ok());

    let (market_dir, data_dir) = match manager_dirs(app) {
        Ok(dirs) => dirs,
        Err(error) => {
            app.error_message = Some(t!(app.i18n, "cmd-plugin-init-failed", err = error));
            return Ok(());
        }
    };
    let (data_dir, global_activity) = match plugin_root_activity(app, &data_dir) {
        Ok(owned) => owned,
        Err(error) => {
            app.error_message = Some(error);
            return Ok(());
        }
    };
    let plugin_dir = market_dir.join("plugins").join(&plugin_id);
    if !plugin_dir.exists() {
        app.error_message = Some(t!(app.i18n, "cmd-plugin-not-found", id = plugin_id));
        return Ok(());
    }

    // 查找插件版本
    let version = match load_registry_entries(&market_dir, &data_dir).and_then(|entries| {
        entries
            .into_iter()
            .find(|entry| entry.id == plugin_id)
            .map(|entry| entry.version)
            .filter(|version| !version.is_empty())
            .ok_or_else(|| "插件注册表缺少具体版本".to_string())
    }) {
        Ok(version) => version,
        Err(error) => {
            app.error_message = Some(t!(
                app.i18n,
                "cmd-plugin-run-failed",
                id = plugin_id,
                err = error
            ));
            return Ok(());
        }
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
    // (bool, String) = (是否错误, 消息)——成功/信息性结果走 info_message，失败走 error_message
    let result_holder: Arc<Mutex<Option<PluginRunMessage>>> = Arc::new(Mutex::new(None));
    app.plugin_run_pending = Some(result_holder.clone());
    app.success_message = Some((
        t!(app.i18n, "cmd-plugin-running", id = plugin_id),
        Instant::now(),
    ));

    let plugin_id_clone = plugin_id;
    let market_dir_clone = market_dir;

    std::thread::spawn(move || {
        // 两根可能不同；实际线程保留真正 Vault 与插件全局目录的准入。
        let _native_activity = native_activity;
        let _global_activity = global_activity;
        // R2-V7：运行时初始化失败优雅降级为错误消息（不再 panic）
        let rt = match crate::util::shared_runtime() {
            Ok(rt) => rt,
            Err(e) => {
                if let Ok(mut h) = result_holder.lock() {
                    *h = Some((true, format!("初始化共享运行时失败: {e}")));
                }
                return;
            }
        };

        let outcome = rt.block_on(async {
            let manager = match solosoul_plugin::PluginManager::new_with_dirs_owned(
                market_dir_clone,
                data_dir,
                _global_activity.root_owner(),
            ) {
                Ok(m) => m,
                Err(e) => return (true, format!("初始化插件管理器失败: {}", e)),
            };

            // 安装插件到本地
            if let Err(e) = manager
                .install_from_registry(&plugin_id_clone, &version)
                .await
            {
                return (true, format!("安装插件 {} 失败: {}", plugin_id_clone, e));
            }

            let sink: Arc<TerminalPluginSink> = Arc::new(TerminalPluginSink);

            match manager
                .run(
                    &plugin_id_clone,
                    params,
                    sink,
                    Some(vault),
                    Some(account_id),
                    attachment_key,
                )
                .await
            {
                Ok(result) => (
                    false,
                    format!(
                        "Plugin {} completed: exit_code={}, fuel={}",
                        plugin_id_clone, result.exit_code, result.fuel_consumed
                    ),
                ),
                Err(e) => (
                    true,
                    format!("Plugin {} run failed: {}", plugin_id_clone, e),
                ),
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
    install::start(app, plugin_id, false)
}

/// /plugin_update <plugin_id> — 在原会话后台更新插件。
pub fn update_plugin(app: &mut App, plugin_id: Option<&str>) -> Result<()> {
    install::start(app, plugin_id, true)
}

/// /plugin_cancel <plugin_id> — 请求取消，实际回收前保留任务占位。
pub fn cancel_plugin(app: &mut App, plugin_id: Option<&str>) -> Result<()> {
    install::cancel(app, plugin_id)
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
    if reject_invalid_plugin_id(app, &plugin_id) {
        return Ok(());
    }

    app.drain_task_events(32)?;
    if app.plugin_installs.contains_key(&plugin_id) {
        app.info_message = Some(t!(app.i18n, "cmd-plugin-task-active", id = &plugin_id));
        return Ok(());
    }
    let Some(manager) = create_manager(app) else {
        return Ok(());
    };

    match manager.uninstall(&plugin_id) {
        Ok(()) => {
            install::remove_from_list(&mut app.phase, &plugin_id);
            if let Some(page) = app.previous_phase.as_mut() {
                install::remove_from_list(page, &plugin_id);
            }
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
                app.info_message = Some(t!(app.i18n, "cmd-plugin-no-sessions"));
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
                app.info_message = Some(
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
            let mut plugins: Vec<_> = installed
                .into_iter()
                .map(|p| PluginSummary {
                    id: p.id,
                    name: p.name,
                    installed_version: Some(p.version.clone()),
                    version: p.version,
                    description: p.description,
                    tier: format!("{:?}", p.tier).to_lowercase(),
                })
                .collect();
            plugins.sort_by(|a, b| a.id.cmp(&b.id));
            app.phase = AppPhase::PluginList {
                plugins,
                selected: 0,
                filter: String::new(),
                installed_only: true,
            };
        }
        Err(e) => {
            app.error_message = Some(t!(app.i18n, "cmd-plugin-list-installed-failed", err = e))
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
                app.info_message = Some(t!(app.i18n, "cmd-plugin-no-audit-logs"));
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
                app.info_message = Some(
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
    // P035：非法 ID 直接视为未找到，杜绝路径逃逸读取任意 manifest.json
    if !is_valid_plugin_id(plugin_id) {
        return None;
    }
    let market_dir = resolve_plugin_market_dir();
    let manifest_path = market_dir
        .join("plugins")
        .join(plugin_id)
        .join("manifest.json");

    let content = std::fs::read_to_string(&manifest_path).ok()?;
    serde_json::from_str::<solosoul_plugin::PluginManifest>(&content).ok()
}

/// /plugin_registry_update — 异步刷新远程插件注册表。
pub fn update_registry(app: &mut App) -> Result<()> {
    let Some(manager) = create_manager(app) else {
        return Ok(());
    };

    let rt = crate::util::shared_runtime()?;

    app.success_message = Some((t!(app.i18n, "cmd-plugin-updating-registry"), Instant::now()));

    let result_holder: std::sync::Arc<std::sync::Mutex<Option<PluginRunMessage>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));
    let holder = result_holder.clone();

    std::thread::spawn(move || {
        rt.block_on(async {
            match manager.update_registry().await {
                Ok(()) => {
                    if let Ok(mut h) = holder.lock() {
                        *h = Some((false, "Plugin registry updated.".to_string()));
                    }
                }
                Err(e) => {
                    if let Ok(mut h) = holder.lock() {
                        *h = Some((true, format!("Failed to update registry: {}", e)));
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
    let Some(keyword) = keyword else {
        app.error_message = Some(t!(app.i18n, "cmd-plugin-usage-search"));
        return Ok(());
    };
    let keyword = keyword.to_lowercase();
    if let Some(entries) = market_summaries(app) {
        let plugins: Vec<_> = entries
            .into_iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&keyword)
                    || p.description.to_lowercase().contains(&keyword)
            })
            .collect();
        if plugins.is_empty() {
            app.error_message = Some(t!(
                app.i18n,
                "cmd-plugin-search-no-match",
                keyword = keyword
            ));
        } else {
            app.phase = AppPhase::PluginList {
                plugins,
                selected: 0,
                filter: String::new(),
                installed_only: false,
            };
        }
    }
    Ok(())
}

/// 创建 PluginManager 实例（提取公共代码）。
fn manager_dirs(_app: &App) -> Result<(PathBuf, PathBuf), solosoul_plugin::PluginError> {
    #[cfg(test)]
    if let Some(dirs) = &_app.plugin_test_dirs {
        return Ok(dirs.clone());
    }
    // 插件仍使用既有全局目录，不能悄悄改为账户 --data-dir。
    Ok((
        resolve_plugin_market_dir(),
        solosoul_plugin::PluginStore::data_dir()?,
    ))
}

/// 构造即写 data/plugins，所有调用者都必须先取得实际全局根所有权。
struct OwnedPluginManager {
    manager: solosoul_plugin::PluginManager,
    _activity: solosoul_core::import_activity::RootActivityGuard,
}
impl std::ops::Deref for OwnedPluginManager {
    type Target = solosoul_plugin::PluginManager;
    fn deref(&self) -> &Self::Target {
        &self.manager
    }
}

fn plugin_data_owner(
    app: &App,
    data: &Path,
) -> Result<Arc<solosoul_vault::root_owner::VaultRootOwner>, String> {
    let canonical = std::fs::canonicalize(data).ok();
    let current = app.vault_service.root_owner();
    if canonical.as_deref() == Some(current.root()) {
        return Ok(current);
    }
    let mut cached = app
        .plugin_root_owner
        .lock()
        .map_err(|_| "PLUGIN_DATA_OWNER_POISONED".to_string())?;
    if let Some(owner) = cached.as_ref() {
        if canonical.as_deref() != Some(owner.root()) {
            return Err("PLUGIN_DATA_DIRECTORY_CHANGED".to_string());
        }
        return Ok(Arc::clone(owner));
    }
    // 不同根必须真正 acquire，失败原样返回；不存在默认根写入 fallback。
    let owner = solosoul_vault::root_owner::VaultRootOwner::acquire(data)?;
    *cached = Some(Arc::clone(&owner));
    Ok(owner)
}

fn plugin_root_activity(
    app: &App,
    data: &Path,
) -> Result<(PathBuf, solosoul_core::import_activity::RootActivityGuard), String> {
    let owner = plugin_data_owner(app, data)?;
    let canonical = owner.root().to_path_buf();
    let activity = solosoul_core::import_activity::begin_owned_root_activity(owner)?;
    Ok((canonical, activity))
}

fn create_manager(app: &mut App) -> Option<OwnedPluginManager> {
    let result = (|| -> Result<OwnedPluginManager, String> {
        let (market, data) = manager_dirs(app).map_err(|error| error.to_string())?;
        let (data, activity) = plugin_root_activity(app, &data)?;
        let manager = solosoul_plugin::PluginManager::new_with_dirs_owned(
            market,
            data,
            activity.root_owner(),
        )
        .map_err(|error| error.to_string())?;
        Ok(OwnedPluginManager {
            manager,
            _activity: activity,
        })
    })();
    match result {
        Ok(manager) => Some(manager),
        Err(error) => {
            app.error_message = Some(t!(app.i18n, "cmd-plugin-init-failed", err = error));
            None
        }
    }
}

/// 成功取得实际存储所有权后才可使用原市场备用入口；失锁必须明确报错。
pub(crate) fn load_manifest_for_app(
    app: &App,
    id: &str,
) -> Result<Option<solosoul_plugin::PluginManifest>, String> {
    if !is_valid_plugin_id(id) {
        return Ok(None);
    }
    let (_, data) = manager_dirs(app).map_err(|error| error.to_string())?;
    let (data, _activity) = plugin_root_activity(app, &data)?;
    let store = solosoul_plugin::PluginStore::new_with_data_dir_owned(data, _activity.root_owner())
        .map_err(|error| error.to_string())?;
    Ok(store.load_manifest(id).ok().or_else(|| load_manifest(id)))
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

/// 复用共享映射式注册表，不维护 CLI 私有 JSON schema。
fn load_registry_entries(market_dir: &Path, data_dir: &Path) -> Result<Vec<RegistryEntry>, String> {
    solosoul_plugin::PluginRegistry::new_with_dirs(market_dir.to_owned(), data_dir.to_owned())
        .load(&[])
        .map(|entries| entries.into_iter().map(RegistryEntry::from).collect())
        .map_err(|error| error.to_string())
}

/// 仅为 CLI 展示的投影；反序列化由共享 PluginRegistry 完成。
#[derive(Debug, Clone)]
struct RegistryEntry {
    id: String,
    name: String,
    version: String,
    description: Option<String>,
    tier: Option<String>,
}

impl From<solosoul_plugin::MarketPluginInfo> for RegistryEntry {
    fn from(info: solosoul_plugin::MarketPluginInfo) -> Self {
        Self {
            id: info.plugin_id,
            name: info.registry_entry.name,
            version: info.registry_entry.latest_version.unwrap_or_default(),
            description: Some(info.registry_entry.description),
            tier: Some(format!("{:?}", info.tier).to_lowercase()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// P035：插件 ID 白名单——合法 ID 放行，路径逃逸/分隔符/空 ID 拒绝。
    #[test]
    fn test_is_valid_plugin_id() {
        for valid in [
            "hello",
            "my-plugin",
            "my_plugin",
            "a.b-c_d",
            "abc123",
            "A-B_C.1",
            "plugin.v1",
        ] {
            assert!(is_valid_plugin_id(valid), "应为合法 ID: {:?}", valid);
        }
        for invalid in [
            "",
            ".",
            "..",
            "../evil",
            "a/b",
            "a\\b",
            "a b",
            "https://x",
            "/etc/passwd",
            "..\\..\\x",
            "plugin#1",
        ] {
            assert!(!is_valid_plugin_id(invalid), "应为非法 ID: {:?}", invalid);
        }
    }

    /// P035：load_manifest 对非法 ID 直接返回 None，不拼路径。
    #[test]
    fn test_load_manifest_rejects_invalid_id() {
        assert!(load_manifest("../evil").is_none());
        assert!(load_manifest("a/b").is_none());
        assert!(load_manifest("..").is_none());
    }
}

#[cfg(test)]
#[path = "plugin/rf214_tests.rs"]
mod rf214_tests;

#[cfg(test)]
#[path = "plugin/rf905_tests.rs"]
mod rf905_tests;
