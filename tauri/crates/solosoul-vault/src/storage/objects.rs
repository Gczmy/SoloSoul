//! Object CRUD 域 —— 自 `storage.rs` 拆分（P223-② 表域拆分试点）。
//!
//! 本模块承载 `VaultStore` 的对象读写方法（save_object / load_object / list_objects /
//! list_object_metadata / search_objects / delete_object / restore_object 等 15 个方法，
//! 原 storage.rs 2554-3184 行，逐行搬运零行为变更）。
//!
//! 共享设施经 `super::` 访问父模块私有项：`data_key()`、`with_tx`、`json_contains_ignore_case`
//! 与对象 SQL 常量（OBJECT_COLUMNS / OBJECT_SELECT_BASE / OBJECT_LOAD_SQL / OBJECT_SAVE_SQL）。
//! `save_object_tx` / `load_object_tx` 以 `pub(crate)` 暴露——根模块同步应用路径
//! （`apply_object_sync_record_tx`）跨域复用，事务内语义不变。

use rusqlite::{params, Connection, OptionalExtension};

use super::{
    json_contains_ignore_case, object_has_attachments, with_tx, VaultStore, OBJECT_COLUMNS,
    OBJECT_LOAD_SQL, OBJECT_SAVE_SQL, OBJECT_SELECT_BASE,
};
use crate::encryption::{decrypt_text_field, encrypt_text_field, DataEncryptionKey};
use crate::{ObjectRecord, ObjectSummary};

impl VaultStore {
    // ── Object CRUD ─────────────────────────────────────────

    pub fn save_object(&self, obj: &ObjectRecord) -> Result<(), String> {
        let key = self.data_key()?;
        // 方案 B（R-3 根治）：本地写统一生成并落库 HLC。生成需读 sync_hlc 最大值，
        // 必须在持锁前调用（new_local_hlc 内部自行锁 conn）。
        let hlc = self.new_local_hlc()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        with_tx(
            conn,
            "Failed to begin transaction",
            "Failed to commit transaction",
            |c| {
                Self::save_object_tx(c, &key, obj)?;
                Self::set_record_hlc_tx(c, "objects", &obj.id, &hlc)?;
                Ok(())
            },
        )
    }

