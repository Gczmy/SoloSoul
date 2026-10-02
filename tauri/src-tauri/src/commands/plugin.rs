//! 插件系统 Tauri Commands

use crate::commands::error::{BackendError, BackendErrorCode as Code, BackendErrorStage as Stage};
use crate::commands::{current_account_optional, vault_handle};
use crate::plugin::errors;
use crate::state::AppState;
use solosoul_plugin::event::PluginEvent;
use solosoul_plugin::install_progress::{PluginInstallPhase, PluginInstallProgress};
use solosoul_plugin::manifest::{
    MarketPluginInfo, PluginAuditEntry, PluginInstallResult, PluginManifest, PluginResult,
    PluginTier,
};
use solosoul_plugin::session::PluginSession;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tauri::{ipc::Channel, Manager, Resource, ResourceId, State, Webview};

/// 取消即丢弃下载 future；同步校验/落盘开始后完成提交，不谎报“取消但已安装”。
struct PluginInstallOperation {
    cancelled: tokio::sync::watch::Sender<bool>,
    started: AtomicBool,
}
impl Resource for PluginInstallOperation {
    fn close(self: Arc<Self>) {
        self.cancelled.send_replace(true);
    }
}
impl PluginInstallOperation {
    fn new() -> Self {
        let (cancelled, _) = tokio::sync::watch::channel(false);
        Self {
            cancelled,
            started: AtomicBool::new(false),
        }
    }
    async fn run_projected<T, E>(
        &self,
        future: impl std::future::Future<Output = Result<T, E>>,
        error: impl Fn(Code) -> E,
    ) -> Result<T, E> {
        if self.started.swap(true, Ordering::AcqRel) {
            return Err(error(Code::PluginInstallAlreadyStarted));
        }
        let mut cancel = self.cancelled.subscribe();
        if *cancel.borrow() {
            return Err(error(Code::PluginInstallCancelled));
        }
        tokio::select! {biased;
            _=cancel.changed()=>Err(error(Code::PluginInstallCancelled)),
            result=future=>result,
        }
    }
    async fn run_typed<T>(
        &self,
        future: impl std::future::Future<Output = Result<T, BackendError>>,
    ) -> Result<T, BackendError> {
        self.run_projected(future, |code| BackendError::new(code).at(Stage::Install))
            .await
    }
    #[cfg(test)]
    async fn run<T>(
        &self,
        future: impl std::future::Future<Output = Result<T, String>>,
    ) -> Result<T, String> {
        self.run_projected(future, |code| match code {
            Code::PluginInstallAlreadyStarted => "安装任务已启动".into(),
            _ => "PLUGIN_INSTALL_CANCELLED".into(),
        })
        .await
    }
}
async fn await_bundled_resources() -> Result<(), BackendError> {
    crate::android_resources::await_ready()
        .await
        .map_err(|error| errors::typed(solosoul_plugin::error::PluginError::RegistryError(error)))
}

static PLUGIN_INSTALL_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// 核心包落盘后还需同步状态/迁移绑定；命令真正完成之前保留最后 2%。
fn emit_install_progress(
    channel: &Channel<PluginInstallProgress>,
    mut progress: PluginInstallProgress,
) {
    if progress.phase == PluginInstallPhase::Completed {
        progress.phase = PluginInstallPhase::Finalizing;
        progress.percent = 98;
    }
    let _ = channel.send(progress);
}
#[tauri::command]
pub fn create_plugin_install(webview: Webview) -> ResourceId {
    webview.resources_table().add(PluginInstallOperation::new())
}

#[tauri::command]
pub async fn plugin_list_all(
    state: State<'_, AppState>,
    tier: Option<String>,
) -> Result<Vec<MarketPluginInfo>, BackendError> {
    let tier_filter = match tier {
        Some(t) => Some(PluginTier::parse(&t).ok_or_else(|| {
            BackendError::caused_by(Code::PluginInvalidArgument, Stage::Validate, t)
        })?),
        None => None,
    };
    await_bundled_resources().await?;
    state
        .plugin_manager
        .list_all(tier_filter)
        .map_err(errors::typed)
}

#[tauri::command]
pub async fn plugin_list_installed(
    state: State<'_, AppState>,
) -> Result<Vec<PluginManifest>, BackendError> {
    state.plugin_manager.list_installed().map_err(errors::typed)
}

#[tauri::command]
pub async fn plugin_list_attachments(state: State<'_, AppState>) -> Result<String, BackendError> {
    let vault_store = vault_handle(&state)
        .map_err(|cause| errors::legacy(Code::PluginReadFailed, Stage::Read, cause))?;
    let account_id = current_account_optional(&state)
        .ok_or_else(|| BackendError::new(Code::VaultLocked).at(Stage::Read))?;
    let resolver =
        solosoul_plugin::FieldResolver::with_vault(vault_store.store_arc(), account_id, vec![]);
    resolver.list_attachments().map_err(errors::typed)
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
            let _ = BackendError::caused_by(Code::PluginStoreFailed, Stage::Write, e);
        }
    }
}

