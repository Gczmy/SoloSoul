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
use solosoul_core::VaultService as CoreVaultService;
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
        //
        // ⚠️ 源目录必须取 `svc.base_path()`（当前 Vault 实际数据位置），不能固定取 data_dir：
        // - 本地模式：base_path = data_dir（账户/对象数据在 data_dir 根）
        // - SAF 模式：base_path = data_dir/saf_vault_temp（真实数据在 SAF 缓存目录）
        // 旧实现固定用 data_dir 作为迁移源，SAF→SAF 切换时会把 data_dir（不含账户数据）
        // 以 clear_dst=true 复制进 saf_vault_temp，**清空真实缓存**，重启后账户全部丢失
        // （表现为「重新创建账户」页面）。src==dst 时 migrate_vault_data 为 no-op，
        // 直接保留缓存并同步到新 SAF URI，不再清空。
        let local_dir = {
            let svc = state
                .vault_service
                .read()
                .map_err(|_| "Vault service lock poisoned".to_string())?;
            svc.base_path().clone()
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
            // ① 先拉取远端（目标 SAF 目录）已有数据到 temp——
            // 防止本地（可能只有新账户）清单覆盖远端 accounts.json 导致旧账户丢失。
            // 远端为空目录时 no-op。
            let fs: Arc<dyn VaultFileSystem> = Arc::new(SafVaultFileSystem::new(
                uri_owned,
                temp_dir_inner.clone(),
                Arc::new(TauriSafSyncDriver::<tauri::Wry>::new(handle.clone())),
            ));
            fs.sync_from_remote()
                .map_err(|e| format!("从 SAF 拉取已有数据失败: {e}"))?;

            // ② 合并本地数据到 temp（clear_dst=false 不清空，保留刚拉取的远端数据）
            if local_dir != temp_dir_inner {
                let _ = handle.emit(
                    "sync-progress",
                    serde_json::json!({"phase": "migrate", "current": 1, "total": 3}),
                );
                migrate_vault_data(&local_dir, &temp_dir_inner, false)?;
                let _ = handle.emit(
                    "sync-progress",
                    serde_json::json!({"phase": "migrate", "current": 2, "total": 3}),
                );
            }

            // ③ 重建账户清单：load_accounts 读合并后 temp 的 accounts.json，
            // 再扫描 acc_* 目录恢复「清单中缺失但目录还在」的旧账户
            // （① 拉取的远端旧账户目录不被 ② 清空，此处即可找回）。
            {
                let svc = CoreVaultService::with_file_system(temp_dir_inner.clone(), fs.clone());
                svc.load_accounts();
                let recovered = svc.scan_orphan_accounts()?;
                if !recovered.is_empty() {
                    tracing::info!("[vault_set_directory] 恢复孤儿账户: {:?}", recovered);
                }
            }

            // ④ 推送合并后的完整数据到 SAF
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

        // 调度 WorkManager 兜底同步，确保应用被系统回收后仍能同步到 SAF。
        if let Err(e) = state.schedule_saf_fallback_sync() {
            tracing::warn!("[vault_set_directory] failed to schedule SAF fallback sync: {e}");
        }

        Ok(SetVaultDirectoryResult {
            success: true,
            needs_restart: true,
            message: "目录已设置，请重启应用以使用新的 Vault 目录".to_string(),
        })
    } else {
        // 切回本地：取消后台兜底同步，并删除配置。
        if let Err(e) = state.cancel_saf_fallback_sync() {
            tracing::warn!("[vault_set_directory] failed to cancel SAF fallback sync: {e}");
        }
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
///
/// - `clear_dst = true`：迁移前清空 dst 顶层（用于全新目标目录，如首次
///   切换到 SAF 时把本地数据迁入空的 `saf_vault_temp`）。
/// - `clear_dst = false`：不清空 dst，仅把 src 内容合并进 dst（用于 SAF
///   降级场景——此时 src（`saf_vault_temp`）位于 dst（应用数据目录）内部，
///   若清空 dst 会连带删除源目录本身与应用级目录，导致数据丢失与启动闪退）。
///
/// 仅顶层跳过应用级配置、资源、缓存和 SAF 临时目录本身，避免循环/冲突；
/// 嵌套目录中的同名文件夹仍正常迁移，避免误删用户 Vault 数据。
pub(crate) fn migrate_vault_data(
    src: &std::path::Path,
    dst: &std::path::Path,
    clear_dst: bool,
) -> Result<(), String> {
    fn inner(
        src: &std::path::Path,
        dst: &std::path::Path,
        depth: usize,
        clear_dst: bool,
    ) -> Result<(), String> {
        if src == dst {
            return Ok(());
        }
        std::fs::create_dir_all(dst).map_err(|e| format!("创建目标目录失败: {e}"))?;

        // 仅在顶层且要求清空时清空目标，避免嵌套时误删已迁移的兄弟目录。
        // 注意：SAF 降级场景（src 位于 dst 内部）必须传 false，
        // 否则会先删除源目录本身及应用级目录（logs/app_resources/models）。
        if depth == 0 && clear_dst {
            clear_target_dir(dst)?;
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
                inner(&src_path, &dst_path, depth + 1, clear_dst)?;
            } else {
                std::fs::copy(&src_path, &dst_path).map_err(|e| format!("复制文件失败: {e}"))?;
            }
        }

        Ok(())
    }

    inner(src, dst, 0, clear_dst)
}

