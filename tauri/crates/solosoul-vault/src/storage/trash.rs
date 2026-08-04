//! Trash CRUD 域 —— 自 `storage.rs` 拆分（P223-② 表域拆分）。
//!
//! 本模块承载 `VaultStore` 的回收站读写方法（save_trash_item / trash_and_soft_delete_batch /
//! list_trash_items / get_trash_item / delete_trash_item / cleanup_expired_trash 等 7 个方法，
//! 原 storage.rs 2559-2819 行，逐行搬运零行为变更）。
//!
//! 共享设施经 `super::` 访问父模块私有项：`data_key()`、`with_tx` 与回收站 SQL 常量
//! （TRASH_SAVE_SQL / OBJECT_SOFT_DELETE_SQL）。`save_trash_item_tx` 以 `pub(crate)` 暴露——
//! 根模块同步应用路径（`apply_trash_sync_record_tx`）跨域复用，事务内语义不变。
//! `delete_object`（兄弟域 objects）与 `log_structured`（父模块）同为 storage 子树内
//! 可见方法，直接经 `self.` 调用。

use rusqlite::{params, Connection, OptionalExtension};

use super::{with_tx, VaultStore, OBJECT_SOFT_DELETE_SQL, TRASH_SAVE_SQL};
use crate::encryption::{decrypt_field, encrypt_field, DataEncryptionKey};
use crate::{TrashItem, TrashItemSummary};

impl VaultStore {
    // ── Trash CRUD (§23) ────────────────────────────────────

    pub fn save_trash_item(&self, item: &TrashItem) -> Result<(), String> {
        let key = self.data_key()?;
        // 方案 B（R-3 根治）：本地写统一生成并落库 HLC。save_trash_item_tx 被
        // sync_apply 远端应用路径复用（自写 HLC），故本地写 HLC 在入口事务内落。
        let hlc = self.new_local_hlc()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        with_tx(
            conn,
            "Failed to begin transaction",
            "Failed to commit transaction",
            |c| {
                Self::save_trash_item_tx(c, &key, item)?;
                Self::set_record_hlc_tx(c, "trash_items", &item.id, &hlc)?;
                Ok(())
            },
        )
    }

    /// P211: 单事务批量「入回收站 + 软删对象」。
    ///
    /// page_delete 等批量删除场景使用：替代「逐条 `save_trash_item` + 逐条
    /// `delete_object(soft)`」的 N×2 次 auto-commit 写事务（每次 save/delete 各自
    /// 获取连接锁并提交）。所有 `items` 插入与 `soft_delete_ids` 软删在同一事务内
    /// 完成——任一步失败整体回滚，回收站条目与对象软删不会产生半成品。
    /// 软删 `updated_at`/`deleted_at` 统一取一次时间戳，与 `delete_object(soft)`
    /// 语义一致（HLC 回退沿用 updated_at）。
    pub fn trash_and_soft_delete_batch(
        &self,
        items: &[TrashItem],
        soft_delete_ids: &[String],
    ) -> Result<(), String> {
        let key = self.data_key()?;
        // 方案 B（R-3 根治）：批内逐个生成 HLC——trash 条目一组 + 软删对象一组
        // （new_local_hlc 读 sync_hlc 最大值递增保证唯一，必须在持锁前调用）。
        // 软删对象必须落新 HLC：阶段 1 后对象已有 save 时 HLC，不更新则对端
        // 永远不会看到 is_deleted=1（对象行保留，updated_at 回退路径不再生效）。
        let item_hlcs = items
            .iter()
            .map(|_| self.new_local_hlc())
            .collect::<Result<Vec<_>, _>>()?;
        let obj_hlcs = soft_delete_ids
            .iter()
            .map(|_| self.new_local_hlc())
            .collect::<Result<Vec<_>, _>>()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        // P213: 手动事务（Transaction 无 DerefMut，prepare_cached 需要 &mut Connection）。
        with_tx(
            conn,
            "Failed to begin transaction",
            "Failed to commit transaction",
            |c| {
                for (item, hlc) in items.iter().zip(item_hlcs.iter()) {
                    Self::save_trash_item_tx(c, &key, item)?;
                    Self::set_record_hlc_tx(c, "trash_items", &item.id, hlc)?;
                }
                if !soft_delete_ids.is_empty() {
                    let now = chrono::Utc::now().to_rfc3339();
                    // 先执行全部软删（stmt 持 c 借用，作用域块内完成即释放），
                    // 再统一落 HLC——同事务内原子，顺序不影响结果。
                    {
                        let mut stmt = c
                            .prepare_cached(OBJECT_SOFT_DELETE_SQL)
                            .map_err(|e| format!("Failed to prepare soft delete: {e}"))?;
                        for id in soft_delete_ids {
                            stmt.execute(params![&now, id])
                                .map_err(|e| format!("Failed to soft delete: {e}"))?;
                        }
                    }
                    for (id, hlc) in soft_delete_ids.iter().zip(obj_hlcs.iter()) {
                        Self::set_record_hlc_tx(c, "objects", id, hlc)?;
                    }
                }
                Ok(())
            },
        )
    }

