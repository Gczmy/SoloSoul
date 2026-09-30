//! RF-016：永久删除附件的本机清理意图，不参与导出或同步。
//! 元数据和删除许可同事务提交；文件动作只执行数据库中的实际许可。

use std::collections::BTreeSet;

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde_json::Value;

use super::{with_tx, VaultStore};
use crate::encryption::{decrypt_text_field, DataEncryptionKey};

/// object_id 是元数据所有者；恢复换 ID 时 storage_object_id 可以保留旧物理对象 ID。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachmentCleanupIntent {
    pub account_id: String,
    pub object_id: String,
    pub storage_object_id: String,
    pub attachment_id: String,
    pub created_at: i64,
    pub attempts: u32,
    pub last_error_code: Option<String>,
}

const INTENT_COLUMNS: &str =
    "account_id, object_id, storage_object_id, attachment_id, created_at, attempts, last_error_code";

fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.len() > 64
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("Invalid attachment cleanup identifier".to_string());
    }
    Ok(())
}

/// 任何坏数组、重复 ID 或错误类型都不能作为「没有附件引用」的证明。
fn attachment_entries(properties: &Value) -> Result<&[Value], String> {
    let properties = properties
        .as_object()
        .ok_or("Invalid attachment properties")?;
    let Some(value) = properties.get("__attachments") else {
        return Ok(&[]);
    };
    let entries = value.as_array().ok_or("Invalid attachment list")?;
    let mut ids = BTreeSet::new();
    for entry in entries {
        let entry = entry.as_object().ok_or("Invalid attachment entry")?;
        let id = entry
            .get("id")
            .and_then(Value::as_str)
            .ok_or("Invalid attachment identifier")?;
        validate_id(id)?;
        if !ids.insert(id) {
            return Err("Duplicate attachment identifier".to_string());
        }
        if let Some(storage_id) = entry.get("objectId") {
            validate_id(
                storage_id
                    .as_str()
                    .ok_or("Invalid attachment storage identifier")?,
            )?;
        }
        // 和 AttachmentMeta 的必需字段契约一致；objectId 缺失仅为旧数据兼容。
        for field in ["fileName", "mimeType", "createdAt"] {
            if entry.get(field).and_then(Value::as_str).is_none() {
                return Err("Invalid required attachment string metadata".to_string());
            }
        }
        for field in ["vaultPath", "srcPath", "deletedAt", "description"] {
            if entry
                .get(field)
                .is_some_and(|value| !value.is_null() && !value.is_string())
            {
                return Err("Invalid attachment string metadata".to_string());
            }
        }
        if entry.get("sizeBytes").and_then(Value::as_u64).is_none() {
            return Err("Invalid attachment size metadata".to_string());
        }
        if let Some(tags) = entry.get("tags") {
            if !tags
                .as_array()
                .is_some_and(|tags| tags.iter().all(Value::is_string))
            {
                return Err("Invalid attachment tag metadata".to_string());
            }
        }
    }
    Ok(entries)
}

/// 字面路径仅识别保守引用，不成为文件动作参数。兼容两平台分隔符和大小写。
fn references_storage_path(path: &str, storage_id: &str, attachment_id: &str) -> bool {
    let parts: Vec<_> = path
        .split(['/', '\\'])
        .filter(|part| !part.is_empty())
        .collect();
    parts.windows(3).any(|parts| {
        parts[0].eq_ignore_ascii_case("attachments")
            && parts[1].eq_ignore_ascii_case(storage_id)
            && parts[2].eq_ignore_ascii_case(attachment_id)
    })
}

fn storage_object_id(entry: &Value, owner: &str) -> Result<String, String> {
    let storage_id = entry
        .get("objectId")
        .and_then(Value::as_str)
        .unwrap_or(owner);
    validate_id(storage_id)?;
    // 字面 vaultPath/srcPath 不能授予文件删除权限；兼容移动或恢复后的旧路径。
    // 文件执行器只接受该真实元数据条目的 storage/id，并独立验证目录边界。
    Ok(storage_id.to_string())
}

fn read_intent(row: &rusqlite::Row<'_>) -> rusqlite::Result<AttachmentCleanupIntent> {
    Ok(AttachmentCleanupIntent {
        account_id: row.get(0)?,
        object_id: row.get(1)?,
        storage_object_id: row.get(2)?,
        attachment_id: row.get(3)?,
        created_at: row.get(4)?,
        attempts: row.get(5)?,
        last_error_code: row.get(6)?,
    })
}

fn load_intent(
    conn: &Connection,
    account_id: &str,
    object_id: &str,
    attachment_id: &str,
) -> Result<Option<AttachmentCleanupIntent>, String> {
    conn.query_row(
        &format!(
            "SELECT {INTENT_COLUMNS} FROM attachment_cleanup_intents
                   WHERE account_id = ?1 AND object_id = ?2 AND attachment_id = ?3"
        ),
        params![account_id, object_id, attachment_id],
        read_intent,
    )
    .optional()
    .map_err(|_| "Cannot load attachment cleanup intent".to_string())
}