/// P045: 清空目标目录（仅顶层迁移时调用）——从 migrate_vault_data 内层拆出，
/// 消除「if depth==0 && clear_dst → for → if is_dir」3 层嵌套。
fn clear_target_dir(dst: &std::path::Path) -> Result<(), String> {
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
    Ok(())
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

        migrate_vault_data(src.path(), dst.path(), true).unwrap();

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

        migrate_vault_data(src.path(), dst.path(), true).unwrap();

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

        migrate_vault_data(src.path(), src.path(), true).unwrap();

        assert!(src.path().join("vault.db").exists());
    }

    /// 回归测试：SAF 降级场景 src（saf_vault_temp）位于 dst（应用数据目录）内部。
    /// 使用合并模式（clear_dst=false）时不得清空 dst——否则会连同源目录本身
    /// 以及 logs/app_resources/models 等应用级目录一起删除，
    /// 导致用户数据被毁并触发插件管理器初始化失败（启动闪退）。
    #[test]
    fn test_migrate_vault_data_merge_keeps_src_inside_dst() {
        let root = tempfile::TempDir::new().unwrap();
        let dst = root.path().to_path_buf();
        let src = dst.join("saf_vault_temp");
        std::fs::create_dir_all(&src).unwrap();

        // src 中的 Vault 数据
        std::fs::write(src.join("accounts.json"), "[]").unwrap();
        std::fs::create_dir_all(src.join("acc_1")).unwrap();
        std::fs::write(src.join("acc_1").join("config.json"), "{}").unwrap();

        // dst 中的应用级目录（合并模式下必须保留）
        std::fs::create_dir_all(dst.join("logs")).unwrap();
        std::fs::write(dst.join("logs").join("app.log"), "log").unwrap();
        std::fs::create_dir_all(dst.join("app_resources")).unwrap();
        std::fs::write(dst.join("app_resources").join("f.txt"), "f").unwrap();

        // 合并迁移：不清空 dst
        migrate_vault_data(&src, &dst, false).unwrap();

        // Vault 数据应合并到 dst 顶层
        assert!(dst.join("accounts.json").exists());
        assert!(dst.join("acc_1").join("config.json").exists());

        // 应用级目录应保留，且 src 本身不被删除
        assert!(dst.join("logs").join("app.log").exists());
        assert!(dst.join("app_resources").join("f.txt").exists());
        assert!(src.join("accounts.json").exists(), "源目录不应被清空");
    }

    /// 与上面合并模式对称：clear_dst=true 时仍会清空目标顶层（切换 SAF 场景），
    /// 确保新增参数没有破坏既有清空语义。
    #[test]
    fn test_migrate_vault_data_clear_dst_still_clears_dst() {
        let src = tempfile::TempDir::new().unwrap();
        let dst = tempfile::TempDir::new().unwrap();
        std::fs::write(src.path().join("accounts.json"), "[]").unwrap();

        // dst 中残留旧数据
        std::fs::write(dst.path().join("old.db"), "old").unwrap();
        std::fs::create_dir_all(dst.path().join("old_dir")).unwrap();
        std::fs::write(dst.path().join("old_dir").join("x.txt"), "x").unwrap();

        migrate_vault_data(src.path(), dst.path(), true).unwrap();

        // 清空模式：旧数据被清除，新数据写入
        assert!(!dst.path().join("old.db").exists());
        assert!(!dst.path().join("old_dir").exists());
        assert!(dst.path().join("accounts.json").exists());
    }
}
