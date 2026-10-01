//! RF022：本机导入 journal / epoch 许可；完整计划只供 native/Core 消费。
//! 文件闭包只能执行受控的有限文件动作，不得回调 Vault 或长时间 hash/解密。
use super::import_batch::{connection_revision, execute_import_batch_tx, validate_batch, BatchHlc};
use crate::encryption::{decrypt_text_field, encrypt_text_field, DataEncryptionKey};
use crate::{ImportBatchRevision, ImportDatabaseBatch, ImportDatabaseCommit, Profile, VaultStore};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImportSourceKind {
    Manual,
    Cloud,
    Recovery,
    Cli,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImportOperationPhase {
    RecordsCommitted,
    Attachments,
    Preferences,
    Complete,
    Abandoned,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImportAttachmentPhase {
    Planned,
    Staged,
    Published,
    MetadataCommitted,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSourceProof {
    pub sha256: String,
    pub length: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportCiphertextProof {
    pub sha256: String,
    pub length: u64,
    pub plaintext_length: u64,
    pub stage_epoch: u64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAttachmentOwnerPlan {
    pub owner_id: String,
    pub expected_attachments: Option<Value>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAttachmentStepPlan {
    pub entry_ordinal: u32,
    pub source_object_id: String,
    pub source_attachment_id: String,
    pub owner_id: String,
    pub attachment_id: String,
    pub safe_file_name: String,
    pub metadata: Value,
    pub initial_staged_proof: Option<ImportCiphertextProof>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOperationStart {
    pub operation_id: String,
    pub source_kind: ImportSourceKind,
    pub source: ImportSourceProof,
    pub root_binding: String,
    pub request_fingerprint: String,
    pub plan: Value,
    pub owners: Vec<ImportAttachmentOwnerPlan>,
    pub steps: Vec<ImportAttachmentStepPlan>,
    pub preferences_required: bool,
    /// Recovery ready 数据在当前账户加密 plan 中；不允许包密码或派生 key。
    pub source_ready: Option<Value>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAttachmentStep {
    pub plan: ImportAttachmentStepPlan,
    pub phase: ImportAttachmentPhase,
    pub staged_proof: Option<ImportCiphertextProof>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOperationRecord {
    pub start: ImportOperationStart,
    pub phase: ImportOperationPhase,
    pub epoch: u64,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub database_commit: ImportDatabaseCommit,
    pub attachment_count: usize,
    pub preferences_imported: bool,
    pub steps: Vec<ImportAttachmentStep>,
}
#[derive(Clone, Debug)]
pub struct ImportOperationCommit {
    pub already_committed: bool,
    pub database_commit: ImportDatabaseCommit,
    pub operation: ImportOperationRecord,
}
/// 不反序列化、不从 IPC 构造；调用方每次重新解锁须重新 claim。
#[derive(Clone, Debug)]
pub struct ImportOperationLease {
    store: uuid::Uuid,
    account_id: String,
    operation_id: String,
    root_binding: String,
    epoch: u64,
}
impl ImportOperationLease {
    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    pub fn root_binding(&self) -> &str {
        &self.root_binding
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOwnedAttachmentMarker {
    pub account_id: String,
    pub operation_id: String,
    pub entry_ordinal: u32,
    pub owner_id: String,
    pub attachment_id: String,
    pub root_binding: String,
}
const INVALID: &str = "import_operation_invalid_plan";
const IDENTITY: &str = "import_operation_identity_mismatch";
const LOCKED: &str = "import_operation_locked";
const READ: &str = "import_operation_read_failed";
const WRITE: &str = "import_operation_write_failed";
const LEASE: &str = "import_operation_stale_lease";
const STATE: &str = "import_operation_invalid_phase";
fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis().max(0)
}
fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
fn valid_uuid(s: &str) -> bool {
    uuid::Uuid::parse_str(s).is_ok_and(|v| v.to_string() == s)
}
fn valid_hash(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn phase_name(p: ImportOperationPhase) -> &'static str {
    match p {
        ImportOperationPhase::RecordsCommitted => "recordsCommitted",
        ImportOperationPhase::Attachments => "attachments",
        ImportOperationPhase::Preferences => "preferences",
        ImportOperationPhase::Complete => "complete",
        ImportOperationPhase::Abandoned => "abandoned",
    }
}
fn step_phase_name(p: ImportAttachmentPhase) -> &'static str {
    match p {
        ImportAttachmentPhase::Planned => "planned",
        ImportAttachmentPhase::Staged => "staged",
        ImportAttachmentPhase::Published => "published",
        ImportAttachmentPhase::MetadataCommitted => "metadataCommitted",
    }
}
fn valid_proof(p: &ImportCiphertextProof) -> bool {
    valid_hash(&p.sha256) && p.length > 0 && p.stage_epoch <= i64::MAX as u64
}
fn attachment_entries(value: &Value) -> Result<&[Value], String> {
    let entries = value.as_array().ok_or(INVALID)?;
    let mut ids = BTreeSet::new();
    for entry in entries {
        let m = entry.as_object().ok_or(INVALID)?;
        let id = m.get("id").and_then(Value::as_str).ok_or(INVALID)?;
        if !valid_id(id) || !ids.insert(id.to_ascii_lowercase()) {
            return Err(INVALID.into());
        }
        if let Some(id) = m.get("objectId") {
            if !id.as_str().is_some_and(valid_id) {
                return Err(INVALID.into());
            }
        }
        for f in ["fileName", "mimeType", "createdAt"] {
            if m.get(f).and_then(Value::as_str).is_none() {
                return Err(INVALID.into());
            }
        }
        if m.get("sizeBytes").and_then(Value::as_u64).is_none() {
            return Err(INVALID.into());
        }
        for f in ["vaultPath", "srcPath", "deletedAt", "description"] {
            if m.get(f).is_some_and(|v| !v.is_null() && !v.is_string()) {
                return Err(INVALID.into());
            }
        }
        if m.get("tags")
            .is_some_and(|v| !v.as_array().is_some_and(|a| a.iter().all(Value::is_string)))
        {
            return Err(INVALID.into());
        }
    }
    Ok(entries)
}
fn validate_start(start: &ImportOperationStart) -> Result<(), String> {
    if !valid_uuid(&start.operation_id)
        || !valid_hash(&start.source.sha256)
        || start.source.length == 0
        || !valid_hash(&start.root_binding)
        || !valid_hash(&start.request_fingerprint)
        || !start.plan.is_object()
    {
        return Err(INVALID.into());
    }
    let mut owners = BTreeSet::new();
    for owner in &start.owners {
        if !valid_id(&owner.owner_id) || !owners.insert(owner.owner_id.clone()) {
            return Err(INVALID.into());
        }
        if let Some(v) = &owner.expected_attachments {
            attachment_entries(v)?;
        }
    }
    let mut ordinals = BTreeSet::new();
    let mut ids = BTreeSet::new();
    for step in &start.steps {
        if !ordinals.insert(step.entry_ordinal)
            || !valid_id(&step.owner_id)
            || !owners.contains(&step.owner_id)
            || !valid_id(&step.source_object_id)
            || !valid_id(&step.source_attachment_id)
            || !valid_uuid(&step.attachment_id)
            || !ids.insert(step.attachment_id.clone())
            || step.safe_file_name.is_empty()
            || step.safe_file_name.len() > 255
            || step
                .safe_file_name
                .chars()
                .any(|c| c.is_control() || matches!(c, '/' | '\\' | ':'))
        {
            return Err(INVALID.into());
        }
        let one = json!([step.metadata]);
        attachment_entries(&one)?;
        if step.metadata.get("id").and_then(Value::as_str) != Some(step.attachment_id.as_str())
            || step.metadata.get("objectId").and_then(Value::as_str) != Some(step.owner_id.as_str())
            || step.metadata.get("fileName").and_then(Value::as_str)
                != Some(step.safe_file_name.as_str())
        {
            return Err(INVALID.into());
        }
        if let Some(p) = &step.initial_staged_proof {
            if !valid_proof(p)
                || p.stage_epoch != 0
                || start.source_kind != ImportSourceKind::Recovery
            {
                return Err(INVALID.into());
            }
        }
    }
    if start.source_kind == ImportSourceKind::Recovery {
        if !start.source_ready.as_ref().is_some_and(Value::is_object)
            || start.steps.iter().any(|s| s.initial_staged_proof.is_none())
        {
            return Err(INVALID.into());
        }
    } else if start.source_ready.is_some() {
        return Err(INVALID.into());
    }
    Ok(())
}
fn encode<T: Serialize>(
    key: &DataEncryptionKey,
    account: &str,
    id: &str,
    value: &T,
) -> Result<String, String> {
    let text = serde_json::to_string(&json!({"account":account,"id":id,"payload":value}))
        .map_err(|_| INVALID)?;
    encrypt_text_field(key, &text).map_err(|_| WRITE.into())
}
fn decode<T: DeserializeOwned>(
    key: &DataEncryptionKey,
    account: &str,
    id: &str,
    text: &str,
) -> Result<T, String> {
    // 新表没有旧明文兼容；拒绝把伪造 plaintext 当有效 journal。
    if !text.starts_with("solo:") {
        return Err(READ.into());
    }
    let text = decrypt_text_field(key, text).map_err(|_| READ)?;
    let v: Value = serde_json::from_str(&text).map_err(|_| READ)?;
    if v.get("account").and_then(Value::as_str) != Some(account)
        || v.get("id").and_then(Value::as_str) != Some(id)
    {
        return Err(IDENTITY.into());
    }
    serde_json::from_value(v.get("payload").cloned().ok_or(READ)?).map_err(|_| READ.into())
}
// Immutable start in plan_enc; only small progress gets rewritten per phase/epoch.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OperationProgress {
    phase: ImportOperationPhase,
    epoch: u64,
    created_at_ms: i64,
    updated_at_ms: i64,
    database_commit: ImportDatabaseCommit,
    attachment_count: usize,
    preferences_imported: bool,
}
impl From<&ImportOperationRecord> for OperationProgress {
    fn from(op: &ImportOperationRecord) -> Self {
        Self {
            phase: op.phase,
            epoch: op.epoch,
            created_at_ms: op.created_at_ms,
            updated_at_ms: op.updated_at_ms,
            database_commit: op.database_commit.clone(),
            attachment_count: op.attachment_count,
            preferences_imported: op.preferences_imported,
        }
    }
}
pub(super) fn load_tx(
    conn: &Connection,
    key: &DataEncryptionKey,
    account: &str,
    id: &str,
) -> Result<Option<ImportOperationRecord>, String> {
    let row:Option<(String,String,u64,i64,i64,String,String)>=conn.query_row(
        "SELECT account_id,phase,epoch,created_at_ms,updated_at_ms,plan_enc,result_enc FROM import_operations WHERE operation_id=?1",params![id],
        |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))).optional().map_err(|_|READ)?;
    let Some((owner, phase, epoch, created, updated, body, result)) = row else {
        return Ok(None);
    };
    if owner != account {
        return Err(IDENTITY.into());
    }
    let start: ImportOperationStart = decode(key, account, id, &body)?;
    validate_start(&start)?;
    let progress: OperationProgress = decode(key, account, id, &result)?;
    if start.operation_id != id
        || phase_name(progress.phase) != phase
        || progress.epoch != epoch
        || progress.created_at_ms != created
        || progress.updated_at_ms != updated
    {
        return Err(READ.into());
    }
    let mut op = ImportOperationRecord {
        start,
        phase: progress.phase,
        epoch,
        created_at_ms: created,
        updated_at_ms: updated,
        database_commit: progress.database_commit,
        attachment_count: progress.attachment_count,
        preferences_imported: progress.preferences_imported,
        steps: Vec::new(),
    };
    let mut stmt=conn.prepare("SELECT entry_ordinal,phase,step_enc FROM import_attachment_steps WHERE operation_id=?1 ORDER BY entry_ordinal").map_err(|_|READ)?;
    let mut rows = stmt.query(params![id]).map_err(|_| READ)?;
    let plans: HashMap<_, _> = op
        .start
        .steps
        .iter()
        .map(|p| (p.entry_ordinal, p))
        .collect();
    while let Some(row) = rows.next().map_err(|_| READ)? {
        let ordinal: u32 = row.get(0).map_err(|_| READ)?;
        let phase: String = row.get(1).map_err(|_| READ)?;
        let enc: String = row.get(2).map_err(|_| READ)?;
        let step: ImportAttachmentStep = decode(key, account, id, &enc)?;
        if step.plan.entry_ordinal != ordinal
            || step_phase_name(step.phase) != phase
            || !plans.get(&ordinal).is_some_and(|p| **p == step.plan)
            || (step.phase == ImportAttachmentPhase::Planned) != step.staged_proof.is_none()
            || step
                .staged_proof
                .as_ref()
                .is_some_and(|p| !valid_proof(p) || p.stage_epoch > epoch)
        {
            return Err(READ.into());
        }
        op.steps.push(step);
    }
    if op.steps.len() != op.start.steps.len()
        || op.attachment_count
            != op
                .steps
                .iter()
                .filter(|s| s.phase == ImportAttachmentPhase::MetadataCommitted)
                .count()
    {
        return Err(READ.into());
    }
    Ok(Some(op))
}
fn save_tx(
    conn: &Connection,
    key: &DataEncryptionKey,
    account: &str,
    op: &ImportOperationRecord,
    expected_epoch: u64,
    original_steps: &[ImportAttachmentStep],
) -> Result<(), String> {
    let result = encode(
        key,
        account,
        &op.start.operation_id,
        &OperationProgress::from(op),
    )?;
    let changed=conn.execute("UPDATE import_operations SET phase=?1,epoch=?2,updated_at_ms=?3,result_enc=?4 WHERE operation_id=?5 AND account_id=?6 AND epoch=?7",
        params![phase_name(op.phase),op.epoch,op.updated_at_ms,result,op.start.operation_id,account,expected_epoch]).map_err(|_|WRITE)?;
    if changed != 1 {
        return Err(LEASE.into());
    }
    let original: HashMap<_, _> = original_steps
        .iter()
        .map(|step| (step.plan.entry_ordinal, step))
        .collect();
    for step in &op.steps {
        if original
            .get(&step.plan.entry_ordinal)
            .is_some_and(|old| *old == step)
        {
            continue;
        }
        let enc = encode(key, account, &op.start.operation_id, step)?;
        let changed=conn.execute("UPDATE import_attachment_steps SET phase=?1,step_enc=?2 WHERE operation_id=?3 AND entry_ordinal=?4",
            params![step_phase_name(step.phase),enc,op.start.operation_id,step.plan.entry_ordinal]).map_err(|_|WRITE)?;
        if changed != 1 {
            return Err(WRITE.into());
        }
    }
    Ok(())
}
fn insert_tx(
    conn: &Connection,
    key: &DataEncryptionKey,
    account: &str,
    op: &ImportOperationRecord,
) -> Result<(), String> {
    conn.execute("INSERT INTO import_operations(operation_id,account_id,phase,epoch,created_at_ms,updated_at_ms,plan_enc,result_enc) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
        params![op.start.operation_id,account,phase_name(op.phase),op.epoch,op.created_at_ms,op.updated_at_ms,encode(key,account,&op.start.operation_id,&op.start)?,encode(key,account,&op.start.operation_id,&OperationProgress::from(op))?]).map_err(|_|WRITE)?;
    for step in &op.steps {
        conn.execute("INSERT INTO import_attachment_steps(operation_id,entry_ordinal,phase,step_enc) VALUES(?1,?2,?3,?4)",
            params![op.start.operation_id,step.plan.entry_ordinal,step_phase_name(step.phase),encode(key,account,&op.start.operation_id,step)?]).map_err(|_|WRITE)?;
    }
    Ok(())
}
fn list_ids_tx(conn: &Connection, account: &str) -> Result<Vec<String>, String> {
    let mut stmt=conn.prepare("SELECT operation_id FROM import_operations WHERE account_id=?1 ORDER BY created_at_ms,operation_id").map_err(|_|READ)?;
    let rows = stmt
        .query_map(params![account], |r| r.get(0))
        .map_err(|_| READ)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|_| READ.into())
}
fn matches_request(op: &ImportOperationRecord, start: &ImportOperationStart) -> bool {
    op.start.source_kind == start.source_kind
        && op.start.source == start.source
        && op.start.root_binding == start.root_binding
        && op.start.request_fingerprint == start.request_fingerprint
}
fn node_id_tx(conn: &Connection) -> Result<String, String> {
    use base64::Engine as _;
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM metadata WHERE key='sync_node_id'",
            [],
            |r| r.get(0),
        )
        .ok();
    let value = value
        .map(|v| {
            base64::engine::general_purpose::STANDARD
                .decode(v)
                .map_err(|_| "import_batch_hlc_failed")
        })
        .transpose()?;
    let text = value
        .and_then(|v| String::from_utf8(v).ok())
        .unwrap_or_else(|| "unknown".into());
    Ok(VaultStore::normalize_sync_node_id(&text))
}
impl VaultStore {
    /// Native 受控原目录身份；不接受 IPC root string 构造。
    pub fn import_root_binding(&self) -> Result<String, String> {
        let root = self.config.path.parent().ok_or(IDENTITY)?;
        let canonical = std::fs::canonicalize(root).map_err(|_| IDENTITY)?;
        let native = canonical.to_str().ok_or(IDENTITY)?;
        Ok(format!("{:x}", Sha256::digest(native.as_bytes())))
    }
    fn operation_key(&self, account: &str) -> Result<DataEncryptionKey, String> {
        if account != self.config.account_id {
            return Err(IDENTITY.into());
        }
        self.data_key().map_err(|_| LOCKED.into())
    }
    pub fn commit_import_batch_with_operation(
        &self,
        account: &str,
        revision: &ImportBatchRevision,
        batch: &ImportDatabaseBatch,
        start: &ImportOperationStart,
    ) -> Result<ImportOperationCommit, String> {
        self.validate_import_identity(account, revision)
            .map_err(|e| e.to_string())?;
        validate_start(start)?;
        if start.root_binding != self.import_root_binding()? {
            return Err(IDENTITY.into());
        }
        let key = self.operation_key(account)?;
        let mut guard = self.conn.lock().map_err(|_| LOCKED)?;
        let conn = guard.as_mut().ok_or(LOCKED)?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| "import_batch_begin_failed")?;
        let mut existing = load_tx(&tx, &key, account, &start.operation_id)?;
        if existing.is_none() && start.source_kind == ImportSourceKind::Cloud {
            for id in list_ids_tx(&tx, account)? {
                let op = load_tx(&tx, &key, account, &id)?.ok_or(READ)?;
                if op.phase != ImportOperationPhase::Abandoned && matches_request(&op, start) {
                    existing = Some(op);
                    break;
                }
            }
        }
        if let Some(op) = existing {
            if !matches_request(&op, start) {
                return Err(IDENTITY.into());
            }
            if op.phase == ImportOperationPhase::Abandoned {
                return Err(STATE.into());
            }
            tx.commit().map_err(|_| "import_batch_commit_failed")?;
            return Ok(ImportOperationCommit {
                already_committed: true,
                database_commit: op.database_commit.clone(),
                operation: op,
            });
        }
        validate_batch(account, batch).map_err(|e| e.to_string())?;
        if connection_revision(&tx, self.import_instance_id, account).map_err(|e| e.to_string())?
            != *revision
        {
            return Err("import_batch_stale_view".into());
        }
        let node = if batch.templates.is_empty() && batch.objects.is_empty() {
            None
        } else {
            Some(node_id_tx(&tx)?)
        };
        let database_commit =
            execute_import_batch_tx(&tx, &key, account, batch, node).map_err(|e| e.to_string())?;
        let now = now_ms();
        let mut steps: Vec<_> = start
            .steps
            .iter()
            .map(|plan| ImportAttachmentStep {
                phase: if plan.initial_staged_proof.is_some() {
                    ImportAttachmentPhase::Staged
                } else {
                    ImportAttachmentPhase::Planned
                },
                staged_proof: plan.initial_staged_proof.clone(),
                plan: plan.clone(),
            })
            .collect();
        steps.sort_by_key(|s| s.plan.entry_ordinal);
        let op = ImportOperationRecord {
            start: start.clone(),
            phase: ImportOperationPhase::RecordsCommitted,
            epoch: 0,
            created_at_ms: now,
            updated_at_ms: now,
            database_commit: database_commit.clone(),
            attachment_count: 0,
            preferences_imported: false,
            steps,
        };
        insert_tx(&tx, &key, account, &op)?;
        tx.commit().map_err(|_| "import_batch_commit_failed")?;
        Ok(ImportOperationCommit {
            already_committed: false,
            database_commit,
            operation: op,
        })
    }
    pub fn load_import_operation(
        &self,
        account: &str,
        id: &str,
    ) -> Result<Option<ImportOperationRecord>, String> {
        if !valid_uuid(id) {
            return Err(INVALID.into());
        }
        let key = self.operation_key(account)?;
        let guard = self.conn.lock().map_err(|_| LOCKED)?;
        let conn = guard.as_ref().ok_or(LOCKED)?;
        load_tx(conn, &key, account, id)
    }
    pub fn list_import_operations(
        &self,
        account: &str,
    ) -> Result<Vec<ImportOperationRecord>, String> {
        let key = self.operation_key(account)?;
        let guard = self.conn.lock().map_err(|_| LOCKED)?;
        let conn = guard.as_ref().ok_or(LOCKED)?;
        let mut out = Vec::new();
        for id in list_ids_tx(conn, account)? {
            let op = load_tx(conn, &key, account, &id)?.ok_or(READ)?;
            if !matches!(
                op.phase,
                ImportOperationPhase::Complete | ImportOperationPhase::Abandoned
            ) {
                out.push(op)
            }
        }
        Ok(out)
    }
    pub fn find_cloud_import_operation(
        &self,
        account: &str,
        source: &ImportSourceProof,
        fingerprint: &str,
        root: &str,
    ) -> Result<Option<ImportOperationRecord>, String> {
        let key = self.operation_key(account)?;
        if root != self.import_root_binding()? {
            return Err(IDENTITY.into());
        }
        let guard = self.conn.lock().map_err(|_| LOCKED)?;
        let conn = guard.as_ref().ok_or(LOCKED)?;
        for id in list_ids_tx(conn, account)? {
            let op = load_tx(conn, &key, account, &id)?.ok_or(READ)?;
            if op.start.source_kind == ImportSourceKind::Cloud
                && op.start.source == *source
                && op.start.request_fingerprint == fingerprint
                && op.start.root_binding == root
                && op.phase != ImportOperationPhase::Abandoned
            {
                return Ok(Some(op));
            }
        }
        Ok(None)
    }
    pub fn claim_import_operation(
        &self,
        account: &str,
        id: &str,
        root: &str,
    ) -> Result<ImportOperationLease, String> {
        if root != self.import_root_binding()? {
            return Err(IDENTITY.into());
        }
        let key = self.operation_key(account)?;
        let mut guard = self.conn.lock().map_err(|_| LOCKED)?;
        let conn = guard.as_mut().ok_or(LOCKED)?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| WRITE)?;
        let mut op = load_tx(&tx, &key, account, id)?.ok_or("import_operation_not_found")?;
        if op.start.root_binding != root {
            return Err(IDENTITY.into());
        }
        if matches!(
            op.phase,
            ImportOperationPhase::Complete | ImportOperationPhase::Abandoned
        ) {
            return Err(STATE.into());
        }
        let old = op.epoch;
        op.epoch = old
            .checked_add(1)
            .filter(|e| *e <= i64::MAX as u64)
            .ok_or(LEASE)?;
        op.updated_at_ms = now_ms().max(op.updated_at_ms);
        save_tx(&tx, &key, account, &op, old, &op.steps)?;
        tx.commit().map_err(|_| WRITE)?;
        Ok(ImportOperationLease {
            store: self.import_instance_id,
            account_id: account.into(),
            operation_id: id.into(),
            root_binding: root.into(),
            epoch: op.epoch,
        })
    }
    fn with_import_lease<T>(
        &self,
        lease: &ImportOperationLease,
        action: impl FnOnce(
            &Connection,
            &DataEncryptionKey,
            &mut ImportOperationRecord,
        ) -> Result<T, String>,
    ) -> Result<T, String> {
        if lease.store != self.import_instance_id
            || lease.root_binding != self.import_root_binding()?
        {
            return Err(LEASE.into());
        }
        let key = self.operation_key(&lease.account_id)?;
        let mut guard = self.conn.lock().map_err(|_| LOCKED)?;
        let conn = guard.as_mut().ok_or(LOCKED)?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| WRITE)?;
        let mut op = load_tx(&tx, &key, &lease.account_id, &lease.operation_id)?
            .ok_or("import_operation_not_found")?;
        if op.epoch != lease.epoch || op.start.root_binding != lease.root_binding {
            return Err(LEASE.into());
        }
        if op.phase == ImportOperationPhase::Abandoned {
            return Err(STATE.into());
        }
        let original_steps = op.steps.clone();
        op.updated_at_ms = now_ms().max(op.updated_at_ms);
        let result = action(&tx, &key, &mut op)?;
        save_tx(
            &tx,
            &key,
            &lease.account_id,
            &op,
            lease.epoch,
            &original_steps,
        )?;
        tx.commit().map_err(|_| WRITE)?;
        Ok(result)
    }
    pub fn confirm_import_attachment_staged(
        &self,
        lease: &ImportOperationLease,
        ordinal: u32,
        proof: &ImportCiphertextProof,
    ) -> Result<(), String> {
        if !valid_proof(proof) || proof.stage_epoch != lease.epoch {
            return Err(LEASE.into());
        }
        self.with_import_lease(lease, |_, _, op| {
            if op.phase == ImportOperationPhase::Complete {
                return Err(STATE.into());
            }
            let step = op
                .steps
                .iter_mut()
                .find(|s| s.plan.entry_ordinal == ordinal)
                .ok_or(INVALID)?;
            if step.phase != ImportAttachmentPhase::Planned {
                return if step.staged_proof.as_ref() == Some(proof) {
                    Ok(())
                } else {
                    Err(STATE.into())
                };
            }
            step.phase = ImportAttachmentPhase::Staged;
            step.staged_proof = Some(proof.clone());
            op.phase = ImportOperationPhase::Attachments;
            Ok(())
        })
    }
    pub fn publish_import_attachment(
        &self,
        lease: &ImportOperationLease,
        ordinal: u32,
        action: impl FnOnce(&ImportAttachmentStep) -> Result<(), String>,
    ) -> Result<bool, String> {
        self.with_import_lease(lease, |_, _, op| {
            let step = op
                .steps
                .iter_mut()
                .find(|s| s.plan.entry_ordinal == ordinal)
                .ok_or(INVALID)?;
            if matches!(
                step.phase,
                ImportAttachmentPhase::Published | ImportAttachmentPhase::MetadataCommitted
            ) {
                return Ok(false);
            }
            if op.phase == ImportOperationPhase::Complete
                || step.phase != ImportAttachmentPhase::Staged
                || step.staged_proof.is_none()
            {
                return Err(STATE.into());
            }
            action(step)?;
            step.phase = ImportAttachmentPhase::Published;
            op.phase = ImportOperationPhase::Attachments;
            Ok(true)
        })
    }
    pub fn commit_import_attachment_metadata(
        &self,
        lease: &ImportOperationLease,
        owner: &str,
    ) -> Result<ImportOperationRecord, String> {
        self.with_import_lease(lease,|conn,key,op|{
            let selected:Vec<usize>=op.steps.iter().enumerate().filter_map(|(i,s)|(s.plan.owner_id==owner).then_some(i)).collect();
            if selected.is_empty(){return Err(INVALID.into())}
            if selected.iter().all(|i|op.steps[*i].phase==ImportAttachmentPhase::MetadataCommitted){return Ok(op.clone())}
            if op.phase==ImportOperationPhase::Complete||op.steps.iter().any(|s|!matches!(s.phase,ImportAttachmentPhase::Published|ImportAttachmentPhase::MetadataCommitted)){return Err(STATE.into())}
            let baseline=op.start.owners.iter().find(|o|o.owner_id==owner).ok_or(INVALID)?;
            let row:Option<(String,bool,u32,String)>=conn.query_row("SELECT account_id,is_deleted,version,properties FROM objects WHERE id=?1",params![owner],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(|_|READ)?;
            let Some((account,deleted,version,encrypted))=row else{return Err("import_operation_owner_missing".into())};
            if account!=lease.account_id||deleted{return Err("import_operation_owner_changed".into())}
            let text=decrypt_text_field(key,&encrypted).map_err(|_|READ)?;let mut props:Value=serde_json::from_str(&text).map_err(|_|READ)?;
            if props.get("__attachments").cloned()!=baseline.expected_attachments{return Err("import_operation_attachment_conflict".into())}
            let entries:Vec<Value>=selected.iter().map(|i|{let step=&op.steps[*i];let mut meta=step.plan.metadata.clone();meta["sizeBytes"]=json!(step.staged_proof.as_ref().expect("published proof validated").plaintext_length);meta}).collect();
            let values=Value::Array(entries);attachment_entries(&values)?;props.as_object_mut().ok_or(INVALID)?.insert("__attachments".into(),values);
            let new_version=version.checked_add(1).ok_or("import_operation_version_overflow")?;
            let hlc=BatchHlc::new(conn,node_id_tx(conn)?).and_then(|mut h|h.next()).map_err(|e|e.to_string())?;
            let enc=encrypt_text_field(key,&serde_json::to_string(&props).map_err(|_|INVALID)?).map_err(|_|WRITE)?;
            if conn.execute("UPDATE objects SET properties=?1,version=?2,updated_at=?3 WHERE id=?4 AND account_id=?5 AND is_deleted=0 AND version=?6",params![enc,new_version,chrono::Utc::now().to_rfc3339(),owner,lease.account_id,version]).map_err(|_|WRITE)?!=1{return Err("import_operation_owner_changed".into())}
            Self::set_record_hlc_tx(conn,"objects",owner,&hlc).map_err(|_|"import_batch_hlc_failed")?;
            for i in selected{op.steps[i].phase=ImportAttachmentPhase::MetadataCommitted;}
            op.attachment_count=op.steps.iter().filter(|s|s.phase==ImportAttachmentPhase::MetadataCommitted).count();op.phase=ImportOperationPhase::Attachments;Ok(op.clone())
        })
    }
    pub fn commit_import_preferences(
        &self,
        lease: &ImportOperationLease,
        profile: &Profile,
    ) -> Result<ImportOperationRecord, String> {
        if profile.id != lease.account_id {
            return Err(IDENTITY.into());
        }
        self.with_import_lease(lease, |conn, key, op| {
            if op.preferences_imported {
                return Ok(op.clone());
            }
            if op.phase == ImportOperationPhase::Complete
                || !op.start.preferences_required
                || op
                    .steps
                    .iter()
                    .any(|s| s.phase != ImportAttachmentPhase::MetadataCommitted)
            {
                return Err(STATE.into());
            }
            let hlc = BatchHlc::new(conn, node_id_tx(conn)?)
                .and_then(|mut h| h.next())
                .map_err(|e| e.to_string())?;
            Self::save_profile_tx(conn, key, profile)
                .map_err(|_| "import_operation_preferences_failed")?;
            Self::set_record_hlc_tx(conn, "profiles", &profile.id, &hlc)
                .map_err(|_| "import_batch_hlc_failed")?;
            op.preferences_imported = true;
            op.phase = ImportOperationPhase::Preferences;
            Ok(op.clone())
        })
    }
    pub fn complete_import_operation(
        &self,
        lease: &ImportOperationLease,
    ) -> Result<ImportOperationRecord, String> {
        self.with_import_lease(lease, |_, _, op| {
            if op
                .steps
                .iter()
                .any(|s| s.phase != ImportAttachmentPhase::MetadataCommitted)
                || (op.start.preferences_required && !op.preferences_imported)
            {
                return Err(STATE.into());
            }
            op.phase = ImportOperationPhase::Complete;
            Ok(op.clone())
        })
    }
    pub fn abandon_import_operation(&self, lease: &ImportOperationLease) -> Result<(), String> {
        self.with_import_lease(lease, |_, _, op| {
            if op.phase == ImportOperationPhase::Complete {
                return Err(STATE.into());
            }
            op.phase = ImportOperationPhase::Abandoned;
            Ok(())
        })
    }
    /// 已知 native marker 仍只是定位；未知/foreign/坏引用均不授予删除。
    pub fn with_import_orphan_delete_guard<T>(
        &self,
        account: &str,
        marker: &ImportOwnedAttachmentMarker,
        action: impl FnOnce() -> Result<T, String>,
    ) -> Result<Option<T>, String> {
        let key = self.operation_key(account)?;
        if marker.account_id != account
            || marker.root_binding != self.import_root_binding()?
            || !valid_uuid(&marker.operation_id)
            || !valid_id(&marker.owner_id)
            || !valid_uuid(&marker.attachment_id)
        {
            return Ok(None);
        }
        let mut guard = self.conn.lock().map_err(|_| LOCKED)?;
        let conn = guard.as_mut().ok_or(LOCKED)?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| WRITE)?;
        let Some(op) = load_tx(&tx, &key, account, &marker.operation_id)? else {
            return Ok(None);
        };
        let Some(step) = op.steps.iter().find(|s| {
            s.plan.entry_ordinal == marker.entry_ordinal
                && s.plan.owner_id == marker.owner_id
                && s.plan.attachment_id == marker.attachment_id
        }) else {
            return Ok(None);
        };
        if op.start.root_binding != marker.root_binding
            || !matches!(
                op.phase,
                ImportOperationPhase::Complete | ImportOperationPhase::Abandoned
            )
            || step.phase == ImportAttachmentPhase::Planned
        {
            return Ok(None);
        }
        // 新 marker 目录只查当前账户全部最新引用（含软删除），不复用旧 scanner cache。
        let mut stmt = tx
            .prepare("SELECT properties FROM objects WHERE account_id=?1")
            .map_err(|_| READ)?;
        let rows = stmt
            .query_map(params![account], |r| r.get::<_, String>(0))
            .map_err(|_| READ)?;
        for row in rows {
            let text = decrypt_text_field(&key, &row.map_err(|_| READ)?).map_err(|_| READ)?;
            let props: Value = serde_json::from_str(&text).map_err(|_| READ)?;
            let props = props.as_object().ok_or(READ)?;
            if let Some(v) = props.get("__attachments") {
                for e in attachment_entries(v)? {
                    if e["id"]
                        .as_str()
                        .is_some_and(|s| s.eq_ignore_ascii_case(&marker.attachment_id))
                    {
                        return Ok(None);
                    }
                    for f in ["vaultPath", "srcPath"] {
                        if e.get(f).and_then(Value::as_str).is_some_and(|s| {
                            s.split(['/', '\\'])
                                .filter(|v| !v.is_empty())
                                .collect::<Vec<_>>()
                                .windows(3)
                                .any(|p| {
                                    p[0].eq_ignore_ascii_case("attachments")
                                        && p[1].eq_ignore_ascii_case(&marker.owner_id)
                                        && p[2].eq_ignore_ascii_case(&marker.attachment_id)
                                })
                        }) {
                            return Ok(None);
                        }
                    }
                }
            }
        }
        drop(stmt);
        let result = action()?;
        tx.commit().map_err(|_| WRITE)?;
        Ok(Some(result))
    }
}
/// 必须在既有 reencrypt_all 同一事务中换钥，不开启新事务、不重新取 data key。
pub(super) fn reencrypt_import_operations(
    tx: &rusqlite::Transaction<'_>,
    old: &DataEncryptionKey,
    new: &DataEncryptionKey,
) -> Result<(), String> {
    // 未激活 stage/published 文件不在旧附件换钥扫描中；读 authenticated phase，坏行拒绝而非视为零。
    let identities = {
        let mut s = tx
            .prepare("SELECT account_id,operation_id FROM import_operations ORDER BY operation_id")
            .map_err(|_| READ)?;
        let rows = s
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(|_| READ)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| READ)?
    };
    for (account, id) in identities {
        let op = load_tx(tx, old, &account, &id)?.ok_or(READ)?;
        if !matches!(
            op.phase,
            ImportOperationPhase::Complete | ImportOperationPhase::Abandoned
        ) {
            return Err("IMPORT_OPERATIONS_PENDING".into());
        }
    }

    super::rewrite_table(
        tx,
        "SELECT operation_id,plan_enc,result_enc FROM import_operations",
        "UPDATE import_operations SET plan_enc=?1,result_enc=?2 WHERE operation_id=?3",
        "import_operations",
        false,
        |r| {
            let p: String = r.get(1).map_err(|_| READ)?;
            let q: String = r.get(2).map_err(|_| READ)?;
            if !p.starts_with("solo:") || !q.starts_with("solo:") {
                return Err(READ.into());
            }
            let p = decrypt_text_field(old, &p)?;
            let q = decrypt_text_field(old, &q)?;
            Ok(Some(vec![
                rusqlite::types::Value::Text(encrypt_text_field(new, &p)?),
                rusqlite::types::Value::Text(encrypt_text_field(new, &q)?),
            ]))
        },
    )?;
    // 复合主键不使用只接受首列 id 的 rewrite_table。
    let rows = {
        let mut s = tx
            .prepare("SELECT operation_id,entry_ordinal,step_enc FROM import_attachment_steps")
            .map_err(|_| READ)?;
        let rows = s
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, u32>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })
            .map_err(|_| READ)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| READ)?
    };
    for (id, ordinal, enc) in rows {
        if !enc.starts_with("solo:") {
            return Err(READ.into());
        }
        let text = decrypt_text_field(old, &enc)?;
        tx.execute("UPDATE import_attachment_steps SET step_enc=?1 WHERE operation_id=?2 AND entry_ordinal=?3",params![encrypt_text_field(new,&text)?,id,ordinal]).map_err(|_|WRITE)?;
    }
    Ok(())
}
#[cfg(test)]
mod tests;
