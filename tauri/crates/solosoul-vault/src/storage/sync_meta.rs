//! Sync 元数据域 —— 自 `storage.rs` 拆分（P223-② 表域拆分）。
//!
//! 本模块承载 `VaultStore` 的 HLC 读写 / Peer 状态 / 水印 / 墓碑方法
//! （get_record_hlc / save_peer_state / update_peer_watermark(_with_cursor) /
//! record_tombstone / list_tombstones_since 等 22 个方法，原 storage.rs
//! 1068-1486 行，逐行搬运零行为变更）。
//!
//! 共享设施经 `super::` 访问父模块私有项：HLC 读写常量 SQL（HLC_GET_SQL / HLC_SET_SQL）。
//! 12 个跨域私有助手（now_rfc3339 / parse_time_ms / get|set_record_hlc(_tx) /
//! record_hlc_or_fallback / local_node_id / new_tombstone|local_hlc /
//! record_tombstone / list_tombstones_since / hlc_after_watermark）以 `pub(crate)`
//! 暴露——根模块同步变更清单（list_sync_changes_since* / 冲突解决 / delete_profile /
//! 模板删除）跨域复用；`max_hlc_wall_time_for_node` 仅域内调用保持私有。

use rusqlite::{params, Connection, OptionalExtension};

use super::{VaultStore, HLC_GET_SQL, HLC_SET_SQL};

impl VaultStore {
    // ── Sync state helpers ──────────────────────────────────

    pub(crate) fn now_rfc3339() -> String {
        chrono::Utc::now().to_rfc3339()
    }

    pub(crate) fn parse_time_ms(s: &str) -> u64 {
        chrono::DateTime::parse_from_rfc3339(s)
            .map(|d| d.timestamp_millis() as u64)
            .unwrap_or(0)
    }

