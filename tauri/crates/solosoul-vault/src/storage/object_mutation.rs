//! 普通对象读改写的提交边界；回调只能修改当前记录，不能重入 Vault API。

use super::{object_edit_locked, VaultStore, OBJECT_EDIT_LOCKED, OBJECT_EDIT_LOCK_KEY};
use crate::ObjectRecord;

#[derive(Debug)]
pub enum ObjectMutationError<E> {
    Read(String),
    Write(String),
    Edit(E),
    VersionOverflow,
}

impl From<ObjectMutationError<String>> for String {
    fn from(error: ObjectMutationError<String>) -> Self {
        match error {
            ObjectMutationError::Read(message)
            | ObjectMutationError::Write(message)
            | ObjectMutationError::Edit(message) => message,
            ObjectMutationError::VersionOverflow => "Object version overflow".into(),
        }
    }
}

impl VaultStore {
    /// 在同一写事务中读取最新记录、执行编辑并提交对象及 HLC。
    /// `false` 表示没有修改；对象不存在或明确无修改时返回 `None`。
    /// 回调不能进行 IO 或调用 Vault 方法；失败不会提交记录，也不消耗版本。
    pub fn mutate_object<E>(
        &self,
        id: &str,
        edit: impl FnOnce(&mut ObjectRecord) -> Result<bool, E>,
    ) -> Result<Option<ObjectRecord>, ObjectMutationError<E>> {
        use ObjectMutationError as Error;
        let key = self.data_key().map_err(Error::Read)?;
        // local_node_id 自行持有连接锁，必须在事务前捕获。
        let node = Self::normalize_sync_node_id(&self.local_node_id());
        let mut guard = self.conn.lock().map_err(|e| Error::Read(e.to_string()))?;
        let conn = guard
            .as_mut()
            .ok_or_else(|| Error::Read("Vault is locked".into()))?;
        // Immediate 也序列化同一数据库的不同连接，禁止旧读快照升级后覆盖。
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| Error::Write(format!("Object mutation begin: {e}")))?;
        let Some(mut record) = Self::load_object_tx(&tx, &key, id).map_err(Error::Read)? else {
            return Ok(None);
        };
        let account_id = record.account_id.clone();
        let created_at = record.created_at.clone();
        let version = record.version;
        let edit_lock = record.properties.get(OBJECT_EDIT_LOCK_KEY).cloned();
        let locked = object_edit_locked(&record.properties);
        if !edit(&mut record).map_err(Error::Edit)? {
            return Ok(None);
        }
        if locked {
            return Err(Error::Write(OBJECT_EDIT_LOCKED.into()));
        }
        if record.id != id || record.account_id != account_id || record.created_at != created_at {
            return Err(Error::Write("Object identity cannot change".into()));
        }
        // 普通编辑不能改变保护标记，锁状态只由显式入口控制。
        if let Some(properties) = record.properties.as_object_mut() {
            if let Some(value) = edit_lock {
                properties.insert(OBJECT_EDIT_LOCK_KEY.into(), value);
            } else {
                properties.remove(OBJECT_EDIT_LOCK_KEY);
            }
        }
        record.version = version.checked_add(1).ok_or(Error::VersionOverflow)?;
        record.updated_at = chrono::Utc::now().to_rfc3339();
        let hlc = Self::new_local_hlc_tx(&tx, &node).map_err(Error::Write)?;
        Self::save_object_tx(&tx, &key, &record, None).map_err(Error::Write)?;
        Self::set_record_hlc_tx(&tx, "objects", id, &hlc).map_err(Error::Write)?;
        tx.commit()
            .map_err(|e| Error::Write(format!("Object mutation commit: {e}")))?;
        Ok(Some(record))
    }
}

#[cfg(test)]
mod tests;
