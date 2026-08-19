//! 插件系统 Tauri Commands

use crate::commands::{current_account_optional, vault_handle};
use crate::plugin::{
    MarketPluginInfo, PluginAuditEntry, PluginEvent, PluginInstallResult, PluginManifest,
    PluginResult, PluginSession, PluginTier,
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
    let resolver = solosoul_plugin::FieldResolver::with_vault(vault_store, account_id, vec![]);
    resolver.list_attachments().map_err(|e| e.to_string())
}

/// 从 PluginManifest 的 contracts 中提取 (type_id, role_id, default_property_id) 元组列表。
fn extract_binding_candidates(manifest: &PluginManifest) -> Vec<(String, String, String)> {
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

/// 安装成功后，对已解锁的 Vault 执行种子模板 contract_bindings 迁移。
/// 任一前置条件（Vault 未解锁 / 无账户 / 插件未在已安装列表）不满足或迁移失败
/// 仅告警，不阻断安装主流程（与原来的 if-let 链语义一致）。
fn migrate_seed_bindings(state: &AppState, plugin_id: &str) {
    let Ok(vault) = vault_handle(state) else {
        return;
    };
    let Some(account_id) = current_account_optional(state) else {
        return;
    };
    let Ok(installed) = state.plugin_manager.list_installed() else {
        return;
    };
    let Some(manifest) = installed.iter().find(|m| m.id == plugin_id) else {
        return;
    };
    let candidates = extract_binding_candidates(manifest);
    if candidates.is_empty() {
        return;
    }
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

    state.auto_sync.trigger_debounce();
    state.device_auto_sync.trigger_data_change();

    // 安装成功后，对已解锁的 Vault 执行种子模板 contract_bindings 迁移
    migrate_seed_bindings(&state, &plugin_id);

    Ok(result)
}

#[command]
pub async fn plugin_update(
    state: State<'_, AppState>,
    plugin_id: String,
) -> Result<PluginInstallResult, String> {
    let result = state
        .plugin_manager
        .update(&plugin_id)
        .await
        .map_err(|e| e.to_string())?;
    state.auto_sync.trigger_debounce();
    state.device_auto_sync.trigger_data_change();
    Ok(result)
}

#[command]
pub async fn plugin_uninstall(state: State<'_, AppState>, plugin_id: String) -> Result<(), String> {
    state
        .plugin_manager
        .uninstall(&plugin_id)
        .map_err(|e| e.to_string())?;
    state.auto_sync.trigger_debounce();
    state.device_auto_sync.trigger_data_change();
    Ok(())
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
    // P001: 附件静态加密密钥——插件复制附件到工作区前解密。
    let attachment_key = state
        .vault_service
        .read()
        .ok()
        .and_then(|svc| svc.attachment_encryption_key().ok())
        .and_then(|k| k.as_slice().try_into().ok());

    // 将 Tauri IPC Channel 适配为 crate 的 PluginEventSink（P012 方向 B 第④步）
    let sink: std::sync::Arc<dyn crate::plugin::PluginEventSink> =
        std::sync::Arc::new(crate::plugin::TauriChannelSink::new(channel));

    state
        .plugin_manager
        .run(
            &plugin_id,
            params,
            sink,
            vault_store,
            account_id,
            attachment_key,
        )
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
) -> Result<Vec<PluginSession>, String> {
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

/// 校验插件提供的输出文件路径并返回其 canonical 形式（P004/P060 共享）。
///
/// 安全约束：插件返回的 `path` 属于不可信数据。本助手强制校验：
///
/// 1. `output_dir` 必须真实存在且是目录；
/// 2. `path` 必须真实存在且是普通文件；
/// 3. `path` 的 canonical 形式必须位于 `output_dir` 的 canonical 之内（防御纵深，
///    结合前端输出目录选择器，插件无法写穿其运行时的输出目录）。
fn resolve_output_file(output_dir: &str, path: &str) -> Result<std::path::PathBuf, String> {
    let out_dir = std::path::Path::new(output_dir);
    let out_canon = out_dir
        .canonicalize()
        .map_err(|e| format!("无法解析输出目录: {}", e))?;
    if !out_canon.is_dir() {
        return Err("输出目录不存在".to_string());
    }

    let p = std::path::Path::new(path);
    let canon = p
        .canonicalize()
        .map_err(|e| format!("无法解析文件: {}", e))?;
    if !canon.is_file() {
        return Err("输出文件不存在".to_string());
    }
    if !canon.starts_with(&out_canon) {
        return Err("输出文件位于插件输出目录之外，已拒绝访问".to_string());
    }
    Ok(canon)
}

/// 打开插件生成的输出文件（P004）。
///
/// 安全约束：插件返回的 `outputPath` 属于不可信数据，此前前端直接 `open(file://)`
/// 任意路径，恶意插件可诱导用户打开 `.app`/脚本逃逸 WASM 沙箱。本命令通过
/// `resolve_output_file` 校验后，才用系统默认应用打开（`opener` crate，与附件预览一致）。
///
/// 同时 `tauri.conf.json` 的 shell.open 正则已移除 `file://` 与绝对路径项（P032），
/// 即使绕过本命令也无法再经 plugin-shell 打开本地文件。
#[command]
pub fn plugin_open_output_file(output_dir: String, path: String) -> Result<(), String> {
    let canon = resolve_output_file(&output_dir, &path)?;

    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        let _ = canon;
        Err("当前平台暂不支持".to_string())
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        opener::open(&canon).map_err(|e| format!("打开文件失败: {}", e))?;
        Ok(())
    }
}

/// 将插件生成的输出文件复制到用户选择的目标目录（P060）。
///
/// 安全约束：此前前端直接用 plugin-fs `copyFile` 复制插件返回的 `path`，且失败静默吞错。
/// 本命令强制校验：
///
/// 1. 源 `path` 的 canonical 形式必须位于插件声明的 `output_dir` 之内（与 `plugin_open_output_file` 一致）；
/// 2. `file_name` 必须是单一文件名（不含路径分隔符、非 `.`/`..`、非空），
///    防止插件返回的 `fileName` 携带路径遍历写穿用户所选目录；
/// 3. `dest_dir` 必须是真实存在的目录（用户经保存/目录选择对话框提供）。
#[command]
pub fn plugin_copy_output_file(
    output_dir: String,
    path: String,
    dest_dir: String,
    file_name: String,
) -> Result<(), String> {
    let canon = resolve_output_file(&output_dir, &path)?;

    if file_name.is_empty()
        || file_name == "."
        || file_name == ".."
        || file_name.contains('/')
        || file_name.contains('\\')
    {
        return Err("非法文件名".to_string());
    }

    let dir = std::path::Path::new(&dest_dir);
    if !dir.is_dir() {
        return Err("目标目录不存在".to_string());
    }

    let dest = dir.join(&file_name);
    std::fs::copy(&canon, &dest)
        .map(|_| ())
        .map_err(|e| format!("复制文件失败: {}", e))
}
