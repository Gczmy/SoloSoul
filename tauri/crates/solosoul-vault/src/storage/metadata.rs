//! 审计 / 元数据 / 用户模板域 —— 自 `storage.rs` 拆分（P223-② 表域拆分第七域）。
//!
//! 本模块承载 `VaultStore` 四簇方法（原 storage.rs 1121-1730 行，逐行搬运零行为变更）：
//! ① 审计日志（`log_structured` + 私有 `normalize_details_text` 助手 + `list_audit_log`）；
//! ② 私有元数据存取（`read_metadata`/`write_metadata`，供根模块 sync 节点状态方法复用）；
//! ③ Guide embeddings for RAG（`save|list|clear|count_guide_embeddings`）与 sys_config
//! （`get|set_sys_config`，snapshots 域跨域复用）；④ 用户模板（`save_user_template(_tx)` /
//! `load_user_template(_tx)` / `list_user_templates` / `find_user_template_by_content_hash` /
//! `delete_user_template` / `count_user_templates` / `check_field_usage`）。
//!
//! 共享设施：`data_key()` 经根模块隐私向下可见；`USER_TEMPLATE_SAVE_SQL` /
//! `USER_TEMPLATE_LOAD_SQL` 为根模块 SQL 常量（`super::`）；`decrypt_text_field` /
//! `encrypt_text_field` 从 `crate::encryption` 导入；`record_tombstone` 属 sync_meta 域
//! pub(crate)。4 个跨域私有助手（`read_metadata`/`write_metadata`/`save_user_template_tx`/
//! `load_user_template_tx`）提 `pub(crate)`，其余 14 个 pub API 可见性不变（src-tauri /
//! CLI / solosoul-sync 跨 crate 调用 + 根测试模块）。

use rusqlite::{params, Connection};

use super::{VaultStore, USER_TEMPLATE_LOAD_SQL, USER_TEMPLATE_SAVE_SQL};
use crate::encryption::{decrypt_text_field, encrypt_text_field, DataEncryptionKey};

