//! RF-024：加密包导入的验证、计划、提交、恢复与预览服务。
//! 客户端只提供已授权路径、原会话和普通请求数据，不能转向当前新会话。
use super::operation::{
    import_request_fingerprint, prepare_import_operation, prepare_recovery_handoff,
    OwnedImportPackage,
};
use super::ImportTarget;
use crate::{VaultService, VaultSession};
use solosoul_vault::{
    ImportAttachmentPhase, ImportOperationPhase, ImportOperationRecord, ImportSourceKind,
    ObjectSummary,
};
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use zeroize::Zeroizing;
mod batch;
mod commit;
pub(super) mod legacy;
pub mod model;
pub mod package;
pub mod source;
pub use model::*;
use package::*;
fn generate_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn import_err(code: &str) -> String {
    code.to_owned()
}
fn import_err_with_detail(code: &str, detail: &str) -> String {
    format!("{code}:{detail}")
}
fn derive_export_key_cfg(
    password: &str,
    salt: &[u8],
    config: &solosoul_crypto::kdf::KdfConfig,
) -> Result<Zeroizing<[u8; 32]>, String> {
    solosoul_crypto::kdf::derive_export_key(password, salt, config)
        .map_err(|error| error.to_string())
}
fn load_attachments(props: &serde_json::Value) -> Vec<super::AttachmentMeta> {
    props
        .get("__attachments")
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .unwrap_or_default()
}
#[allow(clippy::too_many_arguments)]
fn log_audit_best_effort(
    vault: &solosoul_vault::VaultStore,
    action: &str,
    category: &str,
    object_id: Option<&str>,
    object_name: Option<&str>,
    source: &str,
    details: Option<&str>,
) {
    if let Err(error) =
        vault.log_structured(action, category, object_id, object_name, source, details)
    {
        tracing::warn!("[import] 写入审计日志失败: {error}");
    }
}
const IMPORT_TMP_PREFIX: &str = "solosoul-import-tmp-";