    /// P115: 事务内保存回收站条目（连接由调用方持有，批量应用单事务内复用）。
    pub(crate) fn save_trash_item_tx(
        conn: &mut Connection,
        key: &DataEncryptionKey,
        item: &TrashItem,
    ) -> Result<(), String> {
        let encrypted_data = encrypt_field(key, &item.data)?;
        let mut stmt = conn
            .prepare_cached(TRASH_SAVE_SQL)
            .map_err(|e| format!("save_trash_item prepare: {}", e))?;
        stmt.execute(rusqlite::params![
            item.id,
            item.item_type,
            item.original_id,
            item.original_parent_id,
            item.original_section_type,
            item.original_sort_order,
            encrypted_data,
            item.deleted_at,
            item.expires_at,
            item.deleted_by,
            item.name_snapshot,
            item.icon_snapshot,
        ])
        .map_err(|e| format!("save_trash_item: {}", e))?;
        Ok(())
    }

    pub fn list_trash_items(
        &self,
        item_type: Option<&str>,
        since: Option<i64>,
    ) -> Result<Vec<TrashItemSummary>, String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut sql = String::from(
            "SELECT id, item_type, name_snapshot, icon_snapshot, deleted_at, expires_at, original_parent_id, original_section_type, original_id, data
             FROM trash_items WHERE 1=1"
        );
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        if let Some(t) = item_type {
            sql.push_str(" AND item_type = ?1");
            params.push(Box::new(t.to_string()));
        }
        if let Some(s) = since {
            sql.push_str(&format!(" AND deleted_at >= ?{}", params.len() + 1));
            params.push(Box::new(s));
        }
        sql.push_str(" ORDER BY deleted_at DESC LIMIT 500");
        let p: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|b| b.as_ref()).collect();
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let items = stmt
            .query_map(p.as_slice(), |row| {
                let raw_data: Vec<u8> = row.get(9)?;
                let decrypted = decrypt_field(&key, &raw_data).ok();
                let contract_type_id = decrypted
                    .and_then(|d| serde_json::from_slice::<serde_json::Value>(&d).ok())
                    .and_then(|v| {
                        v.get("contract_type_id")
                            .and_then(|c| c.as_str().map(String::from))
                    });
                Ok(TrashItemSummary {
                    id: row.get(0)?,
                    item_type: row.get(1)?,
                    original_id: row.get(8)?,
                    name: row.get(2)?,
                    icon_id: row.get(3)?,
                    deleted_at: row.get(4)?,
                    expires_at: row.get(5)?,
                    original_parent_id: row.get(6)?,
                    original_section_type: row.get(7)?,
                    contract_type_id,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(items)
    }

    pub fn get_trash_item(&self, id: &str) -> Result<Option<TrashItem>, String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn.prepare(
            "SELECT id, item_type, original_id, original_parent_id, original_section_type,
             original_sort_order, data, deleted_at, expires_at, deleted_by, name_snapshot, icon_snapshot
             FROM trash_items WHERE id = ?1"
        ).map_err(|e| e.to_string())?;
        let result = stmt
            .query_row(rusqlite::params![id], |row| {
                let raw_data: Vec<u8> = row.get(6)?;
                let data = decrypt_field(&key, &raw_data).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        6,
                        rusqlite::types::Type::Blob,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Trash data decryption failed: {}", e),
                        )),
                    )
                })?;
                Ok(TrashItem {
                    id: row.get(0)?,
                    item_type: row.get(1)?,
                    original_id: row.get(2)?,
                    original_parent_id: row.get(3)?,
                    original_section_type: row.get(4)?,
                    original_sort_order: row.get(5)?,
                    data,
                    deleted_at: row.get(7)?,
                    expires_at: row.get(8)?,
                    deleted_by: row.get(9)?,
                    name_snapshot: row.get(10)?,
                    icon_snapshot: row.get(11)?,
                })
            })
            .optional()
            .map_err(|e| format!("Failed to get trash item: {}", e))?;
        Ok(result)
    }

    pub fn delete_trash_item(&self, id: &str) -> Result<(), String> {
        // #1（§4.5）：回收站条目永久删除（purge）是墓碑变更——删行后记录墓碑，
        // 使对端回收站同步删除该条目（对端 apply 端按 data 为 null 识别删除）。
        // 行不存在（重复 purge/幂等清理）时不记墓碑，避免幽灵墓碑。
        // 已知取舍：DELETE 与 record_tombstone 非原子（与 delete_object 硬删、
        // delete_profile 既有模式一致）；两者间崩溃留下「行已删但无墓碑」。
        let affected = {
            let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
            let conn = guard.as_mut().ok_or("Vault is locked")?;
            conn.execute(
                "DELETE FROM trash_items WHERE id = ?1",
                rusqlite::params![id],
            )
            .map_err(|e| e.to_string())?
        };
        if affected > 0 {
            self.record_tombstone("trash_items", id)?;
        }
        Ok(())
    }

    /// 清理所有已过期的回收站项目。
    ///
    /// 逻辑：
    /// 1. 查询 `trash_items` 表中 `expires_at` 不为空且小于当前时间戳的项目。
    /// 2. 对于非 template 类型的项目，先删除对应原始对象（物理删除）。
    /// 3. 记录审计日志 `trash_permanent_delete`。
    /// 4. 从回收站表中删除该项目。
    ///
    /// 返回成功清理的项目数量。
    pub fn cleanup_expired_trash(&self) -> Result<usize, String> {
        let now_ms = chrono::Utc::now().timestamp_millis();

        // 先查询所有过期项目并释放连接锁，再逐个调用会重新加锁的删除/日志方法，
        // 避免在持有 Mutex 的同时再次加锁导致死锁。
        let expired: Vec<(String, String, String, String)> = {
            let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
            let conn = guard.as_mut().ok_or("Vault is locked")?;
            let mut stmt = conn
                .prepare(
                    "SELECT id, item_type, original_id, name_snapshot
                     FROM trash_items
                     WHERE expires_at IS NOT NULL AND expires_at < ?1",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(rusqlite::params![now_ms], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            rows
        };

        let mut cleaned = 0usize;
        for (trash_id, item_type, original_id, name_snapshot) in expired {
            // 先尝试物理删除原始对象；失败时保留回收站记录，避免产生孤儿对象。
            if item_type != "template" {
                if let Err(e) = self.delete_object(&original_id, false) {
                    tracing::warn!(
                        "[cleanup_expired_trash] skip trash_id={} because delete_object failed: {}",
                        trash_id,
                        e
                    );
                    continue;
                }
            }

            // 从回收站移除；成功后再记审计日志，避免审计与实际状态不一致。
            if let Err(e) = self.delete_trash_item(&trash_id) {
                tracing::warn!(
                    "[cleanup_expired_trash] delete_trash_item failed for trash_id={}: {}",
                    trash_id,
                    e
                );
                continue;
            }
            let _ = self.log_structured(
                "trash_permanent_delete",
                "trash_item",
                Some(&trash_id),
                Some(&name_snapshot),
                "system",
                Some(&format!(
                    "original_id={} item_type={} reason=expired_auto_cleanup",
                    original_id, item_type
                )),
            );
            cleaned += 1;
        }

        Ok(cleaned)
    }
}
