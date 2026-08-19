//! Snapshot CRUD 域 —— 自 `storage.rs` 拆分（P223-② 表域拆分）。
//!
//! 本模块承载 `VaultStore` 的对象历史快照读写方法（list_snapshots / get_snapshot /
//! count_snapshots_batch / delete_snapshots / snapshots_size_batch / save_snapshot(_at) /
//! copy_snapshots 与一次性迁移修复 backfill_missing_snapshots / repair_restored_objects /
//! backfill_missing_property_labels 等 11 个方法，原 storage.rs 2558-3001 行，逐行搬运零行为变更）。
//!
//! 快照方法全部为 `pub fn`，跨模块调用（src-tauri 命令 / CLI / 根模块 open 迁移路径）无需放宽可见性；
//! 跨域助手（`data_key()` / `get_sys_config` / `set_sys_config` / objects 域 `load_object`/`save_object` /
//! 模板域 `load_user_template`）均为 VaultStore 固有方法或 storage 子树内可见，直接经 `self.` 调用。
//! `normalize_details_text` 仅被根模块 `log_structured` 使用，留在父模块（属审计日志域）。

use rusqlite::OptionalExtension;

use super::VaultStore;
use crate::encryption::{decrypt_field, encrypt_field};

impl VaultStore {
    // ── Snapshot CRUD（快照域）──────────────────────────────

