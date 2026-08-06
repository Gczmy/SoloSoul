//! /embed_model 本地 Embedding 模型管理命令。
//!
//! CLI 直接管理本地 embedding 模型目录 `{base_path}/embed_models/<id>/`，
//! 通过 reqwest 从 SoloSoul 模型注册表拉取清单，sha256 校验后写入磁盘。
//! 激活的 `local_embed_model_id` 仍由 GUI 设置（LlmConfig），CLI 当前不修改。
//!
//! 子命令：
//! - `/embed_model list` —— 列出本地目录中的模型
//! - `/embed_model install <id>` —— 下载并安装
//! - `/embed_model remove <id>` —— 删除本地模型目录
//! - `/embed_model status` —— 显示本地目录（不读 LLM config）
//! - `/embed_model help` —— 帮助

use crate::app::App;
use crate::t;
use color_eyre::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;

const DEFAULT_REGISTRY_URL: &str = "https://models.solosoul.dev/embed-registry.json";

pub fn handle(app: &mut App, argv: &[&str]) -> Result<()> {
    let sub = argv.first().copied().unwrap_or("status");
    match sub {
        "list" => {
            list(app);
            Ok(())
        }
        "install" => {
            install(app, argv.get(1).copied().unwrap_or(""));
            Ok(())
        }
        "remove" => {
            remove(app, argv.get(1).copied().unwrap_or(""));
            Ok(())
        }
        "status" => {
            status(app);
            Ok(())
        }
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => {
            app.error_message = Some(t!(app.i18n, "cmd-unknown-subcommand", cmd = other));
            Ok(())
        }
    }
}

