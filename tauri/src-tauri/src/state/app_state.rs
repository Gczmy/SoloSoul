use crate::services::sync_service::SyncService;
use crate::services::vault_service::VaultService;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppState {
    pub handle: tauri::AppHandle,
    pub vault_service: Arc<RwLock<VaultService>>,
    pub sync_service: Arc<SyncService>,
}

impl AppState {
    pub fn new(handle: tauri::AppHandle) -> Result<Self, anyhow::Error> {
        let vault_service = Arc::new(RwLock::new(VaultService::new()));
        let sync_service = Arc::new(SyncService::new(vault_service.clone()));
        Ok(Self {
            handle,
            vault_service,
            sync_service,
        })
    }

    pub fn app_handle(&self) -> &tauri::AppHandle {
        &self.handle
    }
}
