// P030: SAF 配置文件 IO 与 Vault 初始化辅助从 app_state.rs 拆分（对齐报告指引
// state/saf_config.rs）。全部以 impl AppState 扩展方法形式迁移——外部调用点
// （AppState::write_saf_config_to_remote、Self::load_saved_saf_uri 等）零改动。
use crate::attachment_import_plugin::AttachmentImportPluginHandle;
use crate::fs::normalize_path;
use crate::fs::saf_sync_driver::TauriSafSyncDriver;
use solosoul_core::vault_file_system::{SafVaultFileSystem, VaultFileSystem};
use solosoul_core::VaultService;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use super::app_state::AppState;
use tauri::Manager;

/// `.solosoul_config` 文件名（存放在 SAF 目录根，用于重装后自动发现）。
const SAF_CONFIG_FILE: &str = ".solosoul_config";

impl AppState {
    fn app_config_path(data_dir: &std::path::Path) -> PathBuf {
        data_dir.join("app_config.json")
    }

    pub(crate) fn load_saved_saf_uri(data_dir: &std::path::Path) -> Option<String> {
        let path = Self::app_config_path(data_dir);
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

    /// 写入 .solosoul_config 到本地临时目录，并通过 SAF 同步写入远端。
    /// 使卸载重装后用户选择相同目录时能自动恢复 SAF URI。
    pub(crate) fn write_saf_config_to_remote(
        temp_dir: &Path,
        saf_uri: &str,
        sync_driver: Arc<dyn solosoul_core::vault_file_system::SafSyncDriver>,
    ) -> Result<(), String> {
        let config_path = temp_dir.join(SAF_CONFIG_FILE);
        let config = serde_json::json!({
            "version": 1,
            "saf_tree_uri": saf_uri,
            "created_at": Self::now_rfc3339(),
        });
        std::fs::write(
            &config_path,
            serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?,
        )
        .map_err(|e| format!("写入 .solosoul_config 失败: {e}"))?;

        // 通过完整的 SAF 文件系统同步，确保 .solosoul_config 被上传到远端
        let fs = SafVaultFileSystem::new(saf_uri.to_string(), temp_dir.to_path_buf(), sync_driver);
        fs.sync_to_remote()?;
        tracing::info!("[AppState] .solosoul_config written and synced to SAF");
        Ok(())
    }

    /// 从本地临时目录读取 .solosoul_config，提取 saf_tree_uri。
    pub(crate) fn read_saf_config_uri(temp_dir: &Path) -> Option<String> {
        let config_path = temp_dir.join(SAF_CONFIG_FILE);
        if !config_path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(&config_path).ok()?;
        let config: serde_json::Value = serde_json::from_str(&content).ok()?;
        config
            .get("saf_tree_uri")
            .and_then(|v| v.as_str())
            .map(String::from)
    }

    /// ISO 8601 格式时间戳，替代 chrono crate 依赖。
    fn now_rfc3339() -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = now.as_secs();
        let days = secs / 86400;
        let time_secs = secs % 86400;
        let hours = time_secs / 3600;
        let minutes = (time_secs % 3600) / 60;
        let seconds = time_secs % 60;

        let mut y = 1970i64;
        let mut remaining_days = days as i64;
        loop {
            let days_in_year = if Self::is_leap_year(y) { 366 } else { 365 };
            if remaining_days < days_in_year {
                break;
            }
            remaining_days -= days_in_year;
            y += 1;
        }
        let month_days = if Self::is_leap_year(y) {
            [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        } else {
            [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        };
        let mut m = 1usize;
        for days_in_month in month_days {
            if remaining_days < days_in_month {
                break;
            }
            remaining_days -= days_in_month;
            m += 1;
        }
        let d = remaining_days + 1;

        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}+00:00",
            y, m, d, hours, minutes, seconds
        )
    }