/// P013: 清扫数据目录内崩溃残留的导入明文孤儿临时目录（SIGKILL/断电时
/// `TempDir` 无法 Drop 递归删除）。前缀匹配 + `remove_dir_all` 整目录清除；
/// 单个条目失败仅 warn 不阻断（下次启动/导入仍会重试）。
/// 启动时（lib.rs setup）清扫；旧一次性导入保留原前置调用，可恢复 worker 不扫描并行任务。
pub fn cleanup_orphan_import_temps(data_dir: &std::path::Path) -> Result<(), String> {
    let Ok(entries) = std::fs::read_dir(data_dir) else {
        return Ok(());
    };
    let mut cleaned = 0usize;
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with(IMPORT_TMP_PREFIX) {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if !meta.is_dir() {
            continue;
        }
        match std::fs::remove_dir_all(entry.path()) {
            Ok(()) => cleaned += 1,
            Err(e) => tracing::warn!("[import] 清扫孤儿导入临时目录失败: {} err={}", name, e),
        }
    }
    if cleaned > 0 {
        tracing::info!("[import] 启动/导入前清扫 {cleaned} 个孤儿导入临时目录");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn import_execute_for_session(
    svc: &crate::VaultService,
    session: &crate::VaultSession,
    file_path: String,
    password: zeroize::Zeroizing<String>,
    strategy: AdvancedImportStrategy,
    selections: Option<Vec<ImportSelection>>,
    selected_attachment_ids: Option<Vec<String>>,
    object_strategies: HashMap<String, AdvancedImportStrategy>,
    locale: &str,
    progress: Option<Arc<dyn Fn(u8) + Send + Sync>>,
) -> Result<ImportOutcome, String> {
    let mut result = ImportOutcome {
        session_generation: session.generation(),
        ..Default::default()
    };
    let mut stage = ImportStage::Preparation;
    if let Err(error) = import_execute_steps(
        svc,
        session,
        file_path,
        password,
        strategy,
        selections,
        selected_attachment_ids,
        object_strategies,
        locale,
        progress,
        &mut result,
        &mut stage,
        None,
    ) {
        result.fail(stage, &error);
    }
    Ok(result)
}

struct ResumableImportContext {
    operation_id: String,
    source_kind: ImportSourceKind,
    accepted: bool,
}

#[allow(clippy::too_many_arguments)]
fn import_execute_resumable_for_session(
    svc: &VaultService,
    session: &VaultSession,
    file_path: String,
    password: Zeroizing<String>,
    strategy: AdvancedImportStrategy,
    selections: Option<Vec<ImportSelection>>,
    selected_attachment_ids: Option<Vec<String>>,
    object_strategies: HashMap<String, AdvancedImportStrategy>,
    locale: &str,
    progress: Option<Arc<dyn Fn(u8) + Send + Sync>>,
    operation_id: &str,
    source_kind: ImportSourceKind,
) -> Result<ImportOutcome, String> {
    let _activity = crate::import_activity::begin_import_activity(svc.base_path())?;
    uuid::Uuid::parse_str(operation_id).map_err(|_| import_err("INVALID_OPERATION_ID"))?;
    let mut result = ImportOutcome {
        operation_id: Some(operation_id.into()),
        session_generation: session.generation(),
        ..Default::default()
    };
    let mut stage = ImportStage::Preparation;
    let mut context = ResumableImportContext {
        operation_id: operation_id.into(),
        source_kind,
        accepted: false,
    };
    if let Err(error) = import_execute_steps(
        svc,
        session,
        file_path,
        password,
        strategy,
        selections,
        selected_attachment_ids,
        object_strategies,
        locale,
        progress,
        &mut result,
        &mut stage,
        Some(&mut context),
    ) {
        result.fail(stage, &error);
        if context.accepted {
            result.status = ImportStatus::Partial;
        }
    }
    Ok(result)
}

fn operation_commit_failure_stage(error: &str) -> ImportStage {
    match error {
        "import_batch_templates_failed" => ImportStage::Templates,
        "import_batch_snapshots_failed" => ImportStage::Snapshots,
        _ => ImportStage::Objects,
    }
}

#[allow(clippy::too_many_arguments)]
fn normalized_import_options(
    svc: &VaultService,
    session: &VaultSession,
    kind: ImportSourceKind,
    path: &str,
    strategy: AdvancedImportStrategy,
    selections: &Option<Vec<ImportSelection>>,
    attachments: &Option<Vec<String>>,
    overrides: &HashMap<String, AdvancedImportStrategy>,
    locale: &str,
) -> Result<serde_json::Value, String> {
    let selected = build_selected_ids(selections.clone());
    let attachment_ids = attachments
        .as_ref()
        .map(|ids| ids.iter().cloned().collect::<BTreeSet<_>>());
    let mut options = serde_json::json!({"strategy":strategy,"selectedObjectIds":selected,"selectedAttachmentIds":attachment_ids,
        "objectStrategies":overrides,"locale":locale});
    if kind == ImportSourceKind::Cloud {
        if strategy != AdvancedImportStrategy::SkipExisting
            || selections.is_some()
            || attachments.is_some()
            || !overrides.is_empty()
            || locale != "en-US"
        {
            return Err(import_err("INVALID_CLOUD_IMPORT_OPTIONS"));
        }
        let (device, hlc) =
            source::cloud_import_source_identity(svc, session, std::path::Path::new(path))?;
        options = serde_json::json!({"strategy":"skipExisting","selectedObjectIds":null,"selectedAttachmentIds":null,
            "objectStrategies":{},"locale":"en-US","deviceId":device,"hlc":hlc});
    }
    Ok(options)
}

pub fn apply_operation_result(
    record: &ImportOperationRecord,
    generation: u64,
    result: &mut ImportOutcome,
) {
    result.operation_id = Some(record.start.operation_id.clone());
    result.session_generation = generation;
    result.object_count = record.database_commit.object_ids.len();
    result.template_count = record.database_commit.template_ids.len();
    result.snapshot_count = record.database_commit.snapshot_write_count;
    result.attachment_count = record.attachment_count;
    result.attachment_files_written = record
        .steps
        .iter()
        .filter(|step| {
            matches!(
                step.phase,
                ImportAttachmentPhase::Published | ImportAttachmentPhase::MetadataCommitted
            )
        })
        .count();
    result.preferences_imported = record.preferences_imported;
    result.status = if record.phase == ImportOperationPhase::Complete {
        ImportStatus::Complete
    } else {
        ImportStatus::Partial
    };
}

#[allow(clippy::too_many_arguments)]
fn run_existing_operation(
    svc: &VaultService,
    session: &VaultSession,
    record: &ImportOperationRecord,
    owned: Option<&OwnedImportPackage>,
    password: Option<&str>,
    progress: Option<Arc<dyn Fn(u8) + Send + Sync>>,
    result: &mut ImportOutcome,
    stage: &mut ImportStage,
) -> Result<(), String> {
    *stage = ImportStage::Attachments;
    let key = svc.attachment_key_for_session(session)?;
    let callback = progress.map(wrap_attachment_progress);
    let execution = commit::complete_operation(
        svc,
        session,
        record,
        svc.base_path(),
        owned,
        password,
        &key,
        callback.as_deref(),
    );
    let committed = execution.progress;
    let persisted = execution.persisted;
    let execution = execution.outcome;
    if let Some(latest) = persisted.as_ref() {
        apply_operation_result(latest, session.generation(), result);
    }
    result.attachment_count = result.attachment_count.max(committed.committed_count);
    result.attachment_files_written = result
        .attachment_files_written
        .max(committed.written_file_count);
    if record.start.preferences_required && result.attachment_count == record.steps.len() {
        *stage = ImportStage::Preferences;
    }
    match execution {
        Ok(complete) => {
            apply_operation_result(&complete, session.generation(), result);
            result.failure_stage = None;
            result.error_code = None;
            // 原审计仍为 best-effort；已完成任务的重复 Resume 不重复添加，审计失效不改写 journal 完成事实。
            if record.phase != ImportOperationPhase::Complete {
                let strategy = serde_json::from_value::<AdvancedImportStrategy>(
                    record.start.plan["requestOptions"]["strategy"].clone(),
                )
                .unwrap_or(AdvancedImportStrategy::SkipExisting);
                let name = record
                    .start
                    .plan
                    .get("sourceName")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("Imported package");
                let details = build_import_details(
                    result.object_count,
                    result.attachment_count,
                    name,
                    strategy,
                );
                let _ = svc.with_session(session, |vault| {
                    log_audit_best_effort(
                        vault,
                        "import_execute",
                        "import",
                        None,
                        None,
                        "user",
                        Some(&details.to_string()),
                    );
                    Ok(())
                });
            }
            Ok(())
        }
        Err(error) => Err(if error.to_string() == "解密失败：密码错误或文件已损坏" {
            import_err("DECRYPT_FAILED")
        } else {
            error.to_string()
        }),
    }
}

fn resume_import_for_session(
    svc: &VaultService,
    session: &VaultSession,
    operation_id: &str,
    source_path: Option<String>,
    password: Option<Zeroizing<String>>,
    progress: Option<Arc<dyn Fn(u8) + Send + Sync>>,
) -> Result<ImportOutcome, String> {
    let _activity = crate::import_activity::begin_import_activity(svc.base_path())?;
    let record = svc
        .with_session(session, |vault| {
            vault.load_import_operation(session.account_id(), operation_id)
        })?
        .ok_or_else(|| import_err("OPERATION_NOT_FOUND"))?;
    if record.phase == ImportOperationPhase::Abandoned {
        return Err(import_err("OPERATION_ABANDONED"));
    }
    let mut result = ImportOutcome::default();
    apply_operation_result(&record, session.generation(), &mut result);
    let mut stage = ImportStage::Preparation;
    let execution = (|| {
        let owned = source_path
            .as_ref()
            .map(|path| {
                OwnedImportPackage::capture(std::path::Path::new(path), svc.base_path())
                    .map_err(|error| error.to_string())
            })
            .transpose()?;
        run_existing_operation(
            svc,
            session,
            &record,
            owned.as_ref(),
            password.as_deref().map(|password| password.as_str()),
            progress,
            &mut result,
            &mut stage,
        )
    })();
    if let Err(error) = execution {
        result.fail(stage, &error);
        result.status = ImportStatus::Partial;
    }
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
fn import_execute_steps(
    svc: &crate::VaultService,
    session: &crate::VaultSession,
    file_path: String,
    password: zeroize::Zeroizing<String>,
    strategy: AdvancedImportStrategy,
    selections: Option<Vec<ImportSelection>>,
    selected_attachment_ids: Option<Vec<String>>,
    object_strategies: HashMap<String, AdvancedImportStrategy>,
    locale: &str,
    progress: Option<Arc<dyn Fn(u8) + Send + Sync>>,
    result: &mut ImportOutcome,
    stage: &mut ImportStage,
    mut resumable: Option<&mut ResumableImportContext>,
) -> Result<(), String> {
    let target = ImportTarget::Session {
        service: svc,
        session,
    };
    target.commit(|_| Ok(()))?;
    let account_id = session.account_id();
    let vault_att_key = svc.attachment_key_for_session(session)?;

    let owned = if resumable.is_some() {
        Some(
            OwnedImportPackage::capture(std::path::Path::new(&file_path), svc.base_path())
                .map_err(|e| e.to_string())?,
        )
    } else {
        None
    };
    let frozen_options = resumable
        .as_ref()
        .map(|context| {
            normalized_import_options(
                svc,
                session,
                context.source_kind,
                &file_path,
                strategy,
                &selections,
                &selected_attachment_ids,
                &object_strategies,
                locale,
            )
        })
        .transpose()?;
    if let (Some(context), Some(owned), Some(options)) = (
        resumable.as_deref_mut(),
        owned.as_ref(),
        frozen_options.as_ref(),
    ) {
        let fingerprint = import_request_fingerprint(options).map_err(|e| e.to_string())?;
        let binding = session.vault().import_root_binding()?;
        let mut existing = target
            .commit(|vault| vault.load_import_operation(account_id, &context.operation_id))?;
        if existing.is_none() && context.source_kind == ImportSourceKind::Cloud {
            existing = target.commit(|vault| {
                vault.find_cloud_import_operation(
                    account_id,
                    owned.source_proof(),
                    &fingerprint,
                    &binding,
                )
            })?;
        }
        if let Some(record) = existing {
            context.accepted = true;
            apply_operation_result(&record, session.generation(), result);
            if record.start.source_kind != context.source_kind
                || record.start.source != *owned.source_proof()
                || record.start.request_fingerprint != fingerprint
                || record.start.root_binding != binding
            {
                return Err(import_err("OPERATION_MISMATCH"));
            }
            return run_existing_operation(
                svc,
                session,
                &record,
                Some(owned),
                Some(password.as_str()),
                progress,
                result,
                stage,
            );
        }
    }
    if password.is_empty() {
        return Err(import_err("PASSWORD_REQUIRED"));
    }

    // ── 阶段 1：解密包读取（password 为 Zeroizing，自动 Deref 为 &str）──
    // P013: 导入前清扫上次崩溃残留的明文孤儿临时目录（数据目录内）。
    if resumable.is_none() {
        let _ = cleanup_orphan_import_temps(svc.base_path());
    }
    let parse_path = owned
        .as_ref()
        .map(|package| package.path())
        .unwrap_or_else(|| std::path::Path::new(&file_path));
    let (manifest, payload, key) = decrypt_package(
        parse_path.to_str().ok_or("Invalid import path encoding")?,
        &password,
        svc.base_path(),
    )?;

    // Build selection set if provided
    let selected_ids = build_selected_ids(selections);

    let objects = payload["objects"]
        .as_array()
        .ok_or("No objects array in payload")?;
    let package_ids = build_package_ids(&payload);

    // ── 阶段 1.5：解析包内对象历史快照（object_id → 快照列表）──
    // 导出端携带每个对象的全部历史快照（含原时间戳），导入时按原时间线恢复，
    // 保证跨设备恢复后历史记录数量与旧设备一致。
    let package_snapshots = build_package_snapshots(&payload);

    // ── 阶段 2：一致只读视图与模板计划，准备期间不写入数据库 ──
    *stage = ImportStage::Templates;
    // 入口已验证原会话；捕获全部 raw 行也在 session gate 外，不阻塞锁定。
    // 始终读取 session 的原 Vault，Locked/陈旧修订不改取当前新账户。
    let view = target
        .vault()
        .read_import_view(account_id)
        .map_err(|error| error.to_string())?;
    // 按旧调用边界：包没有模板/空数组时，不严格扫描无关本地模板。
    // Vault 的按需模板 accessor 是本候选明确依赖，详见 README。
    let local_templates = if payload["templates"]
        .as_array()
        .is_some_and(|templates| !templates.is_empty())
    {
        target.vault().list_import_view_user_templates(&view)?
    } else {
        Vec::new()
    };
    let now = chrono::Utc::now().to_rfc3339();
    let templates = batch::prepare_templates(local_templates, account_id, &payload, &now)?;
    let available_templates: HashMap<_, _> = templates
        .available
        .iter()
        .cloned()
        .map(|template| (template.id.clone(), template))
        .collect();

    // ── 阶段 3：沿用 RF1063 的实际选择 + 有效策略 KeepBoth 统一映射 ──
    let id_map =
        build_keepboth_id_map(objects, strategy, &object_strategies, selected_ids.as_ref());

    // ── 阶段 4：有序准备全部对象和历史，再由原会话门闩提交唯一 SQLite 批次 ──
    *stage = ImportStage::Objects;
    let mut plan = batch::prepare_objects(
        target.vault(),
        &view,
        objects,
        account_id,
        strategy,
        &object_strategies,
        selected_ids.as_ref(),
        &package_ids,
        &templates.id_map,
        &available_templates,
        &id_map,
        &package_snapshots,
        templates.added,
        &now,
        locale,
        progress.as_deref(),
        stage,
    )?;
    *stage = ImportStage::Objects;
    if let (Some(context), Some(owned), Some(options)) = (resumable, owned.as_ref(), frozen_options)
    {
        let selected: Option<std::collections::HashSet<String>> = selected_attachment_ids
            .as_ref()
            .map(|ids| ids.iter().cloned().collect());
        let include_preferences = manifest
            .extra_files
            .iter()
            .any(|name| name == "preferences.enc");
        let mut start = prepare_import_operation(
            &context.operation_id,
            context.source_kind,
            options,
            owned,
            &payload,
            &plan.imported_object_ids,
            &id_map,
            selected.as_ref(),
            &now,
            svc.base_path(),
            target.vault(),
            &view,
            &mut plan.batch,
            manifest.has_attachments,
            include_preferences,
        )
        .map_err(|e| e.to_string())?;
        start.plan["sourceName"] = serde_json::json!(std::path::Path::new(&file_path)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default());
        let handoff = if context.source_kind == ImportSourceKind::Recovery {
            *stage = ImportStage::Preparation;
            let opened = owned
                .decrypt(&password, svc.base_path())
                .map_err(|e| e.to_string())?;
            Some(
                prepare_recovery_handoff(
                    svc,
                    session,
                    svc.base_path(),
                    owned,
                    &opened,
                    &mut start,
                    &vault_att_key,
                )
                .map_err(|e| e.to_string())?,
            )
        } else {
            None
        };
        // Recovery handoff 重编码 typed plan；展示名称在其完成后补回，不参与请求 fingerprint。
        start.plan["sourceName"] = serde_json::json!(std::path::Path::new(&file_path)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default());
        *stage = ImportStage::Objects;
        let committed = commit::commit_operation(svc, session, &view.revision, &plan.batch, &start)
            .inspect_err(|error| {
                *stage = operation_commit_failure_stage(error);
            })?;
        context.accepted = true;
        if let Some(handoff) = handoff {
            handoff.accept();
        }
        apply_operation_result(&committed.operation, session.generation(), result);
        return run_existing_operation(
            svc,
            session,
            &committed.operation,
            Some(owned),
            Some(password.as_str()),
            progress,
            result,
            stage,
        );
    }
    let committed = target.commit(|vault| {
        vault
            .commit_import_batch(account_id, &view.revision, &plan.batch)
            .map_err(|error| {
                *stage = batch::commit_failure_stage(error);
                error.to_string()
            })
    })?;
    // 只能发布真实 COMMIT 成功的结果；数据库失败没有部分计数。
    result.template_count = committed.template_ids.len();
    result.object_count = committed.object_ids.len();
    result.snapshot_count = committed.snapshot_write_count;
    let imported_object_ids = plan.imported_object_ids;

    // 构建选中附件 ID 集合，用于附件过滤
    let sel_att_ids_set: Option<std::collections::HashSet<String>> =
        selected_attachment_ids.map(|ids| ids.into_iter().collect());

    // ── 阶段 5+6：导入附件与偏好设置（附件进度续接 80-100）──
    // 密钥已在入口绑定原会话，不能在耗时解密后重新读取当前账户密钥。
    *stage = ImportStage::Attachments;
    import_attachments_and_preferences(
        &target,
        svc.base_path(),
        &file_path,
        &key,
        &manifest,
        &payload,
        &id_map,
        &imported_object_ids,
        sel_att_ids_set.as_ref(),
        &now,
        progress.clone(),
        account_id,
        &vault_att_key,
        result,
        stage,
    )?;

    let details = build_import_details(
        result.object_count,
        result.attachment_count,
        &file_path,
        strategy,
    );
    target.commit(|vault| {
        log_audit_best_effort(
            vault,
            "import_execute",
            "import",
            None,
            None,
            "user",
            Some(&details.to_string()),
        );
        Ok(())
    })?;

    Ok(())
}
/// 构建选中附件/对象 ID 集合（selections 中 selected=true 的 object_id）。
pub fn build_selected_ids(selections: Option<Vec<ImportSelection>>) -> Option<BTreeSet<String>> {
    selections.map(|sels| {
        sels.into_iter()
            .filter(|s| s.selected)
            .map(|s| s.object_id)
            .collect()
    })
}

/// 阶段 5+6：导入附件（加密，流式解密）与偏好设置。
/// 附件阶段进度续接对象阶段末尾（80-100），避免进度条回落。返回附件导入数量。
#[allow(clippy::too_many_arguments)]
fn import_attachments_and_preferences(
    target: &ImportTarget<'_>,
    base_path: &std::path::Path,
    file_path: &str,
    key: &[u8; 32],
    manifest: &ManifestData,
    payload: &serde_json::Value,
    id_map: &HashMap<String, String>,
    imported_object_ids: &std::collections::HashSet<String>,
    sel_att_ids_set: Option<&std::collections::HashSet<String>>,
    now: &str,
    progress: Option<Arc<dyn Fn(u8) + Send + Sync>>,
    account_id: &str,
    vault_att_key: &[u8; 32],
    result: &mut ImportOutcome,
    stage: &mut ImportStage,
) -> Result<(), String> {
    let att_progress = progress.map(wrap_attachment_progress);
    if manifest.has_attachments {
        // P012: 附件导入统一走 core 唯一实现（进度/选择性/KeepBoth 重映射均由 core 承载），
        // 不再有 GUI 侧平行实现。
        let salt = hex::decode(&manifest.salt_hex).map_err(|e| format!("Invalid salt: {e}"))?;
        let mut committed = crate::export_import::AttachmentImportProgress::default();
        let attachment_result = crate::export_import::import_attachments_into(
            target,
            base_path,
            std::path::Path::new(file_path),
            key,
            &salt,
            imported_object_ids,
            payload,
            Some(vault_att_key),
            id_map,
            sel_att_ids_set,
            now,
            att_progress.as_deref(),
            &mut committed,
        );
        result.attachment_count = committed.committed_count;
        result.attachment_files_written = committed.written_file_count;
        attachment_result.map_err(|e| e.to_string())?;
    }
    *stage = ImportStage::Preferences;
    import_preferences(target, file_path, key, manifest, account_id)?;
    result.preferences_imported = manifest
        .extra_files
        .iter()
        .any(|name| name == "preferences.enc");
    Ok(())
}

/// 组装导入审计详情（count / attachmentCount / fileName / strategy）。
fn build_import_details(
    imported: usize,
    imported_attachments_count: usize,
    file_path: &str,
    strategy: AdvancedImportStrategy,
) -> serde_json::Value {
    let file_name = std::path::Path::new(file_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| file_path.to_string());
    serde_json::json!({
        "count": imported,
        "attachmentCount": imported_attachments_count,
        "fileName": file_name,
        "strategy": match strategy {
            AdvancedImportStrategy::SkipExisting => "skipExisting",
            AdvancedImportStrategy::Overwrite => "overwrite",
            AdvancedImportStrategy::KeepBoth => "keepBoth",
        },
    })
}

/// 阶段 1.5：解析包内对象历史快照（object_id → 快照列表）。
/// 导出端携带每个对象的全部历史快照（含原时间戳），导入时按原时间线恢复。
fn build_package_snapshots(payload: &serde_json::Value) -> HashMap<String, Vec<serde_json::Value>> {
    payload["snapshots"]
        .as_array()
        .map(|arr| {
            let mut map: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
            for snap in arr {
                if let Some(oid) = snap["object_id"].as_str() {
                    map.entry(oid.to_string()).or_default().push(snap.clone());
                }
            }
            map
        })
        .unwrap_or_default()
}

/// 阶段 3：按实际生效策略，为本次选中对象预构建 KeepBoth ID 映射表。
/// 未选对象保留原 ID 引用；选中对象的前向引用、附件和重复源 ID 共用此表。
fn build_keepboth_id_map(
    objects: &[serde_json::Value],
    strategy: AdvancedImportStrategy,
    object_strategies: &HashMap<String, AdvancedImportStrategy>,
    selected_ids: Option<&BTreeSet<String>>,
) -> HashMap<String, String> {
    let mut id_map: HashMap<String, String> = HashMap::new();
    for obj_val in objects {
        let id = obj_val["id"].as_str().unwrap_or("");
        if id.is_empty() || selected_ids.is_some_and(|selected| !selected.contains(id)) {
            continue;
        }
        if object_strategies.get(id).copied().unwrap_or(strategy)
            == AdvancedImportStrategy::KeepBoth
        {
            id_map.insert(id.to_string(), generate_id());
        }
    }
    id_map
}

/// 阶段 5：附件进度续接对象阶段末尾（80-100），避免进度条回落。
pub fn wrap_attachment_progress(
    cb: Arc<dyn Fn(u8) + Send + Sync>,
) -> Arc<dyn Fn(u8) + Send + Sync> {
    Arc::new(move |pct: u8| {
        cb((80 + u16::from(pct) * 20 / 100) as u8);
    })
}

// ── 阶段化辅助函数（P023 拆分）──────────────────────────────────

/// 阶段 1：读取并解密导入包，返回 (manifest, payload, 派生密钥)。
///
/// P013: 明文临时目录建于 `temp_base`（保险库数据目录，0700）内而非系统 temp——
/// 进程 SIGKILL/崩溃时残留明文仍位于受保护的数据目录，且前缀固定可被
/// `cleanup_orphan_import_temps` 清扫；正常路径 `TempDir` Drop 递归删除整个目录。
fn decrypt_package(
    file_path: &str,
    password: &str,
    temp_base: &std::path::Path,
) -> Result<(ManifestData, serde_json::Value, Zeroizing<[u8; 32]>), String> {
    let manifest = read_manifest(file_path)?;
    let salt = hex::decode(&manifest.salt_hex).map_err(|e| format!("Invalid salt: {}", e))?;
    // P202: 按 manifest 声明参数派生（旧格式包无 kdf 字段回退 balanced 兼容）。
    let key = derive_export_key_cfg(password, &salt, &manifest.kdf_config())?;
    // R2-15: 主 payload 流式解密——`payload.enc` 经 decrypt_chunked_stream 直接写入临时文件，
    // 再从文件流式解析 JSON；峰值内存由「密文 + 明文 + JSON 树」约 3× 降至约 1× payload。
    let tmp_dir = tempfile::Builder::new()
        .prefix(IMPORT_TMP_PREFIX)
        .tempdir_in(temp_base)
        .map_err(|e| format!("创建临时目录失败: {}", e))?;
    let mut tmp = tempfile::NamedTempFile::new_in(tmp_dir.path())
        .map_err(|e| format!("创建临时文件失败: {}", e))?;
    decrypt_zip_entry_streaming(file_path, "payload.enc", &key, &mut tmp)?;
    let payload: serde_json::Value = {
        let f = std::fs::File::open(tmp.path()).map_err(|e| format!("读取临时文件失败: {}", e))?;
        serde_json::from_reader(f).map_err(|e| format!("Invalid payload: {}", e))?
    };
    // tmp（NamedTempFile）先于 tmp_dir Drop；tmp_dir 随后递归删除整个临时目录。
    Ok((manifest, payload, key))
}

/// 阶段 1.5 helper：按原时间戳恢复对象历史快照（base64 解码 → 加密写入）。
/// 返回成功恢复的快照条数；快照为空/解码失败返回 0（调用方回退到 diff_imported 初始快照）。
/// P1 辅助：判断包内快照列表中是否存在至少一条可恢复的快照（base64 可解码且非空）。
/// 覆盖导入仅在确有可恢复快照时才清空本地旧历史，防止损坏包（快照全部解码失败）
/// 误删本地历史后仅回退为一条 diff_imported 快照。
pub fn snapshots_any_restorable(snaps: &[serde_json::Value]) -> bool {
    snaps.iter().any(|snap| match snap["data"].as_str() {
        Some(b64) => base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)
            .map(|d| !d.is_empty())
            .unwrap_or(false),
        None => false,
    })
}

pub fn restore_package_snapshots(
    vault: &solosoul_vault::VaultStore,
    object_id: &str,
    snaps: &[serde_json::Value],
) -> usize {
    restore_package_snapshots_tracked(
        &ImportTarget::Direct(vault),
        object_id,
        snaps,
        &mut ImportOutcome::default(),
    )
    .unwrap()
}

fn restore_package_snapshots_tracked(
    target: &ImportTarget<'_>,
    object_id: &str,
    snaps: &[serde_json::Value],
    result: &mut ImportOutcome,
) -> Result<usize, String> {
    let mut restored = 0usize;
    // 导出按最新在前排列；倒序写入保留同毫秒版本的原有新旧顺序（RF-1067）。
    for snap in snaps.iter().rev() {
        // 原时间戳缺失/非法时回退到当前时间，避免 0 时间戳破坏历史排序
        let timestamp = snap["timestamp"]
            .as_i64()
            .filter(|t| *t > 0)
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
        let triggered_by = snap["triggered_by"].as_str().unwrap_or("import");
        let diff_summary = snap["diff_summary"].as_str().unwrap_or("diff_imported");
        let data = match snap["data"].as_str() {
            Some(b64) => {
                match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64) {
                    Ok(d) => d,
                    Err(e) => {
                        tracing::warn!(
                            "[import] 快照 base64 解码失败，跳过: object={} err={}",
                            object_id,
                            e
                        );
                        continue;
                    }
                }
            }
            None => continue,
        };
        if data.is_empty() {
            continue;
        }
        target.commit(|vault| {
            vault.save_snapshot_at(object_id, triggered_by, &data, diff_summary, timestamp)
        })?;
        restored += 1;
        result.snapshot_count += 1;
    }
    Ok(restored)
}

/// 阶段 2：重建包内引用的模板（快照隔离，按内容哈希去重），返回 原模板 ID → 本地模板 ID 映射。
pub fn rebuild_imported_templates(
    vault: &solosoul_vault::VaultStore,
    account_id: &str,
    payload: &serde_json::Value,
) -> Result<std::collections::HashMap<String, String>, String> {
    let view = vault
        .read_import_view(account_id)
        .map_err(|error| error.to_string())?;
    let initial = if payload["templates"]
        .as_array()
        .is_some_and(|templates| !templates.is_empty())
    {
        vault.list_import_view_user_templates(&view)?
    } else {
        Vec::new()
    };
    let now = chrono::Utc::now().to_rfc3339();
    let plan = batch::prepare_templates(initial, account_id, payload, &now)?;
    vault
        .commit_import_batch(
            account_id,
            &view.revision,
            &solosoul_vault::ImportDatabaseBatch {
                templates: plan.added,
                objects: Vec::new(),
            },
        )
        .map_err(|error| error.to_string())?;
    Ok(plan.id_map)
}

/// 阶段 4.1：构建导入对象记录（含 KeepBoth ID 引用重写）。
#[allow(clippy::too_many_arguments)]
fn build_import_record(
    obj_val: &serde_json::Value,
    account_id: &str,
    id_map: &HashMap<String, String>,
    resolved_template_id: Option<String>,
    final_id: &str,
    final_name: &str,
    properties: serde_json::Value,
    property_labels: Option<serde_json::Value>,
    now: &str,
) -> solosoul_vault::ObjectRecord {
    solosoul_vault::ObjectRecord {
        contract_type_id: obj_val["contract_type_id"].as_str().map(String::from),
        id: final_id.to_string(),
        account_id: account_id.to_string(),
        type_id: obj_val["type_id"].as_str().unwrap_or("note").to_string(),
        section_type: obj_val["section_type"]
            .as_str()
            .unwrap_or("identity")
            .to_string(),
        name: final_name.to_string(),
        icon_name: obj_val["icon_name"]
            .as_str()
            .unwrap_or("document")
            .to_string(),
        parent_id: obj_val["parent_id"].as_str().map(|pid| {
            // 无条件重写引用：如果父对象被 KeepBoth 重写了 ID，使用新 ID
            id_map.get(pid).cloned().unwrap_or_else(|| pid.to_string())
        }),
        children_ids: {
            // 无条件重写 children_ids 引用
            let mut cids: Vec<String> = obj_val["children_ids"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            for cid in &mut cids {
                if let Some(new_cid) = id_map.get(cid) {
                    *cid = new_cid.clone();
                }
            }
            cids
        },
        properties,
        property_labels,
        sensitivity_level: obj_val["sensitivity_level"]
            .as_str()
            .unwrap_or("internal")
            .to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: obj_val["tags"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        template_id: resolved_template_id,
        template_type: obj_val["template_type"].as_str().map(String::from),
        template_hash: obj_val["template_hash"].as_str().map(String::from),
        ignored_template_hash: obj_val["ignored_template_hash"].as_str().map(String::from),
        created_at: obj_val["created_at"].as_str().unwrap_or(now).to_string(),
        updated_at: now.to_string(),
        version: obj_val["version"].as_u64().unwrap_or(1) as u32,
    }
}

/// 合并模板 property_labels 进现有 labels：模板值作为兜底，不覆盖已有值。
fn merge_labels_into(tpl: &serde_json::Value, existing: &mut serde_json::Value) {
    if let (Some(tpl_obj), Some(existing_obj)) = (tpl.as_object(), existing.as_object_mut()) {
        for (k, v) in tpl_obj {
            existing_obj.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }
}

/// 阶段 6：导入偏好设置（如包内含 preferences.enc）。
fn import_preferences(
    target: &ImportTarget<'_>,
    file_path: &str,
    key: &[u8; 32],
    manifest: &ManifestData,
    account_id: &str,
) -> Result<(), String> {
    if !manifest
        .extra_files
        .contains(&"preferences.enc".to_string())
    {
        return Ok(());
    }
    let prefs_salt = hex::decode(&manifest.salt_hex)
        .map_err(|e| format!("Invalid salt_hex in manifest: {}", e))?;
    let prefs_key =
        solosoul_crypto::hkdf_ext::derive_hkdf_key(key, &prefs_salt, b"solosoul:preferences:v1")
            .map_err(|e| format!("derive prefs key: {}", e))?;
    let prefs_enc = read_file_from_zip(file_path, "preferences.enc")?;
    let prefs_dec = solosoul_crypto::cipher::decrypt_from_bytes(&prefs_key, &prefs_enc, None)
        .map_err(|_| "Invalid imported preferences".to_string())?;
    let profile = solosoul_vault::Profile::new_with_id(account_id, account_id, prefs_dec.to_vec());
    target.commit(|vault| vault.save_profile(&profile))?;
    Ok(())
}

/// P019：导入预览的对象摘要映射（自 import_decrypt_preview 拆出，逐字保持）。
fn build_preview_object_summaries(payload: &serde_json::Value) -> Vec<ObjectSummary> {
    payload["objects"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|o| {
                    Some(ObjectSummary {
                        contract_type_id: o["contract_type_id"].as_str().map(String::from),
                        id: o["id"].as_str()?.to_string(),
                        name: o["name"].as_str()?.to_string(),
                        collection_type: o["type_id"].as_str()?.to_string(),
                        section_type: o["section_type"].as_str().unwrap_or("").to_string(),
                        sensitivity_level: o["sensitivity_level"]
                            .as_str()
                            .unwrap_or("internal")
                            .to_string(),
                        created_at: o["created_at"].as_str().unwrap_or("").to_string(),
                        updated_at: o["updated_at"].as_str().unwrap_or("").to_string(),
                        is_deleted: false,
                        template_id: o["template_id"].as_str().map(String::from),
                        template_type: o["template_type"].as_str().map(String::from),
                        template_hash: o["template_hash"].as_str().map(String::from),
                        ignored_template_hash: o["ignored_template_hash"]
                            .as_str()
                            .map(String::from),
                        icon_name: o["icon_name"].as_str().unwrap_or("document").to_string(),
                        parent_id: o["parent_id"].as_str().map(String::from),
                        properties: o["properties"].clone(),
                        property_labels: None,
                        // 与导出范围树同一口径（solosoul_vault::object_has_attachments）：
                        // 未软删附件存在性，供导入侧对象行按附件展开。
                        has_attachments: solosoul_vault::object_has_attachments(&o["properties"]),
                        // 字段敏感度集合（导入包无 property_labels，由 __fields/dynamic_group 推导）
                        sensitivity_levels: solosoul_vault::object_field_sensitivity_levels(
                            None,
                            &o["properties"],
                        ),
                        tags: o["tags"]
                            .as_array()
                            .map(|t| {
                                t.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn execute_encrypted_import(
    service: &VaultService,
    session: &VaultSession,
    request: EncryptedImportRequest,
    progress: Option<Arc<dyn Fn(u8) + Send + Sync>>,
) -> Result<ImportOutcome, ImportFailure> {
    let EncryptedImportRequest {
        source_path,
        password,
        options,
        operation,
    } = request;
    let ImportOptions {
        strategy,
        selections,
        selected_attachment_ids,
        object_strategies,
        locale,
    } = options;
    match operation {
        Some((id, kind)) => import_execute_resumable_for_session(
            service,
            session,
            source_path,
            password,
            strategy,
            selections,
            selected_attachment_ids,
            object_strategies,
            &locale,
            progress,
            &id,
            kind,
        ),
        None => import_execute_for_session(
            service,
            session,
            source_path,
            password,
            strategy,
            selections,
            selected_attachment_ids,
            object_strategies,
            &locale,
            progress,
        ),
    }
    .map_err(ImportFailure::from)
}
pub fn resume_encrypted_import(
    service: &VaultService,
    session: &VaultSession,
    id: &str,
    source_path: Option<String>,
    password: Option<Zeroizing<String>>,
    progress: Option<Arc<dyn Fn(u8) + Send + Sync>>,
) -> Result<ImportOutcome, ImportFailure> {
    resume_import_for_session(service, session, id, source_path, password, progress)
        .map_err(ImportFailure::from)
}

pub fn decrypt_import_preview(
    service: &VaultService,
    session: &VaultSession,
    file_path: &str,
    password: &str,
) -> Result<DecryptedImportPreview, ImportFailure> {
    service
        .with_session(session, |_| Ok(()))
        .map_err(ImportFailure::from)?;
    let vault = session.vault();
    let (manifest, payload, _key) =
        decrypt_package(file_path, password, vault.base_path()).map_err(ImportFailure::from)?;
    service
        .with_session(session, |_| Ok(()))
        .map_err(ImportFailure::from)?;
    // P019：对象映射拆至 build_preview_object_summaries。
    let objects = build_preview_object_summaries(&payload);

    // P043: 批量加载本地对象一次（IN 查询），替代逐条 load_object——
    // 大导入包下将 N 次锁竞争 + N 次查询降为 1 次（非热路径但廉价且语义等价）。
    let ids: Vec<String> = objects.iter().map(|o| o.id.clone()).collect();
    let existing_map = vault.load_objects_batch(&ids).unwrap_or_default();

    let mut conflicts = Vec::new();
    for obj in &objects {
        if let Some(existing) = existing_map.get(&obj.id) {
            // Soft-deleted objects are in trash and should not be treated as conflicts.
            if !existing.is_deleted {
                // 比较名称判断冲突类型：名称相同为 Identical，否则为 RenamedLocal
                //（只能区分名称是否相同，无法判断是本地改名还是导入包名称被修改）
                let kind = if obj.name == existing.name {
                    ConflictKind::Identical
                } else {
                    ConflictKind::RenamedLocal
                };
                conflicts.push(ConflictInfo {
                    object_id: obj.id.clone(),
                    imported_name: obj.name.clone(),
                    existing_name: existing.name.clone(),
                    kind,
                });
            }
        }
    }

    let has_preferences = manifest
        .extra_files
        .contains(&"preferences.enc".to_string());

    // Build attachment preview list from payload
    let mut attachments = Vec::new();
    if manifest.has_attachments {
        for obj in &objects {
            let atts = load_attachments(&obj.properties);
            for att in &atts {
                if att.deleted_at.is_some() {
                    continue;
                }
                attachments.push(AttachmentImportInfo {
                    id: att.id.clone(),
                    object_id: obj.id.clone(),
                    file_name: att.file_name.clone(),
                    size_bytes: att.size_bytes,
                });
            }
        }
    }

    Ok(DecryptedImportPreview {
        objects,
        conflicts,
        has_preferences,
        has_audit_log: false,
        attachments,
    })
}

pub fn unique_shadow_name(
    vault: &solosoul_vault::VaultStore,
    view: &solosoul_vault::ImportReadView,
    shadow: &HashMap<String, solosoul_vault::ObjectRecord>,
    base: &str,
    locale: &str,
) -> Result<String, String> {
    batch::unique_shadow_name(vault, view, shadow, base, locale)
}

#[cfg(test)]
mod rf024;
