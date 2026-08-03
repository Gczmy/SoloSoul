//! Vault store - SQLite storage with app-layer AES-256-GCM encryption

use rusqlite::{params, Connection, OptionalExtension};
use std::sync::Mutex;
use zeroize::Zeroize;

use serde::Deserialize;

use crate::encryption::{
    decrypt_field, decrypt_text_field, encrypt_field, encrypt_text_field, ensure_encrypted_text,
    DataEncryptionKey,
};
use crate::migration::run_migrations;
use crate::{Profile, ProfileSummary, VaultConfig, VaultState, VaultStats};

// 表域拆分（P223-②）：objects 域（试点）、snapshots 域、sync_meta 域与 trash 域已抽至子模块。
// 后续 sync_changes / sync_apply / metadata / templates 等域按此模式拆分：
// 方法体逐行搬运到 src/storage/<domain>.rs 的 `impl VaultStore { .. }`，
// 跨域被根模块调用的私有助手提升为 `pub(crate)`，其余保持私有。
mod objects;
mod snapshots;
mod sync_changes;
mod sync_meta;
mod trash;

/// 高频查询共享列列表
const OBJECT_COLUMNS: &str = "\
    id, account_id, type_id, section_type, name, icon_name, parent_id, \
    children_ids, properties, property_labels, sensitivity_level, \
    is_deleted, deleted_at, tags_json, template_id, template_type, \
    contract_type_id, template_hash, ignored_template_hash, created_at, updated_at, version";

/// P213: 对象表 SELECT 前缀常量（load_objects_batch 等动态拼接场景复用，避免每次 format! 分配）。
const OBJECT_SELECT_BASE: &str = "SELECT id, account_id, type_id, section_type, name, icon_name, \
    parent_id, children_ids, properties, property_labels, sensitivity_level, \
    is_deleted, deleted_at, tags_json, template_id, template_type, \
    contract_type_id, template_hash, ignored_template_hash, created_at, updated_at, version \
    FROM objects";

/// P213: load_object 的完整常量 SQL（唯一主键查询，最高频语句）。
const OBJECT_LOAD_SQL: &str = "SELECT id, account_id, type_id, section_type, name, icon_name, \
    parent_id, children_ids, properties, property_labels, sensitivity_level, \
    is_deleted, deleted_at, tags_json, template_id, template_type, \
    contract_type_id, template_hash, ignored_template_hash, created_at, updated_at, version \
    FROM objects WHERE id = ?1";

/// P213: save_object 的 UPSERT 常量 SQL（写对象最高频语句）。
const OBJECT_SAVE_SQL: &str = "INSERT INTO objects (id, account_id, type_id, section_type, name, icon_name, parent_id, \
     children_ids, properties, property_labels, sensitivity_level, \
     is_deleted, deleted_at, tags_json, template_id, template_type, \
     contract_type_id, template_hash, ignored_template_hash, created_at, updated_at, version) \
     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22) \
     ON CONFLICT(id) DO UPDATE SET \
       type_id=excluded.type_id, section_type=excluded.section_type, name=excluded.name, icon_name=excluded.icon_name, \
       parent_id=excluded.parent_id, children_ids=excluded.children_ids, \
       properties=excluded.properties, property_labels=excluded.property_labels, \
       sensitivity_level=excluded.sensitivity_level, \
       is_deleted=excluded.is_deleted, deleted_at=excluded.deleted_at, \
       tags_json=excluded.tags_json, \
       template_id=excluded.template_id, template_type=excluded.template_type, \
       contract_type_id=excluded.contract_type_id, template_hash=excluded.template_hash, \
       ignored_template_hash=excluded.ignored_template_hash, \
       updated_at=excluded.updated_at, version=excluded.version";

/// P213: load_profile 常量 SQL。
const PROFILE_LOAD_SQL: &str =
    "SELECT id, name, data, created_at, updated_at, version FROM profiles WHERE id = ?1";

/// P213: save_profile 的 UPSERT 常量 SQL。
const PROFILE_SAVE_SQL: &str =
    "INSERT INTO profiles (id, name, data, created_at, updated_at, version) \
     VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
     ON CONFLICT(id) DO UPDATE SET \
        name = excluded.name, data = excluded.data, \
        updated_at = excluded.updated_at, version = excluded.version";

/// P213: HLC 读写常量 SQL（同步热路径）。
const HLC_GET_SQL: &str =
    "SELECT wall_time_ms, counter, node_id FROM sync_hlc WHERE table_name = ?1 AND record_id = ?2";
const HLC_SET_SQL: &str =
    "INSERT INTO sync_hlc (table_name, record_id, wall_time_ms, counter, node_id, updated_at) \
     VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
     ON CONFLICT(table_name, record_id) DO UPDATE SET \
        wall_time_ms = excluded.wall_time_ms, \
        counter = excluded.counter, \
        node_id = excluded.node_id, \
        updated_at = excluded.updated_at";

/// P213: 回收站/用户模板读写常量 SQL。
const TRASH_SAVE_SQL: &str =
    "INSERT INTO trash_items (id, item_type, original_id, original_parent_id, \
     original_section_type, original_sort_order, data, deleted_at, expires_at, deleted_by, \
     name_snapshot, icon_snapshot) \
     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)";
const USER_TEMPLATE_SAVE_SQL: &str = "INSERT INTO user_templates (id, account_id, name, icon_id, properties_json, category, contract_type_id, created_at, updated_at) \
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) \
     ON CONFLICT(id) DO UPDATE SET \
         name = excluded.name, \
         icon_id = excluded.icon_id, \
         properties_json = excluded.properties_json, \
         category = excluded.category, \
         contract_type_id = excluded.contract_type_id, \
         updated_at = excluded.updated_at";
const USER_TEMPLATE_LOAD_SQL: &str = "SELECT id, account_id, name, icon_id, properties_json, category, contract_type_id, created_at, updated_at \
     FROM user_templates WHERE id = ?1";

/// P213: 对象软删常量 SQL（回收站批量入站/单删共用）。
const OBJECT_SOFT_DELETE_SQL: &str =
    "UPDATE objects SET is_deleted = 1, deleted_at = ?1, updated_at = ?1 WHERE id = ?2";

/// P213: 手动事务封装——BEGIN/COMMIT/ROLLBACK 三件套。
///
/// rusqlite 的 [`rusqlite::Transaction`] 只实现不可变 Deref（无法获得 `&mut Connection`，
/// 因而无法在其上使用 `prepare_cached`）。本助手改为在调用方持有的 `&mut Connection` 上
/// 手动 BEGIN/COMMIT/ROLLBACK：回调内可直接 `prepare_cached` 复用预编译语句，
/// 失败自动 ROLLBACK，成功 COMMIT。语义与 `conn.transaction()` 等价。
///
/// 注意：与 `Transaction` 不同，回调 panic 时不会自动回滚（无法在 unwind 中持有借用）。
/// 调用方应确保回调内无 panic 操作；本库约定错误一律经 `Result` 返回，故可接受。
fn with_tx<T>(
    conn: &mut Connection,
    begin_err: &'static str,
    commit_err: &'static str,
    f: impl FnOnce(&mut Connection) -> Result<T, String>,
) -> Result<T, String> {
    conn.execute_batch("BEGIN")
        .map_err(|e| format!("{begin_err}: {e}"))?;
    let result = f(conn);
    match &result {
        Ok(_) => conn
            .execute_batch("COMMIT")
            .map_err(|e| format!("{commit_err}: {e}"))?,
        Err(_) => {
            let _ = conn.execute_batch("ROLLBACK");
        }
    }
    result
}

/// P210: 大小写不敏感子串匹配整个 JSON Value（对象键 + 字符串值 + 数字 + 布尔）。
///
/// 旧实现 `value.to_string().to_lowercase()` 每次对每个对象重新序列化 JSON
/// （含引号/花括号/转义/格式化）并整体复制一份小写字符串，属 Value→String 往返浪费。
/// 本函数递归遍历值树，仅对文本片段做小写匹配：
///   - 对象键与字符串值均参与匹配（保持旧实现“键也可命中”的搜索面）；
///   - 数字/布尔按文本形式匹配（与旧序列化结果一致）；
///   - 字符串值按未转义原文匹配——旧实现匹配的是 JSON 转义形态（如值含真实换行时搜索
///     `\\n` 会命中反斜杠+n），新实现匹配真实文本，更符合用户直觉（转义形态命中是
///     序列化的偶然产物，非产品意图）；
///   - null 不再命中字面 `null`（旧序列化为 `null` 文本可命中，搜索 null 属病态用例，
///     缺失可接受）。
///
/// `needle_lower` 必须已小写（调用方统一处理，避免重复 lowercase）。
pub fn json_contains_ignore_case(value: &serde_json::Value, needle_lower: &str) -> bool {
    if needle_lower.is_empty() {
        return true;
    }
    match value {
        serde_json::Value::String(s) => {
            // 长度快速失败，避免短值无谓的 to_lowercase 分配
            s.len() >= needle_lower.len() && s.to_lowercase().contains(needle_lower)
        }
        serde_json::Value::Number(n) => n.to_string().contains(needle_lower),
        serde_json::Value::Bool(b) => b.to_string().contains(needle_lower),
        serde_json::Value::Null => false,
        serde_json::Value::Array(items) => items
            .iter()
            .any(|v| json_contains_ignore_case(v, needle_lower)),
        serde_json::Value::Object(map) => map.iter().any(|(k, v)| {
            k.to_lowercase().contains(needle_lower) || json_contains_ignore_case(v, needle_lower)
        }),
    }
}

/// Vault store with SQLite backing
pub struct VaultStore {
    conn: Mutex<Option<Connection>>,
    config: VaultConfig, // reserved for future path-based vault operations
    state: Mutex<VaultState>,
    data_key: Mutex<Option<DataEncryptionKey>>,
}

impl VaultStore {
    /// Open or create a vault at the given path
    pub fn open(config: VaultConfig) -> Result<Self, String> {
        let path = config.path.join("vault.db");
        let mut conn =
            Connection::open(&path).map_err(|e| format!("Failed to open vault: {}", e))?;

        // Set busy timeout
        let _: Result<(), _> = conn.query_row("PRAGMA busy_timeout = 5000;", [], |_| Ok(()));

        // Initialize schema
        Self::init_schema(&conn)?;
        run_migrations(&mut conn)?;

        let data_key = config.data_key.map(DataEncryptionKey::new);
        let store = Self {
            conn: Mutex::new(Some(conn)),
            config,
            state: Mutex::new(VaultState::Unlocked),
            data_key: Mutex::new(data_key),
        };

        // Migrate plaintext legacy data to encrypted format on first open.
        store.migrate_to_encrypted_format()?;

        // 一次性补齐旧对象缺失的初始 snapshot，使历史 badge 能正常显示。
        // 仅在 Vault 已解锁（有 data_key）时执行；已标记过的 Vault 会自动跳过。
        if store.data_key().is_ok() {
            let _ = store.backfill_missing_snapshots();
        }

        // 修复旧版 object_restore 因字段名大小写不一致导致的“隐形”对象。
        // 仅在 Vault 已解锁时执行；已标记过的 Vault 会自动跳过。
        if store.data_key().is_ok() {
            let _ = store.repair_restored_objects();
        }

        // 为旧版恢复对象补齐丢失的字段级敏感度副本 property_labels。
        // 仅在 Vault 已解锁时执行；已标记过的 Vault 会自动跳过。
        if store.data_key().is_ok() {
            let _ = store.backfill_missing_property_labels();
        }

        Ok(store)
    }

    pub fn base_path(&self) -> &std::path::Path {
        &self.config.path
    }

    fn data_key(&self) -> Result<DataEncryptionKey, String> {
        let guard = self.data_key.lock().map_err(|e| e.to_string())?;
        guard
            .as_ref()
            .cloned()
            .ok_or_else(|| "Vault data key not available".to_string())
    }

    fn init_schema(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS profiles (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                data BLOB NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                version INTEGER DEFAULT 1
            );
            CREATE INDEX IF NOT EXISTS idx_profile_name ON profiles(name);

            CREATE TABLE IF NOT EXISTS metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT
            );

