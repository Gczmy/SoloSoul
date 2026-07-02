//! 插件系统 Tauri Commands

use crate::commands::{current_account_optional, vault_handle};
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
pub async fn plugin_list_attachments(state: State<'_, AppState>) -> Result<String, String> {
    let vault_store = vault_handle(&state)?;
    let account_id = current_account_optional(&state).ok_or("未选择账户")?;
    let resolver = crate::plugin::FieldResolver::with_vault(vault_store, account_id, vec![]);
    resolver.list_attachments().map_err(|e| e.to_string())
}

/// 从 PluginManifest 的 contracts 中提取 (type_id, role_id, default_property_id) 元组列表。
fn extract_binding_candidates(
    manifest: &PluginManifest,
) -> Vec<(String, String, String)> {
    let mut candidates = Vec::new();
    for contract in &manifest.contracts {
        for role in &contract.roles {
            if let Some(ref default_pid) = role.default_property_id {
                candidates.push((
                    contract.type_id.clone(),
                    role.role_id.clone(),
                    default_pid.clone(),
                ));
            }
        }
    }
    candidates
}

#[command]
pub async fn plugin_install(
    state: State<'_, AppState>,
    plugin_id: String,
    version: String,
) -> Result<PluginInstallResult, String> {
    let result = state
        .plugin_manager
        .install_from_registry(&plugin_id, &version)
        .await
        .map_err(|e| e.to_string())?;

    // 安装成功后，对已解锁的 Vault 执行种子模板 contract_bindings 迁移
    if let Ok(vault) = vault_handle(&state) {
        if let Some(account_id) = current_account_optional(&state) {
            if let Ok(installed) = state.plugin_manager.list_installed() {
                if let Some(manifest) = installed.iter().find(|m| m.id == plugin_id) {
                    let candidates = extract_binding_candidates(manifest);
                    if !candidates.is_empty() {
                        match solosoul_core::template_service::migrate_contract_bindings(
                            &vault,
                            &account_id,
                            &candidates,
                        ) {
                            Ok(count) => {
                                tracing::info!(
                                    "Plugin install: migrated {} seed template field bindings for {}",
                                    count,
                                    plugin_id
                                );
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Plugin install: seed template migration failed for {}: {}",
                                    plugin_id,
                                    e
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(result)
}

#[command]
pub async fn plugin_update(
    state: State<'_, AppState>,
    plugin_id: String,
) -> Result<PluginInstallResult, String> {
    state
        .plugin_manager
        .update(&plugin_id)
        .await
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
    let vault_store = vault_handle(&state).ok();
    let account_id = current_account_optional(&state);

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
