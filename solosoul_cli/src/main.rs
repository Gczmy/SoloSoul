//! SoloSoul 终端 CLI 入口。

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use color_eyre::Result;
use solosoul_cli::cli::Cli;
use solosoul_cli::tui::{install_panic_hook, Tui};
use solosoul_cli::util::OwnerPinnedWriter;
use solosoul_core::VaultService;
use tracing_appender::non_blocking::WorkerGuard;

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();
    let data_dir = resolve_data_dir(cli.data_dir);

    // 所有权失败立即返回：账户、日志和终端初始化均不得先写入目标目录。
    let vault_service = VaultService::try_with_base_path(data_dir)
        .map_err(|error| {
            // 失锁时不能先读该目录的语言偏好，固定双语提示保留稳定错误码。
            let message = if error == "VAULT_DIRECTORY_BUSY" {
                "Vault 目录正被其他进程使用，请关闭后重试。Another process is using the Vault directory. Close it and try again."
            } else {
                "无法取得 Vault 目录所有权，启动已停止。Cannot acquire Vault directory ownership; startup stopped."
            };
            color_eyre::eyre::eyre!("{message} ({error})")
        })?;
    // 同 Arc 供 main 和实际日志 writer 复用；不把 WorkerGuard Drop 当作已 join。
    let root_owner = vault_service.root_owner();
    let _log_guard = init_logging(Arc::clone(&root_owner))?;
    std::env::set_var("SOLOSOUL_DATA_DIR", root_owner.root());
    // try_with_base_path 与旧 with_base_path 一样不自动填充账户缓存。
    vault_service.load_accounts();

    // 只有 TUI 线程异常才恢复终端；受管 worker 的异常由任务事件反馈。
    install_panic_hook();

    // 进入全屏 TUI。
    let mut tui = Tui::new(vault_service)?;
    tui.run()?;

    Ok(())
}

/// 解析数据目录优先级：--data-dir > SOLOSOUL_DATA_DIR > 默认目录。
fn resolve_data_dir(flag: Option<PathBuf>) -> PathBuf {
    if let Some(dir) = flag {
        return dir;
    }
    if let Ok(dir) = std::env::var("SOLOSOUL_DATA_DIR") {
        return PathBuf::from(dir);
    }
    default_data_dir()
}

fn default_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(profile) = std::env::var("USERPROFILE") {
            return PathBuf::from(profile).join(".solosoul");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".solosoul")
    } else {
        eprintln!(
            "错误：无法确定 HOME 目录。请通过 --data-dir 或 SOLOSOUL_DATA_DIR 环境变量指定数据目录。"
        );
        std::process::exit(1);
    }
}

/// 可通过 `RUST_LOG` 覆盖日志级别的 crate 白名单。
///
/// 白名单之外的 target（reqwest/tokio/rusqlite 等依赖）一律保持默认 INFO，
/// 防止 `RUST_LOG=debug` 把依赖的 LLM 请求、vault 操作明细写进 cli.log。
const LOG_CRATE_ALLOWLIST: &[&str] = &[
    "solosoul_cli",
    "solosoul",
    "solosoul_core",
    "solosoul_crypto",
    "solosoul_sync",
    "solosoul_vault",
    "solosoul_plugin",
];

/// 将 tracing 日志写入 `{data_dir}/logs/cli.log`（按日轮转，避免无限增长）。
fn init_logging(
    root_owner: Arc<solosoul_vault::root_owner::VaultRootOwner>,
) -> Result<WorkerGuard> {
    let log_dir = root_owner.root().join("logs");
    std::fs::create_dir_all(&log_dir)?;

    let file_appender = tracing_appender::rolling::daily(&log_dir, "cli.log");
    // non_blocking 的真正线程持 writer；即使 WorkerGuard 等待超时，根锁仍保活。
    let writer = OwnerPinnedWriter::new(file_appender, root_owner);
    let (non_blocking, guard) = tracing_appender::non_blocking(writer);

    let filter = build_env_filter();

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_env_filter(filter)
        .init();

    Ok(guard)
}

/// 从 `RUST_LOG` 构建 EnvFilter，仅接受白名单内 crate 的 directive。
///
/// 裸级别 directive（如 `RUST_LOG=debug`，无 crate 前缀）会被忽略，
/// 避免依赖 crate 的 debug 日志泄入本地日志文件；默认级别为 INFO。
fn build_env_filter() -> tracing_subscriber::EnvFilter {
    let mut directives: Vec<String> = Vec::new();
    if let Ok(rust_log) = std::env::var("RUST_LOG") {
        for raw in rust_log.split(',') {
            let d = raw.trim();
            if d.is_empty() {
                continue;
            }
            // 提取 target 部分（可能带 =level 后缀）
            let target_part = d.split('=').next().unwrap_or(d).trim();
            let target = target_part.strip_prefix("target:").unwrap_or(target_part);
            let top = target.split("::").next().unwrap_or(target);
            if LOG_CRATE_ALLOWLIST.contains(&top) {
                directives.push(d.to_string());
            }
        }
    }
    directives.push("info".to_string());
    tracing_subscriber::EnvFilter::try_new(directives.join(","))
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
}
