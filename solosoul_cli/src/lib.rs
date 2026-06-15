//! SoloSoul 终端 CLI 库。

/// 测试用全局锁，用于串行化涉及 `SOLOSOUL_DATA_DIR` 的测试。
pub static VAULT_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub mod app;
pub mod cli;
pub mod commands;
pub mod events;
pub mod screens;
pub mod tui;
pub mod widgets;