    pub fn list_snapshots(&self, object_id: &str) -> Result<Vec<serde_json::Value>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, triggered_by, diff_summary FROM object_snapshots WHERE object_id=?1 ORDER BY timestamp DESC LIMIT 50"
        ).map_err(|e| e.to_string())?;
        let snapshots = stmt
            .query_map(rusqlite::params![object_id], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_,String>(0)?,
                    "timestamp": row.get::<_,i64>(1)?,
                    "triggeredBy": row.get::<_,String>(2)?,
                    "diffSummary": row.get::<_,String>(3)?,
                }))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(snapshots)
    }

    /// P013: 批量加载多对象全部快照（含 data 解密），一次 SQL 替代
    /// 「每对象 list_snapshots + 每快照 get_snapshot」的 N+M 次查询（导出打包场景）。
    /// 保留单对象 LIMIT 50 语义（ROW_NUMBER 窗口函数按 timestamp DESC 取前 50）。
    /// 返回 `(object_id, meta_json, data_bytes)` 按 object_id 升序、对象内时间倒序。
    pub fn list_snapshots_with_data_batch(
        &self,
        object_ids: &[String],
    ) -> Result<Vec<(String, serde_json::Value, Vec<u8>)>, String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        if object_ids.is_empty() {
            return Ok(Vec::new());
        }
        // IN 子句参数绑定（与 count_snapshots_batch 同模式，数量随输入变化属 SQLite 限制，
        // 仅查询本 Vault 内 object_id，无注入风险）。
        let placeholders = std::iter::repeat_n("?", object_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT object_id, id, timestamp, triggered_by, diff_summary, data FROM (\
             SELECT object_id, id, timestamp, triggered_by, diff_summary, data, \
             ROW_NUMBER() OVER (PARTITION BY object_id ORDER BY timestamp DESC) AS rn \
             FROM object_snapshots WHERE object_id IN ({}) \
             ) WHERE rn <= 50",
            placeholders
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(object_ids.iter()), |row| {
                let raw: Vec<u8> = row.get(5)?;
                let decrypted = decrypt_field(&key, &raw).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        5,
                        rusqlite::types::Type::Blob,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Snapshot decryption failed: {}", e),
                        )),
                    )
                })?;
                Ok((
                    row.get::<_, String>(0)?,
                    serde_json::json!({
                        "id": row.get::<_, String>(1)?,
                        "timestamp": row.get::<_, i64>(2)?,
                        "triggeredBy": row.get::<_, String>(3)?,
                        "diffSummary": row.get::<_, String>(4)?,
                    }),
                    decrypted,
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    pub fn get_snapshot(&self, snapshot_id: &str) -> Result<Option<Vec<u8>>, String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let result: Option<Vec<u8>> = conn
            .query_row(
                "SELECT data FROM object_snapshots WHERE id=?1",
                rusqlite::params![snapshot_id],
                |r| {
                    let raw: Vec<u8> = r.get(0)?;
                    decrypt_field(&key, &raw).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Blob,
                            Box::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!("Snapshot decryption failed: {}", e),
                            )),
                        )
                    })
                },
            )
            .optional()
            .map_err(|e| format!("Failed to load snapshot: {}", e))?;
        Ok(result)
    }

    /// 返回快照所属对象 ID（`object_id` 列明文，供归属校验用）。
    pub fn get_snapshot_owner(&self, snapshot_id: &str) -> Result<Option<String>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        conn.query_row(
            "SELECT object_id FROM object_snapshots WHERE id=?1",
            rusqlite::params![snapshot_id],
            |r| r.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| format!("Failed to load snapshot owner: {}", e))
    }

    pub fn count_snapshots_batch(
        &self,
        object_ids: &[String],
    ) -> Result<std::collections::HashMap<String, usize>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        if object_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        // 使用固定 "?" 占位符并通过参数绑定传递 ID，避免字符串拼接。
        // IN 子句数量随输入变化，属于 SQLite 限制；此处仅查询本 Vault 内的 object_id，
        // 不暴露 SQL 注入风险。
        let placeholders = std::iter::repeat_n("?", object_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT object_id, COUNT(*) FROM object_snapshots WHERE object_id IN ({}) GROUP BY object_id",
            placeholders
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let map: std::collections::HashMap<String, usize> = stmt
            .query_map(rusqlite::params_from_iter(object_ids.iter()), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<std::collections::HashMap<String, usize>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(map)
    }

    /// 删除某对象的全部历史快照。
    /// 导入 Overwrite 覆盖场景用于先清空本地旧历史，防止包内快照叠加导致历史数量翻倍。
    pub fn delete_snapshots(&self, object_id: &str) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        conn.execute(
            "DELETE FROM object_snapshots WHERE object_id = ?1",
            rusqlite::params![object_id],
        )
        .map_err(|e| format!("delete_snapshots: {}", e))?;
        Ok(())
    }

    /// 批量统计多个对象的快照数据总字节数（`LENGTH(data)`，加密后大小）。
    /// 仅用于导出体积估算，不涉及解密。
    pub fn snapshots_size_batch(&self, object_ids: &[String]) -> Result<u64, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        if object_ids.is_empty() {
            return Ok(0);
        }
        let placeholders = std::iter::repeat_n("?", object_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT COALESCE(SUM(LENGTH(data)), 0) FROM object_snapshots WHERE object_id IN ({})",
            placeholders
        );
        let total: i64 = conn
            .query_row(&sql, rusqlite::params_from_iter(object_ids.iter()), |row| {
                row.get(0)
            })
            .map_err(|e| e.to_string())?;
        Ok(total as u64)
    }

    /// 一次性迁移：为当前 Vault 中没有 snapshot 的活跃对象补一条初始 snapshot。
    /// 通过 sys_config 标记避免重复执行。
    pub fn backfill_missing_snapshots(&self) -> Result<usize, String> {
        const BACKFILL_FLAG: &str = "snapshot_backfill_v1";
        if self
            .get_sys_config(BACKFILL_FLAG)?
            .map(|v| v == "1")
            .unwrap_or(false)
        {
            return Ok(0);
        }

        let object_ids: Vec<String> = {
            let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
            let conn = guard.as_mut().ok_or("Vault is locked")?;
            let mut stmt = conn
                .prepare(
                    "SELECT o.id FROM objects o
                     LEFT JOIN object_snapshots s ON o.id = s.object_id
                     WHERE o.is_deleted = 0 AND s.object_id IS NULL",
                )
                .map_err(|e| e.to_string())?;
            let ids: Vec<String> = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            ids
        };

        let mut created = 0;
        for id in &object_ids {
            if let Ok(Some(obj)) = self.load_object(id) {
                let snapshot_data = serde_json::to_vec(&serde_json::json!({
                    "name": obj.name,
                    "tags": obj.tags_json,
                    "properties": obj.properties,
                }))
                .unwrap_or_default();
                if self
                    .save_snapshot(
                        id,
                        "backfill",
                        &snapshot_data,
                        "Auto-created initial snapshot",
                    )
                    .is_ok()
                {
                    created += 1;
                }
            }
        }

        let _ = self.set_sys_config(BACKFILL_FLAG, "1");
        Ok(created)
    }

    /// 修复因旧版 object_restore 用 snake_case 读取 camelCase trash 数据导致的“隐形”对象。
    /// 这些对象仍在数据库中，但 account_id / type_id / parent_id 错误，导致前端按页面筛选时看不到。
    /// 通过 sys_config 标记确保每个 Vault 只执行一次。
    pub fn repair_restored_objects(&self) -> Result<usize, String> {
        const REPAIR_FLAG: &str = "restored_objects_repair_v1";
        if self
            .get_sys_config(REPAIR_FLAG)?
            .map(|v| v == "1")
            .unwrap_or(false)
        {
            return Ok(0);
        }

        let account_id = self.config.account_id.clone();
        let built_in_sections = ["identity", "travel", "financial", "professional"];

        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;

        let mut stmt = conn
            .prepare(
                "SELECT id, section_type, type_id, parent_id, account_id
                 FROM objects
                 WHERE is_deleted = 0
                   AND (account_id = 'imported'
                        OR (type_id = 'note' AND section_type != 'note')
                        OR (parent_id IS NULL AND section_type NOT IN ('identity','travel','financial','professional') AND id != section_type))",
            )
            .map_err(|e| e.to_string())?;

        #[derive(Debug)]
        struct Row {
            id: String,
            section_type: String,
            type_id: String,
            parent_id: Option<String>,
            account_id: String,
        }

        let rows: Vec<Row> = stmt
            .query_map([], |row| {
                Ok(Row {
                    id: row.get(0)?,
                    section_type: row.get(1)?,
                    type_id: row.get(2)?,
                    parent_id: row.get(3)?,
                    account_id: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        let mut fixed = 0usize;
        let now = chrono::Utc::now().to_rfc3339();

        for row in rows {
            let mut new_account_id = row.account_id.clone();
            let mut new_type_id = row.type_id.clone();
            let mut new_parent_id = row.parent_id.clone();

            if new_account_id == "imported" {
                new_account_id = account_id.clone();
            }
            if new_type_id == "note" && row.section_type != "note" {
                new_type_id = row.section_type.clone();
            }
            if new_parent_id.is_none() && !built_in_sections.contains(&row.section_type.as_str()) {
                // 自定义页面：尝试把 section_type 对应的页面对象 ID 作为 parent_id
                let page_id: Option<String> = conn
                    .query_row(
                        "SELECT id FROM objects WHERE id = ?1 AND is_deleted = 0 LIMIT 1",
                        rusqlite::params![&row.section_type],
                        |r| r.get(0),
                    )
                    .optional()
                    .unwrap_or(None);
                if page_id.is_some() {
                    new_parent_id = page_id;
                }
            }

            if new_account_id != row.account_id
                || new_type_id != row.type_id
                || new_parent_id != row.parent_id
            {
                conn.execute(
                    "UPDATE objects
                     SET account_id = ?1, type_id = ?2, parent_id = ?3, updated_at = ?4, version = version + 1
                     WHERE id = ?5",
                    rusqlite::params![
                        new_account_id,
                        new_type_id,
                        new_parent_id,
                        &now,
                        &row.id
                    ],
                )
                .map_err(|e| format!("repair object {}: {}", row.id, e))?;
                fixed += 1;
            }
        }

        drop(stmt);
        drop(guard);
        let _ = self.set_sys_config(REPAIR_FLAG, "1");
        Ok(fixed)
    }

    /// 为旧版 object_restore 恢复出来的对象补齐缺失的 property_labels。
    /// 这些对象的字段级敏感度副本在旧 bug 中丢失，导致所有字段都显示为“内部”。
    /// 本方法只读取模板并为 property_labels 为空的对象写入敏感度映射，不会覆盖已有数据。
    /// 通过 sys_config 标记确保每个 Vault 只执行一次。
    pub fn backfill_missing_property_labels(&self) -> Result<usize, String> {
        const BACKFILL_FLAG: &str = "property_labels_backfill_v1";
        if self
            .get_sys_config(BACKFILL_FLAG)?
            .map(|v| v == "1")
            .unwrap_or(false)
        {
            return Ok(0);
        }

        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;

        let ids: Vec<String> = conn
            .prepare(
                "SELECT id FROM objects
                 WHERE is_deleted = 0 AND template_id IS NOT NULL",
            )
            .map_err(|e| e.to_string())?
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        drop(guard);

        let mut filled = 0usize;
        for id in ids {
            let mut record = match self.load_object(&id)? {
                Some(r) => r,
                None => continue,
            };

            // 已有非空 property_labels 时不覆盖
            let needs_fill = match record.property_labels.as_ref() {
                None => true,
                Some(v) => v.as_object().map(|m| m.is_empty()).unwrap_or(true),
            };
            if !needs_fill {
                continue;
            }

            let Some(ref template_id) = record.template_id else {
                continue;
            };
            let tpl = match self.load_user_template(template_id)? {
                Some(t) => t,
                None => continue,
            };

            let mut labels = serde_json::Map::new();
            for prop in &tpl.properties {
                if let Some(ref sl) = prop.sensitivity_level {
                    labels.insert(prop.id.clone(), serde_json::Value::String(sl.clone()));
                }
            }
            if labels.is_empty() {
                continue;
            }

            record.property_labels = Some(serde_json::Value::Object(labels));
            record.updated_at = chrono::Utc::now().to_rfc3339();
            record.version += 1;
            self.save_object(&record)?;
            filled += 1;
        }

        let _ = self.set_sys_config(BACKFILL_FLAG, "1");
        Ok(filled)
    }

    /// §25.5 — Save an object snapshot for history（使用当前时间戳）。
    pub fn save_snapshot(
        &self,
        object_id: &str,
        triggered_by: &str,
        data: &[u8],
        diff_summary: &str,
    ) -> Result<(), String> {
        self.save_snapshot_at(
            object_id,
            triggered_by,
            data,
            diff_summary,
            chrono::Utc::now().timestamp_millis(),
        )
    }

    /// §25.5 — Save an object snapshot for history with an explicit timestamp。
    /// 导入/恢复时用于保留原设备上的历史顺序（跨设备恢复后历史记录保持一致）。
    pub fn save_snapshot_at(
        &self,
        object_id: &str,
        triggered_by: &str,
        data: &[u8],
        diff_summary: &str,
        timestamp_ms: i64,
    ) -> Result<(), String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let id = uuid::Uuid::new_v4().to_string();
        let encrypted_data = encrypt_field(&key, data)?;
        conn.execute(
            "INSERT INTO object_snapshots (id, object_id, timestamp, triggered_by, data, diff_summary)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                id,
                object_id,
                timestamp_ms,
                triggered_by,
                encrypted_data,
                diff_summary
            ],
        )
        .map_err(|e| format!("save_snapshot: {}", e))?;
        Ok(())
    }

    /// Copy all snapshots from one object to another (used when restoring a trashed object
    /// under a new ID to preserve its history).
    pub fn copy_snapshots(&self, from_object_id: &str, to_object_id: &str) -> Result<(), String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn
            .prepare(
                "SELECT timestamp, triggered_by, data, diff_summary
             FROM object_snapshots WHERE object_id = ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows: Vec<(i64, String, Vec<u8>, String)> = stmt
            .query_map(rusqlite::params![from_object_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        drop(stmt);
        let mut insert = conn.prepare(
            "INSERT INTO object_snapshots (id, object_id, timestamp, triggered_by, data, diff_summary)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
        ).map_err(|e| e.to_string())?;
        for (timestamp, triggered_by, raw_data, diff_summary) in rows {
            let plain = decrypt_field(&key, &raw_data)?;
            let encrypted = encrypt_field(&key, &plain)?;
            let id = uuid::Uuid::new_v4().to_string();
            insert
                .execute(rusqlite::params![
                    id,
                    to_object_id,
                    timestamp,
                    triggered_by,
                    encrypted,
                    diff_summary
                ])
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