    /// P212: 单事务批量保存对象（导入等批量场景），替代逐条 `save_object` 的
    /// N 次 auto-commit 写事务。任一条失败整体回滚，不产生半导入。
    /// 方案 B：每个对象在写事务内同时落库独立 HLC（new_local_hlc 递增保证唯一）。
    pub fn save_objects_batch(&self, objects: &[ObjectRecord]) -> Result<(), String> {
        let key = self.data_key()?;
        // 批内逐个生成 HLC：每个都读 sync_hlc 最大值，保证递增（同批不同对象
        // 时间戳可能相同，但 node/counter 组合与 id 决胜保证 keyset 分页不重不漏）。
        let hlcs = objects
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
                for (obj, hlc) in objects.iter().zip(hlcs.iter()) {
                    Self::save_object_tx(c, &key, obj)?;
                    Self::set_record_hlc_tx(c, "objects", &obj.id, hlc)?;
                }
                Ok(())
            },
        )
    }

    /// P115: 事务内保存对象（连接由调用方持有，批量应用单事务内复用）。
    pub(crate) fn save_object_tx(
        conn: &mut Connection,
        key: &DataEncryptionKey,
        obj: &ObjectRecord,
    ) -> Result<(), String> {
        // 保存模板名称到 properties，用于模板被删除后仍能显示原始模板名
        let mut properties = obj.properties.clone();
        if let Some(ref tid) = obj.template_id {
            let tpl_name: Result<String, _> = conn.query_row(
                "SELECT name FROM user_templates WHERE id = ?1",
                params![tid],
                |row| row.get(0),
            );
            if let Ok(name) = tpl_name {
                if let Some(map) = properties.as_object_mut() {
                    map.insert(
                        "__templateName".to_string(),
                        serde_json::Value::String(name),
                    );
                }
            }
        }
        let children_json = serde_json::to_string(&obj.children_ids)
            .map_err(|e| format!("serialize children_ids: {}", e))?;
        let props_json = serde_json::to_string(&properties)
            .map_err(|e| format!("serialize properties: {}", e))?;
        let labels_json = if let Some(ref v) = obj.property_labels {
            serde_json::to_string(v).map_err(|e| format!("serialize property_labels: {}", e))?
        } else {
            String::new()
        };
        let encrypted_props = encrypt_text_field(key, &props_json)?;
        let encrypted_labels = if labels_json.is_empty() {
            String::new()
        } else {
            encrypt_text_field(key, &labels_json)?
        };
        let tags_str =
            serde_json::to_string(&obj.tags_json).map_err(|e| format!("serialize tags: {}", e))?;
        let mut stmt = conn
            .prepare_cached(OBJECT_SAVE_SQL)
            .map_err(|e| format!("save_object prepare: {}", e))?;
        stmt.execute(params![
            obj.id,
            obj.account_id,
            obj.type_id,
            obj.section_type,
            obj.name,
            obj.icon_name,
            obj.parent_id,
            children_json,
            encrypted_props,
            encrypted_labels,
            obj.sensitivity_level,
            obj.is_deleted as i32,
            obj.deleted_at,
            tags_str,
            obj.template_id,
            obj.template_type,
            obj.contract_type_id.clone(),
            obj.template_hash.clone(),
            obj.ignored_template_hash.clone(),
            obj.created_at,
            obj.updated_at,
            obj.version,
        ])
        .map_err(|e| format!("save_object: {}", e))?;
        Ok(())
    }

    /// P225: 行 → ObjectRecord 映射（load_object_tx / load_objects_batch / list_object_records
    /// 三处重复闭包收敛为单一实现；错误文案统一为 Object 前缀，原 search 路径前缀为 Search）。
    fn object_row_to_record(
        key: &DataEncryptionKey,
        row: &rusqlite::Row,
    ) -> rusqlite::Result<ObjectRecord> {
        let children_str: String = row.get(7)?;
        let props_str: String = row.get(8)?;
        let labels_str: String = row.get(9)?;
        let tags_str: String = row.get(13)?;
        let deleted: i32 = row.get(11)?;
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
            decrypt_text_field(key, &labels_str)
        }
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                9,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Object labels decryption failed: {}", e),
                )),
            )
        })?;
        Ok(ObjectRecord {
            id: row.get(0)?,
            account_id: row.get(1)?,
            type_id: row.get(2)?,
            section_type: row.get(3)?,
            name: row.get(4)?,
            icon_name: row.get(5)?,
            parent_id: row.get(6)?,
            children_ids: serde_json::from_str(&children_str).unwrap_or_default(),
            properties: serde_json::from_str(&decrypted_props).unwrap_or(serde_json::Value::Null),
            property_labels: if decrypted_labels.is_empty() {
                None
            } else {
                serde_json::from_str(&decrypted_labels).ok()
            },
            sensitivity_level: row.get(10)?,
            is_deleted: deleted != 0,
            deleted_at: row.get(12)?,
            tags_json: serde_json::from_str(&tags_str).unwrap_or_default(),
            template_id: row.get(14)?,
            template_type: row.get(15)?,
            contract_type_id: row.get(16)?,
            template_hash: row.get(17)?,
            ignored_template_hash: row.get(18)?,
            created_at: row.get(19)?,
            updated_at: row.get(20)?,
            version: row.get(21)?,
        })
    }

    pub fn load_object(&self, id: &str) -> Result<Option<ObjectRecord>, String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        Self::load_object_tx(conn, &key, id)
    }

    /// P115: 事务内加载对象（连接由调用方持有，批量应用单事务内复用）。
    pub(crate) fn load_object_tx(
        conn: &mut Connection,
        key: &DataEncryptionKey,
        id: &str,
    ) -> Result<Option<ObjectRecord>, String> {
        let mut stmt = conn
            .prepare_cached(OBJECT_LOAD_SQL)
            .map_err(|e| format!("load_object: {}", e))?;
        let result = stmt
            .query_row(params![id], |row| Self::object_row_to_record(key, row))
            .optional()
            .map_err(|e| format!("Failed to load object: {}", e))?;
        Ok(result)
    }

    /// P110: Batch-load multiple objects by ID, avoiding N+1 `load_object` calls.
    /// Returns a HashMap keyed by object ID, containing only the objects that were found.
    pub fn load_objects_batch(
        &self,
        ids: &[String],
    ) -> Result<std::collections::HashMap<String, ObjectRecord>, String> {
        if ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;

        // Build placeholders: (?1,?2,...,?N)
        let placeholders: Vec<String> = (1..=ids.len()).map(|i| format!("?{}", i)).collect();
        // P213: 复用 OBJECT_SELECT_BASE 前缀常量，避免每次 format! 拼接整段列清单。
        let sql = format!(
            "{} WHERE id IN ({})",
            OBJECT_SELECT_BASE,
            placeholders.join(",")
        );

        // P213: prepare_cached 按 SQL 文本缓存（同批量大小命中），避免每次重编译。
        let mut stmt = conn
            .prepare_cached(&sql)
            .map_err(|e| format!("load_objects_batch: {}", e))?;

        // Convert IDs to a slice of &dyn ToSql
        let params_refs: Vec<&dyn rusqlite::types::ToSql> = ids
            .iter()
            .map(|id| id as &dyn rusqlite::types::ToSql)
            .collect();

        let rows = stmt
            .query_map(params_refs.as_slice(), |row| {
                Self::object_row_to_record(&key, row)
            })
            .map_err(|e| format!("load_objects_batch query: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("load_objects_batch collect: {}", e))?;

        let mut result = std::collections::HashMap::with_capacity(rows.len());
        for obj in rows {
            result.insert(obj.id.clone(), obj);
        }
        Ok(result)
    }

    /// R020: batch-load object attachment IDs without the N+1 `load_object` calls
    /// required by `get_vault_stats`. Returns `(object_id, attachment_ids)` pairs
    /// for all active objects in the account.
    pub fn list_object_attachment_ids(
        &self,
        account_id: &str,
    ) -> Result<Vec<(String, Vec<String>)>, String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn
            .prepare("SELECT id, properties FROM objects WHERE account_id = ?1 AND is_deleted = 0")
            .map_err(|e| format!("list_object_attachment_ids: {}", e))?;
        let rows = stmt
            .query_map(params![account_id], |row| {
                let id: String = row.get(0)?;
                let props_str: String = row.get(1)?;
                let decrypted = decrypt_text_field(&key, &props_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Object properties decryption failed: {}", e),
                        )),
                    )
                })?;
                Ok((id, decrypted))
            })
            .map_err(|e| format!("list_object_attachment_ids: {}", e))?;

        let mut result = Vec::new();
        for row in rows {
            let (id, decrypted) = row.map_err(|e| format!("list_object_attachment_ids: {}", e))?;
            let props: serde_json::Value = serde_json::from_str(&decrypted)
                .map_err(|e| format!("deserialize attachment props: {}", e))?;
            let att_ids: Vec<String> = props
                .get("__attachments")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|a| a.get("id").and_then(|i| i.as_str()).map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            result.push((id, att_ids));
        }
        Ok(result)
    }

    pub fn list_objects(
        &self,
        account_id: &str,
        type_id: Option<&str>,
        parent_id: Option<&str>,
        keyword: Option<&str>,
        include_deleted: bool,
        only_deleted: bool,
    ) -> Result<Vec<ObjectSummary>, String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;

        let lower_kw = keyword.map(|k| k.to_lowercase());
        let mut sql = String::from(
            "SELECT id, name, type_id, section_type, sensitivity_level, created_at, updated_at, is_deleted, properties, tags_json, template_id, template_type, contract_type_id, template_hash, ignored_template_hash, icon_name, property_labels, parent_id
             FROM objects WHERE account_id = ?1",
        );
        let mut param_idx = 2;
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> =
            vec![Box::new(account_id.to_string())];

        if only_deleted {
            sql.push_str(" AND is_deleted = 1");
        } else if !include_deleted {
            sql.push_str(" AND is_deleted = 0");
        }

        if let Some(tid) = type_id {
            sql.push_str(&format!(" AND type_id = ?{}", param_idx));
            param_values.push(Box::new(tid.to_string()));
            param_idx += 1;
        }

        if let Some(pid) = parent_id {
            sql.push_str(&format!(" AND parent_id = ?{}", param_idx));
            param_values.push(Box::new(pid.to_string()));
        }

        // properties 已加密，无法使用 SQL LIKE。所有 keyword 匹配在解密后的内存数据上进行。
        sql.push_str(" ORDER BY created_at ASC, id ASC");

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("list_objects: {}", e))?;

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let objects = stmt
            .query_map(params_refs.as_slice(), |row: &rusqlite::Row<'_>| {
                let deleted_int: i32 = row.get(7)?;
                let props_str: String = row.get(8)?;
                let tags_str: String = row.get(9)?;
                let decrypted_props = decrypt_text_field(&key, &props_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        8,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("decrypt object props: {}", e),
                        )),
                    )
                })?;
                let labels_str: String = row.get::<_, String>(16).unwrap_or_default();
                let decrypted_labels = if labels_str.is_empty() {
                    String::new()
                } else {
                    decrypt_text_field(&key, &labels_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            16,
                            rusqlite::types::Type::Text,
                            Box::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!("decrypt object labels: {}", e),
                            )),
                        )
                    })?
                };
                let property_labels: Option<serde_json::Value> = if decrypted_labels.is_empty() {
                    None
                } else {
                    Some(serde_json::from_str(&decrypted_labels).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            9,
                            rusqlite::types::Type::Text,
                            Box::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!("deserialize property_labels: {}", e),
                            )),
                        )
                    })?)
                };
                let properties: serde_json::Value = serde_json::from_str(&decrypted_props)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            8,
                            rusqlite::types::Type::Text,
                            Box::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!("deserialize properties: {}", e),
                            )),
                        )
                    })?;
                let tags: Vec<String> = serde_json::from_str(&tags_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        13,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("deserialize tags: {}", e),
                        )),
                    )
                })?;
                // 附件存在性由已解密的 properties 推导（无额外解密成本）。
                let has_attachments = object_has_attachments(&properties);
                Ok(ObjectSummary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    collection_type: row.get(2)?,
                    section_type: row.get(3)?,
                    sensitivity_level: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    is_deleted: deleted_int != 0,
                    template_id: row.get(10)?,
                    template_type: row.get(11)?,
                    contract_type_id: row.get(12)?,
                    template_hash: row.get(13)?,
                    ignored_template_hash: row.get(14)?,
                    icon_name: row.get(15)?,
                    parent_id: row.get(17)?,
                    properties,
                    property_labels,
                    tags,
                    has_attachments,
                })
            })
            .map_err(|e| format!("list_objects query: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("list_objects collect: {}", e))?;

        // Memory-level keyword filtering on decrypted name and properties.
        // P210: properties 用 json_contains_ignore_case 递归匹配，避免整值 to_string() 往返。
        if let Some(kw) = lower_kw {
            let filtered: Vec<ObjectSummary> = objects
                .into_iter()
                .filter(|o| {
                    o.name.to_lowercase().contains(&kw)
                        || json_contains_ignore_case(&o.properties, &kw)
                })
                .collect();
            Ok(filtered)
        } else {
            Ok(objects)
        }
    }

    /// P111: metadata-only listing —— 不 SELECT/解密 `properties`/`property_labels`/`tags`。
    ///
    /// 返回的 `ObjectSummary` 中 `properties = Null`、`property_labels = None`、`tags = []`，
    /// 其余身份/排序/分区/敏感度字段与 `list_objects` 一致（`ORDER BY created_at ASC, id ASC`）。
    /// 适用于只需元数据、随后单独 `load_object`/`load_objects_batch` 的调用方
    /// （page_delete、模板迁移、附件清单收集、导出范围收集、回收站重名检查等），
    /// 避免主列表公共路径上的全表 AES-GCM 解密 + JSON 解析。
    pub fn list_object_metadata(
        &self,
        account_id: &str,
        type_id: Option<&str>,
        parent_id: Option<&str>,
        include_deleted: bool,
        only_deleted: bool,
    ) -> Result<Vec<ObjectSummary>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;

        let mut sql = String::from(
            "SELECT id, name, type_id, section_type, sensitivity_level, created_at, updated_at, is_deleted, template_id, template_type, contract_type_id, template_hash, ignored_template_hash, icon_name, parent_id
             FROM objects WHERE account_id = ?1",
        );
        let mut param_idx = 2;
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> =
            vec![Box::new(account_id.to_string())];

        if only_deleted {
            sql.push_str(" AND is_deleted = 1");
        } else if !include_deleted {
            sql.push_str(" AND is_deleted = 0");
        }

        if let Some(tid) = type_id {
            sql.push_str(&format!(" AND type_id = ?{}", param_idx));
            param_values.push(Box::new(tid.to_string()));
            param_idx += 1;
        }

        if let Some(pid) = parent_id {
            sql.push_str(&format!(" AND parent_id = ?{}", param_idx));
            param_values.push(Box::new(pid.to_string()));
        }

        sql.push_str(" ORDER BY created_at ASC, id ASC");

        // P213: prepare_cached 按 SQL 文本缓存（同过滤器组合命中），避免每次重编译。
        let mut stmt = conn
            .prepare_cached(&sql)
            .map_err(|e| format!("list_object_metadata: {}", e))?;

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let objects = stmt
            .query_map(params_refs.as_slice(), |row: &rusqlite::Row<'_>| {
                let deleted_int: i32 = row.get(7)?;
                Ok(ObjectSummary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    collection_type: row.get(2)?,
                    section_type: row.get(3)?,
                    sensitivity_level: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    is_deleted: deleted_int != 0,
                    template_id: row.get(8)?,
                    template_type: row.get(9)?,
                    contract_type_id: row.get(10)?,
                    template_hash: row.get(11)?,
                    ignored_template_hash: row.get(12)?,
                    icon_name: row.get(13)?,
                    parent_id: row.get(14)?,
                    // P111: 不解密负载列，占位值（调用方不得依赖）
                    properties: serde_json::Value::Null,
                    property_labels: None,
                    tags: vec![],
                    // metadata-only 路径不解密 properties，附件存在性不可知 → false。
                    has_attachments: false,
                })
            })
            .map_err(|e| format!("list_object_metadata query: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("list_object_metadata collect: {}", e))?;

        Ok(objects)
    }

    pub fn delete_object(&self, id: &str, soft: bool) -> Result<(), String> {
        if soft {
            // 方案 B：软删也是写变更，落库新 HLC（对象行保留，is_deleted 翻转）。
            let hlc = self.new_local_hlc()?;
            let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
            let conn = guard.as_mut().ok_or("Vault is locked")?;
            let now = chrono::Utc::now().to_rfc3339();
            with_tx(
                conn,
                "Failed to begin transaction",
                "Failed to commit transaction",
                |c| {
                    c.execute(
                        "UPDATE objects SET is_deleted = 1, deleted_at = ?1, updated_at = ?1 WHERE id = ?2",
                        params![now, id],
                    )
                    .map_err(|e| format!("soft_delete_object: {}", e))?;
                    Self::set_record_hlc_tx(c, "objects", id, &hlc)?;
                    Ok(())
                },
            )
        } else {
            // #1（§4.5）：硬删是墓碑变更——先删行，再记录墓碑（record_tombstone 内部
            // 落库 sync_tombstones + 墓碑 HLC，wall 强制大于本节点既往值，保证对端
            // conflict 裁决时删除胜出）。行不存在（重复 purge/幂等清理）时不记墓碑，
            // 避免产生无实体的幽灵墓碑。
            // 已知取舍：DELETE 事务提交与 record_tombstone（独立语句）非原子——两者间
            // 进程崩溃留下「行已删但无墓碑」：删除不传播但对端也不回魂（与 delete_profile
            // 既有模式一致，R-4① 同款窄窗口已在 vault_service 侧有 pending 机制兜底）。
            let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
            let conn = guard.as_mut().ok_or("Vault is locked")?;
            let affected = with_tx(
                conn,
                "Failed to begin transaction",
                "Failed to commit transaction",
                |c| {
                    c.execute("DELETE FROM objects WHERE id = ?1", params![id])
                        .map_err(|e| format!("delete_object: {}", e))
                },
            )?;
            drop(guard);
            if affected > 0 {
                self.record_tombstone("objects", id)?;
            }
            Ok(())
        }
    }

    pub fn restore_object(&self, id: &str) -> Result<(), String> {
        // 方案 B：恢复是写变更，落库新 HLC。
        let hlc = self.new_local_hlc()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        with_tx(
            conn,
            "Failed to begin transaction",
            "Failed to commit transaction",
            |c| {
                c.execute(
                    "UPDATE objects SET is_deleted = 0, deleted_at = NULL, updated_at = ?1 WHERE id = ?2",
                    params![chrono::Utc::now().to_rfc3339(), id],
                )
                .map_err(|e| format!("restore_object: {}", e))?;
                Self::set_record_hlc_tx(c, "objects", id, &hlc)?;
                Ok(())
            },
        )
    }

    /// Count non-deleted objects for an account, optionally filtered by type or parent.
    /// Pure SQL COUNT — does not decrypt any payload (search pagination N+1 fix).
    pub fn count_objects(
        &self,
        account_id: &str,
        type_id: Option<&str>,
        parent_id: Option<&str>,
    ) -> Result<usize, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut sql =
            String::from("SELECT COUNT(*) FROM objects WHERE account_id = ?1 AND is_deleted = 0");
        let mut params: Vec<String> = vec![account_id.to_string()];
        if let Some(t) = type_id {
            params.push(t.to_string());
            sql.push_str(" AND type_id = ?");
            sql.push_str(&params.len().to_string());
        }
        if let Some(p) = parent_id {
            params.push(p.to_string());
            sql.push_str(" AND parent_id = ?");
            sql.push_str(&params.len().to_string());
        }
        let count: i64 = conn
            .query_row(&sql, rusqlite::params_from_iter(params.iter()), |r| {
                r.get(0)
            })
            .map_err(|e| format!("count_objects: {}", e))?;
        Ok(count as usize)
    }

    /// Load full object records (decrypted) for an account without query filtering.
    /// Single full-table scan shared by advanced search and template-membership expansion.
    pub fn list_object_records(&self, account_id: &str) -> Result<Vec<ObjectRecord>, String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        // properties 已加密，无法使用 SQL LIKE。所有匹配在解密后的内存数据上进行。
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {} FROM objects WHERE account_id = ?1 AND is_deleted = 0 ORDER BY updated_at DESC",
                OBJECT_COLUMNS
            ))
            .map_err(|e| format!("list_object_records: {}", e))?;
        let results = stmt
            .query_map(params![account_id], |row| {
                Self::object_row_to_record(&key, row)
            })
            .map_err(|e| format!("list_object_records query: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("list_object_records collect: {}", e))?;
        Ok(results)
    }

    /// Search objects by name or serialized properties (case-insensitive).
    /// Performs one full scan via [`Self::list_object_records`] and filters in memory.
    pub fn search_objects(
        &self,
        account_id: &str,
        query: &str,
    ) -> Result<Vec<ObjectRecord>, String> {
        let lower_query = query.to_lowercase();
        let results = self.list_object_records(account_id)?;
        // P210: properties 用 json_contains_ignore_case 递归匹配，避免整值 to_string() 往返。
        Ok(results
            .into_iter()
            .filter(|r| {
                r.name.to_lowercase().contains(&lower_query)
                    || json_contains_ignore_case(&r.properties, &lower_query)
            })
            .collect())
    }
}
