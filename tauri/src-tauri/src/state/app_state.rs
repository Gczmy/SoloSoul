use crate::attachment_import_plugin::AttachmentImportPluginHandle;
use crate::fs::saf_sync_driver::TauriSafSyncDriver;
use crate::plugin::PluginManager;
use solosoul_core::vault_file_system::SafVaultFileSystem;
use solosoul_core::VaultService;
use solosoul_sync::SyncService;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tauri::Emitter;
use tauri::Manager;

pub struct AppState {
    pub handle: tauri::AppHandle,
    pub vault_service: Arc<RwLock<VaultService>>,
    pub sync_service: Arc<SyncService>,
    pub plugin_manager: Arc<PluginManager>,
}

impl AppState {
    /// 解析应用级配置路径（app_config.json），与 Vault 数据目录分离，
    /// 以便在创建 VaultService 之前读取 SAF 目录等配置。
    fn app_config_path(data_dir: &std::path::Path) -> PathBuf {
        data_dir.join("app_config.json")
    }

    /// 读取 app_config.json 中保存的 SAF tree URI（如果存在）。
    fn load_saved_saf_uri(data_dir: &std::path::Path) -> Option<String> {
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

    pub fn new(handle: tauri::AppHandle) -> Result<Self, anyhow::Error> {
        // 移动端使用 Tauri 应用私有数据目录；桌面端沿用 VaultService 默认路径
        let vault_service = if cfg!(mobile) {
            let data_dir = handle
                .path()
                .resolve(".", tauri::path::BaseDirectory::Data)
                .map_err(|e| anyhow::anyhow!("无法解析应用数据目录: {e}"))?;
            tracing::info!("[AppState] mobile data_dir: {}", data_dir.display());

            let svc = match Self::load_saved_saf_uri(&data_dir) {
                Some(uri) => {
                    tracing::info!("[AppState] using SAF vault directory: {uri}");
                    let temp_dir = data_dir.join("saf_vault_temp");
                    std::fs::create_dir_all(&temp_dir)?;
                    let sync_driver =
                        Arc::new(TauriSafSyncDriver::<tauri::Wry>::new(handle.clone()));
                    let fs = Arc::new(SafVaultFileSystem::new(uri, temp_dir.clone(), sync_driver));
                    // 首次同步延迟到 init_saf_sync 中异步执行，不阻塞启动。
                    VaultService::with_file_system(temp_dir, fs)
                }
                None => {
                    tracing::info!("[AppState] using local app-private vault directory");
                    VaultService::with_base_path(data_dir)
                }
            };
            svc.load_accounts();
            tracing::info!(
                "[AppState] loaded accounts count: {}",
                svc.list_accounts().len()
            );
            Arc::new(RwLock::new(svc))
        } else {
            Arc::new(RwLock::new(VaultService::new()))
        };
        let sync_service = Arc::new(SyncService::new(vault_service.clone()));

        // 插件管理器初始化失败不阻止应用启动，降级为无插件模式
        let plugin_manager = match PluginManager::new_with_app_handle(&handle) {
            Ok(pm) => Arc::new(pm),
            Err(e) => {
                tracing::warn!(
                    "[AppState] PluginManager 初始化失败，将以无插件模式运行: {:#}",
                    e
                );
                // 回退：使用不依赖市场目录的默认构造（用于开发或子模块未初始化场景）
                let fallback = PluginManager::new().map_err(|fallback_err| {
                    anyhow::anyhow!(
                        "PluginManager 初始化失败: {:#}, 回退构造也失败: {:#}",
                        e,
                        fallback_err
                    )
                })?;
                Arc::new(fallback)
            }
        };

        Ok(Self {
            handle,
            vault_service,
            sync_service,
            plugin_manager,
        })
    }

    /// 判断当前是否使用了 SAF 远程存储。
    pub fn has_saf_vault(&self) -> bool {
        self.vault_service
            .read()
            .map(|svc| svc.is_remote_storage())
            .unwrap_or(false)
    }

    /// 启动后台自动同步任务。
    /// 每 30 秒检查 dirty flag，有脏数据时自动同步到 SAF。
    /// 该任务不会阻塞应用启动。
    ///
    /// 注意：实际的 SAF 同步操作（JNI 调用）在 `spawn_blocking` 中执行，
    /// 避免在 async runtime worker 线程上执行阻塞的 Kotlin 桥接调用。
    pub fn start_auto_sync_task(&self) -> tokio::task::JoinHandle<()> {
        let svc = self.vault_service.clone();
        let app = self.handle.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                let need_sync = {
                    let read_guard = match svc.read() {
                        Ok(g) => g,
                        Err(_) => continue,
                    };
                    read_guard.is_remote_storage()
                };
                if !need_sync {
                    continue;
                }
                // 在 spawn_blocking 中执行阻塞的 JNI 同步操作，
                // 避免在 tokio worker 线程上调用 run_mobile_plugin。
                // 注意：release 构建使用 panic = "abort"，此处 JoinError 分支
                // 在生产构建中不可达（任何 panic 直接 abort 进程）。
                let svc_clone = svc.clone();
                let result = tokio::task::spawn_blocking(move || -> Result<(), String> {
                    let read_guard = svc_clone
                        .read()
                        .map_err(|_| "Vault service lock poisoned".to_string())?;
                    read_guard.sync_if_dirty()
                })
                .await;
                match result {
                    Ok(Ok(())) => {
                        let _ = app.emit(
                            "sync-progress",
                            serde_json::json!({"phase": "auto_sync", "current": 1, "total": 1}),
                        );
                        tracing::debug!("[auto-sync] sync_if_dirty completed");
                    }
                    Ok(Err(e)) => {
                        tracing::warn!("[auto-sync] sync_if_dirty failed: {e}");
                    }
                    Err(join_err) => {
                        // 仅 debug 构建可达：release（panic=abort）下 panic 直接 abort 进程
                        tracing::error!("[auto-sync] sync task panicked: {join_err}");
                    }
                }
            }
        })
    }

    pub async fn init_saf_sync(&self) -> Result<(), String> {
        let svc = self.vault_service.clone();
        let app_handle = self.handle.clone();

        // 解析数据目录并读取保存的 SAF URI（async 侧执行，path() 不阻塞）
        let data_dir = app_handle
            .path()
            .resolve(".", tauri::path::BaseDirectory::Data)
            .map_err(|e| format!("无法解析应用数据目录: {e}"))?;
        let saved_uri = Self::load_saved_saf_uri(&data_dir);

        // 前置检查：验证 SAF URI 仍然可访问（在 spawn_blocking 外部执行，
        // 避免在阻塞线程中调用 JNI 桥接）
        if let Some(ref uri) = saved_uri {
            let plugin_handle = app_handle.state::<AttachmentImportPluginHandle<tauri::Wry>>();
            let valid = plugin_handle
                .check_vault_dir_access(uri)
                .unwrap_or(false);
            if !valid {
                tracing::error!(
                    "[AppState] SAF directory access revoked for {}, skipping initial sync",
                    uri
                );
                return Err(
                    "SAF 目录访问权限已被撤销，请前往「设置 > 保险库目录」重新选择目录。"
                        .to_string(),
                );
            }
        }

        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let read_guard =
                svc.read().map_err(|_| "Vault service lock poisoned".to_string())?;
            let accounts_path = read_guard.base_path().join("accounts.json");
            // 只在本地临时目录已有账户数据时才跳过同步
            if accounts_path.exists() {
                tracing::info!(
                    "[AppState] SAF temp dir already has accounts.json, skipping initial sync"
                );
                return Ok(());
            }
            read_guard.sync_from_remote()?;
            tracing::info!("[AppState] SAF initial sync completed");
            Ok(())
        })
        .await
        .map_err(|e| format!("SAF sync task panicked: {e}"))?
    }
}
