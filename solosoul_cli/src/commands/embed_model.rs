//! /embed_model 本地模型列表与受管下载命令。
//!
//! CLI 保持原始二进制 model.bin 格式；GUI 的激活模型与安装格式由 GUI 管理。

use crate::app::{App, AppPhase};
use crate::screens::embed_model::EmbedDownloadView;
use crate::t;
use crate::tasks::TaskId;
use color_eyre::Result;
use std::path::PathBuf;
use std::time::Instant;

mod download;

const DEFAULT_REGISTRY_URL: &str = "https://models.solosoul.dev/embed-registry.json";

/// 模型占位保留到真实终态；取消请求本身不能让同模型重复启动。
#[derive(Debug, Clone)]
pub(crate) struct EmbedDownload {
    pub task_id: TaskId,
    pub cancelling: bool,
}

pub fn handle(app: &mut App, argv: &[&str]) -> Result<()> {
    let sub = argv.first().copied().unwrap_or("status");
    match sub {
        "list" => list(app),
        "install" => install(app, argv.get(1).copied().unwrap_or("")),
        "cancel" => cancel(app, argv.get(1).copied().unwrap_or("")),
        "remove" => remove(app, argv.get(1).copied().unwrap_or("")),
        "status" => status(app),
        "help" | "--help" | "-h" => print_help(),
        other => app.error_message = Some(t!(app.i18n, "cmd-unknown-subcommand", cmd = other)),
    }
    Ok(())
}

pub fn help_text() -> Vec<&'static str> {
    vec![
        "用法: /embed_model <subcommand> [args]",
        "  list                       列出本地模型和下载进度",
        "  install <model_id>         在后台下载并安装指定模型",
        "  cancel <model_id>          取消尚未提交的下载",
        "  remove <model_id>          删除本地 embedding 模型目录",
        "  status                     显示当前本地目录情况",
        "  help                       显示本帮助",
    ]
}

fn print_help() {
    for line in help_text() {
        println!("{line}");
    }
}

pub fn install_dir(app: &App) -> PathBuf {
    app.vault_service.base_path().join("embed_models")
}

fn list(app: &mut App) {
    let dir = install_dir(app);
    let entries = scan_local_models(&dir);
    if !matches!(app.phase, AppPhase::EmbedModelList { .. }) {
        app.previous_phase = Some(app.phase.clone());
    }
    app.phase = AppPhase::EmbedModelList {
        models: entries,
        info: format!("本地目录: {}", dir.display()),
    };
}

fn status(app: &mut App) {
    list(app);
    if let AppPhase::EmbedModelList { info, .. } = &mut app.phase {
        info.push_str("；CLI 管理 model.bin，GUI 模型安装与激活由 GUI 设置。");
    }
}

fn scan_local_models(dir: &std::path::Path) -> Vec<crate::screens::embed_model::EmbedModelEntry> {
    let mut entries = Vec::new();
    if let Ok(read) = std::fs::read_dir(dir) {
        for entry in read.flatten() {
            let Some(id) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if let Some(bytes) = download::installed_size(dir, &id) {
                entries.push(crate::screens::embed_model::EmbedModelEntry {
                    id,
                    installed: true,
                    size_mb: bytes as f32 / 1024.0 / 1024.0,
                    source: "本地".to_string(),
                });
            }
        }
    }
    entries.sort_by(|left, right| left.id.cmp(&right.id));
    entries
}

pub(crate) fn download_views(app: &App) -> Vec<EmbedDownloadView> {
    let mut downloads: Vec<_> = app
        .embed_downloads
        .iter()
        .map(|(id, task)| {
            let (current, total) = app
                .task_progress
                .get(&task.task_id)
                .copied()
                .unwrap_or((0, None));
            EmbedDownloadView {
                model_id: id.clone(),
                current,
                total,
                cancelling: task.cancelling,
            }
        })
        .collect();
    downloads.sort_by(|left, right| left.model_id.cmp(&right.model_id));
    downloads
}

fn install(app: &mut App, model_id: &str) {
    if !download::valid_model_id(model_id) {
        app.error_message = Some(t!(app.i18n, "cmd-embed-invalid-id"));
        return;
    }
    let Ok(account) = super::require_unlocked(app) else {
        return;
    };
    let session = match app.vault_service.capture_session(&account) {
        Ok(session) => session,
        Err(_) => {
            app.error_message = Some(t!(app.i18n, "cmd-need-unlock"));
            return;
        }
    };
    if let Err(error) = app.drain_task_events(32) {
        app.error_message = Some(error.to_string());
        return;
    }
    if app.embed_downloads.contains_key(model_id) {
        app.info_message = Some(t!(app.i18n, "cmd-embed-in-progress", model = model_id));
        return;
    }
    let dir = install_dir(app);
    if download::installed_size(&dir, model_id).is_some() {
        app.info_message = Some(t!(
            app.i18n,
            "cmd-embed-already-installed",
            model = model_id
        ));
        return;
    }
    let registry = std::env::var("SOLOSOUL_EMBED_REGISTRY")
        .unwrap_or_else(|_| DEFAULT_REGISTRY_URL.to_string());
    let id = model_id.to_string();
    match app.tasks.spawn(session, move |context| {
        download::download_model(context, id, dir, registry)
    }) {
        Ok(task_id) => {
            app.embed_downloads.insert(
                model_id.to_string(),
                EmbedDownload {
                    task_id,
                    cancelling: false,
                },
            );
            list(app);
        }
        Err(error) => {
            app.error_message = Some(t!(app.i18n, "cmd-embed-install-failed", err = error))
        }
    }
}