#[tauri::command]
pub async fn plugin_install(
    state: State<'_, AppState>,
    webview: Webview,
    plugin_id: String,
    version: String,
    operation_id: Option<ResourceId>,
    on_progress: Channel<PluginInstallProgress>,
) -> Result<PluginInstallResult, BackendError> {
    let operation = match operation_id {
        Some(id) => webview
            .resources_table()
            .get::<PluginInstallOperation>(id)
            .map_err(|e| {
                BackendError::caused_by(Code::PluginInvalidOperation, Stage::Validate, e)
            })?,
        None => Arc::new(PluginInstallOperation::new()),
    };
    let result = operation
        .run_typed(async {
            await_bundled_resources().await?;
            let _guard = PLUGIN_INSTALL_LOCK.lock().await;
            state
                .plugin_manager
                .install_from_registry_with_progress(&plugin_id, &version, &|progress| {
                    emit_install_progress(&on_progress, progress);
                })
                .await
                .map_err(errors::typed)
        })
        .await?;
    state.auto_sync.trigger_debounce();
    state.device_auto_sync.trigger_data_change();
    migrate_seed_bindings(&state, &plugin_id);
    let _ = on_progress.send(PluginInstallProgress::completed());
    Ok(result)
}

#[tauri::command]
pub async fn plugin_update(
    state: State<'_, AppState>,
    webview: Webview,
    plugin_id: String,
    operation_id: Option<ResourceId>,
    on_progress: Channel<PluginInstallProgress>,
) -> Result<PluginInstallResult, BackendError> {
    let operation = match operation_id {
        Some(id) => webview
            .resources_table()
            .get::<PluginInstallOperation>(id)
            .map_err(|e| {
                BackendError::caused_by(Code::PluginInvalidOperation, Stage::Validate, e)
            })?,
        None => Arc::new(PluginInstallOperation::new()),
    };
    let result = operation
        .run_typed(async {
            await_bundled_resources().await?;
            let _guard = PLUGIN_INSTALL_LOCK.lock().await;
            state
                .plugin_manager
                .update_with_progress(&plugin_id, &|progress| {
                    emit_install_progress(&on_progress, progress);
                })
                .await
                .map_err(errors::typed)
        })
        .await?;
    state.auto_sync.trigger_debounce();
    state.device_auto_sync.trigger_data_change();
    let _ = on_progress.send(PluginInstallProgress::completed());
    Ok(result)
}

#[tauri::command]
pub async fn plugin_uninstall(
    state: State<'_, AppState>,
    plugin_id: String,
) -> Result<(), BackendError> {
    let _guard = PLUGIN_INSTALL_LOCK.lock().await;
    state
        .plugin_manager
        .uninstall(&plugin_id)
        .map_err(errors::typed)?;
    state.auto_sync.trigger_debounce();
    state.device_auto_sync.trigger_data_change();
    Ok(())
}

#[tauri::command]
pub async fn plugin_run(
    state: State<'_, AppState>,
    plugin_id: String,
    params: HashMap<String, String>,
    channel: Channel<PluginEvent>,
) -> Result<PluginResult, BackendError> {
    // 锁定态仍可运行不读取 Vault 的插件；维护拒绝或锁损坏不能降为无 Vault。
    let vault_activity = match vault_handle(&state) {
        Ok(handle) => Some(handle),
        Err(error) if error == "Vault not unlocked" => None,
        Err(error) => {
            return Err(errors::legacy(
                Code::PluginExecutionFailed,
                Stage::Execute,
                error,
            ))
        }
    };
    // capsule 保留到 outer function end，覆盖 capture → Core 派发的间隙。
    let vault_store = vault_activity.as_ref().map(|handle| handle.store_arc());
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
        .map_err(errors::typed)
}

#[tauri::command]
pub async fn plugin_consent_response(
    state: State<'_, AppState>,
    request_id: String,
    approved: bool,
    value: Option<String>,
) -> Result<(), BackendError> {
    state
        .plugin_manager
        .consent_response(&request_id, approved, value)
        .await
        .map_err(errors::typed)
}

#[tauri::command]
pub async fn plugin_dialog_response(
    state: State<'_, AppState>,
    request_id: String,
    value: Option<String>,
) -> Result<(), BackendError> {
    state
        .plugin_manager
        .dialog_response(&request_id, value)
        .await
        .map_err(errors::typed)
}

#[tauri::command]
pub async fn plugin_list_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<PluginSession>, BackendError> {
    state.plugin_manager.list_sessions().map_err(errors::typed)
}

