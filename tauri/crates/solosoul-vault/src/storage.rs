//! Vault store - SQLite storage with app-layer AES-256-GCM encryption

use rusqlite::{params, Connection, OptionalExtension};
use std::sync::Mutex;

use crate::migration::run_migrations;
use crate::{
    ObjectRecord, ObjectSummary, Profile, ProfileSummary, TrashItem, TrashItemSummary, VaultConfig,
    VaultState, VaultStats,
};

/// Vault store with SQLite backing
pub struct VaultStore {
    conn: Mutex<Option<Connection>>,
    #[allow(dead_code)]
    config: VaultConfig, // reserved for future path-based vault operations
    state: VaultState,
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

        Ok(Self {
            conn: Mutex::new(Some(conn)),
            config,
            state: VaultState::Unlocked,
        })
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
        self.state
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

    pub fn lock(&mut self) {
        if let Ok(mut guard) = self.conn.lock() {
            if let Some(conn) = guard.take() {
                let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)");
            }
        }
        self.state = VaultState::Locked;
    }

    pub fn save_profile(&self, profile: &Profile) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO profiles (id, name, data, created_at, updated_at, version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name, data = excluded.data,
                updated_at = excluded.updated_at, version = excluded.version",
            params![
                profile.id,
                profile.name,
                profile.data,
                profile.created_at.to_rfc3339(),
                now,
                profile.version
            ],
        )
        .map_err(|e| format!("Failed to save profile: {}", e))?;
        Ok(())
    }

    pub fn load_profile(&self, id: &str) -> Result<Option<Profile>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn.prepare(
            "SELECT id, name, data, created_at, updated_at, version FROM profiles WHERE id = ?1"
        ).map_err(|e| format!("Failed to prepare: {}", e))?;
        let result = stmt
            .query_row(params![id], |row| {
                let created_str: String = row.get(3)?;
                let updated_str: String = row.get(4)?;
                Ok(Profile {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    data: row.get(2)?,
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
        Ok(())
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
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let children_json = serde_json::to_string(&obj.children_ids).unwrap_or_default();
        let props_json = serde_json::to_string(&obj.properties).unwrap_or_default();
        let labels_json = obj
            .property_labels
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .unwrap_or_default();
        let tags_str = serde_json::to_string(&obj.tags_json).unwrap_or_default();
        conn.execute(
            "INSERT INTO objects (id, account_id, type_id, section_type, name, icon_name, parent_id,
             children_ids, properties, property_labels, sensitivity_level,
             is_deleted, deleted_at, tags_json, template_id, template_type, created_at, updated_at, version)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19)
             ON CONFLICT(id) DO UPDATE SET
               type_id=excluded.type_id, section_type=excluded.section_type, name=excluded.name, icon_name=excluded.icon_name,
               parent_id=excluded.parent_id, children_ids=excluded.children_ids,
               properties=excluded.properties, property_labels=excluded.property_labels,
               sensitivity_level=excluded.sensitivity_level,
               is_deleted=excluded.is_deleted, deleted_at=excluded.deleted_at,
               tags_json=excluded.tags_json,
               template_id=excluded.template_id, template_type=excluded.template_type,
               updated_at=excluded.updated_at, version=excluded.version",
            params![
                obj.id, obj.account_id, obj.type_id, obj.section_type, obj.name, obj.icon_name,
                obj.parent_id, children_json, props_json, labels_json,
                obj.sensitivity_level, obj.is_deleted as i32, obj.deleted_at,
                tags_str, obj.template_id, obj.template_type,
                obj.created_at, obj.updated_at, obj.version,
            ],
        )
        .map_err(|e| format!("save_object: {}", e))?;
        Ok(())
    }

    pub fn load_object(&self, id: &str) -> Result<Option<ObjectRecord>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn
            .prepare(
                "SELECT id, account_id, type_id, section_type, name, icon_name, parent_id,
                 children_ids, properties, property_labels, sensitivity_level,
                 is_deleted, deleted_at, tags_json, template_id, template_type, created_at, updated_at, version
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
                Ok(ObjectRecord {
                    id: row.get(0)?,
                    account_id: row.get(1)?,
                    type_id: row.get(2)?,
                    section_type: row.get(3)?,
                    name: row.get(4)?,
                    icon_name: row.get(5)?,
                    parent_id: row.get(6)?,
                    children_ids: serde_json::from_str(&children_str).unwrap_or_default(),
                    properties: serde_json::from_str(&props_str).unwrap_or(serde_json::Value::Null),
                    property_labels: if labels_str.is_empty() {
                        None
                    } else {
                        serde_json::from_str(&labels_str).ok()
                    },
                    sensitivity_level: row.get(10)?,
                    is_deleted: deleted != 0,
                    deleted_at: row.get(12)?,
                    tags_json: serde_json::from_str(&tags_str).unwrap_or_default(),
                    template_id: row.get(14)?,
                    template_type: row.get(15)?,
                    created_at: row.get(16)?,
                    updated_at: row.get(17)?,
                    version: row.get(18)?,
                })
            })
            .ok();
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
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;

        let mut sql = String::from(
            "SELECT id, name, type_id, section_type, sensitivity_level, created_at, updated_at, is_deleted, properties, tags_json, template_id, template_type
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
            param_idx += 1;
        }

        if let Some(kw) = keyword {
            let like = format!("%{}%", kw);
            sql.push_str(&format!(
                " AND (name LIKE ?{} OR properties LIKE ?{})",
                param_idx,
                param_idx + 1,
            ));
            param_values.push(Box::new(like.clone()));
            param_values.push(Box::new(like));
        }

        sql.push_str(" ORDER BY updated_at DESC");

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
                    properties: serde_json::from_str(&props_str).unwrap_or(serde_json::Value::Null),
                    tags: serde_json::from_str(&tags_str).unwrap_or_default(),
                })
            })
            .map_err(|e| format!("list_objects query: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("list_objects collect: {}", e))?;
        Ok(objects)
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
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let like = format!("%{}%", query);
        let mut stmt = conn
            .prepare(
                "SELECT id, account_id, type_id, name, icon_name, parent_id,
                 children_ids, properties, property_labels, sensitivity_level,
                 is_deleted, deleted_at, tags_json, template_id, template_type, created_at, updated_at, version
                 FROM objects
                 WHERE account_id = ?1 AND is_deleted = 0
                   AND (name LIKE ?2 OR properties LIKE ?2)
                 ORDER BY updated_at DESC",
            )
            .map_err(|e| format!("search_objects: {}", e))?;
        let results = stmt
            .query_map(params![account_id, like], |row| {
                let children_str: String = row.get(6)?;
                let props_str: String = row.get(7)?;
                let labels_str: String = row.get(8)?;
                let deleted: i32 = row.get(10)?;
                Ok(ObjectRecord {
                    id: row.get(0)?,
                    account_id: row.get(1)?,
                    type_id: row.get(2)?,
                    section_type: String::new(),
                    name: row.get(3)?,
                    icon_name: row.get(4)?,
                    parent_id: row.get(5)?,
                    children_ids: serde_json::from_str(&children_str).unwrap_or_default(),
                    properties: serde_json::from_str(&props_str).unwrap_or(serde_json::Value::Null),
                    property_labels: if labels_str.is_empty() {
                        None
                    } else {
                        serde_json::from_str(&labels_str).ok()
                    },
                    sensitivity_level: row.get(9)?,
                    is_deleted: deleted != 0,
                    deleted_at: row.get(11)?,
                    tags_json: serde_json::from_str(&row.get::<_, String>(12)?).unwrap_or_default(),
                    template_id: row.get(13)?,
                    template_type: row.get(14)?,
                    created_at: row.get(15)?,
                    updated_at: row.get(16)?,
                    version: row.get(17)?,
                })
            })
            .map_err(|e| format!("search_objects query: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("search_objects collect: {}", e))?;
        Ok(results)
    }

    // ── Trash CRUD (§23) ────────────────────────────────────

    pub fn save_trash_item(&self, item: &TrashItem) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
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
                item.data,
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
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut sql = String::from(
            "SELECT id, item_type, name_snapshot, icon_snapshot, deleted_at, expires_at, original_parent_id, original_section_type
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
                Ok(TrashItemSummary {
                    id: row.get(0)?,
                    item_type: row.get(1)?,
                    name: row.get(2)?,
                    icon_id: row.get(3)?,
                    deleted_at: row.get(4)?,
                    expires_at: row.get(5)?,
                    original_parent_name: row.get(6)?,
                    original_section_type: row.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(items)
    }

    pub fn get_trash_item(&self, id: &str) -> Result<Option<TrashItem>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn.prepare(
            "SELECT id, item_type, original_id, original_parent_id, original_section_type,
             original_sort_order, data, deleted_at, expires_at, deleted_by, name_snapshot, icon_snapshot
             FROM trash_items WHERE id = ?1"
        ).map_err(|e| e.to_string())?;
        let result = stmt
            .query_row(rusqlite::params![id], |row| {
                Ok(TrashItem {
                    id: row.get(0)?,
                    item_type: row.get(1)?,
                    original_id: row.get(2)?,
                    original_parent_id: row.get(3)?,
                    original_section_type: row.get(4)?,
                    original_sort_order: row.get(5)?,
                    data: row.get(6)?,
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
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let result: Option<Vec<u8>> = conn
            .query_row(
                "SELECT data FROM object_snapshots WHERE id=?1",
                rusqlite::params![snapshot_id],
                |r| r.get(0),
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
        let map = stmt
            .query_map(params.as_slice(), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<std::collections::HashMap<String, usize>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(map)
    }

    /// §25.5 — Save an object snapshot for history
    pub fn save_snapshot(
        &self,
        object_id: &str,
        triggered_by: &str,
        data: &[u8],
        diff_summary: &str,
    ) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "INSERT INTO object_snapshots (id, object_id, timestamp, triggered_by, data, diff_summary)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, object_id, now, triggered_by, data, diff_summary],
        ).map_err(|e| format!("save_snapshot: {}", e))?;
        Ok(())
    }

    /// Copy all snapshots from one object to another (used when restoring a trashed object
    /// under a new ID to preserve its history).
    pub fn copy_snapshots(&self, from_object_id: &str, to_object_id: &str) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        conn.execute(
            "INSERT INTO object_snapshots (id, object_id, timestamp, triggered_by, data, diff_summary)
             SELECT lower(hex(randomblob(16))), ?1, timestamp, triggered_by, data, diff_summary
             FROM object_snapshots WHERE object_id = ?2",
            rusqlite::params![to_object_id, from_object_id],
        ).map_err(|e| format!("copy_snapshots: {}", e))?;
        Ok(())
    }

    /// Write an audit log entry with structured fields.
    /// Backward-compatible: old entries log_action(action, details) will have entity_type/entity_id/entity_name/performed_by as NULL.
    pub fn log_action(&self, action: &str, details: &str) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO audit_log (timestamp, action, performed_by, details) VALUES (?1, ?2, 'system', ?3)",
            rusqlite::params![now, action, details],
        )
        .map_err(|e| format!("log_action: {}", e))?;
        Ok(())
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
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO audit_log (timestamp, action, entity_type, entity_id, entity_name, performed_by, details)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                now,
                action_type,
                entity_type,
                entity_id,
                entity_name,
                performed_by,
                details,
            ],
        )
        .map_err(|e| format!("log_structured: {}", e))?;
        Ok(())
    }

    /// List recent audit log entries, newest first.
    pub fn list_audit_log(&self, limit: usize) -> Result<Vec<crate::AuditLogEntry>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, action, entity_type, entity_id, entity_name, performed_by, details
             FROM audit_log ORDER BY id DESC LIMIT ?1"
        ).map_err(|e| format!("list_audit_log prepare: {}", e))?;
        let entries = stmt
            .query_map(rusqlite::params![limit as i64], |row| {
                Ok(crate::AuditLogEntry {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    action_type: row.get(2)?,
                    entity_type: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    entity_id: row.get::<_, Option<String>>(4)?,
                    entity_name: row.get::<_, Option<String>>(5)?,
                    performed_by: row
                        .get::<_, Option<String>>(6)?
                        .unwrap_or_else(|| "system".to_string()),
                    details: row.get::<_, Option<String>>(7)?,
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
    #[allow(dead_code)]
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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    fn delete_metadata(&self, key: &str, prefix: &str) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let full_key = format!("{}_{}", prefix, key);
        conn.execute("DELETE FROM metadata WHERE key = ?1", params![full_key])
            .map_err(|e| format!("Failed to delete metadata: {}", e))?;
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
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let props_json = serde_json::to_string(&template.properties)
            .map_err(|e| format!("serialize properties: {}", e))?;
        conn.execute(
            "INSERT INTO user_templates (id, account_id, name, icon_id, properties_json, category, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET
                 name = excluded.name,
                 icon_id = excluded.icon_id,
                 properties_json = excluded.properties_json,
                 category = excluded.category,
                 updated_at = excluded.updated_at",
            params![
                &template.id,
                &template.account_id,
                &template.name,
                &template.icon_id,
                props_json,
                &template.category,
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
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn.prepare(
            "SELECT id, account_id, name, icon_id, properties_json, category, created_at, updated_at
             FROM user_templates WHERE id = ?1"
        ).map_err(|e| format!("prepare load_user_template: {}", e))?;

        let result = stmt.query_row(params![template_id], |row| {
            let props_json: String = row.get(4)?;
            let properties: Vec<crate::TemplateProperty> = serde_json::from_str(&props_json)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            Ok(crate::UserTemplate {
                id: row.get(0)?,
                account_id: row.get(1)?,
                name: row.get(2)?,
                icon_id: row.get(3)?,
                properties,
                category: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
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
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn.prepare(
            "SELECT id, account_id, name, icon_id, properties_json, category, created_at, updated_at
             FROM user_templates WHERE account_id = ?1 ORDER BY created_at DESC"
        ).map_err(|e| format!("prepare list_user_templates: {}", e))?;

        let rows = stmt
            .query_map(params![account_id], |row| {
                let props_json: String = row.get(4)?;
                let properties: Vec<crate::TemplateProperty> = serde_json::from_str(&props_json)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                Ok(crate::UserTemplate {
                    id: row.get(0)?,
                    account_id: row.get(1)?,
                    name: row.get(2)?,
                    icon_id: row.get(3)?,
                    properties,
                    category: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            })
            .map_err(|e| format!("list_user_templates query: {}", e))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| format!("list_user_templates row: {}", e))?);
        }
        Ok(result)
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

    /// Check if a template is in trash (soft-deleted).
    pub fn is_template_in_trash(&self, template_id: &str) -> Result<bool, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn
            .prepare(
                "SELECT 1 FROM trash WHERE item_type = 'template' AND original_id = ?1 LIMIT 1",
            )
            .map_err(|e| format!("prepare is_template_in_trash: {}", e))?;
        let exists: Option<i32> = stmt
            .query_row(params![template_id], |row| row.get(0))
            .optional()
            .map_err(|e| format!("is_template_in_trash query: {}", e))?;
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

    fn setup() -> (VaultStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let config = VaultConfig::new("test_account", dir.path().to_path_buf());
        let vault = VaultStore::open(config).unwrap();
        (vault, dir)
    }

    #[test]
    fn test_vault_open() {
        let dir = TempDir::new().unwrap();
        let config = VaultConfig::new("test", dir.path().to_path_buf());
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
        let mut vault = vault;
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
        let (mut vault, _dir) = setup();
        vault.lock();
        assert_eq!(vault.state(), VaultState::Locked);

        let profile = Profile::new("test", vec![1, 2, 3]);
        assert!(vault.save_profile(&profile).is_err());
        assert!(vault.load_profile("test").is_err());
        assert!(vault.list_profiles().is_err());
        assert!(vault.stats().is_err());

        let obj = ObjectRecord {
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
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
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
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
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
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
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
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
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
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
                version: 1,
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
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
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
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
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
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
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
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
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
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
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
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
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
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
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
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
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
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
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
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
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
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
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
        assert_eq!(counts.get("obj-c"), None);
    }

    #[test]
    fn test_count_snapshots_batch_empty() {
        let (vault, _dir) = setup();
        let counts = vault.count_snapshots_batch(&[]).unwrap();
        assert!(counts.is_empty());
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

        vault.delete_metadata("k1", "pfx").unwrap();
        assert!(vault.read_metadata("k1", "pfx").unwrap().is_none());
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
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
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
            id: format!("utpl_{}", uuid::Uuid::new_v4().simple()),
            account_id: account_id.to_string(),
            name: name.to_string(),
            icon_id: Some("document".to_string()),
            properties: vec![
                crate::TemplateProperty {
                    id: "full_name".to_string(),
                    name: "姓名".to_string(),
                    prop_type: crate::PropertyType::Text,
                    sensitivity_level: None,
                    sensitive: Some(false),
                    options: None,
                    deprecated_at: None,
                },
                crate::TemplateProperty {
                    id: "passport_number".to_string(),
                    name: "护照号码".to_string(),
                    prop_type: crate::PropertyType::Text,
                    sensitivity_level: None,
                    sensitive: Some(true),
                    options: None,
                    deprecated_at: None,
                },
                crate::TemplateProperty {
                    id: "expiry_date".to_string(),
                    name: "过期日期".to_string(),
                    prop_type: crate::PropertyType::Date,
                    sensitivity_level: None,
                    sensitive: Some(false),
                    options: None,
                    deprecated_at: None,
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
        // DESC order: a2 should be first (created later)
        assert_eq!(list[0].name, "模板B");
    }

    #[test]
    fn test_user_template_update() {
        let (vault, _dir) = setup();
        let mut tpl = make_test_template("acc-1", "旧名称");
        vault.save_user_template(&tpl).unwrap();

        tpl.name = "新名称".to_string();
        tpl.icon_id = Some("passport".to_string());
        tpl.properties.push(crate::TemplateProperty {
            id: "new_field".to_string(),
            name: "新字段".to_string(),
            prop_type: crate::PropertyType::Boolean,
            sensitivity_level: None,
            sensitive: Some(false),
            options: None,
            deprecated_at: None,
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
            id: "tpl_test_001".to_string(),
            account_id: "test_account".to_string(),
            name: "Test Template".to_string(),
            icon_id: Some("document".to_string()),
            properties: vec![crate::TemplateProperty {
                id: "field1".to_string(),
                name: "field1".to_string(),
                prop_type: crate::PropertyType::Text,
                sensitivity_level: None,
                sensitive: None,
                options: None,
                deprecated_at: None,
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
}
