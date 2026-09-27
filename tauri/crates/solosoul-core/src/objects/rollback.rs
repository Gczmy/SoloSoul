//! GUI / CLI 共用回滚用例。对象保存是提交点，历史与审计分别报告后续失败。

use solosoul_vault::{ObjectRecord, VaultStore};

/// Err 始终表示对象尚未保存；保存后的附属写入失败通过 RollbackOutcome 返回。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollbackErrorStage {
    SnapshotOwner,
    SnapshotNotFound,
    Ownership,
    SnapshotRead,
    SnapshotParse,
    ObjectRead,
    ObjectNotFound,
    Labels,
    Version,
    Serialize,
    ObjectSave,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct RollbackError {
    pub stage: RollbackErrorStage,
    pub message: String,
}

impl RollbackError {
    fn new(stage: RollbackErrorStage, message: impl Into<String>) -> Self {
        Self {
            stage,
            message: message.into(),
        }
    }
}

#[derive(Debug)]
pub struct RollbackOutcome {
    pub record: ObjectRecord,
    pub snapshot_error: Option<String>,
    pub audit_error: Option<String>,
}

/// 同对象归属、解析、字段恢复、版本及序列化全部在首次写入前完成。
/// 保持原对象保存/历史/审计三个独立写入边界，不把部分成功伪装成完整失败。
pub fn rollback_object(
    vault: &VaultStore,
    object_id: &str,
    snapshot_id: &str,
) -> Result<RollbackOutcome, RollbackError> {
    use RollbackErrorStage as Stage;

    let owner = vault
        .get_snapshot_owner(snapshot_id)
        .map_err(|e| RollbackError::new(Stage::SnapshotOwner, e))?
        .ok_or_else(|| RollbackError::new(Stage::SnapshotNotFound, "Snapshot not found"))?;
    if owner.is_empty() || owner != object_id {
        return Err(RollbackError::new(
            Stage::Ownership,
            "Snapshot does not belong to object",
        ));
    }
    let data = vault
        .get_snapshot(snapshot_id)
        .map_err(|e| RollbackError::new(Stage::SnapshotRead, e))?
        .ok_or_else(|| RollbackError::new(Stage::SnapshotNotFound, "Snapshot not found"))?;
    let snapshot: serde_json::Value = serde_json::from_slice(&data)
        .map_err(|e| RollbackError::new(Stage::SnapshotParse, format!("Parse: {e}")))?;
    let mut record = vault
        .load_object(object_id)
        .map_err(|e| RollbackError::new(Stage::ObjectRead, e))?
        .ok_or_else(|| RollbackError::new(Stage::ObjectNotFound, "Object not found"))?;

    if let Some(name) = snapshot["name"].as_str() {
        record.name = name.to_string();
    }
    if let Some(tags) = snapshot["tags"].as_array() {
        record.tags_json = tags
            .iter()
            .filter_map(|value| value.as_str().map(str::to_string))
            .collect();
    }
    if !snapshot["properties"].is_null() {
        record.properties = snapshot["properties"].clone();
    }
    // RF-007：camelCase 键存在即优先（包括 null），无键保留，显式 null 清除。
    if let Some(labels) = snapshot
        .get("propertyLabels")
        .or_else(|| snapshot.get("property_labels"))
    {
        record.property_labels = match labels {
            serde_json::Value::Null => None,
            serde_json::Value::Object(_) => Some(labels.clone()),
            _ => {
                return Err(RollbackError::new(
                    Stage::Labels,
                    "Snapshot field labels must be an object or null",
                ))
            }
        };
    }
    record.version = record
        .version
        .checked_add(1)
        .ok_or_else(|| RollbackError::new(Stage::Version, "Object version overflow"))?;
    record.updated_at = chrono::Utc::now().to_rfc3339();
    let rollback_data = serde_json::to_vec(&serde_json::json!({
        "name": record.name,
        "tags": record.tags_json,
        "properties": record.properties,
        "propertyLabels": record.property_labels,
    }))
    .map_err(|e| {
        RollbackError::new(
            Stage::Serialize,
            format!("Serialize rollback snapshot: {e}"),
        )
    })?;

    vault
        .save_object(&record)
        .map_err(|e| RollbackError::new(Stage::ObjectSave, e))?;

    // 两项独立尝试：历史写失败仍须记录此次已经保存的对象回滚。
    let snapshot_error = vault
        .save_snapshot(object_id, "rollback", &rollback_data, "diff_rollback")
        .err();
    let audit_error = vault
        .log_structured(
            "object_rollback",
            "object",
            Some(object_id),
            Some(&record.name),
            "user",
            Some(&format!(
                "section={} snapshot={}",
                record.section_type, snapshot_id
            )),
        )
        .err();
    Ok(RollbackOutcome {
        record,
        snapshot_error,
        audit_error,
    })
}

#[cfg(test)]
mod tests;
