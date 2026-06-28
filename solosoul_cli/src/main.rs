//! SoloSoul 终端 CLI 入口。

use std::path::{Path, PathBuf};

use clap::Parser;
use color_eyre::Result;
use solosoul_cli::cli::{Cli, Commands};
use solosoul_cli::tui::{restore_terminal, Tui};
use solosoul_core::VaultService;
use tracing_appender::non_blocking::WorkerGuard;

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();
    let data_dir = resolve_data_dir(cli.data_dir);

    // 初始化日志文件输出，避免污染 TUI 界面。
    let _log_guard = init_logging(&data_dir)?;

    // 根据子命令快速路径执行（如 upgrade）。
    if let Some(cmd) = cli.command {
        match cmd {
            Commands::Upgrade => {
                println!(
                    "升级功能将在 Phase 4 实现。当前版本：{}",
                    env!("CARGO_PKG_VERSION")
                );
                return Ok(());
            }
        }
    }

    // 设置数据目录环境变量，供 VaultService 读取。
    if let Ok(dir) = data_dir.canonicalize() {
        std::env::set_var("SOLOSOUL_DATA_DIR", dir);
    } else {
        std::env::set_var("SOLOSOUL_DATA_DIR", &data_dir);
    }
    let vault_service = VaultService::new();

    // panic 时恢复终端。
    std::panic::set_hook(Box::new(|info| {
        let _ = restore_terminal();
        eprintln!("Panic: {}", info);
    }));

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

/// 将 tracing 日志写入 `{data_dir}/logs/cli.log`。
fn init_logging(data_dir: &Path) -> Result<WorkerGuard> {
    let log_dir = data_dir.join("logs");
    std::fs::create_dir_all(&log_dir)?;

    let file_appender = tracing_appender::rolling::never(&log_dir, "cli.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    Ok(guard)
}
