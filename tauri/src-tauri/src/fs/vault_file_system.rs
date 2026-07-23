//! Vault 文件系统抽象层（Tauri 应用层 shim）
//!
//! 真正的 `VaultFileSystem` trait 与 `LocalVaultFileSystem` 实现位于
//! `solosoul-core`，本文件仅做 re-export，方便 Tauri 应用层统一引用。

pub use solosoul_core::vault_file_system::{LocalVaultFileSystem, VaultFileSystem};
