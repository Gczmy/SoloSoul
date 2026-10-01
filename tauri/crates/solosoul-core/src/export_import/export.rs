//! RF-023：完整加密包用例。主机只适配路径、请求和错误；两种历史包策略共用执行器。
use super::export_output::{create_export_output, finish_export_output, write_export_output};
use super::*;
use crate::{VaultService, VaultSession};
use std::collections::BTreeSet;

/// Advanced 是 GUI/Cloud/Recovery 的历史包；LegacyDirect 保留旧 CLI/Core 包字段。
#[derive(Clone, Copy)]
enum PackageProfile {
    Advanced,
    LegacyDirect,
}

/// 已经完成主机边界适配的业务选择，不向 IPC 序列化。
#[derive(Debug, Clone)]
pub struct EncryptedExportScope {
    pub selected_page_ids: Vec<String>,
    pub selected_object_ids: Vec<String>,
    pub selected_tags: Vec<String>,
    pub include_all: bool,
    pub attachments: AttachmentExportScope,
    pub include_preferences: bool,
    pub include_behavioral: bool,
}
impl EncryptedExportScope {
    pub fn full_snapshot() -> Self {
        Self {
            selected_page_ids: vec![],
            selected_object_ids: vec![],
            selected_tags: vec![],
            include_all: true,
            attachments: AttachmentExportScope::All,
            include_preferences: true,
            include_behavioral: false,
        }
    }
}

/// 密码仅借用调用方受保护的存储；计划只保留 Zeroizing 派生密钥。
pub struct EncryptedExportRequest<'a> {
    pub scope: &'a EncryptedExportScope,
    pub password: &'a str,
    pub password_hint: &'a Option<String>,
    pub app_version: &'a str,
}

