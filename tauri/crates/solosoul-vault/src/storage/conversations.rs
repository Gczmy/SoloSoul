//! LLM 会话域（P004）—— 自 profile preferences blob 迁出的按行存储。
//!
//! 背景：会话此前整体存于 profile preferences 的 `llmConversations` blob，任何
//! save/list/rename/delete 都触发「整 blob 解密 → 深克隆全部会话 → 序列化 →
//! 加密 → 写盘」，每条聊天消息（流式结束触发 llm_save_conversation）都全量重写。
//!
//! 本域提供 `llm_conversations` 表的行级 CRUD（data 列为 AES-256-GCM 加密 JSON，
//! 与 profiles.data 同款 `encrypt_field`/`decrypt_field`），以及同步变更清单
//! （list_conversation_changes_since）与应用（apply_conversation_sync_record_tx），
//! 对齐 objects/audit_log 的存储方式。

use rusqlite::{params, Connection, OptionalExtension};

use super::{with_tx, VaultStore};
use crate::encryption::{decrypt_field, encrypt_field, DataEncryptionKey};
use crate::BorrowedSyncRecord;

impl VaultStore {
    /// 保存/覆盖单条会话（按 id 行级写入，不触碰其他会话）。返回新 HLC。
    pub fn save_conversation(
        &self,
        account_id: &str,
        id: &str,
        updated_at: &str,
        data: &[u8],
    ) -> Result<crate::RecordHlc, String> {
        let key = self.data_key()?;
        let hlc = self.new_local_hlc()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        with_tx(
            conn,
            "save_conversation begin",
            "save_conversation commit",
            |c| {
                Self::save_conversation_tx(c, &key, account_id, id, updated_at, data)?;
                Self::set_record_hlc_tx(c, "llm_conversations", id, &hlc)?;
                Ok(())
            },
        )?;
        Ok(hlc)
    }

    /// P115 风格：事务内保存会话（连接由调用方持有，批量应用单事务内复用）。
    pub(crate) fn save_conversation_tx(
        conn: &mut Connection,
        key: &DataEncryptionKey,
        account_id: &str,
        id: &str,
        updated_at: &str,
        data: &[u8],
    ) -> Result<(), String> {
        let encrypted = encrypt_field(key, data)?;
        conn.execute(
            "INSERT INTO llm_conversations (id, account_id, data, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET data = excluded.data, updated_at = excluded.updated_at",
            params![id, account_id, encrypted, updated_at],
        )
        .map_err(|e| format!("save_conversation: {e}"))?;
        Ok(())
    }

    /// 读取单条会话明文 data（无记录返回 None）。仅解密目标行。
    pub fn load_conversation(&self, account_id: &str, id: &str) -> Result<Option<Vec<u8>>, String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let result = conn
            .query_row(
                "SELECT data FROM llm_conversations WHERE id = ?1 AND account_id = ?2",
                params![id, account_id],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .optional()
            .map_err(|e| format!("load_conversation: {e}"))?;
        match result {
            Some(raw) => decrypt_field(&key, &raw)
                .map(Some)
                .map_err(|e| format!("load_conversation decrypt: {e}")),
            None => Ok(None),
        }
    }

