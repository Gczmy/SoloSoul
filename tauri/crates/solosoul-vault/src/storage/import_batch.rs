//! RF-021：一致导入准备视图与模板/对象/历史/HLC 原子批次。
//! 只读视图保留加密原始行；严格读取由调用方按旧路径触发，metadata 消费不解密对象。

use std::collections::{BTreeSet, HashMap};
use std::fmt;

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

use super::objects::{
    build_list_object_metadata_sql, build_list_objects_sql, map_object_metadata_row,
    ObjectListRowRaw, ObjectRowRaw,
};
use super::{UserTemplateRowRaw, VaultStore, OBJECT_SELECT_BASE};
use crate::{ObjectRecord, ObjectSummary, RecordHlc, UserTemplate};

/// 只能由实际 VaultStore 生成；不接受包、IPC 或持久化数据构造。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportBatchRevision {
    instance_id: uuid::Uuid,
    account_id: String,
    total_changes: u64,
    data_version: i64,
}

/// 对象/模板保持私有冻结 raw 行；未使用的坏记录不提前解密或阻断准备，不持有数据密钥。
/// Host 需按包原顺序叠加已计划对象，再调用对应严格读取。
pub struct ImportReadView {
    pub revision: ImportBatchRevision,
    raw_templates: Vec<(Option<String>, Result<UserTemplateRowRaw, String>)>,
    raw_objects: Vec<(Option<String>, Result<ObjectRowRaw, String>)>,
    raw_active_objects: Vec<(Option<String>, Result<ObjectListRowRaw, String>)>,
    object_metadata: Result<Vec<ObjectSummary>, String>,
}

impl fmt::Debug for ImportReadView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImportReadView")
            .field("revision", &self.revision)
            .field("object_count", &self.raw_objects.len())
            .field("template_count", &self.raw_templates.len())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
pub struct ImportSnapshot {
    pub id: String,
    pub timestamp_ms: i64,
    pub triggered_by: String,
    pub data: Vec<u8>,
    pub diff_summary: String,
}

#[derive(Clone, Debug)]
pub enum ImportHistoryChange {
    Keep,
    Append(Vec<ImportSnapshot>),
    Replace(Vec<ImportSnapshot>),
}

