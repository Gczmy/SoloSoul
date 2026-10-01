//! RF903：全部本机可恢复引用的严格视图；不使用 UI 分页或宽松空集合回退。
//! SQL cursor 逐行读取，单行超限即拒绝清理。文件 action 只能作有限文件操作，不能重入 Vault。
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use base64::Engine as _;
use rusqlite::{Connection, Row, TransactionBehavior};
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Number, Value};
use sha2::{Digest, Sha256};

use super::{import_operations, VaultStore, OBJECT_SELECT_BASE};
use crate::encryption::{decrypt_field, is_encrypted_blob, DataEncryptionKey};
use crate::root_owner::VaultRootOwner;
use crate::ImportOperationPhase;

const INVALID: &str = "attachment_cleanup_reference_invalid";
const LIMIT: &str = "attachment_cleanup_reference_too_large";
const CHANGED: &str = "attachment_cleanup_view_changed";
const LOCKED: &str = "attachment_cleanup_locked";
const ACCOUNT: &str = "attachment_cleanup_account_mismatch";
const PATH: &str = "attachment_cleanup_path_changed";
const UNAVAILABLE: &str = "attachment_cleanup_references_unavailable";
const MAX_ROW_BYTES: usize = 16 * 1024 * 1024;

/// 仅用作保守引用匹配；这些路径字符串不能授权文件动作。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AttachmentReferenceCandidate {
    pub storage_object_id: String,
    pub attachment_id: String,
    pub canonical_path: PathBuf,
    pub literal_aliases: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AttachmentReferenceStats {
    pub object_rows: u64,
    pub trash_rows: u64,
    pub snapshot_rows: u64,
    pub profile_rows: u64,
    pub conflict_rows: u64,
    pub journal_rows: u64,
}

/// Native capability，不序列化、不输出恢复正文或 key witness；不能跨 Store/新解锁复用。
#[derive(Clone)]
pub struct AttachmentReferenceView {
    account: String,
    store: uuid::Uuid,
    owner: Arc<VaultRootOwner>,
    key_witness: [u8; 32],
    digest: [u8; 32],
    stats: AttachmentReferenceStats,
}
impl AttachmentReferenceView {
    pub fn stats(&self) -> &AttachmentReferenceStats {
        &self.stats
    }
}

