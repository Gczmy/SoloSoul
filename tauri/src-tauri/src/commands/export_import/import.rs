use solosoul_core::{VaultService, VaultSession};
use solosoul_vault::{ImportOperationRecord, ImportSourceKind};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use super::*;

// ── Import commands ────────────────────────────────────────────

/// P013: 导入解密明文临时目录前缀。临时目录建于**数据目录内**（0700，与敏感数据
/// 同姿态），不再落系统 temp；前缀固定以便启动时/下次导入前清扫崩溃残留孤儿目录。
/// 目录由 `tempfile::Builder` 生成唯一随机后缀（同前缀并存多个互不冲突）。
pub(crate) fn cleanup_orphan_import_temps(data_dir: &std::path::Path) -> Result<(), String> {
    solosoul_core::export_import::import::cleanup_orphan_import_temps(data_dir)
}

/// P013: 桌面端导入文件路径白名单校验（Desktop/Documents/Downloads + SOLOSOUL_FS_BASE），
/// 拒绝越界路径；移动端文件来自 SAF 选择/应用内路径，不做此校验。
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub(super) fn resolve_import_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    file_path: &str,
) -> Result<PathBuf, String> {
    crate::commands::fs::resolve_allowed_path(app, file_path)
}

#[cfg(any(target_os = "android", target_os = "ios"))]
pub(super) fn resolve_import_path<R: tauri::Runtime>(
    _app: &tauri::AppHandle<R>,
    file_path: &str,
) -> Result<PathBuf, String> {
    Ok(PathBuf::from(file_path))
}

fn validate_import_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    file_path: &str,
) -> Result<(), String> {
    resolve_import_path(app, file_path).map(|_| ())
}

#[tauri::command]
pub async fn import_parse_package<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    file_path: String,
) -> Result<ImportPreview, String> {
    validate_import_path(&app, &file_path)?;
    let fp = file_path.clone();
    let result = tokio::task::spawn_blocking(move || {
        // P201: 统一经 read_manifest_json 读取（含 100MB 大小上限，防 ZIP 炸弹 OOM）
        let v = read_manifest_json(&fp)?;

        let extra_files: Vec<String> = v
            .get("extra_files")
            .and_then(|x| x.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(ImportPreview {
            file_path: fp,
            version: v
                .get("version")
                .and_then(|x| x.as_str())
                .unwrap_or("1.0")
                .to_string(),
            object_count: v.get("object_count").and_then(|x| x.as_u64()).unwrap_or(0) as usize,
            has_attachments: v
                .get("has_attachments")
                .and_then(|x| x.as_bool())
                .unwrap_or(false),
            extra_files,
            export_time: v
                .get("export_time")
                .and_then(|x| x.as_str())
                .map(|x| x.to_string()),
            password_hint: v
                .get("password_hint")
                .and_then(|x| x.as_str())
                .filter(|s| !s.is_empty())
                .map(|x| x.to_string()),
        })
    })
    .await;

    match result {
        Ok(Ok(preview)) => Ok(preview),
        Ok(Err(e)) => Err(e),
        Err(join_err) => Err(format!("Blocking task failed: {}", join_err)),
    }
}

#[tauri::command]
pub async fn import_decrypt_preview<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: State<'_, AppState>,
    file_path: String,
    password: String,
) -> Result<DecryptedImportPreview, String> {
    let password = Zeroizing::new(password);
    let job = PreviewJob::prepare(
        Arc::clone(&state.vault_service),
        file_path,
        password,
        |path| resolve_import_path(&app, path),
    )?;
    run_preview_job(job, PreviewJob::run).await
}

/// RF-026：密码、已授权路径和原会话一起移入 worker，不持同步锁等待解密。
pub(super) struct PreviewJob {
    vault_service: Arc<RwLock<VaultService>>,
    session: VaultSession,
    file_path: String,
    password: Zeroizing<String>,
}

impl PreviewJob {
    pub(super) fn prepare(
        vault_service: Arc<RwLock<VaultService>>,
        file_path: String,
        password: Zeroizing<String>,
        resolve_path: impl FnOnce(&str) -> Result<PathBuf, String>,
    ) -> Result<Self, String> {
        // 使用授权返回的规范路径；不能排队后重新解析用户提供的符号链接。
        let file_path = resolve_path(&file_path)?
            .into_os_string()
            .into_string()
            .map_err(|_| "Invalid import path encoding".to_string())?;
        let session = {
            let svc = vault_service
                .read()
                .map_err(|_| "Vault service lock poisoned".to_string())?;
            let account = svc
                .get_current_account()
                .ok_or_else(|| "Vault not unlocked".to_string())?;
            svc.capture_session(&account)?
        };
        Ok(Self {
            vault_service,
            session,
            file_path,
            password,
        })
    }

    pub(super) fn run(self) -> Result<DecryptedImportPreview, String> {
        let svc = self
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        solosoul_core::export_import::import::decrypt_import_preview(
            &svc,
            &self.session,
            &self.file_path,
            &self.password,
        )
        .map(Into::into)
        .map_err(crate::services::encrypted_import::map_import_failure)
    }
}

