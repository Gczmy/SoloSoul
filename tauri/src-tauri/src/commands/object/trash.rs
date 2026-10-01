use crate::commands::settings::resolve_ui_prefs_path;
use crate::commands::{current_account, vault_handle};
use crate::state::AppState;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::State;

use super::snapshot::{load_trash_retention, retention_ms};

/// Result returned by object_restore / trash_restore describing what happened.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreOutcome {
    pub restored_id: String,
    pub name: String,
    pub cascaded_page_name: Option<String>,
    pub cascaded_count: u32,
    pub rebuilt_page_name: Option<String>,
    pub consumed_trash_ids: Vec<String>,
}

impl From<solosoul_core::objects::RestoreResult> for RestoreOutcome {
    fn from(result: solosoul_core::objects::RestoreResult) -> Self {
        Self {
            restored_id: result.restored_id,
            name: result.restored_name,
            cascaded_page_name: result.cascaded_page_name,
            cascaded_count: result.cascaded_count,
            rebuilt_page_name: result.rebuilt_page_name,
            consumed_trash_ids: result.consumed_trash_ids,
        }
    }
}

#[tauri::command]
pub async fn object_trash_list(
    state: State<'_, AppState>,
    account_id: String,
    since: Option<i64>,
) -> Result<Vec<solosoul_vault::TrashItemSummary>, crate::commands::error::BackendError> {
    let _ = account_id;
    let vault = super::errors::vault_handle(&state)?;
    // P114: 回收站全量解密移入 spawn_blocking，避免阻塞 tokio worker。
    tokio::task::spawn_blocking(move || {
        vault
            .list_trash_items(None, since)
            .map_err(super::errors::read)
    })
    .await
    .map_err(super::errors::task)?
}

/// Read the user's language setting from plaintext UI preferences.
fn get_ui_language<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    svc: &solosoul_core::vault_service::VaultService,
) -> String {
    let path = match resolve_ui_prefs_path(app, svc) {
        Ok(p) => p,
        Err(_) => return "en-US".to_string(),
    };
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(prefs) = serde_json::from_str::<serde_json::Value>(&content) {
                if prefs.is_object() {
                    if let Some(lang) = prefs.get("language").and_then(|v| v.as_str()) {
                        return lang.to_string();
                    }
                }
            }
        }
    }
    "en-US".to_string()
}

/// Restore an object from trash. Delegates to solosoul-core::objects::restore_from_trash_with_lang.
///
/// P002：历史 IPC 命令 `object_restore` 已从 handler/ACL 面移除（前端回收站统一走
/// `trash_restore`），本函数保留为 `trash_restore` 的内部共享助手（不再暴露为命令）。
pub async fn object_restore(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    trash_id: String,
    lang: Option<String>,
) -> Result<RestoreOutcome, String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let _activity = solosoul_core::import_activity::begin_owned_root_activity(svc.root_owner())?;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();
    let _trash = vault
        .get_trash_item(&trash_id)?
        .ok_or("Trash item not found")?;

    let fallback_lang = get_ui_language(&app, &svc);
    let lang = lang.as_deref().unwrap_or(&fallback_lang);

    let result = solosoul_core::objects::restore_from_trash_with_lang(vault, &trash_id, lang)?;
    state.auto_sync.trigger_debounce();
    state.device_auto_sync.trigger_data_change();

    Ok(result.into())
}

#[tauri::command]
pub async fn trash_restore(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    trash_id: String,
    lang: Option<String>,
) -> Result<RestoreOutcome, String> {
    object_restore(app, state, trash_id, lang).await
}