impl ImportHistoryChange {
    fn snapshots(&self) -> &[ImportSnapshot] {
        match self {
            Self::Keep => &[],
            Self::Append(snapshots) | Self::Replace(snapshots) => snapshots,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ImportObjectWrite {
    pub record: ObjectRecord,
    pub history: ImportHistoryChange,
}

#[derive(Clone, Debug, Default)]
pub struct ImportDatabaseBatch {
    /// 仅包含新增模板；复用模板通过 read view 决定，不在此覆盖已有模板。
    pub templates: Vec<UserTemplate>,
    /// 不按对象 ID 折叠，重复 ID 的记录及历史操作按顺序执行。
    pub objects: Vec<ImportObjectWrite>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImportDatabaseCommit {
    pub object_write_count: usize,
    pub object_ids: BTreeSet<String>,
    pub template_ids: BTreeSet<String>,
    /// 本批执行的快照 INSERT 数，可能含后续同 owner Replace 删除的条目。
    pub snapshot_write_count: usize,
    /// 本批创建且提交时仍存活的快照 ID，不包含保留的旧快照。
    pub snapshot_ids: BTreeSet<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportBatchError {
    AccountMismatch,
    WrongStore,
    StaleView,
    InvalidPlan,
    Locked,
    Read,
    Begin,
    Templates,
    Objects,
    Snapshots,
    Hlc,
    Commit,
}

impl fmt::Display for ImportBatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::AccountMismatch => "import_batch_account_mismatch",
            Self::WrongStore => "import_batch_wrong_store",
            Self::StaleView => "import_batch_stale_view",
            Self::InvalidPlan => "import_batch_invalid_plan",
            Self::Locked => "import_batch_locked",
            Self::Read => "import_batch_read_failed",
            Self::Begin => "import_batch_begin_failed",
            Self::Templates => "import_batch_templates_failed",
            Self::Objects => "import_batch_objects_failed",
            Self::Snapshots => "import_batch_snapshots_failed",
            Self::Hlc => "import_batch_hlc_failed",
            Self::Commit => "import_batch_commit_failed",
        })
    }
}

impl std::error::Error for ImportBatchError {}

fn connection_revision(
    conn: &Connection,
    instance_id: uuid::Uuid,
    account_id: &str,
) -> Result<ImportBatchRevision, ImportBatchError> {
    let data_version = conn
        .query_row("PRAGMA main.data_version", [], |row| row.get(0))
        .map_err(|_| ImportBatchError::Read)?;
    Ok(ImportBatchRevision {
        instance_id,
        account_id: account_id.to_string(),
        total_changes: conn.total_changes(),
        data_version,
    })
}

fn validate_batch(account_id: &str, batch: &ImportDatabaseBatch) -> Result<(), ImportBatchError> {
    let mut template_ids = BTreeSet::new();
    let mut snapshot_ids = BTreeSet::new();
    for template in &batch.templates {
        if template.account_id != account_id {
            return Err(ImportBatchError::AccountMismatch);
        }
        if template.id.is_empty() || !template_ids.insert(&template.id) {
            return Err(ImportBatchError::InvalidPlan);
        }
    }
    for write in &batch.objects {
        if write.record.account_id != account_id {
            return Err(ImportBatchError::AccountMismatch);
        }
        if write.record.id.is_empty() {
            return Err(ImportBatchError::InvalidPlan);
        }
        if !matches!(write.history, ImportHistoryChange::Keep)
            && write.history.snapshots().is_empty()
        {
            return Err(ImportBatchError::InvalidPlan);
        }
        for snapshot in write.history.snapshots() {
            if uuid::Uuid::parse_str(&snapshot.id).is_err()
                || snapshot.timestamp_ms <= 0
                || snapshot.data.is_empty()
                || !snapshot_ids.insert(&snapshot.id)
            {
                return Err(ImportBatchError::InvalidPlan);
            }
        }
    }
    Ok(())
}

/// 已有同主键不同账户不得由 UPSERT 覆盖；模板计划只能新增。
fn validate_database_owners(
    conn: &Connection,
    account_id: &str,
    batch: &ImportDatabaseBatch,
) -> Result<(), ImportBatchError> {
    for template in &batch.templates {
        let owner: Option<String> = conn
            .query_row(
                "SELECT account_id FROM user_templates WHERE id = ?1",
                params![template.id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| ImportBatchError::Templates)?;
        if let Some(owner) = owner {
            return Err(if owner == account_id {
                ImportBatchError::InvalidPlan
            } else {
                ImportBatchError::AccountMismatch
            });
        }
    }
    for write in &batch.objects {
        let owner: Option<String> = conn
            .query_row(
                "SELECT account_id FROM objects WHERE id = ?1",
                params![write.record.id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| ImportBatchError::Objects)?;
        if owner.is_some_and(|owner| owner != account_id) {
            return Err(ImportBatchError::AccountMismatch);
        }
        // 包 snapshot ID 不可授予任何旧历史行的写权限，即使本批稍后 Replace 删除它。
        for snapshot in write.history.snapshots() {
            let exists: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM object_snapshots WHERE id = ?1)",
                    params![snapshot.id],
                    |row| row.get(0),
                )
                .map_err(|_| ImportBatchError::Snapshots)?;
            if exists {
                return Err(ImportBatchError::InvalidPlan);
            }
        }
    }
    Ok(())
}

struct BatchHlc {
    node_id: String,
    last_wall_ms: u64,
    now_ms: u64,
}

impl BatchHlc {
    fn new(conn: &Connection, node_id: String) -> Result<Self, ImportBatchError> {
        let max: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(wall_time_ms), 0) FROM sync_hlc WHERE node_id = ?1",
                params![node_id],
                |row| row.get(0),
            )
            .map_err(|_| ImportBatchError::Hlc)?;
        let last_wall_ms = u64::try_from(max).map_err(|_| ImportBatchError::Hlc)?;
        let now_ms = u64::try_from(chrono::Utc::now().timestamp_millis())
            .map_err(|_| ImportBatchError::Hlc)?;
        Ok(Self {
            node_id,
            last_wall_ms,
            now_ms,
        })
    }

    fn next(&mut self) -> Result<RecordHlc, ImportBatchError> {
        let wall_time_ms = self
            .last_wall_ms
            .checked_add(1)
            .ok_or(ImportBatchError::Hlc)?
            .max(self.now_ms);
        if wall_time_ms > i64::MAX as u64 {
            return Err(ImportBatchError::Hlc);
        }
        self.last_wall_ms = wall_time_ms;
        Ok(RecordHlc {
            wall_time_ms,
            counter: 0,
            node_id: self.node_id.clone(),
        })
    }
}

impl VaultStore {
    fn validate_import_identity(
        &self,
        account_id: &str,
        revision: &ImportBatchRevision,
    ) -> Result<(), ImportBatchError> {
        if account_id != self.config.account_id || revision.account_id != account_id {
            return Err(ImportBatchError::AccountMismatch);
        }
        if revision.instance_id != self.import_instance_id {
            return Err(ImportBatchError::WrongStore);
        }
        Ok(())
    }