pub(super) async fn run_preview_job(
    job: PreviewJob,
    execute: impl FnOnce(PreviewJob) -> Result<DecryptedImportPreview, String> + Send + 'static,
) -> Result<DecryptedImportPreview, String> {
    // 调度器保留同一个原会话，防止 worker 完成后、返回 DTO 前发生账户切换。
    let vault_service = Arc::clone(&job.vault_service);
    let session = job.session.clone();
    let preview = tokio::task::spawn_blocking(move || execute(job))
        .await
        .map_err(|_| "导入预览任务执行失败".to_string())??;
    let svc = vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    svc.with_session(&session, |_| Ok(preview))
}

/// P2: Advanced import with object selection and strategy
#[tauri::command]
pub async fn import_execute_advanced<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: State<'_, AppState>,
    account_id: String,
    req: AdvancedImportRequest,
) -> Result<ImportResult, String> {
    let job = ImportJob::prepare(
        Arc::clone(&state.vault_service),
        &account_id,
        req,
        None,
        |path| resolve_import_path(&app, path),
    )?;
    let auto_sync = state.auto_sync.clone();
    run_import_job(job, ImportJob::run, move || auto_sync.trigger_debounce()).await
}

/// RF-027：调度前固定原会话与授权路径，所有同步导入工作由 worker 持有。
pub(super) struct ImportJob {
    vault_service: Arc<RwLock<VaultService>>,
    session: VaultSession,
    source_path: String,
    password: Zeroizing<String>,
    strategy: ImportStrategy,
    selections: Option<Vec<ImportSelection>>,
    selected_attachment_ids: Option<Vec<String>>,
    object_strategies: HashMap<String, ImportStrategy>,
    locale: String,
    progress: Option<Arc<dyn Fn(u8) + Send + Sync>>,
    operation_id: String,
    _activity: solosoul_core::import_activity::ImportActivityGuard,
}

impl ImportJob {
    pub(super) fn prepare(
        vault_service: Arc<RwLock<VaultService>>,
        account_id: &str,
        req: AdvancedImportRequest,
        progress: Option<Arc<dyn Fn(u8) + Send + Sync>>,
        resolve_path: impl FnOnce(&str) -> Result<PathBuf, String>,
    ) -> Result<Self, String> {
        let AdvancedImportRequest {
            selections,
            strategy,
            source_path,
            password,
            selected_attachment_ids,
            object_strategies,
            locale,
            operation_id,
        } = req;
        let password = Zeroizing::new(password);
        let source_path = resolve_path(&source_path)?
            .into_os_string()
            .into_string()
            .map_err(|_| "Invalid import path encoding".to_string())?;
        let (session, activity) = {
            let svc = vault_service
                .read()
                .map_err(|_| "Vault service lock poisoned".to_string())?;
            let session = svc.capture_session(account_id)?;
            let activity = solosoul_core::import_activity::begin_import_activity(svc.base_path())?;
            (session, activity)
        };
        let operation_id = operation_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        uuid::Uuid::parse_str(&operation_id).map_err(|_| import_err("INVALID_OPERATION_ID"))?;
        Ok(Self {
            vault_service,
            session,
            source_path,
            password,
            strategy,
            selections,
            selected_attachment_ids,
            object_strategies,
            locale,
            progress,
            operation_id,
            _activity: activity,
        })
    }

    pub(super) fn run(self) -> Result<ImportResult, String> {
        let svc = self
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        // 不能经 internal 重新 capture_session，否则排队中的旧请求会借用新会话。
        let request = crate::services::encrypted_import::core_request(
            self.source_path,
            self.password,
            self.strategy,
            self.selections,
            self.selected_attachment_ids,
            self.object_strategies,
            &self.locale,
            Some((self.operation_id, ImportSourceKind::Manual)),
        );
        solosoul_core::export_import::import::execute_encrypted_import(
            &svc,
            &self.session,
            request,
            self.progress,
        )
        .map(Into::into)
        .map_err(crate::services::encrypted_import::map_import_failure)
    }
}