/// 帮助文本，供 `/embed_model help` 显示。
pub fn help_text() -> Vec<&'static str> {
    vec![
        "用法: /embed_model <subcommand> [args]",
        "  list                       列出本地已安装/可用模型",
        "  install <model_id>         从注册表下载并安装指定模型",
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

/// CLI 用户使用的 embedding 模型本地目录：`{base_path}/embed_models`。
pub fn install_dir(app: &App) -> PathBuf {
    app.vault_service.base_path().join("embed_models")
}

fn list(app: &mut App) {
    let dir = install_dir(app);
    let entries = scan_local_models(&dir);
    app.previous_phase = Some(app.phase.clone());
    app.phase = crate::app::AppPhase::EmbedModelList {
        models: entries,
        info: format!("本地目录: {}", dir.display()),
    };
}

fn status(app: &mut App) {
    let dir = install_dir(app);
    let entries = scan_local_models(&dir);
    app.previous_phase = Some(app.phase.clone());
    app.phase = crate::app::AppPhase::EmbedModelList {
        models: entries,
        info: format!(
            "本地目录: {}；激活模型请在 GUI 设置，CLI 不直接读写 LlmConfig.active_embed_model_id。",
            dir.display()
        ),
    };
}

fn scan_local_models(dir: &PathBuf) -> Vec<crate::screens::embed_model::EmbedModelEntry> {
    let mut entries = Vec::new();
    if !dir.exists() {
        return entries;
    }
    if let Ok(read) = std::fs::read_dir(dir) {
        for e in read.flatten() {
            let path = e.path();
            if !path.is_dir() {
                continue;
            }
            entries.push(crate::screens::embed_model::EmbedModelEntry {
                id: e.file_name().to_string_lossy().to_string(),
                installed: true,
                size_mb: dir_size(&path) as f32 / 1024.0 / 1024.0,
                source: "本地".to_string(),
            });
        }
    }
    entries
}

fn dir_size(path: &PathBuf) -> u64 {
    let mut total = 0u64;
    if let Ok(read) = std::fs::read_dir(path) {
        for entry in read.flatten() {
            let p = entry.path();
            if p.is_file() {
                total += entry.metadata().map(|m| m.len()).unwrap_or(0);
            } else if p.is_dir() {
                total += dir_size(&p);
            }
        }
    }
    total
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegistryEntry {
    id: String,
    name: String,
    size_mb: f32,
    #[serde(default)]
    description: String,
    sha256: String,
    download_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegistryFile {
    #[serde(default)]
    models: Vec<RegistryEntry>,
}

fn install(app: &mut App, model_id: &str) {
    if model_id.is_empty() {
        app.error_message = Some(t!(app.i18n, "cmd-embed-usage"));
        return;
    }
    let dir = install_dir(app);
    let target = dir.join(model_id);
    if target.exists() {
        // R2-X2: 已安装提示为信息语义，走中性 info overlay
        app.info_message = Some(t!(
            app.i18n,
            "cmd-embed-already-installed",
            model = model_id
        ));
        return;
    }

    // R2-V7：运行时初始化失败优雅降级（不再 panic 退出 TUI）
    let rt = match crate::util::shared_runtime() {
        Ok(rt) => rt,
        Err(e) => {
            app.error_message = Some(format!("初始化共享运行时失败: {e}"));
            return;
        }
    };
    let result = rt.block_on(download_model(model_id, &target));
    match result {
        Ok(report) => {
            tracing::info!("embed_model install {} ok: {}", model_id, report);
            app.success_message = Some((
                t!(app.i18n, "cmd-embed-installed", model = model_id),
                Instant::now(),
            ));
        }
        Err(e) => {
            app.error_message = Some(t!(app.i18n, "cmd-embed-install-failed", err = e));
        }
    }
}

async fn download_model(model_id: &str, target_dir: &std::path::Path) -> Result<String, String> {
    let registry_url = std::env::var("SOLOSOUL_EMBED_REGISTRY")
        .unwrap_or_else(|_| DEFAULT_REGISTRY_URL.to_string());
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("构造 HTTP 客户端失败: {}", e))?;
    let registry_text = client
        .get(&registry_url)
        .send()
        .await
        .map_err(|e| format!("拉取注册表失败: {}", e))?
        .error_for_status()
        .map_err(|e| format!("注册表返回错误: {}", e))?
        .text()
        .await
        .map_err(|e| format!("读取注册表响应失败: {}", e))?;
    let registry: RegistryFile = serde_json::from_str(&registry_text)
        .map_err(|e| format!("解析注册表失败: {} (顶层需为 {{\"models\": [...]}})", e))?;
    let entry = registry
        .models
        .iter()
        .find(|m| m.id == model_id)
        .ok_or_else(|| format!("注册表中未找到模型 {}", model_id))?;

    let bytes = client
        .get(&entry.download_url)
        .send()
        .await
        .map_err(|e| format!("下载失败: {}", e))?
        .error_for_status()
        .map_err(|e| format!("下载返回错误: {}", e))?
        .bytes()
        .await
        .map_err(|e| format!("读取模型字节失败: {}", e))?;

    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let got = format!("{:x}", hasher.finalize());
    if !entry.sha256.is_empty() && got != entry.sha256 {
        return Err(format!(
            "sha256 校验失败: 期望 {} 实际 {}",
            entry.sha256, got
        ));
    }

    std::fs::create_dir_all(target_dir).map_err(|e| format!("创建模型目录失败: {}", e))?;
    let bin_path = target_dir.join("model.bin");
    std::fs::write(&bin_path, &bytes).map_err(|e| format!("写入模型文件失败: {}", e))?;

    Ok(format!(
        "写入 {} ({} bytes)",
        bin_path.display(),
        bytes.len()
    ))
}

fn remove(app: &mut App, model_id: &str) {
    if model_id.is_empty() {
        app.error_message = Some(t!(app.i18n, "cmd-embed-remove-usage"));
        return;
    }
    let dir = install_dir(app).join(model_id);
    if !dir.exists() {
        app.error_message = Some(t!(app.i18n, "cmd-embed-not-installed", model = model_id));
        return;
    }
    // 激活模型由 GUI 端 LlmConfig 管理,此处仅删除目录。
    match std::fs::remove_dir_all(&dir) {
        Ok(()) => {
            app.success_message = Some((
                t!(app.i18n, "cmd-embed-removed", model = model_id),
                Instant::now(),
            ));
        }
        Err(e) => {
            app.error_message = Some(t!(app.i18n, "cmd-embed-remove-failed", err = e));
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