    pub fn read_import_view(&self, account_id: &str) -> Result<ImportReadView, ImportBatchError> {
        if account_id != self.config.account_id {
            return Err(ImportBatchError::AccountMismatch);
        }
        // 只确认 key 可用，立即 Drop 临时副本；raw 视图本身不解密/不持有 key。
        drop(self.data_key().map_err(|_| ImportBatchError::Locked)?);
        let mut guard = self.conn.lock().map_err(|_| ImportBatchError::Locked)?;
        let conn = guard.as_mut().ok_or(ImportBatchError::Locked)?;
        let before = connection_revision(conn, self.import_instance_id, account_id)?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|_| ImportBatchError::Read)?;

        let raw_objects = {
            let mut statement = tx
                .prepare(&format!(
                    "{OBJECT_SELECT_BASE} WHERE account_id = ?1 ORDER BY created_at ASC, id ASC"
                ))
                .map_err(|_| ImportBatchError::Read)?;
            let rows = statement
                .query_map(params![account_id], |row| {
                    // Blob/坏 UTF-8 ID 不匹配普通 String 主键查询，错误仍留给 strict 消费。
                    let id = row.get::<_, String>(0).ok();
                    let raw = ObjectRowRaw::from_row(row)
                        .map_err(|e| format!("Failed to load object: {e}"));
                    Ok((id, raw))
                })
                .map_err(|_| ImportBatchError::Read)?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|_| ImportBatchError::Read)?
        };

        // 原 list_objects 独有解析边界（不解析 children_ids 等），不能以 load_record 替代。
        let raw_active_objects = {
            let (sql, _) = build_list_objects_sql(account_id, None, None, false, false);
            let mut statement = tx.prepare(&sql).map_err(|_| ImportBatchError::Read)?;
            let rows = statement
                .query_map(params![account_id], |row| {
                    let id = row.get::<_, String>(0).ok();
                    let raw = ObjectListRowRaw::from_row(row)
                        .map_err(|e| format!("list_objects collect: {e}"));
                    Ok((id, raw))
                })
                .map_err(|_| ImportBatchError::Read)?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|_| ImportBatchError::Read)?
        };

        // Core SkipExisting 原 15 列 active metadata 边界：冻结结果/错误，消费时才严格返回。
        // 未使用的 typed 错误不能阻断 Host load 或 Core Overwrite/Merge 的准备。
        let object_metadata = (|| -> Result<Vec<ObjectSummary>, String> {
            let (sql, _) =
                build_list_object_metadata_sql(account_id, None, None, false, false, false);
            let mut statement = tx
                .prepare(&sql)
                .map_err(|e| format!("list_object_metadata: {e}"))?;
            let rows = statement
                .query_map(params![account_id], |row| {
                    map_object_metadata_row(row, false)
                })
                .map_err(|e| format!("list_object_metadata query: {e}"))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| format!("list_object_metadata collect: {e}"))
        })();

        let raw_templates = {
            let mut statement = tx
                .prepare(
                    "SELECT id, account_id, name, icon_id, properties_json, category,
                            contract_type_id, created_at, updated_at
                     FROM user_templates WHERE account_id = ?1 ORDER BY created_at ASC",
                )
                .map_err(|_| ImportBatchError::Read)?;
            let rows = statement
                .query_map(params![account_id], |row| {
                    // 坏 ID 不可匹配正常字符串查询，但该行仍留在 strict list 中返回错误。
                    let id = row.get::<_, String>(0).ok();
                    let raw = UserTemplateRowRaw::from_row(row)
                        .map_err(|error| format!("list_user_templates row: {error}"));
                    Ok((id, raw))
                })
                .map_err(|_| ImportBatchError::Read)?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|_| ImportBatchError::Read)?
        };
        tx.commit().map_err(|_| ImportBatchError::Read)?;
        // data_version 对外部连接的观察需在只读事务结束后重新读取。
        let after = connection_revision(conn, self.import_instance_id, account_id)?;
        if before != after {
            return Err(ImportBatchError::StaleView);
        }
        Ok(ImportReadView {
            revision: after,
            raw_templates,
            raw_objects,
            raw_active_objects,
            object_metadata,
        })
    }

    /// 新模板 accessor 使用当前调用的临时密钥；校验归属和连接仍解锁。
    /// 锁外解析不保证撤销已进入的同步读取，最终写入仍必须验证原会话/revision。
    fn import_template_read_key(
        &self,
        view: &ImportReadView,
    ) -> Result<crate::encryption::DataEncryptionKey, String> {
        self.validate_import_identity(&view.revision.account_id, &view.revision)
            .map_err(|error| error.to_string())?;
        let key = self
            .data_key()
            .map_err(|_| ImportBatchError::Locked.to_string())?;
        let guard = self
            .conn
            .lock()
            .map_err(|_| ImportBatchError::Locked.to_string())?;
        if guard.is_none() {
            return Err(ImportBatchError::Locked.to_string());
        }
        drop(guard);
        Ok(key)
    }

    /// 与旧 list_user_templates 相同账户/created_at ASC/strict parser；仅调用时解密。
    /// 原始行或解析错误都来自同一 view，不改读后来修复/新增/删除的 live 记录。
    pub fn list_import_view_user_templates(
        &self,
        view: &ImportReadView,
    ) -> Result<Vec<UserTemplate>, String> {
        let key = self.import_template_read_key(view)?;
        view.raw_templates
            .iter()
            .map(|(_, raw)| {
                raw.clone()?
                    .into_template(&key)
                    .map_err(|error| format!("list_user_templates row: {error}"))
            })
            .collect()
    }

    /// 与 frozen 所属账户的主键读取相同；未匹配返回 None，坏匹配模板严格返回 Err。
    /// Host 兼容继承是否吞错由 Host 明确决定；存储 API 不降级为空。
    pub fn load_import_view_user_template(
        &self,
        view: &ImportReadView,
        id: &str,
    ) -> Result<Option<UserTemplate>, String> {
        let key = self.import_template_read_key(view)?;
        view.raw_templates
            .iter()
            .find(|(row_id, _)| row_id.as_deref() == Some(id))
            .map(|(_, raw)| {
                raw.clone()?
                    .into_template(&key)
                    .map_err(|error| format!("load_user_template: {error}"))
            })
            .transpose()
    }

    /// 等价于 Host 原 load_object 的严格解析，但读取准备时冻结的行。
    pub fn load_import_view_object(
        &self,
        view: &ImportReadView,
        id: &str,
    ) -> Result<Option<ObjectRecord>, String> {
        self.validate_import_identity(&view.revision.account_id, &view.revision)
            .map_err(|e| e.to_string())?;
        let key = self.data_key()?;
        view.raw_objects
            .iter()
            .find(|(row_id, _)| row_id.as_deref() == Some(id))
            .map(|(_, raw)| {
                raw.clone()?
                    .into_record(&key)
                    .map_err(|e| format!("Failed to load object: {e}"))
            })
            .transpose()
    }

    /// 等价于 Host unique_object_name 的 active list 严格解析。
    /// 已被前序计划覆盖的 ID 必须排除，并由 Host 加回新的 shadow record。
    pub fn list_import_view_active_objects(
        &self,
        view: &ImportReadView,
        shadowed_ids: &BTreeSet<String>,
    ) -> Result<Vec<ObjectSummary>, String> {
        self.validate_import_identity(&view.revision.account_id, &view.revision)
            .map_err(|e| e.to_string())?;
        let key = self.data_key()?;
        view.raw_active_objects
            .iter()
            .filter(|(id, _)| match id {
                Some(id) => !shadowed_ids.contains(id),
                None => true,
            })
            .map(|(_, raw)| {
                raw.clone()?
                    .into_summary(&key)
                    .map_err(|e| format!("list_objects decrypt: {e}"))
            })
            .collect()
    }

    /// 等价于 Core SkipExisting 原 active list_object_metadata（不带 tags）的严格边界。
    /// SQL 结果或 typed 错误来自同一 frozen view，不解密负载，不改读 live 修复结果。
    pub fn list_import_view_object_metadata(
        &self,
        view: &ImportReadView,
    ) -> Result<Vec<ObjectSummary>, String> {
        self.validate_import_identity(&view.revision.account_id, &view.revision)
            .map_err(|e| e.to_string())?;
        let guard = self
            .conn
            .lock()
            .map_err(|_| ImportBatchError::Locked.to_string())?;
        if guard.is_none() {
            return Err(ImportBatchError::Locked.to_string());
        }
        drop(guard);
        view.object_metadata.clone()
    }

    pub fn commit_import_batch(
        &self,
        account_id: &str,
        revision: &ImportBatchRevision,
        batch: &ImportDatabaseBatch,
    ) -> Result<ImportDatabaseCommit, ImportBatchError> {
        self.validate_import_identity(account_id, revision)?;
        validate_batch(account_id, batch)?;
        let key = self.data_key().map_err(|_| ImportBatchError::Locked)?;
        // 不在持 conn Mutex 时调用 public HLC/metadata getter。
        let node_id = if batch.templates.is_empty() && batch.objects.is_empty() {
            None
        } else {
            let node_id = self
                .get_sync_node_id()
                .map_err(|_| ImportBatchError::Hlc)?
                .unwrap_or_else(|| "unknown".to_string());
            Some(Self::normalize_sync_node_id(&node_id))
        };
        let mut guard = self.conn.lock().map_err(|_| ImportBatchError::Locked)?;
        let conn = guard.as_mut().ok_or(ImportBatchError::Locked)?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| ImportBatchError::Begin)?;
        if connection_revision(&tx, self.import_instance_id, account_id)? != *revision {
            return Err(ImportBatchError::StaleView);
        }
        validate_database_owners(&tx, account_id, batch)?;
        // 空选择/全部跳过保持原无 HLC 读取边界，但仍校验原 Store、会话和 revision。
        let Some(node_id) = node_id else {
            tx.commit().map_err(|_| ImportBatchError::Commit)?;
            return Ok(ImportDatabaseCommit::default());
        };
        let mut hlc = BatchHlc::new(&tx, node_id)?;
        let mut committed = ImportDatabaseCommit::default();
        for template in &batch.templates {
            Self::save_user_template_tx(&tx, &key, template)
                .map_err(|_| ImportBatchError::Templates)?;
            Self::set_record_hlc_tx(&tx, "user_templates", &template.id, &hlc.next()?)
                .map_err(|_| ImportBatchError::Hlc)?;
            committed.template_ids.insert(template.id.clone());
        }
        // 只读取本批对象实际引用的模板；无引用的坏 name 不得新增导入失败。
        // SQL 本身失败明确回滚，单行 name 类型/UTF-8 错沿用旧 save_object_tx 的无 fallback。
        let template_names = {
            let referenced: BTreeSet<String> = batch
                .objects
                .iter()
                .filter_map(|write| write.record.template_id.clone())
                .collect();
            let mut names = HashMap::new();
            if !referenced.is_empty() {
                let mut statement = tx
                    .prepare_cached(
                        "SELECT name FROM user_templates WHERE account_id = ?1 AND id = ?2",
                    )
                    .map_err(|_| ImportBatchError::Templates)?;
                for id in referenced {
                    let name =
                        statement.query_row(params![account_id, id], |row| row.get::<_, String>(0));
                    match name {
                        Ok(name) => {
                            names.insert(id, name);
                        }
                        Err(rusqlite::Error::QueryReturnedNoRows)
                        | Err(rusqlite::Error::InvalidColumnType(_, _, _))
                        | Err(rusqlite::Error::FromSqlConversionFailure(_, _, _)) => {}
                        Err(_) => return Err(ImportBatchError::Templates),
                    }
                }
            }
            names
        };
        let mut created_snapshot_ids = BTreeSet::new();
        for write in &batch.objects {
            Self::save_object_tx(&tx, &key, &write.record, Some(&template_names))
                .map_err(|_| ImportBatchError::Objects)?;
            Self::set_record_hlc_tx(&tx, "objects", &write.record.id, &hlc.next()?)
                .map_err(|_| ImportBatchError::Hlc)?;
            if matches!(write.history, ImportHistoryChange::Replace(_)) {
                Self::delete_snapshots_tx(&tx, &write.record.id)
                    .map_err(|_| ImportBatchError::Snapshots)?;
            }
            for snapshot in write.history.snapshots() {
                Self::save_snapshot_at_tx(
                    &tx,
                    &key,
                    &snapshot.id,
                    &write.record.id,
                    &snapshot.triggered_by,
                    &snapshot.data,
                    &snapshot.diff_summary,
                    snapshot.timestamp_ms,
                )
                .map_err(|_| ImportBatchError::Snapshots)?;
                created_snapshot_ids.insert(snapshot.id.clone());
                committed.snapshot_write_count += 1;
            }
            committed.object_write_count += 1;
            committed.object_ids.insert(write.record.id.clone());
        }
        for id in created_snapshot_ids {
            let exists: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM object_snapshots WHERE id = ?1)",
                    params![id],
                    |row| row.get(0),
                )
                .map_err(|_| ImportBatchError::Snapshots)?;
            if exists {
                committed.snapshot_ids.insert(id);
            }
        }
        // Transaction::commit 失败或任何早退由 Drop 回滚整个批次。
        tx.commit().map_err(|_| ImportBatchError::Commit)?;
        Ok(committed)
    }
}

#[cfg(test)]
mod tests;
