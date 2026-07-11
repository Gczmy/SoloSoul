//! Vault store - SQLite storage with app-layer AES-256-GCM encryption

use rusqlite::{params, Connection, OptionalExtension};
use std::sync::Mutex;
use zeroize::Zeroize;

use crate::encryption::{
    decrypt_field, decrypt_text_field, encrypt_field, encrypt_text_field, ensure_encrypted_text,
    DataEncryptionKey,
};
use crate::migration::run_migrations;
use crate::{
    ObjectRecord, ObjectSummary, Profile, ProfileSummary, TrashItem, TrashItemSummary, VaultConfig,
    VaultState, VaultStats,
};

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
                params![chrono::Utc::now().to_rfc3339(), now.clone()],
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

        tx.commit().map_err(|e| e.to_string())?;
        result
    }

    pub fn save_profile(&self, profile: &Profile) -> Result<(), String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let now = chrono::Utc::now().to_rfc3339();
        let encrypted_data = encrypt_field(&key, &profile.data)?;
        conn.execute(
            "INSERT INTO profiles (id, name, data, created_at, updated_at, version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name, data = excluded.data,
                updated_at = excluded.updated_at, version = excluded.version",
            params![
                profile.id,
                profile.name,
                encrypted_data,
                profile.created_at.to_rfc3339(),
                now,
                profile.version
            ],
        )
        .map_err(|e| format!("Failed to save profile: {}", e))?;
        Ok(())
    }

    pub fn load_profile(&self, id: &str) -> Result<Option<Profile>, String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn.prepare(
            "SELECT id, name, data, created_at, updated_at, version FROM profiles WHERE id = ?1"
        ).map_err(|e| format!("Failed to prepare: {}", e))?;
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
            .ok();
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

    // ── Sync state helpers ──────────────────────────────────

    fn now_rfc3339() -> String {
        chrono::Utc::now().to_rfc3339()
    }

    fn parse_time_ms(s: &str) -> u64 {
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
        let result = conn
            .query_row(
                "SELECT wall_time_ms, counter, node_id FROM sync_hlc WHERE table_name = ?1 AND record_id = ?2",
                params![table, record_id],
                |row| {
                    Ok(crate::RecordHlc {
                        wall_time_ms: row.get(0)?,
                        counter: row.get::<_, i32>(1)? as u32,
                        node_id: row.get(2)?,
                    })
                },
            )
            .optional()
            .map_err(|e| format!("get_record_hlc: {}", e))?;
        Ok(result)
    }

    fn set_record_hlc(
        &self,
        table: &str,
        record_id: &str,
        hlc: &crate::RecordHlc,
    ) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        conn.execute(
            "INSERT INTO sync_hlc (table_name, record_id, wall_time_ms, counter, node_id, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(table_name, record_id) DO UPDATE SET
                wall_time_ms = excluded.wall_time_ms,
                counter = excluded.counter,
                node_id = excluded.node_id,
                updated_at = excluded.updated_at",
            params![
                table,
                record_id,
                hlc.wall_time_ms,
                hlc.counter as i32,
                &hlc.node_id,
                Self::now_rfc3339(),
            ],
        )
        .map_err(|e| format!("set_record_hlc: {}", e))?;
        Ok(())
    }

    fn record_hlc_or_fallback(
        &self,
        table: &str,
        record_id: &str,
        updated_at: &str,
        local_node_id: &str,
    ) -> Result<crate::RecordHlc, String> {
        if let Some(hlc) = self.get_record_hlc(table, record_id)? {
            return Ok(hlc);
        }
        Ok(crate::RecordHlc {
            wall_time_ms: Self::parse_time_ms(updated_at),
            counter: 0,
            node_id: local_node_id.to_string(),
        })
    }

    pub fn save_peer_state(&self, peer: &crate::PeerSyncState) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        conn.execute(
            "INSERT INTO sync_peers (peer_node_id, peer_name, trusted, public_key_fingerprint, last_seen, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(peer_node_id) DO UPDATE SET
                peer_name = excluded.peer_name,
                trusted = excluded.trusted,
                public_key_fingerprint = excluded.public_key_fingerprint,
                last_seen = excluded.last_seen,
                updated_at = excluded.updated_at",
            params![
                &peer.peer_node_id,
                &peer.peer_name,
                peer.trusted as i32,
                &peer.public_key_fingerprint,
                peer.last_seen,
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
                "SELECT peer_node_id, peer_name, trusted, public_key_fingerprint, last_seen, created_at, updated_at
                 FROM sync_peers WHERE peer_node_id = ?1",
                params![peer_node_id],
                |row| {
                    Ok(crate::PeerSyncState {
                        peer_node_id: row.get(0)?,
                        peer_name: row.get(1)?,
                        trusted: row.get::<_, i32>(2)? != 0,
                        public_key_fingerprint: row.get(3)?,
                        last_seen: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
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
                "SELECT peer_node_id, peer_name, trusted, public_key_fingerprint, last_seen, created_at, updated_at
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
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|e| format!("list_peers query: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("list_peers collect: {}", e))?;
        Ok(peers)
    }

    pub fn set_peer_trusted(&self, peer_node_id: &str, trusted: bool) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        conn.execute(
            "UPDATE sync_peers SET trusted = ?1, updated_at = ?2 WHERE peer_node_id = ?3",
            params![trusted as i32, Self::now_rfc3339(), peer_node_id],
        )
        .map_err(|e| format!("set_peer_trusted: {}", e))?;
        Ok(())
    }

    pub fn delete_peer(&self, peer_node_id: &str) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        conn.execute(
            "DELETE FROM sync_peers WHERE peer_node_id = ?1",
            params![peer_node_id],
        )
        .map_err(|e| format!("delete_peer: {}", e))?;
        Ok(())
    }

    pub fn update_peer_watermark(
        &self,
        peer_node_id: &str,
        table: &str,
        watermark: &crate::SyncWatermark,
    ) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        conn.execute(
            "INSERT INTO sync_watermarks (peer_node_id, table_name, wall_time_ms, counter, node_id, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(peer_node_id, table_name) DO UPDATE SET
                wall_time_ms = excluded.wall_time_ms,
                counter = excluded.counter,
                node_id = excluded.node_id,
                updated_at = excluded.updated_at",
            params![
                peer_node_id,
                table,
                watermark.wall_time_ms,
                watermark.counter as i32,
                &watermark.node_id,
                Self::now_rfc3339(),
            ],
        )
        .map_err(|e| format!("update_peer_watermark: {}", e))?;
        Ok(())
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

    fn local_node_id(&self) -> String {
        self.get_sync_node_id()
            .ok()
            .flatten()
            .unwrap_or_else(|| "unknown".to_string())
    }

    fn new_tombstone_hlc(&self) -> Result<crate::RecordHlc, String> {
        let node_id = self.local_node_id();
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
    fn record_tombstone(&self, table: &str, record_id: &str) -> Result<(), String> {
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

    fn list_tombstones_since(
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

    fn hlc_after_watermark(hlc: &crate::RecordHlc, watermark: &crate::SyncWatermark) -> bool {
        hlc.wall_time_ms > watermark.wall_time_ms
            || (hlc.wall_time_ms == watermark.wall_time_ms
                && (hlc.counter > watermark.counter
                    || (hlc.counter == watermark.counter && hlc.node_id > watermark.node_id)))
    }

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
    /// Returns at most `limit` records starting from `offset`. This allows the sync
    /// engine to stream large tables in multiple `Batch` messages without loading
    /// the entire result set into a single message.
    pub fn list_sync_changes_since_paginated(
        &self,
        table: &str,
        watermark: &crate::SyncWatermark,
        account_id: &str,
        local_node_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<crate::VaultSyncRecord>, String> {
        let all = self.list_sync_changes_since(table, watermark, account_id, local_node_id)?;
        Ok(all.into_iter().skip(offset).take(limit).collect())
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
        let key = self.data_key()?;
        let rows = {
            let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
            let conn = guard.as_mut().ok_or("Vault is locked")?;
            let mut stmt = conn
                .prepare(
                    "SELECT id, account_id, type_id, section_type, name, icon_name, parent_id,
                     children_ids, properties, property_labels, sensitivity_level,
                     is_deleted, deleted_at, tags_json, template_id, template_type,
                     contract_type_id, template_hash, ignored_template_hash, created_at, updated_at, version
                     FROM objects WHERE account_id = ?1",
                )
                .map_err(|e| format!("list_object_changes: {}", e))?;
            let rows = stmt
                .query_map(params![account_id], |row| {
                    let props_str: String = row.get(8)?;
                    let labels_str: String = row.get(9)?;
                    let decrypted_props = decrypt_text_field(&key, &props_str).map_err(|e| {
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
                    Ok(obj)
                })
                .map_err(|e| format!("list_object_changes query: {}", e))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("list_object_changes collect: {}", e))?;
            rows
        };

        let mut out = Vec::new();
        for obj in rows {
            let hlc =
                self.record_hlc_or_fallback("objects", &obj.id, &obj.updated_at, local_node_id)?;
            if !Self::hlc_after_watermark(&hlc, watermark) {
                continue;
            }
            let id = obj.id.clone();
            out.push(crate::VaultSyncRecord {
                id,
                table: "objects".to_string(),
                data: serde_json::to_value(&obj).unwrap_or_default(),
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
                data: serde_json::to_value(&tpl).unwrap_or_default(),
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
        let key = self.data_key()?;
        let rows = {
            let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
            let conn = guard.as_mut().ok_or("Vault is locked")?;
            let mut stmt = conn
                .prepare(
                    "SELECT id, item_type, original_id, original_parent_id, original_section_type,
                     original_sort_order, data, deleted_at, expires_at, deleted_by, name_snapshot, icon_snapshot
                     FROM trash_items",
                )
                .map_err(|e| format!("list_trash_changes: {}", e))?;
            let rows = stmt
                .query_map([], |row| {
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
                    Ok((item, deleted_at))
                })
                .map_err(|e| format!("list_trash_changes query: {}", e))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("list_trash_changes collect: {}", e))?;
            rows
        };

        let mut out = Vec::new();
        for (item, deleted_at) in rows {
            let updated = chrono::DateTime::from_timestamp(deleted_at, 0)
                .map(|d| d.to_rfc3339())
                .unwrap_or_default();
            let hlc =
                self.record_hlc_or_fallback("trash_items", &item.id, &updated, local_node_id)?;
            if !Self::hlc_after_watermark(&hlc, watermark) {
                continue;
            }
            let id = item.id.clone();
            out.push(crate::VaultSyncRecord {
                id,
                table: "trash_items".to_string(),
                data: serde_json::to_value(&item).unwrap_or_default(),
                hlc,
                deleted: false,
            });
        }
        Ok(out)
    }

    /// Apply a single incoming sync record. Returns true if the local state changed.
    pub fn apply_sync_record(
        &self,
        record: &crate::VaultSyncRecord,
        local_node_id: &str,
    ) -> Result<bool, String> {
        // Conflict resolution: only accept records with HLC greater than the local HLC.
        let current = self.get_record_hlc(&record.table, &record.id)?;
        if let Some(ref cur) = current {
            if !Self::record_hlc_is_newer(&record.hlc, cur) {
                return Ok(false);
            }
        }

        let applied = match record.table.as_str() {
            "profiles" => self.apply_profile_sync_record(record),
            "objects" => self.apply_object_sync_record(record, local_node_id),
            "user_templates" => self.apply_user_template_sync_record(record),
            "trash_items" => self.apply_trash_sync_record(record),
            _ => Err(format!("Unsupported sync table: {}", record.table)),
        }?;

        if applied {
            self.set_record_hlc(&record.table, &record.id, &record.hlc)?;
        }
        Ok(applied)
    }

    fn record_hlc_is_newer(remote: &crate::RecordHlc, local: &crate::RecordHlc) -> bool {
        remote.wall_time_ms > local.wall_time_ms
            || (remote.wall_time_ms == local.wall_time_ms
                && (remote.counter > local.counter
                    || (remote.counter == local.counter && remote.node_id > local.node_id)))
    }

    fn apply_profile_sync_record(&self, record: &crate::VaultSyncRecord) -> Result<bool, String> {
        if record.deleted {
            // Apply remote tombstone directly without creating a local tombstone,
            // so the remote HLC remains the authoritative deletion timestamp.
            let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
            let conn = guard.as_mut().ok_or("Vault is locked")?;
            conn.execute("DELETE FROM profiles WHERE id = ?1", params![&record.id])
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
            id: record.id.clone(),
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
        self.save_profile(&profile)?;
        Ok(true)
    }

    fn apply_object_sync_record(
        &self,
        record: &crate::VaultSyncRecord,
        local_node_id: &str,
    ) -> Result<bool, String> {
        let mut obj: crate::ObjectRecord = serde_json::from_value(record.data.clone())
            .map_err(|e| format!("object decode: {}", e))?;
        // Bump version if the local node is modifying an existing object.
        if self.load_object(&obj.id)?.is_some() {
            obj.version += 1;
            obj.updated_at = Self::now_rfc3339();
        }
        // Re-encrypt properties locally.
        self.save_object(&obj)?;
        // Update HLC with the remote value.
        self.set_record_hlc("objects", &record.id, &record.hlc)?;
        let _ = local_node_id;
        Ok(true)
    }

    fn apply_user_template_sync_record(
        &self,
        record: &crate::VaultSyncRecord,
    ) -> Result<bool, String> {
        if record.deleted {
            let _ = self.load_user_template(&record.id); // ensure vault is accessible
            let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
            let conn = guard.as_mut().ok_or("Vault is locked")?;
            conn.execute(
                "DELETE FROM user_templates WHERE id = ?1",
                params![record.id],
            )
            .map_err(|e| format!("delete template: {}", e))?;
            return Ok(true);
        }
        let tpl: crate::UserTemplate = serde_json::from_value(record.data.clone())
            .map_err(|e| format!("template decode: {}", e))?;
        self.save_user_template(&tpl)?;
        Ok(true)
    }

    fn apply_trash_sync_record(&self, record: &crate::VaultSyncRecord) -> Result<bool, String> {
        let item: crate::TrashItem = serde_json::from_value(record.data.clone())
            .map_err(|e| format!("trash decode: {}", e))?;
        self.save_trash_item(&item)?;
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

    // ── Object CRUD ─────────────────────────────────────────

    pub fn save_object(&self, obj: &ObjectRecord) -> Result<(), String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
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
        let children_json = serde_json::to_string(&obj.children_ids).unwrap_or_default();
        let props_json = serde_json::to_string(&properties).unwrap_or_default();
        let labels_json = obj
            .property_labels
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .unwrap_or_default();
        let encrypted_props = encrypt_text_field(&key, &props_json)?;
        let encrypted_labels = if labels_json.is_empty() {
            String::new()
        } else {
            encrypt_text_field(&key, &labels_json)?
        };
        let tags_str = serde_json::to_string(&obj.tags_json).unwrap_or_default();
        conn.execute(
            "INSERT INTO objects (id, account_id, type_id, section_type, name, icon_name, parent_id,
             children_ids, properties, property_labels, sensitivity_level,
             is_deleted, deleted_at, tags_json, template_id, template_type,
             contract_type_id, template_hash, ignored_template_hash, created_at, updated_at, version)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22)
             ON CONFLICT(id) DO UPDATE SET
               type_id=excluded.type_id, section_type=excluded.section_type, name=excluded.name, icon_name=excluded.icon_name,
               parent_id=excluded.parent_id, children_ids=excluded.children_ids,
               properties=excluded.properties, property_labels=excluded.property_labels,
               sensitivity_level=excluded.sensitivity_level,
               is_deleted=excluded.is_deleted, deleted_at=excluded.deleted_at,
               tags_json=excluded.tags_json,
               template_id=excluded.template_id, template_type=excluded.template_type,
               contract_type_id=excluded.contract_type_id, template_hash=excluded.template_hash,
               ignored_template_hash=excluded.ignored_template_hash,
               updated_at=excluded.updated_at, version=excluded.version",
            params![
                obj.id, obj.account_id, obj.type_id, obj.section_type, obj.name, obj.icon_name,
                obj.parent_id, children_json, encrypted_props, encrypted_labels,
                obj.sensitivity_level, obj.is_deleted as i32, obj.deleted_at,
                tags_str, obj.template_id, obj.template_type,
                obj.contract_type_id.clone(), obj.template_hash.clone(), obj.ignored_template_hash.clone(),
                obj.created_at, obj.updated_at, obj.version,
            ],
        )
        .map_err(|e| format!("save_object: {}", e))?;
        Ok(())
    }

    pub fn load_object(&self, id: &str) -> Result<Option<ObjectRecord>, String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn
            .prepare(
                "SELECT id, account_id, type_id, section_type, name, icon_name, parent_id,
                 children_ids, properties, property_labels, sensitivity_level,
                 is_deleted, deleted_at, tags_json, template_id, template_type,
                 contract_type_id, template_hash, ignored_template_hash, created_at, updated_at, version
                 FROM objects WHERE id = ?1",
            )
            .map_err(|e| format!("load_object: {}", e))?;
        let result = stmt
            .query_row(params![id], |row| {
                let children_str: String = row.get(7)?;
                let props_str: String = row.get(8)?;
                let labels_str: String = row.get(9)?;
                let tags_str: String = row.get(13)?;
                let deleted: i32 = row.get(11)?;
                let decrypted_props = decrypt_text_field(&key, &props_str).map_err(|e| {
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
                    decrypt_text_field(&key, &labels_str)
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
                    properties: serde_json::from_str(&decrypted_props)
                        .unwrap_or(serde_json::Value::Null),
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
            })
            .ok();
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
        let sql = format!(
            "SELECT id, account_id, type_id, section_type, name, icon_name, parent_id,
             children_ids, properties, property_labels, sensitivity_level,
             is_deleted, deleted_at, tags_json, template_id, template_type,
             contract_type_id, template_hash, ignored_template_hash, created_at, updated_at, version
             FROM objects WHERE id IN ({})",
            placeholders.join(",")
        );

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("load_objects_batch: {}", e))?;

        // Convert IDs to a slice of &dyn ToSql
        let params_refs: Vec<&dyn rusqlite::types::ToSql> = ids
            .iter()
            .map(|id| id as &dyn rusqlite::types::ToSql)
            .collect();

        let rows = stmt
            .query_map(params_refs.as_slice(), |row| {
                let children_str: String = row.get(7)?;
                let props_str: String = row.get(8)?;
                let labels_str: String = row.get(9)?;
                let tags_str: String = row.get(13)?;
                let deleted: i32 = row.get(11)?;
                let decrypted_props = decrypt_text_field(&key, &props_str).map_err(|e| {
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
                    decrypt_text_field(&key, &labels_str)
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
                    properties: serde_json::from_str(&decrypted_props)
                        .unwrap_or(serde_json::Value::Null),
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
            let props: serde_json::Value = serde_json::from_str(&decrypted).unwrap_or_default();
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
            "SELECT id, name, type_id, section_type, sensitivity_level, created_at, updated_at, is_deleted, properties, tags_json, template_id, template_type, contract_type_id, template_hash, ignored_template_hash, icon_name, property_labels
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
        sql.push_str(" ORDER BY created_at DESC");

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("list_objects: {}", e))?;

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let objects = stmt
            .query_map(params_refs.as_slice(), |row| {
                let deleted_int: i32 = row.get(7)?;
                let props_str: String = row.get(8)?;
                let tags_str: String = row.get(9)?;
                let decrypted_props = decrypt_text_field(&key, &props_str).unwrap_or_default();
                let labels_str: String = row.get::<_, String>(16).unwrap_or_default();
                let decrypted_labels = if labels_str.is_empty() {
                    Ok(String::new())
                } else {
                    decrypt_text_field(&key, &labels_str)
                }
                .unwrap_or_default();
                let property_labels: Option<serde_json::Value> = if decrypted_labels.is_empty() {
                    None
                } else {
                    serde_json::from_str(&decrypted_labels).ok()
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
                    template_id: row.get(10)?,
                    template_type: row.get(11)?,
                    contract_type_id: row.get(12)?,
                    template_hash: row.get(13)?,
                    ignored_template_hash: row.get(14)?,
                    icon_name: row.get(15)?,
                    properties: serde_json::from_str(&decrypted_props)
                        .unwrap_or(serde_json::Value::Null),
                    property_labels,
                    tags: serde_json::from_str(&tags_str).unwrap_or_default(),
                })
            })
            .map_err(|e| format!("list_objects query: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("list_objects collect: {}", e))?;

        // Memory-level keyword filtering on decrypted name and properties.
        if let Some(kw) = lower_kw {
            let filtered: Vec<ObjectSummary> = objects
                .into_iter()
                .filter(|o| {
                    o.name.to_lowercase().contains(&kw)
                        || o.properties.to_string().to_lowercase().contains(&kw)
                })
                .collect();
            Ok(filtered)
        } else {
            Ok(objects)
        }
    }

    pub fn delete_object(&self, id: &str, soft: bool) -> Result<(), String> {
        if soft {
            let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
            let conn = guard.as_mut().ok_or("Vault is locked")?;
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE objects SET is_deleted = 1, deleted_at = ?1, updated_at = ?1 WHERE id = ?2",
                params![now, id],
            )
            .map_err(|e| format!("soft_delete_object: {}", e))?;
            Ok(())
        } else {
            let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
            let conn = guard.as_mut().ok_or("Vault is locked")?;
            conn.execute("DELETE FROM objects WHERE id = ?1", params![id])
                .map_err(|e| format!("delete_object: {}", e))?;
            Ok(())
        }
    }

    pub fn restore_object(&self, id: &str) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        conn.execute(
            "UPDATE objects SET is_deleted = 0, deleted_at = NULL, updated_at = ?1 WHERE id = ?2",
            params![chrono::Utc::now().to_rfc3339(), id],
        )
        .map_err(|e| format!("restore_object: {}", e))?;
        Ok(())
    }

    pub fn search_objects(
        &self,
        account_id: &str,
        query: &str,
    ) -> Result<Vec<ObjectRecord>, String> {
        let key = self.data_key()?;
        let lower_query = query.to_lowercase();
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        // properties 已加密，无法使用 SQL LIKE。所有匹配在解密后的内存数据上进行。
        let mut stmt = conn
            .prepare(
                "SELECT id, account_id, type_id, section_type, name, icon_name, parent_id,
                 children_ids, properties, property_labels, sensitivity_level,
                 is_deleted, deleted_at, tags_json, template_id, template_type,
                 contract_type_id, template_hash, ignored_template_hash, created_at, updated_at, version
                 FROM objects
                 WHERE account_id = ?1 AND is_deleted = 0
                 ORDER BY updated_at DESC",
            )
            .map_err(|e| format!("search_objects: {}", e))?;
        let results = stmt
            .query_map(params![account_id], |row| {
                let children_str: String = row.get(7)?;
                let props_str: String = row.get(8)?;
                let labels_str: String = row.get(9)?;
                let deleted: i32 = row.get(11)?;
                let decrypted_props = decrypt_text_field(&key, &props_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        8,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Search properties decryption failed: {}", e),
                        )),
                    )
                })?;
                let decrypted_labels = if labels_str.is_empty() {
                    Ok(String::new())
                } else {
                    decrypt_text_field(&key, &labels_str)
                }
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        9,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Search labels decryption failed: {}", e),
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
                    properties: serde_json::from_str(&decrypted_props)
                        .unwrap_or(serde_json::Value::Null),
                    property_labels: if decrypted_labels.is_empty() {
                        None
                    } else {
                        serde_json::from_str(&decrypted_labels).ok()
                    },
                    sensitivity_level: row.get(10)?,
                    is_deleted: deleted != 0,
                    deleted_at: row.get(12)?,
                    tags_json: serde_json::from_str(&row.get::<_, String>(13)?).unwrap_or_default(),
                    template_id: row.get(14)?,
                    template_type: row.get(15)?,
                    contract_type_id: row.get(16)?,
                    template_hash: row.get(17)?,
                    ignored_template_hash: row.get(18)?,
                    created_at: row.get(19)?,
                    updated_at: row.get(20)?,
                    version: row.get(21)?,
                })
            })
            .map_err(|e| format!("search_objects query: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("search_objects collect: {}", e))?;

        let filtered: Vec<ObjectRecord> = results
            .into_iter()
            .filter(|r| {
                r.name.to_lowercase().contains(&lower_query)
                    || r.properties
                        .to_string()
                        .to_lowercase()
                        .contains(&lower_query)
            })
            .collect();
        Ok(filtered)
    }

    // ── Trash CRUD (§23) ────────────────────────────────────

    pub fn save_trash_item(&self, item: &TrashItem) -> Result<(), String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let encrypted_data = encrypt_field(&key, &item.data)?;
        conn.execute(
            "INSERT INTO trash_items (id, item_type, original_id, original_parent_id,
             original_section_type, original_sort_order, data, deleted_at, expires_at, deleted_by,
             name_snapshot, icon_snapshot)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            rusqlite::params![
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
            ],
        )
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
            "SELECT id, item_type, name_snapshot, icon_snapshot, deleted_at, expires_at, original_parent_id, original_section_type, data
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
                let raw_data: Vec<u8> = row.get(8)?;
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
                    name: row.get(2)?,
                    icon_id: row.get(3)?,
                    deleted_at: row.get(4)?,
                    expires_at: row.get(5)?,
                    original_parent_name: row.get(6)?,
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
            .ok();
        Ok(result)
    }

    pub fn delete_trash_item(&self, id: &str) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        conn.execute(
            "DELETE FROM trash_items WHERE id = ?1",
            rusqlite::params![id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_snapshots(&self, object_id: &str) -> Result<Vec<serde_json::Value>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, triggered_by, diff_summary FROM object_snapshots WHERE object_id=?1 ORDER BY timestamp DESC LIMIT 50"
        ).map_err(|e| e.to_string())?;
        let snapshots = stmt
            .query_map(rusqlite::params![object_id], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_,String>(0)?,
                    "timestamp": row.get::<_,i64>(1)?,
                    "triggeredBy": row.get::<_,String>(2)?,
                    "diffSummary": row.get::<_,String>(3)?,
                }))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(snapshots)
    }

    pub fn get_snapshot(&self, snapshot_id: &str) -> Result<Option<Vec<u8>>, String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let result: Option<Vec<u8>> = conn
            .query_row(
                "SELECT data FROM object_snapshots WHERE id=?1",
                rusqlite::params![snapshot_id],
                |r| {
                    let raw: Vec<u8> = r.get(0)?;
                    decrypt_field(&key, &raw).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Blob,
                            Box::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!("Snapshot decryption failed: {}", e),
                            )),
                        )
                    })
                },
            )
            .ok();
        Ok(result)
    }

    pub fn count_snapshots_batch(
        &self,
        object_ids: &[String],
    ) -> Result<std::collections::HashMap<String, usize>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        if object_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let placeholders: Vec<String> = object_ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect();
        let sql = format!("SELECT object_id, COUNT(*) FROM object_snapshots WHERE object_id IN ({}) GROUP BY object_id", placeholders.join(","));
        let params: Vec<&dyn rusqlite::types::ToSql> = object_ids
            .iter()
            .map(|s| s as &dyn rusqlite::types::ToSql)
            .collect();
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let map: std::collections::HashMap<String, usize> = stmt
            .query_map(params.as_slice(), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<std::collections::HashMap<String, usize>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(map)
    }

    /// 一次性迁移：为当前 Vault 中没有 snapshot 的活跃对象补一条初始 snapshot。
    /// 通过 sys_config 标记避免重复执行。
    pub fn backfill_missing_snapshots(&self) -> Result<usize, String> {
        const BACKFILL_FLAG: &str = "snapshot_backfill_v1";
        if self
            .get_sys_config(BACKFILL_FLAG)?
            .map(|v| v == "1")
            .unwrap_or(false)
        {
            return Ok(0);
        }

        let object_ids: Vec<String> = {
            let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
            let conn = guard.as_mut().ok_or("Vault is locked")?;
            let mut stmt = conn
                .prepare(
                    "SELECT o.id FROM objects o
                     LEFT JOIN object_snapshots s ON o.id = s.object_id
                     WHERE o.is_deleted = 0 AND s.object_id IS NULL",
                )
                .map_err(|e| e.to_string())?;
            let ids: Vec<String> = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            ids
        };

        let mut created = 0;
        for id in &object_ids {
            if let Ok(Some(obj)) = self.load_object(id) {
                let snapshot_data = serde_json::to_vec(&serde_json::json!({
                    "name": obj.name,
                    "tags": obj.tags_json,
                    "properties": obj.properties,
                }))
                .unwrap_or_default();
                if self
                    .save_snapshot(
                        id,
                        "backfill",
                        &snapshot_data,
                        "Auto-created initial snapshot",
                    )
                    .is_ok()
                {
                    created += 1;
                }
            }
        }

        let _ = self.set_sys_config(BACKFILL_FLAG, "1");
        Ok(created)
    }

    /// §25.5 — Save an object snapshot for history
    pub fn save_snapshot(
        &self,
        object_id: &str,
        triggered_by: &str,
        data: &[u8],
        diff_summary: &str,
    ) -> Result<(), String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp_millis();
        let encrypted_data = encrypt_field(&key, data)?;
        conn.execute(
            "INSERT INTO object_snapshots (id, object_id, timestamp, triggered_by, data, diff_summary)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, object_id, now, triggered_by, encrypted_data, diff_summary],
        ).map_err(|e| format!("save_snapshot: {}", e))?;
        Ok(())
    }

    /// Copy all snapshots from one object to another (used when restoring a trashed object
    /// under a new ID to preserve its history).
    pub fn copy_snapshots(&self, from_object_id: &str, to_object_id: &str) -> Result<(), String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn
            .prepare(
                "SELECT timestamp, triggered_by, data, diff_summary
             FROM object_snapshots WHERE object_id = ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows: Vec<(i64, String, Vec<u8>, String)> = stmt
            .query_map(rusqlite::params![from_object_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        drop(stmt);
        let mut insert = conn.prepare(
            "INSERT INTO object_snapshots (id, object_id, timestamp, triggered_by, data, diff_summary)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
        ).map_err(|e| e.to_string())?;
        for (timestamp, triggered_by, raw_data, diff_summary) in rows {
            let plain = decrypt_field(&key, &raw_data)?;
            let encrypted = encrypt_field(&key, &plain)?;
            let id = uuid::Uuid::new_v4().to_string();
            insert
                .execute(rusqlite::params![
                    id,
                    to_object_id,
                    timestamp,
                    triggered_by,
                    encrypted,
                    diff_summary
                ])
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Write an audit log entry with structured fields.
    /// Backward-compatible: old entries log_action(action, details) will have entity_type/entity_id/entity_name/performed_by as NULL.
    pub fn log_action(&self, action: &str, details: &str) -> Result<(), String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let now = chrono::Utc::now().to_rfc3339();
        let encrypted_details = encrypt_text_field(&key, details)?;
        conn.execute(
            "INSERT INTO audit_log (timestamp, action, performed_by, details) VALUES (?1, ?2, 'system', ?3)",
            rusqlite::params![now, action, encrypted_details],
        )
        .map_err(|e| format!("log_action: {}", e))?;
        Ok(())
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
                    while after < len && !bytes[after].is_ascii_whitespace() && bytes[after] != b'=' {
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
            obj.insert(key.to_string(), serde_json::Value::String(value.to_string()));
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
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, action, entity_type, entity_id, entity_name, performed_by, details
             FROM audit_log ORDER BY id DESC LIMIT ?1"
        ).map_err(|e| format!("list_audit_log prepare: {}", e))?;
        let entries = stmt
            .query_map(rusqlite::params![limit as i64], |row| {
                let raw_name: Option<String> = row.get(5)?;
                let raw_details: Option<String> = row.get(7)?;
                let entity_name = raw_name
                    .as_deref()
                    .map(|n| decrypt_text_field(&key, n))
                    .transpose()
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            5,
                            rusqlite::types::Type::Text,
                            Box::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!("Audit entity_name decryption failed: {}", e),
                            )),
                        )
                    })?;
                let details = raw_details
                    .as_deref()
                    .map(|d| decrypt_text_field(&key, d))
                    .transpose()
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            7,
                            rusqlite::types::Type::Text,
                            Box::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!("Audit details decryption failed: {}", e),
                            )),
                        )
                    })?;
                Ok(crate::AuditLogEntry {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    action_type: row.get(2)?,
                    entity_type: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    entity_id: row.get::<_, Option<String>>(4)?,
                    entity_name,
                    performed_by: row
                        .get::<_, Option<String>>(6)?
                        .unwrap_or_else(|| "system".to_string()),
                    details,
                })
            })
            .map_err(|e| format!("list_audit_log query: {}", e))?;
        let mut result = Vec::new();
        for entry in entries {
            result.push(entry.map_err(|e| format!("list_audit_log row: {}", e))?);
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

    /// Save a guide embedding chunk. Overwrites if id already exists.
    pub fn save_guide_embedding(&self, chunk: &crate::GuideEmbeddingChunk) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let embedding_bytes: Vec<u8> = chunk
            .embedding
            .iter()
            .flat_map(|f| f.to_ne_bytes())
            .collect();
        conn.execute(
            "INSERT OR REPLACE INTO guide_embeddings (id, guide_id, chunk_index, chunk_text, embedding, model, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                chunk.id, chunk.guide_id, chunk.chunk_index,
                chunk.chunk_text, embedding_bytes, chunk.model, chunk.created_at
            ],
        ).map_err(|e| format!("save_guide_embedding: {}", e))?;
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

    /// Delete all embeddings for a specific guide.
    pub fn delete_guide_embeddings(&self, guide_id: &str) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        conn.execute(
            "DELETE FROM guide_embeddings WHERE guide_id = ?1",
            params![guide_id],
        )
        .map_err(|e| format!("delete_guide_embeddings: {}", e))?;
        Ok(())
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
        let props_json = serde_json::to_string(&template.properties)
            .map_err(|e| format!("serialize properties: {}", e))?;
        let encrypted_props = encrypt_text_field(&key, &props_json)?;
        conn.execute(
            "INSERT INTO user_templates (id, account_id, name, icon_id, properties_json, category, contract_type_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
                 name = excluded.name,
                 icon_id = excluded.icon_id,
                 properties_json = excluded.properties_json,
                 category = excluded.category,
                 contract_type_id = excluded.contract_type_id,
                 updated_at = excluded.updated_at",
            params![
                &template.id,
                &template.account_id,
                &template.name,
                &template.icon_id,
                encrypted_props,
                &template.category,
                &template.contract_type_id,
                &template.created_at,
                &template.updated_at,
            ],
        )
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
        let mut stmt = conn.prepare(
            "SELECT id, account_id, name, icon_id, properties_json, category, contract_type_id, created_at, updated_at
             FROM user_templates WHERE id = ?1"
        ).map_err(|e| format!("prepare load_user_template: {}", e))?;

        let result = stmt.query_row(params![template_id], |row| {
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

    /// Check if a user template exists (any account).
    pub fn user_template_exists(&self, template_id: &str) -> Result<bool, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn
            .prepare("SELECT 1 FROM user_templates WHERE id = ?1 LIMIT 1")
            .map_err(|e| format!("prepare user_template_exists: {}", e))?;
        let exists: Option<i32> = stmt
            .query_row(params![template_id], |row| row.get(0))
            .optional()
            .map_err(|e| format!("user_template_exists query: {}", e))?;
        Ok(exists.is_some())
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
    use crate::Profile;
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

        let mut updated = obj.clone();
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
            name: long_name.clone(),
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
            deleted_at: chrono::Utc::now().timestamp(),
            expires_at: Some(chrono::Utc::now().timestamp() + 86400),
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
                deleted_at: chrono::Utc::now().timestamp(),
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
        let now = chrono::Utc::now().timestamp();
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
    fn test_log_action_and_list() {
        let (vault, _dir) = setup();
        vault.log_action("create", "created profile").unwrap();
        vault.log_action("update", "updated profile").unwrap();

        let logs = vault.list_audit_log(10).unwrap();
        assert!(logs.len() >= 2);
        assert_eq!(logs[0].action_type, "update");
        assert_eq!(logs[1].action_type, "create");
    }

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
        vault.save_guide_embedding(&chunk).unwrap();

        let list = vault.list_guide_embeddings().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].chunk_text, "Hello world");
        assert_eq!(list[0].embedding, vec![0.1f32, 0.2, 0.3, 0.4]);
    }

    #[test]
    fn test_delete_guide_embeddings() {
        let (vault, _dir) = setup();
        for i in 0..3 {
            let chunk = crate::GuideEmbeddingChunk {
                id: format!("chunk-{}", i),
                guide_id: "guide-a".to_string(),
                chunk_index: i,
                chunk_text: format!("text {}", i),
                embedding: vec![i as f32],
                model: "model".to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
            };
            vault.save_guide_embedding(&chunk).unwrap();
        }
        let chunk_other = crate::GuideEmbeddingChunk {
            id: "chunk-other".to_string(),
            guide_id: "guide-b".to_string(),
            chunk_index: 0,
            chunk_text: "other".to_string(),
            embedding: vec![99.0],
            model: "model".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        vault.save_guide_embedding(&chunk_other).unwrap();

        vault.delete_guide_embeddings("guide-a").unwrap();
        let remaining = vault.list_guide_embeddings().unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].guide_id, "guide-b");
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
        vault.save_guide_embedding(&chunk).unwrap();
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
            vault.save_guide_embedding(&chunk).unwrap();
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
            *guard = Some(new_key.clone());
        }

        let loaded_profile = vault.load_profile("reenc").unwrap().unwrap();
        assert_eq!(loaded_profile.data, b"data");

        let loaded_obj = vault.load_object("obj-reenc").unwrap().unwrap();
        assert_eq!(loaded_obj.properties, serde_json::json!({"k": "v"}));
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
        let pinned_created_at = loaded_v1.created_at.clone();

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