#[derive(Debug, thiserror::Error)]
pub enum ExportFailure {
    #[error("PASSWORD_EMPTY")]
    PasswordEmpty,
    #[error("SAME_AS_MASTER_PASSWORD")]
    SameAsMasterPassword,
    #[error("MASTER_VERIFY_FAILED: {0}")]
    MasterVerifyFailed(String),
    #[error("NO_OBJECTS_SELECTED")]
    NoObjectsSelected,
    #[error("ATTACHMENT_TOO_LARGE: {0}")]
    AttachmentTooLarge(String),
    #[error("TOTAL_SIZE_EXCEEDED")]
    TotalSizeExceeded,
    #[error("{0}")]
    Backend(#[from] ExportError),
}
impl From<String> for ExportFailure {
    fn from(e: String) -> Self {
        Self::Backend(ExportError::Msg(e))
    }
}
impl From<std::io::Error> for ExportFailure {
    fn from(e: std::io::Error) -> Self {
        Self::Backend(ExportError::Io(e))
    }
}
impl From<serde_json::Error> for ExportFailure {
    fn from(e: serde_json::Error) -> Self {
        Self::Backend(ExportError::Serde(e))
    }
}

#[derive(Debug)]
pub struct ExportOutcome {
    pub object_count: usize,
}
#[derive(Debug)]
pub struct EncryptedExportEstimate {
    pub object_count: usize,
    pub attachment_count: usize,
    pub attachment_selected_count: usize,
    pub estimated_bytes: u64,
    pub template_count: usize,
    pub template_names: Vec<String>,
}

/// 私有计划固定 payload、范围、密钥、附件和附加内容，不可由 IPC 或另一会话重建。
struct ExportExtra {
    name: &'static str,
    label: &'static [u8],
    content: Zeroizing<Vec<u8>>,
}

struct ExportPlan {
    profile: PackageProfile,
    payload: PayloadTemp,
    payload_size: u64,
    key: Zeroizing<[u8; 32]>,
    salt: [u8; 16],
    source_key: Option<Zeroizing<[u8; 32]>>,
    entries: Vec<ExportAttachmentEntry>,
    manifest: serde_json::Value,
    extras: Vec<ExportExtra>,
    object_count: usize,
}

/// 原会话执行。授权输出路径由可信调用方决定，Core 不解释平台白名单。
pub fn execute_encrypted_export(
    svc: &VaultService,
    session: &VaultSession,
    req: &EncryptedExportRequest<'_>,
    path: &Path,
) -> Result<ExportOutcome, ExportFailure> {
    let _activity = crate::import_activity::begin_owned_root_activity(session.root_owner())?;
    let plan = prepare_session_export(svc, session, req)?;
    let count = plan.object_count;
    execute_plan(plan, path, |output| {
        publish_for_session(svc, session, output, path, count)
    })
}

fn prepare_session_export(
    svc: &VaultService,
    session: &VaultSession,
    req: &EncryptedExportRequest<'_>,
) -> Result<ExportPlan, ExportFailure> {
    let vault = session.vault();
    let account_id = session.account_id();
    let source_key = svc.attachment_key_for_session(session)?;
    validate_export_password(svc, account_id, req.password)?;
    svc.with_session(session, |_| Ok(()))?;
    let records = collect_advanced_objects(vault, account_id, req.scope)?;
    if records.is_empty() && !req.scope.include_all {
        return Err(ExportFailure::NoObjectsSelected);
    }
    let templates: Vec<serde_json::Value> =
        collect_advanced_templates(vault, account_id, req.scope, &records)?
            .iter()
            .filter_map(|tpl| serde_json::to_value(tpl).ok())
            .collect();
    let snapshots = collect_object_snapshots(vault, &records)?;
    let payload = serialize_export_payload(&records, &templates, &snapshots)?;
    let (payload, payload_size) = write_payload_to_temp(svc.base_path(), &payload)?;
    let salt = solosoul_crypto::kdf::generate_salt();
    let key = derive_export_key(req.password, &salt)?;
    let (entries, attachment_bytes) =
        collect_advanced_attachments(svc, &records, &req.scope.attachments)?;
    if payload_size + attachment_bytes + entries.len() as u64 * 28 > MAX_EXPORT_TOTAL_BYTES {
        return Err(ExportFailure::TotalSizeExceeded);
    }
    let mut extras = Vec::new();
    if req.scope.include_preferences {
        if let Ok(Some(profile)) = vault.load_profile(account_id) {
            extras.push(ExportExtra {
                name: "preferences.enc",
                label: b"solosoul:preferences:v1",
                content: Zeroizing::new(profile.data),
            });
        }
    }
    if req.scope.include_behavioral {
        if let Ok(logs) = vault.list_audit_log(MAX_AUDIT_LOG_EXPORT) {
            let data = serde_json::to_vec(&logs).unwrap_or_default();
            if !data.is_empty() {
                extras.push(ExportExtra {
                    name: "behavioral.enc",
                    label: b"solosoul:behavioral:v1",
                    content: Zeroizing::new(data),
                });
            }
        }
    }
    let extra_names = extras
        .iter()
        .map(|e| e.name.to_string())
        .collect::<Vec<_>>();
    let manifest = build_manifest_json(
        req.scope,
        records.len(),
        !entries.is_empty(),
        extra_names.iter().any(|x| x == "preferences.enc"),
        extra_names.iter().any(|x| x == "behavioral.enc"),
        !templates.is_empty(),
        &extra_names,
        req.password_hint,
        &salt,
        req.app_version,
    );
    Ok(ExportPlan {
        profile: PackageProfile::Advanced,
        payload,
        payload_size,
        key,
        salt,
        source_key: Some(source_key),
        entries,
        manifest,
        extras,
        object_count: records.len(),
    })
}

/// 旧公开 API 适配。保持原包 profile 和缺源解密密钥错误，共用原子 writer。
pub(super) fn execute_legacy_export(
    vault: &VaultStore,
    account_id: &str,
    password: &str,
    path: &Path,
    scope: &ExportScope,
    base: &Path,
    attachments: &AttachmentExportScope,
) -> Result<usize, ExportError> {
    let _activity = crate::import_activity::begin_owned_root_activity(vault.root_owner())?;
    let records = super::collect_scope_objects(vault, account_id, scope)?;
    if records.is_empty() {
        return Err(ExportError::Msg("没有选中任何对象".into()));
    }
    let payload_value = build_payload(vault, &records);
    let has_templates = payload_value["templates"]
        .as_array()
        .is_some_and(|a| !a.is_empty());
    let (payload, payload_size) = write_payload_to_temp(base, &payload_value)?;
    let salt = solosoul_crypto::kdf::generate_salt();
    let key = derive_export_key(password, &salt)?;
    let attachments = super::collect_attachment_entries(base, &records, attachments)?;
    let bytes: u64 = attachments
        .iter()
        .map(|(_, _, _, src)| std::fs::metadata(src).map(|m| m.len()).unwrap_or(0))
        .sum();
    if payload_size + bytes + attachments.len() as u64 * 28 > MAX_EXPORT_TOTAL_BYTES {
        return Err(ExportError::Msg("导出包总大小超过限制".into()));
    }
    let entries = attachments
        .into_iter()
        .map(|(obj_id, att_id, _, src)| ExportAttachmentEntry {
            obj_id,
            att_id,
            src,
        })
        .collect::<Vec<_>>();
    let manifest = build_manifest(scope, &records, !entries.is_empty(), has_templates, &salt);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let count = records.len();
    let plan = ExportPlan {
        profile: PackageProfile::LegacyDirect,
        payload,
        payload_size,
        key,
        salt,
        source_key: None,
        entries,
        manifest,
        extras: vec![],
        object_count: count,
    };
    execute_plan(plan, path, |output| {
        output
            .persist(path)
            .map_err(|e| format!("Publish ZIP: {}", e.error))?;
        log_export(vault, path, count);
        Ok(())
    })
    .map(|outcome| outcome.object_count)
    .map_err(|e| match e {
        ExportFailure::Backend(e) => e,
        other => ExportError::Msg(other.to_string()),
    })
}

/// 两种包 profile 唯一的 ZIP 写入、收尾与发布流程。
fn execute_plan(
    plan: ExportPlan,
    path: &Path,
    publish: impl FnOnce(tempfile::TempPath) -> Result<(), String>,
) -> Result<ExportOutcome, ExportFailure> {
    let (file, output) = create_export_output(path)?;
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let zip = write_export_output(file, |zip| {
        write_attachment_entries(
            zip,
            options,
            &plan.key,
            &plan.salt,
            &plan.entries,
            plan.source_key.as_deref(),
        )
        .map_err(|e| e.to_string())?;
        for extra in &plan.extras {
            write_encrypted_extra(
                zip,
                options,
                &plan.key,
                &plan.salt,
                extra.label,
                extra.name,
                &extra.content,
            )?;
        }
        match plan.profile {
            PackageProfile::Advanced => write_manifest_and_payload(
                zip,
                options,
                &plan.manifest,
                plan.payload.path(),
                plan.payload_size,
                &plan.key,
            ),
            PackageProfile::LegacyDirect => {
                // Legacy 的错误上下文和字段由兼容策略保留。
                let data = serde_json::to_vec_pretty(&plan.manifest).map_err(|e| e.to_string())?;
                zip.start_file("manifest.json", options)
                    .map_err(|e| format!("写入 manifest 条目失败: {e}"))?;
                zip.write_all(&data)
                    .map_err(|e| format!("写入 manifest 数据失败: {e}"))?;
                zip.start_file("payload.enc", options)
                    .map_err(|e| format!("写入 payload 条目失败: {e}"))?;
                let mut reader = std::io::BufReader::new(
                    File::open(plan.payload.path()).map_err(|e| e.to_string())?,
                );
                solosoul_crypto::cipher::encrypt_chunked_stream(
                    &plan.key,
                    plan.payload_size,
                    &mut reader,
                    zip,
                )
                .map_err(|e| format!("加密 payload 流失败: {e}"))
            }
        }
    })?;
    finish_export_output(zip, output, File::sync_all, publish)?;
    Ok(ExportOutcome {
        object_count: plan.object_count,
    })
}

fn log_export(vault: &VaultStore, path: &Path, count: usize) {
    if let Err(e) = vault.log_structured(
        "export_execute",
        "export",
        None,
        None,
        "user",
        Some(&format!("exported {} objects to {}", count, path.display())),
    ) {
        tracing::warn!("export audit failed: {e}");
    }
}
fn publish_for_session(
    svc: &VaultService,
    session: &VaultSession,
    output: tempfile::TempPath,
    path: &Path,
    count: usize,
) -> Result<(), String> {
    svc.with_session(session, |vault| {
        output
            .persist(path)
            .map_err(|e| format!("Publish ZIP: {}", e.error))?;
        log_export(vault, path, count);
        Ok(())
    })
}
/// 保留 RF-017 真实收尾故障入口；只在原 session 中发布。
pub fn finalize_export_for_session(
    svc: &VaultService,
    session: &VaultSession,
    zip: ZipWriter<File>,
    output: tempfile::TempPath,
    path: &Path,
    count: usize,
) -> Result<(), String> {
    finish_export_output(zip, output, File::sync_all, |output| {
        publish_for_session(svc, session, output, path, count)
    })
}

const MAX_AUDIT_LOG_EXPORT: usize = 100_000;
pub fn collect_advanced_objects(
    vault: &solosoul_vault::VaultStore,
    account_id: &str,
    scope: &EncryptedExportScope,
) -> Result<Vec<solosoul_vault::ObjectRecord>, String> {
    // P005 复核：include_all 分支此前 list_objects（全量解密）取 id + load_objects_batch
    // （再解密）双重解密。全量导出直接一次 list_object_records（已解密完整记录）
    // 按页面/标签过滤即可，避免对同一批数据解密两遍。
    if scope.include_all {
        let mut records = vault.list_object_records(account_id)?;
        records.retain(|r| {
            let page_ok = scope.selected_page_ids.is_empty()
                || scope.selected_page_ids.contains(&r.section_type);
            let tag_ok = scope.selected_tags.is_empty()
                || r.tags_json.iter().any(|t| scope.selected_tags.contains(t));
            page_ok && tag_ok
        });
        records.sort_by(|a, b| a.id.cmp(&b.id));
        return Ok(records);
    }

    // P003: selected 分支此前用 list_objects（全库解密 properties 仅为筛 id），随后
    // load_objects_batch 再解密一次——双重解密。现改用 metadata-only + 明文 tags_json
    // 的 list_object_metadata_with_tags（纯 SQL，不解密 properties），命中对象才解密。
    let all = vault.list_object_metadata_with_tags(account_id, None, None, false, false)?;
    let mut selected_ids: BTreeSet<String> = scope.selected_object_ids.iter().cloned().collect();

    // Add all IDs belonging to selected pages
    for summary in &all {
        if !scope.selected_page_ids.is_empty()
            && scope.selected_page_ids.contains(&summary.section_type)
        {
            selected_ids.insert(summary.id.clone());
        }
    }

    // Filter by tags (P2): if selected_tags is non-empty, keep only objects with ANY matching tag
    if !scope.selected_tags.is_empty() {
        selected_ids.retain(|id| {
            all.iter()
                .any(|s| s.id == *id && s.tags.iter().any(|t| scope.selected_tags.contains(t)))
        });
    }

    if selected_ids.is_empty() {
        return Ok(Vec::new());
    }

    let id_list: Vec<String> = selected_ids.into_iter().collect();
    let by_id = vault.load_objects_batch(&id_list)?;
    let mut records: Vec<solosoul_vault::ObjectRecord> = by_id.into_values().collect();
    // 保持确定顺序：按 id 升序（与旧实现遍历 BTreeSet 的返回顺序一致）。
    records.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(records)
}

pub fn collect_referenced_templates(
    vault: &solosoul_vault::VaultStore,
    records: &[solosoul_vault::ObjectRecord],
) -> Vec<solosoul_vault::UserTemplate> {
    let template_ids: BTreeSet<String> = records
        .iter()
        .filter_map(|r| r.template_id.clone())
        .collect();
    template_ids
        .iter()
        .filter_map(|tid| vault.load_user_template(tid).ok().flatten())
        .collect()
}

pub fn collect_advanced_templates(
    vault: &solosoul_vault::VaultStore,
    account_id: &str,
    scope: &EncryptedExportScope,
    records: &[solosoul_vault::ObjectRecord],
) -> Result<Vec<solosoul_vault::UserTemplate>, String> {
    if scope.include_all {
        let mut all = vault.list_user_templates(account_id)?;
        all.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(all)
    } else {
        Ok(collect_referenced_templates(vault, records))
    }
}

fn validate_export_password(
    svc: &crate::vault_service::VaultService,
    account_id: &str,
    password: &str,
) -> Result<(), ExportFailure> {
    // ── Validate password (any non-empty password is accepted, P0-008) ──
    if password.is_empty() {
        return Err(ExportFailure::PasswordEmpty);
    }

    // ── Verify export password is NOT the master password ──────
    // P012：走阶梯锁定路径（失败计数/锁定与解锁一致，消除无限速布尔 oracle）
    match svc.verify_password_with_lockout(account_id, password) {
        Ok(true) => Err(ExportFailure::SameAsMasterPassword),
        Ok(false) => Ok(()), // export password is different from master password — OK
        Err(e) => Err(ExportFailure::MasterVerifyFailed(e)),
    }
}

fn collect_advanced_attachments(
    svc: &crate::vault_service::VaultService,
    records: &[solosoul_vault::ObjectRecord],
    scope: &AttachmentExportScope,
) -> Result<(Vec<super::ExportAttachmentEntry>, u64), ExportFailure> {
    if matches!(scope, AttachmentExportScope::None) {
        return Ok((Vec::new(), 0));
    }
    let mut entries: Vec<super::ExportAttachmentEntry> = Vec::new();
    let mut total_bytes: u64 = 0;

    for rec in records {
        let atts = load_attachments(&rec.properties);
        if atts.is_empty() {
            continue;
        }
        let base_dir = svc.base_path().join("attachments").join(&rec.id);
        for att in &atts {
            if att.deleted_at.is_some() {
                continue;
            }
            // 先过滤范围，未选中的超大或非法路径附件不能阻断有效选择。
            if !scope.includes(&att.id) {
                continue;
            }
            // Single attachment size limit
            if att.size_bytes > MAX_ATTACHMENT_BYTES {
                return Err(ExportFailure::AttachmentTooLarge(att.file_name.clone()));
            }

            let src = super::resolve_attachment_src(
                &base_dir,
                att.vault_path.as_deref(),
                att.src_path.as_deref(),
                &att.id,
                &att.file_name,
            );

            if let Some(src) = src {
                validate_attachment_path(svc.base_path().join("attachments").as_path(), &src)?;
                total_bytes += att.size_bytes;
                entries.push(super::ExportAttachmentEntry {
                    obj_id: rec.id.clone(),
                    att_id: att.id.clone(),
                    src,
                });
            }
        }
    }
    Ok((entries, total_bytes))
}

fn validate_attachment_path(base: &std::path::Path, path: &std::path::Path) -> Result<(), String> {
    let base_abs = std::path::absolute(base).map_err(|e| e.to_string())?;
    let path_abs = std::path::absolute(path).map_err(|e| e.to_string())?;
    if !path_abs.starts_with(&base_abs) {
        return Err(format!(
            "Attachment path escapes vault attachments directory: {}",
            path.display()
        ));
    }
    Ok(())
}

fn write_encrypted_extra(
    zip: &mut ZipWriter<File>,
    options: SimpleFileOptions,
    key: &[u8; 32],
    salt: &[u8],
    label: &[u8],
    file_name: &str,
    content: &[u8],
) -> Result<String, String> {
    let extra_key = solosoul_crypto::hkdf_ext::derive_hkdf_key(key, salt, label)
        .map_err(|e| format!("derive {file_name} key: {e}"))?;
    let enc = solosoul_crypto::cipher::encrypt_to_bytes(&extra_key, content, None)
        .map_err(|e| format!("encrypt {file_name}: {e}"))?;
    zip.start_file(file_name, options)
        .map_err(|e| e.to_string())?;
    zip.write_all(&enc).map_err(|e| e.to_string())?;
    Ok(file_name.to_string())
}

fn write_manifest_and_payload(
    zip: &mut ZipWriter<File>,
    options: SimpleFileOptions,
    manifest: &serde_json::Value,
    payload_path: &std::path::Path,
    payload_size: u64,
    key: &[u8; 32],
) -> Result<(), String> {
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    zip.start_file("manifest.json", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(&manifest_bytes).map_err(|e| e.to_string())?;

    // ── payload.enc (encrypted via streaming chunked cipher — P1-023) ──
    zip.start_file("payload.enc", options)
        .map_err(|e| e.to_string())?;
    {
        let mut reader = std::io::BufReader::new(
            std::fs::File::open(payload_path).map_err(|e| format!("open payload tmp: {e}"))?,
        );
        solosoul_crypto::cipher::encrypt_chunked_stream(key, payload_size, &mut reader, zip)
            .map_err(|e| format!("encrypt payload stream: {e}"))?;
    }
    Ok(())
}

fn collect_object_snapshots(
    vault: &solosoul_vault::VaultStore,
    records: &[solosoul_vault::ObjectRecord],
) -> Result<Vec<serde_json::Value>, String> {
    let object_ids: Vec<String> = records.iter().map(|r| r.id.clone()).collect();
    let mut snapshots: Vec<serde_json::Value> = Vec::new();
    for (object_id, meta, data) in vault.list_snapshots_with_data_batch(&object_ids)? {
        snapshots.push(serde_json::json!({
            "object_id": object_id,
            "timestamp": meta["timestamp"],
            "triggered_by": meta["triggeredBy"],
            "diff_summary": meta["diffSummary"],
            "data": base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                &data
            ),
        }));
    }
    Ok(snapshots)
}

fn serialize_export_payload(
    records: &[solosoul_vault::ObjectRecord],
    templates: &[serde_json::Value],
    snapshots: &[serde_json::Value],
) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "objects": records.iter().map(|r| serde_json::json!({
            "id": r.id,
            "account_id": r.account_id,
            "type_id": r.type_id,
            "section_type": r.section_type,
            "name": r.name,
            "icon_name": r.icon_name,
            "parent_id": r.parent_id,
            "children_ids": r.children_ids,
            "properties": r.properties,
            "property_labels": r.property_labels,
            "sensitivity_level": r.sensitivity_level,
            "contract_type_id": r.contract_type_id,
            "tags": r.tags_json,
            "created_at": r.created_at,
            "updated_at": r.updated_at,
            "version": r.version,
            "template_id": r.template_id,
            "template_type": r.template_type,
        })).collect::<Vec<_>>(),        "templates": templates,
        "snapshots": snapshots,
    }))
}

