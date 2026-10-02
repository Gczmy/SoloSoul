//! RF-021 / RF-024：Core 的数据库计划。包解析/引用/名称/历史准备均不写数据库。
//! 依赖 Vault 的冻结 import view，唯一数据库批次由 import_execute_steps 提交。

use serde_json::Value;
use solosoul_vault::{
    ImportBatchError, ImportDatabaseBatch, ImportHistoryChange, ImportObjectWrite, ImportReadView,
    ImportSnapshot, ObjectRecord, UserTemplate, VaultStore,
};
use std::collections::{BTreeSet, HashMap, HashSet};

use super::{build_import_record, merge_labels_into};
use super::{generate_id, AdvancedImportStrategy, ImportStage};

pub(super) struct TemplatePlan {
    pub id_map: HashMap<String, String>,
    pub added: Vec<UserTemplate>,
    pub available: Vec<UserTemplate>,
}

/// initial 保持旧 list_user_templates 的 created_at ASC 顺序。
/// 只有包确实重建模板时才要求严格全列表；继承可从冻结视图按旧兼容边界读取。
pub(super) fn prepare_templates(
    initial: Vec<UserTemplate>,
    account_id: &str,
    payload: &Value,
    now: &str,
) -> Result<TemplatePlan, String> {
    let mut plan = TemplatePlan {
        id_map: HashMap::new(),
        added: vec![],
        available: initial,
    };
    if let Some(templates) = payload["templates"].as_array() {
        for value in templates {
            let mut template: UserTemplate = serde_json::from_value(value.clone())
                .map_err(|_| "Invalid imported template".to_string())?;
            let original_id = template.id.clone();
            let hash = crate::export_import::user_template_content_hash(&template);
            let matching = plan
                .available
                .iter()
                .find(|local| crate::export_import::user_template_content_hash(local) == hash);
            let local_id = if let Some(matching) = matching {
                matching.id.clone()
            } else {
                let local_id = if plan.available.iter().any(|local| local.id == original_id) {
                    crate::export_import::imported_template_id(&original_id, &hash)
                } else {
                    original_id.clone()
                };
                if !plan.available.iter().any(|local| local.id == local_id) {
                    template.id = local_id.clone();
                    template.account_id = account_id.to_string();
                    template.created_at = now.to_string();
                    template.updated_at = Some(now.to_string());
                    plan.added.push(template.clone());
                    plan.available.push(template);
                    // 每次旧写入后，下个 find-by-hash 都再次按 created_at 排序读取。
                    plan.available
                        .sort_by(|a, b| a.created_at.cmp(&b.created_at));
                }
                local_id
            };
            plan.id_map.insert(original_id, local_id);
        }
    }
    Ok(plan)
}

pub(super) struct DatabasePlan {
    pub batch: ImportDatabaseBatch,
    pub imported_object_ids: HashSet<String>,
}

/// 非 KeepBoth 使用完整 strict load；名称使用 strict active list，二者不能互换。
/// 前序计划已覆盖的原 ID 从 frozen list 排除，再加回计划最终 active record。
pub(super) fn unique_shadow_name(
    vault: &VaultStore,
    view: &ImportReadView,
    shadow: &HashMap<String, ObjectRecord>,
    base_name: &str,
    locale: &str,
) -> Result<String, String> {
    let shadowed_ids: BTreeSet<String> = shadow.keys().cloned().collect();
    let mut names: HashSet<String> = vault
        .list_import_view_active_objects(view, &shadowed_ids)?
        .into_iter()
        .map(|record| record.name)
        .collect();
    names.extend(
        shadow
            .values()
            .filter(|record| !record.is_deleted)
            .map(|record| record.name.clone()),
    );
    let suffix = if locale.starts_with("zh") || locale.starts_with("cmn") {
        "（导入）"
    } else {
        " (Imported)"
    };
    let candidate = format!("{base_name}{suffix}");
    if !names.contains(&candidate) {
        return Ok(candidate);
    }
    let mut counter = 2u32;
    loop {
        let candidate = format!("{base_name}{suffix} {counter}");
        if !names.contains(&candidate) {
            return Ok(candidate);
        }
        counter += 1;
    }
}

