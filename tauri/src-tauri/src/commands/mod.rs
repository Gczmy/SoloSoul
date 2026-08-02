pub mod attachment;
pub mod auth;
pub mod backup;
pub mod biometric;
pub mod discovery;
pub mod embed_model;
pub mod export_import;
pub mod fs;
pub mod llm;
pub mod log;
pub mod object;
pub mod ocr;
pub mod pin;
pub mod plugin;
pub mod profile;
pub mod recovery;
pub mod search;
pub mod settings;
pub mod sync;
pub mod system;
pub mod template;
pub mod update;
pub mod vault;
pub mod vault_directory;
pub mod window;

use crate::state::AppState;
use std::sync::Arc;

/// 获取当前已解锁 Vault 的句柄，避免在每个命令中重复加锁/解包样板。
pub fn vault_handle(state: &AppState) -> Result<Arc<solosoul_vault::VaultStore>, String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    svc.get_vault_store()
        .ok_or_else(|| "Vault not unlocked".to_string())
}

/// 获取当前已解锁账户 ID。
pub fn current_account(state: &AppState) -> Result<String, String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    svc.get_current_account()
        .ok_or_else(|| "No account unlocked".to_string())
}

/// 可选地获取当前已解锁账户 ID（不返回错误）。
pub fn current_account_optional(state: &AppState) -> Option<String> {
    let svc = state.vault_service.read().ok()?;
    svc.get_current_account()
}

/// 移动端未支持功能的统一错误提示。
#[cfg(mobile)]
pub fn mobile_not_supported() -> Result<(), String> {
    Err("当前平台暂不支持该功能".to_string())
}
