//! Vault 目录管理命令（桌面 / Android）。
//!
//! Phase 1：支持 Android 端选择 SAF 目录作为持久化 Vault 存储位置，
//! 并提供手动同步命令。

use crate::attachment_import_plugin::AttachmentImportPluginHandle;
use crate::fs::normalize_path;
use crate::fs::saf_sync_driver::TauriSafSyncDriver;
use crate::state::{AppState, InitializeVaultResult};
use serde::{Deserialize, Serialize};
use solosoul_core::vault_file_system::{SafVaultFileSystem, VaultFileSystem};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

/// 当前 Vault 目录类型。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VaultDirectoryType {
    /// 应用私有目录（默认）。
    #[serde(rename = "local")]
    Local,
    /// SAF 用户选择目录。
    #[serde(rename = "saf")]
    Saf,
}

/// `vault_get_directory` 的返回结构。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultDirectoryInfo {
    pub directory_type: VaultDirectoryType,
    /// 当前激活的 SAF tree URI（未使用 SAF 时为 None）。
    pub saf_tree_uri: Option<String>,
    /// SAF tree URI 是否仍然可访问（授权未被撤销）。
    /// 本地目录模式下该字段恒为 true。
    pub valid: bool,
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
    let data_dir = normalize_path(
        &state
            .handle
            .path()
            .resolve(".", tauri::path::BaseDirectory::Data)
            .map_err(|e| format!("无法解析应用数据目录: {e}"))?,
    );
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

    let valid = if let Some(ref uri) = saved_uri {
        check_saf_uri_validity(&state.handle, uri)
    } else {
        true
    };

    Ok(VaultDirectoryInfo {
        saf_tree_uri: saved_uri,
        directory_type,
        valid,
    })
}