/// 注入规则复用 Core 的唯一模板投影（Root 需显式公开该纯函数，见 README）。
/// 包 property_labels 优先，模板只兜底；不新增 hash/contract 等创建专有规则。
fn inherit_template(
    vault: &VaultStore,
    view: &ImportReadView,
    value: &Value,
    template_id_map: &HashMap<String, String>,
    templates: &HashMap<String, UserTemplate>,
    properties: &mut Value,
) -> (Option<String>, Option<Value>) {
    let id = value["template_id"].as_str().map(|id| {
        template_id_map
            .get(id)
            .cloned()
            .unwrap_or_else(|| id.to_string())
    });
    let mut labels = if value["property_labels"].is_null() {
        None
    } else {
        Some(value["property_labels"].clone())
    };
    if let Some(id) = id.as_ref() {
        // 沿用旧继承 getter：真实缺失/模板字段读错均不新增投影。
        // 所需 frozen accessor 见 README；不使用 live SQL 或不受 revision 约束的 load。
        let template = templates.get(id).cloned().or_else(|| {
            vault
                .load_import_view_user_template(view, id)
                .ok()
                .flatten()
        });
        if let Some(template) = template {
            let (defaults, fields) = crate::objects::project_template_properties(&template);
            match (defaults, &mut labels) {
                (Some(defaults), Some(existing)) => merge_labels_into(&defaults, existing),
                (Some(defaults), None) => labels = Some(defaults),
                _ => {}
            }
            crate::objects::inject_property_fields(properties, &fields);
            if let Some(values) = properties.as_object_mut() {
                values.insert("__templateName".into(), Value::String(template.name));
            }
        }
        // 坏模板的存储 name fallback 留在 Vault save_object_tx，不能改变保存前快照。
    }
    (id, labels)
}

/// 新 UUID 不复用包快照 ID；倒序写入最新在前的包历史，保留同毫秒新旧顺序。
fn package_history(snaps: &[Value]) -> Vec<ImportSnapshot> {
    let mut decoded = Vec::new();
    for snap in snaps.iter().rev() {
        let Some(encoded) = snap["data"].as_str() else {
            continue;
        };
        let data = match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded)
        {
            Ok(data) if !data.is_empty() => data,
            Ok(_) => continue,
            Err(error) => {
                tracing::warn!("[import] 快照 base64 解码失败，跳过: err={error}");
                continue;
            }
        };
        decoded.push(ImportSnapshot {
            id: generate_id(),
            timestamp_ms: snap["timestamp"]
                .as_i64()
                .filter(|value| *value > 0)
                .unwrap_or_else(|| chrono::Utc::now().timestamp_millis()),
            triggered_by: snap["triggered_by"]
                .as_str()
                .unwrap_or("import")
                .to_string(),
            data,
            diff_summary: snap["diff_summary"]
                .as_str()
                .unwrap_or("diff_imported")
                .to_string(),
        });
    }
    decoded
}

