use super::*;
use solosoul_core::{VaultService, VaultSession};
use std::sync::{Arc, RwLock};

// ── Export commands ────────────────────────────────────────────

/// P039: 系统分区（侧栏顺序）+ 显示名单一来源——数组/映射/集合均由它派生，
/// 新增分区只需在此追加一处。
const SYSTEM_SECTIONS: &[(&str, &str)] = &[
    ("identity", "Identity"),
    ("travel", "Travel"),
    ("financial", "Financial"),
    ("professional", "Professional"),
    ("note", "Notes"),
    ("document", "Documents"),
];

#[tauri::command]
pub async fn export_get_scope_tree(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<Vec<PageGroup>, String> {
    let vault = vault_handle(&state)?;

    // 阶段 1：拉取对象 + 模板映射
    let objects = vault
        .list_objects(&account_id, None, None, None, false, false)
        .map_err(|e| format!("list_objects: {}", e))?;
    let template_map: std::collections::HashMap<String, solosoul_vault::UserTemplate> = vault
        .list_user_templates(&account_id)
        .unwrap_or_default()
        .into_iter()
        .map(|t| (t.id.clone(), t))
        .collect();

    // 阶段 2：合并模板敏感度（与导出 preflight object_max_sensitivity 口径一致）
    let objects = merge_template_sensitivities(objects, &template_map);

    // 阶段 3：收集自定义页面（page 类型对象）与分组
    let (custom_pages, custom_page_order) = collect_custom_pages(&objects);
    let custom_page_ids: std::collections::HashSet<String> = custom_pages.keys().cloned().collect();
    let groups = group_objects(objects, &custom_page_ids);

    // 阶段 4：组装 PageGroup 列表（系统分区 → 自定义页面 → 剩余/孤儿过滤）
    let result = build_page_groups(groups, &custom_pages, &custom_page_order);

    Ok(result)
}

/// P037: 模板敏感度合并——list_objects 已从 property_labels/__fields/dynamic_group 推导，
/// 此处再并入模板定义的 sensitivity_level，并按敏感度升序去重后写回（范围树展示用）。
fn merge_template_sensitivities(
    objects: Vec<ObjectSummary>,
    template_map: &std::collections::HashMap<String, solosoul_vault::UserTemplate>,
) -> Vec<ObjectSummary> {
    objects
        .into_iter()
        .map(|mut o| {
            if let Some(ref tid) = o.template_id {
                if let Some(tpl) = template_map.get(tid) {
                    for prop in &tpl.properties {
                        if let Some(ref sl) = prop.sensitivity_level {
                            if !o.sensitivity_levels.contains(sl) {
                                o.sensitivity_levels.push(sl.clone());
                            }
                        }
                    }
                }
            }
            o.sensitivity_levels
                .sort_by_key(|l| solosoul_vault::sensitivity_rank(l));
            o
        })
        .collect()
}

/// P037: 收集自定义页面对象（type_id = "page"）为 lookup：page_id -> (name, icon)，
/// 保持 list_objects 的出现顺序。
fn collect_custom_pages(
    objects: &[ObjectSummary],
) -> (
    std::collections::HashMap<String, (String, String)>,
    Vec<String>,
) {
    let mut custom_pages: std::collections::HashMap<String, (String, String)> =
        std::collections::HashMap::new();
    let mut custom_page_order: Vec<String> = Vec::new();
    for obj in objects {
        if obj.collection_type == "page" && !custom_pages.contains_key(&obj.id) {
            custom_pages.insert(obj.id.clone(), (obj.name.clone(), obj.icon_name.clone()));
            custom_page_order.push(obj.id.clone());
        }
    }
    (custom_pages, custom_page_order)
}

/// P037: 按 section_type/collection_type 将非页面对象分组。
fn group_objects(
    objects: Vec<ObjectSummary>,
    custom_page_ids: &std::collections::HashSet<String>,
) -> std::collections::HashMap<String, Vec<ObjectSummary>> {
    let mut groups: std::collections::HashMap<String, Vec<ObjectSummary>> =
        std::collections::HashMap::new();
    for obj in objects {
        // Skip page-defining objects — they are already represented as section headers
        // and should not appear as duplicate items inside their own page section.
        if obj.collection_type == "page" {
            continue;
        }
        let group_key =
            if !obj.section_type.is_empty() && custom_page_ids.contains(&obj.section_type) {
                // Object belongs to a custom page — use page ID as group key
                obj.section_type.clone()
            } else if !obj.section_type.is_empty() {
                obj.section_type.clone()
            } else if !obj.collection_type.is_empty() {
                obj.collection_type.clone()
            } else {
                "uncategorized".to_string()
            };
        groups.entry(group_key).or_default().push(obj);
    }
    groups
}

/// P037: 组装 PageGroup 列表——系统分区（侧栏顺序）→ 自定义页面（出现顺序）→
/// 剩余分组（过滤孤儿 UUID：软删除自定义页面残留的子对象，pre-P0-1 bug）。
fn build_page_groups(
    mut groups: std::collections::HashMap<String, Vec<ObjectSummary>>,
    custom_pages: &std::collections::HashMap<String, (String, String)>,
    custom_page_order: &[String],
) -> Vec<PageGroup> {
    // P039: 系统分区从单一 SYSTEM_SECTIONS 派生（侧栏顺序 + 显示名）
    let system_keys: Vec<&str> = SYSTEM_SECTIONS.iter().map(|(k, _)| *k).collect();

    let mut result = Vec::new();

    // 1. System sections in sidebar order
    for key in &system_keys {
        if let Some(objs) = groups.remove(*key) {
            let display = SYSTEM_SECTIONS
                .iter()
                .find(|(k, _)| *k == *key)
                .map(|(_, name)| name.to_string())
                .unwrap_or_else(|| key.to_string());
            result.push(PageGroup {
                section_type: key.to_string(),
                page_name: display,
                object_count: objs.len(),
                objects: objs,
            });
        }
    }

    // 2. Custom page groups in order they appear from list_objects
    for page_id in custom_page_order {
        let (page_name, _icon) = &custom_pages[page_id];
        let objs = groups.remove(page_id.as_str()).unwrap_or_default();
        result.push(PageGroup {
            section_type: page_id.clone(),
            page_name: page_name.clone(),
            object_count: objs.len(),
            objects: objs,
        });
    }

    // 3. Any remaining groups (uncategorized, etc.)
    // Filter out orphan UUID groups that belong to already-deleted custom pages.
    // These appear when a custom page was soft-deleted without its child objects
    // (pre-P0-1 bug), leaving orphan objects with section_type = page UUID.
    // P039: 系统分区集合由 SYSTEM_SECTIONS 派生（另含 uncategorized）
    let system_sections_set: std::collections::HashSet<&str> = SYSTEM_SECTIONS
        .iter()
        .map(|(k, _)| *k)
        .chain(std::iter::once("uncategorized"))
        .collect();

    let mut remaining: Vec<(String, Vec<ObjectSummary>)> = groups
        .into_iter()
        .filter(|(key, _)| {
            // Keep non-UUID keys (e.g. "uncategorized") and UUIDs that match known custom pages
            if system_sections_set.contains(key.as_str()) {
                return true;
            }
            if uuid::Uuid::parse_str(key).is_err() {
                return true;
            }
            // UUID key: only keep if it's a known custom page
            custom_pages.contains_key(key)
        })
        .collect();
    remaining.sort_by(|a, b| a.0.cmp(&b.0));
    for (st, objs) in remaining {
        result.push(PageGroup {
            section_type: st.clone(),
            page_name: st,
            object_count: objs.len(),
            objects: objs,
        });
    }
    result
}

#[tauri::command]
pub async fn export_estimate_size(
    state: State<'_, AppState>,
    account_id: String,
    scope: ExportScope,
) -> Result<ExportEstimate, String> {
    let vault = vault_handle(&state)?;
    let e = solosoul_core::export_import::export::estimate_encrypted_export(
        &vault,
        &account_id,
        &scope.to_core_scope(scope.attachment_export_scope()),
    )?;
    Ok(ExportEstimate {
        object_count: e.object_count,
        attachment_count: e.attachment_count,
        attachment_selected_count: e.attachment_selected_count,
        estimated_bytes: e.estimated_bytes,
        template_count: e.template_count,
        template_names: e.template_names,
    })
}

/// 估算范围内的未删除附件：(可用数量, 选中数量, 选中元数据声明字节数)。
/// None 不读取附件元数据；Selected(empty) 仍报告可用数量，选中与字节数为零。
#[cfg(test)]
pub(super) fn estimate_attachments(
    records: &[solosoul_vault::ObjectRecord],
    scope: &AttachmentExportScope,
) -> (usize, usize, u64) {
    solosoul_core::export_import::export::estimate_attachments(records, scope)
}

// ── Export execution helpers ─────────────────────────────────

/// 解析保存路径（支持 ~/ 前缀）并追加 .solosoul 后缀，确保父目录存在。
///
/// P001: 桌面端在返回前强制校验落盘位置位于允许基目录（Desktop/Documents/Downloads
/// /SOLOSOUL_FS_BASE）内，与 `attachment_download` 同白名单——防 XSS 后以应用权限向任意
/// 路径写入攻击者可控 zip。移动端前端仅能经 SAF URI/staging 中转（无法传任意路径），跳过校验。
#[allow(unused_variables)]
fn resolve_zip_path(app: &tauri::AppHandle, save_path: &str) -> Result<String, String> {
    let resolved = if save_path.starts_with("~/") {
        #[cfg(mobile)]
        {
            app.path()
                .resolve(&save_path[2..], tauri::path::BaseDirectory::Data)
                .map_err(|e| format!("无法解析应用数据目录: {e}"))?
                .to_string_lossy()
                .to_string()
        }
        #[cfg(desktop)]
        {
            let home = std::env::var("HOME").map_err(|_| {
                "HOME environment variable not set; cannot resolve ~/ in save path".to_string()
            })?;
            home + &save_path[1..]
        }
    } else {
        save_path.to_string()
    };
    let zip_path = if resolved.ends_with(".solosoul") {
        resolved
    } else {
        format!("{resolved}.solosoul")
    };

    // P001: 桌面端落盘位置白名单校验（拒绝 `..` 组件 + 必须在 allowed_fs_bases 内）。
    #[cfg(desktop)]
    validate_export_dest(&zip_path)?;

    if let Some(parent) = std::path::Path::new(&zip_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    Ok(zip_path)
}

/// P001: 校验导出落盘路径位于允许基目录内（与 `attachment_download` 的校验语义一致）。
/// P220: 提升为 `pub(crate)` 供 `export_docx` 复用（文档导出同样需要白名单校验）。
#[cfg(desktop)]
pub(crate) fn validate_export_dest(zip_path: &str) -> Result<(), String> {
    let dest = std::path::Path::new(zip_path);
    // 拒绝路径穿越组件
    if dest
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("导出路径不得包含 '..'".to_string());
    }

    let allowed_bases = crate::commands::attachment::allowed_fs_bases();
    // P015: 白名单为空时 fail-closed 拒绝（而非放行任意导出路径）
    if allowed_bases.is_empty() {
        tracing::warn!(
            "[export] allowed FS bases empty — rejecting export destination (fail-closed)"
        );
        return Err(
            "允许的路径白名单为空（Desktop/Documents/Downloads 与 SOLOSOUL_FS_BASE 均不可解析），已拒绝导出"
                .to_string(),
        );
    }

    // 目标不存在时 canonicalize 父目录（对齐 attachment_download 的宽容处理）
    let dest_canon = if dest.exists() {
        dest.canonicalize()
            .map_err(|e| format!("无法解析导出路径: {e}"))?
    } else if let Some(parent) = dest.parent() {
        if parent.exists() {
            parent
                .canonicalize()
                .map_err(|_| "无法解析导出路径父目录".to_string())?
        } else {
            return Err("导出路径父目录不存在".to_string());
        }
    } else {
        return Err("无效的导出路径".to_string());
    };

    let in_allowed = allowed_bases.iter().any(|base| {
        if dest_canon.starts_with(base) {
            return true;
        }
        if let Some(parent) = dest_canon.parent() {
            parent.starts_with(base)
        } else {
            false
        }
    });
    if !in_allowed {
        return Err(
            "导出位置必须在 Desktop、Documents、Downloads 或 SOLOSOUL_FS_BASE 内".to_string(),
        );
    }
    Ok(())
}

#[tauri::command]
pub async fn export_execute(
    #[allow(unused_variables)] app: tauri::AppHandle,
    state: State<'_, AppState>,
    account_id: String,
    req: ExportRequest,
) -> Result<String, String> {
    let job = ExportJob::prepare(Arc::clone(&state.vault_service), &account_id, req, |path| {
        resolve_zip_path(&app, path)
    })?;
    run_export_job(move || job.run()).await
}

/// RF-025：排队前固定原会话与授权路径，只将 owned 数据送入阻塞线程。
/// 路径解析器在 prepare 内同步执行；桌面白名单与移动端 staging 仍由命令负责。
pub(super) struct ExportJob {
    vault_service: Arc<RwLock<VaultService>>,
    session: VaultSession,
    scope: ExportScope,
    password: Zeroizing<String>,
    password_hint: Option<String>,
    zip_path: String,
}

impl ExportJob {
    pub(super) fn prepare(
        vault_service: Arc<RwLock<VaultService>>,
        account_id: &str,
        req: ExportRequest,
        resolve_path: impl FnOnce(&str) -> Result<String, String>,
    ) -> Result<Self, String> {
        let (zip_path, session) = {
            let svc = vault_service
                .read()
                .map_err(|_| "Vault service lock poisoned".to_string())?;
            let zip_path = resolve_path(&req.save_path)?;
            (zip_path, svc.capture_session(account_id)?)
        };
        Ok(Self {
            vault_service,
            session,
            scope: req.scope,
            password: Zeroizing::new(req.password),
            password_hint: req.password_hint,
            zip_path,
        })
    }

    pub(super) fn run(self) -> Result<String, String> {
        self.run_with_activity(|| {})
    }

    /// 在真实 worker 起步后登记 activity；排队期间仍允许旧会话正常失效。
    pub(super) fn run_with_activity(self, started: impl FnOnce()) -> Result<String, String> {
        // 只在阻塞线程内持服务读锁；不可重新捕获排队后的当前会话。
        let svc = self
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        let _activity =
            solosoul_core::import_activity::begin_owned_root_activity(svc.root_owner())?;
        started();
        let scope = self
            .scope
            .to_core_scope(self.scope.attachment_export_scope());
        let request = solosoul_core::export_import::export::EncryptedExportRequest {
            scope: &scope,
            password: &self.password,
            password_hint: &self.password_hint,
            app_version: env!("CARGO_PKG_VERSION"),
        };
        solosoul_core::export_import::export::execute_encrypted_export(
            &svc,
            &self.session,
            &request,
            std::path::Path::new(&self.zip_path),
        )
        .map_err(crate::services::encrypted_export::map_export_failure)?;
        Ok(self.zip_path)
    }
}

pub(super) async fn run_export_job(
    job: impl FnOnce() -> Result<String, String> + Send + 'static,
) -> Result<String, String> {
    tokio::task::spawn_blocking(job)
        .await
        // JoinError 可能携带 panic 内容，不能将其拼入面向用户的错误。
        .map_err(|_| "导出任务执行失败".to_string())?
}

/// 导出核心逻辑（与 `export_execute` 共享，供云同步快照、跨设备恢复复用）。
///
/// 与命令层的差异：
/// - 落盘路径必须由后端可信调用方生成，不能直接使用前端传入的路径；
/// - 密码校验同样在此执行（快照口令不得为主密码）。
///
/// 前置条件：`svc` 处于解锁态。
#[cfg(test)]
pub(crate) fn execute_export_core(
    svc: &solosoul_core::vault_service::VaultService,
    account_id: &str,
    req: &ExportRequest,
    zip_path: &str,
) -> Result<(), String> {
    let session = svc.capture_session(account_id)?;
    execute_export_for_session(svc, &session, req, zip_path)
}

/// 后台调用方显式提供附件范围；此入口只捕获一次会话，供恢复主机使用。
#[cfg(test)]
pub(crate) fn execute_export_core_with_attachment_scope(
    svc: &solosoul_core::VaultService,
    account_id: &str,
    req: &ExportRequest,
    zip_path: &str,
    attachment_scope: &AttachmentExportScope,
) -> Result<(), String> {
    let session = svc.capture_session(account_id)?;
    execute_export_for_session_with_attachment_scope(svc, &session, req, zip_path, attachment_scope)
}

/// 手动导出的兼容适配入口；既有请求字段与空选择含义保持不变。
#[cfg(test)]
pub(crate) fn execute_export_for_session(
    svc: &solosoul_core::VaultService,
    session: &solosoul_core::VaultSession,
    req: &ExportRequest,
    zip_path: &str,
) -> Result<(), String> {
    let attachment_scope = req.scope.attachment_export_scope();
    execute_export_for_session_with_attachment_scope(svc, session, req, zip_path, &attachment_scope)
}

/// 云同步固定起始会话；业务层只解释明确类型，不能在 KDF 后换读当前 Vault。
#[cfg(test)]
pub(crate) fn execute_export_for_session_with_attachment_scope(
    svc: &VaultService,
    session: &VaultSession,
    req: &ExportRequest,
    zip_path: &str,
    attachments: &AttachmentExportScope,
) -> Result<(), String> {
    let scope = req.scope.to_core_scope(attachments.clone());
    let request = solosoul_core::export_import::export::EncryptedExportRequest {
        scope: &scope,
        password: &req.password,
        password_hint: &req.password_hint,
        app_version: env!("CARGO_PKG_VERSION"),
    };
    solosoul_core::export_import::export::execute_encrypted_export(
        svc,
        session,
        &request,
        std::path::Path::new(zip_path),
    )
    .map(|_| ())
    .map_err(crate::services::encrypted_export::map_export_failure)
}

/// RF-017：收尾 IO 不持会话门闩，只有最终替换与成功审计在原会话内发布。
#[cfg(test)]
pub(super) fn finalize_export_for_session(
    svc: &VaultService,
    session: &VaultSession,
    zip: ZipWriter<File>,
    output: tempfile::TempPath,
    zip_path: &str,
    object_count: usize,
) -> Result<(), String> {
    solosoul_core::export_import::export::finalize_export_for_session(
        svc,
        session,
        zip,
        output,
        std::path::Path::new(zip_path),
        object_count,
    )
}

// ── Attachment info for export UI ──────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentInfo {
    pub id: String,
    pub file_name: String,
    pub size_bytes: u64,
}

/// P005: 批量读取对象附件（N+1 优化）——一次 `load_objects_batch` 解密多个对象，
/// 消除前端全选时逐对象 IPC + 逐对象整解密的 N+1 放大。返回 object_id → 附件列表。
#[tauri::command]
pub async fn export_get_attachments_batch(
    state: State<'_, AppState>,
    _account_id: String,
    object_ids: Vec<String>,
) -> Result<std::collections::HashMap<String, Vec<AttachmentInfo>>, String> {
    let vault = vault_handle(&state)?;

    let records = vault.load_objects_batch(&object_ids)?;
    let mut result = std::collections::HashMap::with_capacity(object_ids.len());
    for id in &object_ids {
        let atts = records
            .get(id)
            .map(|r| load_attachments(&r.properties))
            .unwrap_or_default();
        let infos: Vec<AttachmentInfo> = atts
            .into_iter()
            .filter(|a| a.deleted_at.is_none())
            .map(|a| AttachmentInfo {
                id: a.id,
                file_name: a.file_name,
                size_bytes: a.size_bytes,
            })
            .collect();
        result.insert(id.clone(), infos);
    }
    Ok(result)
}
