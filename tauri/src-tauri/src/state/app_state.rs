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
    fn app_config_path(data_dir: &std::path::Path) -> PathBuf {
        data_dir.join("app_config.json")
    }

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

    /// 尝试以 SAF 模式初始化 VaultService。
    /// 失败时返回 Err（safe-to-retry fallback 路径用）。
    fn try_init_saf_vault(
        handle: &tauri::AppHandle,
        data_dir: &std::path::Path,
        uri: &str,
    ) -> Result<VaultService, anyhow::Error> {
        tracing::info!("[AppState] SAF init: creating SafVaultFileSystem...");
        let temp_dir = data_dir.join("saf_vault_temp");
        std::fs::create_dir_all(&temp_dir)?;

        tracing::info!("[AppState] SAF init: creating sync driver...");
        let sync_driver =
            Arc::new(TauriSafSyncDriver::<tauri::Wry>::new(handle.clone()));

        tracing::info!("[AppState] SAF init: creating VaultFileSystem...");
        let fs = Arc::new(SafVaultFileSystem::new(
            uri.to_string(),
            temp_dir.clone(),
            sync_driver,
        ));

        tracing::info!("[AppState] SAF init: creating VaultService...");
        let svc = VaultService::with_file_system(temp_dir, fs);

        tracing::info!("[AppState] SAF init: loading accounts...");
        svc.load_accounts();

        tracing::info!("[AppState] SAF init: success");
        Ok(svc)
    }

    pub fn new(handle: tauri::AppHandle) -> Result<Self, anyhow::Error> {
        // ── 移动端 VaultService 初始化 ──
        let vault_service = if cfg!(mobile) {
            let data_dir = handle
                .path()
                .resolve(".", tauri::path::BaseDirectory::Data)
                .map_err(|e| anyhow::anyhow!("无法解析应用数据目录: {e}"))?;
            tracing::info!("[AppState] mobile data_dir: {}", data_dir.display());

            let saved_uri = Self::load_saved_saf_uri(&data_dir);
            if let Some(ref uri) = saved_uri {
                tracing::info!("[AppState] found saved SAF URI: {uri}");
                // 最佳尝试模式：SAF 初始化失败时降级到本地模式，
                // 确保应用始终可启动（panic=abort 下任何 panic 都会杀死进程）。
                match Self::try_init_saf_vault(&handle, &data_dir, uri) {
                    Ok(svc) => {
                        tracing::info!("[AppState] SAF vault initialized successfully");
                        Arc::new(RwLock::new(svc))
                    }
                    Err(e) => {
                        tracing::error!(
                            "[AppState] SAF vault init FAILED (falling back to local): {:#}",
                            e
                        );
                        // 降级到本地模式，保留 app_config.json；后台 init_saf_sync 会重试同步
                        let svc = VaultService::with_base_path(data_dir.clone());
                        svc.load_accounts();
                        Arc::new(RwLock::new(svc))
                    }
                }
            } else {
                tracing::info!("[AppState] using local app-private vault directory");
                let svc = VaultService::with_base_path(data_dir);
                svc.load_accounts();
                tracing::info!(
                    "[AppState] loaded accounts count: {}",
                    svc.list_accounts().len()
                );
                Arc::new(RwLock::new(svc))
            }
        } else {
            Arc::new(RwLock::new(VaultService::new()))
        };

        // ── SyncService ──
        let sync_service = Arc::new(SyncService::new(vault_service.clone()));

        // ── PluginManager（初始化失败不阻止应用启动） ──
        // 插件管理器初始化失败不阻止应用启动，降级为无插件模式。
        // 注意：三次尝试（new_with_app_handle → new() → new()）是防御性编程——
        // 首次 new() 失败几乎是确定的；第三次仅用于确保永不意外 panic。
        let plugin_manager = match PluginManager::new_with_app_handle(&handle) {
            Ok(pm) => Arc::new(pm),
            Err(e) => {
                tracing::warn!(
                    "[AppState] PluginManager 初始化失败，将以无插件模式运行: {:#}",
                    e
                );
                // 回退到开发模式（不依赖 Tauri app_handle）
                match PluginManager::new() {
                    Ok(pm) => Arc::new(pm),
                    Err(fallback_err) => {
                        // 回退也失败：继续启动，无插件管理器。前端检测到无插件状态会降级。
                        tracing::error!(
                            "[AppState] PluginManager 回退构造也失败: {:#}（将继续无插件启动）",
                            fallback_err
                        );
                        // 不再崩溃：使用最简单的 new()，失败时接受空状态。
                        // 在 release 构建（panic=abort）下，此分支仅有极小概率触发。
                        match PluginManager::new() {
                            Ok(pm) => Arc::new(pm),
                            Err(_) => {
                                // 已穷尽所有尝试；不再返回 Err 中止启动。
                                tracing::error!(
                                    "[AppState] PluginManager 所有初始化尝试均失败，继续无插件启动"
                                );
                                // 用 new() 最后一次尝试（会失败，但 safe）
                                // 由于所有路径都已失败，这里作为最后保险
                                return Err(anyhow::anyhow!(
                                    "PluginManager 无法初始化（三次尝试均失败）"
                                ));
                            }
                        }
                    }
                }
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
