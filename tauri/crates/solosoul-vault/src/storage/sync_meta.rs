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

use super::{with_tx, VaultStore, HLC_GET_SQL, HLC_SET_SQL};

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

    /// P011: 批量取回多条记录的 HLC——单次 `IN` 查询替代逐行 `get_record_hlc`
    /// （每个热路径变更清单函数原本每行一次 SELECT + 锁获取）。
    /// 返回 `HashMap<record_id, RecordHlc>`；无 HLC 行的记录不在 map 中，
    /// 调用方按 `updated_at` 构造 fallback（等价原 `record_hlc_or_fallback` 语义，
    /// 该方法已被本批量路径取代删除）。
    pub(crate) fn get_record_hlcs_batch(
        &self,
        table: &str,
        record_ids: &[String],
    ) -> Result<std::collections::HashMap<String, crate::RecordHlc>, String> {
        let mut out = std::collections::HashMap::with_capacity(record_ids.len());
        if record_ids.is_empty() {
            return Ok(out);
        }
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        // 占位符构造：?, ?, ...（id 数）——ids 是内部数据库主键（UUID），
        // 非用户输入，无注入面。
        let placeholders = vec!["?"; record_ids.len()].join(", ");
        let sql = format!(
            "SELECT record_id, wall_time_ms, counter, node_id FROM sync_hlc \
             WHERE table_name = ?1 AND record_id IN ({placeholders})"
        );
        let mut stmt = conn
            .prepare_cached(&sql)
            .map_err(|e| format!("get_record_hlcs_batch prepare: {e}"))?;
        let mut params_vec: Vec<&dyn rusqlite::ToSql> = vec![&table];
        for id in record_ids {
            params_vec.push(id);
        }
        let mut rows = stmt
            .query(rusqlite::params_from_iter(params_vec))
            .map_err(|e| format!("get_record_hlcs_batch query: {e}"))?;
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("get_record_hlcs_batch next: {e}"))?
        {
            out.insert(
                row.get::<_, String>(0)
                    .map_err(|e| format!("get_record_hlcs_batch id: {e}"))?,
                crate::RecordHlc {
                    wall_time_ms: row
                        .get::<_, i64>(1)
                        .map_err(|e| format!("get_record_hlcs_batch wall: {e}"))?
                        as u64,
                    // HLC_SET_SQL 以 i32 落库，读回转回 u32（同 get_record_hlc_tx 语义）。
                    counter: row
                        .get::<_, i32>(2)
                        .map_err(|e| format!("get_record_hlcs_batch counter: {e}"))?
                        as u32,
                    node_id: row
                        .get::<_, String>(3)
                        .map_err(|e| format!("get_record_hlcs_batch node: {e}"))?,
                },
            );
        }
        Ok(out)
    }

    /// P011: 批量 fallback——与 `record_hlc_or_fallback` 语义等价：
    /// 已有 HLC 用库值，否则按 `updated_at` 构造零计数 fallback。
    pub(crate) fn resolve_hlc_or_fallback_batch(
        &self,
        table: &str,
        records: &[(String, String)],
        local_node_id: &str,
    ) -> Result<std::collections::HashMap<String, crate::RecordHlc>, String> {
        let ids: Vec<String> = records.iter().map(|(id, _)| id.clone()).collect();
        let mut map = self.get_record_hlcs_batch(table, &ids)?;
        for (id, updated_at) in records {
            map.entry(id.clone()).or_insert_with(|| crate::RecordHlc {
                wall_time_ms: Self::parse_time_ms(updated_at),
                counter: 0,
                node_id: local_node_id.to_string(),
            });
        }
        Ok(map)
    }

    pub fn save_peer_state(&self, peer: &crate::PeerSyncState) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        conn.execute(
            "INSERT INTO sync_peers (peer_node_id, peer_name, trusted, public_key_fingerprint, last_seen, client_type, trusted_at, last_addr, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(peer_node_id) DO UPDATE SET
                peer_name = excluded.peer_name,
                trusted = excluded.trusted,
                public_key_fingerprint = excluded.public_key_fingerprint,
                last_seen = excluded.last_seen,
                client_type = excluded.client_type,
                trusted_at = excluded.trusted_at,
                last_addr = excluded.last_addr,
                updated_at = excluded.updated_at",
            params![
                &peer.peer_node_id,
                &peer.peer_name,
                peer.trusted as i32,
                &peer.public_key_fingerprint,
                peer.last_seen,
                &peer.client_type,
                peer.trusted_at,
                &peer.last_addr,
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
                "SELECT peer_node_id, peer_name, trusted, public_key_fingerprint, last_seen, client_type, trusted_at, last_addr, created_at, updated_at
                 FROM sync_peers WHERE peer_node_id = ?1",
                params![peer_node_id],
                |row| {
                    Ok(crate::PeerSyncState {
                        peer_node_id: row.get(0)?,
                        peer_name: row.get(1)?,
                        trusted: row.get::<_, i32>(2)? != 0,
                        public_key_fingerprint: row.get(3)?,
                        last_seen: row.get(4)?,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                        client_type: row.get(5)?,
                        trusted_at: row.get(6)?,
                        last_addr: row.get(7)?,
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
                "SELECT peer_node_id, peer_name, trusted, public_key_fingerprint, last_seen, client_type, trusted_at, last_addr, created_at, updated_at
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
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                    client_type: row.get(5)?,
                    trusted_at: row.get(6)?,
                    last_addr: row.get(7)?,
                })
            })
            .map_err(|e| format!("list_peers query: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("list_peers collect: {}", e))?;
        Ok(peers)
    }

    /// 更新 peer 信任状态。信任时记录 trusted_at（最近信任时间），撤销时清空。
    pub fn set_peer_trusted(&self, peer_node_id: &str, trusted: bool) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        conn.execute(
            "UPDATE sync_peers SET trusted = ?1, trusted_at = ?2, updated_at = ?3 WHERE peer_node_id = ?4",
            params![
                trusted as i32,
                if trusted {
                    Some(chrono::Utc::now().timestamp())
                } else {
                    None::<i64>
                },
                Self::now_rfc3339(),
                peer_node_id
            ],
        )
        .map_err(|e| format!("set_peer_trusted: {}", e))?;
        Ok(())
    }

    pub fn delete_peer(&self, peer_node_id: &str) -> Result<(), String> {
        // #1 墓碑清理前置（§4.5.1）：联动删除该 peer 的 sync_watermarks 水位行。
        // 否则被忘记/删除设备的水位残留会永远「保住」其名下墓碑（清理逻辑按
        // 存续 peer 水位判定可删性），导致 sync_tombstones 清理永久失效。
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        with_tx(conn, "delete_peer begin", "delete_peer commit", |c| {
            c.execute(
                "DELETE FROM sync_peers WHERE peer_node_id = ?1",
                params![peer_node_id],
            )
            .map_err(|e| format!("delete_peer: {}", e))?;
            c.execute(
                "DELETE FROM sync_watermarks WHERE peer_node_id = ?1",
                params![peer_node_id],
            )
            .map_err(|e| format!("delete_peer watermarks: {}", e))?;
            Ok(())
        })
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
    /// 与 session.rs 节点规范化逐字节一致（migration.rs 复用此 pub(crate) 版本）。
    pub(crate) fn normalize_sync_node_id(node_id: &str) -> String {
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

    /// §4.5.1 方案 C：清理可安全删除的墓碑（单事务，返回删除数量）。
    ///
    /// **安全条件（水位老化）**：对每个墓碑，所有「仍存续且已同步过该表」的 peer
    /// 水位都 ≥ 墓碑 HLC（协议按水位线性推进，达到即收到并应用）——等价于该表
    /// 存续 peer 水位**最小值 ≥ 墓碑 HLC**。满足则墓碑不再被任何 peer 需要，可删。
    /// 新 peer（无该表水位行）从零全量同步不需要墓碑（对象行已不存在，发不发结果
    /// 一致），不计入约束。
    ///
    /// **时间兜底（仅纯单机/未配对）**：该表无任何存续 peer 水位行时，按
    /// `created_at` 老化（默认 365 天）删除——纯单机无对端，删除不会导致数据回魂。
    /// 注意：只要存在任一存续 peer 水位行，就走水位判定（严格正确），时间兜底
    /// 绝不越权，离线 peer 的墓碑被正确保留。
    ///
    /// **与 sync_hlc 解耦**：删除墓碑不联动删 sync_hlc 对应行（重建记录覆盖或成
    /// 无害孤儿，评估结论暂不联动）。
    pub fn cleanup_expired_tombstones(&self) -> Result<usize, String> {
        let now_ms = Self::parse_time_ms(&Self::now_rfc3339());
        // 365 天兜底阈值（纯单机/未配对墓碑）
        let time_cutoff_ms = now_ms.saturating_sub(365 * 24 * 60 * 60 * 1000);
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let removed = with_tx(
            conn,
            "cleanup tombstones begin",
            "cleanup tombstones commit",
            |c| {
                // 1) 存续 peer 水位（JOIN sync_peers 存活过滤——delete_peer 已联动删
                //    水位，此处 JOIN 双保险，防历史残留）。
                let peer_watermarks: Vec<(String, String, i64, i32, String)> = {
                    let mut stmt = c
                        .prepare(
                            "SELECT w.peer_node_id, w.table_name, w.wall_time_ms, w.counter, w.node_id
                             FROM sync_watermarks w
                             JOIN sync_peers p ON w.peer_node_id = p.peer_node_id",
                        )
                        .map_err(|e| format!("cleanup tombstones peer query: {}", e))?;
                    let rows = stmt
                        .query_map([], |row| {
                            Ok((
                                row.get(0)?,
                                row.get(1)?,
                                row.get(2)?,
                                row.get(3)?,
                                row.get(4)?,
                            ))
                        })
                        .map_err(|e| format!("cleanup tombstones peer rows: {}", e))?
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| format!("cleanup tombstones peer collect: {}", e))?;
                    rows
                };
                // 2) 全部墓碑
                let tombstones: Vec<(String, String, i64, i32, String, String)> = {
                    let mut stmt = c
                        .prepare(
                            "SELECT table_name, record_id, wall_time_ms, counter, node_id, created_at
                             FROM sync_tombstones",
                        )
                        .map_err(|e| format!("cleanup tombstones query: {}", e))?;
                    let rows = stmt
                        .query_map([], |row| {
                            Ok((
                                row.get(0)?,
                                row.get(1)?,
                                row.get(2)?,
                                row.get(3)?,
                                row.get(4)?,
                                row.get(5)?,
                            ))
                        })
                        .map_err(|e| format!("cleanup tombstones rows: {}", e))?
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| format!("cleanup tombstones collect: {}", e))?;
                    rows
                };
                let mut removed = 0usize;
                for (table, record_id, wall, counter, node, created_at) in tombstones {
                    let tombstone_hlc = crate::RecordHlc {
                        wall_time_ms: wall as u64,
                        counter: counter as u32,
                        node_id: node,
                    };
                    let table_wms: Vec<&(String, String, i64, i32, String)> =
                        peer_watermarks.iter().filter(|w| w.1 == table).collect();
                    let safe = if table_wms.is_empty() {
                        // 纯单机/未配对：时间兜底
                        Self::parse_time_ms(&created_at) <= time_cutoff_ms
                    } else {
                        // 水位老化：所有 peer 水位 ≥ 墓碑 HLC ⇔ 该表水位最小值 ≥ 墓碑 HLC。
                        // P019: 手写三元组 min 收敛为 Ord 派生 + Iterator::min（语义逐位一致）。
                        let min_wm: Option<crate::SyncWatermark> = table_wms
                            .iter()
                            .map(|w| crate::SyncWatermark {
                                wall_time_ms: w.2 as u64,
                                counter: w.3 as u32,
                                node_id: w.4.clone(),
                            })
                            .min();
                        match min_wm {
                            // 墓碑不严格大于水位最小值 ⇔ 所有 peer 水位 ≥ 墓碑 HLC
                            Some(wm) => !Self::hlc_after_watermark(&tombstone_hlc, &wm),
                            None => false,
                        }
                    };
                    if safe {
                        c.execute(
                            "DELETE FROM sync_tombstones WHERE table_name = ?1 AND record_id = ?2",
                            params![table, record_id],
                        )
                        .map_err(|e| format!("cleanup tombstone delete: {}", e))?;
                        removed += 1;
                    }
                }
                Ok(removed)
            },
        )?;
        Ok(removed)
    }

    pub(crate) fn hlc_after_watermark(
        hlc: &crate::RecordHlc,
        watermark: &crate::SyncWatermark,
    ) -> bool {
        // P019: 两类型字段同构（wall/counter/node 字典序），转 SyncWatermark 后借 Ord
        // 严格大于比较，语义与手写三元组逐位一致。
        let hlc_wm = crate::SyncWatermark {
            wall_time_ms: hlc.wall_time_ms,
            counter: hlc.counter,
            node_id: hlc.node_id.clone(),
        };
        hlc_wm > *watermark
    }
}