    fn is_leap_year(y: i64) -> bool {
        (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
    }

    /// 保存或删除 SAF tree URI 配置。
    pub(crate) fn save_saf_uri(
        data_dir: &std::path::Path,
        uri: Option<&str>,
    ) -> Result<(), String> {
        let path = Self::app_config_path(data_dir);
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

    /// 尝试以 SAF 模式初始化 VaultService。
    pub(crate) fn try_init_saf_vault(
        handle: &tauri::AppHandle,
        data_dir: &std::path::Path,
        uri: &str,
    ) -> Result<VaultService, anyhow::Error> {
        tracing::info!("[AppState] SAF init: creating SafVaultFileSystem...");
        let temp_dir = data_dir.join("saf_vault_temp");
        std::fs::create_dir_all(&temp_dir)?;

        let sync_driver = Arc::new(TauriSafSyncDriver::<tauri::Wry>::new(handle.clone()));

        let fs = Arc::new(SafVaultFileSystem::new(
            uri.to_string(),
            temp_dir.clone(),
            sync_driver,
        ));

        tracing::info!("[AppState] SAF init: creating VaultService...");
        let svc = VaultService::try_with_file_system(temp_dir, fs)
            .map_err(super::root_tasks::ownership_error)?;

        tracing::info!("[AppState] SAF init: loading accounts...");
        svc.load_accounts();

        tracing::info!("[AppState] SAF init: success");
        Ok(svc)
    }

    /// 用本地 App-private 目录初始化 VaultService。
    pub(crate) fn try_init_local_vault(
        data_dir: &std::path::Path,
    ) -> Result<VaultService, anyhow::Error> {
        tracing::info!(
            "[AppState] local vault init: data_dir={}",
            data_dir.display()
        );
        let svc = VaultService::try_with_base_path(data_dir.to_path_buf())
            .map_err(super::root_tasks::ownership_error)?;
        svc.load_accounts();
        tracing::info!(
            "[AppState] loaded accounts count: {}",
            svc.list_accounts().len()
        );
        Ok(svc)
    }

    /// 创建一个占位用的 VaultService（首次启动尚未选择目录时使用）。
    /// 占位目录使用应用私有目录下的 `.uninitialized_vault`。
    pub(super) fn placeholder_vault(
        data_dir: &std::path::Path,
    ) -> Result<VaultService, anyhow::Error> {
        let placeholder_dir = data_dir.join(".uninitialized_vault");
        std::fs::create_dir_all(&placeholder_dir)?;
        let svc = VaultService::try_with_base_path(placeholder_dir)
            .map_err(super::root_tasks::ownership_error)?;
        svc.load_accounts();
        Ok(svc)
    }

    /// 失效 SAF 的缓存降级先取得真实源、目标 owner，再执行合并。
    /// 锁文件保持原身份；任何 journal 继续绑定原缓存，不能迁移。
    pub(crate) fn try_init_fallback_vault(data_dir: &Path) -> Result<VaultService, anyhow::Error> {
        use solosoul_core::import_activity::{
            begin_owned_root_maintenance, ensure_import_root_movable, has_bound_imports,
        };
        use solosoul_core::vault_file_system::LocalVaultFileSystem;
        use solosoul_vault::root_owner::VaultRootOwner;
        let cache = data_dir.join("saf_vault_temp");
        if !cache.exists() {
            return Self::try_init_local_vault(data_dir);
        }
        let source = Self::try_init_local_vault(&cache)?;
        let source_owner = source.root_owner();
        let _source_gate = begin_owned_root_maintenance(Arc::clone(&source_owner))
            .map_err(super::root_tasks::ownership_error)?;
        // 无法核对 journal 时保留原根，沿用 RF022 的恢复规则。
        if has_bound_imports(source_owner.root()).unwrap_or(true) {
            return Ok(source);
        }
        let target =
            VaultRootOwner::acquire(data_dir).map_err(super::root_tasks::ownership_error)?;
        let _target_gate = begin_owned_root_maintenance(Arc::clone(&target))
            .map_err(super::root_tasks::ownership_error)?;
        ensure_import_root_movable(target.root()).map_err(super::root_tasks::ownership_error)?;
        match crate::commands::vault_directory::migrate_vault_data(
            source_owner.root(),
            target.root(),
            false,
        ) {
            Ok(()) => {
                // 只清理复制成功的缓存内容，不能删除仍持有的 .lock 文件。
                if let Err(error) =
                    crate::commands::vault_directory::clear_target_dir(source_owner.root())
                {
                    tracing::warn!("[AppState] failed to clean up SAF temp cache: {error}");
                }
            }
            Err(error) => {
                tracing::error!("[AppState] temp cache migration failed (non-fatal): {error}")
            }
        }
        let fs = Arc::new(LocalVaultFileSystem::new(target.root().to_path_buf()));
        let service = VaultService::try_with_root_owner(target, fs)
            .map_err(super::root_tasks::ownership_error)?;
        service.load_accounts();
        Ok(service)
    }

    /// 移动端 VaultService 初始化：优先 SAF 目录（失效则降级本地），
    /// 首次启动使用占位目录；桌面端返回空 VaultService。
    pub(crate) fn init_vault_service(
        handle: &tauri::AppHandle,
    ) -> Result<Arc<RwLock<VaultService>>, anyhow::Error> {
        if !cfg!(mobile) {
            return Ok(Arc::new(RwLock::new(
                VaultService::try_new().map_err(super::root_tasks::ownership_error)?,
            )));
        }

        let data_dir = normalize_path(
            &handle
                .path()
                .resolve(".", tauri::path::BaseDirectory::Data)
                .map_err(|e| anyhow::anyhow!("无法解析应用数据目录: {e}"))?,
        );
        tracing::info!("[AppState] mobile data_dir: {}", data_dir.display());

        let saved_uri = Self::load_saved_saf_uri(&data_dir);
        let Some(ref uri) = saved_uri else {
            // 首次启动：尚未选择目录，使用占位 VaultService，
            // 等 onboarding 调用 initialize_vault 后再热替换。
            tracing::info!(
                "[AppState] first launch: using placeholder vault until directory is selected"
            );
            return Ok(Arc::new(RwLock::new(Self::placeholder_vault(&data_dir)?)));
        };
        tracing::info!("[AppState] found saved SAF URI: {uri}");

        // 先检查 SAF URI 是否仍然可访问（防止用户手动删除外部目录后以失效状态启动）。
        // check_vault_dir_access 在 Android 上通过 Kotlin 插件查询 ContentResolver
        // 判断目录是否存在；非 Android 平台返回 false，由下层 cfg 守卫。
        let is_valid = {
            let plugin_handle = handle.state::<AttachmentImportPluginHandle<tauri::Wry>>();
            plugin_handle.check_vault_dir_access(uri).unwrap_or(false)
        };

        if !is_valid {
            tracing::warn!(
                "[AppState] saved SAF URI is no longer accessible (dir deleted or revoked), falling back to local vault"
            );
            // 取消可能已调度的 WorkManager 兜底同步，避免旧 URI 持续重试。
            let _ = handle
                .state::<AttachmentImportPluginHandle<tauri::Wry>>()
                .cancel_fallback_sync();
            // 注意：故意【不】清除已失效的 SAF URI——
            // 前端登录后会调用 vault_check_directory 检测失效并弹窗 + 横幅提示
            // 用户重新选择目录（每次启动都应提醒）；auto-sync 每 30s 也会发射
            // saf-auth-revoked 事件维持横幅。若在此清空 URI，前端将无从得知
            // 外部目录已丢失，表现为「目录被删除后重启无任何提示」的回归。
            // 用户重新选择目录（vault_set_directory 保存新 URI）或主动切回本地
            // （保存 None）后，此路径自然退出。

            Self::try_init_fallback_vault(&data_dir).map(|svc| Arc::new(RwLock::new(svc)))
        } else {
            match Self::try_init_saf_vault(handle, &data_dir, uri) {
                Ok(svc) => {
                    tracing::info!("[AppState] SAF vault initialized successfully");
                    Ok(Arc::new(RwLock::new(svc)))
                }
                Err(e) => {
                    tracing::error!(
                        "[AppState] SAF vault init FAILED (falling back to local): {:#}",
                        e
                    );
                    // 目录锁/owner 失败必须停止，不能换根继续写入。
                    Err(e)
                }
            }
        }
    }
}

#[cfg(test)]
mod rf905_tests;
