//! 命令行参数定义。

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "solosoul")]
#[command(about = "SoloSoul 终端版本 — 本地优先、零知识的个人数字孪生")]
#[command(version)]
pub struct Cli {
    /// 指定数据目录，覆盖 SOLOSOUL_DATA_DIR 与默认目录
    #[arg(long, value_name = "PATH")]
    pub data_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// 检查并安装新版本（Phase 4 实现，Phase 1 仅占位输出提示）
    Upgrade,
}
