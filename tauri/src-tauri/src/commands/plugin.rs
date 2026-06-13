//! 插件系统 Tauri Commands

use crate::plugin::{
    MarketPluginInfo, PluginAuditEntry, PluginEvent, PluginInstallResult, PluginManifest,
    PluginResult, PluginSessionInfo, PluginTier,
};
use crate::state::AppState;
use std::collections::HashMap;
use tauri::{command, ipc::Channel, State};

#[command]
pub async fn plugin_list_all(
    state: State<'_, AppState>,
    tier: Option<String>,
) -> Result<Vec<MarketPluginInfo>, String> {
    let tier_filter = match tier {
        Some(t) => Some(PluginTier::parse(&t).ok_or_else(|| format!("非法 tier: {}", t))?),
        None => None,
    };
    state
        .plugin_manager
        .list_all(tier_filter)
        .map_err(|e| e.to_string())
}

#[command]
pub async fn plugin_list_installed(
    state: State<'_, AppState>,
) -> Result<Vec<PluginManifest>, String> {
    state
        .plugin_manager
        .list_installed()
        .map_err(|e| e.to_string())
}

#[command]
pub async fn plugin_install(
    state: State<'_, AppState>,
    plugin_id: String,
    version: String,
) -> Result<PluginInstallResult, String> {
    state
        .plugin_manager
        .install_from_registry(&plugin_id, &version)
        .map_err(|e| e.to_string())
}

#[command]
pub async fn plugin_update(
    state: State<'_, AppState>,
    plugin_id: String,
) -> Result<PluginInstallResult, String> {
    state
        .plugin_manager
        .update(&plugin_id)
        .map_err(|e| e.to_string())
}

#[command]
pub async fn plugin_uninstall(state: State<'_, AppState>, plugin_id: String) -> Result<(), String> {
    state
        .plugin_manager
        .uninstall(&plugin_id)
        .map_err(|e| e.to_string())
}

#[command]
pub async fn plugin_run(
    state: State<'_, AppState>,
    plugin_id: String,
    params: HashMap<String, String>,
    channel: Channel<PluginEvent>,
) -> Result<PluginResult, String> {
    let (vault_store, account_id) = {
        let svc = state.vault_service.read().unwrap();
        let vault_store = svc.get_vault_store();
        let account_id = svc.get_current_account();
        (vault_store, account_id)
    };

    state
        .plugin_manager
        .run(&plugin_id, params, channel, vault_store, account_id)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn plugin_consent_response(
    state: State<'_, AppState>,
    request_id: String,
    approved: bool,
    value: Option<String>,
) -> Result<(), String> {
    state
        .plugin_manager
        .consent_response(&request_id, approved, value)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn plugin_dialog_response(
    state: State<'_, AppState>,
    request_id: String,
    value: Option<String>,
) -> Result<(), String> {
    state
        .plugin_manager
        .dialog_response(&request_id, value)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn plugin_list_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<PluginSessionInfo>, String> {
    state
        .plugin_manager
        .list_sessions()
        .map_err(|e| e.to_string())
}

#[command]
pub async fn plugin_audit_log(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<PluginAuditEntry>, String> {
    state
        .plugin_manager
        .audit_log(limit)
        .map_err(|e| e.to_string())
}

#[command]
pub async fn plugin_update_registry(state: State<'_, AppState>) -> Result<(), String> {
    state
        .plugin_manager
        .update_registry()
        .await
        .map_err(|e| e.to_string())
}