    /// 列出账户全部会话的 (id, updated_at, 明文 data)。仅解密本账户行。
    pub fn list_conversations(
        &self,
        account_id: &str,
    ) -> Result<Vec<(String, String, Vec<u8>)>, String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn
            .prepare(
                "SELECT id, updated_at, data FROM llm_conversations WHERE account_id = ?1
                 ORDER BY updated_at ASC",
            )
            .map_err(|e| format!("list_conversations: {e}"))?;
        let rows = stmt
            .query_map(params![account_id], |row| {
                let raw: Vec<u8> = row.get(2)?;
                let data = decrypt_field(&key, &raw).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        2,
                        rusqlite::types::Type::Blob,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Conversation decryption failed: {e}"),
                        )),
                    )
                })?;
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, data))
            })
            .map_err(|e| format!("list_conversations query: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("list_conversations collect: {e}"))?;
        Ok(rows)
    }

    /// 统计账户会话密文总字节数（纯 SQL SUM(LENGTH(data))，不解密）。
    /// P004: 替代旧的 profile blob 读取（vault stats 的 ai_conversations_size）。
    pub fn conversations_size(&self, account_id: &str) -> Result<u64, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let total: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(LENGTH(data)), 0) FROM llm_conversations WHERE account_id = ?1",
                params![account_id],
                |r| r.get(0),
            )
            .map_err(|e| format!("conversations_size: {e}"))?;
        Ok(total as u64)
    }

    /// 物理删除会话行（记墓碑，供设备同步传播删除）。
    pub fn delete_conversation(&self, account_id: &str, id: &str) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let affected = conn
            .execute(
                "DELETE FROM llm_conversations WHERE id = ?1 AND account_id = ?2",
                params![id, account_id],
            )
            .map_err(|e| format!("delete_conversation: {e}"))?;
        drop(guard);
        if affected > 0 {
            self.record_tombstone("llm_conversations", id)?;
        }
        Ok(())
    }

    /// 事务内物理删除（sync_apply 远端 tombstone 路径使用）。
    pub(crate) fn delete_conversation_tx(conn: &mut Connection, id: &str) -> Result<(), String> {
        conn.execute("DELETE FROM llm_conversations WHERE id = ?1", params![id])
            .map_err(|e| format!("delete_conversation_tx: {e}"))?;
        Ok(())
    }

    /// 同步变更清单：会话表为小表，维持内存分页（先按有效 HLC 升序再 take）。
    pub(crate) fn list_conversation_changes_since(
        &self,
        watermark: &crate::SyncWatermark,
        account_id: &str,
        local_node_id: &str,
    ) -> Result<Vec<crate::VaultSyncRecord>, String> {
        let key = self.data_key()?;
        let rows = {
            let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
            let conn = guard.as_mut().ok_or("Vault is locked")?;
            let mut stmt = conn
                .prepare(
                    "SELECT id, account_id, data, updated_at FROM llm_conversations WHERE account_id = ?1",
                )
                .map_err(|e| format!("list_conversation_changes: {e}"))?;
            let rows = stmt
                .query_map(params![account_id], |row| {
                    let raw: Vec<u8> = row.get(2)?;
                    let data = decrypt_field(&key, &raw).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            2,
                            rusqlite::types::Type::Blob,
                            Box::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!("Conversation decryption failed: {e}"),
                            )),
                        )
                    })?;
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        data,
                        row.get::<_, String>(3)?,
                    ))
                })
                .map_err(|e| format!("list_conversation_changes query: {e}"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("list_conversation_changes collect: {e}"))?;
            rows
        };

        let mut out = Vec::new();
        for (id, _acct, data, updated) in rows {
            let hlc =
                self.record_hlc_or_fallback("llm_conversations", &id, &updated, local_node_id)?;
            if !Self::hlc_after_watermark(&hlc, watermark) {
                continue;
            }
            let value = serde_json::json!({
                "id": id,
                "accountId": account_id,
                "data": base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    &data
                ),
                "updatedAt": updated,
            });
            out.push(crate::VaultSyncRecord {
                id,
                table: "llm_conversations".to_string(),
                data: value,
                hlc,
                deleted: false,
            });
        }
        let mut tombstones =
            self.list_tombstones_since("llm_conversations", watermark, local_node_id)?;
        out.append(&mut tombstones);
        Ok(out)
    }

    /// 事务内应用单条会话同步记录（连接由调用方持有）。
    pub(crate) fn apply_conversation_sync_record_tx(
        conn: &mut Connection,
        key: &DataEncryptionKey,
        record: &BorrowedSyncRecord,
    ) -> Result<bool, String> {
        // 墓碑（data 为 null，deleted=true）→ 删除本地会话行。不重记本地墓碑，
        // 远程 HLC 保持权威删除时间戳。
        if record.deleted && record.data.is_null() {
            Self::delete_conversation_tx(conn, record.id)?;
            return Ok(true);
        }
        let data_b64 = record
            .data
            .get("data")
            .and_then(|v| v.as_str())
            .ok_or("Missing conversation data")?;
        let data = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data_b64)
            .map_err(|e| format!("conversation data decode: {e}"))?;
        let updated = record
            .data
            .get("updatedAt")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(Self::now_rfc3339);
        // 会话为账户级数据，同步记录携带 accountId（与 ObjectRecord 自带 account_id 同理），
        // 本地按该账户 upsert，保证与 list_conversations 的账户过滤一致。
        let account_id = record
            .data
            .get("accountId")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        Self::save_conversation_tx(conn, key, account_id, record.id, &updated, &data)?;
        Ok(true)
    }

    /// 冲突 UI 本地快照（对齐其余表）。
    pub(crate) fn conversation_local_snapshot(
        &self,
        id: &str,
    ) -> Result<Option<serde_json::Value>, String> {
        match self.load_conversation("", id)? {
            Some(data) => serde_json::from_slice(&data)
                .map(Some)
                .map_err(|e| format!("serialize conversation: {e}")),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VaultConfig;
    use tempfile::TempDir;

    fn setup_vault() -> (VaultStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let config =
            VaultConfig::new("test_account", dir.path().to_path_buf()).with_data_key([0x42u8; 32]);
        let vault = VaultStore::open(config).unwrap();
        (vault, dir)
    }

    #[test]
    fn test_conversation_row_crud() {
        let (vault, _dir) = setup_vault();
        let account_id = "test_account";

        // 空账户
        assert!(vault.list_conversations(account_id).unwrap().is_empty());
        assert!(vault.load_conversation(account_id, "c1").unwrap().is_none());
        assert_eq!(vault.conversations_size(account_id).unwrap(), 0);

        // 写入两行
        let hlc = vault
            .save_conversation(account_id, "c1", "2024-01-01T00:00:00Z", b"{\"id\":\"c1\"}")
            .unwrap();
        assert!(hlc.wall_time_ms > 0);
        vault
            .save_conversation(account_id, "c2", "2024-02-01T00:00:00Z", b"{\"id\":\"c2\"}")
            .unwrap();

        // 行级读取仅返回目标行
        let c1 = vault.load_conversation(account_id, "c1").unwrap().unwrap();
        assert_eq!(c1, b"{\"id\":\"c1\"}".to_vec());
        assert!(vault
            .load_conversation(account_id, "nope")
            .unwrap()
            .is_none());

        // 列表两行
        let rows = vault.list_conversations(account_id).unwrap();
        assert_eq!(rows.len(), 2);
        // 按 updated_at 升序：c1 在前
        assert_eq!(rows[0].0, "c1");

        // 覆盖更新（upsert 不新增行）
        vault
            .save_conversation(
                account_id,
                "c1",
                "2024-03-01T00:00:00Z",
                b"{\"id\":\"c1\",\"v\":2}",
            )
            .unwrap();
        let rows = vault.list_conversations(account_id).unwrap();
        assert_eq!(rows.len(), 2);
        let c1_new = vault.load_conversation(account_id, "c1").unwrap().unwrap();
        assert_eq!(c1_new, b"{\"id\":\"c1\",\"v\":2}".to_vec());

        // 尺寸统计为密文长度（> 明文长度，且 > 0）
        let size = vault.conversations_size(account_id).unwrap();
        assert!(size > 0);

        // 物理删除 + 墓碑
        vault.delete_conversation(account_id, "c1").unwrap();
        assert!(vault.load_conversation(account_id, "c1").unwrap().is_none());
        let rows = vault.list_conversations(account_id).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "c2");
        // 墓碑已记录（供同步传播删除）
        let tombstones = vault
            .list_tombstones_since(
                "llm_conversations",
                &crate::SyncWatermark {
                    wall_time_ms: 0,
                    counter: 0,
                    node_id: String::new(),
                },
                "node_a",
            )
            .unwrap();
        assert!(
            tombstones.iter().any(|t| t.id == "c1" && t.deleted),
            "删除会话必须记录墓碑"
        );
    }

    #[test]
    fn test_conversation_sync_changes_and_apply() {
        let (vault, _dir) = setup_vault();
        let account_id = "test_account";
        vault
            .save_conversation(account_id, "c1", "2024-01-01T00:00:00Z", b"{\"id\":\"c1\"}")
            .unwrap();

        // 变更清单包含新会话
        let recs = vault
            .list_sync_changes_since(
                "llm_conversations",
                &crate::SyncWatermark {
                    wall_time_ms: 0,
                    counter: 0,
                    node_id: String::new(),
                },
                account_id,
                "node_a",
            )
            .unwrap();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].table, "llm_conversations");
        assert_eq!(recs[0].id, "c1");
        assert!(!recs[0].deleted);
        // data 为 base64 明文 JSON
        let b64 = recs[0].data.get("data").and_then(|v| v.as_str()).unwrap();
        let decoded =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64).unwrap();
        assert_eq!(decoded, b"{\"id\":\"c1\"}".to_vec());

        // 对端 apply：写入另一账户数据，再同步应用
        let remote: crate::VaultSyncRecord = crate::VaultSyncRecord {
            id: "c2".to_string(),
            table: "llm_conversations".to_string(),
            data: serde_json::json!({
                "id": "c2",
                "accountId": account_id,
                "data": base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    b"{\"id\":\"c2\"}"
                ),
                "updatedAt": "2024-05-01T00:00:00Z",
            }),
            hlc: crate::RecordHlc {
                wall_time_ms: 1000,
                counter: 1,
                node_id: "remote-node".to_string(),
            },
            deleted: false,
        };
        let local_node = "node_a";
        assert!(vault.apply_sync_record(&remote, local_node).unwrap());
        let c2 = vault.load_conversation(account_id, "c2").unwrap().unwrap();
        assert_eq!(c2, b"{\"id\":\"c2\"}".to_vec());

        // 远端墓碑删除
        let tomb: crate::VaultSyncRecord = crate::VaultSyncRecord {
            id: "c2".to_string(),
            table: "llm_conversations".to_string(),
            data: serde_json::Value::Null,
            hlc: crate::RecordHlc {
                wall_time_ms: 2000,
                counter: 1,
                node_id: "remote-node".to_string(),
            },
            deleted: true,
        };
        assert!(vault.apply_sync_record(&tomb, local_node).unwrap());
        assert!(vault.load_conversation(account_id, "c2").unwrap().is_none());

        // 本地冲突（旧 HLC）应被拒绝
        let stale: crate::VaultSyncRecord = crate::VaultSyncRecord {
            id: "c1".to_string(),
            table: "llm_conversations".to_string(),
            data: serde_json::json!({
                "id": "c1",
                "accountId": account_id,
                "data": "e30=",
            }), // base64 "{}"
            hlc: crate::RecordHlc {
                wall_time_ms: 1,
                counter: 0,
                node_id: "remote-node".to_string(),
            },
            deleted: false,
        };
        assert!(!vault.apply_sync_record(&stale, local_node).unwrap());
        let c1 = vault.load_conversation(account_id, "c1").unwrap().unwrap();
        assert_eq!(c1, b"{\"id\":\"c1\"}".to_vec(), "旧 HLC 不得覆盖本地");
    }
}
