use crate::services::vault_service::VaultService;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppState {
    pub handle: tauri::AppHandle,
    pub vault_service: Arc<RwLock<VaultService>>,
}

impl AppState {
    pub fn new(handle: tauri::AppHandle) -> Result<Self, anyhow::Error> {
        Ok(Self {
            handle,
            vault_service: Arc::new(RwLock::new(VaultService::new())),
        })
    }

    pub fn app_handle(&self) -> &tauri::AppHandle {
        &self.handle
    }
}
