//! Vault store - SQLite storage with app-layer AES-256-GCM encryption

use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::migration::run_migrations;
use crate::{ObjectRecord, ObjectSummary, Profile, ProfileSummary, TrashItem, TrashItemSummary, VaultConfig, VaultState, VaultStats};

/// Vault store with SQLite backing
pub struct VaultStore {
    conn: Mutex<Option<Connection>>,
    config: VaultConfig,
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
                details TEXT
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
                item_type TEXT NOT NULL CHECK(item_type IN ('page','collection','object')),
                original_id TEXT NOT NULL,
                original_parent_id TEXT,
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
            "#,
        )
        .map_err(|e| format!("Failed to init schema: {}", e))?;

        // Migration: add tags_json column if missing (added in schema v2, §24)
        let _ = conn.execute("ALTER TABLE objects ADD COLUMN tags_json TEXT NOT NULL DEFAULT '[]'", []);
        // Migration: add section_type column if missing (§25.1.3)
        let _ = conn.execute("ALTER TABLE objects ADD COLUMN section_type TEXT NOT NULL DEFAULT 'identity'", []);

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
        let total_size_bytes: u64 = conn
            .query_row(
                "SELECT COALESCE(SUM(LENGTH(data)), 0) FROM profiles",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let last_modified: Option<String> = conn
            .query_row("SELECT MAX(updated_at) FROM profiles", [], |r| r.get(0))
            .ok();
        Ok(VaultStats {
            profile_count,
            total_size_bytes,
            last_modified,
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
             is_deleted, deleted_at, tags_json, created_at, updated_at, version)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)
             ON CONFLICT(id) DO UPDATE SET
               type_id=excluded.type_id, section_type=excluded.section_type, name=excluded.name, icon_name=excluded.icon_name,
               parent_id=excluded.parent_id, children_ids=excluded.children_ids,
               properties=excluded.properties, property_labels=excluded.property_labels,
               sensitivity_level=excluded.sensitivity_level,
               is_deleted=excluded.is_deleted, deleted_at=excluded.deleted_at,
               tags_json=excluded.tags_json,
               updated_at=excluded.updated_at, version=excluded.version",
            params![
                obj.id, obj.account_id, obj.type_id, obj.section_type, obj.name, obj.icon_name,
                obj.parent_id, children_json, props_json, labels_json,
                obj.sensitivity_level, obj.is_deleted as i32, obj.deleted_at,
                tags_str, obj.created_at, obj.updated_at, obj.version,
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
                "SELECT id, account_id, type_id, name, icon_name, parent_id,
                 children_ids, properties, property_labels, sensitivity_level,
                 is_deleted, deleted_at, tags_json, created_at, updated_at, version
                 FROM objects WHERE id = ?1",
            )
            .map_err(|e| format!("load_object: {}", e))?;
        let result = stmt.query_row(params![id], |row| {
            let children_str: String = row.get(6)?;
            let props_str: String = row.get(7)?;
            let labels_str: String = row.get(8)?;
            let tags_str: String = row.get(12)?;
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
                deleted_at: row.get(11)?, // note: deleted_at col index changed due to tags_json insert
                tags_json: serde_json::from_str(&tags_str).unwrap_or_default(),
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
                version: row.get(15)?,
            })
        }).ok();
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
            "SELECT id, name, type_id, sensitivity_level, created_at, updated_at, is_deleted, properties, tags_json
             FROM objects WHERE account_id = ?1",
        );
        let mut param_idx = 2;
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(account_id.to_string())];

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

        let mut stmt = conn.prepare(&sql).map_err(|e| format!("list_objects: {}", e))?;

        let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();

        let objects = stmt
            .query_map(params_refs.as_slice(), |row| {
                let deleted_int: i32 = row.get(6)?;
                let props_str: String = row.get(7)?;
                let tags_str: String = row.get(8)?;
                Ok(ObjectSummary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    collection_type: row.get(2)?,
                    sensitivity_level: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                    is_deleted: deleted_int != 0,
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

    pub fn search_objects(&self, account_id: &str, query: &str) -> Result<Vec<ObjectRecord>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let like = format!("%{}%", query);
        let mut stmt = conn
            .prepare(
                "SELECT id, account_id, type_id, name, icon_name, parent_id,
                 children_ids, properties, property_labels, sensitivity_level,
                 is_deleted, deleted_at, tags_json, created_at, updated_at, version
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
                    created_at: row.get(13)?,
                    updated_at: row.get(14)?,
                    version: row.get(15)?,
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
             original_sort_order, data, deleted_at, expires_at, deleted_by,
             name_snapshot, icon_snapshot)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            rusqlite::params![
                item.id, item.item_type, item.original_id, item.original_parent_id,
                item.original_sort_order, item.data, item.deleted_at, item.expires_at,
                item.deleted_by, item.name_snapshot, item.icon_snapshot,
            ],
        ).map_err(|e| format!("save_trash_item: {}", e))?;
        Ok(())
    }

    pub fn list_trash_items(
        &self, item_type: Option<&str>, since: Option<i64>,
    ) -> Result<Vec<TrashItemSummary>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut sql = String::from(
            "SELECT id, item_type, name_snapshot, icon_snapshot, deleted_at, expires_at, original_parent_id
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
        let items = stmt.query_map(p.as_slice(), |row| {
            Ok(TrashItemSummary {
                id: row.get(0)?, item_type: row.get(1)?, name: row.get(2)?,
                icon_id: row.get(3)?, deleted_at: row.get(4)?, expires_at: row.get(5)?,
                original_parent_name: row.get(6)?,
            })
        }).map_err(|e| e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e| e.to_string())?;
        Ok(items)
    }

    pub fn get_trash_item(&self, id: &str) -> Result<Option<TrashItem>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn.prepare(
            "SELECT id, item_type, original_id, original_parent_id, original_sort_order,
             data, deleted_at, expires_at, deleted_by, name_snapshot, icon_snapshot
             FROM trash_items WHERE id = ?1"
        ).map_err(|e| e.to_string())?;
        let result = stmt.query_row(rusqlite::params![id], |row| {
            Ok(TrashItem {
                id: row.get(0)?, item_type: row.get(1)?, original_id: row.get(2)?,
                original_parent_id: row.get(3)?, original_sort_order: row.get(4)?,
                data: row.get(5)?, deleted_at: row.get(6)?, expires_at: row.get(7)?,
                deleted_by: row.get(8)?, name_snapshot: row.get(9)?, icon_snapshot: row.get(10)?,
            })
        }).ok();
        Ok(result)
    }

    pub fn delete_trash_item(&self, id: &str) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        conn.execute("DELETE FROM trash_items WHERE id = ?1", rusqlite::params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_snapshots(&self, object_id: &str) -> Result<Vec<serde_json::Value>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, triggered_by, diff_summary FROM object_snapshots WHERE object_id=?1 ORDER BY timestamp DESC LIMIT 50"
        ).map_err(|e| e.to_string())?;
        let snapshots = stmt.query_map(rusqlite::params![object_id], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_,String>(0)?,
                "timestamp": row.get::<_,i64>(1)?,
                "triggeredBy": row.get::<_,String>(2)?,
                "diffSummary": row.get::<_,String>(3)?,
            }))
        }).map_err(|e| e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e| e.to_string())?;
        Ok(snapshots)
    }

    pub fn get_snapshot(&self, snapshot_id: &str) -> Result<Option<Vec<u8>>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let result: Option<Vec<u8>> = conn.query_row(
            "SELECT data FROM object_snapshots WHERE id=?1", rusqlite::params![snapshot_id], |r| r.get(0)
        ).ok();
        Ok(result)
    }

    pub fn count_snapshots_batch(&self, object_ids: &[String]) -> Result<std::collections::HashMap<String, usize>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let placeholders: Vec<String> = object_ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
        let sql = format!("SELECT object_id, COUNT(*) FROM object_snapshots WHERE object_id IN ({}) GROUP BY object_id", placeholders.join(","));
        let params: Vec<&dyn rusqlite::types::ToSql> = object_ids.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let map = stmt.query_map(params.as_slice(), |row| {
            Ok((row.get::<_,String>(0)?, row.get::<_,i64>(1)? as usize))
        }).map_err(|e| e.to_string())?
        .collect::<Result<std::collections::HashMap<String, usize>, _>>()
        .map_err(|e| e.to_string())?;
        Ok(map)
    }

    /// §25.5 — Save an object snapshot for history
    pub fn save_snapshot(&self, object_id: &str, triggered_by: &str, data: &[u8], diff_summary: &str) -> Result<(), String> {
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

    /// Write an audit log entry.
    pub fn log_action(&self, action: &str, details: &str) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO audit_log (timestamp, action, details) VALUES (?1, ?2, ?3)",
            rusqlite::params![now, action, details],
        )
        .map_err(|e| format!("log_action: {}", e))?;
        Ok(())
    }

    // Metadata helpers for encrypted blob storage
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

    fn delete_metadata(&self, key: &str, prefix: &str) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let full_key = format!("{}_{}", prefix, key);
        conn.execute("DELETE FROM metadata WHERE key = ?1", params![full_key])
            .map_err(|e| format!("Failed to delete metadata: {}", e))?;
        Ok(())
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
}