#[allow(clippy::too_many_arguments)]
fn build_manifest_json(
    scope: &EncryptedExportScope,
    object_count: usize,
    has_attachments: bool,
    has_preferences: bool,
    has_behavioral: bool,
    has_templates: bool,
    extra_files: &[String],
    password_hint: &Option<String>,
    salt: &[u8],
    app_version: &str,
) -> serde_json::Value {
    serde_json::json!({
        "version": "2.0",
        "export_scope": if scope.selected_page_ids.is_empty() && scope.selected_object_ids.is_empty() { "full" } else { "partial" },
        "selected_pages": scope.selected_page_ids,
        "selected_objects": scope.selected_object_ids,
        "selected_tags": scope.selected_tags,
        "object_count": object_count,
        "export_time": chrono::Utc::now().to_rfc3339(),
        "export_platform": std::env::consts::OS,
        "export_app_version": app_version,
        "has_attachments": has_attachments,
        "has_preferences": has_preferences,
        "has_behavioral": has_behavioral,
        "has_templates": has_templates,
        "extra_files": extra_files,
        "password_hint": password_hint.clone().unwrap_or_default(),
        "salt_hex": hex::encode(salt),
        // P202: 导出包携带实际 KDF 参数，导入端按声明派生（旧包无此字段回退 balanced）。
        // from_env()：release 为 production（OWASP 推荐 64MiB/3iter），debug 为 development。
        "kdf": super::kdf_to_manifest_value(
            &solosoul_crypto::kdf::KdfConfig::from_env(),
        ),
    })
}

