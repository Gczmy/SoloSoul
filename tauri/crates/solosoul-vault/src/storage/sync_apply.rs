//! 同步应用域 —— 自 `storage.rs` 拆分（P223-② 表域拆分第六域）。
//!
//! 本模块承载 `VaultStore` 的同步记录应用与冲突处理方法（原 storage.rs 1069-1511 行，
//! 逐行搬运零行为变更）：`apply_sync_record`（单事务）/ `apply_sync_records_batch`
//! （P115 批量单事务）/ 四表域 `apply_*_sync_record_tx` 私有实现，以及同步冲突
//! 全生命周期（`save_sync_conflicts_batch` / `list_sync_conflicts` / `get_sync_conflict` /
//! `get_sync_conflict_local_data` / `delete_sync_conflict` / `resolve_sync_conflict`）
//! 与两个私有助手（`record_hlc_is_newer`、`hard_delete_record`）。
//!
//! 共享设施经 `super::` 访问父模块私有项（`with_tx` 自由函数）；跨域 pub(crate) 助手
//! 按原路径引用：`save_object_tx`/`load_object_tx`（objects 域）、`save_trash_item_tx`
//! （trash 域）、`now_rfc3339`/`set_record_hlc*`/`get_record_hlc_tx`/`record_tombstone`
//! （sync_meta 域）。8 个 pub API 可见性不变（solosoul-sync delta.rs 与 src-tauri
//! commands/sync.rs 跨 crate 调用 + 根测试模块）；域内私有方法保持私有。

use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;

use super::{with_tx, VaultStore};
use crate::encryption::{decrypt_field, DataEncryptionKey};

impl VaultStore {
    /// Apply a single incoming sync record. Returns true if the local state changed.
    /// 单条记录应用（单事务语义：HLC 预检 + 写入 + HLC 更新全部在一个事务内）。
    pub fn apply_sync_record(
        &self,
        record: &crate::VaultSyncRecord,
        local_node_id: &str,
    ) -> Result<bool, String> {
        let borrowed = crate::BorrowedSyncRecord {
            id: &record.id,
            table: &record.table,
            data: &record.data,
            hlc: &record.hlc,
            deleted: record.deleted,
        };
        let key = self.data_key()?;
        let ui_prefs_enabled = self.ui_prefs_sync_enabled();
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        // P213: 手动事务（Transaction 无 DerefMut，prepare_cached 需要 &mut Connection）。
        let outcome = with_tx(
            conn,
            "apply_sync_record begin",
            "apply_sync_record commit",
            |c| Self::apply_sync_record_tx(c, &key, &borrowed, local_node_id, ui_prefs_enabled),
        )?;
        Ok(outcome.applied)
    }

    /// P115: 事务内应用单条同步记录（连接由调用方持有）。
    ///
    /// 与 `apply_sync_record` 的差异：不获取/持有连接，适用于批量事务循环；
    /// 返回 [`crate::SyncApplyOutcome`]，包含写前本地 HLC 供冲突报告复用。
    fn apply_sync_record_tx(
        conn: &mut Connection,
        key: &DataEncryptionKey,
        record: &crate::BorrowedSyncRecord,
        local_node_id: &str,
        ui_prefs_enabled: bool,
    ) -> Result<crate::SyncApplyOutcome, String> {
        // Conflict resolution: only accept records with HLC greater than the local HLC.
        let local_hlc = Self::get_record_hlc_tx(conn, record.table, record.id)?;
        if let Some(ref cur) = local_hlc {
            if !Self::record_hlc_is_newer(record.hlc, cur) {
                return Ok(crate::SyncApplyOutcome {
                    applied: false,
                    local_hlc,
                    error: None,
                });
            }
        }

        let applied = match record.table {
            "profiles" => Self::apply_profile_sync_record_tx(conn, key, record, ui_prefs_enabled),
            "objects" => Self::apply_object_sync_record_tx(conn, key, record, local_node_id),
            "user_templates" => Self::apply_user_template_sync_record_tx(conn, key, record),
            "trash_items" => Self::apply_trash_sync_record_tx(conn, key, record),
            "llm_conversations" => Self::apply_conversation_sync_record_tx(conn, key, record),
            _ => Err(format!("Unsupported sync table: {}", record.table)),
        }?;

        if applied {
            Self::set_record_hlc_tx(conn, record.table, record.id, record.hlc)?;
        }
        Ok(crate::SyncApplyOutcome {
            applied,
            local_hlc,
            error: None,
        })
    }