fn cancel(app: &mut App, model_id: &str) {
    if !download::valid_model_id(model_id) {
        app.error_message = Some(t!(app.i18n, "cmd-embed-cancel-usage"));
        return;
    }
    let Some(download) = app.embed_downloads.get_mut(model_id) else {
        app.info_message = Some(t!(app.i18n, "cmd-embed-no-download", model = model_id));
        return;
    };
    if app.tasks.request_cancel(download.task_id) {
        download.cancelling = true;
    } else {
        app.info_message = Some(t!(app.i18n, "cmd-embed-finishing", model = model_id));
    }
}

fn remove(app: &mut App, model_id: &str) {
    if !download::valid_model_id(model_id) {
        app.error_message = Some(t!(app.i18n, "cmd-embed-invalid-id"));
        return;
    }
    if app.embed_downloads.contains_key(model_id) {
        app.error_message = Some(t!(app.i18n, "cmd-embed-in-progress", model = model_id));
        return;
    }
    let root = install_dir(app);
    let dir = root.join(model_id);
    if !dir.exists() {
        app.error_message = Some(t!(app.i18n, "cmd-embed-not-installed", model = model_id));
        return;
    }
    // 删除路径必须是模型根下的真实直接子目录，不能追随符号链接/重解析跳转。
    let safe = root
        .canonicalize()
        .ok()
        .zip(dir.canonicalize().ok())
        .is_some_and(|(root, target)| {
            target.parent() == Some(root.as_path())
                && target.file_name() == dir.file_name()
                && std::fs::symlink_metadata(&dir)
                    .is_ok_and(|m| !m.file_type().is_symlink() && m.is_dir())
        });
    if !safe {
        app.error_message = Some(t!(app.i18n, "cmd-embed-invalid-id"));
        return;
    }
    match std::fs::remove_dir_all(&dir) {
        Ok(()) => {
            for page in std::iter::once(&mut app.phase).chain(app.previous_phase.iter_mut()) {
                if let AppPhase::EmbedModelList { models, .. } = page {
                    models.retain(|model| model.id != model_id);
                }
            }
            app.success_message = Some((
                t!(app.i18n, "cmd-embed-removed", model = model_id),
                Instant::now(),
            ));
        }
        Err(error) => {
            app.error_message = Some(t!(app.i18n, "cmd-embed-remove-failed", err = error))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, AppPhase};
    use solosoul_core::VaultService;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn setup_app() -> (App, TempDir) {
        let _guard = crate::VAULT_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let dir = TempDir::new().unwrap();
        let vault = VaultService::with_base_path(dir.path().to_path_buf());
        vault
            .create_account("EmbedTest", crate::TEST_PASSWORD, None)
            .unwrap();
        let app = App::new(Arc::new(vault)).unwrap();
        (app, dir)
    }

    #[test]
    fn embed_model_dir_is_under_vault_base() {
        let (app, dir) = setup_app();
        let expected = dir.path().join("embed_models");
        assert_eq!(install_dir(&app), expected);
    }

    #[test]
    fn embed_model_status_empty_dir() {
        let (mut app, _dir) = setup_app();
        status(&mut app);
        if let AppPhase::EmbedModelList { models, info } = &app.phase {
            assert!(models.is_empty());
            assert!(info.contains("目录"));
        } else {
            panic!("expected EmbedModelList");
        }
    }

    #[test]
    fn embed_model_list_detects_installed() {
        let (mut app, dir) = setup_app();
        let models_dir = dir.path().join("embed_models").join("test-model");
        std::fs::create_dir_all(&models_dir).unwrap();
        std::fs::write(models_dir.join("model.bin"), b"x".repeat(2048)).unwrap();
        list(&mut app);
        if let AppPhase::EmbedModelList { models, .. } = &app.phase {
            assert_eq!(models.len(), 1);
            assert_eq!(models[0].id, "test-model");
            assert!(models[0].installed);
        } else {
            panic!("expected EmbedModelList");
        }
    }

    #[test]
    fn embed_model_remove_missing() {
        let (mut app, _dir) = setup_app();
        remove(&mut app, "non-existent");
        assert!(app.error_message.is_some());
    }
}

#[cfg(test)]
#[path = "embed_model/rf212_tests.rs"]
mod rf212_tests;
