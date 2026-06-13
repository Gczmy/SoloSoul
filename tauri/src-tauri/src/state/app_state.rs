use crate::plugin::PluginManager;
use crate::services::sync_service::SyncService;
use crate::services::vault_service::VaultService;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppState {
    pub handle: tauri::AppHandle,
    pub vault_service: Arc<RwLock<VaultService>>,
    pub sync_service: Arc<SyncService>,
    pub plugin_manager: Arc<PluginManager>,
}

impl AppState {
    pub fn new(handle: tauri::AppHandle) -> Result<Self, anyhow::Error> {
        let vault_service = Arc::new(RwLock::new(VaultService::new()));
        let sync_service = Arc::new(SyncService::new(vault_service.clone()));
        let plugin_manager = Arc::new(PluginManager::new_with_app_handle(&handle)?);
        Ok(Self {
            handle,
            vault_service,
            sync_service,
            plugin_manager,
        })
    }

    pub fn app_handle(&self) -> &tauri::AppHandle {
        &self.handle
    }
}