            CREATE TABLE IF NOT EXISTS audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                action TEXT NOT NULL,
                details TEXT,
                entity_type TEXT,
                entity_id TEXT,
                entity_name TEXT,
                performed_by TEXT DEFAULT 'user'
            );

            CREATE TABLE IF NOT EXISTS sync_state (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS sync_peers (
                peer_node_id TEXT PRIMARY KEY,
                peer_name TEXT,
                trusted INTEGER NOT NULL DEFAULT 0,
                public_key_fingerprint TEXT,
                last_seen INTEGER,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS sync_watermarks (
                peer_node_id TEXT NOT NULL,
                table_name TEXT NOT NULL,
                wall_time_ms INTEGER NOT NULL DEFAULT 0,
                counter INTEGER NOT NULL DEFAULT 0,
                node_id TEXT NOT NULL DEFAULT '',
                cursor_id TEXT,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (peer_node_id, table_name)
            );

            CREATE TABLE IF NOT EXISTS sync_hlc (
                table_name TEXT NOT NULL,
                record_id TEXT NOT NULL,
                wall_time_ms INTEGER NOT NULL,
                counter INTEGER NOT NULL,
                node_id TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (table_name, record_id)
            );

            CREATE TABLE IF NOT EXISTS sync_tombstones (
                table_name TEXT NOT NULL,
                record_id TEXT NOT NULL,
                wall_time_ms INTEGER NOT NULL,
                counter INTEGER NOT NULL,
                node_id TEXT NOT NULL,
                deleted_by_node_id TEXT NOT NULL,
                created_at TEXT NOT NULL,
                PRIMARY KEY (table_name, record_id)
            );

            CREATE TABLE IF NOT EXISTS sys_config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at INTEGER NOT NULL,
                description TEXT
            );

            CREATE TABLE IF NOT EXISTS objects (
                id TEXT PRIMARY KEY,
                account_id TEXT NOT NULL,
                type_id TEXT NOT NULL DEFAULT 'note',
                section_type TEXT NOT NULL DEFAULT 'identity',
                name TEXT NOT NULL,
                icon_name TEXT NOT NULL DEFAULT 'document',
                parent_id TEXT,
                children_ids TEXT NOT NULL DEFAULT '[]',
                properties TEXT NOT NULL DEFAULT '{}',
                property_labels TEXT DEFAULT '{}',
                sensitivity_level TEXT NOT NULL DEFAULT 'internal',
                is_deleted INTEGER NOT NULL DEFAULT 0,
                deleted_at TEXT,
                tags_json TEXT NOT NULL DEFAULT '[]',
                template_id TEXT,
                template_type TEXT CHECK(template_type IN ('system', 'user')),
                contract_type_id TEXT,
                template_hash TEXT,
                ignored_template_hash TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                version INTEGER DEFAULT 1
            );
            CREATE INDEX IF NOT EXISTS idx_objects_account ON objects(account_id);
            CREATE INDEX IF NOT EXISTS idx_objects_parent ON objects(parent_id);
            CREATE INDEX IF NOT EXISTS idx_objects_type ON objects(type_id);
            CREATE INDEX IF NOT EXISTS idx_objects_deleted ON objects(is_deleted);

            CREATE TABLE IF NOT EXISTS trash_items (
                id TEXT PRIMARY KEY,
                item_type TEXT NOT NULL,
                original_id TEXT NOT NULL,
                original_parent_id TEXT,
                original_section_type TEXT,
                original_sort_order INTEGER,
                data BLOB NOT NULL,
                deleted_at INTEGER NOT NULL,
                expires_at INTEGER,
                deleted_by TEXT NOT NULL DEFAULT 'user',
                name_snapshot TEXT NOT NULL,
                icon_snapshot TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_trash_expires ON trash_items(expires_at);
            CREATE INDEX IF NOT EXISTS idx_trash_deleted_at ON trash_items(deleted_at);
            CREATE INDEX IF NOT EXISTS idx_trash_type ON trash_items(item_type);

            CREATE TABLE IF NOT EXISTS object_snapshots (
                id TEXT PRIMARY KEY,
                object_id TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                triggered_by TEXT NOT NULL DEFAULT 'user_edit',
                data BLOB NOT NULL,
                diff_summary TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_snapshots_object ON object_snapshots(object_id, timestamp DESC);
            CREATE INDEX IF NOT EXISTS idx_snapshots_timestamp ON object_snapshots(timestamp);

            CREATE TABLE IF NOT EXISTS guide_embeddings (
                id TEXT PRIMARY KEY,
                guide_id TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                chunk_text TEXT NOT NULL,
                embedding BLOB NOT NULL,
                model TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_guide_embeddings_guide ON guide_embeddings(guide_id);
            "#,
        )
        .map_err(|e| format!("Failed to init schema: {}", e))?;

        // Migration: add tags_json column if missing (added in schema v2, §24)
        let _ = conn.execute(
            "ALTER TABLE objects ADD COLUMN tags_json TEXT NOT NULL DEFAULT '[]'",
            [],
        );
        // Migration: add section_type column if missing (§25.1.3)
        let _ = conn.execute(
            "ALTER TABLE objects ADD COLUMN section_type TEXT NOT NULL DEFAULT 'identity'",
            [],
        );

        let version_exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sys_config WHERE key = 'data_version')",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !version_exists {
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO sys_config (key, value, updated_at) VALUES ('data_version', '1', ?1)",
                params![now],
            )
            .map_err(|e| format!("Failed to init data_version: {}", e))?;
        }
        Ok(())
    }

    pub fn state(&self) -> VaultState {
        *self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn stats(&self) -> Result<VaultStats, String> {
        let guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_ref().ok_or("Vault is locked")?;
        let profile_count: usize = conn
            .query_row("SELECT COUNT(*) FROM profiles", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;

        // Profiles data
        let profiles_size: u64 = conn
            .query_row(
                "SELECT COALESCE(SUM(LENGTH(data)), 0) FROM profiles",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        // Objects properties
        let objects_size: u64 = conn
            .query_row(
                "SELECT COALESCE(SUM(LENGTH(properties)), 0) FROM objects",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        // Trash data
        let trash_size: u64 = conn
            .query_row(
                "SELECT COALESCE(SUM(LENGTH(data)), 0) FROM trash_items",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        // Snapshots data
        let snapshots_size: u64 = conn
            .query_row(
                "SELECT COALESCE(SUM(LENGTH(data)), 0) FROM object_snapshots",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;

        let last_modified: Option<String> = conn
            .query_row("SELECT MAX(updated_at) FROM profiles", [], |r| r.get(0))
            .ok();

        Ok(VaultStats {
            profile_count,
            total_size_bytes: profiles_size + objects_size + trash_size + snapshots_size,
            last_modified,
            profiles_size,
            objects_size,
            trash_size,
            snapshots_size,
            attachments_size: 0, // filled in by get_vault_stats command
            ai_conversations_size: 0,
        })
    }

    pub fn lock(&self) {
        if let Ok(mut guard) = self.conn.lock() {
            if let Some(conn) = guard.take() {
                let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)");
            }
        }
        if let Ok(mut key) = self.data_key.lock() {
            if let Some(mut k) = key.take() {
                k.0.zeroize();
            }
        }
        if let Ok(mut s) = self.state.lock() {
            *s = VaultState::Locked;
        }
    }

    /// Migrate legacy plaintext sensitive fields to encrypted format.
    /// Triggered automatically on first open where encryption_version < 1.
    pub fn migrate_to_encrypted_format(&self) -> Result<(), String> {
        let encryption_version: u32 = self
            .get_sys_config("encryption_version")
            .ok()
            .flatten()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        if encryption_version >= 1 {
            return Ok(());
        }

        let key = self.data_key()?;

        // Backup the database file before migration.
        let db_path = self.config.path.join("vault.db");
        let backup_path = self.config.path.join("vault.db.pre_enc.bak");
        if db_path.exists() {
            if let Err(e) = std::fs::copy(&db_path, &backup_path) {
                tracing::error!(
                    "Failed to backup vault db before encryption migration: {}",
                    e
                );
                return Err(format!("Migration backup failed: {}", e));
            }
        }

        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;

        let result: Result<(), String> = (|| {
            // profiles.data
            {
                let mut stmt = tx
                    .prepare("SELECT id, data FROM profiles")
                    .map_err(|e| e.to_string())?;
                let rows: Vec<(String, Vec<u8>)> = stmt
                    .query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
                    })
                    .map_err(|e| e.to_string())?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.to_string())?;
                drop(stmt);
                let mut update = tx
                    .prepare("UPDATE profiles SET data = ?1 WHERE id = ?2")
                    .map_err(|e| e.to_string())?;
                for (id, data) in rows {
                    if !crate::encryption::is_encrypted_blob(&data) && !data.is_empty() {
                        let encrypted = encrypt_field(&key, &data)?;
                        update
                            .execute(params![encrypted, id])
                            .map_err(|e| e.to_string())?;
                    }
                }
            }

            // objects.properties / property_labels
            {
                let mut stmt = tx
                    .prepare("SELECT id, properties, property_labels FROM objects")
                    .map_err(|e| e.to_string())?;
                let rows: Vec<(String, String, Option<String>)> = stmt
                    .query_map([], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, Option<String>>(2)?,
                        ))
                    })
                    .map_err(|e| e.to_string())?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.to_string())?;
                drop(stmt);
                let mut update = tx
                    .prepare(
                        "UPDATE objects SET properties = ?1, property_labels = ?2 WHERE id = ?3",
                    )
                    .map_err(|e| e.to_string())?;
                for (id, properties, labels) in rows {
                    let encrypted_props = ensure_encrypted_text(&key, &properties)?;
                    let encrypted_labels = labels
                        .as_deref()
                        .map(|l| ensure_encrypted_text(&key, l))
                        .transpose()?
                        .unwrap_or_default();
                    update
                        .execute(params![encrypted_props, encrypted_labels, id])
                        .map_err(|e| e.to_string())?;
                }
            }

            // trash_items.data
            {
                let mut stmt = tx
                    .prepare("SELECT id, data FROM trash_items")
                    .map_err(|e| e.to_string())?;
                let rows: Vec<(String, Vec<u8>)> = stmt
                    .query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
                    })
                    .map_err(|e| e.to_string())?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.to_string())?;
                drop(stmt);
                let mut update = tx
                    .prepare("UPDATE trash_items SET data = ?1 WHERE id = ?2")
                    .map_err(|e| e.to_string())?;
                for (id, data) in rows {
                    if !crate::encryption::is_encrypted_blob(&data) && !data.is_empty() {
                        let encrypted = encrypt_field(&key, &data)?;
                        update
                            .execute(params![encrypted, id])
                            .map_err(|e| e.to_string())?;
                    }
                }
            }

            // object_snapshots.data
            {
                let mut stmt = tx
                    .prepare("SELECT id, data FROM object_snapshots")
                    .map_err(|e| e.to_string())?;
                let rows: Vec<(String, Vec<u8>)> = stmt
                    .query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
                    })
                    .map_err(|e| e.to_string())?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.to_string())?;
                drop(stmt);
                let mut update = tx
                    .prepare("UPDATE object_snapshots SET data = ?1 WHERE id = ?2")
                    .map_err(|e| e.to_string())?;
                for (id, data) in rows {
                    if !crate::encryption::is_encrypted_blob(&data) && !data.is_empty() {
                        let encrypted = encrypt_field(&key, &data)?;
                        update
                            .execute(params![encrypted, id])
                            .map_err(|e| e.to_string())?;
                    }
                }
            }

            // user_templates.properties_json
            {
                let mut stmt = tx
                    .prepare("SELECT id, properties_json FROM user_templates")
                    .map_err(|e| e.to_string())?;
                let rows: Vec<(String, String)> = stmt
                    .query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                    })
                    .map_err(|e| e.to_string())?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.to_string())?;
                drop(stmt);
                let mut update = tx
                    .prepare("UPDATE user_templates SET properties_json = ?1 WHERE id = ?2")
                    .map_err(|e| e.to_string())?;
                for (id, props_json) in rows {
                    let encrypted = ensure_encrypted_text(&key, &props_json)?;
                    update
                        .execute(params![encrypted, id])
                        .map_err(|e| e.to_string())?;
                }
            }

            // audit_log.details / entity_name
            {
                let mut stmt = tx
                    .prepare("SELECT id, details, entity_name FROM audit_log")
                    .map_err(|e| e.to_string())?;
                let rows: Vec<(i64, Option<String>, Option<String>)> = stmt
                    .query_map([], |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, Option<String>>(1)?,
                            row.get::<_, Option<String>>(2)?,
                        ))
                    })
                    .map_err(|e| e.to_string())?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.to_string())?;
                drop(stmt);
                let mut update = tx
                    .prepare("UPDATE audit_log SET details = ?1, entity_name = ?2 WHERE id = ?3")
                    .map_err(|e| e.to_string())?;
                for (id, details, entity_name) in rows {
                    let encrypted_details = details
                        .as_deref()
                        .map(|d| ensure_encrypted_text(&key, d))
                        .transpose()?
                        .unwrap_or_default();
                    let encrypted_name = entity_name
                        .as_deref()
                        .map(|n| ensure_encrypted_text(&key, n))
                        .transpose()?
                        .unwrap_or_default();
                    update
                        .execute(params![encrypted_details, encrypted_name, id])
                        .map_err(|e| e.to_string())?;
                }
            }

            let now = chrono::Utc::now().to_rfc3339();
            tx.execute(
                "INSERT OR REPLACE INTO sys_config (key, value, updated_at) VALUES ('encryption_version', ?1, ?2)",
                params!["1", now],
            ).map_err(|e| e.to_string())?;
            tx.execute(
                "INSERT OR REPLACE INTO sys_config (key, value, updated_at) VALUES ('encryption_migrated_at', ?1, ?2)",
                params![chrono::Utc::now().to_rfc3339(), now],
            ).map_err(|e| e.to_string())?;

            Ok(())
        })();

        match result {
            Ok(()) => {
                tx.commit().map_err(|e| e.to_string())?;
                tracing::info!("Vault encryption migration completed successfully");
                Ok(())
            }
            Err(e) => {
                tracing::error!("Vault encryption migration failed: {}", e);
                // Transaction is dropped here, causing rollback.
                Err(format!("Encryption migration failed: {}", e))
            }
        }
    }

    /// Re-encrypt all sensitive fields with a new key.
    /// Used by `change_password` to ensure data is accessible only with the new password.
    pub fn reencrypt_all(
        &self,
        old_key: &DataEncryptionKey,
        new_key: &DataEncryptionKey,
    ) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;

        let result: Result<(), String> = (|| {
            // profiles
            {
                let mut stmt = tx
                    .prepare("SELECT id, data FROM profiles")
                    .map_err(|e| e.to_string())?;
                let rows: Vec<(String, Vec<u8>)> = stmt
                    .query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
                    })
                    .map_err(|e| e.to_string())?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.to_string())?;
                let rows_len = rows.len();
                drop(stmt);
                let mut update = tx
                    .prepare("UPDATE profiles SET data = ?1 WHERE id = ?2")
                    .map_err(|e| e.to_string())?;
                for (id, data) in rows {
                    let plain = decrypt_field(old_key, &data)?;
                    let encrypted = encrypt_field(new_key, &plain)?;
                    update
                        .execute(params![encrypted, id])
                        .map_err(|e| e.to_string())?;
                }
                tracing::info!("reencrypt_progress: table=profiles, rows={}", rows_len);
            }

            // objects
            {
                let mut stmt = tx
                    .prepare("SELECT id, properties, property_labels FROM objects")
                    .map_err(|e| e.to_string())?;
                let rows: Vec<(String, String, Option<String>)> = stmt
                    .query_map([], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, Option<String>>(2)?,
                        ))
                    })
                    .map_err(|e| e.to_string())?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.to_string())?;
                let rows_len = rows.len();
                drop(stmt);
                let mut update = tx
                    .prepare(
                        "UPDATE objects SET properties = ?1, property_labels = ?2 WHERE id = ?3",
                    )
                    .map_err(|e| e.to_string())?;
                for (id, properties, labels) in rows {
                    let plain_props = decrypt_text_field(old_key, &properties)?;
                    let encrypted_props = encrypt_text_field(new_key, &plain_props)?;
                    let plain_labels = labels
                        .as_deref()
                        .map(|l| decrypt_text_field(old_key, l))
                        .transpose()?;
                    let encrypted_labels = plain_labels
                        .map(|l| encrypt_text_field(new_key, &l))
                        .transpose()?
                        .unwrap_or_default();
                    update
                        .execute(params![encrypted_props, encrypted_labels, id])
                        .map_err(|e| e.to_string())?;
                }
                tracing::info!("reencrypt_progress: table=objects, rows={}", rows_len);
            }

            // trash_items
            {
                let mut stmt = tx
                    .prepare("SELECT id, data FROM trash_items")
                    .map_err(|e| e.to_string())?;
                let rows: Vec<(String, Vec<u8>)> = stmt
                    .query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
                    })
                    .map_err(|e| e.to_string())?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.to_string())?;
                let rows_len = rows.len();
                drop(stmt);
                let mut update = tx
                    .prepare("UPDATE trash_items SET data = ?1 WHERE id = ?2")
                    .map_err(|e| e.to_string())?;
                for (id, data) in rows {
                    let plain = decrypt_field(old_key, &data)?;
                    let encrypted = encrypt_field(new_key, &plain)?;
                    update
                        .execute(params![encrypted, id])
                        .map_err(|e| e.to_string())?;
                }
                tracing::info!("reencrypt_progress: table=trash_items, rows={}", rows_len);
            }

            // object_snapshots
            {
                let mut stmt = tx
                    .prepare("SELECT id, data FROM object_snapshots")
                    .map_err(|e| e.to_string())?;
                let rows: Vec<(String, Vec<u8>)> = stmt
                    .query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
                    })
                    .map_err(|e| e.to_string())?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.to_string())?;
                let rows_len = rows.len();
                drop(stmt);
                let mut update = tx
                    .prepare("UPDATE object_snapshots SET data = ?1 WHERE id = ?2")
                    .map_err(|e| e.to_string())?;
                for (id, data) in rows {
                    let plain = decrypt_field(old_key, &data)?;
                    let encrypted = encrypt_field(new_key, &plain)?;
                    update
                        .execute(params![encrypted, id])
                        .map_err(|e| e.to_string())?;
                }
                tracing::info!(
                    "reencrypt_progress: table=object_snapshots, rows={}",
                    rows_len
                );
            }

            // user_templates
            {
                let mut stmt = tx
                    .prepare("SELECT id, properties_json FROM user_templates")
                    .map_err(|e| e.to_string())?;
                let rows: Vec<(String, String)> = stmt
                    .query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                    })
                    .map_err(|e| e.to_string())?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.to_string())?;
                let rows_len = rows.len();
                drop(stmt);
                let mut update = tx
                    .prepare("UPDATE user_templates SET properties_json = ?1 WHERE id = ?2")
                    .map_err(|e| e.to_string())?;
                for (id, props_json) in rows {
                    let plain = decrypt_text_field(old_key, &props_json)?;
                    let encrypted = encrypt_text_field(new_key, &plain)?;
                    update
                        .execute(params![encrypted, id])
                        .map_err(|e| e.to_string())?;
                }
                tracing::info!(
                    "reencrypt_progress: table=user_templates, rows={}",
                    rows_len
                );
            }

            // audit_log
            {
                let mut stmt = tx
                    .prepare("SELECT id, details, entity_name FROM audit_log")
                    .map_err(|e| e.to_string())?;
                let rows: Vec<(i64, Option<String>, Option<String>)> = stmt
                    .query_map([], |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, Option<String>>(1)?,
                            row.get::<_, Option<String>>(2)?,
                        ))
                    })
                    .map_err(|e| e.to_string())?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.to_string())?;
                let rows_len = rows.len();
                drop(stmt);
                let mut update = tx
                    .prepare("UPDATE audit_log SET details = ?1, entity_name = ?2 WHERE id = ?3")
                    .map_err(|e| e.to_string())?;
                for (id, details, entity_name) in rows {
                    let plain_details = details
                        .as_deref()
                        .map(|d| decrypt_text_field(old_key, d))
                        .transpose()?;
                    let encrypted_details = plain_details
                        .map(|d| encrypt_text_field(new_key, &d))
                        .transpose()?
                        .unwrap_or_default();
                    let plain_name = entity_name
                        .as_deref()
                        .map(|n| decrypt_text_field(old_key, n))
                        .transpose()?;
                    let encrypted_name = plain_name
                        .map(|n| encrypt_text_field(new_key, &n))
                        .transpose()?
                        .unwrap_or_default();
                    update
                        .execute(params![encrypted_details, encrypted_name, id])
                        .map_err(|e| e.to_string())?;
                }
                tracing::info!("reencrypt_progress: table=audit_log, rows={}", rows_len);
            }

            Ok(())
        })();

        // N-2: 仅在全部解密+重加密成功时提交；任一行失败则整体回滚（丢弃 tx 触发），
        // 避免“部分行已换新钥、失败行仍为旧钥”的混态——混态会令改密/KDF 升级后
        // 账户部分数据永久不可解密。
        match result {
            Ok(()) => {
                tx.commit().map_err(|e| e.to_string())?;
                tracing::info!("Vault re-encryption completed successfully");
                Ok(())
            }
            Err(e) => {
                tracing::error!("Vault re-encryption failed, transaction rolled back: {}", e);
                Err(e)
            }
        }
    }

    /// N-2: 替换内存中的数据加密密钥（不改写磁盘，仅影响后续读写）。
    /// 供改密/KDF 升级回滚时先读回再写回；调用方负责保持与磁盘数据一致。
    /// 与 `test_reencrypt_all_roundtrip` 内部手动替换行为一致。
    pub fn set_data_key(&self, key: DataEncryptionKey) {
        if let Ok(mut guard) = self.data_key.lock() {
            *guard = Some(key);
        }
    }

    pub fn save_profile(&self, profile: &Profile) -> Result<(), String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        Self::save_profile_tx(conn, &key, profile)
    }

    /// P115: 事务内保存 Profile（连接由调用方持有，批量应用单事务内复用）。
    /// P213: 事务内保存 Profile（连接由调用方持有，批量应用单事务内复用）。
    fn save_profile_tx(
        conn: &mut Connection,
        key: &DataEncryptionKey,
        profile: &Profile,
    ) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();
        let encrypted_data = encrypt_field(key, &profile.data)?;
        let mut stmt = conn
            .prepare_cached(PROFILE_SAVE_SQL)
            .map_err(|e| format!("Failed to prepare save_profile: {}", e))?;
        stmt.execute(params![
            profile.id,
            profile.name,
            encrypted_data,
            profile.created_at.to_rfc3339(),
            now,
            profile.version
        ])
        .map_err(|e| format!("Failed to save profile: {}", e))?;
        Ok(())
    }

    pub fn load_profile(&self, id: &str) -> Result<Option<Profile>, String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn
            .prepare_cached(PROFILE_LOAD_SQL)
            .map_err(|e| format!("Failed to prepare: {}", e))?;
        let result = stmt
            .query_row(params![id], |row| {
                let created_str: String = row.get(3)?;
                let updated_str: String = row.get(4)?;
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
                Ok(Profile {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    data,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_str)
                        .map(|d| d.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&updated_str)
                        .map(|d| d.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    version: row.get(5)?,
                })
            })
            .optional()
            .map_err(|e| format!("Failed to load profile: {}", e))?;
        Ok(result)
    }

    pub fn delete_profile(&self, id: &str) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let affected = conn
            .execute("DELETE FROM profiles WHERE id = ?1", params![id])
            .map_err(|e| format!("Failed to delete: {}", e))?;
        if affected == 0 {
            return Err("Profile not found".to_string());
        }
        drop(guard);
        self.record_tombstone("profiles", id)?;
        Ok(())
    }

    /// Apply a single incoming sync record. Returns true if the local state changed.
    /// 单条记录应用（单事务语义：HLC 预检 + 写入 + HLC 更新全部在一个事务内）。
    pub fn apply_sync_record(
        &self,
        record: &crate::VaultSyncRecord,
        local_node_id: &str,
    ) -> Result<bool, String> {
        let borrowed = crate::BorrowedSyncRecord {
            id: &record.id,
            table: &record.table,
            data: &record.data,
            hlc: &record.hlc,
            deleted: record.deleted,
        };
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        // P213: 手动事务（Transaction 无 DerefMut，prepare_cached 需要 &mut Connection）。
        let outcome = with_tx(
            conn,
            "apply_sync_record begin",
            "apply_sync_record commit",
            |c| Self::apply_sync_record_tx(c, &key, &borrowed, local_node_id),
        )?;
        Ok(outcome.applied)
    }

    /// P115: 事务内应用单条同步记录（连接由调用方持有）。
    ///
    /// 与 `apply_sync_record` 的差异：不获取/持有连接，适用于批量事务循环；
    /// 返回 [`crate::SyncApplyOutcome`]，包含写前本地 HLC 供冲突报告复用。
    fn apply_sync_record_tx(
        conn: &mut Connection,
        key: &DataEncryptionKey,
        record: &crate::BorrowedSyncRecord,
        local_node_id: &str,
    ) -> Result<crate::SyncApplyOutcome, String> {
        // Conflict resolution: only accept records with HLC greater than the local HLC.
        let local_hlc = Self::get_record_hlc_tx(conn, record.table, record.id)?;
        if let Some(ref cur) = local_hlc {
            if !Self::record_hlc_is_newer(record.hlc, cur) {
                return Ok(crate::SyncApplyOutcome {
                    applied: false,
                    local_hlc,
                    error: None,
                });
            }
        }

        let applied = match record.table {
            "profiles" => Self::apply_profile_sync_record_tx(conn, key, record),
            "objects" => Self::apply_object_sync_record_tx(conn, key, record, local_node_id),
            "user_templates" => Self::apply_user_template_sync_record_tx(conn, key, record),
            "trash_items" => Self::apply_trash_sync_record_tx(conn, key, record),
            _ => Err(format!("Unsupported sync table: {}", record.table)),
        }?;

        if applied {
            Self::set_record_hlc_tx(conn, record.table, record.id, record.hlc)?;
        }
        Ok(crate::SyncApplyOutcome {
            applied,
            local_hlc,
            error: None,
        })
    }

    /// P115: 整批应用同步记录——单连接 + 单事务 + 借用视图（零克隆）。
    ///
    /// 相比旧实现（`apply_sync_records` 逐条 `apply_sync_record`）：
    ///   - 整批包一个事务：所有 INSERT/UPDATE/DELETE + sync_hlc 更新一次 commit；
    ///   - HLC 重复查询 ×2 → ×1：每条记录只查一次本地 HLC，结果带写前本地 HLC
    ///     供调用方冲突报告复用，不再二次查询；
    ///   - `data` 传引用：接收 [`crate::BorrowedSyncRecord`] 借用视图，不再逐条克隆 JSON。
    ///
    /// 返回每条记录的应用结果；单条记录失败不中断整批（错误落入 `SyncApplyOutcome.error`），
    /// 整批事务最终统一 commit/rollback。
    pub fn apply_sync_records_batch(
        &self,
        records: &[crate::BorrowedSyncRecord],
        local_node_id: &str,
    ) -> Result<Vec<crate::SyncApplyOutcome>, String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?; // P213: 手动事务（Transaction 无 DerefMut，prepare_cached 需要 &mut Connection）。
        with_tx(conn, "apply batch begin", "apply batch commit", |c| {
            let mut outcomes = Vec::with_capacity(records.len());
            for record in records {
                // 单条失败不中断整批：错误落入 outcome，事务最终统一 commit/rollback。
                match Self::apply_sync_record_tx(c, &key, record, local_node_id) {
                    Ok(outcome) => outcomes.push(outcome),
                    Err(e) => outcomes.push(crate::SyncApplyOutcome {
                        applied: false,
                        local_hlc: None,
                        error: Some(e),
                    }),
                }
            }
            Ok(outcomes)
        })
    }

    fn record_hlc_is_newer(remote: &crate::RecordHlc, local: &crate::RecordHlc) -> bool {
        remote.wall_time_ms > local.wall_time_ms
            || (remote.wall_time_ms == local.wall_time_ms
                && (remote.counter > local.counter
                    || (remote.counter == local.counter && remote.node_id > local.node_id)))
    }

    // ── Sync conflict helpers ────────────────────────────────

    /// 持久化一条同步冲突记录。
    #[allow(clippy::too_many_arguments)]
    pub fn save_sync_conflict(
        &self,
        table: &str,
        record_id: &str,
        local_hlc: &crate::RecordHlc,
        remote_hlc: &crate::RecordHlc,
        local_data: &serde_json::Value,
        remote_data: &serde_json::Value,
        remote_deleted: bool,
    ) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let local_hlc_json = serde_json::to_string(local_hlc).map_err(|e| e.to_string())?;
        let remote_hlc_json = serde_json::to_string(remote_hlc).map_err(|e| e.to_string())?;
        let local_data_json = serde_json::to_string(local_data).map_err(|e| e.to_string())?;
        let remote_data_json = serde_json::to_string(remote_data).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO sync_conflicts (id, table_name, record_id, local_hlc, remote_hlc, local_data, remote_data, remote_deleted, winner, created_at, resolved)
             VALUES ((lower(hex(randomblob(16)))), ?1, ?2, ?3, ?4, ?5, ?6, ?7, 'local', ?8, 0)
             ON CONFLICT(table_name, record_id) DO UPDATE SET
                local_hlc = excluded.local_hlc,
                remote_hlc = excluded.remote_hlc,
                local_data = excluded.local_data,
                remote_data = excluded.remote_data,
                remote_deleted = excluded.remote_deleted,
                winner = excluded.winner",
            params![table, record_id, local_hlc_json, remote_hlc_json, local_data_json, remote_data_json, remote_deleted, Self::now_rfc3339()],
        )
        .map_err(|e| format!("save_sync_conflict: {}", e))?;
        Ok(())
    }

    /// 列出所有未解决的同步冲突摘要。
    pub fn list_sync_conflicts(&self) -> Result<Vec<crate::SyncConflictSummary>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn
            .prepare(
                "SELECT id, table_name, record_id, local_hlc, remote_hlc, winner, created_at
                 FROM sync_conflicts WHERE resolved = 0 ORDER BY created_at DESC",
            )
            .map_err(|e| format!("list_sync_conflicts: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(crate::SyncConflictSummary {
                    id: row.get(0)?,
                    table_name: row.get(1)?,
                    record_id: row.get(2)?,
                    local_hlc_json: row.get(3)?,
                    remote_hlc_json: row.get(4)?,
                    winner: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })
            .map_err(|e| format!("list_sync_conflicts query: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("list_sync_conflicts collect: {}", e))?;
        Ok(rows)
    }

    /// 获取一条冲突的详情（含本地和远程数据）。
    pub fn get_sync_conflict(
        &self,
        conflict_id: &str,
    ) -> Result<Option<crate::SyncConflictDetail>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let result = conn
            .query_row(
                "SELECT id, table_name, record_id, local_hlc, remote_hlc, local_data, remote_data, remote_deleted, winner, created_at
                 FROM sync_conflicts WHERE id = ?1 AND resolved = 0",
                params![conflict_id],
                |row| {
                    Ok(crate::SyncConflictDetail {
                        id: row.get(0)?,
                        table_name: row.get(1)?,
                        record_id: row.get(2)?,
                        local_hlc_json: row.get(3)?,
                        remote_hlc_json: row.get(4)?,
                        local_data_json: row.get(5)?,
                        remote_data_json: row.get(6)?,
                        remote_deleted: row.get::<_, i32>(7)? != 0,
                        winner: row.get(8)?,
                        created_at: row.get(9)?,
                    })
                },
            )
            .optional()
            .map_err(|e| format!("get_sync_conflict: {}", e))?;
        Ok(result)
    }

    /// 获取当前本地记录的同步格式快照，用于在冲突 UI 中展示冲突发生时的本地状态。
    pub fn get_sync_conflict_local_data(
        &self,
        table: &str,
        record_id: &str,
    ) -> Result<Option<serde_json::Value>, String> {
        let value = match table {
            "profiles" => {
                if let Some(p) = self.load_profile(record_id)? {
                    serde_json::to_value(&p).map_err(|e| format!("serialize profile: {}", e))?
                } else {
                    return Ok(None);
                }
            }
            "objects" => {
                if let Some(obj) = self.load_object(record_id)? {
                    serde_json::to_value(&obj).map_err(|e| format!("serialize object: {}", e))?
                } else {
                    return Ok(None);
                }
            }
            "user_templates" => {
                if let Some(tpl) = self.load_user_template(record_id)? {
                    serde_json::to_value(&tpl).map_err(|e| format!("serialize template: {}", e))?
                } else {
                    return Ok(None);
                }
            }
            "trash_items" => {
                if let Some(item) = self.get_trash_item(record_id)? {
                    serde_json::to_value(&item)
                        .map_err(|e| format!("serialize trash item: {}", e))?
                } else {
                    return Ok(None);
                }
            }
            _ => return Ok(None),
        };
        Ok(Some(value))
    }

    /// 将冲突记录标记为已解决并删除。
    pub fn delete_sync_conflict(&self, conflict_id: &str) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        conn.execute(
            "DELETE FROM sync_conflicts WHERE id = ?1",
            params![conflict_id],
        )
        .map_err(|e| format!("delete_sync_conflict: {}", e))?;
        Ok(())
    }

    /// 根据策略解决冲突。
    /// strategy: "keep_local" | "keep_remote" | "dismiss"
    /// 返回是否应用了远程数据（keep_remote 时）。
    pub fn resolve_sync_conflict(&self, conflict_id: &str, strategy: &str) -> Result<bool, String> {
        let detail = match self.get_sync_conflict(conflict_id)? {
            Some(d) => d,
            None => return Err("Conflict not found or already resolved".to_string()),
        };

        let apply_remote = match strategy {
            "keep_remote" => {
                if detail.remote_deleted {
                    // 远程为删除 tombstone，直接删除本地记录并记录 tombstone。
                    self.hard_delete_record(&detail.table_name, &detail.record_id)?;
                    true
                } else {
                    let remote_data: serde_json::Value =
                        serde_json::from_str(&detail.remote_data_json)
                            .map_err(|e| format!("Failed to parse remote_data: {}", e))?;
                    let fresh_hlc = self.new_local_hlc()?;
                    let vault_rec = crate::VaultSyncRecord {
                        id: detail.record_id.clone(),
                        table: detail.table_name.clone(),
                        data: remote_data,
                        hlc: fresh_hlc,
                        deleted: false,
                    };
                    let local_node_id = self.local_node_id();
                    self.apply_sync_record(&vault_rec, &local_node_id)?
                }
            }
            "keep_local" | "dismiss" => false,
            _ => {
                return Err(format!(
                    "Unknown conflict resolution strategy: {}",
                    strategy
                ))
            }
        };

        self.delete_sync_conflict(conflict_id)?;
        Ok(apply_remote)
    }

    /// 根据表名和记录 ID 硬删除本地记录（冲突解决 keep_remote 且远程为 tombstone 时使用）。
    fn hard_delete_record(&self, table: &str, record_id: &str) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        match table {
            "profiles" => {
                conn.execute("DELETE FROM profiles WHERE id = ?1", params![record_id])
                    .map_err(|e| e.to_string())?;
            }
            "objects" => {
                conn.execute("DELETE FROM objects WHERE id = ?1", params![record_id])
                    .map_err(|e| e.to_string())?;
            }
            "user_templates" => {
                conn.execute(
                    "DELETE FROM user_templates WHERE id = ?1",
                    params![record_id],
                )
                .map_err(|e| e.to_string())?;
            }
            "trash_items" => {
                conn.execute("DELETE FROM trash_items WHERE id = ?1", params![record_id])
                    .map_err(|e| e.to_string())?;
            }
            _ => return Err(format!("Unsupported sync table: {}", table)),
        }
        drop(guard);
        self.record_tombstone(table, record_id)?;
        self.set_record_hlc(table, record_id, &self.new_tombstone_hlc()?)?;
        Ok(())
    }

    /// P115: 事务内应用单条 Profile 同步记录（连接由调用方持有）。
    fn apply_profile_sync_record_tx(
        conn: &mut Connection,
        key: &DataEncryptionKey,
        record: &crate::BorrowedSyncRecord,
    ) -> Result<bool, String> {
        if record.deleted {
            // Apply remote tombstone directly without creating a local tombstone,
            // so the remote HLC remains the authoritative deletion timestamp.
            conn.execute("DELETE FROM profiles WHERE id = ?1", params![record.id])
                .map_err(|e| format!("delete profile: {}", e))?;
            return Ok(true);
        }
        let data_b64 = record
            .data
            .get("data")
            .and_then(|v| v.as_str())
            .ok_or("Missing profile data")?;
        let data = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data_b64)
            .map_err(|e| format!("profile data decode: {}", e))?;
        let now = Self::now_rfc3339();
        let created = record
            .data
            .get("createdAt")
            .and_then(|v| v.as_str())
            .unwrap_or(&now);
        let updated = record
            .data
            .get("updatedAt")
            .and_then(|v| v.as_str())
            .unwrap_or(&now);
        let profile = crate::Profile {
            id: record.id.to_string(),
            name: record
                .data
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            data,
            created_at: chrono::DateTime::parse_from_rfc3339(created)
                .map(|d| d.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: chrono::DateTime::parse_from_rfc3339(updated)
                .map(|d| d.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            version: record
                .data
                .get("version")
                .and_then(|v| v.as_u64())
                .unwrap_or(1) as u32,
        };
        Self::save_profile_tx(conn, key, &profile)?;
        Ok(true)
    }

    /// P115: 事务内应用单条对象同步记录（连接由调用方持有）。
    fn apply_object_sync_record_tx(
        conn: &mut Connection,
        key: &DataEncryptionKey,
        record: &crate::BorrowedSyncRecord,
        local_node_id: &str,
    ) -> Result<bool, String> {
        let mut obj: crate::ObjectRecord = crate::ObjectRecord::deserialize(record.data)
            .map_err(|e| format!("object decode: {}", e))?;
        // Bump version if the local node is modifying an existing object.
        if Self::load_object_tx(conn, key, &obj.id)?.is_some() {
            obj.version += 1;
            obj.updated_at = Self::now_rfc3339();
        }
        // Re-encrypt properties locally.
        Self::save_object_tx(conn, key, &obj)?;
        let _ = local_node_id;
        Ok(true)
    }

    /// P115: 事务内应用单条模板同步记录（连接由调用方持有）。
    fn apply_user_template_sync_record_tx(
        conn: &mut Connection,
        key: &DataEncryptionKey,
        record: &crate::BorrowedSyncRecord,
    ) -> Result<bool, String> {
        if record.deleted {
            let _ = Self::load_user_template_tx(conn, key, record.id); // ensure vault is accessible
            conn.execute(
                "DELETE FROM user_templates WHERE id = ?1",
                params![record.id],
            )
            .map_err(|e| format!("delete template: {}", e))?;
            return Ok(true);
        }
        let tpl: crate::UserTemplate = crate::UserTemplate::deserialize(record.data)
            .map_err(|e| format!("template decode: {}", e))?;
        Self::save_user_template_tx(conn, key, &tpl)?;
        Ok(true)
    }

    /// P115: 事务内应用单条回收站同步记录（连接由调用方持有）。
    fn apply_trash_sync_record_tx(
        conn: &mut Connection,
        key: &DataEncryptionKey,
        record: &crate::BorrowedSyncRecord,
    ) -> Result<bool, String> {
        let item: crate::TrashItem = crate::TrashItem::deserialize(record.data)
            .map_err(|e| format!("trash decode: {}", e))?;
        Self::save_trash_item_tx(conn, key, &item)?;
        Ok(true)
    }

    pub fn get_sync_node_id(&self) -> Result<Option<String>, String> {
        self.read_metadata("node_id", "sync")
            .map(|b| b.and_then(|v| String::from_utf8(v).ok()))
    }

    pub fn set_sync_node_id(&self, node_id: &str) -> Result<(), String> {
        self.write_metadata("node_id", "sync", node_id.as_bytes())
    }

    pub fn get_sync_secret_key(&self) -> Result<Option<[u8; 32]>, String> {
        self.read_metadata("secret_key", "sync").map(|b| {
            b.map(|v| {
                let mut key = [0u8; 32];
                let len = v.len().min(32);
                key[..len].copy_from_slice(&v[..len]);
                key
            })
        })
    }

    pub fn set_sync_secret_key(&self, key: &[u8; 32]) -> Result<(), String> {
        self.write_metadata("secret_key", "sync", key)
    }

    pub fn list_profiles(&self) -> Result<Vec<ProfileSummary>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn.prepare(
            "SELECT id, name, created_at, updated_at, version FROM profiles ORDER BY updated_at DESC"
        ).map_err(|e| format!("Failed to prepare: {}", e))?;
        let profiles = stmt
            .query_map([], |row| {
                let created_str: String = row.get(2)?;
                let updated_str: String = row.get(3)?;
                Ok(ProfileSummary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_str)
                        .map(|d| d.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&updated_str)
                        .map(|d| d.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    version: row.get(4)?,
                })
            })
            .map_err(|e| format!("Failed to query: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect: {}", e))?;
        Ok(profiles)
    }

    /// 将 details 字符串规范化为 JSON 对象（如果它是 `key=value ...` 格式）或保留原样。
    /// 这样前端可以直接解析结构化字段，而无需再用正则拆分 key=value。
    fn normalize_details_text(details: Option<&str>) -> Option<String> {
        let text = details?;
        // 若已经是合法 JSON（可能是调用方直接传入的对象/数组），按原样保留。
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
            return Some(value.to_string());
        }
        // 尝试解析 `key=value` 序列。
        let mut obj = serde_json::Map::new();
        let bytes = text.as_bytes();
        let len = bytes.len();
        let mut pos = 0usize;
        // 跳过前导空白。
        while pos < len && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let start = pos;
        while pos < len {
            // key
            let key_start = pos;
            while pos < len {
                let c = bytes[pos];
                if c == b'=' || c.is_ascii_whitespace() {
                    break;
                }
                pos += 1;
            }
            if pos >= len || bytes[pos] != b'=' {
                break;
            }
            let key = std::str::from_utf8(&bytes[key_start..pos]).ok()?;
            if key.is_empty() {
                break;
            }
            pos += 1; // skip '='
            let value_start = pos;
            // 查找下一个 ` key=` 位置作为 value 结束。
            let mut value_end = len;
            let mut scan = pos;
            while scan < len {
                if bytes[scan].is_ascii_whitespace() {
                    let mut after = scan + 1;
                    while after < len && bytes[after].is_ascii_whitespace() {
                        after += 1;
                    }
                    let key2_start = after;
                    while after < len && !bytes[after].is_ascii_whitespace() && bytes[after] != b'='
                    {
                        after += 1;
                    }
                    if after < len && bytes[after] == b'=' {
                        let key2 = std::str::from_utf8(&bytes[key2_start..after]).ok()?;
                        if !key2.is_empty() {
                            value_end = scan;
                            break;
                        }
                    }
                }
                scan += 1;
            }
            let value = std::str::from_utf8(&bytes[value_start..value_end])
                .ok()?
                .trim_end();
            obj.insert(
                key.to_string(),
                serde_json::Value::String(value.to_string()),
            );
            pos = value_end;
            while pos < len && bytes[pos].is_ascii_whitespace() {
                pos += 1;
            }
        }
        if start == 0 && !obj.is_empty() && pos == len {
            Some(serde_json::Value::Object(obj).to_string())
        } else {
            Some(text.to_string())
        }
    }

    /// Write a structured audit log entry with full fields.
    pub fn log_structured(
        &self,
        action_type: &str,
        entity_type: &str,
        entity_id: Option<&str>,
        entity_name: Option<&str>,
        performed_by: &str,
        details: Option<&str>,
    ) -> Result<(), String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let now = chrono::Utc::now().to_rfc3339();
        let encrypted_name = entity_name
            .map(|n| encrypt_text_field(&key, n))
            .transpose()?;
        let normalized = Self::normalize_details_text(details);
        let encrypted_details = normalized
            .as_deref()
            .map(|d| encrypt_text_field(&key, d))
            .transpose()?;
        conn.execute(
            "INSERT INTO audit_log (timestamp, action, entity_type, entity_id, entity_name, performed_by, details)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                now,
                action_type,
                entity_type,
                entity_id,
                encrypted_name,
                performed_by,
                encrypted_details,
            ],
        )
        .map_err(|e| format!("log_structured: {}", e))?;
        Ok(())
    }

    /// List recent audit log entries, newest first.
    pub fn list_audit_log(&self, limit: usize) -> Result<Vec<crate::AuditLogEntry>, String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        // 先收集原始行数据（不在此处解密），避免单行解密失败导致整个查询失败。
        // 这对于密码修改后部分审计日志可能用旧密钥加密的场景尤为重要。
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, action, entity_type, entity_id, entity_name, performed_by, details
             FROM audit_log ORDER BY id DESC LIMIT ?1"
        ).map_err(|e| format!("list_audit_log prepare: {}", e))?;
        // 原始审计日志行：id, timestamp, action, entity_type, entity_id, entity_name, performed_by, details
        type RawAuditRow = (
            i64,
            String,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        );
        let raw_rows: Vec<RawAuditRow> = stmt
            .query_map(rusqlite::params![limit as i64], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            })
            .map_err(|e| format!("list_audit_log query: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("list_audit_log collect: {}", e))?;
        drop(stmt);
        drop(guard);

        // 逐行解密，解密失败的条目跳过而非中断整个列表。
        let mut result = Vec::new();
        let mut skip_count = 0u32;
        for (
            id,
            timestamp,
            action_type,
            entity_type,
            entity_id,
            raw_name,
            performed_by,
            raw_details,
        ) in raw_rows
        {
            let entity_name = match raw_name.as_deref() {
                Some(n) => match decrypt_text_field(&key, n) {
                    Ok(dec) => Some(dec),
                    Err(e) => {
                        tracing::warn!(
                            "list_audit_log: entity_name decryption failed for entry id={}: {}",
                            id,
                            e
                        );
                        skip_count += 1;
                        Some(format!("[decryption error: {}]", e))
                    }
                },
                None => None,
            };
            let details = match raw_details.as_deref() {
                Some(d) => match decrypt_text_field(&key, d) {
                    Ok(dec) => Some(dec),
                    Err(e) => {
                        tracing::warn!(
                            "list_audit_log: skipping details for entry id={}, decryption failed: {}",
                            id, e
                        );
                        skip_count += 1;
                        Some(format!("[decryption error: {}]", e))
                    }
                },
                None => None,
            };
            result.push(crate::AuditLogEntry {
                id,
                timestamp,
                action_type,
                entity_type: entity_type.unwrap_or_default(),
                entity_id,
                entity_name,
                performed_by: performed_by.unwrap_or_else(|| "system".to_string()),
                details,
            });
        }
        if skip_count > 0 {
            tracing::warn!(
                "list_audit_log: skipped {} field(s) due to decryption failures (possible stale encryption after password change)",
                skip_count
            );
        }
        Ok(result)
    }

    // Metadata helpers for encrypted blob storage (reserved)
    fn read_metadata(&self, key: &str, prefix: &str) -> Result<Option<Vec<u8>>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let full_key = format!("{}_{}", prefix, key);
        let result: Option<String> = conn
            .query_row(
                "SELECT value FROM metadata WHERE key = ?1",
                params![full_key],
                |r| r.get(0),
            )
            .ok();
        match result {
            Some(encoded) => {
                use base64::Engine as _;
                let engine = base64::engine::general_purpose::STANDARD;
                let decoded = engine
                    .decode(&encoded)
                    .map_err(|e| format!("Base64 decode error: {}", e))?;
                Ok(Some(decoded))
            }
            None => Ok(None),
        }
    }

    fn write_metadata(&self, key: &str, prefix: &str, data: &[u8]) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let now = chrono::Utc::now().to_rfc3339();
        let full_key = format!("{}_{}", prefix, key);
        use base64::Engine as _;
        let engine = base64::engine::general_purpose::STANDARD;
        let encoded = engine.encode(data);
        conn.execute(
            "INSERT INTO metadata (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![full_key, encoded, now],
        ).map_err(|e| format!("Failed to write metadata: {}", e))?;
        Ok(())
    }

    // ── Guide embeddings for RAG (§RAG-1) ────────────────────────

    /// Batch-save guide embedding chunks in a single transaction (P051).
    /// 批量版本只开一次事务 + 复用 prepared statement，重建耗时大幅下降。
    pub fn save_guide_embeddings(
        &self,
        chunks: &[crate::GuideEmbeddingChunk],
    ) -> Result<(), String> {
        if chunks.is_empty() {
            return Ok(());
        }
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let tx = conn
            .transaction()
            .map_err(|e| format!("save_guide_embeddings begin: {}", e))?;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT OR REPLACE INTO guide_embeddings (id, guide_id, chunk_index, chunk_text, embedding, model, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                )
                .map_err(|e| format!("save_guide_embeddings prepare: {}", e))?;
            for (idx, chunk) in chunks.iter().enumerate() {
                let embedding_bytes: Vec<u8> = chunk
                    .embedding
                    .iter()
                    .flat_map(|f| f.to_ne_bytes())
                    .collect();
                stmt.execute(params![
                    chunk.id,
                    chunk.guide_id,
                    chunk.chunk_index,
                    chunk.chunk_text,
                    embedding_bytes,
                    chunk.model,
                    chunk.created_at
                ])
                .map_err(|e| format!("save_guide_embeddings chunk {}: {}", idx, e))?;
            }
        }
        tx.commit()
            .map_err(|e| format!("save_guide_embeddings commit: {}", e))?;
        Ok(())
    }

    /// Load all guide embedding chunks.
    pub fn list_guide_embeddings(&self) -> Result<Vec<crate::GuideEmbeddingChunk>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn
            .prepare(
                "SELECT id, guide_id, chunk_index, chunk_text, embedding, model, created_at
             FROM guide_embeddings ORDER BY guide_id, chunk_index",
            )
            .map_err(|e| format!("list_guide_embeddings prepare: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                let embedding_bytes: Vec<u8> = row.get(4)?;
                let embedding: Vec<f32> = embedding_bytes
                    .chunks_exact(4)
                    .map(|b| f32::from_ne_bytes([b[0], b[1], b[2], b[3]]))
                    .collect();
                Ok(crate::GuideEmbeddingChunk {
                    id: row.get(0)?,
                    guide_id: row.get(1)?,
                    chunk_index: row.get(2)?,
                    chunk_text: row.get(3)?,
                    embedding,
                    model: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })
            .map_err(|e| format!("list_guide_embeddings query: {}", e))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| format!("list_guide_embeddings row: {}", e))?);
        }
        Ok(result)
    }

    /// Clear all guide embeddings (used for rebuild).
    pub fn clear_guide_embeddings(&self) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        conn.execute("DELETE FROM guide_embeddings", [])
            .map_err(|e| format!("clear_guide_embeddings: {}", e))?;
        Ok(())
    }

    /// Get the count of guide embeddings.
    pub fn count_guide_embeddings(&self) -> Result<usize, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM guide_embeddings", [], |r| r.get(0))
            .map_err(|e| format!("count_guide_embeddings: {}", e))?;
        Ok(count as usize)
    }

    // ── sys_config helpers ────────────────────────────────────────

    /// Read a value from sys_config by key.
    pub fn get_sys_config(&self, key: &str) -> Result<Option<String>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let result: Option<String> = conn
            .query_row(
                "SELECT value FROM sys_config WHERE key = ?1",
                params![key],
                |r| r.get(0),
            )
            .ok();
        Ok(result)
    }

    /// Write or update a value in sys_config.
    pub fn set_sys_config(&self, key: &str, value: &str) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR REPLACE INTO sys_config (key, value, updated_at) VALUES (?1, ?2, ?3)",
            params![key, value, now],
        )
        .map_err(|e| format!("set_sys_config: {}", e))?;
        Ok(())
    }

    // ── User template helpers (§29 模板系统重构 P1) ──────────────

    /// Save or update a user template (UPSERT).
    pub fn save_user_template(&self, template: &crate::UserTemplate) -> Result<(), String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        Self::save_user_template_tx(conn, &key, template)
    }

    /// P115: 事务内保存用户模板（连接由调用方持有，批量应用单事务内复用）。
    fn save_user_template_tx(
        conn: &mut Connection,
        key: &DataEncryptionKey,
        template: &crate::UserTemplate,
    ) -> Result<(), String> {
        let props_json = serde_json::to_string(&template.properties)
            .map_err(|e| format!("serialize properties: {}", e))?;
        let encrypted_props = encrypt_text_field(key, &props_json)?;
        let mut stmt = conn
            .prepare_cached(USER_TEMPLATE_SAVE_SQL)
            .map_err(|e| format!("save_user_template prepare: {}", e))?;
        stmt.execute(params![
            &template.id,
            &template.account_id,
            &template.name,
            &template.icon_id,
            encrypted_props,
            &template.category,
            &template.contract_type_id,
            &template.created_at,
            &template.updated_at,
        ])
        .map_err(|e| format!("save_user_template: {}", e))?;
        Ok(())
    }

    /// Load a single user template by ID.
    pub fn load_user_template(
        &self,
        template_id: &str,
    ) -> Result<Option<crate::UserTemplate>, String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        Self::load_user_template_tx(conn, &key, template_id)
    }

    /// P115: 事务内加载用户模板（连接由调用方持有）。
    fn load_user_template_tx(
        conn: &mut Connection,
        key: &DataEncryptionKey,
        template_id: &str,
    ) -> Result<Option<crate::UserTemplate>, String> {
        let mut stmt = conn
            .prepare_cached(USER_TEMPLATE_LOAD_SQL)
            .map_err(|e| format!("prepare load_user_template: {}", e))?;

        let result = stmt.query_row(params![template_id], |row| {
            let props_json: String = row.get(4)?;
            let decrypted = decrypt_text_field(key, &props_json).map_err(|e| {
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
            Ok(crate::UserTemplate {
                contract_type_id: row.get(6)?,
                id: row.get(0)?,
                account_id: row.get(1)?,
                name: row.get(2)?,
                icon_id: row.get(3)?,
                properties,
                category: row.get(5)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        });

        match result {
            Ok(tpl) => Ok(Some(tpl)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("load_user_template: {}", e)),
        }
    }

    /// List all user templates for a given account, ordered by created_at DESC.
    pub fn list_user_templates(
        &self,
        account_id: &str,
    ) -> Result<Vec<crate::UserTemplate>, String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn.prepare(
            "SELECT id, account_id, name, icon_id, properties_json, category, contract_type_id, created_at, updated_at
             FROM user_templates WHERE account_id = ?1 ORDER BY created_at ASC"
        ).map_err(|e| format!("prepare list_user_templates: {}", e))?;

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
                Ok(crate::UserTemplate {
                    contract_type_id: row.get(6)?,
                    id: row.get(0)?,
                    account_id: row.get(1)?,
                    name: row.get(2)?,
                    icon_id: row.get(3)?,
                    properties,
                    category: row.get(5)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .map_err(|e| format!("list_user_templates query: {}", e))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| format!("list_user_templates row: {}", e))?);
        }
        Ok(result)
    }
    /// Find a user template by content hash.
    ///
    /// Scans all user templates for the given account, computing the content hash
    /// for each and returning the first match. Two templates with identical content
    /// (same name, icon_id, category, contract_type_id, and properties) produce the
    /// same hash regardless of their database IDs.
    pub fn find_user_template_by_content_hash(
        &self,
        account_id: &str,
        hash: &str,
    ) -> Result<Option<crate::UserTemplate>, String> {
        let templates = self.list_user_templates(account_id)?;
        for tpl in templates {
            if crate::template_hash::user_template_content_hash(&tpl) == hash {
                return Ok(Some(tpl));
            }
        }
        Ok(None)
    }

    /// Delete a user template by ID.
    pub fn delete_user_template(&self, template_id: &str) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        conn.execute(
            "DELETE FROM user_templates WHERE id = ?1",
            params![template_id],
        )
        .map_err(|e| format!("delete_user_template: {}", e))?;
        drop(guard);
        self.record_tombstone("user_templates", template_id)?;
        Ok(())
    }

    /// Count user templates for an account.
    pub fn count_user_templates(&self, account_id: &str) -> Result<usize, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM user_templates WHERE account_id = ?1",
                params![account_id],
                |r| r.get(0),
            )
            .map_err(|e| format!("count_user_templates: {}", e))?;
        Ok(count as usize)
    }

    /// Check whether a template field is used by any object (active or soft-deleted).
    /// Returns (active_count, soft_deleted_count).
    pub fn check_field_usage(
        &self,
        account_id: &str,
        field_key: &str,
    ) -> Result<(usize, usize), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let like = format!("%\"{}\":%", field_key);
        let active: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM objects WHERE account_id = ?1 AND properties LIKE ?2 AND is_deleted = 0",
                params![account_id, &like],
                |r| r.get(0),
            )
            .map_err(|e| format!("check_field_usage active: {}", e))?;
        let soft_deleted: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM objects WHERE account_id = ?1 AND properties LIKE ?2 AND is_deleted = 1",
                params![account_id, &like],
                |r| r.get(0),
            )
            .map_err(|e| format!("check_field_usage soft_deleted: {}", e))?;
        Ok((active as usize, soft_deleted as usize))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ObjectRecord, Profile, TrashItem};
    use tempfile::TempDir;

    fn test_key() -> [u8; 32] {
        [0x42u8; 32]
    }

    fn setup() -> (VaultStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let config =
            VaultConfig::new("test_account", dir.path().to_path_buf()).with_data_key(test_key());
        let vault = VaultStore::open(config).unwrap();
        (vault, dir)
    }

    #[test]
    fn test_vault_open() {
        let dir = TempDir::new().unwrap();
        let config = VaultConfig::new("test", dir.path().to_path_buf()).with_data_key(test_key());
        assert!(VaultStore::open(config).is_ok());
    }

    #[test]
    fn test_save_and_load_profile() {
        let (vault, _dir) = setup();
        let profile = Profile::new("test", vec![1, 2, 3, 4, 5]);
        vault.save_profile(&profile).unwrap();
        let loaded = vault.load_profile(&profile.id).unwrap().unwrap();
        assert_eq!(loaded.name, profile.name);
        assert_eq!(loaded.data, profile.data);
    }

    #[test]
    fn test_update_profile() {
        let (vault, _dir) = setup();
        let mut profile = Profile::new("test", vec![1, 2, 3]);
        vault.save_profile(&profile).unwrap();
        profile.update_data(vec![10, 20, 30, 40]);
        vault.save_profile(&profile).unwrap();
        let loaded = vault.load_profile(&profile.id).unwrap().unwrap();
        assert_eq!(loaded.data, vec![10, 20, 30, 40]);
        assert_eq!(loaded.version, 2);
    }

    #[test]
    fn test_delete_profile() {
        let (vault, _dir) = setup();
        let profile = Profile::new("test", vec![1, 2, 3]);
        vault.save_profile(&profile).unwrap();
        vault.delete_profile(&profile.id).unwrap();
        assert!(vault.load_profile(&profile.id).unwrap().is_none());
    }

    #[test]
    fn test_list_profiles() {
        let (vault, _dir) = setup();
        for i in 0..3 {
            vault
                .save_profile(&Profile::new(&format!("p{}", i), vec![i]))
                .unwrap();
        }
        assert_eq!(vault.list_profiles().unwrap().len(), 3);
    }

    #[test]
    fn test_lock() {
        let (vault, _dir) = setup();
        let vault = vault;
        let profile = Profile::new("test", vec![1, 2, 3]);
        vault.save_profile(&profile).unwrap();
        vault.lock();
        assert_eq!(vault.state(), VaultState::Locked);
    }

    #[test]
    fn test_vault_stats() {
        let (vault, _dir) = setup();
        let profile = Profile::new("test", vec![1, 2, 3, 4, 5]);
        vault.save_profile(&profile).unwrap();
        let stats = vault.stats().unwrap();
        assert_eq!(stats.profile_count, 1);
        assert!(stats.total_size_bytes > 0);
    }

    #[test]
    fn test_search_profiles() {
        let (vault, _dir) = setup();
        vault
            .save_profile(&Profile::new_with_id("alpha", "Alpha Profile", vec![1]))
            .unwrap();
        vault
            .save_profile(&Profile::new_with_id("beta", "Beta Profile", vec![2]))
            .unwrap();
        // search via list and filter in memory
        let all = vault.list_profiles().unwrap();
        assert!(all.iter().any(|p| p.name.contains("Alpha")));
    }

    // ── Error boundary tests ──────────────────────────────────

    #[test]
    fn test_load_nonexistent_profile_returns_none() {
        let (vault, _dir) = setup();
        assert!(vault.load_profile("does-not-exist").unwrap().is_none());
    }

    #[test]
    fn test_delete_nonexistent_profile_fails() {
        let (vault, _dir) = setup();
        let result = vault.delete_profile("does-not-exist");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_locked_vault_rejects_operations() {
        let (vault, _dir) = setup();
        vault.lock();
        assert_eq!(vault.state(), VaultState::Locked);

        let profile = Profile::new("test", vec![1, 2, 3]);
        assert!(vault.save_profile(&profile).is_err());
        assert!(vault.load_profile("test").is_err());
        assert!(vault.list_profiles().is_err());
        assert!(vault.stats().is_err());

        let obj = ObjectRecord {
            contract_type_id: None,
            id: "obj-1".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Test".to_string(),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::Value::Null,
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            template_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            ..Default::default()
        };
        assert!(vault.save_object(&obj).is_err());
        assert!(vault.load_object("obj-1").is_err());
        assert!(vault
            .list_objects("acc-1", None, None, None, false, false)
            .is_err());
    }

    #[test]
    fn test_concurrent_profile_writes() {
        let (vault, _dir) = setup();
        use std::sync::Arc;
        use std::thread;

        let vault_arc = Arc::new(vault);
        let mut handles = vec![];
        for i in 0..10 {
            let v = Arc::clone(&vault_arc);
            handles.push(thread::spawn(move || {
                let profile = Profile::new(&format!("concurrent-{}", i), vec![i as u8]);
                v.save_profile(&profile).unwrap();
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        let all = vault_arc.list_profiles().unwrap();
        assert_eq!(all.len(), 10);
    }

    #[test]
    fn test_object_crud_with_special_characters() {
        let (vault, _dir) = setup();
        let obj = ObjectRecord {
            contract_type_id: None,
            id: "obj-special".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Test \"quotes\" and 'apostrophes'".to_string(),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({"content": "Line1\nLine2\tTab"}),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec!["tag-with-dash".to_string()],
            template_id: None,
            template_type: None,
            template_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            ..Default::default()
        };
        vault.save_object(&obj).unwrap();
        let loaded = vault.load_object("obj-special").unwrap().unwrap();
        assert_eq!(loaded.name, "Test \"quotes\" and 'apostrophes'");
        assert_eq!(
            loaded.properties,
            serde_json::json!({"content": "Line1\nLine2\tTab"})
        );
    }

    #[test]
    fn test_object_soft_delete_and_restore() {
        let (vault, _dir) = setup();
        let obj = ObjectRecord {
            contract_type_id: None,
            id: "obj-del".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "To Delete".to_string(),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::Value::Null,
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            template_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            ..Default::default()
        };
        vault.save_object(&obj).unwrap();

        // Soft delete
        vault.delete_object("obj-del", true).unwrap();
        let active = vault
            .list_objects("acc-1", None, None, None, false, false)
            .unwrap();
        assert_eq!(active.len(), 0);
        let deleted = vault
            .list_objects("acc-1", None, None, None, false, true)
            .unwrap();
        assert_eq!(deleted.len(), 1);

        // Restore
        vault.restore_object("obj-del").unwrap();
        let restored = vault
            .list_objects("acc-1", None, None, None, false, false)
            .unwrap();
        assert_eq!(restored.len(), 1);
        assert!(!restored[0].is_deleted);
    }

    #[test]
    fn test_list_object_metadata_no_decrypt_but_identity_fields() {
        // P111: metadata-only 查询返回身份字段，负载列置占位值（不解密）。
        let (vault, _dir) = setup();
        let obj = ObjectRecord {
            contract_type_id: None,
            id: "obj-meta-1".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "note".to_string(),
            name: "元数据测试对象".to_string(),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({"title": "解密内容不应出现"}),
            property_labels: Some(serde_json::json!({"title": "sensitive"})),
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec!["tag-a".to_string()],
            template_id: Some("tpl-1".to_string()),
            template_type: Some("user".to_string()),
            template_hash: Some("hash-1".to_string()),
            ignored_template_hash: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            version: 1,
        };
        vault.save_object(&obj).unwrap();

        let metas = vault
            .list_object_metadata("acc-1", None, None, false, false)
            .unwrap();
        assert_eq!(metas.len(), 1);
        let m = &metas[0];
        // 身份字段完整
        assert_eq!(m.id, "obj-meta-1");
        assert_eq!(m.name, "元数据测试对象");
        assert_eq!(m.collection_type, "note");
        assert_eq!(m.section_type, "note");
        assert_eq!(m.sensitivity_level, "internal");
        assert_eq!(m.template_id.as_deref(), Some("tpl-1"));
        assert_eq!(m.icon_name, "document");
        assert!(!m.is_deleted);
        // 负载列占位：不返回解密内容
        assert_eq!(m.properties, serde_json::Value::Null);
        assert!(m.property_labels.is_none());
        assert!(m.tags.is_empty());

        // 与全量 list_objects 的身份字段一致
        let full = vault
            .list_objects("acc-1", None, None, None, false, false)
            .unwrap();
        assert_eq!(full.len(), 1);
        assert_eq!(full[0].id, m.id);
        assert_eq!(full[0].name, m.name);
        assert_eq!(full[0].sensitivity_level, m.sensitivity_level);
        assert_eq!(full[0].template_id, m.template_id);
        // 全量版本仍返回真实 properties
        assert_eq!(
            full[0].properties["title"],
            serde_json::Value::String("解密内容不应出现".to_string())
        );

        // type_id / parent_id 过滤生效
        assert!(vault
            .list_object_metadata("acc-1", Some("page"), None, false, false)
            .unwrap()
            .is_empty());
        assert!(
            vault
                .list_object_metadata("acc-1", Some("note"), None, false, false)
                .unwrap()
                .len()
                == 1
        );
        // 软删对象在 only_deleted=true 时才出现
        vault.delete_object("obj-meta-1", true).unwrap();
        assert!(vault
            .list_object_metadata("acc-1", None, None, false, false)
            .unwrap()
            .is_empty());
        let deleted = vault
            .list_object_metadata("acc-1", None, None, false, true)
            .unwrap();
        assert_eq!(deleted.len(), 1);
        assert!(deleted[0].is_deleted);
    }

    #[test]
    fn test_object_hard_delete() {
        let (vault, _dir) = setup();
        let obj = ObjectRecord {
            contract_type_id: None,
            id: "obj-hard".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "To Purge".to_string(),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::Value::Null,
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            template_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            ..Default::default()
        };
        vault.save_object(&obj).unwrap();
        vault.delete_object("obj-hard", false).unwrap();
        assert!(vault.load_object("obj-hard").unwrap().is_none());
    }

    #[test]
    fn test_list_objects_with_filters() {
        let (vault, _dir) = setup();
        for i in 0..5 {
            let obj = ObjectRecord {
                contract_type_id: None,
                id: format!("obj-{}", i),
                account_id: "acc-1".to_string(),
                type_id: if i % 2 == 0 { "note" } else { "task" }.to_string(),
                section_type: "identity".to_string(),
                name: format!("Item {}", i),
                icon_name: "document".to_string(),
                parent_id: if i == 0 {
                    None
                } else {
                    Some("obj-0".to_string())
                },
                children_ids: vec![],
                properties: serde_json::json!({"idx": i}),
                property_labels: None,
                sensitivity_level: if i == 0 { "public" } else { "internal" }.to_string(),
                is_deleted: false,
                deleted_at: None,
                tags_json: vec![],
                template_id: None,
                template_type: None,
                template_hash: None,
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
                version: 1,
                ..Default::default()
            };
            vault.save_object(&obj).unwrap();
        }

        let all = vault
            .list_objects("acc-1", None, None, None, false, false)
            .unwrap();
        assert_eq!(all.len(), 5);

        let notes = vault
            .list_objects("acc-1", Some("note"), None, None, false, false)
            .unwrap();
        assert_eq!(notes.len(), 3); // obj-0, obj-2, obj-4

        let children = vault
            .list_objects("acc-1", None, Some("obj-0"), None, false, false)
            .unwrap();
        assert_eq!(children.len(), 4); // obj-1..4

        let keyword = vault
            .list_objects("acc-1", None, None, Some("Item 2"), false, false)
            .unwrap();
        assert_eq!(keyword.len(), 1);
        assert_eq!(keyword[0].id, "obj-2");
    }

    #[test]
    fn test_load_nonexistent_object_returns_none() {
        let (vault, _dir) = setup();
        assert!(vault.load_object("ghost").unwrap().is_none());
    }

    #[test]
    fn test_profile_save_with_large_data() {
        let (vault, _dir) = setup();
        let big_data = vec![0u8; 1024 * 1024]; // 1MB
        let profile = Profile::new_with_id("big", "big", big_data.clone());
        vault.save_profile(&profile).unwrap();
        let loaded = vault.load_profile("big").unwrap().unwrap();
        assert_eq!(loaded.data.len(), 1024 * 1024);
        assert_eq!(loaded.data, big_data);
    }

    #[test]
    fn test_corrupted_db_file_fails_to_open() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("vault.db");
        // Write non-SQLite garbage
        std::fs::write(&db_path, b"this is not a sqlite database").unwrap();
        let config = VaultConfig::new("test", dir.path().to_path_buf());
        let result = VaultStore::open(config);
        assert!(result.is_err());
    }

    // ── Object CRUD edge cases ────────────────────────────────

    #[test]
    fn test_save_load_delete_object() {
        let (vault, _dir) = setup();
        let obj = ObjectRecord {
            contract_type_id: None,
            id: "obj-1".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Test Object".to_string(),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({"key": "value"}),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec!["tag1".to_string()],
            template_id: None,
            template_type: None,
            template_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            ..Default::default()
        };
        vault.save_object(&obj).unwrap();

        let loaded = vault.load_object("obj-1").unwrap().unwrap();
        assert_eq!(loaded.name, "Test Object");
        assert_eq!(loaded.properties, serde_json::json!({"key": "value"}));
        assert_eq!(loaded.tags_json, vec!["tag1".to_string()]);

        vault.delete_object("obj-1", false).unwrap();
        assert!(vault.load_object("obj-1").unwrap().is_none());
    }

    #[test]
    fn test_save_object_upsert() {
        let (vault, _dir) = setup();
        let obj = ObjectRecord {
            contract_type_id: None,
            id: "obj-upsert".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Original".to_string(),
            icon_name: "doc".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::Value::Null,
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            template_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            ..Default::default()
        };
        vault.save_object(&obj).unwrap();

        let mut updated = obj;
        updated.name = "Updated".to_string();
        updated.version = 2;
        vault.save_object(&updated).unwrap();

        let loaded = vault.load_object("obj-upsert").unwrap().unwrap();
        assert_eq!(loaded.name, "Updated");
        assert_eq!(loaded.version, 2);
    }

    #[test]
    fn test_list_objects_empty_collection() {
        let (vault, _dir) = setup();
        let list = vault
            .list_objects("acc-empty", None, None, None, false, false)
            .unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn test_list_objects_include_deleted() {
        let (vault, _dir) = setup();
        let obj = ObjectRecord {
            contract_type_id: None,
            id: "obj-del-1".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Deleted Item".to_string(),
            icon_name: "doc".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::Value::Null,
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            template_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            ..Default::default()
        };
        vault.save_object(&obj).unwrap();
        vault.delete_object("obj-del-1", true).unwrap();

        let active = vault
            .list_objects("acc-1", None, None, None, false, false)
            .unwrap();
        assert_eq!(active.len(), 0);

        let include_del = vault
            .list_objects("acc-1", None, None, None, true, false)
            .unwrap();
        assert_eq!(include_del.len(), 1);

        let only_del = vault
            .list_objects("acc-1", None, None, None, false, true)
            .unwrap();
        assert_eq!(only_del.len(), 1);
    }

    #[test]
    fn test_list_objects_keyword_unicode() {
        let (vault, _dir) = setup();
        let obj = ObjectRecord {
            contract_type_id: None,
            id: "obj-unicode".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "日本語テスト".to_string(),
            icon_name: "doc".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({"content": "你好世界"}),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            template_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            ..Default::default()
        };
        vault.save_object(&obj).unwrap();

        let by_name = vault
            .list_objects("acc-1", None, None, Some("日本語"), false, false)
            .unwrap();
        assert_eq!(by_name.len(), 1);

        let by_prop = vault
            .list_objects("acc-1", None, None, Some("你好"), false, false)
            .unwrap();
        assert_eq!(by_prop.len(), 1);
    }

    // ── P210 json_contains_ignore_case 单测 ──────────────────

    #[test]
    fn test_json_contains_string_value_case_insensitive() {
        let v = serde_json::json!({"content": "Hello World"});
        assert!(json_contains_ignore_case(&v, "hello"));
        assert!(json_contains_ignore_case(&v, "world"));
        assert!(!json_contains_ignore_case(&v, "xyz"));
    }

    #[test]
    fn test_json_contains_matches_object_key() {
        let v = serde_json::json!({"emailAddress": "a@b.c"});
        // 键命中（旧 to_string() 序列化也含键，保持搜索面）；needle 须已小写
        assert!(json_contains_ignore_case(&v, "email"));
        assert!(json_contains_ignore_case(&v, "address"));
    }

    #[test]
    fn test_json_contains_nested() {
        let v = serde_json::json!({
            "a": {
                "b": [
                    {"c": "deep value"},
                    "plain",
                    123
                ]
            }
        });
        assert!(json_contains_ignore_case(&v, "deep"));
        assert!(json_contains_ignore_case(&v, "plain"));
        // 数字按文本匹配
        assert!(json_contains_ignore_case(&v, "123"));
        assert!(!json_contains_ignore_case(&v, "nothing"));
    }

    #[test]
    fn test_json_contains_unicode() {
        let v = serde_json::json!({"内容": "你好世界"});
        assert!(json_contains_ignore_case(&v, "你好"));
        assert!(json_contains_ignore_case(&v, "世界"));
    }

    #[test]
    fn test_json_contains_empty_needle() {
        let v = serde_json::json!({"a": "b"});
        assert!(json_contains_ignore_case(&v, ""));
    }

    #[test]
    fn test_json_contains_non_text_scalars() {
        // 布尔/null 语义：布尔可文本匹配，null 不命中
        assert!(json_contains_ignore_case(&serde_json::json!(true), "true"));
        assert!(!json_contains_ignore_case(&serde_json::json!(null), "null"));
        assert!(!json_contains_ignore_case(&serde_json::json!(42), "forty"));
    }

    #[test]
    fn test_search_objects_matches_property_value_case_insensitive() {
        let (vault, _dir) = setup();
        let obj = ObjectRecord {
            contract_type_id: None,
            id: "obj-p210".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "No Match Here".to_string(),
            icon_name: "doc".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({"card": {"number": "SHADOW-2024"}}),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            template_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            ..Default::default()
        };
        vault.save_object(&obj).unwrap();

        // 嵌套属性值命中（大小写不敏感）；旧 to_string() 实现同样命中
        let results = vault.search_objects("acc-1", "shadow").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "obj-p210");

        // 对象键命中
        let by_key = vault.search_objects("acc-1", "card").unwrap();
        assert_eq!(by_key.len(), 1);
    }

    #[test]
    fn test_search_objects_basic() {
        let (vault, _dir) = setup();
        let obj = ObjectRecord {
            contract_type_id: None,
            id: "obj-search".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Searchable Name".to_string(),
            icon_name: "doc".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({"content": "find me"}),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            template_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            ..Default::default()
        };
        vault.save_object(&obj).unwrap();

        let results = vault.search_objects("acc-1", "Searchable").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "obj-search");

        let by_prop = vault.search_objects("acc-1", "find me").unwrap();
        assert_eq!(by_prop.len(), 1);
    }

    #[test]
    fn test_search_objects_no_results() {
        let (vault, _dir) = setup();
        let results = vault
            .search_objects("acc-1", "nonexistent-keyword-12345")
            .unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_objects_excludes_deleted() {
        let (vault, _dir) = setup();
        let obj = ObjectRecord {
            contract_type_id: None,
            id: "obj-s-del".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Will be deleted".to_string(),
            icon_name: "doc".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::Value::Null,
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            template_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            ..Default::default()
        };
        vault.save_object(&obj).unwrap();
        vault.delete_object("obj-s-del", true).unwrap();

        let results = vault.search_objects("acc-1", "Will be deleted").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_restore_object_nonexistent() {
        let (vault, _dir) = setup();
        // Should not error even if object doesn't exist (SQLite UPDATE with no match is OK)
        vault.restore_object("ghost-object").unwrap();
    }

    #[test]
    fn test_restore_object_already_active() {
        let (vault, _dir) = setup();
        let obj = ObjectRecord {
            contract_type_id: None,
            id: "obj-active".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Active".to_string(),
            icon_name: "doc".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::Value::Null,
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            template_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            ..Default::default()
        };
        vault.save_object(&obj).unwrap();
        vault.restore_object("obj-active").unwrap();
        let loaded = vault.load_object("obj-active").unwrap().unwrap();
        assert!(!loaded.is_deleted);
        assert!(loaded.deleted_at.is_none());
    }

    #[test]
    fn test_object_with_unicode_name() {
        let (vault, _dir) = setup();
        let obj = ObjectRecord {
            contract_type_id: None,
            id: "obj-uni".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "🚀 日本語 ñoël 中文".to_string(),
            icon_name: "🌍".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::Value::Null,
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec!["タグ".to_string()],
            template_id: None,
            template_type: None,
            template_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            ..Default::default()
        };
        vault.save_object(&obj).unwrap();
        let loaded = vault.load_object("obj-uni").unwrap().unwrap();
        assert_eq!(loaded.name, "🚀 日本語 ñoël 中文");
        assert_eq!(loaded.icon_name, "🌍");
        assert_eq!(loaded.tags_json, vec!["タグ".to_string()]);
    }

    #[test]
    fn test_object_template_fields_roundtrip() {
        let (vault, _dir) = setup();
        let obj = ObjectRecord {
            contract_type_id: None,
            id: "obj-tpl".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "passport".to_string(),
            section_type: "identity".to_string(),
            name: "My Passport".to_string(),
            icon_name: "passport".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({"fullName": "Alice"}),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: Some("passport".to_string()),
            template_type: Some("system".to_string()),
            template_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            ..Default::default()
        };
        vault.save_object(&obj).unwrap();
        let loaded = vault.load_object("obj-tpl").unwrap().unwrap();
        assert_eq!(loaded.template_id, Some("passport".to_string()));
        assert_eq!(loaded.template_type, Some("system".to_string()));

        // Update to remove template association
        let mut updated = loaded;
        updated.template_id = None;
        updated.template_type = None;
        vault.save_object(&updated).unwrap();
        let reloaded = vault.load_object("obj-tpl").unwrap().unwrap();
        assert_eq!(reloaded.template_id, None);
        assert_eq!(reloaded.template_type, None);
    }

    #[test]
    fn test_object_with_long_name_and_properties() {
        let (vault, _dir) = setup();
        let long_name = "a".repeat(5000);
        let big_props = serde_json::json!({
            "content": "x".repeat(10000),
            "nested": {
                "array": (0..100).collect::<Vec<i32>>(),
            }
        });
        let obj = ObjectRecord {
            contract_type_id: None,
            id: "obj-long".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: long_name,
            icon_name: "doc".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: big_props.clone(),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            template_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            ..Default::default()
        };
        vault.save_object(&obj).unwrap();
        let loaded = vault.load_object("obj-long").unwrap().unwrap();
        assert_eq!(loaded.name.len(), 5000);
        assert_eq!(loaded.properties, big_props);
    }

    #[test]
    fn test_object_with_empty_name() {
        let (vault, _dir) = setup();
        let obj = ObjectRecord {
            contract_type_id: None,
            id: "obj-empty-name".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "".to_string(),
            icon_name: "doc".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::Value::Null,
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            template_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            ..Default::default()
        };
        vault.save_object(&obj).unwrap();
        let loaded = vault.load_object("obj-empty-name").unwrap().unwrap();
        assert_eq!(loaded.name, "");
    }

    // ── Profile edge cases ────────────────────────────────────

    #[test]
    fn test_save_profile_empty_data() {
        let (vault, _dir) = setup();
        let profile = Profile::new_with_id("empty", "Empty Profile", vec![]);
        vault.save_profile(&profile).unwrap();
        let loaded = vault.load_profile("empty").unwrap().unwrap();
        assert!(loaded.data.is_empty());
    }

    #[test]
    fn test_save_profile_unicode_name() {
        let (vault, _dir) = setup();
        let profile = Profile::new_with_id("uni", "プロフィール 🎌", vec![1, 2, 3]);
        vault.save_profile(&profile).unwrap();
        let loaded = vault.load_profile("uni").unwrap().unwrap();
        assert_eq!(loaded.name, "プロフィール 🎌");
    }

    #[test]
    fn test_profile_version_increment_on_update() {
        let (vault, _dir) = setup();
        let mut profile = Profile::new_with_id("ver", "Version Test", vec![1]);
        vault.save_profile(&profile).unwrap();
        profile.update_data(vec![2]);
        vault.save_profile(&profile).unwrap();
        profile.update_data(vec![3]);
        vault.save_profile(&profile).unwrap();
        let loaded = vault.load_profile("ver").unwrap().unwrap();
        assert_eq!(loaded.version, 3);
    }

    // ── Trash CRUD ────────────────────────────────────────────

    #[test]
    fn test_trash_crud() {
        let (vault, _dir) = setup();
        let item = TrashItem {
            id: "trash-1".to_string(),
            item_type: "object".to_string(),
            original_id: "orig-1".to_string(),
            original_parent_id: Some("parent-1".to_string()),
            original_section_type: Some("identity".to_string()),
            original_sort_order: Some(42),
            data: vec![1, 2, 3, 4, 5],
            deleted_at: chrono::Utc::now().timestamp_millis(),
            expires_at: Some(chrono::Utc::now().timestamp_millis() + 86400000),
            deleted_by: "user".to_string(),
            name_snapshot: "Deleted Object".to_string(),
            icon_snapshot: Some("icon-1".to_string()),
        };
        vault.save_trash_item(&item).unwrap();

        let loaded = vault.get_trash_item("trash-1").unwrap().unwrap();
        assert_eq!(loaded.original_id, "orig-1");
        assert_eq!(loaded.data, vec![1, 2, 3, 4, 5]);

        let list = vault.list_trash_items(None, None).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Deleted Object");

        vault.delete_trash_item("trash-1").unwrap();
        assert!(vault.get_trash_item("trash-1").unwrap().is_none());
    }

    #[test]
    fn test_trash_and_soft_delete_batch() {
        let (vault, _dir) = setup();
        let obj_ids: Vec<String> = (0..2).map(|i| format!("batch-obj-{}", i)).collect();
        for (i, id) in obj_ids.iter().enumerate() {
            let obj = ObjectRecord {
                id: id.clone(),
                account_id: "acc-1".to_string(),
                name: format!("Batch Object {}", i),
                properties: serde_json::json!({ "k": "v" }),
                ..Default::default()
            };
            vault.save_object(&obj).unwrap();
        }

        let items: Vec<TrashItem> = (0..2)
            .map(|i| TrashItem {
                id: format!("trash-batch-{}", i),
                item_type: "object".to_string(),
                original_id: format!("batch-obj-{}", i),
                original_parent_id: None,
                original_section_type: Some("identity".to_string()),
                original_sort_order: None,
                data: vec![1, 2, 3],
                deleted_at: 1,
                expires_at: Some(2),
                deleted_by: "user".to_string(),
                name_snapshot: format!("Batch Object {}", i),
                icon_snapshot: None,
            })
            .collect();

        vault.trash_and_soft_delete_batch(&items, &obj_ids).unwrap();

        // 回收站两条（含数据往返）
        let list = vault.list_trash_items(None, None).unwrap();
        assert_eq!(list.len(), 2);
        for i in 0..2 {
            let loaded = vault
                .get_trash_item(&format!("trash-batch-{}", i))
                .unwrap()
                .unwrap();
            assert_eq!(loaded.data, vec![1, 2, 3]);
        }
        // 对象已软删
        let deleted = vault
            .list_object_metadata("acc-1", None, None, true, true)
            .unwrap();
        assert_eq!(deleted.len(), 2);
        let id_set: std::collections::HashSet<String> = obj_ids.iter().cloned().collect();
        for d in &deleted {
            assert!(id_set.contains(&d.id));
        }
        // 非软删列表中不再出现
        let live = vault
            .list_object_metadata("acc-1", None, None, false, false)
            .unwrap();
        assert!(live.is_empty());
    }

    #[test]
    fn test_trash_and_soft_delete_batch_empty() {
        let (vault, _dir) = setup();
        vault.trash_and_soft_delete_batch(&[], &[]).unwrap();
        assert!(vault.list_trash_items(None, None).unwrap().is_empty());
    }

    #[test]
    fn test_save_objects_batch() {
        let (vault, _dir) = setup();
        let objs: Vec<ObjectRecord> = (0..3)
            .map(|i| ObjectRecord {
                id: format!("batch-save-{}", i),
                account_id: "acc-1".to_string(),
                name: format!("Batch Save {}", i),
                properties: serde_json::json!({ "n": i }),
                ..Default::default()
            })
            .collect();

        vault.save_objects_batch(&objs).unwrap();

        for o in &objs {
            let loaded = vault.load_object(&o.id).unwrap().unwrap();
            assert_eq!(loaded.name, o.name);
            assert_eq!(loaded.properties["n"], o.properties["n"]);
        }
        // 空批量 no-op
        vault.save_objects_batch(&[]).unwrap();
    }

    #[test]
    fn test_list_trash_items_filter_by_type() {
        let (vault, _dir) = setup();
        for t in &["page", "collection", "object"] {
            let item = TrashItem {
                id: format!("trash-{}", t),
                item_type: t.to_string(),
                original_id: format!("orig-{}", t),
                original_parent_id: None,
                original_section_type: None,
                original_sort_order: None,
                data: vec![],
                deleted_at: chrono::Utc::now().timestamp_millis(),
                expires_at: None,
                deleted_by: "user".to_string(),
                name_snapshot: format!("{} item", t),
                icon_snapshot: None,
            };
            vault.save_trash_item(&item).unwrap();
        }

        let pages = vault.list_trash_items(Some("page"), None).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].item_type, "page");

        let all = vault.list_trash_items(None, None).unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_list_trash_items_filter_by_since() {
        let (vault, _dir) = setup();
        let now = chrono::Utc::now().timestamp_millis();
        let old_item = TrashItem {
            id: "trash-old".to_string(),
            item_type: "object".to_string(),
            original_id: "orig-old".to_string(),
            original_parent_id: None,
            original_section_type: None,
            original_sort_order: None,
            data: vec![],
            deleted_at: now - 10000,
            expires_at: None,
            deleted_by: "user".to_string(),
            name_snapshot: "Old".to_string(),
            icon_snapshot: None,
        };
        let new_item = TrashItem {
            id: "trash-new".to_string(),
            item_type: "object".to_string(),
            original_id: "orig-new".to_string(),
            original_parent_id: None,
            original_section_type: None,
            original_sort_order: None,
            data: vec![],
            deleted_at: now,
            expires_at: None,
            deleted_by: "user".to_string(),
            name_snapshot: "New".to_string(),
            icon_snapshot: None,
        };
        vault.save_trash_item(&old_item).unwrap();
        vault.save_trash_item(&new_item).unwrap();

        let recent = vault.list_trash_items(None, Some(now - 5000)).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].id, "trash-new");
    }

    // ── R-2: trash_items.deleted_at 为毫秒，HLC 回退 wall 不得放大 1000× ────
    //
    // 生产写入 deleted_at 一律使用 `Utc::now().timestamp_millis()`（page_delete /
    // delete_object / template_delete / objects.rs），但 list_trash_changes_since 曾
    // 用 `from_timestamp(deleted_at, 0)` 把毫秒按秒解释，得到放大 1000× 的垃圾
    // wall_time（约 58534 年）。本测试锁定回退 HLC 的 wall 必须精确等于 deleted_at
    // 毫秒值——修复前该断言会失败（wall == deleted_at * 1000）。
    #[test]
    fn test_trash_changes_since_honors_millisecond_deleted_at() {
        let (vault, _dir) = setup();
        // 2024-01-01T00:00:00.123Z
        let deleted_ms = 1704067200123i64;
        let item = TrashItem {
            id: "trash-ms".to_string(),
            item_type: "object".to_string(),
            original_id: "orig-ms".to_string(),
            original_parent_id: None,
            original_section_type: None,
            original_sort_order: None,
            data: vec![1, 2, 3],
            deleted_at: deleted_ms,
            expires_at: None,
            deleted_by: "user".to_string(),
            name_snapshot: "Ms".to_string(),
            icon_snapshot: None,
        };
        vault.save_trash_item(&item).unwrap();

        let records = vault
            .list_sync_changes_since(
                "trash_items",
                &crate::SyncWatermark::default(),
                "acc",
                "local",
            )
            .unwrap();
        assert_eq!(records.len(), 1);
        // R-2：回退 HLC 的 wall 必须等于 deleted_at 毫秒值（修复前按秒解释为
        // 1704067200123 秒 → wall 放大 1000× 的垃圾值）。
        assert_eq!(
            records[0].hlc.wall_time_ms, deleted_ms as u64,
            "trash HLC 回退 wall 必须精确等于 deleted_at 毫秒值"
        );
    }

    // ── R-1: trash_items keyset 分页——同 deleted_at 回退行跨页不得漏发 ──────
    //
    // P110 同构缺陷：page_delete 给整页对象同一个 deleted_at 毫秒值，通用小表分页
    // 「严格 hlc_after_watermark + 内存 take(limit)」在删除含 >limit 对象的页面时
    // 第 2 页空页 break——剩余 trash_items 永久不同步。keyset 化后（(有效 HLC,
    // t.id) 全序 + 游标推进 + 等值组尾部放行）跨页不重不漏。
    #[test]
    fn test_paginated_trash_keyset_equal_deleted_at_completeness() {
        let (vault, _dir) = setup();
        // page_delete 生产场景：整页对象同一个 deleted_at 毫秒值
        let deleted_ms = 1704067200123i64;
        let watermark = crate::SyncWatermark {
            wall_time_ms: (deleted_ms - 1000) as u64,
            counter: 0,
            node_id: "peer_x".to_string(),
        };
        for i in 1..=7usize {
            vault
                .save_trash_item(&TrashItem {
                    id: format!("trash_{:02}", i),
                    item_type: "object".to_string(),
                    original_id: format!("orig_{:02}", i),
                    original_parent_id: None,
                    original_section_type: None,
                    original_sort_order: None,
                    data: vec![i as u8],
                    deleted_at: deleted_ms,
                    expires_at: None,
                    deleted_by: "user".to_string(),
                    name_snapshot: format!("trash_{:02}", i),
                    icon_snapshot: None,
                })
                .unwrap();
        }

        let paged_ids = collect_paginated_trash_ids(&vault, watermark.clone(), "local_node", 2);

        // 全部 7 条必须无缺漏、无重复
        assert_eq!(paged_ids.len(), 7, "等值 HLC 回退行不得因页边界漏发");
        let uniq: std::collections::HashSet<&str> = paged_ids.iter().map(|s| s.as_str()).collect();
        assert_eq!(uniq.len(), 7, "keyset 分页不得重复投递");
        // 同 HLC 组内按 id 升序（trash_01..trash_07 字典序 == 数字序）
        assert_eq!(
            paged_ids,
            vec![
                "trash_01".to_string(),
                "trash_02".to_string(),
                "trash_03".to_string(),
                "trash_04".to_string(),
                "trash_05".to_string(),
                "trash_06".to_string(),
                "trash_07".to_string(),
            ],
            "等值 HLC 组内必须按 id 升序稳定分页"
        );
    }

    // ── R-1: 真实 HLC 行（对端应用写入）与回退行混合时按有效 HLC 排序分页 ──
    #[test]
    fn test_paginated_trash_keyset_mixed_real_hlc_ordering() {
        let (vault, _dir) = setup();
        // 回退行：本地删除，无 sync_hlc 行，有效 HLC wall == deleted_at 毫秒
        let local_ms = 1704067200123i64;
        // 真实 HLC 行：模拟对端应用写入（wall 更早、counter 更高）
        let peer_wall = 1704066000000u64;
        for i in 1..=3usize {
            vault
                .save_trash_item(&TrashItem {
                    id: format!("local_{:02}", i),
                    item_type: "object".to_string(),
                    original_id: format!("orig_l{:02}", i),
                    original_parent_id: None,
                    original_section_type: None,
                    original_sort_order: None,
                    data: vec![i as u8],
                    deleted_at: local_ms,
                    expires_at: None,
                    deleted_by: "user".to_string(),
                    name_snapshot: format!("local_{:02}", i),
                    icon_snapshot: None,
                })
                .unwrap();
        }
        for i in 1..=3usize {
            let id = format!("peer_{:02}", i);
            vault
                .save_trash_item(&TrashItem {
                    id: id.clone(),
                    item_type: "object".to_string(),
                    original_id: format!("orig_p{:02}", i),
                    original_parent_id: None,
                    original_section_type: None,
                    original_sort_order: None,
                    data: vec![(i + 10) as u8],
                    deleted_at: local_ms,
                    expires_at: None,
                    deleted_by: "peer".to_string(),
                    name_snapshot: format!("peer_{:02}", i),
                    icon_snapshot: None,
                })
                .unwrap();
            // 对端行写入真实 HLC（wall 更早 → 有效排序应在本地回退行之前）
            vault
                .set_record_hlc(
                    "trash_items",
                    &id,
                    &crate::RecordHlc {
                        wall_time_ms: peer_wall,
                        counter: i as u32,
                        node_id: "peer_node".to_string(),
                    },
                )
                .unwrap();
        }

        let watermark = crate::SyncWatermark::default();
        let paged_ids = collect_paginated_trash_ids(&vault, watermark.clone(), "local_node", 2);

        // 有效 HLC 排序：真实 HLC（peer_wall 早）在前，回退行（local_ms 晚）在后
        assert_eq!(
            paged_ids,
            vec![
                "peer_01".to_string(),
                "peer_02".to_string(),
                "peer_03".to_string(),
                "local_01".to_string(),
                "local_02".to_string(),
                "local_03".to_string(),
            ],
            "真实 HLC 行与回退行必须按有效 HLC 全序稳定分页"
        );
    }

    /// R-1 回归测试共用：以会话层同款 keyset 迭代（每页把水印推进到本页最大有效
    /// HLC、页游标推进到本页最后一条 id）逐页收集 trash_items 变更 id。
    fn collect_paginated_trash_ids(
        vault: &VaultStore,
        mut watermark: crate::SyncWatermark,
        local_node_id: &str,
        limit: usize,
    ) -> Vec<String> {
        let mut out = Vec::new();
        let mut last_row_id: Option<String> = None;
        loop {
            let page = vault
                .list_sync_changes_since_paginated(
                    "trash_items",
                    &watermark,
                    "acc",
                    local_node_id,
                    limit,
                    last_row_id.as_deref(),
                )
                .unwrap();
            if page.is_empty() {
                break;
            }
            for rec in &page {
                out.push(rec.id.clone());
            }
            // 水印推进到本页最大有效 HLC（与会话层 update_peer_watermark(max) 一致）
            if let Some((w, c, n)) = page
                .iter()
                .map(|r| (r.hlc.wall_time_ms, r.hlc.counter, r.hlc.node_id.clone()))
                .max()
            {
                watermark = crate::SyncWatermark {
                    wall_time_ms: w,
                    counter: c,
                    node_id: n,
                };
            }
            last_row_id = page.last().map(|r| r.id.clone());
        }
        out
    }

    // ── R-3: 页游标持久化——会话中断后等值 HLC 组尾部跨会话续传 ───────────
    //
    // N-1 已声明残余：keyset 页游标仅存内存，会话中断（断网/崩溃/退出）后已持久化
    // 水印停在等值组最大值而游标丢失，重启以 NULL 游标重查会跳过三元组 == 水印的
    // 组尾行（at-least-once 缺口）。R-3 把游标并入 sync_watermarks.cursor_id，
    // 中断后从 get_peer_watermark_cursor 恢复续传。
    #[test]
    fn test_peer_watermark_cursor_roundtrip() {
        let (vault, _dir) = setup();
        let wm = crate::SyncWatermark {
            wall_time_ms: 1704067200123,
            counter: 0,
            node_id: "local_node".to_string(),
        };
        assert_eq!(
            vault.get_peer_watermark_cursor("peer1", "objects").unwrap(),
            None
        );

        vault
            .update_peer_watermark_with_cursor("peer1", "objects", &wm, Some("obj_05"))
            .unwrap();
        assert_eq!(
            vault.get_peer_watermark_cursor("peer1", "objects").unwrap(),
            Some("obj_05".to_string())
        );
        // 读回水印不受游标污染
        assert_eq!(vault.get_peer_watermark("peer1", "objects").unwrap(), wm);

        // 清空游标
        vault
            .update_peer_watermark_with_cursor("peer1", "objects", &wm, None)
            .unwrap();
        assert_eq!(
            vault.get_peer_watermark_cursor("peer1", "objects").unwrap(),
            None
        );
    }

    #[test]
    fn test_peer_watermark_cursor_resume_delivers_equal_hlc_tail() {
        let (vault, _dir) = setup();
        let ts = "2026-08-01T12:00:00.000+00:00";
        // 5 个同 updated_at 回退行（等值 HLC 组，模拟 page_delete 同 ms 批量删除）
        for i in 1..=5usize {
            vault
                .save_object(&crate::ObjectRecord {
                    id: format!("resume_{:02}", i),
                    account_id: "test_account".to_string(),
                    name: format!("resume_{:02}", i),
                    section_type: "identity".to_string(),
                    properties: serde_json::json!({ "k": i }),
                    sensitivity_level: "internal".to_string(),
                    created_at: ts.to_string(),
                    updated_at: ts.to_string(),
                    ..Default::default()
                })
                .unwrap();
        }

        let local_hlc_node = "local_node";
        let peer = "peer_r3";

        // 会话 1：只同步第 1 页（limit=2）→ 水印推进到组最大值 T、游标 = 本页最后一条
        let watermark = crate::SyncWatermark::default();
        let page1 = vault
            .list_sync_changes_since_paginated(
                "objects",
                &watermark,
                "test_account",
                local_hlc_node,
                2,
                None,
            )
            .unwrap();
        assert_eq!(page1.len(), 2);
        let max1 = page1
            .iter()
            .map(|r| (r.hlc.wall_time_ms, r.hlc.counter, r.hlc.node_id.clone()))
            .max()
            .unwrap();
        let wm1 = crate::SyncWatermark {
            wall_time_ms: max1.0,
            counter: max1.1,
            node_id: max1.2,
        };
        let cursor1 = page1.last().map(|r| r.id.clone()).unwrap();
        vault
            .update_peer_watermark_with_cursor(peer, "objects", &wm1, Some(&cursor1))
            .unwrap();
        let mut all: Vec<String> = page1.iter().map(|r| r.id.clone()).collect();

        // 会话 2（模拟中断恢复）：从持久化水印 + 游标续传剩余记录
        let wm2 = vault.get_peer_watermark(peer, "objects").unwrap();
        let mut last_row_id = vault.get_peer_watermark_cursor(peer, "objects").unwrap();
        loop {
            let page = vault
                .list_sync_changes_since_paginated(
                    "objects",
                    &wm2,
                    "test_account",
                    local_hlc_node,
                    2,
                    last_row_id.as_deref(),
                )
                .unwrap();
            if page.is_empty() {
                break;
            }
            for rec in &page {
                all.push(rec.id.clone());
            }
            last_row_id = page.last().map(|r| r.id.clone());
        }

        // 全部 5 条无缺漏无重复（修复前：NULL 游标重查严格 > 水印 → 组尾 3 条永久跳过）
        assert_eq!(all.len(), 5, "会话中断后等值 HLC 组尾部必须续传，不得漏发");
        let uniq: std::collections::HashSet<&str> = all.iter().map(|s| s.as_str()).collect();
        assert_eq!(uniq.len(), 5, "续传不得重复投递");
        assert_eq!(
            all,
            vec![
                "resume_01".to_string(),
                "resume_02".to_string(),
                "resume_03".to_string(),
                "resume_04".to_string(),
                "resume_05".to_string(),
            ],
            "续传必须从游标后按 id 升序继续"
        );
    }

    #[test]
    fn test_get_trash_item_nonexistent() {
        let (vault, _dir) = setup();
        assert!(vault.get_trash_item("does-not-exist").unwrap().is_none());
    }

    #[test]
    fn test_delete_trash_item_nonexistent() {
        let (vault, _dir) = setup();
        // DELETE on non-existing row should succeed (no affected rows check)
        vault.delete_trash_item("does-not-exist").unwrap();
    }

    // ── Snapshot CRUD ─────────────────────────────────────────

    #[test]
    fn test_snapshot_save_and_get() {
        let (vault, _dir) = setup();
        let data = b"snapshot data";
        vault
            .save_snapshot("obj-1", "user_edit", data, "added field")
            .unwrap();

        let snapshots = vault.list_snapshots("obj-1").unwrap();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0]["triggeredBy"], "user_edit");

        let snapshot_id = snapshots[0]["id"].as_str().unwrap();
        let loaded = vault.get_snapshot(snapshot_id).unwrap().unwrap();
        assert_eq!(loaded, data);
    }

    #[test]
    fn test_get_snapshot_nonexistent() {
        let (vault, _dir) = setup();
        assert!(vault.get_snapshot("nonexistent-id").unwrap().is_none());
    }

    #[test]
    fn test_save_snapshot_at_preserves_timestamp() {
        let (vault, _dir) = setup();
        // 跨设备恢复需要保留旧设备上的原始时间戳，保证历史顺序一致
        let original_ts = 1_700_000_000_000i64; // 2023-11-14T22:13:20Z
        vault
            .save_snapshot_at(
                "obj-1",
                "user_edit",
                b"snap data",
                "diff_updated",
                original_ts,
            )
            .unwrap();

        let snapshots = vault.list_snapshots("obj-1").unwrap();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0]["timestamp"], original_ts);
        assert_eq!(snapshots[0]["triggeredBy"], "user_edit");
        assert_eq!(snapshots[0]["diffSummary"], "diff_updated");

        // 数据仍可解密读取
        let snapshot_id = snapshots[0]["id"].as_str().unwrap();
        let loaded = vault.get_snapshot(snapshot_id).unwrap().unwrap();
        assert_eq!(loaded, b"snap data");
    }

    #[test]
    fn test_save_snapshot_at_multiple_preserves_order() {
        let (vault, _dir) = setup();
        // 旧时间戳在后写入，仍应排在列表最前（timestamp DESC）
        vault
            .save_snapshot_at(
                "obj-1",
                "user_edit",
                b"newer",
                "diff_updated",
                2_000_000_000_000i64,
            )
            .unwrap();
        vault
            .save_snapshot_at(
                "obj-1",
                "user_edit",
                b"older",
                "diff_created",
                1_000_000_000_000i64,
            )
            .unwrap();

        let snapshots = vault.list_snapshots("obj-1").unwrap();
        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0]["timestamp"], 2_000_000_000_000i64);
        assert_eq!(snapshots[1]["timestamp"], 1_000_000_000_000i64);
    }

    #[test]
    fn test_count_snapshots_batch() {
        let (vault, _dir) = setup();
        vault
            .save_snapshot("obj-a", "user_edit", b"a1", "")
            .unwrap();
        vault
            .save_snapshot("obj-a", "user_edit", b"a2", "")
            .unwrap();
        vault
            .save_snapshot("obj-b", "user_edit", b"b1", "")
            .unwrap();

        let counts = vault
            .count_snapshots_batch(&[
                "obj-a".to_string(),
                "obj-b".to_string(),
                "obj-c".to_string(),
            ])
            .unwrap();
        assert_eq!(counts.get("obj-a"), Some(&2));
        assert_eq!(counts.get("obj-b"), Some(&1));
        // 纯计数：没有 snapshot 的对象不会出现在结果中
        assert_eq!(counts.get("obj-c"), None);
    }

    #[test]
    fn test_count_snapshots_batch_empty() {
        let (vault, _dir) = setup();
        let counts = vault.count_snapshots_batch(&[]).unwrap();
        assert!(counts.is_empty());
    }

    #[test]
    fn test_backfill_missing_snapshots() {
        let (vault, _dir) = setup();
        let now = chrono::Utc::now().to_rfc3339();
        let record = ObjectRecord {
            id: "obj-no-snap".to_string(),
            account_id: "test_account".to_string(),
            type_id: "identity".to_string(),
            section_type: "identity".to_string(),
            name: "No Snapshot".to_string(),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({"note": "initial"}),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            contract_type_id: None,
            template_hash: None,
            created_at: now.clone(),
            updated_at: now,
            version: 1,
            ..Default::default()
        };
        vault.save_object(&record).unwrap();

        // setup() 已触发过一次 backfill 并设置了标记，先重置标记以测试本次迁移
        vault.set_sys_config("snapshot_backfill_v1", "0").unwrap();

        // 首次 backfill 应为无 snapshot 的对象创建一条初始 snapshot
        let created = vault.backfill_missing_snapshots().unwrap();
        assert_eq!(created, 1);
        let counts = vault
            .count_snapshots_batch(&["obj-no-snap".to_string()])
            .unwrap();
        assert_eq!(counts.get("obj-no-snap"), Some(&1));

        // 再次调用应被标记跳过
        let created2 = vault.backfill_missing_snapshots().unwrap();
        assert_eq!(created2, 0);
    }

    #[test]
    fn test_delete_snapshots() {
        let (vault, _dir) = setup();
        vault
            .save_snapshot("obj-del", "user_edit", b"data1", "summary1")
            .unwrap();
        vault
            .save_snapshot("obj-del", "user_edit", b"data2", "summary2")
            .unwrap();
        assert_eq!(vault.list_snapshots("obj-del").unwrap().len(), 2);

        vault.delete_snapshots("obj-del").unwrap();
        assert_eq!(vault.list_snapshots("obj-del").unwrap().len(), 0);

        // 不存在的对象删除也是幂等成功
        vault.delete_snapshots("no-such-obj").unwrap();
    }

    #[test]
    fn test_snapshots_size_batch() {
        let (vault, _dir) = setup();
        vault
            .save_snapshot("obj-sz", "user_edit", b"small", "")
            .unwrap();
        vault
            .save_snapshot("obj-sz", "user_edit", b"a bit longer snapshot data", "")
            .unwrap();
        vault
            .save_snapshot("obj-sz2", "user_edit", b"x", "")
            .unwrap();

        // 空列表返回 0
        assert_eq!(vault.snapshots_size_batch(&[]).unwrap(), 0);

        // obj-sz 有 2 条，obj-sz2 有 1 条，长度应等于加密前字节数之和（LENGTH(data) 为密文，非明文；
        // 只验证非零且单对象 ≥ 另一对象，不做明文长度断言）。
        let size1 = vault.snapshots_size_batch(&["obj-sz".to_string()]).unwrap();
        let size2 = vault
            .snapshots_size_batch(&["obj-sz2".to_string()])
            .unwrap();
        let both = vault
            .snapshots_size_batch(&["obj-sz".to_string(), "obj-sz2".to_string()])
            .unwrap();
        assert!(size1 > 0);
        assert!(size2 > 0);
        assert_eq!(both, size1 + size2);

        // 不存在的对象贡献 0
        let none = vault
            .snapshots_size_batch(&["no-such-obj".to_string()])
            .unwrap();
        assert_eq!(none, 0);
    }

    #[test]
    fn test_copy_snapshots() {
        let (vault, _dir) = setup();
        vault
            .save_snapshot("src-obj", "user_edit", b"data1", "summary1")
            .unwrap();
        vault
            .save_snapshot("src-obj", "auto_save", b"data2", "summary2")
            .unwrap();

        vault.copy_snapshots("src-obj", "dst-obj").unwrap();

        let src_list = vault.list_snapshots("src-obj").unwrap();
        let dst_list = vault.list_snapshots("dst-obj").unwrap();
        assert_eq!(dst_list.len(), 2);
        assert_eq!(src_list.len(), 2);

        // IDs should differ because copy uses randomblob
        let src_ids: std::collections::HashSet<String> = src_list
            .iter()
            .map(|s| s["id"].as_str().unwrap().to_string())
            .collect();
        let dst_ids: std::collections::HashSet<String> = dst_list
            .iter()
            .map(|s| s["id"].as_str().unwrap().to_string())
            .collect();
        assert!(src_ids.is_disjoint(&dst_ids));
    }

    #[test]
    fn test_copy_snapshots_empty_source() {
        let (vault, _dir) = setup();
        vault.copy_snapshots("no-snapshots", "dst-obj").unwrap();
        let dst_list = vault.list_snapshots("dst-obj").unwrap();
        assert!(dst_list.is_empty());
    }

    // ── Audit log ─────────────────────────────────────────────

    #[test]
    fn test_log_structured_and_list() {
        let (vault, _dir) = setup();
        vault
            .log_structured(
                "delete",
                "profile",
                Some("prof-1"),
                Some("My Profile"),
                "user",
                Some("soft delete"),
            )
            .unwrap();

        let logs = vault.list_audit_log(10).unwrap();
        assert!(!logs.is_empty());
        let entry = &logs[0];
        assert_eq!(entry.action_type, "delete");
        assert_eq!(entry.entity_type, "profile");
        assert_eq!(entry.entity_id, Some("prof-1".to_string()));
        assert_eq!(entry.entity_name, Some("My Profile".to_string()));
        assert_eq!(entry.performed_by, "user");
        assert_eq!(entry.details, Some("soft delete".to_string()));
    }

    // ── Guide embeddings ──────────────────────────────────────

    #[test]
    fn test_guide_embedding_roundtrip() {
        let (vault, _dir) = setup();
        let chunk = crate::GuideEmbeddingChunk {
            id: "chunk-1".to_string(),
            guide_id: "guide-1".to_string(),
            chunk_index: 0,
            chunk_text: "Hello world".to_string(),
            embedding: vec![0.1f32, 0.2, 0.3, 0.4],
            model: "test-model".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        vault.save_guide_embeddings(&[chunk]).unwrap();

        let list = vault.list_guide_embeddings().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].chunk_text, "Hello world");
        assert_eq!(list[0].embedding, vec![0.1f32, 0.2, 0.3, 0.4]);
    }

    #[test]
    fn test_clear_guide_embeddings() {
        let (vault, _dir) = setup();
        let chunk = crate::GuideEmbeddingChunk {
            id: "chunk-x".to_string(),
            guide_id: "guide-x".to_string(),
            chunk_index: 0,
            chunk_text: "x".to_string(),
            embedding: vec![1.0],
            model: "model".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        vault.save_guide_embeddings(&[chunk]).unwrap();
        assert_eq!(vault.count_guide_embeddings().unwrap(), 1);

        vault.clear_guide_embeddings().unwrap();
        assert_eq!(vault.count_guide_embeddings().unwrap(), 0);
        assert!(vault.list_guide_embeddings().unwrap().is_empty());
    }

    #[test]
    fn test_count_guide_embeddings() {
        let (vault, _dir) = setup();
        assert_eq!(vault.count_guide_embeddings().unwrap(), 0);
        for i in 0..5 {
            let chunk = crate::GuideEmbeddingChunk {
                id: format!("chunk-{}", i),
                guide_id: format!("guide-{}", i),
                chunk_index: 0,
                chunk_text: "t".to_string(),
                embedding: vec![1.0],
                model: "m".to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
            };
            vault.save_guide_embeddings(&[chunk]).unwrap();
        }
        assert_eq!(vault.count_guide_embeddings().unwrap(), 5);
    }

    // ── sys_config ────────────────────────────────────────────

    #[test]
    fn test_sys_config_roundtrip() {
        let (vault, _dir) = setup();
        assert!(vault.get_sys_config("my_key").unwrap().is_none());

        vault.set_sys_config("my_key", "my_value").unwrap();
        assert_eq!(
            vault.get_sys_config("my_key").unwrap(),
            Some("my_value".to_string())
        );

        vault.set_sys_config("my_key", "updated_value").unwrap();
        assert_eq!(
            vault.get_sys_config("my_key").unwrap(),
            Some("updated_value".to_string())
        );
    }

    // ── Private metadata helpers ──────────────────────────────

    #[test]
    fn test_metadata_read_write_delete() {
        let (vault, _dir) = setup();
        assert!(vault.read_metadata("k1", "pfx").unwrap().is_none());

        vault.write_metadata("k1", "pfx", b"hello bytes").unwrap();
        let loaded = vault.read_metadata("k1", "pfx").unwrap().unwrap();
        assert_eq!(loaded, b"hello bytes");
    }

    #[test]
    fn test_metadata_overwrite() {
        let (vault, _dir) = setup();
        vault.write_metadata("k", "pfx", b"first").unwrap();
        vault.write_metadata("k", "pfx", b"second").unwrap();
        let loaded = vault.read_metadata("k", "pfx").unwrap().unwrap();
        assert_eq!(loaded, b"second");
    }

    // ── Additional stats / state tests ────────────────────────

    #[test]
    fn test_stats_empty_vault() {
        let (vault, _dir) = setup();
        let stats = vault.stats().unwrap();
        assert_eq!(stats.profile_count, 0);
        assert_eq!(stats.total_size_bytes, 0);
        assert!(stats.last_modified.is_none());
    }

    #[test]
    fn test_stats_with_objects_and_trash() {
        let (vault, _dir) = setup();
        let profile = Profile::new("test", vec![1, 2, 3]);
        vault.save_profile(&profile).unwrap();

        let obj = ObjectRecord {
            contract_type_id: None,
            id: "obj-stats".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Stats Object".to_string(),
            icon_name: "doc".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({"content": "some data"}),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            template_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            ..Default::default()
        };
        vault.save_object(&obj).unwrap();

        let item = TrashItem {
            id: "trash-stats".to_string(),
            item_type: "object".to_string(),
            original_id: "orig-stats".to_string(),
            original_parent_id: None,
            original_section_type: None,
            original_sort_order: None,
            data: vec![1, 2, 3],
            deleted_at: chrono::Utc::now().timestamp(),
            expires_at: None,
            deleted_by: "user".to_string(),
            name_snapshot: "Trashed".to_string(),
            icon_snapshot: None,
        };
        vault.save_trash_item(&item).unwrap();

        let stats = vault.stats().unwrap();
        assert_eq!(stats.profile_count, 1);
        assert!(stats.profiles_size > 0);
        assert!(stats.objects_size > 0);
        assert!(stats.trash_size > 0);
        assert!(stats.total_size_bytes > 0);
    }

    // ── User template tests (§29 P1) ──────────────────────────

    fn make_test_template(account_id: &str, name: &str) -> crate::UserTemplate {
        crate::UserTemplate {
            contract_type_id: None,
            id: format!("utpl_{}", uuid::Uuid::new_v4().simple()),
            account_id: account_id.to_string(),
            name: name.to_string(),
            icon_id: Some("document".to_string()),
            properties: vec![
                crate::TemplateProperty {
                    contract_field: None,
                    contract_bindings: None,
                    id: "full_name".to_string(),
                    name: "姓名".to_string(),
                    prop_type: crate::PropertyType::Text,
                    sensitivity_level: None,
                    sensitive: Some(false),
                    options: None,
                    deprecated_at: None,
                    allowed_types: None,
                    max_items: None,
                },
                crate::TemplateProperty {
                    contract_field: None,
                    contract_bindings: None,
                    id: "passport_number".to_string(),
                    name: "护照号码".to_string(),
                    prop_type: crate::PropertyType::Text,
                    sensitivity_level: None,
                    sensitive: Some(true),
                    options: None,
                    deprecated_at: None,
                    allowed_types: None,
                    max_items: None,
                },
                crate::TemplateProperty {
                    contract_field: None,
                    contract_bindings: None,
                    id: "expiry_date".to_string(),
                    name: "过期日期".to_string(),
                    prop_type: crate::PropertyType::Date,
                    sensitivity_level: None,
                    sensitive: Some(false),
                    options: None,
                    deprecated_at: None,
                    allowed_types: None,
                    max_items: None,
                },
            ],
            category: Some("identity".to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: None,
        }
    }

    #[test]
    fn test_user_template_save_and_load() {
        let (vault, _dir) = setup();
        let tpl = make_test_template("acc-1", "护照模板");
        vault.save_user_template(&tpl).unwrap();

        let loaded = vault.load_user_template(&tpl.id).unwrap().unwrap();
        assert_eq!(loaded.name, "护照模板");
        assert_eq!(loaded.properties.len(), 3);
        assert_eq!(loaded.properties[2].prop_type, crate::PropertyType::Date);
    }

    #[test]
    fn test_user_template_list_and_count() {
        let (vault, _dir) = setup();
        let a1 = make_test_template("acc-1", "模板A");
        let a2 = make_test_template("acc-1", "模板B");
        let b1 = make_test_template("acc-2", "模板C");

        vault.save_user_template(&a1).unwrap();
        vault.save_user_template(&a2).unwrap();
        vault.save_user_template(&b1).unwrap();

        assert_eq!(vault.count_user_templates("acc-1").unwrap(), 2);
        assert_eq!(vault.count_user_templates("acc-2").unwrap(), 1);

        let list = vault.list_user_templates("acc-1").unwrap();
        assert_eq!(list.len(), 2);
        // ASC order: a1 should be first (created earlier)
        assert_eq!(list[0].name, "模板A");
    }

    #[test]
    fn test_user_template_update() {
        let (vault, _dir) = setup();
        let mut tpl = make_test_template("acc-1", "旧名称");
        vault.save_user_template(&tpl).unwrap();

        tpl.name = "新名称".to_string();
        tpl.icon_id = Some("passport".to_string());
        tpl.properties.push(crate::TemplateProperty {
            contract_field: None,
            contract_bindings: None,
            id: "new_field".to_string(),
            name: "新字段".to_string(),
            prop_type: crate::PropertyType::Boolean,
            sensitivity_level: None,
            sensitive: Some(false),
            options: None,
            deprecated_at: None,
            allowed_types: None,
            max_items: None,
        });
        tpl.updated_at = Some(chrono::Utc::now().to_rfc3339());
        vault.save_user_template(&tpl).unwrap();

        let loaded = vault.load_user_template(&tpl.id).unwrap().unwrap();
        assert_eq!(loaded.name, "新名称");
        assert_eq!(loaded.icon_id, Some("passport".to_string()));
        assert_eq!(loaded.properties.len(), 4);
        assert!(loaded.updated_at.is_some());
    }

    #[test]
    fn test_user_template_delete() {
        let (vault, _dir) = setup();
        let tpl = make_test_template("acc-1", "待删除");
        vault.save_user_template(&tpl).unwrap();
        assert!(vault.load_user_template(&tpl.id).unwrap().is_some());

        vault.delete_user_template(&tpl.id).unwrap();
        assert!(vault.load_user_template(&tpl.id).unwrap().is_none());
        assert_eq!(vault.count_user_templates("acc-1").unwrap(), 0);
    }

    #[test]
    fn test_user_template_load_not_found() {
        let (vault, _dir) = setup();
        assert!(vault.load_user_template("nonexistent").unwrap().is_none());
    }

    #[test]
    fn test_property_type_infer_from_value() {
        use crate::PropertyType;

        assert_eq!(
            PropertyType::infer_from_value(&serde_json::json!(true), "any"),
            PropertyType::Boolean
        );
        assert_eq!(
            PropertyType::infer_from_value(&serde_json::json!(42), "any"),
            PropertyType::Number
        );
        assert_eq!(
            PropertyType::infer_from_value(&serde_json::json!(std::f64::consts::PI), "any"),
            PropertyType::Number
        );
        assert_eq!(
            PropertyType::infer_from_value(&serde_json::json!("hello"), "any"),
            PropertyType::Text
        );
        assert_eq!(
            PropertyType::infer_from_value(&serde_json::json!("hello"), "expiry_date"),
            PropertyType::Text
        );
        assert_eq!(
            PropertyType::infer_from_value(&serde_json::json!("user@example.com"), "email_addr"),
            PropertyType::Email
        );
        assert_eq!(
            PropertyType::infer_from_value(&serde_json::json!("+86-138-0000-0000"), "phone_number"),
            PropertyType::Phone
        );
        assert_eq!(
            PropertyType::infer_from_value(
                &serde_json::json!("https://example.com"),
                "website_url"
            ),
            PropertyType::Url
        );
        assert_eq!(
            PropertyType::infer_from_value(&serde_json::json!(["a", "b"]), "any"),
            PropertyType::MultiSelect
        );
        assert_eq!(
            PropertyType::infer_from_value(&serde_json::json!("2024-01-15"), "issue_date"),
            PropertyType::Date
        );
    }

    #[test]
    fn test_template_soft_delete_appears_in_trash() {
        let (vault, _dir) = setup();

        // 1. Create a user template
        let template = crate::UserTemplate {
            contract_type_id: None,
            id: "tpl_test_001".to_string(),
            account_id: "test_account".to_string(),
            name: "Test Template".to_string(),
            icon_id: Some("document".to_string()),
            properties: vec![crate::TemplateProperty {
                contract_field: None,
                contract_bindings: None,
                id: "field1".to_string(),
                name: "field1".to_string(),
                prop_type: crate::PropertyType::Text,
                sensitivity_level: None,
                sensitive: None,
                options: None,
                deprecated_at: None,
                allowed_types: None,
                max_items: None,
            }],
            category: Some("identity".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: None,
        };
        vault.save_user_template(&template).unwrap();

        // 2. Simulate template_delete: build TrashItem and save
        let template_data = serde_json::to_vec(&template).unwrap();
        let trash = TrashItem {
            id: "trash_tpl_001".to_string(),
            item_type: "template".to_string(),
            original_id: template.id.clone(),
            original_parent_id: None,
            original_section_type: template.category.clone(),
            original_sort_order: None,
            data: template_data,
            deleted_at: 1704067200000i64,
            expires_at: Some(1706659200000i64),
            deleted_by: "user".to_string(),
            name_snapshot: template.name.clone(),
            icon_snapshot: template.icon_id.clone(),
        };
        vault.save_trash_item(&trash).unwrap();

        // 3. Delete the template from user_templates table
        vault.delete_user_template(&template.id).unwrap();

        // 4. List trash items and verify template appears
        let items = vault.list_trash_items(None, None).unwrap();
        assert_eq!(items.len(), 1);
        let item = &items[0];
        assert_eq!(item.id, "trash_tpl_001");
        assert_eq!(item.item_type, "template");
        assert_eq!(item.name, "Test Template");
        assert_eq!(item.icon_id, Some("document".to_string()));
        assert_eq!(item.deleted_at, 1704067200000i64);
        assert_eq!(item.expires_at, Some(1706659200000i64));
        assert_eq!(item.original_section_type, Some("identity".to_string()));

        // 5. Verify filtering by item_type works
        let template_items = vault.list_trash_items(Some("template"), None).unwrap();
        assert_eq!(template_items.len(), 1);
        assert_eq!(template_items[0].name, "Test Template");
    }

    // ── Encryption-specific tests ─────────────────────────────

    #[test]
    fn test_profile_encryption_roundtrip() {
        let (vault, _dir) = setup();
        let data = serde_json::to_vec(&serde_json::json!({
            "identity": {"fullName": "Alice"},
            "financial": {"cards": [{"cardNumber": "1234"}]},
        }))
        .unwrap();
        let profile = Profile::new_with_id("enc", "Encrypted", data.clone());
        vault.save_profile(&profile).unwrap();

        // Verify raw database bytes are encrypted (SOLO magic).
        let raw: Vec<u8> = {
            let guard = vault.conn.lock().unwrap();
            let conn = guard.as_ref().unwrap();
            conn.query_row(
                "SELECT data FROM profiles WHERE id = ?1",
                params!["enc"],
                |r| r.get(0),
            )
            .unwrap()
        };
        assert!(crate::encryption::is_encrypted_blob(&raw));
        assert_ne!(raw, data);

        let loaded = vault.load_profile("enc").unwrap().unwrap();
        assert_eq!(loaded.data, data);
    }

    #[test]
    fn test_object_properties_encryption_roundtrip() {
        let (vault, _dir) = setup();
        let obj = ObjectRecord {
            contract_type_id: None,
            id: "obj-enc".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Encrypted Object".to_string(),
            icon_name: "doc".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({"secret": "value"}),
            property_labels: Some(serde_json::json!({"secret": "Secret"})),
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            template_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            ..Default::default()
        };
        vault.save_object(&obj).unwrap();

        let raw_props: String = {
            let guard = vault.conn.lock().unwrap();
            let conn = guard.as_ref().unwrap();
            conn.query_row(
                "SELECT properties FROM objects WHERE id = ?1",
                params!["obj-enc"],
                |r| r.get(0),
            )
            .unwrap()
        };
        assert!(raw_props.starts_with(crate::encryption::ENCRYPTED_TEXT_PREFIX));

        let loaded = vault.load_object("obj-enc").unwrap().unwrap();
        assert_eq!(loaded.properties, serde_json::json!({"secret": "value"}));
        assert_eq!(
            loaded.property_labels,
            Some(serde_json::json!({"secret": "Secret"}))
        );
    }

    #[test]
    fn test_trash_and_snapshot_encryption_roundtrip() {
        let (vault, _dir) = setup();
        let item = TrashItem {
            id: "trash-enc".to_string(),
            item_type: "object".to_string(),
            original_id: "orig-enc".to_string(),
            original_parent_id: None,
            original_section_type: None,
            original_sort_order: None,
            data: vec![1, 2, 3, 4, 5],
            deleted_at: chrono::Utc::now().timestamp(),
            expires_at: None,
            deleted_by: "user".to_string(),
            name_snapshot: "Enc".to_string(),
            icon_snapshot: None,
        };
        vault.save_trash_item(&item).unwrap();

        let raw_data: Vec<u8> = {
            let guard = vault.conn.lock().unwrap();
            let conn = guard.as_ref().unwrap();
            conn.query_row(
                "SELECT data FROM trash_items WHERE id = ?1",
                params!["trash-enc"],
                |r| r.get(0),
            )
            .unwrap()
        };
        assert!(crate::encryption::is_encrypted_blob(&raw_data));

        let loaded = vault.get_trash_item("trash-enc").unwrap().unwrap();
        assert_eq!(loaded.data, vec![1, 2, 3, 4, 5]);

        vault
            .save_snapshot("obj-enc", "user_edit", b"snapshot", "sum")
            .unwrap();
        let raw_snap: Vec<u8> = {
            let guard = vault.conn.lock().unwrap();
            let conn = guard.as_ref().unwrap();
            conn.query_row("SELECT data FROM object_snapshots LIMIT 1", [], |r| {
                r.get(0)
            })
            .unwrap()
        };
        assert!(crate::encryption::is_encrypted_blob(&raw_snap));

        let snapshots = vault.list_snapshots("obj-enc").unwrap();
        let snap_id = snapshots[0]["id"].as_str().unwrap();
        assert_eq!(vault.get_snapshot(snap_id).unwrap().unwrap(), b"snapshot");
    }

    #[test]
    fn test_migration_from_plaintext() {
        let dir = TempDir::new().unwrap();
        let key = test_key();
        let db_path = dir.path().join("vault.db");

        // Seed a fresh database with plaintext sensitive data (simulating pre-encryption vault).
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS profiles (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    data BLOB NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    version INTEGER DEFAULT 1
                );
                CREATE TABLE IF NOT EXISTS objects (
                    id TEXT PRIMARY KEY,
                    account_id TEXT NOT NULL,
                    type_id TEXT NOT NULL DEFAULT 'note',
                    section_type TEXT NOT NULL DEFAULT 'identity',
                    name TEXT NOT NULL,
                    icon_name TEXT NOT NULL DEFAULT 'document',
                    parent_id TEXT,
                    children_ids TEXT NOT NULL DEFAULT '[]',
                    properties TEXT NOT NULL DEFAULT '{}',
                    property_labels TEXT DEFAULT '{}',
                    sensitivity_level TEXT NOT NULL DEFAULT 'internal',
                    is_deleted INTEGER NOT NULL DEFAULT 0,
                    deleted_at TEXT,
                    tags_json TEXT NOT NULL DEFAULT '[]',
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    version INTEGER DEFAULT 1
                );
                CREATE TABLE IF NOT EXISTS sys_config (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );",
            )
            .unwrap();
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO profiles (id, name, data, created_at, updated_at, version)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params!["plain-profile", "Plain", b"plain data", &now, &now, 1],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO objects (id, account_id, type_id, section_type, name, icon_name,
                 children_ids, properties, property_labels, sensitivity_level, is_deleted,
                 tags_json, created_at, updated_at, version)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    "plain-obj",
                    "acc",
                    "note",
                    "identity",
                    "Plain Object",
                    "document",
                    "[]",
                    r#"{"key":"value"}"#,
                    "{}",
                    "internal",
                    0,
                    "[]",
                    &now,
                    &now,
                    1
                ],
            )
            .unwrap();
        }

        // Re-open with encryption key: migration should encrypt legacy data.
        {
            let config = VaultConfig::new("acc", dir.path().to_path_buf()).with_data_key(key);
            let vault = VaultStore::open(config).unwrap();

            let profile = vault.load_profile("plain-profile").unwrap().unwrap();
            assert_eq!(profile.data, b"plain data");

            let obj = vault.load_object("plain-obj").unwrap().unwrap();
            assert_eq!(obj.properties, serde_json::json!({"key": "value"}));

            let version = vault.get_sys_config("encryption_version").unwrap();
            assert_eq!(version, Some("1".to_string()));
        }
    }

    #[test]
    fn test_reencrypt_all_roundtrip() {
        let (vault, _dir) = setup();
        let profile = Profile::new_with_id("reenc", "ReEnc", b"data".to_vec());
        vault.save_profile(&profile).unwrap();
        let obj = ObjectRecord {
            contract_type_id: None,
            id: "obj-reenc".to_string(),
            account_id: "acc".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "ReEnc".to_string(),
            icon_name: "doc".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({"k": "v"}),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            template_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            ..Default::default()
        };
        vault.save_object(&obj).unwrap();

        let old_key = DataEncryptionKey::new(test_key());
        let new_key = DataEncryptionKey::new([0x99u8; 32]);
        vault.reencrypt_all(&old_key, &new_key).unwrap();

        // After re-opening with new key, data should still decrypt.
        // (Manually swap the internal key to simulate reopening.)
        {
            let mut guard = vault.data_key.lock().unwrap();
            *guard = Some(new_key);
        }

        let loaded_profile = vault.load_profile("reenc").unwrap().unwrap();
        assert_eq!(loaded_profile.data, b"data");

        let loaded_obj = vault.load_object("obj-reenc").unwrap().unwrap();
        assert_eq!(loaded_obj.properties, serde_json::json!({"k": "v"}));
    }

    // ── N-2: reencrypt_all 失败必须整体回滚 ──────────────────────────────
    //
    // 历史 bug：闭包内任一行解密失败返回 Err 时，函数仍无条件 tx.commit()，导致
    // 失败前已处理的行用新钥落库、失败行仍为旧钥的混态——改密/KDF 升级后账户部分
    // 数据永久不可解密。本测试破坏一个对象的密文（GCM 认证失败）后调用
    // reencrypt_all，断言返回 Err 且全部数据仍以旧密钥可解密、内容不变。
    #[test]
    fn test_reencrypt_all_failure_rolls_back() {
        let (vault, _dir) = setup();
        let profile = Profile::new_with_id("reenc-roll", "Roll", b"data".to_vec());
        vault.save_profile(&profile).unwrap();
        vault
            .save_object(&ObjectRecord {
                id: "obj-roll".to_string(),
                account_id: "acc".to_string(),
                type_id: "note".to_string(),
                section_type: "identity".to_string(),
                name: "Roll".to_string(),
                icon_name: "doc".to_string(),
                properties: serde_json::json!({ "k": "v" }),
                sensitivity_level: "internal".to_string(),
                ..Default::default()
            })
            .unwrap();

        let old_key = DataEncryptionKey::new(test_key());
        let new_key = DataEncryptionKey::new([0x99u8; 32]);

        // 破坏 obj-roll 的 properties 密文（翻转 solo: 前缀之后的字节 → GCM 认证失败）
        {
            let mut guard = vault.conn.lock().unwrap();
            let conn = guard.as_mut().unwrap();
            let raw: String = conn
                .query_row(
                    "SELECT properties FROM objects WHERE id = ?1",
                    ["obj-roll"],
                    |r| r.get(0),
                )
                .unwrap();
            let mut bytes = raw.into_bytes();
            let mid = bytes.len() / 2;
            bytes[mid] ^= 0x01;
            conn.execute(
                "UPDATE objects SET properties = ?1 WHERE id = ?2",
                params![String::from_utf8_lossy(&bytes), "obj-roll"],
            )
            .unwrap();
        }

        // 记录损坏后的原始密文（验证失败事务不得部分写入任何行）
        let raw_before: String = {
            let mut guard = vault.conn.lock().unwrap();
            let conn = guard.as_mut().unwrap();
            conn.query_row(
                "SELECT properties FROM objects WHERE id = ?1",
                ["obj-roll"],
                |r| r.get(0),
            )
            .unwrap()
        };

        // 损坏行必须令 reencrypt_all 失败
        assert!(
            vault.reencrypt_all(&old_key, &new_key).is_err(),
            "损坏行应导致 reencrypt_all 失败"
        );

        // 事务必须整体回滚：profile（在损坏对象之前已重加密）仍以旧密钥可解密、
        // 内容不变——若存在“失败仍无条件 commit”的混态，profile 已被换为新钥。
        let loaded_profile = vault.load_profile("reenc-roll").unwrap().unwrap();
        assert_eq!(
            loaded_profile.data, b"data",
            "失败后 profile 仍应以旧钥解密"
        );
        // 损坏行的原始密文字节未被写入（整个事务回滚）
        let raw_after: String = {
            let mut guard = vault.conn.lock().unwrap();
            let conn = guard.as_mut().unwrap();
            conn.query_row(
                "SELECT properties FROM objects WHERE id = ?1",
                ["obj-roll"],
                |r| r.get(0),
            )
            .unwrap()
        };
        assert_eq!(raw_after, raw_before, "失败事务不得部分写入任何行");
    }

    // ── Sync helpers ──────────────────────────────────────────

    #[test]
    fn test_sync_peer_state_roundtrip() {
        let (vault, _dir) = setup();
        let peer = crate::PeerSyncState {
            peer_node_id: "node_abc".to_string(),
            peer_name: Some("Living Room".to_string()),
            trusted: false,
            public_key_fingerprint: Some("deadbeef".to_string()),
            last_seen: Some(1234567890),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        vault.save_peer_state(&peer).unwrap();

        let loaded = vault.load_peer_state("node_abc").unwrap().unwrap();
        assert_eq!(loaded.peer_node_id, "node_abc");
        assert!(!loaded.trusted);

        vault.set_peer_trusted("node_abc", true).unwrap();
        let loaded = vault.load_peer_state("node_abc").unwrap().unwrap();
        assert!(loaded.trusted);

        vault.delete_peer("node_abc").unwrap();
        assert!(vault.load_peer_state("node_abc").unwrap().is_none());
    }

    #[test]
    fn test_apply_sync_record_profile() {
        let (vault, _dir) = setup();
        let data_b64 =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, b"secret data");
        let record = crate::VaultSyncRecord {
            id: "p1".to_string(),
            table: "profiles".to_string(),
            data: serde_json::json!({
                "id": "p1",
                "name": "Test",
                "data": data_b64,
                "createdAt": chrono::Utc::now().to_rfc3339(),
                "updatedAt": chrono::Utc::now().to_rfc3339(),
                "version": 1,
            }),
            hlc: crate::RecordHlc {
                wall_time_ms: 1000,
                counter: 1,
                node_id: "node_a".to_string(),
            },
            deleted: false,
        };
        assert!(vault.apply_sync_record(&record, "node_b").unwrap());
        let loaded = vault.load_profile("p1").unwrap().unwrap();
        assert_eq!(loaded.name, "Test");
        assert_eq!(loaded.data, b"secret data");
    }

    #[test]
    fn test_apply_sync_record_skips_older_hlc() {
        let (vault, _dir) = setup();
        let hlc_newer = crate::RecordHlc {
            wall_time_ms: 2000,
            counter: 0,
            node_id: "node_a".to_string(),
        };
        let hlc_older = crate::RecordHlc {
            wall_time_ms: 1000,
            counter: 0,
            node_id: "node_a".to_string(),
        };
        let make_record = |hlc: crate::RecordHlc, name: &str| crate::VaultSyncRecord {
            id: "p1".to_string(),
            table: "profiles".to_string(),
            data: serde_json::json!({
                "id": "p1",
                "name": name,
                "data": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, b"x"),
                "createdAt": chrono::Utc::now().to_rfc3339(),
                "updatedAt": chrono::Utc::now().to_rfc3339(),
                "version": 1,
            }),
            hlc,
            deleted: false,
        };
        assert!(vault
            .apply_sync_record(&make_record(hlc_newer, "Newer"), "node_b")
            .unwrap());
        assert!(!vault
            .apply_sync_record(&make_record(hlc_older, "Older"), "node_b")
            .unwrap());
        let loaded = vault.load_profile("p1").unwrap().unwrap();
        assert_eq!(loaded.name, "Newer");
    }

    /// P115 回归：`apply_sync_records_batch` 单事务批量应用语义与逐条 `apply_sync_record` 等价。
    /// 覆盖：整批多条成功、HLC 较旧跳过、单条记录失败不中断整批（错误入 outcome）、
    /// 写前本地 HLC 供冲突报告复用。
    #[test]
    fn test_apply_sync_records_batch_matches_single_semantics() {
        let (vault, _dir) = setup();
        let mk_hlc = |wall: u64, counter: u32| crate::RecordHlc {
            wall_time_ms: wall,
            counter,
            node_id: "node_a".to_string(),
        };
        let mk_record = |id: &str, hlc: crate::RecordHlc, name: &str| crate::VaultSyncRecord {
            id: id.to_string(),
            table: "profiles".to_string(),
            data: serde_json::json!({
                "id": id,
                "name": name,
                "data": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, b"x"),
                "createdAt": chrono::Utc::now().to_rfc3339(),
                "updatedAt": chrono::Utc::now().to_rfc3339(),
                "version": 1,
            }),
            hlc,
            deleted: false,
        };
        // 构造借用视图（零克隆入口）
        let records = vec![
            mk_record("p1", mk_hlc(1000, 0), "First"),
            mk_record("p2", mk_hlc(1000, 0), "Second"),
        ];
        let hlcs: Vec<crate::RecordHlc> = records.iter().map(|r| r.hlc.clone()).collect();
        let borrowed: Vec<crate::BorrowedSyncRecord> = records
            .iter()
            .zip(hlcs.iter())
            .map(|(r, hlc)| crate::BorrowedSyncRecord {
                id: &r.id,
                table: &r.table,
                data: &r.data,
                hlc,
                deleted: r.deleted,
            })
            .collect();

        let outcomes = vault.apply_sync_records_batch(&borrowed, "node_b").unwrap();
        assert_eq!(outcomes.len(), 2);
        assert!(outcomes[0].applied);
        assert!(outcomes[1].applied);
        // 写前本地 HLC 应为 None（新记录）
        assert!(outcomes[0].local_hlc.is_none());
        assert!(outcomes[0].error.is_none());

        // 再批：同 HLC 应全部跳过（false），并携带写前本地 HLC 供冲突报告
        let outcomes2 = vault.apply_sync_records_batch(&borrowed, "node_b").unwrap();
        assert!(!outcomes2[0].applied);
        assert_eq!(outcomes2[0].local_hlc, Some(mk_hlc(1000, 0)));

        // 单条记录解码失败不中断整批：p1 旧 HLC 跳过、p2 数据非法（更新 HLC 通过检查后解码失败）→ p2 错误入 outcome
        let mut bad = records.clone();
        bad[1].data = serde_json::json!({ "no": "data" });
        // 关键：p2 必须携带比本地更新的 HLC，才能越过 HLC 跳过分支进入解码路径；
        // 若与本地 HLC 相等（1000）则被跳过、不产生错误。
        bad[1].hlc = mk_hlc(2000, 0);
        let hlcs_bad: Vec<crate::RecordHlc> = bad.iter().map(|r| r.hlc.clone()).collect();
        let borrowed_bad: Vec<crate::BorrowedSyncRecord> = bad
            .iter()
            .zip(hlcs_bad.iter())
            .map(|(r, hlc)| crate::BorrowedSyncRecord {
                id: &r.id,
                table: &r.table,
                data: &r.data,
                hlc,
                deleted: r.deleted,
            })
            .collect();
        // p1 已是旧 HLC（跳过），p2 解码失败
        let outcomes3 = vault
            .apply_sync_records_batch(&borrowed_bad, "node_b")
            .unwrap();
        assert!(!outcomes3[0].applied);
        assert!(!outcomes3[1].applied);
        assert!(outcomes3[1].error.is_some());
        // 整批未因单条失败而中断（返回 Ok 且无 panic）
        assert_eq!(outcomes3.len(), 2);

        let loaded1 = vault.load_profile("p1").unwrap().unwrap();
        assert_eq!(loaded1.name, "First");
        let loaded2 = vault.load_profile("p2").unwrap().unwrap();
        assert_eq!(loaded2.name, "Second");
    }

    /// P109 回归：list_object_changes_since 的水印过滤下推到 SQL 后语义不变。
    /// 覆盖三类路径：有 HLC 行（精确三元组比较）、无 HLC 行（updated_at 回退）、
    /// 以及 wall_time 相等时的 counter / node_id 平局裁决。
    #[test]
    fn test_list_object_changes_since_watermark_pushdown() {
        let (vault, _dir) = setup();
        let wm_wall = VaultStore::parse_time_ms("2026-08-01T00:00:00+00:00");
        let watermark = crate::SyncWatermark {
            wall_time_ms: wm_wall,
            counter: 0,
            node_id: "peer_b".to_string(),
        };

        let mk_obj = |id: &str, updated_at: &str| crate::ObjectRecord {
            id: id.to_string(),
            account_id: "test_account".to_string(),
            name: id.to_string(),
            section_type: "identity".to_string(),
            properties: serde_json::json!({ "k": id }),
            sensitivity_level: "internal".to_string(),
            created_at: updated_at.to_string(),
            updated_at: updated_at.to_string(),
            ..Default::default()
        };

        // 无 HLC 行：updated_at 早于水印 → 排除（回退路径粗筛后由 Rust 精确裁决）
        vault
            .save_object(&mk_obj("obj_old_no_hlc", "2026-07-01T00:00:00+00:00"))
            .unwrap();
        // 无 HLC 行：updated_at 晚于水印 → 包含（回退 HLC 由 updated_at 派生）
        vault
            .save_object(&mk_obj("obj_new_no_hlc", "2026-08-02T00:00:00+00:00"))
            .unwrap();

        // 有 HLC 行：精确三元组比较
        let mk_hlc = |wall: u64, counter: u32, node: &str| crate::RecordHlc {
            wall_time_ms: wall,
            counter,
            node_id: node.to_string(),
        };
        for (id, wall, counter, node) in [
            ("obj_hlc_after", wm_wall + 1000, 0, "peer_b"),
            ("obj_hlc_before", wm_wall - 1000, 0, "peer_b"),
            ("obj_hlc_tie_counter", wm_wall, 1, "peer_b"),
            ("obj_hlc_tie_node_gt", wm_wall, 0, "peer_z"),
            ("obj_hlc_tie_node_lt", wm_wall, 0, "peer_a"),
        ] {
            vault
                .save_object(&mk_obj(id, "2026-07-01T00:00:00+00:00"))
                .unwrap();
            vault
                .set_record_hlc("objects", id, &mk_hlc(wall, counter, node))
                .unwrap();
        }

        let records = vault
            .list_sync_changes_since("objects", &watermark, "test_account", "local_node")
            .unwrap();
        let mut ids: Vec<&str> = records.iter().map(|r| r.id.as_str()).collect();
        ids.sort_unstable();

        // 预期包含：无 HLC 回退的新对象 + HLC 晚于水印 + wall 相等时 counter/node 平局胜出
        assert_eq!(
            ids,
            vec![
                "obj_hlc_after",
                "obj_hlc_tie_counter",
                "obj_hlc_tie_node_gt",
                "obj_new_no_hlc",
            ]
        );
        // 返回的 HLC 与落库值一致（批量 JOIN 取回的 HLC 未被回退逻辑污染）
        let after = records
            .iter()
            .find(|r| r.id == "obj_hlc_after")
            .expect("obj_hlc_after must be present");
        assert_eq!(after.hlc, mk_hlc(wm_wall + 1000, 0, "peer_b"));
        let new_no_hlc = records
            .iter()
            .find(|r| r.id == "obj_new_no_hlc")
            .expect("obj_new_no_hlc must be present");
        assert_eq!(
            new_no_hlc.hlc,
            crate::RecordHlc {
                wall_time_ms: VaultStore::parse_time_ms("2026-08-02T00:00:00+00:00"),
                counter: 0,
                node_id: "local_node".to_string(),
            }
        );
    }

    // ── P110: 分页必须按有效 HLC 升序返回 ───────────────────────────────
    //
    // 会话层每页把 peer watermark 推进到“本页最大 HLC”后重查，因此页面必须
    // 恒为“HLC 最小的 limit 条”。若页面乱序（旧实现全量解密后 skip/take，
    // 无排序），落在页边界之间的记录会被永久跳过。本测试验证：
    //   1) 分页结果按 HLC 三元组升序；
    //   2) 逐页拼接 == 非分页全量结果，无缺漏、无重复；
    //   3) SQL 级 LIMIT/OFFSET 与内存 skip/take 语义一致。
    #[test]
    fn test_list_object_changes_paginated_ordering_and_completeness() {
        let (vault, _dir) = setup();
        let wm_wall = VaultStore::parse_time_ms("2026-08-01T00:00:00+00:00");
        let watermark = crate::SyncWatermark {
            wall_time_ms: wm_wall,
            counter: 0,
            node_id: "peer_b".to_string(),
        };
        let mk_obj = |id: &str, updated_at: &str| crate::ObjectRecord {
            id: id.to_string(),
            account_id: "test_account".to_string(),
            name: id.to_string(),
            section_type: "identity".to_string(),
            properties: serde_json::json!({ "k": id }),
            sensitivity_level: "internal".to_string(),
            created_at: updated_at.to_string(),
            updated_at: updated_at.to_string(),
            ..Default::default()
        };

        // 5 个对象，HLC 交错分布（含 counter 平局与 node 平局裁决路径）
        let mk_hlc = |wall: u64, counter: u32, node: &str| crate::RecordHlc {
            wall_time_ms: wall,
            counter,
            node_id: node.to_string(),
        };
        for (id, wall, counter, node) in [
            ("p_a", wm_wall + 3000, 0, "peer_b"),
            ("p_b", wm_wall + 1000, 5, "peer_a"),
            ("p_c", wm_wall + 1000, 5, "peer_z"), // wall=counter 全平局，node 裁决
            ("p_d", wm_wall + 2000, 0, "peer_b"),
            ("p_e", wm_wall + 1000, 3, "peer_b"),
        ] {
            vault
                .save_object(&mk_obj(id, "2026-07-01T00:00:00+00:00"))
                .unwrap();
            vault
                .set_record_hlc("objects", id, &mk_hlc(wall, counter, node))
                .unwrap();
        }

        // 非分页基准（预期 HLC 升序：p_e → p_b → p_c → p_d → p_a）
        let all = vault
            .list_sync_changes_since("objects", &watermark, "test_account", "local_node")
            .unwrap();
        assert_eq!(
            all.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            vec!["p_e", "p_b", "p_c", "p_d", "p_a"],
            "非分页结果必须按有效 HLC 升序"
        );

        // 分页逐页收集（会话层同款 keyset：每页把水印推进到最大 HLC + 游标推进到最后
        // 一条 id），limit=2。拼接 == 非分页全量即隐式保证逐页升序（每页都是全序切片）。
        let paged_ids = collect_paginated_object_ids(
            &vault,
            watermark.clone(),
            "test_account",
            "local_node",
            2,
        );

        // 逐页拼接 == 非分页结果，无缺漏无重复
        assert_eq!(
            paged_ids,
            all.iter().map(|r| r.id.clone()).collect::<Vec<_>>(),
            "分页拼接必须等于非分页全量结果"
        );
        let uniq: std::collections::HashSet<&str> = paged_ids.iter().map(|s| s.as_str()).collect();
        assert_eq!(uniq.len(), all.len(), "分页结果不得有重复");
    }

    /// N-1 回归测试共用：以会话层同款 keyset 迭代（每页把水印推进到本页最大有效 HLC、
    /// 页游标推进到本页最后一条 id）逐页收集 objects 变更 id。
    fn collect_paginated_object_ids(
        vault: &VaultStore,
        mut watermark: crate::SyncWatermark,
        account_id: &str,
        local_node_id: &str,
        limit: usize,
    ) -> Vec<String> {
        let mut out = Vec::new();
        let mut last_row_id: Option<String> = None;
        loop {
            let page = vault
                .list_sync_changes_since_paginated(
                    "objects",
                    &watermark,
                    account_id,
                    local_node_id,
                    limit,
                    last_row_id.as_deref(),
                )
                .unwrap();
            if page.is_empty() {
                break;
            }
            for rec in &page {
                out.push(rec.id.clone());
            }
            // 水印推进到本页最大有效 HLC（与会话层 update_peer_watermark(max) 一致）
            if let Some((w, c, n)) = page
                .iter()
                .map(|r| (r.hlc.wall_time_ms, r.hlc.counter, r.hlc.node_id.clone()))
                .max()
            {
                watermark = crate::SyncWatermark {
                    wall_time_ms: w,
                    counter: c,
                    node_id: n,
                };
            }
            last_row_id = page.last().map(|r| r.id.clone());
        }
        out
    }

    // ── N-1: keyset 分页——同 HLC 回退行跨页边界不得漏发 ─────────────────
    //
    // P110 引入 SQL 级分页后，会话层把 peer watermark 推进到“本页最大 HLC”再严格 >
    // 重查。同 ms 批量写入的无 HLC 回退行（有效 HLC 三元组完全相同，counter=0、
    // node=local）落在页边界时会被下一页永久跳过（严格 > 排除 hlc == watermark 的
    // 等值组尾部）。本测试构造 7 个同 updated_at 的回退行，以 keyset（last_row_id
    // 游标）逐页收集，断言无缺漏、无重复、组内按 id 稳定升序。
    #[test]
    fn test_paginated_keyset_fallback_equal_hlc_completeness() {
        let (vault, _dir) = setup();
        let ts = "2026-08-01T12:00:00.000+00:00";
        let wm_wall = VaultStore::parse_time_ms(ts);
        let watermark = crate::SyncWatermark {
            wall_time_ms: wm_wall - 1000,
            counter: 0,
            node_id: "peer_x".to_string(),
        };
        for i in 1..=7usize {
            vault
                .save_object(&crate::ObjectRecord {
                    id: format!("fallback_{:02}", i),
                    account_id: "test_account".to_string(),
                    name: format!("fallback_{:02}", i),
                    section_type: "identity".to_string(),
                    properties: serde_json::json!({ "k": i }),
                    sensitivity_level: "internal".to_string(),
                    created_at: ts.to_string(),
                    updated_at: ts.to_string(),
                    ..Default::default()
                })
                .unwrap();
        }

        let paged_ids = collect_paginated_object_ids(
            &vault,
            watermark.clone(),
            "test_account",
            "local_node",
            2,
        );

        // 全部 7 条必须无缺漏、无重复
        assert_eq!(paged_ids.len(), 7, "等值 HLC 回退行不得因页边界漏发");
        let uniq: std::collections::HashSet<&str> = paged_ids.iter().map(|s| s.as_str()).collect();
        assert_eq!(uniq.len(), 7, "keyset 分页不得重复投递");
        // 同 HLC 组内按 id 升序（fallback_01..fallback_07 字典序 == 数字序）
        assert_eq!(
            paged_ids,
            vec![
                "fallback_01".to_string(),
                "fallback_02".to_string(),
                "fallback_03".to_string(),
                "fallback_04".to_string(),
                "fallback_05".to_string(),
                "fallback_06".to_string(),
                "fallback_07".to_string(),
            ],
            "等值 HLC 组内必须按 id 升序稳定分页"
        );
    }

    // ── N-1: 回退假阳性不得填满 LIMIT 预算导致死循环 ──────────────────
    //
    // P110 对无 HLC 回退行以“watermark 所在秒整秒下界”粗筛（宁多勿漏），同秒内
    // updated_at <= watermark 的假阳性会进入 SQL LIMIT 预算，被 Rust 精确过滤后形成
    // “空页但 finished=false、max_hlc=None、水印永不推进”的死循环。修复后 SQL 按
    // (有效 HLC, id) 全序精确过滤，假阳性不再占用 LIMIT 预算，同步必然前进并终止。
    #[test]
    fn test_paginated_keyset_fallback_false_positive_isolation() {
        let (vault, _dir) = setup();
        let wm_wall = VaultStore::parse_time_ms("2026-08-01T12:00:00.500+00:00");
        let watermark = crate::SyncWatermark {
            wall_time_ms: wm_wall,
            counter: 5,
            node_id: "peer_x".to_string(),
        };
        let mk = |id: &str, updated_at: &str| crate::ObjectRecord {
            id: id.to_string(),
            account_id: "test_account".to_string(),
            name: id.to_string(),
            section_type: "identity".to_string(),
            properties: serde_json::json!({ "k": id }),
            sensitivity_level: "internal".to_string(),
            created_at: updated_at.to_string(),
            updated_at: updated_at.to_string(),
            ..Default::default()
        };
        // 假阳性：同秒但早于 watermark（旧粗筛会放行，精确过滤后应被排除）
        for (id, ts) in [
            ("fp_1", "2026-08-01T12:00:00.100+00:00"),
            ("fp_2", "2026-08-01T12:00:00.200+00:00"),
            ("fp_3", "2026-08-01T12:00:00.300+00:00"),
        ] {
            vault.save_object(&mk(id, ts)).unwrap();
        }
        // 真阳性：晚于 watermark
        vault
            .save_object(&mk("tp_1", "2026-08-01T12:00:00.600+00:00"))
            .unwrap();
        vault
            .save_object(&mk("tp_2", "2026-08-01T12:00:00.700+00:00"))
            .unwrap();

        let paged_ids = collect_paginated_object_ids(
            &vault,
            watermark.clone(),
            "test_account",
            "local_node",
            2,
        );

        // 假阳性被排除、真阳性全部投递、循环必然终止
        assert_eq!(
            paged_ids,
            vec!["tp_1".to_string(), "tp_2".to_string()],
            "同秒回退假阳性不得进入结果集，真阳性必须全部投递"
        );
    }

    // ── §30 plugin-template Stage 2 — contract_type_id roundtrip ─────────
    //
    // Stage 1 deliberately left SELECT closures reading contract_type_id as `None`.
    // Stage 2 widens the SELECTs / INSERT so a plugin-declared contract_type_id
    // survives a save → load round-trip. This is the acceptance test for that
    // contract — if it ever regresses, plugins will lose their attach point on
    // every read.
    #[test]
    fn test_contract_type_id_roundtrip() {
        let (vault, _dir) = setup();
        let obj = ObjectRecord {
            contract_type_id: Some("com.test.plugin/v1".to_string()),
            id: "obj-contract-rt".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Contract Test".to_string(),
            icon_name: "doc".to_string(),
            parent_id: None,
            children_ids: Vec::new(),
            properties: serde_json::json!({"key": "value"}),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: Vec::new(),
            template_id: None,
            template_type: None,
            template_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            ..Default::default()
        };
        vault.save_object(&obj).unwrap();
        let loaded = vault
            .load_object("obj-contract-rt")
            .unwrap()
            .expect("round-tripped object must exist");
        assert_eq!(
            loaded.contract_type_id,
            Some("com.test.plugin/v1".to_string()),
            "Stage 2 widening must persist contract_type_id across save → load",
        );

        // list_objects surface (ObjectSummary) should also see it.
        let summaries = vault
            .list_objects("acc-1", None, None, None, false, false)
            .unwrap();
        let summary = summaries
            .iter()
            .find(|s| s.id == "obj-contract-rt")
            .expect("list_objects must surface round-tripped object");
        assert_eq!(
            summary.contract_type_id,
            Some("com.test.plugin/v1".to_string()),
            "list_objects SELECT closure must surface contract_type_id",
        );
        assert_eq!(
            summary.icon_name, "doc",
            "icon_name column (index 14 after widening) must round-trip too",
        );
    }

    // ── Test helper (matches inline-struct-fill project style; closure-free) ──
    fn make_ctid_obj(id: &str, contract_type_id: Option<&str>, name: &str) -> ObjectRecord {
        ObjectRecord {
            contract_type_id: contract_type_id.map(str::to_string),
            id: id.to_string(),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: name.to_string(),
            icon_name: "doc".to_string(),
            parent_id: None,
            children_ids: Vec::new(),
            properties: serde_json::json!({}),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: Vec::new(),
            template_id: None,
            template_type: None,
            template_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            ..Default::default()
        }
    }

    /// Boundary: `contract_type_id = None` must round-trip as `None` across
    /// all read paths. Non-contract columns are pinned so the test stays
    /// differential — a regression in the INSERT widening surfaces via
    /// `name` / `account_id` / `icon_name` / `version` mismatches rather than
    /// silently absorbing through None-defaulting.
    #[test]
    fn test_contract_type_id_none_roundtrip() {
        let (vault, _dir) = setup();
        let obj = make_ctid_obj("obj-ct-none", None, "no-contract");
        vault.save_object(&obj).unwrap();

        let loaded = vault
            .load_object("obj-ct-none")
            .unwrap()
            .expect("round-tripped object must exist");
        assert!(
            loaded.contract_type_id.is_none(),
            "None contract_type_id must survive save -> load (Stage 2 widening)",
        );
        // Pin adjacent columns so this test catches column-shift / INSERT
        // regressions that None-defaulting would silently absorb.
        assert_eq!(loaded.name, "no-contract", "name must round-trip");
        assert_eq!(loaded.account_id, "acc-1", "account_id must round-trip");
        assert_eq!(loaded.icon_name, "doc", "icon_name must round-trip");
        assert_eq!(loaded.version, 1, "version (col 20) must round-trip");

        let summaries = vault
            .list_objects("acc-1", None, None, None, false, false)
            .unwrap();
        let summary = summaries
            .iter()
            .find(|s| s.id == "obj-ct-none")
            .expect("list_objects must surface round-tripped object");
        assert!(
            summary.contract_type_id.is_none(),
            "list_objects SELECT closure must preserve None (no literal injection)",
        );
        assert_eq!(
            summary.icon_name, "doc",
            "ObjectSummary.icon_name (index 14 after widening) must round-trip too",
        );
    }

    /// Boundary: UPSERT via `ON CONFLICT(id) DO UPDATE SET` must overwrite
    /// `contract_type_id` on every save, in both `Some(v1) -> Some(v2)` and
    /// `Some(v1) -> None` directions. Non-mutating fields (`created_at`,
    /// primary key, `version`) are pinned so a future column-shift regression
    /// can't silently rewrite them.
    #[test]
    fn test_contract_type_id_upsert_mutation() {
        let (vault, _dir) = setup();

        // v1 first save locks created_at + version to values we pin across UPSERTs.
        vault
            .save_object(&make_ctid_obj(
                "obj-ct-up",
                Some("com.test.plugin/v1"),
                "v1 name",
            ))
            .unwrap();
        let loaded_v1 = vault
            .load_object("obj-ct-up")
            .unwrap()
            .expect("v1 must persist");
        assert_eq!(
            loaded_v1.contract_type_id,
            Some("com.test.plugin/v1".to_string()),
            "initial Some(v1) save should be readable as Some(v1)",
        );
        let pinned_created_at = loaded_v1.created_at;

        // v2 UPSERT -- contract_type_id overwritten via the widening
        // `ON CONFLICT(id) DO UPDATE SET contract_type_id = excluded.contract_type_id`.
        vault
            .save_object(&make_ctid_obj(
                "obj-ct-up",
                Some("com.test.plugin/v2"),
                "v2 name",
            ))
            .unwrap();
        let loaded_v2 = vault
            .load_object("obj-ct-up")
            .unwrap()
            .expect("v2 UPSERT must persist");
        assert_eq!(
            loaded_v2.contract_type_id,
            Some("com.test.plugin/v2".to_string()),
            "UPSERT must overwrite contract_type_id from v1 -> v2",
        );
        assert_eq!(loaded_v2.name, "v2 name", "UPSERT must overwrite name");
        assert_eq!(
            loaded_v2.created_at, pinned_created_at,
            "created_at must NOT mutate across UPSERTs",
        );
        assert_eq!(
            loaded_v2.id, "obj-ct-up",
            "primary key must stay pinned across UPSERTs",
        );
        assert_eq!(
            loaded_v2.version, 1,
            "version (col 19) must NOT mutate across UPSERTs",
        );

        // Some -> None backdown -- UPSERT must accept the literal NULL.
        vault
            .save_object(&make_ctid_obj("obj-ct-up", None, "v3 detached"))
            .unwrap();
        let loaded_v3 = vault
            .load_object("obj-ct-up")
            .unwrap()
            .expect("None UPSERT must persist");
        assert!(
            loaded_v3.contract_type_id.is_none(),
            "UPSERT must allow overwriting Some -> None",
        );
        assert_eq!(
            loaded_v3.created_at, pinned_created_at,
            "created_at must still stay pinned after Some -> None UPSERT",
        );
        assert_eq!(
            loaded_v3.version, 1,
            "version (col 19) must stay pinned after Some -> None UPSERT",
        );

        // list_objects surface reflects the latest state across every read path.
        let summaries = vault
            .list_objects("acc-1", None, None, None, false, false)
            .unwrap();
        let summary = summaries
            .iter()
            .find(|s| s.id == "obj-ct-up")
            .expect("list_objects must surface upserted object");
        assert!(
            summary.contract_type_id.is_none(),
            "list_objects must reflect Some -> None UPSERT on read",
        );
        assert_eq!(
            summary.icon_name, "doc",
            "ObjectSummary.icon_name must stay pinned across UPSERTs",
        );
    }
}
