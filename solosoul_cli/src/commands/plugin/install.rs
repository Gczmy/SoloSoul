//! RF214：插件安装由任务所有者管理，所有可见状态仅由原会话事件回填。
use super::{manager_dirs, reject_invalid_plugin_id, PluginSummary};
use crate::app::{App, AppPhase};
use crate::i18n::I18n;
use crate::t;
use crate::tasks::{TaskId, TaskOutput};
use color_eyre::Result;
use solosoul_plugin::{PluginInstallPhase, PluginInstallProgress};
use std::time::Instant;

#[derive(Debug, Clone)]
pub(crate) struct PluginInstallTask {
    pub task_id: TaskId,
    pub cancelling: bool,
    pub updated: bool,
    pub progress: PluginInstallProgress,
}

pub(super) fn start(app: &mut App, id: Option<&str>, updated: bool) -> Result<()> {
    let Some(id) = id else {
        app.error_message = Some(app.i18n.t(if updated {
            "cmd-plugin-usage-update"
        } else {
            "cmd-plugin-usage-install"
        }));
        return Ok(());
    };
    if reject_invalid_plugin_id(app, id) {
        return Ok(());
    }
    let Ok(account) = super::super::require_unlocked(app) else {
        return Ok(());
    };
    app.drain_task_events(32)?;
    if app.plugin_installs.contains_key(id) {
        app.info_message = Some(t!(app.i18n, "cmd-plugin-task-active", id = id));
        return Ok(());
    }
    if app.plugin_run_pending.is_some() {
        app.info_message = Some(t!(app.i18n, "cmd-plugin-run-active"));
        return Ok(());
    }
    let session = match app.vault_service.capture_session(&account) {
        Ok(session) => session,
        Err(error) => {
            app.error_message = Some(error);
            return Ok(());
        }
    };
    let (market_dir, data_dir) = match manager_dirs(app) {
        Ok(dirs) => dirs,
        Err(error) => {
            app.error_message = Some(t!(app.i18n, "cmd-plugin-init-failed", err = error));
            return Ok(());
        }
    };
    let plugin_id = id.to_owned();
    let task_id = app.tasks.spawn(session, move |context| async move {
        // 最新具体版本由共享 registry 决定，禁止将字符串 latest 当版本号。
        let prepared = match solosoul_plugin::PluginManager::new_with_dirs(market_dir, data_dir) {
            Ok(manager) => match manager
                .prepare_update_with_progress(&plugin_id, &|progress| {
                    context.report_plugin_progress(progress);
                })
                .await
            {
                Ok(prepared) => {
                    let manifest = prepared.manifest();
                    let name = manifest.name.clone();
                    let description = manifest.description.clone();
                    let tier = format!("{:?}", manifest.tier).to_lowercase();
                    let manifest_json = serde_json::to_string(manifest)
                        .map_err(|_| "无法准备插件详情快照".to_string());
                    manifest_json.map(|manifest_json| {
                        (manager, prepared, name, description, tier, manifest_json)
                    })
                }
                Err(error) => Err(error.to_string()),
            },
            Err(error) => Err(error.to_string()),
        };
        // 同步暂存阶段结束后主动让出；退出/取消可以在最终发布前回收 prepared。
        tokio::task::yield_now().await;
        context.commit(move || {
            let (manager, prepared, name, description, tier, manifest_json) = prepared?;
            let result = manager
                .publish_install(prepared)
                .map_err(|e| e.to_string())?;
            Ok(TaskOutput::PluginInstalled {
                plugin_id: result.plugin_id,
                version: result.version,
                name,
                description,
                tier,
                manifest_json,
                updated,
            })
        })
    });
    let task_id = match task_id {
        Ok(id) => id,
        Err(error) => {
            app.error_message = Some(error);
            return Ok(());
        }
    };
    app.plugin_installs.insert(
        id.to_owned(),
        PluginInstallTask {
            task_id,
            cancelling: false,
            updated,
            progress: PluginInstallProgress {
                percent: 0,
                phase: PluginInstallPhase::Preparing,
                downloaded_bytes: 0,
                total_bytes: None,
            },
        },
    );
    app.success_message = Some((
        t!(app.i18n, "cmd-plugin-task-started", id = id),
        Instant::now(),
    ));
    Ok(())
}

