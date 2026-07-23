//! SAF-backed Vault 文件系统同步驱动（Tauri app 层实现）。
//!
//! `solosoul-core` 只定义了 `SafSyncDriver` trait，真正的同步逻辑需要调用
//! Android Kotlin 插件。本文件把 Tauri 插件句柄包装成一个 `SafSyncDriver`
//! 实现，供 `SafVaultFileSystem` 使用。

use solosoul_core::vault_file_system::SafSyncDriver;
use std::path::Path;
use tauri::{AppHandle, Manager, Runtime};

use crate::attachment_import_plugin::AttachmentImportPluginHandle;

/// 基于 Tauri Android 插件的 SAF 同步驱动。
///
/// 每次同步时从 AppHandle 状态中获取 `AttachmentImportPluginHandle`，
/// 调用 Kotlin 端实现的 `syncDirToRemote` / `syncDirFromRemote` 命令。
pub struct TauriSafSyncDriver<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriSafSyncDriver<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> SafSyncDriver for TauriSafSyncDriver<R> {
    fn sync_to_remote(&self, local_dir: &Path, tree_uri: &str) -> Result<(), String> {
        let handle = self.app.state::<AttachmentImportPluginHandle<R>>();
        handle.sync_dir_to_remote(local_dir.to_string_lossy().as_ref(), tree_uri)
    }

    fn sync_from_remote(&self, local_dir: &Path, tree_uri: &str) -> Result<(), String> {
        let handle = self.app.state::<AttachmentImportPluginHandle<R>>();
        handle.sync_dir_from_remote(local_dir.to_string_lossy().as_ref(), tree_uri)
    }
}
