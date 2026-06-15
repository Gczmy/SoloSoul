//! SoloSoul 终端 CLI 库。

#[cfg(test)]
pub(crate) static VAULT_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub mod app;
pub mod cli;
pub mod commands;
pub mod events;
pub mod screens;
pub mod tui;
pub mod widgets;
