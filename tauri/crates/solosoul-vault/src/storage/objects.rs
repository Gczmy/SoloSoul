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
    json_contains_ignore_case, object_field_sensitivity_levels, object_has_attachments, with_tx,
    LockHoldObserver, VaultStore, OBJECT_COLUMNS, OBJECT_LOAD_SQL, OBJECT_SAVE_SQL,
    OBJECT_SELECT_BASE,
};
use crate::encryption::{decrypt_text_field, encrypt_text_field, DataEncryptionKey};
use crate::{ObjectRecord, ObjectSummary};

/// 构建 `list_objects` 的查询 SQL 与绑定参数（P012 拆分：原内联 47 行抽出）。
///
/// 语义与旧实现逐字节一致：account_id 固定 ?1，type_id/parent_id 可选按序占位，
/// is_deleted 过滤与 ORDER BY 尾缀原样拼接。
fn build_list_objects_sql(
    account_id: &str,
    type_id: Option<&str>,
    parent_id: Option<&str>,
    include_deleted: bool,
    only_deleted: bool,
) -> (String, Vec<Box<dyn rusqlite::types::ToSql>>) {
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

    sql.push_str(" ORDER BY created_at ASC, id ASC");
    (sql, param_values)
}

/// `list_objects` 的行映射闭包（P012 拆分：原内联 ~85 行解密组装抽出）。
///
/// 输入行：0..17 为 OBJECT_COLUMNS 顺序（id/name/type_id/section_type/sensitivity_level/
/// created_at/updated_at/is_deleted/properties/tags_json/template_id/template_type/
/// contract_type_id/template_hash/ignored_template_hash/icon_name/property_labels/parent_id）。
fn map_object_list_row(
    key: &DataEncryptionKey,
    row: &rusqlite::Row<'_>,
) -> Result<ObjectSummary, rusqlite::Error> {
    let deleted_int: i32 = row.get(7)?;
    let props_str: String = row.get(8)?;
    let tags_str: String = row.get(9)?;
    let decrypted_props = decrypt_text_field(key, &props_str).map_err(|e| {
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
        decrypt_text_field(key, &labels_str).map_err(|e| {
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
    let properties: serde_json::Value = serde_json::from_str(&decrypted_props).map_err(|e| {
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
    // 字段级敏感度集合（property_labels / __fields / dynamic_group 推导，升序去重）
    let sensitivity_levels = object_field_sensitivity_levels(property_labels.as_ref(), &properties);
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
        sensitivity_levels,
    })
}

/// P025 Phase 1: 行 → 原始列数据（不解密、不解析 JSON），两阶段读模式的中间形态。
///
/// 持锁阶段仅做 `row.get()` 装箱（微秒级），AES-GCM 解密 + JSON 解析在锁外进行，
/// 缩短对全库 `conn` 互斥锁的占用。`from_row` 供 `query_map` 闭包使用，
/// `into_record` 承载原 `object_row_to_record` 的解密/解析逻辑（错误语义逐字保留：
/// P225 统一 Object 前缀文案、P005 properties 损坏拒绝静默降级为空对象）。
struct ObjectRowRaw {
    id: String,
    account_id: String,
    type_id: String,
    section_type: String,
    name: String,
    icon_name: String,
    parent_id: Option<String>,
    children_ids: String,
    properties: String,
    property_labels: String,
    sensitivity_level: String,
    is_deleted: i32,
    deleted_at: Option<String>,
    tags_json: String,
    template_id: Option<String>,
    template_type: Option<String>,
    contract_type_id: Option<String>,
    template_hash: Option<String>,
    ignored_template_hash: Option<String>,
    created_at: String,
    updated_at: String,
    version: u32,
}

impl ObjectRowRaw {
    /// 仅按 OBJECT_COLUMNS 列序装箱（0..21），不触碰加密内容。
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            account_id: row.get(1)?,
            type_id: row.get(2)?,
            section_type: row.get(3)?,
            name: row.get(4)?,
            icon_name: row.get(5)?,
            parent_id: row.get(6)?,
            children_ids: row.get(7)?,
            properties: row.get(8)?,
            property_labels: row.get(9)?,
            sensitivity_level: row.get(10)?,
            is_deleted: row.get(11)?,
            deleted_at: row.get(12)?,
            tags_json: row.get(13)?,
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

    /// 锁外解密 + JSON 解析为 `ObjectRecord`。
    ///
    /// 原 `object_row_to_record` 逻辑逐字搬入：错误列索引、文案、P005 日志均不变。
    fn into_record(self, key: &DataEncryptionKey) -> rusqlite::Result<ObjectRecord> {
        let decrypted_props = decrypt_text_field(key, &self.properties).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                8,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Object properties decryption failed: {}", e),
                )),
            )
        })?;
        let decrypted_labels = if self.property_labels.is_empty() {
            Ok(String::new())
        } else {
            decrypt_text_field(key, &self.property_labels)
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
            id: self.id.clone(),
            account_id: self.account_id.clone(),
            type_id: self.type_id.clone(),
            section_type: self.section_type.clone(),
            name: self.name.clone(),
            icon_name: self.icon_name.clone(),
            parent_id: self.parent_id.clone(),
            children_ids: serde_json::from_str(&self.children_ids).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    7,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Object children_ids corrupted: {}", e),
                    )),
                )
            })?,
            properties: serde_json::from_str(&decrypted_props).map_err(|e| {
                tracing::error!(
                    id = %self.id,
                    name = %self.name,
                    error = %e,
                    "P005: 对象 properties JSON 反序列化失败，拒绝静默降级为空对象"
                );
                rusqlite::Error::FromSqlConversionFailure(
                    8,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Object properties corrupted (id={}): {}", self.id, e),
                    )),
                )
            })?,
            property_labels: if decrypted_labels.is_empty() {
                None
            } else {
                Some(serde_json::from_str(&decrypted_labels).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        9,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Object property_labels corrupted: {}", e),
                        )),
                    )
                })?)
            },
            sensitivity_level: self.sensitivity_level.clone(),
            is_deleted: self.is_deleted != 0,
            deleted_at: self.deleted_at.clone(),
            tags_json: serde_json::from_str(&self.tags_json).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    13,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Object tags_json corrupted: {}", e),
                    )),
                )
            })?,
            template_id: self.template_id.clone(),
            template_type: self.template_type.clone(),
            contract_type_id: self.contract_type_id.clone(),
            template_hash: self.template_hash.clone(),
            ignored_template_hash: self.ignored_template_hash.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            version: self.version,
        })
    }
}

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
                Self::save_object_tx(c, &key, obj, None)?;
                Self::set_record_hlc_tx(c, "objects", &obj.id, &hlc)?;
                Ok(())
            },
        )
    }

    /// P212: 单事务批量保存对象（导入等批量场景），替代逐条 `save_object` 的
    /// N 次 auto-commit 写事务。任一条失败整体回滚，不产生半导入。
    /// 方案 B：每个对象在写事务内同时落库独立 HLC（new_local_hlc 递增保证唯一）。
    /// P024: 模板名 map 批内一次性加载（N 次逐对象 SELECT 降为 1 次）。
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
                let template_names = Self::load_template_name_map(c);
                for (obj, hlc) in objects.iter().zip(hlcs.iter()) {
                    Self::save_object_tx(c, &key, obj, template_names.as_ref())?;
                    Self::set_record_hlc_tx(c, "objects", &obj.id, hlc)?;
                }
                Ok(())
            },
        )
    }

    /// P024: 一次性加载全部模板 id→name 映射（批量保存路径复用，消除
    /// 逐对象 `SELECT name FROM user_templates` 的 N 次查询）。
    fn load_template_name_map(
        conn: &mut Connection,
    ) -> Option<std::collections::HashMap<String, String>> {
        let mut stmt = conn.prepare("SELECT id, name FROM user_templates").ok()?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .ok()?;
        let map: Result<std::collections::HashMap<_, _>, _> = rows.collect();
        map.ok()
    }

    /// P115: 事务内保存对象（连接由调用方持有，批量应用单事务内复用）。
    /// P024: `template_names` 为可选预加载映射——`Some` 时直接查表（批量路径），
    /// `None` 时回退逐对象查询（单条保存与同步应用路径，行为不变）。
    pub(crate) fn save_object_tx(
        conn: &mut Connection,
        key: &DataEncryptionKey,
        obj: &ObjectRecord,
        template_names: Option<&std::collections::HashMap<String, String>>,
    ) -> Result<(), String> {
        // 保存模板名称到 properties，用于模板被删除后仍能显示原始模板名
        let mut properties = obj.properties.clone();
        if let Some(ref tid) = obj.template_id {
            let tpl_name: Option<String> = match template_names {
                Some(map) => map.get(tid).cloned(),
                None => conn
                    .query_row(
                        "SELECT name FROM user_templates WHERE id = ?1",
                        params![tid],
                        |row| row.get(0),
                    )
                    .ok(),
            };
            if let Some(name) = tpl_name {
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
    /// P025 Phase 1: 委托 `ObjectRowRaw`（from_row 装箱 + into_record 锁外解密），错误语义不变。
    fn object_row_to_record(
        key: &DataEncryptionKey,
        row: &rusqlite::Row,
    ) -> rusqlite::Result<ObjectRecord> {
        let raw = ObjectRowRaw::from_row(row)?;
        raw.into_record(key)
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
        let mut _obs = LockHoldObserver::begin("load_objects_batch");
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        _obs.acquired();
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
            // P025: 子串扫描提取 `__attachments` 段（免全量 JSON 树构造）
            let att_ids = Self::extract_attachment_ids_from_json_text(&decrypted);
            result.push((id, att_ids));
        }
        Ok(result)
    }

    /// P025: 从解密后的 properties JSON 文本中提取 `__attachments` 数组（完整条目），
    /// 避免 serde_json 构造完整对象属性树（大对象反复全量解析）。
    /// `"__attachments":` 是 JSON 键语法，字符串值不可能紧随冒号，故可唯一定位。
    /// 括号配平时正确处理字符串字面量内的转义字符。
    /// P025 复核补充：
    /// - 字符串值内可含转义引号形态 `\"__attachments\":`（marker 前置 `\`），
    ///   此类命中位于字符串内部而非真实键，需跳过并继续向后搜索；
    /// - 首个候选解析失败（括号不配平/非法 JSON）时不再 `unwrap_or_default` 直接放弃，
    ///   而是继续搜索后续 marker，避免真实附件条目被静默丢弃。
    ///
    /// P006: 原 id 提取逻辑抽出完整数组版，id 版与轻量计数（count_active_attachment_stats）
    /// 共用同一扫描语义，避免两处重复。
    pub(crate) fn extract_attachments_array_from_json_text(text: &str) -> Vec<serde_json::Value> {
        let marker = "\"__attachments\":";
        let mut search_from = 0usize;
        while let Some(rel) = text[search_from..].find(marker) {
            let pos = search_from + rel;
            search_from = pos + marker.len();
            // 跳过转义引号形态：marker 起始引号前有奇数个反斜杠 → 位于字符串值内
            let backslashes = text[..pos]
                .bytes()
                .rev()
                .take_while(|&b| b == b'\\')
                .count();
            if backslashes % 2 == 1 {
                continue;
            }
            let rest = &text[pos + marker.len()..];
            let rest = rest.trim_start();
            let Some(bracket) = rest.find('[') else {
                continue;
            };
            let bytes = &rest.as_bytes()[bracket..];
            let mut depth = 0i32;
            let mut in_str = false;
            let mut escaped = false;
            let mut end = 0usize;
            for (i, &b) in bytes.iter().enumerate() {
                if in_str {
                    if escaped {
                        escaped = false;
                    } else if b == b'\\' {
                        escaped = true;
                    } else if b == b'"' {
                        in_str = false;
                    }
                } else {
                    match b {
                        b'"' => in_str = true,
                        b'[' => depth += 1,
                        b']' => {
                            depth -= 1;
                            if depth == 0 {
                                end = i + 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            }
            if end == 0 {
                continue;
            }
            let segment = &rest[bracket..bracket + end];
            if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(segment) {
                return arr;
            }
            // 段解析失败（括号不配平/非法 JSON）→ 继续搜索下一个候选
        }
        Vec::new()
    }

    /// P025: 从解密后的 properties JSON 文本中仅提取 `__attachments` 数组段的 id 列表
    /// （基于 `extract_attachments_array_from_json_text` 的便捷视图）。
    pub(crate) fn extract_attachment_ids_from_json_text(text: &str) -> Vec<String> {
        Self::extract_attachments_array_from_json_text(text)
            .iter()
            .filter_map(|a| a.get("id").and_then(|i| i.as_str()).map(String::from))
            .collect()
    }

    /// P006: 轻量统计活跃附件总数与照片数（免构建完整附件树/文件存在性探测）。
    /// 单次 SQL 全表解密 + P025 子串扫描，仅返回两个计数，供首页角标等轻量场景。
    /// 照片判定与前端 `previewItemByMime`（attachmentUtils.ts）对齐：
    /// mimeType 以 `image/` 开头，或扩展名 ∈ {png,jpg,jpeg,gif,webp,svg}。
    /// 返回 (附件总数, 照片数)。
    pub fn count_active_attachment_stats(
        &self,
        account_id: &str,
    ) -> Result<(usize, usize), String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn
            .prepare("SELECT properties FROM objects WHERE account_id = ?1 AND is_deleted = 0")
            .map_err(|e| format!("count_active_attachment_stats: {}", e))?;
        let rows = stmt
            .query_map(params![account_id], |row| {
                let props_str: String = row.get(0)?;
                let decrypted = decrypt_text_field(&key, &props_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Object properties decryption failed: {}", e),
                        )),
                    )
                })?;
                Ok(decrypted)
            })
            .map_err(|e| format!("count_active_attachment_stats: {}", e))?;

        let mut total = 0usize;
        let mut photos = 0usize;
        for row in rows {
            let decrypted = row.map_err(|e| format!("count_active_attachment_stats: {}", e))?;
            for att in Self::extract_attachments_array_from_json_text(&decrypted) {
                // 仅统计活跃附件（与 build_attachment_tree_pages only_deleted=false 语义一致）
                if att.get("deletedAt").and_then(|v| v.as_str()).is_some() {
                    continue;
                }
                total += 1;
                let mime = att.get("mimeType").and_then(|v| v.as_str()).unwrap_or("");
                let is_image_mime = mime.starts_with("image/");
                let ext = att
                    .get("fileName")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .rsplit('.')
                    .next()
                    .unwrap_or("")
                    .to_lowercase();
                if is_image_mime
                    || matches!(
                        ext.as_str(),
                        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg"
                    )
                {
                    photos += 1;
                }
            }
        }
        Ok((total, photos))
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
        let mut _obs = LockHoldObserver::begin("list_objects");
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        _obs.acquired();
        let conn = guard.as_mut().ok_or("Vault is locked")?;

        let lower_kw = keyword.map(|k| k.to_lowercase());
        let (sql, param_values) = build_list_objects_sql(
            account_id,
            type_id,
            parent_id,
            include_deleted,
            only_deleted,
        );

        // properties 已加密，无法使用 SQL LIKE。所有 keyword 匹配在解密后的内存数据上进行。

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("list_objects: {}", e))?;

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let objects = stmt
            .query_map(params_refs.as_slice(), |row: &rusqlite::Row<'_>| {
                map_object_list_row(&key, row)
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

    /// P111: metadata-only listing —— 不 SELECT/解密 `properties`/`property_labels`。
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
        self.list_object_metadata_impl(
            account_id,
            type_id,
            parent_id,
            include_deleted,
            only_deleted,
            false,
        )
    }

    /// P003: 同 `list_object_metadata`，但额外 SELECT 明文 `tags_json` 列并填充 `tags`。
    ///
    /// tags_json 在存储层为明文（未加密，见 save_object），因此此变体仍不触碰加密的
    /// `properties`/`property_labels` 列——适用于「按页面/标签筛 id 后单独批量加载」的
    /// 调用方（如导出范围收集 `collect_scope_objects`），消除全表解密。
    pub fn list_object_metadata_with_tags(
        &self,
        account_id: &str,
        type_id: Option<&str>,
        parent_id: Option<&str>,
        include_deleted: bool,
        only_deleted: bool,
    ) -> Result<Vec<ObjectSummary>, String> {
        self.list_object_metadata_impl(
            account_id,
            type_id,
            parent_id,
            include_deleted,
            only_deleted,
            true,
        )
    }

    /// metadata-only 列表的共享实现。`with_tags` 决定是否 SELECT 并解析明文 tags_json。
    fn list_object_metadata_impl(
        &self,
        account_id: &str,
        type_id: Option<&str>,
        parent_id: Option<&str>,
        include_deleted: bool,
        only_deleted: bool,
        with_tags: bool,
    ) -> Result<Vec<ObjectSummary>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;

        let mut sql = String::from(
            "SELECT id, name, type_id, section_type, sensitivity_level, created_at, updated_at, is_deleted, template_id, template_type, contract_type_id, template_hash, ignored_template_hash, icon_name, parent_id",
        );
        if with_tags {
            sql.push_str(", tags_json");
        }
        sql.push_str(" FROM objects WHERE account_id = ?1");
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
                let tags = if with_tags {
                    let tags_str: String = row.get(15).unwrap_or_default();
                    serde_json::from_str(&tags_str).unwrap_or_default()
                } else {
                    vec![]
                };
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
                    tags,
                    // metadata-only 路径不解密 properties，附件存在性不可知 → false。
                    has_attachments: false,
                    // metadata-only 路径同理：字段敏感度集合不可知 → 空数组。
                    sensitivity_levels: vec![],
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

    /// P003: 按 id 列表求加密 properties 列字节数总和（纯 SQL SUM(LENGTH())，不解密）。
    ///
    /// 参照 `snapshots_size_batch` 先例，供导出估算（`export_estimate_size`）在不解密
    /// 任何对象负载的情况下估算 payload 体积。properties 为 AES-GCM 密文 + JSON，
    /// 长度略大于明文；调用方按需自行加 base64/头部膨胀系数。
    pub fn objects_size_batch(&self, object_ids: &[String]) -> Result<u64, String> {
        if object_ids.is_empty() {
            return Ok(0);
        }
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let placeholders = std::iter::repeat_n("?", object_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT COALESCE(SUM(LENGTH(properties)), 0) FROM objects WHERE id IN ({})",
            placeholders
        );
        let total: i64 = conn
            .query_row(
                &sql,
                rusqlite::params_from_iter(object_ids.iter().map(|s| s.as_str())),
                |r| r.get(0),
            )
            .map_err(|e| format!("objects_size_batch: {}", e))?;
        Ok(total as u64)
    }

    /// Load full object records (decrypted) for an account without query filtering.
    /// Single full-table scan shared by advanced search and template-membership expansion.
    pub fn list_object_records(&self, account_id: &str) -> Result<Vec<ObjectRecord>, String> {
        let key = self.data_key()?;
        let mut _obs = LockHoldObserver::begin("list_object_records");
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        _obs.acquired();
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        // properties 已加密，无法使用 SQL LIKE。所有匹配在解密后的内存数据上进行。
        // P025 Phase 1: 两阶段读 —— 持锁阶段仅取列装箱（不解密），
        // 释放锁后再统一 AES 解密 + JSON 解析，缩短对全库 conn 锁的占用。
        // stmt 借自 conn（借自 guard），故收在块内先于 guard 释放。
        let raws = {
            let mut stmt = conn
                .prepare(&format!(
                    "SELECT {} FROM objects WHERE account_id = ?1 AND is_deleted = 0 ORDER BY updated_at DESC",
                    OBJECT_COLUMNS
                ))
                .map_err(|e| format!("list_object_records: {}", e))?;
            // 拆成两条语句：rows 先于 stmt drop（逆声明序），避免块末临时值借用残留。
            let rows = stmt
                .query_map(params![account_id], ObjectRowRaw::from_row)
                .map_err(|e| format!("list_object_records query: {}", e))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("list_object_records collect: {}", e))?
        };
        // 观测结束：hold 现仅覆盖 SQL 取数（不含锁外解密），与改造前基线对比即收益。
        drop(guard);
        drop(_obs);
        // 锁外阶段：解密 + JSON 解析，错误语义与 object_row_to_record 逐字一致。
        let results = raws
            .into_iter()
            .map(|raw| raw.into_record(&key))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("list_object_records decrypt: {}", e))?;
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