pub(super) async fn run_import_job(
    job: ImportJob,
    execute: impl FnOnce(ImportJob) -> Result<ImportResult, String> + Send + 'static,
    on_complete: impl FnOnce() + Send,
) -> Result<ImportResult, String> {
    let vault_service = Arc::clone(&job.vault_service);
    let session = job.session.clone();
    // worker panic 时提交状态未知，不能构造 NotCommitted 或空成功结果。
    let result = tokio::task::spawn_blocking(move || execute(job))
        .await
        .map_err(|_| "导入任务执行失败".to_string())??;
    if result.is_complete() {
        // 回调仅作快速通知，不得等待或重入 Vault；生产回调为 try_send。
        // 会话失效或读锁损坏只抑制后续通知，不能抹掉已完成的导入结果。
        match vault_service.read() {
            Ok(svc) => {
                if svc
                    .with_session(&session, |_| {
                        on_complete();
                        Ok(())
                    })
                    .is_err()
                {
                    tracing::debug!("[import] 原会话已失效，跳过完成同步通知");
                }
            }
            Err(_) => tracing::warn!("[import] 服务锁损坏，跳过完成同步通知"),
        }
    }
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
#[cfg(test)]
pub(crate) fn import_execute_internal(
    // B-06 重构：改收读锁 guard（函数体内无 await 点，同步形式避免守卫跨 await 的 !Send），使云同步自动导入等非命令上下文可复用。
    svc: std::sync::RwLockReadGuard<'_, solosoul_core::vault_service::VaultService>,
    account_id: String,
    file_path: String,
    password: zeroize::Zeroizing<String>,
    strategy: ImportStrategy,
    selections: Option<Vec<ImportSelection>>,
    selected_attachment_ids: Option<Vec<String>>,
    object_strategies: HashMap<String, ImportStrategy>,
    locale: &str,
    progress: Option<Arc<dyn Fn(u8) + Send + Sync>>,
) -> Result<ImportResult, String> {
    let session = svc.capture_session(&account_id)?;
    import_execute_for_session(
        &svc,
        &session,
        file_path,
        password,
        strategy,
        selections,
        selected_attachment_ids,
        object_strategies,
        locale,
        progress,
    )
}

/// 后台导入绑定起始会话；数据库准备不持门闩，唯一批次及后续附件/偏好提交分别验证。
#[allow(clippy::too_many_arguments)]
#[cfg(test)]
pub(crate) fn import_execute_for_session(
    svc: &solosoul_core::VaultService,
    session: &solosoul_core::VaultSession,
    file_path: String,
    password: zeroize::Zeroizing<String>,
    strategy: ImportStrategy,
    selections: Option<Vec<ImportSelection>>,
    selected_attachment_ids: Option<Vec<String>>,
    object_strategies: HashMap<String, ImportStrategy>,
    locale: &str,
    progress: Option<Arc<dyn Fn(u8) + Send + Sync>>,
) -> Result<ImportResult, String> {
    let request = crate::services::encrypted_import::core_request(
        file_path,
        password,
        strategy,
        selections,
        selected_attachment_ids,
        object_strategies,
        locale,
        None,
    );
    solosoul_core::export_import::import::execute_encrypted_import(svc, session, request, progress)
        .map(Into::into)
        .map_err(crate::services::encrypted_import::map_import_failure)
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn import_execute_resumable_for_session(
    svc: &VaultService,
    session: &VaultSession,
    file_path: String,
    password: Zeroizing<String>,
    strategy: ImportStrategy,
    selections: Option<Vec<ImportSelection>>,
    selected_attachment_ids: Option<Vec<String>>,
    object_strategies: HashMap<String, ImportStrategy>,
    locale: &str,
    progress: Option<Arc<dyn Fn(u8) + Send + Sync>>,
    operation_id: &str,
    source_kind: ImportSourceKind,
) -> Result<ImportResult, String> {
    let request = crate::services::encrypted_import::core_request(
        file_path,
        password,
        strategy,
        selections,
        selected_attachment_ids,
        object_strategies,
        locale,
        Some((operation_id.into(), source_kind)),
    );
    solosoul_core::export_import::import::execute_encrypted_import(svc, session, request, progress)
        .map(Into::into)
        .map_err(crate::services::encrypted_import::map_import_failure)
}
#[cfg(test)]
pub(crate) fn resume_import_for_session(
    svc: &VaultService,
    session: &VaultSession,
    operation_id: &str,
    source_path: Option<String>,
    password: Option<Zeroizing<String>>,
    progress: Option<Arc<dyn Fn(u8) + Send + Sync>>,
) -> Result<ImportResult, String> {
    solosoul_core::export_import::import::resume_encrypted_import(
        svc,
        session,
        operation_id,
        source_path,
        password,
        progress,
    )
    .map(Into::into)
    .map_err(crate::services::encrypted_import::map_import_failure)
}

pub(crate) fn apply_operation_result(
    record: &ImportOperationRecord,
    generation: u64,
    result: &mut ImportResult,
) {
    let mut outcome = solosoul_core::export_import::import::ImportOutcome::default();
    solosoul_core::export_import::import::apply_operation_result(record, generation, &mut outcome);
    *result = outcome.into();
}
#[cfg(test)]
pub(crate) fn build_selected_ids(
    selections: Option<Vec<ImportSelection>>,
) -> Option<BTreeSet<String>> {
    solosoul_core::export_import::import::build_selected_ids(selections.map(|values| {
        values
            .into_iter()
            .map(
                |value| solosoul_core::export_import::import::ImportSelection {
                    object_id: value.object_id,
                    selected: value.selected,
                },
            )
            .collect()
    }))
}
#[cfg(test)]
pub(crate) use solosoul_core::export_import::import::{
    rebuild_imported_templates, restore_package_snapshots, snapshots_any_restorable,
    wrap_attachment_progress,
};
#[cfg(test)]
pub(crate) fn rf021_unique_shadow_name(
    vault: &solosoul_vault::VaultStore,
    view: &solosoul_vault::ImportReadView,
    shadow: &HashMap<String, solosoul_vault::ObjectRecord>,
    base: &str,
    locale: &str,
) -> Result<String, String> {
    solosoul_core::export_import::import::unique_shadow_name(vault, view, shadow, base, locale)
}
