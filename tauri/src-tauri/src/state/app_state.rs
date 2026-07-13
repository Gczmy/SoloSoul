use crate::plugin::PluginManager;
use solosoul_core::VaultService;
use solosoul_sync::SyncService;
use std::sync::{Arc, RwLock};
use tauri::Manager;

pub struct AppState {
    pub handle: tauri::AppHandle,
    pub vault_service: Arc<RwLock<VaultService>>,
    pub sync_service: Arc<SyncService>,
    pub plugin_manager: Arc<PluginManager>,
}

impl AppState {
    pub fn new(handle: tauri::AppHandle) -> Result<Self, anyhow::Error> {
        // 移动端使用 Tauri 应用私有数据目录；桌面端沿用 VaultService 默认路径
        let vault_service = if cfg!(mobile) {
            let data_dir = handle
                .path()
                .resolve(".", tauri::path::BaseDirectory::Data)
                .map_err(|e| anyhow::anyhow!("无法解析应用数据目录: {e}"))?;
            tracing::info!("[AppState] mobile data_dir: {}", data_dir.display());
            let svc = VaultService::with_base_path(data_dir);
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
}