pub(super) fn cancel(app: &mut App, id: Option<&str>) -> Result<()> {
    let Some(id) = id else {
        app.error_message = Some(t!(app.i18n, "cmd-plugin-usage-cancel"));
        return Ok(());
    };
    if reject_invalid_plugin_id(app, id) {
        return Ok(());
    }
    app.drain_task_events(32)?;
    let Some(task) = app.plugin_installs.get_mut(id) else {
        app.info_message = Some(t!(app.i18n, "cmd-plugin-no-task", id = id));
        return Ok(());
    };
    if app.tasks.request_cancel(task.task_id) {
        task.cancelling = true;
    } else {
        app.info_message = Some(t!(app.i18n, "cmd-plugin-task-finishing", id = id));
    }
    Ok(())
}

/// 仅修改列表；与 /help 的返回缓存一起调用，不切回插件页面。
pub(crate) fn refresh_list(phase: &mut AppPhase, installed: &PluginSummary) {
    if let AppPhase::PluginList {
        plugins,
        selected,
        installed_only,
        filter,
    } = phase
    {
        if let Some(existing) = plugins.iter_mut().find(|p| p.id == installed.id) {
            existing
                .installed_version
                .clone_from(&installed.installed_version);
            if *installed_only {
                *existing = installed.clone();
            }
        } else if *installed_only {
            plugins.push(installed.clone());
        }
        plugins.sort_by(|a, b| a.id.cmp(&b.id));
        clamp_filtered_selection(plugins, selected, filter);
    }
}

pub(crate) fn remove_from_list(phase: &mut AppPhase, id: &str) {
    if let AppPhase::PluginList {
        plugins,
        selected,
        installed_only,
        filter,
    } = phase
    {
        if *installed_only {
            plugins.retain(|p| p.id != id);
        } else if let Some(plugin) = plugins.iter_mut().find(|p| p.id == id) {
            plugin.installed_version = None;
        }
        clamp_filtered_selection(plugins, selected, filter);
    }
}

fn clamp_filtered_selection(plugins: &[PluginSummary], selected: &mut usize, filter: &str) {
    let filter = filter.to_lowercase();
    let visible = plugins
        .iter()
        .filter(|plugin| {
            filter.is_empty()
                || plugin.name.to_lowercase().contains(&filter)
                || plugin.description.to_lowercase().contains(&filter)
        })
        .count();
    *selected = (*selected).min(visible.saturating_sub(1));
}

/// 完成事件回填准备阶段的完整 manifest；保持所在页面与返回缓存，不进行任何 IO。
pub(crate) fn refresh_detail(phase: &mut AppPhase, installed: &solosoul_plugin::PluginManifest) {
    if let AppPhase::PluginDetail { manifest } = phase {
        if manifest.id == installed.id {
            *manifest = installed.clone();
        }
    }
}

pub(crate) fn progress_lines(app: &App) -> Vec<String> {
    let mut installs: Vec<_> = app.plugin_installs.iter().collect();
    installs.sort_by(|a, b| a.0.cmp(b.0));
    installs
        .into_iter()
        .map(|(id, task)| {
            format!(
                "{}  {}  /plugin_cancel {}",
                id,
                progress_label(task, &app.i18n),
                id
            )
        })
        .collect()
}

pub(crate) fn progress_label(task: &PluginInstallTask, i18n: &I18n) -> String {
    let label = if task.cancelling {
        "plugin-task-cancelling"
    } else {
        match task.progress.phase {
            PluginInstallPhase::Preparing => "plugin-task-preparing",
            PluginInstallPhase::Downloading => "plugin-task-downloading",
            PluginInstallPhase::Verifying => "plugin-task-verifying",
            PluginInstallPhase::Installing | PluginInstallPhase::Finalizing => {
                "plugin-task-installing"
            }
            PluginInstallPhase::Completed => "plugin-task-installing",
        }
    };
    let bytes = match task.progress.total_bytes {
        Some(total) => format!("{} / {} B", task.progress.downloaded_bytes, total),
        None => format!("{} B", task.progress.downloaded_bytes),
    };
    format!(
        "{} {}% ({})",
        i18n.t(label),
        task.progress.percent.min(99),
        bytes
    )
}