    /// P115: 整批应用同步记录——单连接 + 单事务 + 借用视图（零克隆）。
    ///
    /// 相比旧实现（`apply_sync_records` 逐条 `apply_sync_record`）：
    ///   - 整批包一个事务：所有 INSERT/UPDATE/DELETE + sync_hlc 更新一次 commit；
    ///   - HLC 重复查询 ×2 → ×1：每条记录只查一次本地 HLC，结果带写前本地 HLC
    ///     供调用方冲突报告复用，不再二次查询；
    ///   - `data` 传引用：接收 [`crate::BorrowedSyncRecord`] 借用视图，不再逐条克隆 JSON。
    ///
    /// 返回每条记录的应用结果；单条记录失败不中断整批（错误落入 `SyncApplyOutcome.error`），
    /// 整批事务最终统一 commit/rollback。
    pub fn apply_sync_records_batch(
        &self,
        records: &[crate::BorrowedSyncRecord],
        local_node_id: &str,
    ) -> Result<Vec<crate::SyncApplyOutcome>, String> {
        let key = self.data_key()?;
        let ui_prefs_enabled = self.ui_prefs_sync_enabled();
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?; // P213: 手动事务（Transaction 无 DerefMut，prepare_cached 需要 &mut Connection）。
        with_tx(conn, "apply batch begin", "apply batch commit", |c| {
            let mut outcomes = Vec::with_capacity(records.len());
            for record in records {
                // 单条失败不中断整批：错误落入 outcome，事务最终统一 commit/rollback。
                match Self::apply_sync_record_tx(c, &key, record, local_node_id, ui_prefs_enabled) {
                    Ok(outcome) => outcomes.push(outcome),
                    Err(e) => outcomes.push(crate::SyncApplyOutcome {
                        applied: false,
                        local_hlc: None,
                        error: Some(e),
                    }),
                }
            }
            Ok(outcomes)
        })
    }

    fn record_hlc_is_newer(remote: &crate::RecordHlc, local: &crate::RecordHlc) -> bool {
        remote.wall_time_ms > local.wall_time_ms
            || (remote.wall_time_ms == local.wall_time_ms
                && (remote.counter > local.counter
                    || (remote.counter == local.counter && remote.node_id > local.node_id)))
    }

    // ── Sync conflict helpers ────────────────────────────────

