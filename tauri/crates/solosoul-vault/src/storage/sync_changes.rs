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

use super::{map_user_template_row, VaultStore, OBJECT_COLUMNS};
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
            "llm_conversations" => {
                self.list_conversation_changes_since(watermark, account_id, local_node_id)
            }
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

        // P011: 批量取回 HLC（单次 IN 查询）替代逐行 SELECT + 锁获取。
        let hlc_map = self.resolve_hlc_or_fallback_batch(
            "profiles",
            &rows
                .iter()
                .map(|(id, _n, _d, _c, u, _v)| (id.clone(), u.clone()))
                .collect::<Vec<_>>(),
            local_node_id,
        )?;
        let mut out = Vec::new();
        for (id, name, mut data, created, updated, version) in rows {
            let hlc = hlc_map[&id].clone();
            if !Self::hlc_after_watermark(&hlc, watermark) {
                continue;
            }
            // 设备关闭「同步设置偏好」时：剥离 preferences 中**仅外观 UI 键**
            // （主题/主题色/背景/语言/侧边栏等，见 UI_PREF_SYNC_EXCLUDED_KEYS）。
            // AI 对话、回收站保留期、自动锁定等账户级设置不受影响、照常同步。
            // 剥失败保持原样发送（不阻断同步）。
            if !self.ui_prefs_sync_enabled() {
                if let Ok(mut v) = serde_json::from_slice::<serde_json::Value>(&data) {
                    if let Some(prefs) = v.get_mut("preferences").and_then(|p| p.as_object_mut()) {
                        for k in super::UI_PREF_SYNC_EXCLUDED_KEYS {
                            prefs.remove(*k);
                        }
                        if let Ok(re) = serde_json::to_vec(&v) {
                            data = re;
                        }
                    }
                }
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

    /// #1（§4.5）：把指定表的墓碑合并进变更清单并施加分页截断。
    ///
    /// 墓碑（sync_tombstones 行，deleted=true、data=null）HLC 由 new_tombstone_hlc
    /// 生成（wall 严格大于本节点既往值），通常大于在册记录 HLC。合并后按
    /// (HLC, id) 全序升序排序再 truncate(limit)：墓碑只可能在页尾被截断，而
    /// watermark 已推进到页内最大 HLC，下页按新 watermark 过滤仍可再次取到，
    /// 不会丢失；保留的是 HLC 最小的 limit 条，被截断的行 HLC 恒 >= 页内最大
    /// HLC（排序保持最小的 limit 条），keyset 下页正确续取。
    fn merge_tombstones(
        &self,
        mut out: Vec<crate::VaultSyncRecord>,
        table: &str,
        watermark: &crate::SyncWatermark,
        local_node_id: &str,
        limit: usize,
    ) -> Result<Vec<crate::VaultSyncRecord>, String> {
        let mut tombstones = self.list_tombstones_since(table, watermark, local_node_id)?;
        out.append(&mut tombstones);
        out.sort_by(|a, b| {
            (&a.hlc.wall_time_ms, &a.hlc.counter, &a.hlc.node_id, &a.id).cmp(&(
                &b.hlc.wall_time_ms,
                &b.hlc.counter,
                &b.hlc.node_id,
                &b.id,
            ))
        });
        out.truncate(limit);
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
        // P017: SQL keyset 查询 + 行级解密拆入 query_object_changes；
        // 本函数保留最终裁决（水印/keyset 等值组尾部）与墓碑合并。
        let rows =
            self.query_object_changes(watermark, account_id, local_node_id, limit, last_row_id)?;
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

        // #1（§4.5）：合并 objects 墓碑（deleted=true, data=null）随本页投递，
        // 对端 apply 端据此删除本地行。排序/截断语义见 `merge_tombstones`。
        self.merge_tombstones(out, "objects", watermark, local_node_id, limit)
    }

    /// P017: 从 `list_object_changes_since_limited` 拆出的 SQL 查询阶段。
    ///
    /// 一次 LEFT JOIN sync_hlc 批量取回 HLC（消除逐对象 HLC SELECT），并把水印/
    /// keyset 谓词下推 SQL（有/无 HLC 两类行均按 (有效 HLC, o.id) 全序精确过滤），
    /// 返回 (ObjectRecord, 有效 HLC) 列表；调用方负责最终裁决与墓碑合并。
    fn query_object_changes(
        &self,
        watermark: &crate::SyncWatermark,
        account_id: &str,
        local_node_id: &str,
        limit: usize,
        last_row_id: Option<&str>,
    ) -> Result<Vec<(crate::ObjectRecord, crate::RecordHlc)>, String> {
        let key = self.data_key()?;
        let rows = {
            let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
            let conn = guard.as_mut().ok_or("Vault is locked")?;
            // P109: 一次 LEFT JOIN sync_hlc 批量取回 HLC（消除逐对象 HLC SELECT），
            // 同时把水印过滤下推到 SQL：
            //   - 有 HLC 记录的行：按 HLC 三元组精确过滤（与 hlc_after_watermark 等价）
            //   - 无 HLC 记录的行（回退 updated_at）：同样按 (CAST(ms), 0, local) 三元组
            //     精确过滤。注意：julianday→ms 为 SQLite 浮点推导，与 Rust parse_time_ms
            //     （chrono RFC3339→ms）对部分时间戳存在 ≤1ms 差异（migration.rs 639 自述），
            //     并无测试断言两者逐字节一致（V006 核实：P110 断言不存在）。watermark 落在
            //     该 1ms 边界时，SQL 过滤（julianday 推导）与 Rust 交付（parse_time_ms）两套
            //     fallback 可能假阴性漏同步无真实 HLC 的行——低概率隐患，登记 V006 备查。
            let (sql, limit_param) = Self::object_changes_sql(limit);
            let mut stmt = conn
                .prepare_cached(&sql)
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
                    |row| Self::map_object_changes_row(&key, local_node_id, row),
                )
                .map_err(|e| format!("list_object_changes query: {}", e))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("list_object_changes collect: {}", e))?;
            rows
        };

        Ok(rows)
    }
    /// P013: 对象变更清单 SQL 拼装 —— o. 前缀列拼接 + keyset 分页游标参数，独立于行解密。
    fn object_changes_sql(limit: usize) -> (String, i64) {
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
        (
            format!(
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
            ),
            limit_param,
        )
    }

    /// P013: 对象变更清单行解密与组装 —— LEFT JOIN sync_hlc 结果 → ObjectRecord + RecordHlc，
    /// 有 HLC 记录用 HLC、否则回退 updated_at（与旧闭包逐字节等价）。
    fn map_object_changes_row(
        key: &crate::encryption::DataEncryptionKey,
        local_node_id: &str,
        row: &rusqlite::Row<'_>,
    ) -> Result<(crate::ObjectRecord, crate::RecordHlc), rusqlite::Error> {
        let props_str: String = row.get(8)?;
        let labels_str: String = row.get(9)?;
        let decrypted_props = decrypt_text_field(key, &props_str).map_err(|e| {
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
            decrypt_text_field(key, &labels_str).map_err(|e| {
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
        let props: serde_json::Value = serde_json::from_str(&decrypted_props).unwrap_or_default();
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
                .query_map(params![account_id], |row| map_user_template_row(&key, row))
                .map_err(|e| format!("list_template_changes query: {}", e))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("list_template_changes collect: {}", e))?;
            rows
        };

        // P011: 批量取回 HLC（单次 IN 查询）替代逐行 SELECT + 锁获取。
        let hlc_map = self.resolve_hlc_or_fallback_batch(
            "user_templates",
            &rows
                .iter()
                .map(|t| (t.id.clone(), t.updated_at.clone().unwrap_or_default()))
                .collect::<Vec<_>>(),
            local_node_id,
        )?;
        let mut out = Vec::new();
        for tpl in rows {
            let hlc = hlc_map[&tpl.id].clone();
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
                    |row| map_trash_change_row(row, &key, local_node_id),
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

        // #1（§4.5）：合并 trash_items 墓碑（deleted=true, data=null）随本页
        // 投递，对端据此删除对应回收站条目。排序/截断语义见 `merge_tombstones`。
        self.merge_tombstones(out, "trash_items", watermark, local_node_id, limit)
    }
}

/// P019-⑤：trash 变更行 → (TrashItem, RecordHlc) 映射（自
/// list_trash_changes_since_limited 的 query_map 闭包拆出，逻辑逐字保持）。
/// 解密失败映射为 FromSqlConversionFailure 以便 collect 阶段统一报错。
#[allow(clippy::type_complexity)]
fn map_trash_change_row(
    row: &rusqlite::Row<'_>,
    key: &crate::DataEncryptionKey,
    local_node_id: &str,
) -> rusqlite::Result<(crate::TrashItem, crate::RecordHlc)> {
    let raw_data: Vec<u8> = row.get(6)?;
    let data = decrypt_field(key, &raw_data).map_err(|e| {
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
}