fn current_reference_exists(
    conn: &Connection,
    key: &DataEncryptionKey,
    intent: &AttachmentCleanupIntent,
) -> Result<bool, String> {
    // 不使用 UI 列表限制，不过滤 account/is_deleted；坏记录也必须被检查。
    let mut statement = conn
        .prepare("SELECT properties FROM objects")
        .map_err(|_| "Cannot read attachment cleanup references".to_string())?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|_| "Cannot query attachment cleanup references".to_string())?;
    let mut referenced = false;
    for row in rows {
        let encrypted = row.map_err(|_| "Cannot read attachment cleanup reference".to_string())?;
        let plain = decrypt_text_field(key, &encrypted)
            .map_err(|_| "Cannot decrypt attachment cleanup references".to_string())?;
        let properties: Value = serde_json::from_str(&plain)
            .map_err(|_| "Invalid attachment cleanup reference JSON".to_string())?;
        for entry in attachment_entries(&properties)? {
            referenced |= entry["id"]
                .as_str()
                .is_some_and(|id| id.eq_ignore_ascii_case(&intent.attachment_id));
            for field in ["vaultPath", "srcPath"] {
                referenced |= entry
                    .get(field)
                    .and_then(Value::as_str)
                    .is_some_and(|path| {
                        references_storage_path(
                            path,
                            &intent.storage_object_id,
                            &intent.attachment_id,
                        )
                    });
            }
        }
    }
    Ok(referenced)
}

fn record_failure(
    conn: &Connection,
    intent: &AttachmentCleanupIntent,
    attempted: bool,
    code: &'static str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE attachment_cleanup_intents
         SET attempts = MIN(attempts + ?4, 4294967295), last_error_code = ?5
         WHERE account_id = ?1 AND object_id = ?2 AND attachment_id = ?3",
        params![
            intent.account_id,
            intent.object_id,
            intent.attachment_id,
            i64::from(attempted),
            code
        ],
    )
    .map_err(|_| "Cannot record attachment cleanup failure".to_string())?;
    Ok(())
}

impl VaultStore {
    fn validate_cleanup_account(&self, account_id: &str) -> Result<(), String> {
        if account_id.is_empty() || account_id != self.config.account_id {
            return Err("Attachment cleanup account mismatch".to_string());
        }
        Ok(())
    }

    /// 读取最新对象，单事务提交真实附件条目移除、版本/HLC和全部意图。
    /// 未知附件 ID 不产生许可；重复已接受请求复用意图，版本不会重复增长。
    pub fn queue_attachment_deletions(
        &self,
        account_id: &str,
        object_id: &str,
        ids: &[String],
    ) -> Result<Vec<AttachmentCleanupIntent>, String> {
        self.validate_cleanup_account(account_id)?;
        validate_id(object_id)?;
        let ids: BTreeSet<_> = ids.iter().map(String::as_str).collect();
        for id in &ids {
            validate_id(id)?;
        }
        let key = self.data_key()?;
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let hlc = self.new_local_hlc()?;
        let mut guard = self
            .conn
            .lock()
            .map_err(|_| "Vault connection lock poisoned")?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        with_tx(
            conn,
            "Queue attachment cleanup begin",
            "Queue attachment cleanup commit",
            |tx| {
                let mut queued = Vec::new();
                for id in &ids {
                    if let Some(intent) = load_intent(tx, account_id, object_id, id)? {
                        validate_id(&intent.storage_object_id)?;
                        queued.push(intent);
                    }
                }
                let mut record = Self::load_object_tx(tx, &key, object_id)?;
                if record
                    .as_ref()
                    .is_some_and(|record| record.account_id != account_id)
                {
                    return Err("Attachment object account mismatch".to_string());
                }
                // 对象已消失时仅接纳全部已有的持久许可，不新增删除权限。
                // 同账户软删对象仍可处理真实附件，保留既有 GUI IPC 语义。
                if record.is_none() {
                    if queued.len() == ids.len() {
                        return Ok(queued);
                    }
                    return Err("Attachment object not found".to_string());
                }
                let record = record.as_mut().ok_or("Attachment object not found")?;
                if record.account_id != account_id {
                    return Err("Attachment object account mismatch".to_string());
                }
                let mut targets = Vec::new();
                for entry in attachment_entries(&record.properties)? {
                    let id = entry["id"]
                        .as_str()
                        .ok_or("Invalid attachment identifier")?;
                    if ids.contains(id) {
                        let storage_id = storage_object_id(entry, object_id)?;
                        if queued.iter().any(|intent| {
                            intent.attachment_id == id && intent.storage_object_id != storage_id
                        }) {
                            return Err("Attachment cleanup storage identifier changed".to_string());
                        }
                        targets.push((id.to_string(), storage_id));
                    }
                }
                if !targets.is_empty() {
                    let kept: Vec<Value> = attachment_entries(&record.properties)?
                        .iter()
                        .filter(|entry| !ids.contains(entry["id"].as_str().unwrap_or("")))
                        .cloned()
                        .collect();
                    record.properties["__attachments"] = Value::Array(kept);
                    record.version = record
                        .version
                        .checked_add(1)
                        .ok_or("Attachment object version overflow")?;
                    record.updated_at = chrono::Utc::now().to_rfc3339();
                    Self::save_object_tx(tx, &key, record, None)?;
                    Self::set_record_hlc_tx(tx, "objects", object_id, &hlc)?;
                    for (attachment_id, storage_id) in &targets {
                        tx.execute(
                            "INSERT INTO attachment_cleanup_intents
                         (account_id, object_id, storage_object_id, attachment_id, created_at)
                         VALUES (?1, ?2, ?3, ?4, ?5)
                         ON CONFLICT(account_id, object_id, attachment_id) DO NOTHING",
                            params![
                                account_id,
                                object_id,
                                storage_id,
                                attachment_id,
                                chrono::Utc::now().timestamp_millis()
                            ],
                        )
                        .map_err(|_| "Cannot queue attachment cleanup intent".to_string())?;
                    }
                }
                // 按去重后的请求顺序返回，既有许可和本轮新许可均来自本事务数据库。
                let mut accepted = Vec::new();
                for id in &ids {
                    if let Some(intent) = load_intent(tx, account_id, object_id, id)? {
                        accepted.push(intent);
                    }
                }
                Ok(accepted)
            },
        )
    }