    /// P016: 批量持久化同步冲突——单连接 + 单事务，N 条冲突一次 commit。
    ///
    /// 相比旧单条版（每次锁 + execute + 隐式事务）：大量冲突时
    /// 不再 N 次锁竞争与 N 次写事务，写入开销从 O(N) 事务降至 O(1) 事务。
    pub fn save_sync_conflicts_batch(
        &self,
        entries: &[crate::SyncConflictBatchEntry],
    ) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let now = Self::now_rfc3339();
        with_tx(conn, "save conflicts begin", "save conflicts commit", |c| {
            for e in entries {
                let local_hlc_json =
                    serde_json::to_string(&e.local_hlc).map_err(|err| err.to_string())?;
                let remote_hlc_json =
                    serde_json::to_string(&e.remote_hlc).map_err(|err| err.to_string())?;
                let local_data_json =
                    serde_json::to_string(&e.local_data).map_err(|err| err.to_string())?;
                let remote_data_json =
                    serde_json::to_string(&e.remote_data).map_err(|err| err.to_string())?;
                c.execute(
                    "INSERT INTO sync_conflicts (id, table_name, record_id, local_hlc, remote_hlc, local_data, remote_data, remote_deleted, winner, created_at, resolved)
                     VALUES ((lower(hex(randomblob(16)))), ?1, ?2, ?3, ?4, ?5, ?6, ?7, 'local', ?8, 0)
                     ON CONFLICT(table_name, record_id) DO UPDATE SET
                        local_hlc = excluded.local_hlc,
                        remote_hlc = excluded.remote_hlc,
                        local_data = excluded.local_data,
                        remote_data = excluded.remote_data,
                        remote_deleted = excluded.remote_deleted,
                        winner = excluded.winner",
                    params![
                        e.table,
                        e.record_id,
                        local_hlc_json,
                        remote_hlc_json,
                        local_data_json,
                        remote_data_json,
                        e.remote_deleted,
                        now
                    ],
                )
                .map_err(|err| format!("save_sync_conflicts_batch: {}", err))?;
            }
            Ok(())
        })
    }

    /// P016: 批量获取冲突候选的本地记录同步格式快照（供自动消解比较与冲突展示）。
    ///
    /// objects 表走 `load_objects_batch` 单查询（避免 N+1 锁/查询），其余表逐条复用
    /// `get_sync_conflict_local_data`（解密开销逐条固有）。缺失 id 记为 `None`。
    pub fn get_sync_conflict_local_data_batch(
        &self,
        table: &str,
        ids: &[String],
    ) -> Result<std::collections::HashMap<String, Option<serde_json::Value>>, String> {
        let mut out = std::collections::HashMap::with_capacity(ids.len());
        match table {
            "objects" => {
                let objs = self.load_objects_batch(ids)?;
                for id in ids {
                    let value = match objs.get(id) {
                        Some(obj) => Some(
                            serde_json::to_value(obj)
                                .map_err(|e| format!("serialize object: {}", e))?,
                        ),
                        None => None,
                    };
                    out.insert(id.clone(), value);
                }
            }
            _ => {
                for id in ids {
                    out.insert(id.clone(), self.get_sync_conflict_local_data(table, id)?);
                }
            }
        }
        Ok(out)
    }

    /// 列出所有未解决的同步冲突摘要。
    pub fn list_sync_conflicts(&self) -> Result<Vec<crate::SyncConflictSummary>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn
            .prepare(
                "SELECT id, table_name, record_id, local_hlc, remote_hlc, winner, created_at
             FROM sync_conflicts WHERE resolved = 0 ORDER BY created_at DESC",
            )
            .map_err(|e| format!("list_sync_conflicts: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(crate::SyncConflictSummary {
                    id: row.get(0)?,
                    table_name: row.get(1)?,
                    record_id: row.get(2)?,
                    local_hlc_json: row.get(3)?,
                    remote_hlc_json: row.get(4)?,
                    winner: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })
            .map_err(|e| format!("list_sync_conflicts query: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("list_sync_conflicts collect: {}", e))?;
        Ok(rows)
    }

    /// 获取一条冲突的详情（含本地和远程数据）。
    pub fn get_sync_conflict(
        &self,
        conflict_id: &str,
    ) -> Result<Option<crate::SyncConflictDetail>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let result = conn
        .query_row(
            "SELECT id, table_name, record_id, local_hlc, remote_hlc, local_data, remote_data, remote_deleted, winner, created_at
             FROM sync_conflicts WHERE id = ?1 AND resolved = 0",
            params![conflict_id],
            |row| {
                Ok(crate::SyncConflictDetail {
                    id: row.get(0)?,
                    table_name: row.get(1)?,
                    record_id: row.get(2)?,
                    local_hlc_json: row.get(3)?,
                    remote_hlc_json: row.get(4)?,
                    local_data_json: row.get(5)?,
                    remote_data_json: row.get(6)?,
                    remote_deleted: row.get::<_, i32>(7)? != 0,
                    winner: row.get(8)?,
                    created_at: row.get(9)?,
                })
            },
        )
        .optional()
        .map_err(|e| format!("get_sync_conflict: {}", e))?;
        Ok(result)
    }

    /// 获取当前本地记录的同步格式快照，用于在冲突 UI 中展示冲突发生时的本地状态。
    pub fn get_sync_conflict_local_data(
        &self,
        table: &str,
        record_id: &str,
    ) -> Result<Option<serde_json::Value>, String> {
        let value = match table {
            "profiles" => {
                if let Some(p) = self.load_profile(record_id)? {
                    serde_json::to_value(&p).map_err(|e| format!("serialize profile: {}", e))?
                } else {
                    return Ok(None);
                }
            }
            "objects" => {
                if let Some(obj) = self.load_object(record_id)? {
                    serde_json::to_value(&obj).map_err(|e| format!("serialize object: {}", e))?
                } else {
                    return Ok(None);
                }
            }
            "user_templates" => {
                if let Some(tpl) = self.load_user_template(record_id)? {
                    serde_json::to_value(&tpl).map_err(|e| format!("serialize template: {}", e))?
                } else {
                    return Ok(None);
                }
            }
            "trash_items" => {
                if let Some(item) = self.get_trash_item(record_id)? {
                    serde_json::to_value(&item)
                        .map_err(|e| format!("serialize trash item: {}", e))?
                } else {
                    return Ok(None);
                }
            }
            "llm_conversations" => return self.conversation_local_snapshot(record_id),
            _ => return Ok(None),
        };
        Ok(Some(value))
    }

    /// 将冲突记录标记为已解决并删除。
    pub fn delete_sync_conflict(&self, conflict_id: &str) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        conn.execute(
            "DELETE FROM sync_conflicts WHERE id = ?1",
            params![conflict_id],
        )
        .map_err(|e| format!("delete_sync_conflict: {}", e))?;
        Ok(())
    }

    /// 根据策略解决冲突。
    /// strategy: "keep_local" | "keep_remote" | "dismiss"
    /// 返回是否应用了远程数据（keep_remote 时）。
    pub fn resolve_sync_conflict(&self, conflict_id: &str, strategy: &str) -> Result<bool, String> {
        let detail = match self.get_sync_conflict(conflict_id)? {
            Some(d) => d,
            None => return Err("Conflict not found or already resolved".to_string()),
        };

        let apply_remote = match strategy {
            "keep_remote" => {
                if detail.remote_deleted {
                    // 远程为删除 tombstone，直接删除本地记录并记录 tombstone。
                    self.hard_delete_record(&detail.table_name, &detail.record_id)?;
                    true
                } else {
                    let remote_data: serde_json::Value =
                        serde_json::from_str(&detail.remote_data_json)
                            .map_err(|e| format!("Failed to parse remote_data: {}", e))?;
                    let fresh_hlc = self.new_local_hlc()?;
                    let vault_rec = crate::VaultSyncRecord {
                        id: detail.record_id.clone(),
                        table: detail.table_name.clone(),
                        data: remote_data,
                        hlc: fresh_hlc,
                        deleted: false,
                    };
                    let local_node_id = self.local_node_id();
                    self.apply_sync_record(&vault_rec, &local_node_id)?
                }
            }
            "keep_local" | "dismiss" => false,
            _ => {
                return Err(format!(
                    "Unknown conflict resolution strategy: {}",
                    strategy
                ))
            }
        };

        self.delete_sync_conflict(conflict_id)?;
        Ok(apply_remote)
    }

    /// 根据表名和记录 ID 硬删除本地记录（冲突解决 keep_remote 且远程为 tombstone 时使用）。
    fn hard_delete_record(&self, table: &str, record_id: &str) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        match table {
            "profiles" => {
                conn.execute("DELETE FROM profiles WHERE id = ?1", params![record_id])
                    .map_err(|e| e.to_string())?;
            }
            "objects" => {
                conn.execute("DELETE FROM objects WHERE id = ?1", params![record_id])
                    .map_err(|e| e.to_string())?;
            }
            "user_templates" => {
                conn.execute(
                    "DELETE FROM user_templates WHERE id = ?1",
                    params![record_id],
                )
                .map_err(|e| e.to_string())?;
            }
            "trash_items" => {
                conn.execute("DELETE FROM trash_items WHERE id = ?1", params![record_id])
                    .map_err(|e| e.to_string())?;
            }
            "llm_conversations" => {
                Self::delete_conversation_tx(conn, record_id)?;
            }
            _ => return Err(format!("Unsupported sync table: {}", table)),
        }
        drop(guard);
        self.record_tombstone(table, record_id)?;
        self.set_record_hlc(table, record_id, &self.new_tombstone_hlc()?)?;
        Ok(())
    }

    /// P115: 事务内应用单条 Profile 同步记录（连接由调用方持有）。
    fn apply_profile_sync_record_tx(
        conn: &mut Connection,
        key: &DataEncryptionKey,
        record: &crate::BorrowedSyncRecord,
        ui_prefs_enabled: bool,
    ) -> Result<bool, String> {
        if record.deleted {
            // Apply remote tombstone directly without creating a local tombstone,
            // so the remote HLC remains the authoritative deletion timestamp.
            conn.execute("DELETE FROM profiles WHERE id = ?1", params![record.id])
                .map_err(|e| format!("delete profile: {}", e))?;
            return Ok(true);
        }
        let data_b64 = record
            .data
            .get("data")
            .and_then(|v| v.as_str())
            .ok_or("Missing profile data")?;
        let mut data = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data_b64)
            .map_err(|e| format!("profile data decode: {}", e))?;
        // 设备关闭「同步设置偏好」：外观 UI 偏好各设备独立——剥掉对端发来的
        // UI 外观键，并把本地已有值合并回来（不被对端覆盖）；AI 对话、回收站
        // 保留期等其余账户级键照常接收。合并失败保持原样应用（不阻断同步）。
        if !ui_prefs_enabled {
            Self::merge_local_ui_prefs(conn, key, record.id, &mut data)?;
        }
        let now = Self::now_rfc3339();
        let created = record
            .data
            .get("createdAt")
            .and_then(|v| v.as_str())
            .unwrap_or(&now);
        let updated = record
            .data
            .get("updatedAt")
            .and_then(|v| v.as_str())
            .unwrap_or(&now);
        let profile = crate::Profile {
            id: record.id.to_string(),
            name: record
                .data
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            data,
            created_at: chrono::DateTime::parse_from_rfc3339(created)
                .map(|d| d.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: chrono::DateTime::parse_from_rfc3339(updated)
                .map(|d| d.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            version: record
                .data
                .get("version")
                .and_then(|v| v.as_u64())
                .unwrap_or(1) as u32,
        };
        Self::save_profile_tx(conn, key, &profile)?;
        Ok(true)
    }

    /// 接收侧偏好合并：incoming profile data 剥掉 UI 外观键，并回填本地 UI 外观键。
    ///
    /// 设备关闭「同步设置偏好」时调用：外观 UI 偏好（主题/主题色/背景/语言/侧边栏等）
    /// 各设备独立——对端发来的值丢弃（不被覆盖），本地已有值保留；其余账户级键
    /// （AI 对话、回收站保留期、自动锁定等）照常接受。
    fn merge_local_ui_prefs(
        conn: &Connection,
        key: &DataEncryptionKey,
        id: &str,
        data: &mut Vec<u8>,
    ) -> Result<(), String> {
        // 明文 data 解析失败时保持原样（不阻断同步，防御性降级）。
        let Ok(mut incoming) = serde_json::from_slice::<serde_json::Value>(data) else {
            return Ok(());
        };
        // 本地 UI 外观键（解密本地 profile 取 preferences）。无记录/解析失败视为空。
        let local_prefs = match Self::load_profile_data_tx(conn, key, id)? {
            Some(raw) => serde_json::from_slice::<serde_json::Value>(&raw)
                .ok()
                .and_then(|v| v.get("preferences").cloned())
                .and_then(|p| p.as_object().cloned())
                .unwrap_or_default(),
            None => serde_json::Map::new(),
        };
        let ui_keys = super::UI_PREF_SYNC_EXCLUDED_KEYS;
        // incoming 非对象（异常数据）时不做合并。
        let Some(obj) = incoming.as_object_mut() else {
            return Ok(());
        };
        // incoming 无 preferences 但本地有 UI 键：创建空对象用于回填（本地偏好保持独立）。
        if obj.get("preferences").is_none() && !local_prefs.is_empty() {
            obj.insert(
                "preferences".to_string(),
                serde_json::Value::Object(serde_json::Map::new()),
            );
        }
        if let Some(prefs) = obj.get_mut("preferences").and_then(|p| p.as_object_mut()) {
            for k in ui_keys {
                prefs.remove(*k);
            }
            for (k, v) in local_prefs {
                if ui_keys.contains(&k.as_str()) {
                    prefs.insert(k, v);
                }
            }
        }
        if let Ok(re) = serde_json::to_vec(&incoming) {
            *data = re;
        }
        Ok(())
    }

    /// 读取本地 Profile 明文 data（解密）。无记录返回 None。供接收侧偏好合并使用。
    fn load_profile_data_tx(
        conn: &Connection,
        key: &DataEncryptionKey,
        id: &str,
    ) -> Result<Option<Vec<u8>>, String> {
        let raw: Option<Vec<u8>> = conn
            .query_row(
                "SELECT data FROM profiles WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| format!("load profile data for sync: {e}"))?;
        match raw {
            Some(bytes) => decrypt_field(key, &bytes)
                .map(Some)
                .map_err(|e| format!("decrypt profile data for sync: {e}")),
            None => Ok(None),
        }
    }

    /// P115: 事务内应用单条对象同步记录（连接由调用方持有）。
    fn apply_object_sync_record_tx(
        conn: &mut Connection,
        key: &DataEncryptionKey,
        record: &crate::BorrowedSyncRecord,
        local_node_id: &str,
    ) -> Result<bool, String> {
        // #1（§4.5）：墓碑记录（data 为 null，deleted=true）→ 删除本地对象行。
        // 不重记本地墓碑（对端应用路径不产生新墓碑），远程 HLC 保持权威删除时间戳。
        if record.deleted && record.data.is_null() {
            conn.execute("DELETE FROM objects WHERE id = ?1", params![record.id])
                .map_err(|e| format!("delete object (tombstone): {}", e))?;
            return Ok(true);
        }
        let mut obj: crate::ObjectRecord = crate::ObjectRecord::deserialize(record.data)
            .map_err(|e| format!("object decode: {}", e))?;
        // Bump version if the local node is modifying an existing object.
        if Self::load_object_tx(conn, key, &obj.id)?.is_some() {
            obj.version += 1;
            obj.updated_at = Self::now_rfc3339();
        }
        // Re-encrypt properties locally. 同步应用路径逐条处理（P024 模板名
        // 查询保留原行为），批量导入路径才走预加载 map。
        Self::save_object_tx(conn, key, &obj, None)?;
        let _ = local_node_id;
        Ok(true)
    }

    /// P115: 事务内应用单条模板同步记录（连接由调用方持有）。
    fn apply_user_template_sync_record_tx(
        conn: &mut Connection,
        key: &DataEncryptionKey,
        record: &crate::BorrowedSyncRecord,
    ) -> Result<bool, String> {
        if record.deleted {
            let _ = Self::load_user_template_tx(conn, key, record.id); // ensure vault is accessible
            conn.execute(
                "DELETE FROM user_templates WHERE id = ?1",
                params![record.id],
            )
            .map_err(|e| format!("delete template: {}", e))?;
            return Ok(true);
        }
        let tpl: crate::UserTemplate = crate::UserTemplate::deserialize(record.data)
            .map_err(|e| format!("template decode: {}", e))?;
        Self::save_user_template_tx(conn, key, &tpl)?;
        Ok(true)
    }

    /// P115: 事务内应用单条回收站同步记录（连接由调用方持有）。
    fn apply_trash_sync_record_tx(
        conn: &mut Connection,
        key: &DataEncryptionKey,
        record: &crate::BorrowedSyncRecord,
    ) -> Result<bool, String> {
        // #1（§4.5）：墓碑记录（data 为 null，deleted=true）→ 删除本地回收站条目。
        // 不重记本地墓碑，远程 HLC 保持权威删除时间戳。
        if record.deleted && record.data.is_null() {
            conn.execute("DELETE FROM trash_items WHERE id = ?1", params![record.id])
                .map_err(|e| format!("delete trash item (tombstone): {}", e))?;
            return Ok(true);
        }
        let item: crate::TrashItem = crate::TrashItem::deserialize(record.data)
            .map_err(|e| format!("trash decode: {}", e))?;
        Self::save_trash_item_tx(conn, key, &item)?;
        Ok(true)
    }
}