/// P014: 批量恢复——单次 IPC 在服务端循环处理（对齐 `trash_permanent_delete_batch`
/// 的批量入参约定），替代前端逐条 invoke 的 N 次串行往返。
/// - 模板项复用 `template_restore`（含「模板已存在」检查与审计）；
/// - 其余类型走 `object_restore`（级联恢复页面/子对象）；
/// - 已被级联恢复/已删除（trash 行已消费）的项**幂等跳过**，对齐单条路径
///   前端「Trash item not found 视为成功」的兜底语义，批量中途不因已恢复项失败；
/// - 真实错误（数据损坏/DB 异常）中止返回 Err，已恢复项保持已恢复（重试幂等）。
#[tauri::command]
pub async fn trash_restore_batch(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    trash_ids: Vec<String>,
    lang: Option<String>,
) -> Result<Vec<RestoreOutcome>, String> {
    let mut outcomes: Vec<RestoreOutcome> = Vec::with_capacity(trash_ids.len());
    for trash_id in &trash_ids {
        let item = {
            let vault = vault_handle(&state)?;
            vault.get_trash_item(trash_id)?
        };
        let Some(item) = item else {
            continue; // 已被级联恢复/已删除 → 幂等跳过
        };
        if item.item_type == "template" {
            let restored_id =
                crate::commands::template::template_restore(state.clone(), trash_id.clone())
                    .await?;
            outcomes.push(RestoreOutcome {
                restored_id,
                name: item.name_snapshot,
                cascaded_page_name: None,
                cascaded_count: 0,
                rebuilt_page_name: None,
                consumed_trash_ids: vec![trash_id.clone()],
            });
        } else {
            outcomes.push(
                object_restore(app.clone(), state.clone(), trash_id.clone(), lang.clone()).await?,
            );
        }
    }
    Ok(outcomes)
}

/// P024: 单条永久删除的共享实现（单删/批量命令复用；逐条自含事务与墓碑，
/// 与 `delete_object`/`delete_trash_item` 既有语义一致）。
pub(crate) fn permanent_delete_one(
    vault: &solosoul_vault::VaultStore,
    trash_id: &str,
) -> Result<(), String> {
    if let Ok(Some(trash)) = vault.get_trash_item(trash_id) {
        if trash.item_type != "template" {
            vault.delete_object(&trash.original_id, false)?;
        }
        crate::commands::log_audit_best_effort(
            vault,
            "trash_permanent_delete",
            "trash_item",
            Some(trash_id),
            Some(&trash.name_snapshot),
            "user",
            Some(&format!("original_id={}", trash.original_id)),
        );
        vault.delete_trash_item(trash_id).ok();
        return Ok(());
    }
    vault.delete_trash_item(trash_id).ok();
    crate::commands::log_audit_best_effort(
        vault,
        "trash_permanent_delete",
        "trash_item",
        Some(trash_id),
        None,
        "user",
        None,
    );
    Ok(())
}

/// P024: 批量永久删除——单次 IPC 在服务端循环处理（替代前端逐条 invoke，
/// 数百条时 N 次 IPC → 1 次）。任一失败中止并返回 Err（已删项保持已删，
/// 与既有并发逐条语义一致）；成功后触发一次自动同步/数据变更广播。
#[tauri::command]
pub async fn trash_permanent_delete_batch(
    state: State<'_, AppState>,
    trash_ids: Vec<String>,
) -> Result<usize, String> {
    let vault = vault_handle(&state)?;
    for id in &trash_ids {
        permanent_delete_one(&vault, id)?;
    }
    state.auto_sync.trigger_debounce();
    state.device_auto_sync.trigger_data_change();
    Ok(trash_ids.len())
}

/// 已获得的维护执行位，正常结束、错误或 panic 时都会释放。
pub(crate) struct CleanupGuard(Arc<std::sync::atomic::AtomicBool>);
impl Drop for CleanupGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