// serde_json::Value 默认最后一个重复键覆盖前一个；清理证明必须拒绝这种有歧义数据。
struct StrictValue(Value);
impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct StrictVisitor;
        impl<'de> Visitor<'de> for StrictVisitor {
            type Value = StrictValue;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("unambiguous JSON")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Bool(v)))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Number(v.into())))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Number(v.into())))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Number(
                    Number::from_f64(v).ok_or_else(|| E::custom("invalid number"))?,
                )))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::String(v.into())))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::String(v)))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Null))
            }
            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(StrictValue(value)) = a.next_element()? {
                    values.push(value);
                }
                Ok(StrictValue(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Self::Value, A::Error> {
                let mut values = Map::new();
                while let Some(key) = a.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(de::Error::custom("duplicate key"));
                    }
                    let StrictValue(value) = a.next_value()?;
                    values.insert(key, value);
                }
                Ok(StrictValue(Value::Object(values)))
            }
        }
        d.deserialize_any(StrictVisitor)
    }
}
fn json(bytes: &[u8]) -> Result<Value, String> {
    if bytes.len() > MAX_ROW_BYTES {
        return Err(LIMIT.into());
    }
    let StrictValue(value) = serde_json::from_slice(bytes).map_err(|_| INVALID)?;
    Ok(value)
}
fn object_json(bytes: &[u8]) -> Result<Value, String> {
    let value = json(bytes)?;
    if !value.is_object() {
        return Err(INVALID.into());
    }
    Ok(value)
}
fn plain_blob(key: &DataEncryptionKey, bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.len() > MAX_ROW_BYTES {
        return Err(LIMIT.into());
    }
    if bytes.starts_with(b"SOLO") && !is_encrypted_blob(bytes) {
        return Err(INVALID.into());
    }
    let plain = decrypt_field(key, bytes).map_err(|_| INVALID)?;
    if plain.len() > MAX_ROW_BYTES {
        return Err(LIMIT.into());
    }
    Ok(plain)
}
fn text_bytes(
    key: &DataEncryptionKey,
    text: &str,
    encrypted_required: bool,
) -> Result<Vec<u8>, String> {
    if text.len() > MAX_ROW_BYTES {
        return Err(LIMIT.into());
    }
    if let Some(encoded) = text.strip_prefix("solo:") {
        let raw = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|_| INVALID)?;
        if !is_encrypted_blob(&raw) {
            return Err(INVALID.into());
        }
        plain_blob(key, &raw)
    } else if encrypted_required {
        Err(INVALID.into())
    } else {
        Ok(text.as_bytes().to_vec())
    }
}
fn text_json(
    key: &DataEncryptionKey,
    text: &str,
    encrypted_required: bool,
) -> Result<Value, String> {
    json(&text_bytes(key, text, encrypted_required)?)
}
fn id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 512
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
fn path_parts(path: &str) -> Vec<&str> {
    // Windows canonical 路径可能含 extended prefix；只规范引用字符串，不授权访问。
    let path = path
        .strip_prefix(r"\\?\UNC\")
        .or_else(|| path.strip_prefix(r"\\?\"))
        .unwrap_or(path);
    let mut parts: Vec<&str> = Vec::new();
    for part in path
        .split(['/', '\\'])
        .filter(|part| !part.is_empty() && *part != ".")
    {
        if part == ".." && parts.last().is_some_and(|last| *last != "..") {
            parts.pop();
        } else {
            parts.push(part);
        }
    }
    parts
}
fn equal_path(a: &str, b: &str) -> bool {
    let a = path_parts(a);
    let b = path_parts(b);
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.eq_ignore_ascii_case(b))
}
fn path_reference(path: &str, c: &AttachmentReferenceCandidate) -> bool {
    if c.canonical_path
        .to_str()
        .is_some_and(|p| equal_path(path, p))
        || c.literal_aliases
            .iter()
            .any(|alias| equal_path(path, alias))
    {
        return true;
    }
    // 旧 native root 或恢复换 owner/id 后保留的字面路径也必须保护原物理目录。
    path_parts(path).windows(3).any(|p| {
        p[0].eq_ignore_ascii_case("attachments")
            && p[1].eq_ignore_ascii_case(&c.storage_object_id)
            && p[2].eq_ignore_ascii_case(&c.attachment_id)
    })
}
fn alias<'a>(
    entry: &'a Map<String, Value>,
    camel: &str,
    snake: &str,
) -> Result<Option<&'a Value>, String> {
    match (entry.get(camel), entry.get(snake)) {
        (Some(a), Some(b)) if a != b => Err(INVALID.into()),
        (Some(a), _) | (_, Some(a)) => Ok(Some(a)),
        _ => Ok(None),
    }
}
fn attachment(value: &Value, c: Option<&AttachmentReferenceCandidate>) -> Result<bool, String> {
    let entry = value.as_object().ok_or(INVALID)?;
    let att_id = entry
        .get("id")
        .and_then(Value::as_str)
        .filter(|s| id(s))
        .ok_or(INVALID)?;
    if let Some(object) = alias(entry, "objectId", "object_id")? {
        if !object.as_str().is_some_and(id) {
            return Err(INVALID.into());
        }
    }
    for (camel, snake) in [
        ("fileName", "file_name"),
        ("mimeType", "mime_type"),
        ("createdAt", "created_at"),
    ] {
        if alias(entry, camel, snake)?
            .and_then(Value::as_str)
            .is_none()
        {
            return Err(INVALID.into());
        }
    }
    if alias(entry, "sizeBytes", "size_bytes")?
        .and_then(Value::as_u64)
        .is_none()
    {
        return Err(INVALID.into());
    }
    for (camel, snake) in [
        ("vaultPath", "vault_path"),
        ("srcPath", "src_path"),
        ("deletedAt", "deleted_at"),
        ("description", "description"),
    ] {
        if alias(entry, camel, snake)?.is_some_and(|v| !v.is_null() && !v.is_string()) {
            return Err(INVALID.into());
        }
    }
    if entry
        .get("tags")
        .is_some_and(|v| !v.as_array().is_some_and(|a| a.iter().all(Value::is_string)))
    {
        return Err(INVALID.into());
    }
    let allowed = [
        "id",
        "objectId",
        "object_id",
        "fileName",
        "file_name",
        "mimeType",
        "mime_type",
        "sizeBytes",
        "size_bytes",
        "createdAt",
        "created_at",
        "deletedAt",
        "deleted_at",
        "srcPath",
        "src_path",
        "vaultPath",
        "vault_path",
        "description",
        "tags",
    ];
    if entry.keys().any(|k| !allowed.contains(&k.as_str())) {
        return Err(INVALID.into());
    }
    Ok(c.is_some_and(|c| att_id.eq_ignore_ascii_case(&c.attachment_id)))
}
fn walk(
    value: &Value,
    c: Option<&AttachmentReferenceCandidate>,
    depth: usize,
) -> Result<bool, String> {
    if depth > 128 {
        return Err(INVALID.into());
    }
    let mut found = false;
    match value {
        Value::String(s) => found = c.is_some_and(|c| path_reference(s, c)),
        Value::Object(map) => {
            if let Some(atts) = map.get("__attachments") {
                let atts = atts.as_array().ok_or(INVALID)?;
                let mut seen = HashSet::new();
                for att in atts {
                    found |= attachment(att, c)?;
                    if !seen.insert(att["id"].as_str().ok_or(INVALID)?.to_ascii_lowercase()) {
                        return Err(INVALID.into());
                    }
                }
            }
            for value in map.values() {
                found |= walk(value, c, depth + 1)?;
            }
        }
        Value::Array(values) => {
            for value in values {
                found |= walk(value, c, depth + 1)?;
            }
        }
        _ => {}
    }
    Ok(found)
}
fn hash_row(hash: &mut Sha256, row: &Row<'_>) -> Result<(), String> {
    let mut bytes = 0usize;
    hash.update((row.as_ref().column_count() as u64).to_le_bytes());
    for index in 0..row.as_ref().column_count() {
        use rusqlite::types::ValueRef;
        match row.get_ref(index).map_err(|_| INVALID)? {
            ValueRef::Null => hash.update([0]),
            ValueRef::Integer(v) => {
                hash.update([1]);
                hash.update(v.to_le_bytes());
            }
            ValueRef::Real(v) => {
                hash.update([2]);
                hash.update(v.to_bits().to_le_bytes());
            }
            ValueRef::Text(v) | ValueRef::Blob(v) => {
                bytes = bytes.checked_add(v.len()).ok_or(LIMIT)?;
                if bytes > MAX_ROW_BYTES {
                    return Err(LIMIT.into());
                }
                hash.update([
                    if matches!(row.get_ref(index).map_err(|_| INVALID)?, ValueRef::Text(_)) {
                        3
                    } else {
                        4
                    },
                ]);
                hash.update((v.len() as u64).to_le_bytes());
                hash.update(v);
            }
        }
    }
    Ok(())
}
fn column_text<'a>(row: &'a Row<'_>, index: usize) -> Result<&'a str, String> {
    row.get_ref(index)
        .map_err(|_| INVALID)?
        .as_str()
        .map_err(|_| INVALID.into())
}
fn bytes_array(value: &Value) -> Result<Vec<u8>, String> {
    let values = value.as_array().ok_or(INVALID)?;
    if values.len() > MAX_ROW_BYTES {
        return Err(LIMIT.into());
    }
    values
        .iter()
        .map(|v| {
            v.as_u64()
                .and_then(|v| u8::try_from(v).ok())
                .ok_or_else(|| INVALID.into())
        })
        .collect()
}
// writer/rollback 支持部分编辑快照和完整 ObjectRecord；普通 properties 仍是用户自由 JSON。
const OBJECT_PAYLOAD_FIELDS: &[&str] = &[
    "id",
    "account_id",
    "accountId",
    "typeId",
    "type_id",
    "sectionType",
    "section_type",
    "name",
    "iconName",
    "icon_name",
    "parentId",
    "parent_id",
    "childrenIds",
    "children_ids",
    "properties",
    "propertyLabels",
    "property_labels",
    "sensitivityLevel",
    "sensitivity_level",
    "isDeleted",
    "is_deleted",
    "deletedAt",
    "deleted_at",
    "tags",
    "tags_json",
    "templateId",
    "template_id",
    "templateType",
    "template_type",
    "contractTypeId",
    "contract_type_id",
    "templateHash",
    "template_hash",
    "ignoredTemplateHash",
    "ignored_template_hash",
    "created_at",
    "createdAt",
    "updated_at",
    "updatedAt",
    "version",
];
fn snapshot_payload(
    value: &Value,
    c: Option<&AttachmentReferenceCandidate>,
) -> Result<bool, String> {
    object_payload(value, c, false)
}
fn object_payload(
    value: &Value,
    c: Option<&AttachmentReferenceCandidate>,
    trash: bool,
) -> Result<bool, String> {
    let map = value.as_object().ok_or(INVALID)?;
    if map.keys().any(|name| {
        !(OBJECT_PAYLOAD_FIELDS.contains(&name.as_str())
            || trash && matches!(name.as_str(), "parentPageName" | "parentPageIcon"))
    }) {
        return Err(INVALID.into());
    }
    if trash {
        for name in ["parentPageName", "parentPageIcon"] {
            if value.get(name).is_some_and(|v| !v.is_string()) {
                return Err(INVALID.into());
            }
        }
    }
    if value
        .get("properties")
        .is_some_and(|v| !v.is_object() && !v.is_null())
    {
        return Err(INVALID.into());
    }
    for name in ["propertyLabels", "property_labels"] {
        if value
            .get(name)
            .is_some_and(|v| !v.is_object() && !v.is_null())
        {
            return Err(INVALID.into());
        }
    }
    for name in ["tags", "tags_json"] {
        if value
            .get(name)
            .is_some_and(|v| !v.as_array().is_some_and(|a| a.iter().all(Value::is_string)))
        {
            return Err(INVALID.into());
        }
    }
    if value.get("name").is_some_and(|v| !v.is_string()) {
        return Err(INVALID.into());
    }
    walk(value, c, 0)
}
fn profile_payload(
    value: &Value,
    c: Option<&AttachmentReferenceCandidate>,
) -> Result<bool, String> {
    // CLI /profile set 允许任意点路径和值；ProfileData 不是整个生产 Profile 的封闭 schema。
    // 只限定对象根，严格 JSON/重复键和附件格式；普通自由 JSON 递归保守保护 ID/path。
    if !value.is_object() {
        return Err(INVALID.into());
    }
    walk(value, c, 0)
}
fn profile_bytes(bytes: &[u8], c: Option<&AttachmentReferenceCandidate>) -> Result<bool, String> {
    if bytes.is_empty() {
        return Ok(false);
    } // 现有 LLM/profile loader 明确定义空 payload 为无数据。
    profile_payload(&object_json(bytes)?, c)
}
fn wrapped_bytes(value: &Value) -> Result<Vec<u8>, String> {
    if let Some(encoded) = value.as_str() {
        if encoded.len() > MAX_ROW_BYTES {
            return Err(LIMIT.into());
        }
        base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|_| INVALID.into())
    } else {
        bytes_array(value)
    }
}
fn trash_payload(
    kind: &str,
    value: &Value,
    c: Option<&AttachmentReferenceCandidate>,
) -> Result<bool, String> {
    if !matches!(kind, "object" | "page" | "template") || !value.is_object() {
        return Err(INVALID.into());
    }
    if kind != "template" {
        if !value.get("properties").is_some_and(Value::is_object) {
            return Err(INVALID.into());
        }
        object_payload(value, c, true)
    } else {
        fields(
            value,
            &[
                "id",
                "accountId",
                "account_id",
                "name",
                "iconId",
                "icon_id",
                "properties",
                "category",
                "createdAt",
                "created_at",
                "updatedAt",
                "updated_at",
                "contractTypeId",
                "contract_type_id",
            ],
        )?;
        if !value.get("properties").is_some_and(Value::is_array) {
            return Err(INVALID.into());
        }
        walk(value, c, 0)
    }
}
fn conflict_payload(
    key: &DataEncryptionKey,
    table: &str,
    value: &Value,
    c: Option<&AttachmentReferenceCandidate>,
) -> Result<bool, String> {
    if value.is_null() {
        return Ok(false);
    } // 本地无记录或远端 tombstone 的显式格式。
    if !value.is_object() {
        return Err(INVALID.into());
    }
    let mut found = walk(value, c, 0)?;
    match table {
        "objects" => {
            if !value.get("properties").is_some_and(Value::is_object) {
                return Err(INVALID.into());
            }
            found |= snapshot_payload(value, c)?;
        }
        "trash_items" => {
            fields(
                value,
                &[
                    "id",
                    "item_type",
                    "original_id",
                    "original_parent_id",
                    "original_section_type",
                    "original_sort_order",
                    "data",
                    "deleted_at",
                    "expires_at",
                    "deleted_by",
                    "name_snapshot",
                    "icon_snapshot",
                ],
            )?;
            let kind = value
                .get("item_type")
                .and_then(Value::as_str)
                .ok_or(INVALID)?;
            let bytes = bytes_array(value.get("data").ok_or(INVALID)?)?;
            found |= trash_payload(kind, &object_json(&bytes)?, c)?;
        }
        "profiles" => {
            fields(
                value,
                &[
                    "id",
                    "name",
                    "data",
                    "created_at",
                    "createdAt",
                    "updated_at",
                    "updatedAt",
                    "version",
                ],
            )?;
            let bytes = wrapped_bytes(value.get("data").ok_or(INVALID)?)?;
            found |= profile_bytes(&plain_blob(key, &bytes)?, c)?;
        }
        "user_templates" => {
            fields(
                value,
                &[
                    "id",
                    "accountId",
                    "name",
                    "iconId",
                    "properties",
                    "category",
                    "createdAt",
                    "updatedAt",
                    "contractTypeId",
                ],
            )?;
            if !value.get("properties").is_some_and(Value::is_array) {
                return Err(INVALID.into());
            }
        }
        "llm_conversations" => {
            // 本地快照是会话 JSON；远端/旧线格式 data 是 base64(明文或 SOLO blob)。
            if let Some(data) = value.get("data") {
                fields(value, &["id", "accountId", "data", "updatedAt"])?;
                found |= walk(
                    &object_json(&plain_blob(key, &wrapped_bytes(data)?)?)?,
                    c,
                    0,
                )?;
            }
        }
        _ => return Err(INVALID.into()),
    }
    Ok(found)
}
fn envelope<'a>(value: &'a Value, account: &str, operation: &str) -> Result<&'a Value, String> {
    let map = value.as_object().ok_or(INVALID)?;
    if map.len() != 3
        || map.get("account").and_then(Value::as_str) != Some(account)
        || map.get("id").and_then(Value::as_str) != Some(operation)
    {
        return Err(INVALID.into());
    }
    map.get("payload").ok_or_else(|| INVALID.into())
}

