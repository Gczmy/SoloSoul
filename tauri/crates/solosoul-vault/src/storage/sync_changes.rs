//! 同步变更清单域 —— 自 `storage.rs` 拆分（P223-② 表域拆分第五域）。
//!
//! 本模块承载 `VaultStore` 的同步变更清单方法（list_sync_changes_since(_paginated) 分发器 +
//! 四表域实现 list_profile/object/user_template/trash_changes_since 与 objects/trash 两个
//! SQL 级 keyset 分页变体，原 storage.rs 1070-1646 行，逐行搬运零行为变更）。
//!
//! 共享设施经 `super::` 访问父模块私有项：`data_key()`、`OBJECT_COLUMNS`；跨域 pub(crate)
//! 助手（`parse_time_ms`/`hlc_after_watermark`，属 sync_meta 域）与 `crate::encryption`
//! 的 `decrypt_text_field` 按原路径引用。4 个 pub API（list_sync_changes_since /
//! list_sync_changes_since_paginated）可见性不变（solosoul-sync/CLI 跨 crate 调用）；
//! 域内私有实现保持私有。

use rusqlite::params;

use super::{VaultStore, OBJECT_COLUMNS};
use crate::encryption::{decrypt_field, decrypt_text_field};

impl VaultStore {
    /// List records in a table that have an HLC newer than the given watermark.
    pub fn list_sync_changes_since(
        &self,
        table: &str,
        watermark: &crate::SyncWatermark,
        account_id: &str,
        local_node_id: &str,
    ) -> Result<Vec<crate::VaultSyncRecord>, String> {
        match table {
            "profiles" => self.list_profile_changes_since(watermark, local_node_id),
            "objects" => self.list_object_changes_since(watermark, account_id, local_node_id),
            "user_templates" => {
                self.list_user_template_changes_since(watermark, account_id, local_node_id)
            }
            "trash_items" => self.list_trash_changes_since(watermark, local_node_id),
            _ => Err(format!("Unsupported sync table: {}", table)),
        }
    }

    /// Paginated version of `list_sync_changes_since`.
    ///
    /// Returns at most `limit` records newer than `watermark`, using keyset pagination:
    /// `last_row_id` is the id of the last record returned on the previous page (the page
    /// cursor). This lets the sync engine stream large tables in multiple `Batch` messages
    /// without loading the entire result set into a single message.
    ///
    /// N-1: objects 走 SQL 级 keyset 分页（(有效 HLC, o.id) 全序 + 游标推进）；
    /// R-1: trash_items 同构——SQL 级 keyset（LEFT JOIN sync_hlc，无 HLC 回退行
    /// wall == deleted_at 毫秒值，见 `list_trash_changes_since_limited`）；
    /// 其余小表（profiles/user_templates）数据量小维持内存分页（先按有效 HLC
    /// 升序排序再 take），游标参数忽略。
    pub fn list_sync_changes_since_paginated(
        &self,
        table: &str,
        watermark: &crate::SyncWatermark,
        account_id: &str,
        local_node_id: &str,
        limit: usize,
        last_row_id: Option<&str>,
    ) -> Result<Vec<crate::VaultSyncRecord>, String> {
        if table == "objects" {
            return self.list_object_changes_since_limited(
                watermark,
                account_id,
                local_node_id,
                limit,
                last_row_id,
            );
        }
        if table == "trash_items" {
            // R-1: trash_items 同构 keyset 化——page_delete 整页同 ms 批量删除
            // 不再因「严格 > + take(limit)」在第 2 页空页 break 而永久漏发。
            return self.list_trash_changes_since_limited(
                watermark,
                local_node_id,
                limit,
                last_row_id,
            );
        }
        let mut all = self.list_sync_changes_since(table, watermark, account_id, local_node_id)?;
        all.sort_by(|a, b| {
            (&a.hlc.wall_time_ms, &a.hlc.counter, &a.hlc.node_id).cmp(&(
                &b.hlc.wall_time_ms,
                &b.hlc.counter,
                &b.hlc.node_id,
            ))
        });
        Ok(all.into_iter().take(limit).collect())
    }

