//! Vault 目录管理命令（桌面 / Android）。
//!
//! Phase 1：支持 Android 端选择 SAF 目录作为持久化 Vault 存储位置，
//! 并提供手动同步命令。

use crate::fs::saf_sync_driver::TauriSafSyncDriver;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use solosoul_core::vault_file_system::{SafVaultFileSystem, VaultFileSystem};
use std::sync::Arc;
use tauri::{Manager, State};

/// 当前 Vault 目录类型。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VaultDirectoryType {
    /// 应用私有目录（默认）。
    Local,
    /// SAF 用户选择目录。
    Saf,
}

/// `vault_get_directory` 的返回结构。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultDirectoryInfo {
    pub directory_type: VaultDirectoryType,
    /// 当前激活的 SAF tree URI（未使用 SAF 时为 None）。
    pub saf_tree_uri: Option<String>,
}

/// `vault_set_directory` 的参数。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetVaultDirectoryPayload {
    /// SAF tree URI。传 None 表示切回本地应用私有目录。
    pub saf_tree_uri: Option<String>,
}

/// `vault_set_directory` 的返回结构。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetVaultDirectoryResult {
    pub success: bool,
    /// 应用需要重启才能在新目录下运行。
    pub needs_restart: bool,
    pub message: String,
}

/// 解析应用级配置路径。
fn app_config_path(data_dir: &std::path::Path) -> std::path::PathBuf {
    data_dir.join("app_config.json")
}

/// 读取 app_config.json 中保存的 SAF tree URI。
fn load_saved_saf_uri(data_dir: &std::path::Path) -> Option<String> {
    let path = app_config_path(data_dir);
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&path).ok()?;
    let config: serde_json::Value = serde_json::from_str(&content).ok()?;
    config
        .get("saf_tree_uri")
        .and_then(|v| v.as_str())
        .map(String::from)
}

/// 保存或删除 SAF tree URI 配置。
fn save_saf_uri(data_dir: &std::path::Path, uri: Option<&str>) -> Result<(), String> {
    let path = app_config_path(data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建配置目录失败: {e}"))?;
    }
    if let Some(uri) = uri {
        let config = serde_json::json!({ "saf_tree_uri": uri });
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?,
        )
        .map_err(|e| format!("写入配置失败: {e}"))?;
    } else if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("删除配置失败: {e}"))?;
    }
    Ok(())
}

/// 获取当前 Vault 目录信息。
#[tauri::command]
pub async fn vault_get_directory(state: State<'_, AppState>) -> Result<VaultDirectoryInfo, String> {
    let data_dir = state
        .handle
        .path()
        .resolve(".", tauri::path::BaseDirectory::Data)
        .map_err(|e| format!("无法解析应用数据目录: {e}"))?;
    let saved_uri = load_saved_saf_uri(&data_dir);

    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;

    let directory_type = if svc.is_remote_storage() {
        VaultDirectoryType::Saf
    } else {
        VaultDirectoryType::Local
    };

    Ok(VaultDirectoryInfo {
        saf_tree_uri: saved_uri,
        directory_type,
    })
}

/// 设置 Vault 目录。
///
/// - `saf_tree_uri` 为 None：切回本地应用私有目录，删除 SAF 配置。
/// - `saf_tree_uri` 为 Some：保存 URI，迁移现有 Vault 数据到新目录，并同步到 SAF。
///
/// 当前实现要求切换目录后重启应用，才能完全在新目录下重新初始化 VaultService。
#[tauri::command]
pub async fn vault_set_directory(
    state: State<'_, AppState>,
    payload: SetVaultDirectoryPayload,
) -> Result<SetVaultDirectoryResult, String> {
    let data_dir = state
        .handle
        .path()
        .resolve(".", tauri::path::BaseDirectory::Data)
        .map_err(|e| format!("无法解析应用数据目录: {e}"))?;

    // 锁定 Vault 以避免迁移过程中数据变更。
    {
        let svc = state
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        svc.lock();
    }

    if let Some(uri) = &payload.saf_tree_uri {
        if !cfg!(target_os = "android") {
            return Err("SAF Vault directory is only supported on Android".to_string());
        }
        if uri.is_empty() {
            return Err("SAF tree URI cannot be empty".to_string());
        }
        let temp_dir = data_dir.join("saf_vault_temp");
        std::fs::create_dir_all(&temp_dir).map_err(|e| format!("创建临时目录失败: {e}"))?;

        // 迁移：把当前本地 Vault 数据复制到 SAF 临时目录
        let local_dir = if cfg!(mobile) {
            data_dir.clone()
        } else {
            svc_base_path(&state)?
        };
        if local_dir != temp_dir {
            migrate_vault_data(&local_dir, &temp_dir)?;
        }

        // 通过临时文件系统同步到 SAF
        let sync_driver = Arc::new(TauriSafSyncDriver::<tauri::Wry>::new(state.handle.clone()));
        let fs = SafVaultFileSystem::new(uri.clone(), temp_dir.clone(), sync_driver);
        fs.sync_to_remote()
            .map_err(|e| format!("首次同步到 SAF 失败: {e}"))?;

        // 持久化配置
        save_saf_uri(&data_dir, Some(uri))?;

        Ok(SetVaultDirectoryResult {
            success: true,
            needs_restart: true,
            message: "目录已设置，请重启应用以使用新的 Vault 目录".to_string(),
        })
    } else {
        // 切回本地：仅删除配置，下次启动使用本地目录。
        save_saf_uri(&data_dir, None)?;
        Ok(SetVaultDirectoryResult {
            success: true,
            needs_restart: true,
            message: "已切回本地目录，请重启应用生效".to_string(),
        })
    }
}

/// 手动将 Vault 数据同步到远端（SAF）。
#[tauri::command]
pub async fn vault_sync_to_remote(state: State<'_, AppState>) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    svc.sync_to_remote()
}

/// 手动从远端（SAF）同步 Vault 数据到本地。
#[tauri::command]
pub async fn vault_sync_from_remote(state: State<'_, AppState>) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    svc.sync_from_remote()
}

/// 获取当前 VaultService 的本地 base_path。
fn svc_base_path(state: &AppState) -> Result<std::path::PathBuf, String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    Ok(svc.base_path().clone())
}

/// 把 src 目录下的 Vault 数据迁移到 dst 目录。
/// 跳过 app_config.json 和 saf_vault_temp 本身，避免循环。
fn migrate_vault_data(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
    if src == dst {
        return Ok(());
    }
    std::fs::create_dir_all(dst).map_err(|e| format!("创建目标目录失败: {e}"))?;

    // 先清空目标目录，避免残留旧数据。
    if let Ok(entries) = std::fs::read_dir(dst) {
        for entry in entries {
            let entry = entry.map_err(|e| format!("读取目标目录项失败: {e}"))?;
            let path = entry.path();
            if path.is_dir() {
                std::fs::remove_dir_all(&path).map_err(|e| format!("删除目标子目录失败: {e}"))?;
            } else {
                std::fs::remove_file(&path).map_err(|e| format!("删除目标文件失败: {e}"))?;
            }
        }
    }

    for entry in std::fs::read_dir(src).map_err(|e| format!("读取源目录失败: {e}"))? {
        let entry = entry.map_err(|e| format!("读取目录项失败: {e}"))?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // 跳过应用级配置和 SAF 临时目录，避免循环/冲突
        if name_str == "app_config.json" || name_str == "saf_vault_temp" {
            continue;
        }

        let src_path = entry.path();
        let dst_path = dst.join(&name);

        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            migrate_vault_data(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(|e| format!("复制文件失败: {e}"))?;
        }
    }

    Ok(())
}