pub(crate) async fn acquire_cleanup_slot(
    service: &std::sync::RwLock<solosoul_core::VaultService>,
    session: &solosoul_core::VaultSession,
    running: Arc<std::sync::atomic::AtomicBool>,
) -> Result<CleanupGuard, String> {
    loop {
        let current = service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?
            .with_session(session, |_| Ok(()));
        current?;
        if running
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            return Ok(CleanupGuard(running));
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}

/// 解锁后按原会话排队维护；过期回收站与附件意图各自收尾，不阻塞解锁响应。
pub fn run_expired_trash_cleanup(state: &crate::state::AppState) {
    let session = match state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned")
        .and_then(|svc| {
            let account = svc.get_current_account().ok_or("Vault not unlocked")?;
            svc.capture_session(&account)
                .map_err(|_| "Vault session is no longer current")
        }) {
        Ok(session) => session,
        Err(_) => return,
    };
    let state = state.clone();
    tauri::async_runtime::spawn(async move {
        let guard = match acquire_cleanup_slot(
            &state.vault_service,
            &session,
            state.trash_cleanup_running.clone(),
        )
        .await
        {
            Ok(guard) => guard,
            Err(_) => return,
        };
        let result = tauri::async_runtime::spawn_blocking(move || {
            let _guard = guard;
            let svc = match state.vault_service.read() {
                Ok(svc) => svc,
                Err(_) => {
                    tracing::warn!("unlock maintenance service unavailable");
                    return;
                }
            };
            // 两个维护动作分别收尾；过期回收站失败不能阻断附件意图恢复。
            match svc.with_session(&session, |vault| vault.cleanup_expired_trash()) {
                Ok(count) if count > 0 => {
                    tracing::info!(count, "expired trash cleanup completed");
                    state.auto_sync.trigger_debounce();
                    state.device_auto_sync.trigger_data_change();
                }
                Ok(_) => {}
                Err(_) => tracing::warn!("expired trash cleanup failed"),
            }
            match solosoul_core::attachment_cleanup::retry_attachment_cleanup_for_session(
                &svc, &session,
            ) {
                Ok(report) if report.pending > 0 => tracing::warn!(
                    pending = report.pending,
                    "unlock maintenance left pending attachment cleanup"
                ),
                Ok(_) => {}
                Err(_) => tracing::warn!("attachment intent retry unavailable"),
            }
        })
        .await;
        if result.is_err() {
            tracing::warn!("unlock maintenance worker failed");
        }
    });
}

/// Delete a page (section_type) and all its objects into trash.
/// If `page_object_id` is provided, the custom page object is also deleted into trash.
#[tauri::command]
pub async fn page_delete(
    state: State<'_, AppState>,
    _account_id: String,
    section_type: String,
    page_object_id: Option<String>,
) -> Result<usize, String> {
    // P020: 服务端派生当前账户，忽略客户端 account_id（陈旧值会把删除目标对准错误账户）
    let account_id = current_account(&state)?;
    let vault = vault_handle(&state)?;

    // P114/P211: 全表筛选 + 批量加载 + 单事务批量入回收站/软删移入 spawn_blocking。
    // P211: 候选对象一次 list_object_metadata（metadata-only）筛选 + 一次 load_objects_batch
    // （单条 IN 查询 + 单轮解密，替代逐对象 load_object 的 N 次解密），回收站写入与
    // 软删收敛为 trash_and_soft_delete_batch 单事务（替代 N×2 次 auto-commit）。
    let count = tokio::task::spawn_blocking(move || -> Result<usize, String> {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let period = load_trash_retention(&vault, &account_id);
        let retention_ms = retention_ms(&period);

        let mut page_name = String::new();

        // 收集目标对象 ID：自定义页对象（如提供）+ 同 section/collection 的全部对象。
        let mut target_ids: Vec<String> = Vec::new();
        if let Some(pid) = &page_object_id {
            target_ids.push(pid.clone());
        }
        let objects = vault
            .list_object_metadata(&account_id, None, None, false, false)
            .map_err(|e| format!("list: {}", e))?;
        for obj in &objects {
            if obj.section_type == section_type || obj.collection_type == section_type {
                if page_name.is_empty() {
                    page_name = section_type.clone();
                }
                target_ids.push(obj.id.clone());
            }
        }

        // 去重：页对象可能同时命中 section 扫描（原实现先软删页对象再列对象，扫描天然
        // 排除；新实现先收集后删除，需显式去重，否则页对象会重复入回收站/重复计数）。
        let mut seen = std::collections::HashSet::new();
        target_ids.retain(|id| seen.insert(id.clone()));

        // 一次批量加载（单条 IN 查询 + 单轮解密）。
        let loaded = vault.load_objects_batch(&target_ids)?;

        // P019：条目构建拆至 build_page_delete_trash_items。
        let (trash_items, soft_delete_ids, page_name) = build_page_delete_trash_items(
            &target_ids,
            &loaded,
            page_object_id.as_deref(),
            page_name,
            now_ms,
            retention_ms,
        )?;

        // 单事务批量写入：回收站条目 + 对象软删，任一步失败整体回滚。
        vault.trash_and_soft_delete_batch(&trash_items, &soft_delete_ids)?;
        let count = trash_items.len();

        crate::commands::log_audit_best_effort(
            &vault,
            "page_delete",
            "page",
            Some(&section_type),
            if page_name.is_empty() {
                None
            } else {
                Some(&page_name)
            },
            "user",
            Some(&format!("count={}", count)),
        );
        Ok(count)
    })
    .await
    .map_err(|e| format!("page_delete task failed: {e}"))??;

    state.auto_sync.trigger_debounce();
    state.device_auto_sync.trigger_data_change();
    Ok(count)
}

/// P019：页删除的回收站条目构建（自 page_delete 拆出，逻辑逐字保持）。
///
/// 按收集顺序重建条目（页对象优先；名称用于审计）：页对象的快照不含父页名，
/// 其余对象写入 `parentPageName`/`parentPageIcon` 供回收站展示归属。返回
/// (条目, 软删 ID 列表, 审计用页名)。
#[allow(clippy::too_many_arguments)]
fn build_page_delete_trash_items(
    target_ids: &[String],
    loaded: &std::collections::HashMap<String, solosoul_vault::ObjectRecord>,
    page_object_id: Option<&str>,
    mut page_name: String,
    now_ms: i64,
    retention_ms: i64,
) -> Result<(Vec<solosoul_vault::TrashItem>, Vec<String>, String), String> {
    let mut trash_items: Vec<solosoul_vault::TrashItem> = Vec::new();
    let mut soft_delete_ids: Vec<String> = Vec::new();
    for id in target_ids {
        let Some(rec) = loaded.get(id) else {
            continue;
        };
        let is_page = page_object_id == Some(id.as_str());
        let record = serde_json::json!({
            "id": rec.id,
            "accountId": rec.account_id,
            "typeId": rec.type_id,
            "sectionType": rec.section_type,
            "name": rec.name,
            "iconName": rec.icon_name,
            "parentId": rec.parent_id,
            "childrenIds": rec.children_ids,
            "properties": rec.properties,
            "propertyLabels": rec.property_labels,
            "sensitivityLevel": rec.sensitivity_level,
            "tags": rec.tags_json,
            "createdAt": rec.created_at,
            "updatedAt": rec.updated_at,
            "version": rec.version,
            "templateId": rec.template_id,
            "templateType": rec.template_type,
            "contractTypeId": rec.contract_type_id,
            "templateHash": rec.template_hash,
        });
        let mut data = record;
        if is_page {
            if page_name.is_empty() {
                page_name = rec.name.clone();
            }
        } else {
            data["parentPageName"] = serde_json::Value::String(page_name.clone());
            data["parentPageIcon"] = serde_json::Value::String(rec.icon_name.clone());
        }
        trash_items.push(solosoul_vault::TrashItem {
            id: format!("trash_{}", uuid::Uuid::new_v4()),
            item_type: if is_page { "page" } else { "object" }.to_string(),
            original_id: rec.id.clone(),
            original_parent_id: if is_page { None } else { rec.parent_id.clone() },
            original_section_type: Some(rec.section_type.clone()),
            original_sort_order: None,
            data: serde_json::to_vec(&data).unwrap_or_default(),
            deleted_at: now_ms,
            expires_at: Some(now_ms + retention_ms),
            deleted_by: "user".to_string(),
            name_snapshot: rec.name.clone(),
            icon_snapshot: Some(rec.icon_name.clone()),
        });
        soft_delete_ids.push(id.clone());
    }
    Ok((trash_items, soft_delete_ids, page_name))
}
