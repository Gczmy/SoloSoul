//! 对象只读策略随加密 properties 保存；普通写入不能通过移除标记绕过锁定。
use super::{with_tx, VaultStore};
use crate::encryption::{decrypt_text_field, DataEncryptionKey};
use crate::ObjectRecord;
use rusqlite::{params, Connection, OptionalExtension};

pub const OBJECT_EDIT_LOCKED: &str = "OBJECT_EDIT_LOCKED";
pub const OBJECT_EDIT_LOCK_KEY: &str = "__editLocked";
#[cfg(test)]
mod tests;

pub fn object_edit_locked(properties: &serde_json::Value) -> bool {
    // 只有显式 false 或缺失视为可编辑；损坏的保护标记不能静默降级。
    properties
        .get(OBJECT_EDIT_LOCK_KEY)
        .is_some_and(|value| value != &serde_json::Value::Bool(false))
}

/// 表单只提交用户字段；保留未发送的内部元数据，保护标记与附件以最新落库值为准。
pub fn preserve_object_edit_metadata(current: &serde_json::Value, next: &mut serde_json::Value) {
    let (Some(old), Some(next)) = (current.as_object(), next.as_object_mut()) else {
        return;
    };
    for (key, value) in old {
        if key.starts_with("__")
            && current
                .get("__fields")
                .and_then(|fields| fields.get(key))
                .is_none()
        {
            next.entry(key.clone()).or_insert_with(|| value.clone());
        }
    }
    for key in [OBJECT_EDIT_LOCK_KEY, "__attachments"] {
        if let Some(value) = old.get(key) {
            next.insert(key.into(), value.clone());
        } else {
            next.remove(key);
        }
    }
}

impl VaultStore {
    pub(crate) fn ensure_object_editable_tx(
        conn: &Connection,
        key: &DataEncryptionKey,
        id: &str,
    ) -> Result<(), String> {
        let raw: Option<String> = conn
            .query_row(
                "SELECT properties FROM objects WHERE id=?1",
                params![id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("Read object edit protection: {e}"))?;
        if let Some(raw) = raw {
            let plain = decrypt_text_field(key, &raw)?;
            let properties: serde_json::Value = serde_json::from_str(&plain)
                .map_err(|e| format!("Read object edit protection: {e}"))?;
            if object_edit_locked(&properties) {
                return Err(OBJECT_EDIT_LOCKED.to_string());
            }
        }
        Ok(())
    }
    pub fn ensure_object_editable(&self, id: &str) -> Result<(), String> {
        let key = self.data_key()?;
        let guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_ref().ok_or("Vault is locked")?;
        Self::ensure_object_editable_tx(conn, &key, id)
    }
    /// 唯一有权绕过普通写保护的状态切换，持锁重读后只改变保护标记。
    pub fn set_object_edit_lock(&self, id: &str, locked: bool) -> Result<ObjectRecord, String> {
        let key = self.data_key()?;
        let node = Self::normalize_sync_node_id(&self.local_node_id());
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        with_tx(conn, "Object lock begin", "Object lock commit", |conn| {
            let mut record = Self::load_object_tx(conn, &key, id)?.ok_or("Object not found")?;
            if record.account_id != self.config.account_id
                || record.is_deleted
                || record.type_id == "page"
            {
                return Err("Object is unavailable for edit protection".to_string());
            }
            if object_edit_locked(&record.properties) == locked {
                return Ok(record);
            }
            if record.properties.is_null() {
                record.properties = serde_json::json!({});
            }
            record
                .properties
                .as_object_mut()
                .ok_or("Invalid object properties")?
                .insert(
                    OBJECT_EDIT_LOCK_KEY.to_string(),
                    serde_json::Value::Bool(locked),
                );
            record.updated_at = chrono::Utc::now().to_rfc3339();
            record.version = record
                .version
                .checked_add(1)
                .ok_or("Object version overflow")?;
            let hlc = Self::new_local_hlc_tx(conn, &node)?;
            Self::save_object_unchecked_tx(conn, &key, &record, None)?;
            Self::set_record_hlc_tx(conn, "objects", id, &hlc)?;
            Ok(record)
        })
    }
}