fn source_digest(conn: &Connection, account: &str) -> Result<[u8; 32], String> {
    let mut hash = Sha256::new();
    hash.update(b"RF903 reference view v1");
    hash.update(account.as_bytes());
    let objects = format!("{OBJECT_SELECT_BASE} ORDER BY id");
    for (table, sql) in [
        ("objects", objects.as_str()),
        ("trash_items", "SELECT * FROM trash_items ORDER BY id"),
        (
            "object_snapshots",
            "SELECT * FROM object_snapshots ORDER BY id",
        ),
        ("profiles", "SELECT * FROM profiles ORDER BY id"),
        (
            "sync_conflicts",
            "SELECT * FROM sync_conflicts WHERE resolved=0 ORDER BY id",
        ),
        (
            "import_operations",
            "SELECT * FROM import_operations ORDER BY operation_id",
        ),
        (
            "import_attachment_steps",
            "SELECT * FROM import_attachment_steps ORDER BY operation_id,entry_ordinal",
        ),
    ] {
        hash.update(table.as_bytes());
        let mut stmt = conn.prepare(sql).map_err(|_| INVALID)?;
        let mut rows = stmt.query([]).map_err(|_| INVALID)?;
        while let Some(row) = rows.next().map_err(|_| INVALID)? {
            hash_row(&mut hash, row)?;
        }
    }
    Ok(hash.finalize().into())
}
fn fields(value: &Value, allowed: &[&str]) -> Result<(), String> {
    let map = value.as_object().ok_or(INVALID)?;
    if map.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(INVALID.into());
    }
    Ok(())
}