    pub fn list_attachment_cleanup_intents(
        &self,
        account_id: &str,
    ) -> Result<Vec<AttachmentCleanupIntent>, String> {
        self.validate_cleanup_account(account_id)?;
        self.data_key()?;
        let guard = self
            .conn
            .lock()
            .map_err(|_| "Vault connection lock poisoned")?;
        let conn = guard.as_ref().ok_or("Vault is locked")?;
        let mut statement = conn
            .prepare(&format!(
                "SELECT {INTENT_COLUMNS} FROM attachment_cleanup_intents
             WHERE account_id = ?1 ORDER BY created_at, object_id, attachment_id"
            ))
            .map_err(|_| "Cannot list attachment cleanup intents".to_string())?;
        let rows = statement
            .query_map([account_id], read_intent)
            .map_err(|_| "Cannot query attachment cleanup intents".to_string())?;
        let intents = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| "Cannot read attachment cleanup intents".to_string())?;
        for intent in &intents {
            validate_id(&intent.object_id)?;
            validate_id(&intent.storage_object_id)?;
            validate_id(&intent.attachment_id)?;
        }
        Ok(intents)
    }

    /// 调用方持原会话门闩；action 接收数据库重载的权威许可，只进行有界文件操作，不能重入 Vault API。
    /// true=完成/许可已不存在；false=当前引用阻止；Err=失败许可仍等待重试。
    /// action 把 NotFound 归一成功；错误原文不保存、不返回。显式永久删除不保留历史回链。
    pub fn run_attachment_cleanup_intent(
        &self,
        intent: &AttachmentCleanupIntent,
        action: impl FnOnce(&AttachmentCleanupIntent) -> Result<(), String>,
    ) -> Result<bool, String> {
        self.validate_cleanup_account(&intent.account_id)?;
        validate_id(&intent.object_id)?;
        validate_id(&intent.storage_object_id)?;
        validate_id(&intent.attachment_id)?;
        let key = self.data_key()?;
        let mut guard = self
            .conn
            .lock()
            .map_err(|_| "Vault connection lock poisoned")?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        // Immediate 防止同一数据库的其他连接在引用重核后、文件动作前提交新引用。
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| "Cannot begin attachment cleanup".to_string())?;
        let Some(stored) = load_intent(
            &tx,
            &intent.account_id,
            &intent.object_id,
            &intent.attachment_id,
        )?
        else {
            return Ok(true);
        };
        validate_id(&stored.storage_object_id)?;
        if stored.storage_object_id != intent.storage_object_id
            || stored.created_at != intent.created_at
        {
            return Err("Attachment cleanup permission changed".to_string());
        }
        let outcome = match current_reference_exists(&tx, &key, &stored) {
            Err(_) => {
                record_failure(&tx, &stored, false, "invalid_references")?;
                Err("Cannot verify attachment cleanup references".to_string())
            }
            Ok(true) => {
                record_failure(&tx, &stored, false, "referenced")?;
                Ok(false)
            }
            Ok(false) => match action(&stored) {
                Err(_) => {
                    record_failure(&tx, &stored, true, "file_action_failed")?;
                    Err("Attachment cleanup file operation failed".to_string())
                }
                Ok(()) => {
                    tx.execute(
                        "DELETE FROM attachment_cleanup_intents
                         WHERE account_id = ?1 AND object_id = ?2 AND attachment_id = ?3",
                        params![stored.account_id, stored.object_id, stored.attachment_id],
                    )
                    .map_err(|_| "Cannot confirm attachment cleanup".to_string())?;
                    Ok(true)
                }
            },
        };
        // action 失败也提交固定错误码；提交失败时 RAII 保留原许可。
        tx.commit()
            .map_err(|_| "Cannot commit attachment cleanup".to_string())?;
        outcome
    }
}

#[cfg(test)]
mod tests;