    pub fn get_record_hlc(
        &self,
        table: &str,
        record_id: &str,
    ) -> Result<Option<crate::RecordHlc>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        Self::get_record_hlc_tx(conn, table, record_id)
    }

    /// P115: 事务内 HLC 查询（连接由调用方持有，批量应用单事务内复用）。
    pub(crate) fn get_record_hlc_tx(
        conn: &mut Connection,
        table: &str,
        record_id: &str,
    ) -> Result<Option<crate::RecordHlc>, String> {
        let result = conn
            .prepare_cached(HLC_GET_SQL)
            .map_err(|e| format!("get_record_hlc prepare: {}", e))?
            .query_row(params![table, record_id], |row| {
                Ok(crate::RecordHlc {
                    wall_time_ms: row.get(0)?,
                    counter: row.get::<_, i32>(1)? as u32,
                    node_id: row.get(2)?,
                })
            })
            .optional()
            .map_err(|e| format!("get_record_hlc: {}", e))?;
        Ok(result)
    }

    pub(crate) fn set_record_hlc(
        &self,
        table: &str,
        record_id: &str,
        hlc: &crate::RecordHlc,
    ) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        Self::set_record_hlc_tx(conn, table, record_id, hlc)
    }

    /// P115: 事务内 HLC 写入（连接由调用方持有）。
    pub(crate) fn set_record_hlc_tx(
        conn: &mut Connection,
        table: &str,
        record_id: &str,
        hlc: &crate::RecordHlc,
    ) -> Result<(), String> {
        let mut stmt = conn
            .prepare_cached(HLC_SET_SQL)
            .map_err(|e| format!("set_record_hlc prepare: {}", e))?;
        stmt.execute(params![
            table,
            record_id,
            hlc.wall_time_ms,
            hlc.counter as i32,
            &hlc.node_id,
            Self::now_rfc3339(),
        ])
        .map_err(|e| format!("set_record_hlc: {}", e))?;
        Ok(())
    }

    pub(crate) fn record_hlc_or_fallback(
        &self,
        table: &str,
        record_id: &str,
        updated_at: &str,
        local_node_id: &str,
    ) -> Result<crate::RecordHlc, String> {
        if let Some(hlc) = self.get_record_hlc(table, record_id)? {
            return Ok(hlc);
        }
        Ok(crate::RecordHlc {
            wall_time_ms: Self::parse_time_ms(updated_at),
            counter: 0,
            node_id: local_node_id.to_string(),
        })
    }

    pub fn save_peer_state(&self, peer: &crate::PeerSyncState) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        conn.execute(
            "INSERT INTO sync_peers (peer_node_id, peer_name, trusted, public_key_fingerprint, last_seen, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(peer_node_id) DO UPDATE SET
                peer_name = excluded.peer_name,
                trusted = excluded.trusted,
                public_key_fingerprint = excluded.public_key_fingerprint,
                last_seen = excluded.last_seen,
                updated_at = excluded.updated_at",
            params![
                &peer.peer_node_id,
                &peer.peer_name,
                peer.trusted as i32,
                &peer.public_key_fingerprint,
                peer.last_seen,
                &peer.created_at,
                &peer.updated_at,
            ],
        )
        .map_err(|e| format!("save_peer_state: {}", e))?;
        Ok(())
    }

    pub fn load_peer_state(
        &self,
        peer_node_id: &str,
    ) -> Result<Option<crate::PeerSyncState>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let result = conn
            .query_row(
                "SELECT peer_node_id, peer_name, trusted, public_key_fingerprint, last_seen, created_at, updated_at
                 FROM sync_peers WHERE peer_node_id = ?1",
                params![peer_node_id],
                |row| {
                    Ok(crate::PeerSyncState {
                        peer_node_id: row.get(0)?,
                        peer_name: row.get(1)?,
                        trusted: row.get::<_, i32>(2)? != 0,
                        public_key_fingerprint: row.get(3)?,
                        last_seen: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    })
                },
            )
            .optional()
            .map_err(|e| format!("load_peer_state: {}", e))?;
        Ok(result)
    }

    pub fn list_peers(&self) -> Result<Vec<crate::PeerSyncState>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn
            .prepare(
                "SELECT peer_node_id, peer_name, trusted, public_key_fingerprint, last_seen, created_at, updated_at
                 FROM sync_peers ORDER BY updated_at DESC",
            )
            .map_err(|e| format!("list_peers: {}", e))?;
        let peers = stmt
            .query_map([], |row| {
                Ok(crate::PeerSyncState {
                    peer_node_id: row.get(0)?,
                    peer_name: row.get(1)?,
                    trusted: row.get::<_, i32>(2)? != 0,
                    public_key_fingerprint: row.get(3)?,
                    last_seen: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|e| format!("list_peers query: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("list_peers collect: {}", e))?;
        Ok(peers)
    }

    pub fn set_peer_trusted(&self, peer_node_id: &str, trusted: bool) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        conn.execute(
            "UPDATE sync_peers SET trusted = ?1, updated_at = ?2 WHERE peer_node_id = ?3",
            params![trusted as i32, Self::now_rfc3339(), peer_node_id],
        )
        .map_err(|e| format!("set_peer_trusted: {}", e))?;
        Ok(())
    }

    pub fn delete_peer(&self, peer_node_id: &str) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        conn.execute(
            "DELETE FROM sync_peers WHERE peer_node_id = ?1",
            params![peer_node_id],
        )
        .map_err(|e| format!("delete_peer: {}", e))?;
        Ok(())
    }

    pub fn update_peer_watermark(
        &self,
        peer_node_id: &str,
        table: &str,
        watermark: &crate::SyncWatermark,
    ) -> Result<(), String> {
        // R-3: 无游标语义 = 仅更新水印（清空游标）。
        self.update_peer_watermark_with_cursor(peer_node_id, table, watermark, None)
    }

    /// R-3: 水印与页游标同事务/同刻落库——会话中断后从持久化水印旁恢复游标，
    /// 等值 HLC 组跨会话续传（`cursor=None` 即清空游标，用于表同步完成时
    /// 保持严格 `>` 语义，防陈旧游标跳过未来同 ms 行）。
    pub fn update_peer_watermark_with_cursor(
        &self,
        peer_node_id: &str,
        table: &str,
        watermark: &crate::SyncWatermark,
        cursor: Option<&str>,
    ) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        conn.execute(
            "INSERT INTO sync_watermarks (peer_node_id, table_name, wall_time_ms, counter, node_id, cursor_id, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(peer_node_id, table_name) DO UPDATE SET
                wall_time_ms = excluded.wall_time_ms,
                counter = excluded.counter,
                node_id = excluded.node_id,
                cursor_id = excluded.cursor_id,
                updated_at = excluded.updated_at",
            params![
                peer_node_id,
                table,
                watermark.wall_time_ms,
                watermark.counter as i32,
                &watermark.node_id,
                cursor,
                Self::now_rfc3339(),
            ],
        )
        .map_err(|e| format!("update_peer_watermark: {}", e))?;
        Ok(())
    }

    /// R-3: 读取持久化页游标（无记录返回 None）。
    pub fn get_peer_watermark_cursor(
        &self,
        peer_node_id: &str,
        table: &str,
    ) -> Result<Option<String>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        conn.query_row(
            "SELECT cursor_id FROM sync_watermarks WHERE peer_node_id = ?1 AND table_name = ?2",
            params![peer_node_id, table],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map(|opt| opt.flatten())
        .map_err(|e| format!("get_peer_watermark_cursor: {}", e))
    }

    pub fn get_peer_watermark(
        &self,
        peer_node_id: &str,
        table: &str,
    ) -> Result<crate::SyncWatermark, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let result = conn
            .query_row(
                "SELECT wall_time_ms, counter, node_id FROM sync_watermarks
                 WHERE peer_node_id = ?1 AND table_name = ?2",
                params![peer_node_id, table],
                |row| {
                    Ok(crate::SyncWatermark {
                        wall_time_ms: row.get(0)?,
                        counter: row.get::<_, i32>(1)? as u32,
                        node_id: row.get(2)?,
                    })
                },
            )
            .optional()
            .map_err(|e| format!("get_peer_watermark: {}", e))?;
        Ok(result.unwrap_or_default())
    }

    pub(crate) fn local_node_id(&self) -> String {
        self.get_sync_node_id()
            .ok()
            .flatten()
            .unwrap_or_else(|| "unknown".to_string())
    }

    pub(crate) fn new_tombstone_hlc(&self) -> Result<crate::RecordHlc, String> {
        let node_id = Self::normalize_sync_node_id(&self.local_node_id());
        let now = Self::parse_time_ms(&Self::now_rfc3339());
        // Bump wall_time to be strictly larger than any existing HLC for this node
        // to guarantee the tombstone wins over prior local changes.
        let max_existing = self.max_hlc_wall_time_for_node(&node_id)?;
        Ok(crate::RecordHlc {
            wall_time_ms: now.max(max_existing + 1),
            counter: 0,
            node_id,
        })
    }

    /// 生成一个属于本地节点的新 HLC，用于覆盖本地版本（冲突解决时）。
    /// 与 sync 层 session.rs 的节点规范化逐字节一致（hex 编码的 16 字节节点）：
    /// 32 字符按 hex 解码，其余取前 16 字节补零。本地写落库 HLC 的 node 必须与
    /// 对端水印落库格式（watermark_to_vault 的 hex 形式）一致，否则 keyset 等值组
    /// 判定 (node == 水印 node) 永不成立，同 wall 行经 strict `>` 反复通过、id 游标
    /// 不推进，分页死循环（方案 B 阶段 1 在 sync 测试实测：`test_generate_delta_
    /// paginated_keyset_production_encoding` 挂起）。
    fn normalize_sync_node_id(node_id: &str) -> String {
        let bytes = if node_id.len() == 32 {
            hex::decode(node_id).unwrap_or_else(|_| Vec::new())
        } else {
            let src = node_id.as_bytes();
            src[..src.len().min(16)].to_vec()
        };
        let mut out = [0u8; 16];
        let len = bytes.len().min(16);
        out[..len].copy_from_slice(&bytes[..len]);
        hex::encode(out)
    }

    pub(crate) fn new_local_hlc(&self) -> Result<crate::RecordHlc, String> {
        let node_id = Self::normalize_sync_node_id(&self.local_node_id());
        let now = Self::parse_time_ms(&Self::now_rfc3339());
        let max_existing = self.max_hlc_wall_time_for_node(&node_id)?;
        Ok(crate::RecordHlc {
            wall_time_ms: now.max(max_existing + 1),
            counter: 0,
            node_id,
        })
    }

    fn max_hlc_wall_time_for_node(&self, node_id: &str) -> Result<u64, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let max: Option<i64> = {
            let max_result = conn
                .query_row(
                    "SELECT COALESCE(MAX(wall_time_ms), 0) FROM sync_hlc WHERE node_id = ?1",
                    params![node_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| format!("max_hlc: {}", e))?;
            max_result
        };
        Ok(max.unwrap_or(0) as u64)
    }

    /// Record a deletion tombstone for a record that is being hard-deleted.
    /// Also updates sync_hlc so the tombstone participates in conflict resolution.
    pub(crate) fn record_tombstone(&self, table: &str, record_id: &str) -> Result<(), String> {
        let hlc = self.new_tombstone_hlc()?;
        let deleted_by = self.local_node_id();
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        conn.execute(
            "INSERT INTO sync_tombstones (table_name, record_id, wall_time_ms, counter, node_id, deleted_by_node_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(table_name, record_id) DO UPDATE SET
                wall_time_ms = excluded.wall_time_ms,
                counter = excluded.counter,
                node_id = excluded.node_id,
                deleted_by_node_id = excluded.deleted_by_node_id,
                created_at = excluded.created_at",
            params![
                table,
                record_id,
                hlc.wall_time_ms,
                hlc.counter as i32,
                &hlc.node_id,
                &deleted_by,
                Self::now_rfc3339(),
            ],
        )
        .map_err(|e| format!("record_tombstone: {}", e))?;
        drop(guard);
        self.set_record_hlc(table, record_id, &hlc)
    }

    pub(crate) fn list_tombstones_since(
        &self,
        table: &str,
        watermark: &crate::SyncWatermark,
        local_node_id: &str,
    ) -> Result<Vec<crate::VaultSyncRecord>, String> {
        let rows = {
            let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
            let conn = guard.as_mut().ok_or("Vault is locked")?;
            let mut stmt = conn
                .prepare(
                    "SELECT table_name, record_id, wall_time_ms, counter, node_id
                     FROM sync_tombstones WHERE table_name = ?1",
                )
                .map_err(|e| format!("list_tombstones: {}", e))?;
            let rows: Vec<crate::VaultSyncRecord> = stmt
                .query_map(params![table], |row| {
                    Ok(crate::VaultSyncRecord {
                        id: row.get(1)?,
                        table: row.get(0)?,
                        data: serde_json::Value::Null,
                        hlc: crate::RecordHlc {
                            wall_time_ms: row.get(2)?,
                            counter: row.get::<_, i32>(3)? as u32,
                            node_id: row.get(4)?,
                        },
                        deleted: true,
                    })
                })
                .map_err(|e| format!("list_tombstones query: {}", e))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("list_tombstones collect: {}", e))?;
            rows
        };

        // Filter by watermark and bump counter/wall_time for local node fallback.
        let mut out = Vec::new();
        for mut rec in rows {
            if rec.hlc.node_id == local_node_id && rec.hlc.counter == 0 {
                rec.hlc.counter = 0;
            }
            if Self::hlc_after_watermark(&rec.hlc, watermark) {
                out.push(rec);
            }
        }
        Ok(out)
    }

    pub(crate) fn hlc_after_watermark(
        hlc: &crate::RecordHlc,
        watermark: &crate::SyncWatermark,
    ) -> bool {
        hlc.wall_time_ms > watermark.wall_time_ms
            || (hlc.wall_time_ms == watermark.wall_time_ms
                && (hlc.counter > watermark.counter
                    || (hlc.counter == watermark.counter && hlc.node_id > watermark.node_id)))
    }
}