fn journal_proof(value: &Value) -> Result<(), String> {
    if !value.is_null() {
        fields(
            value,
            &["sha256", "length", "plaintextLength", "stageEpoch"],
        )?;
    }
    Ok(())
}
fn journal_step_plan(value: &Value) -> Result<(), String> {
    fields(
        value,
        &[
            "entryOrdinal",
            "sourceObjectId",
            "sourceAttachmentId",
            "ownerId",
            "attachmentId",
            "safeFileName",
            "metadata",
            "initialStagedProof",
        ],
    )?;
    if let Some(proof) = value.get("initialStagedProof") {
        journal_proof(proof)?;
    }
    Ok(())
}
fn journal_start(value: &Value) -> Result<(), String> {
    fields(value.get("source").ok_or(INVALID)?, &["sha256", "length"])?;
    for owner in value
        .get("owners")
        .and_then(Value::as_array)
        .ok_or(INVALID)?
    {
        fields(owner, &["ownerId", "expectedAttachments"])?;
    }
    for step in value
        .get("steps")
        .and_then(Value::as_array)
        .ok_or(INVALID)?
    {
        journal_step_plan(step)?;
    }
    Ok(())
}

struct Inventory {
    digest: [u8; 32],
    stats: AttachmentReferenceStats,
    referenced: bool,
}
fn inventory(
    conn: &Connection,
    key: &DataEncryptionKey,
    account: &str,
    c: Option<&AttachmentReferenceCandidate>,
) -> Result<Inventory, String> {
    let digest = source_digest(conn, account)?;
    let mut hash = Sha256::new();
    hash.update(b"RF903 reference view v1");
    hash.update(account.as_bytes());
    let mut stats = AttachmentReferenceStats::default();
    let mut referenced = false;
    // 所有对象含软删除，不沿页面/活对象过滤；完整行变化都会使旧视图失效。
    {
        hash.update(b"objects");
        let mut stmt = conn
            .prepare(&format!("{OBJECT_SELECT_BASE} ORDER BY id"))
            .map_err(|_| INVALID)?;
        let mut rows = stmt.query([]).map_err(|_| INVALID)?;
        while let Some(row) = rows.next().map_err(|_| INVALID)? {
            hash_row(&mut hash, row)?;
            stats.object_rows += 1;
            if column_text(row, 1)? != account {
                return Err(INVALID.into());
            }
            let props = text_json(key, column_text(row, 8)?, false)?;
            if !props.is_object() {
                return Err(INVALID.into());
            }
            referenced |= walk(&props, c, 0)?;
            if !matches!(row.get::<_, i64>(11).map_err(|_| INVALID)?, 0 | 1) {
                return Err(INVALID.into());
            }
            if let rusqlite::types::ValueRef::Text(labels) = row.get_ref(9).map_err(|_| INVALID)? {
                let text = std::str::from_utf8(labels).map_err(|_| INVALID)?;
                let bytes = text_bytes(key, text, false)?;
                // save_object(None) 写原始空 TEXT；既有 readers 也把真实解密空串解释为无 labels。
                // 只在标签专用入口允许空，非空坏 JSON / 非 AEAD 的 solo: 仍拒绝。
                if !bytes.is_empty() {
                    let labels = json(&bytes)?;
                    if !labels.is_object() && !labels.is_null() {
                        return Err(INVALID.into());
                    }
                    referenced |= walk(&labels, c, 0)?;
                }
            } else if !matches!(
                row.get_ref(9).map_err(|_| INVALID)?,
                rusqlite::types::ValueRef::Null
            ) {
                return Err(INVALID.into());
            }
        }
    }
    for (table, sql, data_index) in [
        (
            "trash_items",
            "SELECT * FROM trash_items ORDER BY id",
            6usize,
        ),
        (
            "object_snapshots",
            "SELECT * FROM object_snapshots ORDER BY id",
            4usize,
        ),
        ("profiles", "SELECT * FROM profiles ORDER BY id", 2usize),
    ] {
        hash.update(table.as_bytes());
        let mut stmt = conn.prepare(sql).map_err(|_| INVALID)?;
        let mut rows = stmt.query([]).map_err(|_| INVALID)?;
        while let Some(row) = rows.next().map_err(|_| INVALID)? {
            hash_row(&mut hash, row)?;
            let raw = row
                .get_ref(data_index)
                .map_err(|_| INVALID)?
                .as_blob()
                .map_err(|_| INVALID)?;
            let bytes = plain_blob(key, raw)?;
            match table {
                "trash_items" => {
                    stats.trash_rows += 1;
                    referenced |= trash_payload(column_text(row, 1)?, &object_json(&bytes)?, c)?;
                }
                "object_snapshots" => {
                    stats.snapshot_rows += 1;
                    referenced |= snapshot_payload(&object_json(&bytes)?, c)?;
                }
                _ => {
                    stats.profile_rows += 1;
                    referenced |= profile_bytes(&bytes, c)?;
                }
            }
        }
    }
    {
        hash.update(b"sync_conflicts");
        let mut stmt = conn
            .prepare("SELECT * FROM sync_conflicts WHERE resolved=0 ORDER BY id")
            .map_err(|_| INVALID)?;
        let mut rows = stmt.query([]).map_err(|_| INVALID)?;
        while let Some(row) = rows.next().map_err(|_| INVALID)? {
            hash_row(&mut hash, row)?;
            stats.conflict_rows += 1;
            let table = column_text(row, 1)?;
            let remote_deleted = row.get::<_, i64>(6).map_err(|_| INVALID)?;
            if !matches!(remote_deleted, 0 | 1) {
                return Err(INVALID.into());
            }
            for index in [3, 4] {
                let hlc = json(column_text(row, index)?.as_bytes())?;
                fields(&hlc, &["wall_time_ms", "counter", "node_id"])?;
                let _: crate::RecordHlc = serde_json::from_value(hlc).map_err(|_| INVALID)?;
            }
            if !matches!(
                table,
                "objects" | "trash_items" | "profiles" | "user_templates" | "llm_conversations"
            ) {
                return Err(INVALID.into());
            }
            // migration v21 local_data 位于最后一列；显式列查询避免表迁移列序误读。
            let id = column_text(row, 0)?;
            let (local, remote): (String, String) = conn
                .query_row(
                    "SELECT local_data,remote_data FROM sync_conflicts WHERE id=?1",
                    [id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .map_err(|_| INVALID)?;
            // conflict local/remote 的 null 是合法不存在/tombstone；其余根必须 object。
            for (index, text) in [&local, &remote].into_iter().enumerate() {
                // migration v21 的缺本地历史哨兵；GUI 显式回退当前 DB，已由完整当前表扫描保护。
                if index == 0 && text.is_empty() {
                    continue;
                }
                let raw = if let Some(encoded) = text.strip_prefix("solo:") {
                    let bytes = base64::engine::general_purpose::STANDARD
                        .decode(encoded)
                        .map_err(|_| INVALID)?;
                    if !is_encrypted_blob(&bytes) {
                        return Err(INVALID.into());
                    }
                    plain_blob(key, &bytes)?
                } else {
                    text.as_bytes().to_vec()
                };
                let value = json(&raw)?;
                if index == 0 && value.as_object().is_some_and(Map::is_empty) {
                    continue;
                }
                if index == 1 && value.is_null() && remote_deleted == 0 {
                    return Err(INVALID.into());
                }
                referenced |= conflict_payload(key, table, &value, c)?;
            }
        }
    }
    {
        hash.update(b"import_operations");
        let mut stmt = conn
            .prepare("SELECT * FROM import_operations ORDER BY operation_id")
            .map_err(|_| INVALID)?;
        let mut rows = stmt.query([]).map_err(|_| INVALID)?;
        while let Some(row) = rows.next().map_err(|_| INVALID)? {
            hash_row(&mut hash, row)?;
            stats.journal_rows += 1;
            let operation = column_text(row, 0)?;
            if column_text(row, 1)? != account {
                return Err(INVALID.into());
            }
            // 加载前已限定原始单行。Typed loader 校验全部 phase/epoch/step/plan 身份。
            for index in [6, 7] {
                let value = text_json(key, column_text(row, index)?, true)?;
                let payload = envelope(&value, account, operation)?;
                fields(
                    payload,
                    if index == 6 {
                        &[
                            "operationId",
                            "sourceKind",
                            "source",
                            "rootBinding",
                            "requestFingerprint",
                            "plan",
                            "owners",
                            "steps",
                            "preferencesRequired",
                            "sourceReady",
                        ]
                    } else {
                        &[
                            "phase",
                            "epoch",
                            "createdAtMs",
                            "updatedAtMs",
                            "databaseCommit",
                            "attachmentCount",
                            "preferencesImported",
                        ]
                    },
                )?;
                if index == 6 {
                    journal_start(payload)?;
                } else {
                    fields(
                        payload.get("databaseCommit").ok_or(INVALID)?,
                        &[
                            "object_write_count",
                            "object_ids",
                            "template_ids",
                            "snapshot_write_count",
                            "snapshot_ids",
                        ],
                    )?;
                }
            }
            let op = import_operations::load_tx(conn, key, account, operation)
                .map_err(|_| INVALID)?
                .ok_or(INVALID)?;
            let pending = !matches!(
                op.phase,
                ImportOperationPhase::Complete | ImportOperationPhase::Abandoned
            );
            let target = if pending { c } else { None };
            for owner in &op.start.owners {
                if let Some(atts) = &owner.expected_attachments {
                    referenced |= walk(&serde_json::json!({"__attachments":atts}), target, 0)?;
                }
            }
            for step in &op.steps {
                referenced |= attachment(&step.plan.metadata, target)?;
                if pending {
                    referenced |= c.is_some_and(|c| {
                        step.plan
                            .attachment_id
                            .eq_ignore_ascii_case(&c.attachment_id)
                    });
                }
            }
            referenced |= walk(&op.start.plan, target, 0)?;
            if let Some(ready) = &op.start.source_ready {
                referenced |= walk(ready, target, 0)?;
            }
            if pending && op.start.preferences_required && !op.preferences_imported {
                let prefs = op.start.plan.get("preferences").ok_or(UNAVAILABLE)?;
                if prefs.get("kind").and_then(Value::as_str) != Some("ready") {
                    return Err(UNAVAILABLE.into());
                }
                referenced |= profile_bytes(&bytes_array(prefs.get("data").ok_or(INVALID)?)?, c)?;
            }
        }
    }
    {
        hash.update(b"import_attachment_steps");
        let mut stmt = conn
            .prepare("SELECT * FROM import_attachment_steps ORDER BY operation_id,entry_ordinal")
            .map_err(|_| INVALID)?;
        let mut rows = stmt.query([]).map_err(|_| INVALID)?;
        while let Some(row) = rows.next().map_err(|_| INVALID)? {
            hash_row(&mut hash, row)?;
            let operation = column_text(row, 0)?;
            let present: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM import_operations WHERE operation_id=?1)",
                    [operation],
                    |r| r.get(0),
                )
                .map_err(|_| INVALID)?;
            if !present {
                return Err(INVALID.into());
            }
            let value = text_json(key, column_text(row, 3)?, true)?;
            let payload = envelope(&value, account, operation)?;
            fields(payload, &["plan", "phase", "stagedProof"])?;
            journal_step_plan(payload.get("plan").ok_or(INVALID)?)?;
            if let Some(proof) = payload.get("stagedProof") {
                journal_proof(proof)?;
            }
        }
    }
    let verified: [u8; 32] = hash.finalize().into();
    if verified != digest {
        return Err(CHANGED.into());
    }
    Ok(Inventory {
        digest,
        stats,
        referenced,
    })
}
fn key_witness(key: &DataEncryptionKey) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"RF903 in-memory key witness");
    hash.update(key.0);
    hash.finalize().into()
}
impl VaultStore {
    fn reference_key(&self, account: &str) -> Result<DataEncryptionKey, String> {
        if account.is_empty() || account != self.config.account_id {
            return Err(ACCOUNT.into());
        }
        self.data_key().map_err(|_| LOCKED.into())
    }
    pub fn read_attachment_reference_view(
        &self,
        account: &str,
    ) -> Result<AttachmentReferenceView, String> {
        let key = self.reference_key(account)?;
        let mut guard = self.conn.lock().map_err(|_| LOCKED)?;
        let conn = guard.as_mut().ok_or(LOCKED)?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|_| INVALID)?;
        let state = inventory(&tx, &key, account, None)?;
        tx.rollback().map_err(|_| INVALID)?; // 只读视图，无业务 SQL 写入。
        Ok(AttachmentReferenceView {
            account: account.into(),
            store: self.import_instance_id,
            owner: self.root_owner(),
            key_witness: key_witness(&key),
            digest: state.digest,
            stats: state.stats,
        })
    }
    /// Core 必须持原 maintenance + with_session。None 是引用保护，Some 是真实 action 成功。
    /// Immediate 阻止 view 校验与 action 之间出现新 DB 写；action 禁止重入任何 Vault API。
    pub fn with_unreferenced_attachment_guard<T>(
        &self,
        account: &str,
        view: &AttachmentReferenceView,
        candidate: &AttachmentReferenceCandidate,
        action: impl FnOnce() -> Result<T, String>,
    ) -> Result<Option<T>, String> {
        let key = self.reference_key(account)?;
        if view.account != account
            || view.store != self.import_instance_id
            || !Arc::ptr_eq(&view.owner, &self.root_owner())
            || view.key_witness != key_witness(&key)
        {
            return Err(CHANGED.into());
        }
        if !id(&candidate.attachment_id)
            || !id(&candidate.storage_object_id)
            || candidate
                .literal_aliases
                .iter()
                .any(|s| s.len() > 4096 || s.contains('\0'))
        {
            return Err(PATH.into());
        }
        let root = self.root_owner().root().join("attachments");
        let canonical_root = root.canonicalize().map_err(|_| PATH)?;
        if canonical_root != root {
            return Err(PATH.into());
        }
        let directory = root
            .join(&candidate.storage_object_id)
            .join(&candidate.attachment_id)
            .canonicalize()
            .map_err(|_| PATH)?;
        let actual = candidate.canonical_path.canonicalize().map_err(|_| PATH)?;
        if actual != candidate.canonical_path
            || !directory.starts_with(&root)
            || !actual.starts_with(&directory)
        {
            return Err(PATH.into());
        }
        let mut guard = self.conn.lock().map_err(|_| LOCKED)?;
        let conn = guard.as_mut().ok_or(LOCKED)?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| INVALID)?;
        if source_digest(&tx, account)? != view.digest {
            return Err(CHANGED.into());
        }
        let state = inventory(&tx, &key, account, Some(candidate))?;
        if state.digest != view.digest {
            return Err(CHANGED.into());
        }
        if state.referenced {
            return Ok(None);
        }
        let result = action()?;
        // 只读事务按作用域 Drop 回滚。文件动作已完成，不能以后置 SQL 释放错误吞掉真实结果。
        drop(tx);
        Ok(Some(result))
    }
}

#[cfg(test)]
mod tests;