impl VaultStore {
    /// 将 details 字符串规范化为 JSON 对象（如果它是 `key=value ...` 格式）或保留原样。
    /// 这样前端可以直接解析结构化字段，而无需再用正则拆分 key=value。
    fn normalize_details_text(details: Option<&str>) -> Option<String> {
        let text = details?;
        // 若已经是合法 JSON（可能是调用方直接传入的对象/数组），按原样保留。
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
            return Some(value.to_string());
        }
        // 尝试解析 `key=value` 序列。
        let mut obj = serde_json::Map::new();
        let bytes = text.as_bytes();
        let len = bytes.len();
        let mut pos = 0usize;
        // 跳过前导空白。
        while pos < len && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let start = pos;
        while pos < len {
            // key
            let key_start = pos;
            while pos < len {
                let c = bytes[pos];
                if c == b'=' || c.is_ascii_whitespace() {
                    break;
                }
                pos += 1;
            }
            if pos >= len || bytes[pos] != b'=' {
                break;
            }
            let key = std::str::from_utf8(&bytes[key_start..pos]).ok()?;
            if key.is_empty() {
                break;
            }
            pos += 1; // skip '='
            let value_start = pos;
            // 查找下一个 ` key=` 位置作为 value 结束。
            let mut value_end = len;
            let mut scan = pos;
            while scan < len {
                if bytes[scan].is_ascii_whitespace() {
                    let mut after = scan + 1;
                    while after < len && bytes[after].is_ascii_whitespace() {
                        after += 1;
                    }
                    let key2_start = after;
                    while after < len && !bytes[after].is_ascii_whitespace() && bytes[after] != b'='
                    {
                        after += 1;
                    }
                    if after < len && bytes[after] == b'=' {
                        let key2 = std::str::from_utf8(&bytes[key2_start..after]).ok()?;
                        if !key2.is_empty() {
                            value_end = scan;
                            break;
                        }
                    }
                }
                scan += 1;
            }
            let value = std::str::from_utf8(&bytes[value_start..value_end])
                .ok()?
                .trim_end();
            obj.insert(
                key.to_string(),
                serde_json::Value::String(value.to_string()),
            );
            pos = value_end;
            while pos < len && bytes[pos].is_ascii_whitespace() {
                pos += 1;
            }
        }
        if start == 0 && !obj.is_empty() && pos == len {
            Some(serde_json::Value::Object(obj).to_string())
        } else {
            Some(text.to_string())
        }
    }

    /// Write a structured audit log entry with full fields.
    pub fn log_structured(
        &self,
        action_type: &str,
        entity_type: &str,
        entity_id: Option<&str>,
        entity_name: Option<&str>,
        performed_by: &str,
        details: Option<&str>,
    ) -> Result<(), String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let now = chrono::Utc::now().to_rfc3339();
        let encrypted_name = entity_name
            .map(|n| encrypt_text_field(&key, n))
            .transpose()?;
        let normalized = Self::normalize_details_text(details);
        let encrypted_details = normalized
            .as_deref()
            .map(|d| encrypt_text_field(&key, d))
            .transpose()?;
        conn.execute(
        "INSERT INTO audit_log (timestamp, action, entity_type, entity_id, entity_name, performed_by, details)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            now,
            action_type,
            entity_type,
            entity_id,
            encrypted_name,
            performed_by,
            encrypted_details,
        ],
    )
    .map_err(|e| format!("log_structured: {}", e))?;
        Ok(())
    }

    /// List recent audit log entries, newest first.
    pub fn list_audit_log(&self, limit: usize) -> Result<Vec<crate::AuditLogEntry>, String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        // 先收集原始行数据（不在此处解密），避免单行解密失败导致整个查询失败。
        // 这对于密码修改后部分审计日志可能用旧密钥加密的场景尤为重要。
        let mut stmt = conn.prepare(
        "SELECT id, timestamp, action, entity_type, entity_id, entity_name, performed_by, details
         FROM audit_log ORDER BY id DESC LIMIT ?1"
    ).map_err(|e| format!("list_audit_log prepare: {}", e))?;
        // 原始审计日志行：id, timestamp, action, entity_type, entity_id, entity_name, performed_by, details
        type RawAuditRow = (
            i64,
            String,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        );
        let raw_rows: Vec<RawAuditRow> = stmt
            .query_map(rusqlite::params![limit as i64], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            })
            .map_err(|e| format!("list_audit_log query: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("list_audit_log collect: {}", e))?;
        drop(stmt);
        drop(guard);

        // 逐行解密，解密失败的条目跳过而非中断整个列表。
        let mut result = Vec::new();
        let mut skip_count = 0u32;
        for (
            id,
            timestamp,
            action_type,
            entity_type,
            entity_id,
            raw_name,
            performed_by,
            raw_details,
        ) in raw_rows
        {
            let entity_name = match raw_name.as_deref() {
                Some(n) => match decrypt_text_field(&key, n) {
                    Ok(dec) => Some(dec),
                    Err(e) => {
                        tracing::warn!(
                            "list_audit_log: entity_name decryption failed for entry id={}: {}",
                            id,
                            e
                        );
                        skip_count += 1;
                        Some(format!("[decryption error: {}]", e))
                    }
                },
                None => None,
            };
            let details = match raw_details.as_deref() {
                Some(d) => match decrypt_text_field(&key, d) {
                    Ok(dec) => Some(dec),
                    Err(e) => {
                        tracing::warn!(
                        "list_audit_log: skipping details for entry id={}, decryption failed: {}",
                        id, e
                    );
                        skip_count += 1;
                        Some(format!("[decryption error: {}]", e))
                    }
                },
                None => None,
            };
            result.push(crate::AuditLogEntry {
                id,
                timestamp,
                action_type,
                entity_type: entity_type.unwrap_or_default(),
                entity_id,
                entity_name,
                performed_by: performed_by.unwrap_or_else(|| "system".to_string()),
                details,
            });
        }
        if skip_count > 0 {
            tracing::warn!(
            "list_audit_log: skipped {} field(s) due to decryption failures (possible stale encryption after password change)",
            skip_count
        );
        }
        Ok(result)
    }

    // Metadata helpers for encrypted blob storage (reserved)
    pub(crate) fn read_metadata(&self, key: &str, prefix: &str) -> Result<Option<Vec<u8>>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let full_key = format!("{}_{}", prefix, key);
        let result: Option<String> = conn
            .query_row(
                "SELECT value FROM metadata WHERE key = ?1",
                params![full_key],
                |r| r.get(0),
            )
            .ok();
        match result {
            Some(encoded) => {
                use base64::Engine as _;
                let engine = base64::engine::general_purpose::STANDARD;
                let decoded = engine
                    .decode(&encoded)
                    .map_err(|e| format!("Base64 decode error: {}", e))?;
                Ok(Some(decoded))
            }
            None => Ok(None),
        }
    }

    pub(crate) fn write_metadata(
        &self,
        key: &str,
        prefix: &str,
        data: &[u8],
    ) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let now = chrono::Utc::now().to_rfc3339();
        let full_key = format!("{}_{}", prefix, key);
        use base64::Engine as _;
        let engine = base64::engine::general_purpose::STANDARD;
        let encoded = engine.encode(data);
        conn.execute(
            "INSERT INTO metadata (key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![full_key, encoded, now],
        )
        .map_err(|e| format!("Failed to write metadata: {}", e))?;
        Ok(())
    }

    // ── Guide embeddings for RAG (§RAG-1) ────────────────────────

    /// Batch-save guide embedding chunks in a single transaction (P051).
    /// 批量版本只开一次事务 + 复用 prepared statement，重建耗时大幅下降。
    pub fn save_guide_embeddings(
        &self,
        chunks: &[crate::GuideEmbeddingChunk],
    ) -> Result<(), String> {
        if chunks.is_empty() {
            return Ok(());
        }
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let tx = conn
            .transaction()
            .map_err(|e| format!("save_guide_embeddings begin: {}", e))?;
        {
            let mut stmt = tx
            .prepare(
                "INSERT OR REPLACE INTO guide_embeddings (id, guide_id, chunk_index, chunk_text, embedding, model, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )
            .map_err(|e| format!("save_guide_embeddings prepare: {}", e))?;
            for (idx, chunk) in chunks.iter().enumerate() {
                let embedding_bytes: Vec<u8> = chunk
                    .embedding
                    .iter()
                    .flat_map(|f| f.to_ne_bytes())
                    .collect();
                stmt.execute(params![
                    chunk.id,
                    chunk.guide_id,
                    chunk.chunk_index,
                    chunk.chunk_text,
                    embedding_bytes,
                    chunk.model,
                    chunk.created_at
                ])
                .map_err(|e| format!("save_guide_embeddings chunk {}: {}", idx, e))?;
            }
        }
        tx.commit()
            .map_err(|e| format!("save_guide_embeddings commit: {}", e))?;
        Ok(())
    }

    /// Load all guide embedding chunks.
    pub fn list_guide_embeddings(&self) -> Result<Vec<crate::GuideEmbeddingChunk>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn
            .prepare(
                "SELECT id, guide_id, chunk_index, chunk_text, embedding, model, created_at
         FROM guide_embeddings ORDER BY guide_id, chunk_index",
            )
            .map_err(|e| format!("list_guide_embeddings prepare: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                let embedding_bytes: Vec<u8> = row.get(4)?;
                let embedding: Vec<f32> = embedding_bytes
                    .chunks_exact(4)
                    .map(|b| f32::from_ne_bytes([b[0], b[1], b[2], b[3]]))
                    .collect();
                Ok(crate::GuideEmbeddingChunk {
                    id: row.get(0)?,
                    guide_id: row.get(1)?,
                    chunk_index: row.get(2)?,
                    chunk_text: row.get(3)?,
                    embedding,
                    model: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })
            .map_err(|e| format!("list_guide_embeddings query: {}", e))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| format!("list_guide_embeddings row: {}", e))?);
        }
        Ok(result)
    }

    /// Clear all guide embeddings (used for rebuild).
    pub fn clear_guide_embeddings(&self) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        conn.execute("DELETE FROM guide_embeddings", [])
            .map_err(|e| format!("clear_guide_embeddings: {}", e))?;
        Ok(())
    }

    /// Get the count of guide embeddings.
    pub fn count_guide_embeddings(&self) -> Result<usize, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM guide_embeddings", [], |r| r.get(0))
            .map_err(|e| format!("count_guide_embeddings: {}", e))?;
        Ok(count as usize)
    }

    // ── sys_config helpers ────────────────────────────────────────

    /// Read a value from sys_config by key.
    pub fn get_sys_config(&self, key: &str) -> Result<Option<String>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let result: Option<String> = conn
            .query_row(
                "SELECT value FROM sys_config WHERE key = ?1",
                params![key],
                |r| r.get(0),
            )
            .ok();
        Ok(result)
    }

    /// Write or update a value in sys_config.
    pub fn set_sys_config(&self, key: &str, value: &str) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR REPLACE INTO sys_config (key, value, updated_at) VALUES (?1, ?2, ?3)",
            params![key, value, now],
        )
        .map_err(|e| format!("set_sys_config: {}", e))?;
        Ok(())
    }

    // ── User template helpers (§29 模板系统重构 P1) ──────────────

    /// Save or update a user template (UPSERT).
    pub fn save_user_template(&self, template: &crate::UserTemplate) -> Result<(), String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        Self::save_user_template_tx(conn, &key, template)
    }

    /// P115: 事务内保存用户模板（连接由调用方持有，批量应用单事务内复用）。
    pub(crate) fn save_user_template_tx(
        conn: &mut Connection,
        key: &DataEncryptionKey,
        template: &crate::UserTemplate,
    ) -> Result<(), String> {
        let props_json = serde_json::to_string(&template.properties)
            .map_err(|e| format!("serialize properties: {}", e))?;
        let encrypted_props = encrypt_text_field(key, &props_json)?;
        let mut stmt = conn
            .prepare_cached(USER_TEMPLATE_SAVE_SQL)
            .map_err(|e| format!("save_user_template prepare: {}", e))?;
        stmt.execute(params![
            &template.id,
            &template.account_id,
            &template.name,
            &template.icon_id,
            encrypted_props,
            &template.category,
            &template.contract_type_id,
            &template.created_at,
            &template.updated_at,
        ])
        .map_err(|e| format!("save_user_template: {}", e))?;
        Ok(())
    }

    /// Load a single user template by ID.
    pub fn load_user_template(
        &self,
        template_id: &str,
    ) -> Result<Option<crate::UserTemplate>, String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        Self::load_user_template_tx(conn, &key, template_id)
    }

    /// P115: 事务内加载用户模板（连接由调用方持有）。
    pub(crate) fn load_user_template_tx(
        conn: &mut Connection,
        key: &DataEncryptionKey,
        template_id: &str,
    ) -> Result<Option<crate::UserTemplate>, String> {
        let mut stmt = conn
            .prepare_cached(USER_TEMPLATE_LOAD_SQL)
            .map_err(|e| format!("prepare load_user_template: {}", e))?;

        let result = stmt.query_row(params![template_id], |row| {
            let props_json: String = row.get(4)?;
            let decrypted = decrypt_text_field(key, &props_json).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    4,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Template properties decryption failed: {}", e),
                    )),
                )
            })?;
            let properties: Vec<crate::TemplateProperty> = serde_json::from_str(&decrypted)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            Ok(crate::UserTemplate {
                contract_type_id: row.get(6)?,
                id: row.get(0)?,
                account_id: row.get(1)?,
                name: row.get(2)?,
                icon_id: row.get(3)?,
                properties,
                category: row.get(5)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        });

        match result {
            Ok(tpl) => Ok(Some(tpl)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("load_user_template: {}", e)),
        }
    }

    /// List all user templates for a given account, ordered by created_at DESC.
    pub fn list_user_templates(
        &self,
        account_id: &str,
    ) -> Result<Vec<crate::UserTemplate>, String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn.prepare(
        "SELECT id, account_id, name, icon_id, properties_json, category, contract_type_id, created_at, updated_at
         FROM user_templates WHERE account_id = ?1 ORDER BY created_at ASC"
    ).map_err(|e| format!("prepare list_user_templates: {}", e))?;

        let rows = stmt
            .query_map(params![account_id], |row| {
                let props_json: String = row.get(4)?;
                let decrypted = decrypt_text_field(&key, &props_json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        4,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Template properties decryption failed: {}", e),
                        )),
                    )
                })?;
                let properties: Vec<crate::TemplateProperty> = serde_json::from_str(&decrypted)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                Ok(crate::UserTemplate {
                    contract_type_id: row.get(6)?,
                    id: row.get(0)?,
                    account_id: row.get(1)?,
                    name: row.get(2)?,
                    icon_id: row.get(3)?,
                    properties,
                    category: row.get(5)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .map_err(|e| format!("list_user_templates query: {}", e))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| format!("list_user_templates row: {}", e))?);
        }
        Ok(result)
    }
    /// Find a user template by content hash.
    ///
    /// Scans all user templates for the given account, computing the content hash
    /// for each and returning the first match. Two templates with identical content
    /// (same name, icon_id, category, contract_type_id, and properties) produce the
    /// same hash regardless of their database IDs.
    pub fn find_user_template_by_content_hash(
        &self,
        account_id: &str,
        hash: &str,
    ) -> Result<Option<crate::UserTemplate>, String> {
        let templates = self.list_user_templates(account_id)?;
        for tpl in templates {
            if crate::template_hash::user_template_content_hash(&tpl) == hash {
                return Ok(Some(tpl));
            }
        }
        Ok(None)
    }

    /// Delete a user template by ID.
    pub fn delete_user_template(&self, template_id: &str) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        conn.execute(
            "DELETE FROM user_templates WHERE id = ?1",
            params![template_id],
        )
        .map_err(|e| format!("delete_user_template: {}", e))?;
        drop(guard);
        self.record_tombstone("user_templates", template_id)?;
        Ok(())
    }

    /// Count user templates for an account.
    pub fn count_user_templates(&self, account_id: &str) -> Result<usize, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM user_templates WHERE account_id = ?1",
                params![account_id],
                |r| r.get(0),
            )
            .map_err(|e| format!("count_user_templates: {}", e))?;
        Ok(count as usize)
    }

    /// Check whether a template field is used by any object (active or soft-deleted).
    /// Returns (active_count, soft_deleted_count).
    pub fn check_field_usage(
        &self,
        account_id: &str,
        field_key: &str,
    ) -> Result<(usize, usize), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let like = format!("%\"{}\":%", field_key);
        let active: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM objects WHERE account_id = ?1 AND properties LIKE ?2 AND is_deleted = 0",
            params![account_id, &like],
            |r| r.get(0),
        )
        .map_err(|e| format!("check_field_usage active: {}", e))?;
        let soft_deleted: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM objects WHERE account_id = ?1 AND properties LIKE ?2 AND is_deleted = 1",
            params![account_id, &like],
            |r| r.get(0),
        )
        .map_err(|e| format!("check_field_usage soft_deleted: {}", e))?;
        Ok((active as usize, soft_deleted as usize))
    }
}
