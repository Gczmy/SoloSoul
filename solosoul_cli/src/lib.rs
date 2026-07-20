//! SoloSoul 终端 CLI 库。

/// 测试用全局锁，用于串行化涉及 `SOLOSOUL_DATA_DIR` 的测试。
pub static VAULT_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// 测试用默认密码常量，集中管理避免各文件分散的字符串字面量。
pub const TEST_PASSWORD: &str = "password123";

/// 测试用导出密码常量。
pub const TEST_EXPORT_PASSWORD: &str = "ExportPass1";

pub mod app;
pub mod cli;
pub mod commands;
pub mod events;
pub mod i18n;
pub mod screens;
pub mod theme;
pub mod tui;
pub mod util;
pub mod widgets;