    fn list_profile_changes_since(
        &self,
        watermark: &crate::SyncWatermark,
        local_node_id: &str,
    ) -> Result<Vec<crate::VaultSyncRecord>, String> {
        let key = self.data_key()?;
        let rows = {
            let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
            let conn = guard.as_mut().ok_or("Vault is locked")?;
            let mut stmt = conn
                .prepare("SELECT id, name, data, created_at, updated_at, version FROM profiles")
                .map_err(|e| format!("list_profile_changes: {}", e))?;
            let rows = stmt
                .query_map([], |row| {
                    let raw_data: Vec<u8> = row.get(2)?;
                    let data = decrypt_field(&key, &raw_data).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            2,
                            rusqlite::types::Type::Blob,
                            Box::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!("Profile decryption failed: {}", e),
                            )),
                        )
                    })?;
                    let created: String = row.get(3)?;
                    let updated: String = row.get(4)?;
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        data,
                        created,
                        updated,
                        row.get::<_, u32>(5)?,
                    ))
                })
                .map_err(|e| format!("list_profile_changes query: {}", e))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("list_profile_changes collect: {}", e))?;
            rows
        };

        let mut out = Vec::new();
        for (id, name, data, created, updated, version) in rows {
            let hlc = self.record_hlc_or_fallback("profiles", &id, &updated, local_node_id)?;
            if !Self::hlc_after_watermark(&hlc, watermark) {
                continue;
            }
            let value = serde_json::json!({
                "id": id,
                "name": name,
                "data": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data),
                "createdAt": created,
                "updatedAt": updated,
                "version": version,
            });
            out.push(crate::VaultSyncRecord {
                id,
                table: "profiles".to_string(),
                data: value,
                hlc,
                deleted: false,
            });
        }
        let mut tombstones = self.list_tombstones_since("profiles", watermark, local_node_id)?;
        out.append(&mut tombstones);
        Ok(out)
    }

    fn list_object_changes_since(
        &self,
        watermark: &crate::SyncWatermark,
        account_id: &str,
        local_node_id: &str,
    ) -> Result<Vec<crate::VaultSyncRecord>, String> {
        self.list_object_changes_since_limited(
            watermark,
            account_id,
            local_node_id,
            usize::MAX,
            None,
        )
    }

    /// P110: objects 表的 SQL 级分页实现。
    ///
    /// 与 P109 相同的水印下推 + HLC 批量 JOIN，另加：
    ///   - `ORDER BY` 有效 HLC 三元组 + `o.id` 升序（有 HLC 行用落库三元组；无 HLC
    ///     回退行用 `julianday(updated_at)` 推导的 wall_time，counter=0，node=local）——
    ///     这是分页正确性的关键：会话层每页把 peer watermark 推进到“本页最大 HLC”。
    ///   - N-1 修复：keyset 分页替代 OFFSET——`last_row_id` 作为页游标，WHERE 以
    ///     (有效 HLC, o.id) 全序 > (水印, 游标) 推进，解决 P110 遗留两类问题：
    ///     ① 与页面最大 HLC 相同的回退行（同 ms 批量写入）在严格 > 下被下一页永久跳过
    ///     （数据漏发）；
    ///     ② 同秒回退假阳性（updated_at 在 [整秒下界, 水印] 区间）填满 LIMIT 预算，
    ///     被 Rust 精确过滤后形成“空页但 finished=false、max_hlc=None、水印永不推进”
    ///     的死循环。
    ///     修复后 SQL 对有无 HLC 两类行都做 (三元组, id) 全序精确过滤，假阳性不再
    ///     占用 LIMIT 预算；`last_row_id=None` 时退化为严格三元组 >（非分页语义）。
    ///
    /// `limit=usize::MAX` 时等价于 P109 的非分页行为（LIMIT 不生效）。
    fn list_object_changes_since_limited(
        &self,
        watermark: &crate::SyncWatermark,
        account_id: &str,
        local_node_id: &str,
        limit: usize,
        last_row_id: Option<&str>,
    ) -> Result<Vec<crate::VaultSyncRecord>, String> {
        let key = self.data_key()?;
        let rows = {
            let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
            let conn = guard.as_mut().ok_or("Vault is locked")?;
            // P109: 一次 LEFT JOIN sync_hlc 批量取回 HLC（消除逐对象 HLC SELECT），
            // 同时把水印过滤下推到 SQL：
            //   - 有 HLC 记录的行：按 HLC 三元组精确过滤（与 hlc_after_watermark 等价）
            //   - 无 HLC 记录的行（回退 updated_at）：同样按 (CAST(ms), 0, local) 三元组
            //     精确过滤——julianday→ms 的浮点精度（~86µs）远低于 1ms，与 Rust
            //     parse_time_ms 逐字节一致（P110 断言），故可安全下推，不再粗筛。
            let o_columns = OBJECT_COLUMNS
                .split(',')
                .map(|c| format!("o.{}", c.trim()))
                .collect::<Vec<_>>()
                .join(", ");
            // 分页：LIMIT 传 usize::MAX 时按 SQLite 语义（LIMIT -1）不限制行数。
            let limit_param = if limit == usize::MAX {
                -1i64
            } else {
                limit as i64
            };
            // P213: prepare_cached 按 SQL 文本缓存（o. 前缀列拼接结果稳定），避免每次重编译。
            // keyset：游标 ?7 为 NULL（未分页）时仅严格三元组 >；为字符串时允许
            // (三元组 == 水印) 且 id > 游标的等值组尾部行通过（跨页不重不漏）。
            let mut stmt = conn
            .prepare_cached(&format!(
                "SELECT {cols}, h.wall_time_ms AS hlc_wall, h.counter AS hlc_counter, h.node_id AS hlc_node
                 FROM objects o
                 LEFT JOIN sync_hlc h ON h.table_name = 'objects' AND h.record_id = o.id
                 WHERE o.account_id = ?1 AND (
                    (h.wall_time_ms IS NOT NULL AND (
                        (h.wall_time_ms, COALESCE(h.counter, 0), COALESCE(h.node_id, ?5)) > (?2, ?3, ?4)
                        OR ((h.wall_time_ms, COALESCE(h.counter, 0), COALESCE(h.node_id, ?5)) = (?2, ?3, ?4)
                            AND ?7 IS NOT NULL AND o.id > ?7)
                    ))
                    OR (h.wall_time_ms IS NULL AND (
                        (CAST((julianday(o.updated_at) - 2440587.5) * 86400000.0 AS INTEGER), 0, ?5) > (?2, ?3, ?4)
                        OR ((CAST((julianday(o.updated_at) - 2440587.5) * 86400000.0 AS INTEGER), 0, ?5) = (?2, ?3, ?4)
                            AND ?7 IS NOT NULL AND o.id > ?7)
                    ))
                 )
                 ORDER BY
                   COALESCE(h.wall_time_ms, CAST((julianday(o.updated_at) - 2440587.5) * 86400000.0 AS INTEGER)) ASC,
                   COALESCE(h.counter, 0) ASC,
                   COALESCE(h.node_id, ?5) ASC,
                   o.id ASC
                 LIMIT ?6",
                cols = o_columns,
            ))
            .map_err(|e| format!("list_object_changes: {}", e))?;
            let rows = stmt
                .query_map(
                    params![
                        account_id,
                        watermark.wall_time_ms as i64,
                        watermark.counter as i32,
                        &watermark.node_id,
                        local_node_id,
                        limit_param,
                        last_row_id.map(str::to_owned),
                    ],
                    |row| {
                        let props_str: String = row.get(8)?;
                        let labels_str: String = row.get(9)?;
                        let decrypted_props =
                            decrypt_text_field(&key, &props_str).map_err(|e| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    8,
                                    rusqlite::types::Type::Text,
                                    Box::new(std::io::Error::new(
                                        std::io::ErrorKind::InvalidData,
                                        format!("Object properties decryption failed: {}", e),
                                    )),
                                )
                            })?;
                        let decrypted_labels = if labels_str.is_empty() {
                            Ok(String::new())
                        } else {
                            decrypt_text_field(&key, &labels_str).map_err(|e| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    9,
                                    rusqlite::types::Type::Text,
                                    Box::new(std::io::Error::new(
                                        std::io::ErrorKind::InvalidData,
                                        format!("Object labels decryption failed: {}", e),
                                    )),
                                )
                            })
                        }?;
                        let children: Vec<String> =
                            serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default();
                        let tags: Vec<String> =
                            serde_json::from_str(&row.get::<_, String>(13)?).unwrap_or_default();
                        let labels: Option<serde_json::Value> = if decrypted_labels.is_empty() {
                            None
                        } else {
                            serde_json::from_str(&decrypted_labels).ok()
                        };
                        let props: serde_json::Value =
                            serde_json::from_str(&decrypted_props).unwrap_or_default();
                        let obj = crate::ObjectRecord {
                            id: row.get(0)?,
                            account_id: row.get(1)?,
                            type_id: row.get(2)?,
                            section_type: row.get(3)?,
                            name: row.get(4)?,
                            icon_name: row.get(5)?,
                            parent_id: row.get(6)?,
                            children_ids: children,
                            properties: props,
                            property_labels: labels,
                            sensitivity_level: row.get(10)?,
                            is_deleted: row.get::<_, i32>(11)? != 0,
                            deleted_at: row.get(12)?,
                            tags_json: tags,
                            template_id: row.get(14)?,
                            template_type: row.get(15)?,
                            contract_type_id: row.get(16)?,
                            template_hash: row.get(17)?,
                            ignored_template_hash: row.get(18)?,
                            created_at: row.get(19)?,
                            updated_at: row.get(20)?,
                            version: row.get(21)?,
                        };
                        // 从 JOIN 结果解析有效 HLC：有 HLC 记录用 HLC，否则回退 updated_at
                        let hlc_wall: Option<i64> = row.get(22)?;
                        let hlc = if let Some(wall) = hlc_wall {
                            crate::RecordHlc {
                                wall_time_ms: wall as u64,
                                counter: row.get::<_, Option<i32>>(23)?.unwrap_or(0) as u32,
                                node_id: row.get::<_, Option<String>>(24)?.unwrap_or_default(),
                            }
                        } else {
                            crate::RecordHlc {
                                wall_time_ms: Self::parse_time_ms(&obj.updated_at),
                                counter: 0,
                                node_id: local_node_id.to_string(),
                            }
                        };
                        Ok((obj, hlc))
                    },
                )
                .map_err(|e| format!("list_object_changes query: {}", e))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("list_object_changes collect: {}", e))?;
            rows
        };

        let mut out = Vec::new();
        for (obj, hlc) in rows {
            // 最终裁决（与 SQL 谓词逐字一致）：严格 > 水印，或（keyset 游标存在且
            // 三元组 == 水印且 id > 游标）的等值组尾部行。
            let equal_watermark = hlc.wall_time_ms == watermark.wall_time_ms
                && hlc.counter == watermark.counter
                && hlc.node_id == watermark.node_id;
            let keyset_tail = equal_watermark && last_row_id.is_some_and(|c| c < obj.id.as_str());
            if !Self::hlc_after_watermark(&hlc, watermark) && !keyset_tail {
                continue;
            }
            let id = obj.id.clone();
            out.push(crate::VaultSyncRecord {
                id,
                table: "objects".to_string(),
                data: serde_json::to_value(&obj)
                    .map_err(|e| format!("serialize object for sync: {}", e))?,
                hlc,
                deleted: obj.is_deleted,
            });
        }
        Ok(out)
    }

    fn list_user_template_changes_since(
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
                "SELECT id, account_id, name, icon_id, properties_json, category, contract_type_id, created_at, updated_at
                 FROM user_templates WHERE account_id = ?1",
            )
            .map_err(|e| format!("list_template_changes: {}", e))?;
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
                    let tpl = crate::UserTemplate {
                        contract_type_id: row.get(6)?,
                        id: row.get(0)?,
                        account_id: row.get(1)?,
                        name: row.get(2)?,
                        icon_id: row.get(3)?,
                        properties,
                        category: row.get(5)?,
                        created_at: row.get(7)?,
                        updated_at: row.get(8)?,
                    };
                    Ok(tpl)
                })
                .map_err(|e| format!("list_template_changes query: {}", e))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("list_template_changes collect: {}", e))?;
            rows
        };

        let mut out = Vec::new();
        for tpl in rows {
            let updated = tpl.updated_at.clone().unwrap_or_default();
            let hlc =
                self.record_hlc_or_fallback("user_templates", &tpl.id, &updated, local_node_id)?;
            if !Self::hlc_after_watermark(&hlc, watermark) {
                continue;
            }
            let id = tpl.id.clone();
            out.push(crate::VaultSyncRecord {
                id,
                table: "user_templates".to_string(),
                data: serde_json::to_value(&tpl)
                    .map_err(|e| format!("serialize template for sync: {}", e))?,
                hlc,
                deleted: false,
            });
        }
        let mut tombstones =
            self.list_tombstones_since("user_templates", watermark, local_node_id)?;
        out.append(&mut tombstones);
        Ok(out)
    }

    fn list_trash_changes_since(
        &self,
        watermark: &crate::SyncWatermark,
        local_node_id: &str,
    ) -> Result<Vec<crate::VaultSyncRecord>, String> {
        // R-1: 非分页语义 = LIMIT 不限制 + 无游标（严格三元组 >）。
        self.list_trash_changes_since_limited(watermark, local_node_id, usize::MAX, None)
    }

    /// R-1: trash_items 表 SQL 级 keyset 分页（镜像 list_object_changes_since_limited）。
    ///
    /// 背景：小表通用分页路径此前是「严格 hlc_after_watermark 过滤 + 内存
    /// take(limit)」——page_delete 给整页对象同一个 deleted_at 毫秒值（回退 HLC
    /// 三元组完全相同），删除含 >limit 对象的页面时第 2 页空页 break，剩余
    /// trash_items 永久不同步（P110 同构缺陷）。本实现：
    ///   - LEFT JOIN sync_hlc 一次取回真实 HLC（对端应用写入的行有真实 HLC）；
    ///   - 有无 HLC 两类行均按 (有效 HLC, t.id) 全序 > (水印, 游标) 精确过滤——
    ///     无 HLC 回退行 wall == deleted_at 毫秒值（R-2 修复后无浮点推导）；
    ///   - 等值组尾部（三元组 == 水印 且 id > 游标）放行，跨页不重不漏。
    ///
    /// `last_row_id=None` 时退化为严格三元组 >（非分页语义）。
    fn list_trash_changes_since_limited(
        &self,
        watermark: &crate::SyncWatermark,
        local_node_id: &str,
        limit: usize,
        last_row_id: Option<&str>,
    ) -> Result<Vec<crate::VaultSyncRecord>, String> {
        let key = self.data_key()?;
        let rows = {
            let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
            let conn = guard.as_mut().ok_or("Vault is locked")?;
            // 分页：LIMIT 传 usize::MAX 时按 SQLite 语义（LIMIT -1）不限制行数。
            let limit_param = if limit == usize::MAX {
                -1i64
            } else {
                limit as i64
            };
            // R-1: keyset——游标 ?4 为 NULL（未分页）时仅严格三元组 >；为字符串时
            // 允许 (三元组 == 水印) 且 id > 游标的等值组尾部行通过（跨页不重不漏）。
            let mut stmt = conn
            .prepare_cached(
                "SELECT t.id, t.item_type, t.original_id, t.original_parent_id, t.original_section_type,
                 t.original_sort_order, t.data, t.deleted_at, t.expires_at, t.deleted_by, t.name_snapshot, t.icon_snapshot,
                 h.wall_time_ms AS hlc_wall, h.counter AS hlc_counter, h.node_id AS hlc_node
                 FROM trash_items t
                 LEFT JOIN sync_hlc h ON h.table_name = 'trash_items' AND h.record_id = t.id
                 WHERE (
                    (h.wall_time_ms IS NOT NULL AND (
                        (h.wall_time_ms, COALESCE(h.counter, 0), COALESCE(h.node_id, ?5)) > (?1, ?2, ?3)
                        OR ((h.wall_time_ms, COALESCE(h.counter, 0), COALESCE(h.node_id, ?5)) = (?1, ?2, ?3)
                            AND ?4 IS NOT NULL AND t.id > ?4)
                    ))
                    OR (h.wall_time_ms IS NULL AND (
                        (t.deleted_at, 0, ?5) > (?1, ?2, ?3)
                        OR ((t.deleted_at, 0, ?5) = (?1, ?2, ?3)
                            AND ?4 IS NOT NULL AND t.id > ?4)
                    ))
                 )
                 ORDER BY
                   COALESCE(h.wall_time_ms, t.deleted_at) ASC,
                   COALESCE(h.counter, 0) ASC,
                   COALESCE(h.node_id, ?5) ASC,
                   t.id ASC
                 LIMIT ?6",
            )
            .map_err(|e| format!("list_trash_changes: {}", e))?;
            let rows = stmt
                .query_map(
                    params![
                        watermark.wall_time_ms as i64,
                        watermark.counter as i32,
                        &watermark.node_id,
                        last_row_id.map(str::to_owned),
                        local_node_id,
                        limit_param,
                    ],
                    |row| {
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
                        let deleted_at: i64 = row.get(7)?;
                        let item = crate::TrashItem {
                            id: row.get(0)?,
                            item_type: row.get(1)?,
                            original_id: row.get(2)?,
                            original_parent_id: row.get(3)?,
                            original_section_type: row.get(4)?,
                            original_sort_order: row.get(5)?,
                            data,
                            deleted_at,
                            expires_at: row.get(8)?,
                            deleted_by: row.get(9)?,
                            name_snapshot: row.get(10)?,
                            icon_snapshot: row.get(11)?,
                        };
                        // 有效 HLC：有 HLC 行用落库三元组，否则回退 deleted_at(毫秒)
                        let hlc_wall: Option<i64> = row.get(12)?;
                        let hlc = if let Some(wall) = hlc_wall {
                            crate::RecordHlc {
                                wall_time_ms: wall as u64,
                                counter: row.get::<_, Option<i32>>(13)?.unwrap_or(0) as u32,
                                node_id: row.get::<_, Option<String>>(14)?.unwrap_or_default(),
                            }
                        } else {
                            crate::RecordHlc {
                                wall_time_ms: deleted_at as u64,
                                counter: 0,
                                node_id: local_node_id.to_string(),
                            }
                        };
                        Ok((item, hlc))
                    },
                )
                .map_err(|e| format!("list_trash_changes query: {}", e))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("list_trash_changes collect: {}", e))?;
            rows
        };

        let mut out = Vec::new();
        for (item, hlc) in rows {
            // 最终裁决（与 SQL 谓词逐字一致）：严格 > 水印，或（keyset 游标存在且
            // 三元组 == 水印且 id > 游标）的等值组尾部行。
            let equal_watermark = hlc.wall_time_ms == watermark.wall_time_ms
                && hlc.counter == watermark.counter
                && hlc.node_id == watermark.node_id;
            let keyset_tail = equal_watermark && last_row_id.is_some_and(|c| c < item.id.as_str());
            if !Self::hlc_after_watermark(&hlc, watermark) && !keyset_tail {
                continue;
            }
            let id = item.id.clone();
            out.push(crate::VaultSyncRecord {
                id,
                table: "trash_items".to_string(),
                data: serde_json::to_value(&item)
                    .map_err(|e| format!("serialize trash item for sync: {}", e))?,
                hlc,
                deleted: false,
            });
        }
        Ok(out)
    }
}