#[tauri::command]
pub async fn plugin_audit_log(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<PluginAuditEntry>, BackendError> {
    state
        .plugin_manager
        .audit_log(limit)
        .map(|entries| entries.into_iter().map(errors::project_audit).collect())
        .map_err(errors::typed)
}

#[tauri::command]
pub async fn plugin_update_registry(state: State<'_, AppState>) -> Result<(), BackendError> {
    await_bundled_resources().await?;
    state
        .plugin_manager
        .update_registry()
        .await
        .map_err(errors::typed)
}

/// 校验插件提供的输出文件路径并返回其 canonical 形式（P004/P060 共享）。
///
/// 安全约束：插件返回的 `path` 属于不可信数据。本助手强制校验：
///
/// 1. `output_dir` 必须真实存在且是目录；
/// 2. `path` 必须真实存在且是普通文件；
/// 3. `path` 的 canonical 形式必须位于 `output_dir` 的 canonical 之内（防御纵深，
///    结合前端输出目录选择器，插件无法写穿其运行时的输出目录）。
fn resolve_output_file_with<E>(
    output_dir: &str,
    path: &str,
    error: impl Fn(Code, Stage, String) -> E,
) -> Result<std::path::PathBuf, E> {
    let out_dir = std::path::Path::new(output_dir);
    let out_canon = out_dir.canonicalize().map_err(|e| {
        error(
            Code::PluginOutputReadFailed,
            Stage::Read,
            format!("无法解析输出目录: {}", e),
        )
    })?;
    if !out_canon.is_dir() {
        return Err(error(
            Code::PluginOutputInvalid,
            Stage::Validate,
            "输出目录不存在".to_string(),
        ));
    }

    let p = std::path::Path::new(path);
    let canon = p.canonicalize().map_err(|e| {
        error(
            Code::PluginOutputReadFailed,
            Stage::Read,
            format!("无法解析文件: {}", e),
        )
    })?;
    if !canon.is_file() {
        return Err(error(
            Code::PluginOutputInvalid,
            Stage::Validate,
            "输出文件不存在".to_string(),
        ));
    }
    if !canon.starts_with(&out_canon) {
        return Err(error(
            Code::PluginOutputDenied,
            Stage::Validate,
            "输出文件位于插件输出目录之外，已拒绝访问".to_string(),
        ));
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
#[tauri::command]
pub fn plugin_open_output_file(output_dir: String, path: String) -> Result<(), BackendError> {
    let canon = resolve_output_file_with(&output_dir, &path, BackendError::caused_by)?;

    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        let _ = canon;
        Err(BackendError::new(Code::PluginUnsupported).at(Stage::Task))
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        opener::open(&canon)
            .map_err(|e| BackendError::caused_by(Code::PluginOutputOpenFailed, Stage::Task, e))?;
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
#[tauri::command]
pub fn plugin_copy_output_file(
    output_dir: String,
    path: String,
    dest_dir: String,
    file_name: String,
) -> Result<(), BackendError> {
    let canon = resolve_output_file_with(&output_dir, &path, BackendError::caused_by)?;

    if file_name.is_empty()
        || file_name == "."
        || file_name == ".."
        || file_name.contains('/')
        || file_name.contains('\\')
    {
        return Err(BackendError::new(Code::PluginOutputInvalid).at(Stage::Validate));
    }

    let dir = std::path::Path::new(&dest_dir);
    if !dir.is_dir() {
        return Err(BackendError::new(Code::PluginOutputInvalid).at(Stage::Validate));
    }

    let dest = dir.join(&file_name);
    std::fs::copy(&canon, &dest)
        .map(|_| ())
        .map_err(|e| BackendError::caused_by(Code::PluginOutputWriteFailed, Stage::Write, e))
}

#[cfg(test)]
mod install_operation_tests {
    use super::*;

    #[tokio::test]
    async fn cancel_before_start_never_polls_download() {
        let operation = Arc::new(PluginInstallOperation::new());
        Resource::close(operation.clone());
        let polled = AtomicBool::new(false);
        let result = operation
            .run(async {
                polled.store(true, Ordering::SeqCst);
                Ok(())
            })
            .await;
        assert_eq!(result.unwrap_err(), "PLUGIN_INSTALL_CANCELLED");
        assert!(!polled.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn close_aborts_pending_download_and_disallows_duplicate_start() {
        let operation = Arc::new(PluginInstallOperation::new());
        let (started, ready) = tokio::sync::oneshot::channel();
        let (result, ()) = tokio::join!(
            operation.run(async {
                let _ = started.send(());
                std::future::pending::<Result<(), String>>().await
            }),
            async {
                ready.await.unwrap();
                Resource::close(operation.clone());
            }
        );
        assert_eq!(result.unwrap_err(), "PLUGIN_INSTALL_CANCELLED");
        assert!(operation
            .run(async { Ok(()) })
            .await
            .unwrap_err()
            .contains("已启动"));
    }

    #[tokio::test]
    async fn cancellation_after_commit_does_not_report_false_failure() {
        let operation = Arc::new(PluginInstallOperation::new());
        let result = operation
            .run(async {
                Resource::close(operation.clone());
                Ok("installed")
            })
            .await;
        assert_eq!(result.unwrap(), "installed");
    }
}

#[cfg(test)]
#[path = "plugin/rf306_tests.rs"]
mod rf306_tests;

#[cfg(test)]
#[path = "plugin/rf320_tests.rs"]
mod rf320_tests;