fn prepare_history(
    record: &ObjectRecord,
    source_id: &str,
    strategy: AdvancedImportStrategy,
    package_snapshots: &HashMap<String, Vec<Value>>,
) -> Result<ImportHistoryChange, String> {
    let restored = package_snapshots
        .get(source_id)
        .map(|snaps| package_history(snaps))
        .unwrap_or_default();
    if !restored.is_empty() {
        return Ok(if strategy == AdvancedImportStrategy::Overwrite {
            ImportHistoryChange::Replace(restored)
        } else {
            ImportHistoryChange::Append(restored)
        });
    }
    // 缺失/全坏/空包历史只追加，不删除本地既有历史。
    Ok(ImportHistoryChange::Append(vec![ImportSnapshot {
        id: generate_id(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        triggered_by: "import".into(),
        data: serde_json::to_vec(record).map_err(|error| format!("snapshot ser: {error}"))?,
        diff_summary: "diff_imported".into(),
    }]))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn prepare_objects(
    vault: &VaultStore,
    view: &ImportReadView,
    objects: &[Value],
    account_id: &str,
    strategy: AdvancedImportStrategy,
    overrides: &HashMap<String, AdvancedImportStrategy>,
    selected_ids: Option<&BTreeSet<String>>,
    package_ids: &HashSet<String>,
    template_id_map: &HashMap<String, String>,
    templates: &HashMap<String, UserTemplate>,
    id_map: &HashMap<String, String>,
    package_snapshots: &HashMap<String, Vec<Value>>,
    added_templates: Vec<UserTemplate>,
    now: &str,
    locale: &str,
    progress: Option<&(dyn Fn(u8) + Send + Sync)>,
    stage: &mut ImportStage,
) -> Result<DatabasePlan, String> {
    let mut plan = DatabasePlan {
        batch: ImportDatabaseBatch {
            templates: added_templates,
            objects: vec![],
        },
        imported_object_ids: HashSet::new(),
    };
    let mut shadow: HashMap<String, ObjectRecord> = HashMap::new();
    for (index, value) in objects.iter().enumerate() {
        *stage = ImportStage::Objects;
        let id = value["id"].as_str().unwrap_or("");
        if id.is_empty() || selected_ids.is_some_and(|selected| !selected.contains(id)) {
            continue;
        }
        let effective = overrides.get(id).copied().unwrap_or(strategy);
        if effective != AdvancedImportStrategy::KeepBoth {
            let existing = if let Some(record) = shadow.get(id) {
                Some(record.clone())
            } else {
                vault.load_import_view_object(view, id)?
            };
            if effective == AdvancedImportStrategy::SkipExisting
                && existing.is_some_and(|record| !record.is_deleted)
            {
                continue;
            }
        }
        let mut properties = value["properties"].clone();
        super::package::resolve_cross_scope_references(&mut properties, package_ids);
        let (template_id, labels) = inherit_template(
            vault,
            view,
            value,
            template_id_map,
            templates,
            &mut properties,
        );
        if !id_map.is_empty() {
            super::package::rewrite_id_references(&mut properties, id_map);
        }
        let (final_id, name) = if effective == AdvancedImportStrategy::KeepBoth {
            (
                id_map.get(id).cloned().unwrap_or_else(generate_id),
                unique_shadow_name(
                    vault,
                    view,
                    &shadow,
                    value["name"].as_str().unwrap_or("Imported"),
                    locale,
                )?,
            )
        } else {
            (
                id.to_string(),
                value["name"].as_str().unwrap_or("Imported").to_string(),
            )
        };
        let record = build_import_record(
            value,
            account_id,
            id_map,
            template_id,
            &final_id,
            &name,
            properties,
            labels,
            now,
        );
        *stage = ImportStage::Snapshots;
        let history = prepare_history(&record, id, effective, package_snapshots)?;
        shadow.insert(final_id.clone(), record.clone());
        plan.batch
            .objects
            .push(ImportObjectWrite { record, history });
        plan.imported_object_ids.insert(final_id);
        if effective == AdvancedImportStrategy::KeepBoth {
            plan.imported_object_ids.insert(id.to_string());
        }
        if let Some(callback) = progress {
            callback(((index + 1) * 80 / objects.len().max(1)).min(80) as u8);
        }
    }
    Ok(plan)
}

pub(super) fn commit_failure_stage(error: ImportBatchError) -> ImportStage {
    match error {
        ImportBatchError::Templates => ImportStage::Templates,
        ImportBatchError::Snapshots => ImportStage::Snapshots,
        // revision/identity/begin/HLC/commit 都是本次数据库批次失败，尚无附件/偏好提交。
        _ => ImportStage::Objects,
    }
}