pub fn estimate_attachments(
    records: &[solosoul_vault::ObjectRecord],
    scope: &AttachmentExportScope,
) -> (usize, usize, u64) {
    if matches!(scope, AttachmentExportScope::None) {
        return (0, 0, 0);
    }
    let mut available = 0usize;
    let mut selected = 0usize;
    let mut bytes = 0u64;
    for record in records {
        for attachment in load_attachments(&record.properties) {
            if attachment.deleted_at.is_some() {
                continue;
            }
            available += 1;
            if scope.includes(&attachment.id) {
                selected += 1;
                bytes += attachment.size_bytes;
            }
        }
    }
    (available, selected, bytes)
}

pub fn estimate_encrypted_export(
    vault: &VaultStore,
    account_id: &str,
    scope: &EncryptedExportScope,
) -> Result<EncryptedExportEstimate, String> {
    let records = collect_advanced_objects(vault, account_id, scope)?;
    let count = records.len();

    // 与导出执行（export_execute）共用同一收集逻辑，
    // 保证「导出前展示的模板清单」与最终包内 templates 一致
    let templates = collect_advanced_templates(vault, account_id, scope, &records)?;
    let template_count = templates.len();
    let template_names: Vec<String> = templates.iter().map(|t| t.name.clone()).collect();
    // P003: 对象 payload 体积用纯 SQL SUM(LENGTH(properties)) 估算（不解密、不重新序列化）；
    // 密文长度略大于明文（AES-GCM tag/nonce），加上 name 长度与固定开销更贴近导出包实际体积。
    let ids: Vec<String> = records.iter().map(|r| r.id.clone()).collect();
    let props_bytes = vault.objects_size_batch(&ids).unwrap_or(0);
    let name_bytes: u64 = records.iter().map(|r| r.name.len() as u64).sum();
    let mut estimated_bytes: u64 = props_bytes + name_bytes + (records.len() as u64 * 256);

    // RF-015：入口仅适配一次；可用附件与实际选中附件保持独立统计。
    let attachment_scope = scope.attachments.clone();
    let (attachment_count, attachment_selected_count, attachment_bytes) =
        estimate_attachments(&records, &attachment_scope);
    estimated_bytes += attachment_bytes;

    // Estimate snapshots payload（历史记录，恢复包保证历史数量一致）
    // 按实际加密后字节数估算（snapshots_size_batch 为 LENGTH(data) 之和），
    // base64 编码后再膨胀约 1/3，此处按 1.4x 折算。
    if !records.is_empty() {
        let ids: Vec<String> = records.iter().map(|r| r.id.clone()).collect();
        if let Ok(bytes) = vault.snapshots_size_batch(&ids) {
            estimated_bytes += (bytes as f64 * 1.4) as u64;
        }
    }

    // Estimate preferences payload
    if scope.include_preferences {
        estimated_bytes += 4096; // rough guess
    }

    // Estimate behavioral data (audit log)
    if scope.include_behavioral {
        if let Ok(logs) = vault.list_audit_log(MAX_AUDIT_LOG_EXPORT) {
            let log_json = serde_json::to_vec(&logs).unwrap_or_default();
            estimated_bytes += log_json.len() as u64;
        }
    }

    Ok(EncryptedExportEstimate {
        object_count: count,
        attachment_count,
        attachment_selected_count,
        estimated_bytes,
        template_count,
        template_names,
    })
}

#[cfg(test)]
mod rf023;