/// 设置 Vault 目录。
///
/// - `saf_tree_uri` 为 None：切回本地应用私有目录，删除 SAF 配置。
/// - `saf_tree_uri` 为 Some：保存 URI，迁移现有 Vault 数据到新目录，并同步到 SAF。
///
/// 当前实现要求切换目录后重启应用，才能完全在新目录下重新初始化 VaultService。
///
/// 注意：数据迁移与首次同步在 `spawn_blocking` 中执行，避免阻塞 tokio 异步运行时。
#[tauri::command]
pub async fn vault_set_directory(
    state: State<'_, AppState>,
    payload: SetVaultDirectoryPayload,
) -> Result<SetVaultDirectoryResult, String> {
    let data_dir = normalize_path(
        &state
            .handle
            .path()
            .resolve(".", tauri::path::BaseDirectory::Data)
            .map_err(|e| format!("无法解析应用数据目录: {e}"))?,
    );

    // 锁定 Vault 以避免迁移过程中数据变更。
    //
    // 安全说明：`svc.lock()` 内部获取 `vault_store.write()`，而我们在此处
    // 持有 `vault_service.read()` guard。潜在的 A→B→A 死锁需要同时满足：
    // 1) 另一线程持有 `vault_store.read()`（`vault_handle()` 的极窄窗口）
    // 2) 该线程随后需要 `vault_service.write()`（生产代码中不存在）
    // 因此实践中不可能发生死锁。
    {
        let vault_service_arc = state.vault_service.clone();
        let svc = vault_service_arc
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
        let local_dir = {
            let svc = state
                .vault_service
                .read()
                .map_err(|_| "Vault service lock poisoned".to_string())?;
            if cfg!(mobile) {
                data_dir.clone()
            } else {
                let bp = svc.base_path().clone();
                drop(svc);
                bp
            }
        };

        // 迁移/同步进度通知（spawn_blocking 外执行 emit）
        let _ = state.handle.emit(
            "sync-progress",
            serde_json::json!({"phase": "migrate", "current": 0, "total": 3}),
        );

        // 克隆 temp_dir 供 spawn_blocking 内部使用，外部保留引用以写入 .solosoul_config
        let temp_dir_inner = temp_dir.clone();

        // 在 spawn_blocking 中执行迁移与同步，避免阻塞 tokio worker
        let uri_owned = uri.clone();
        let handle = state.handle.clone();
        tokio::task::spawn_blocking(move || -> Result<(), String> {
            if local_dir != temp_dir_inner {
                let _ = handle.emit(
                    "sync-progress",
                    serde_json::json!({"phase": "migrate", "current": 1, "total": 3}),
                );
                migrate_vault_data(&local_dir, &temp_dir_inner)?;
                let _ = handle.emit(
                    "sync-progress",
                    serde_json::json!({"phase": "migrate", "current": 2, "total": 3}),
                );
            }

            // 通过临时文件系统同步到 SAF
            let sync_driver = Arc::new(TauriSafSyncDriver::<tauri::Wry>::new(handle.clone()));
            let fs = SafVaultFileSystem::new(uri_owned, temp_dir_inner, sync_driver);
            fs.sync_to_remote()
                .map_err(|e| format!("首次同步到 SAF 失败: {e}"))?;

            let _ = handle.emit(
                "sync-progress",
                serde_json::json!({"phase": "migrate", "current": 3, "total": 3}),
            );
            Ok(())
        })
        .await
        .map_err(|e| format!("迁移任务失败: {e}"))??;

        // 写入 .solosoul_config 到 SAF 目录（含 saf_tree_uri 元数据），
        // 使卸载重装后用户选择相同目录时能自动恢复配置。
        let _ = AppState::write_saf_config_to_remote(
            &temp_dir,
            uri,
            Arc::new(TauriSafSyncDriver::<tauri::Wry>::new(state.handle.clone())),
        );
        // 写入 .solosoul_config 失败不影响主流程，仅打日志

        // 持久化配置（轻量 I/O，无需 spawn_blocking）
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
/// 同步期间每次文件操作时向前端发送进度事件。
#[tauri::command]
pub async fn vault_sync_to_remote(
    app: AppHandle<tauri::Wry>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // 发送同步开始事件
    let _ = app.emit(
        "sync-progress",
        serde_json::json!({"phase": "sync_to_remote", "current": 0, "total": 1}),
    );

    let result = {
        let svc = state
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        svc.sync_to_remote()
    };

    // 发送同步完成事件
    let _ = app.emit(
        "sync-progress",
        serde_json::json!({"phase": "sync_to_remote", "current": 1, "total": 1}),
    );

    result
}

/// 手动从远端（SAF）同步 Vault 数据到本地。
/// 同步期间每次文件操作时向前端发送进度事件。
#[tauri::command]
pub async fn vault_sync_from_remote(
    app: AppHandle<tauri::Wry>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // 发送同步开始事件
    let _ = app.emit(
        "sync-progress",
        serde_json::json!({"phase": "sync_from_remote", "current": 0, "total": 1}),
    );

    let result = {
        let svc = state
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        svc.sync_from_remote()
    };

    // 发送同步完成事件
    let _ = app.emit(
        "sync-progress",
        serde_json::json!({"phase": "sync_from_remote", "current": 1, "total": 1}),
    );

    result
}

/// 触发一次切后台同步（仅 SAF 模式下有效）。
///
/// 应用切到后台时前端调用此命令，通知 `AutoSyncManager` 立即执行一次
/// `sync_to_remote()`。命令本身不等待同步完成，立即返回。
#[tauri::command]
pub async fn vault_sync_background(state: State<'_, AppState>) -> Result<(), String> {
    state.auto_sync.trigger_background();
    Ok(())
}

/// 检查 SAF tree URI 是否仍然可访问。
fn check_saf_uri_validity<R: Runtime>(app: &AppHandle<R>, tree_uri: &str) -> bool {
    let handle = app.state::<AttachmentImportPluginHandle<R>>();
    handle.check_vault_dir_access(tree_uri).unwrap_or(false)
}

/// 首次启动时初始化 Vault 目录（无需重启）。
/// 仅在 Android 上可用；桌面端调用会返回错误。
///
/// 当选择 SAF 目录时，命令会等待首次同步完成；同步失败将返回
/// 错误，前端可据此提示用户。
#[tauri::command]
pub async fn init_vault_directory(
    state: State<'_, AppState>,
    payload: SetVaultDirectoryPayload,
) -> Result<InitializeVaultResult, String> {
    state.initialize_vault(payload.saf_tree_uri).await
}

/// 检查当前 Vault 目录的 SAF URI 是否仍然有效。
/// 返回 `{ valid: bool }`。
#[tauri::command]
pub async fn vault_check_directory<R: Runtime>(app: AppHandle<R>) -> Result<bool, String> {
    let data_dir = app
        .path()
        .resolve(".", tauri::path::BaseDirectory::Data)
        .map_err(|e| format!("无法解析应用数据目录: {e}"))?;
    let saved_uri = load_saved_saf_uri(&data_dir);
    match saved_uri {
        Some(uri) => Ok(check_saf_uri_validity(&app, &uri)),
        None => Ok(true), // No SAF URI = nothing to validate
    }
}

// 不应被迁移/同步的应用级目录/文件名称由 build.rs 从 app_level_names.json
// 自动生成，避免 Rust/Kotlin 手动同步。
include!(concat!(env!("OUT_DIR"), "/app_level_names.rs"));

/// 把 src 目录下的 Vault 数据迁移到 dst 目录。
/// 仅顶层跳过应用级配置、资源、缓存和 SAF 临时目录本身，避免循环/冲突；
/// 嵌套目录中的同名文件夹仍正常迁移，避免误删用户 Vault 数据。
pub(crate) fn migrate_vault_data(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
    fn inner(src: &std::path::Path, dst: &std::path::Path, depth: usize) -> Result<(), String> {
        if src == dst {
            return Ok(());
        }
        std::fs::create_dir_all(dst).map_err(|e| format!("创建目标目录失败: {e}"))?;

        // 仅在顶层目录清空目标，避免嵌套时误删已迁移的兄弟目录。
        if depth == 0 {
            if let Ok(entries) = std::fs::read_dir(dst) {
                for entry in entries {
                    let entry = entry.map_err(|e| format!("读取目标目录项失败: {e}"))?;
                    let path = entry.path();
                    if path.is_dir() {
                        std::fs::remove_dir_all(&path)
                            .map_err(|e| format!("删除目标子目录失败: {e}"))?;
                    } else {
                        std::fs::remove_file(&path)
                            .map_err(|e| format!("删除目标文件失败: {e}"))?;
                    }
                }
            }
        }

        for entry in std::fs::read_dir(src).map_err(|e| format!("读取源目录失败: {e}"))? {
            let entry = entry.map_err(|e| format!("读取目录项失败: {e}"))?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // 仅在顶层跳过应用级配置、资源、缓存和 SAF 临时目录
            if depth == 0 && APP_LEVEL_NAMES.contains(&name_str.as_ref()) {
                continue;
            }

            let src_path = entry.path();
            let dst_path = dst.join(&name);

            if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
                inner(&src_path, &dst_path, depth + 1)?;
            } else {
                std::fs::copy(&src_path, &dst_path).map_err(|e| format!("复制文件失败: {e}"))?;
            }
        }

        Ok(())
    }

    inner(src, dst, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrate_vault_data_skips_app_level_entries() {
        let src = tempfile::TempDir::new().unwrap();
        let dst = tempfile::TempDir::new().unwrap();

        // 创建 Vault 数据
        std::fs::write(src.path().join("vault.db"), "vault data").unwrap();
        std::fs::create_dir_all(src.path().join("attachments")).unwrap();
        std::fs::write(src.path().join("attachments").join("a.txt"), "attachment").unwrap();

        // 创建应用级目录/文件
        for name in APP_LEVEL_NAMES {
            let path = src.path().join(name);
            if name.ends_with(".json") {
                std::fs::write(&path, "{}").unwrap();
            } else {
                std::fs::create_dir_all(&path).unwrap();
                std::fs::write(path.join("dummy.txt"), "dummy").unwrap();
            }
        }

        migrate_vault_data(src.path(), dst.path()).unwrap();

        // Vault 数据应被迁移
        assert!(dst.path().join("vault.db").exists());
        assert!(dst.path().join("attachments").join("a.txt").exists());

        // 应用级条目应被跳过（包括其内部内容）
        for name in APP_LEVEL_NAMES {
            assert!(!dst.path().join(name).exists(), "应跳过 {}", name);
        }
        assert!(!dst.path().join("resources").join("dummy.txt").exists());
    }

    #[test]
    fn test_migrate_vault_data_preserves_user_directory_named_resources() {
        let src = tempfile::TempDir::new().unwrap();
        let dst = tempfile::TempDir::new().unwrap();

        // 在 Vault 内部创建一个名为 "resources" 的子目录（不是顶层应用级目录）
        let user_resources = src.path().join("objects").join("resources");
        std::fs::create_dir_all(&user_resources).unwrap();
        std::fs::write(user_resources.join("object.txt"), "user data").unwrap();

        migrate_vault_data(src.path(), dst.path()).unwrap();

        // 顶层的 "resources" 目录不存在（因为 src 顶层没有），
        // 但 Vault 内部的 "objects/resources" 应该被保留
        assert!(dst
            .path()
            .join("objects")
            .join("resources")
            .join("object.txt")
            .exists());
    }

    #[test]
    fn test_migrate_vault_data_same_src_dst_is_noop() {
        let src = tempfile::TempDir::new().unwrap();
        std::fs::write(src.path().join("vault.db"), "vault data").unwrap();

        migrate_vault_data(src.path(), src.path()).unwrap();

        assert!(src.path().join("vault.db").exists());
    }
}
