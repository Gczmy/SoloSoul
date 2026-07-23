use crate::fs::saf_sync_driver::TauriSafSyncDriver;
use crate::plugin::PluginManager;
use solosoul_core::vault_file_system::{SafVaultFileSystem, VaultFileSystem};
use solosoul_core::VaultService;
use solosoul_sync::SyncService;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
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
                    // 仅在本地临时目录没有账户数据时从 SAF 拉取，
                    // 避免每次启动都阻塞应用启动。
                    if !temp_dir.join("accounts.json").exists() {
                        if let Err(e) = fs.sync_from_remote() {
                            tracing::warn!("[AppState] 首次从 SAF 同步失败: {e}");
                        }
                    }
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
}
