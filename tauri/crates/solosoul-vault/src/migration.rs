//! Schema migration runner for vault database

use chrono::Utc;
use rusqlite::{params, Connection};

pub const CURRENT_SCHEMA_VERSION: u32 = 7;

pub fn get_schema_version(conn: &Connection) -> Result<u32, String> {
    let version: String = conn
        .query_row(
            "SELECT value FROM sys_config WHERE key = 'data_version'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| format!("Failed to get schema version: {}", e))?;
    version
        .parse::<u32>()
        .map_err(|e| format!("Invalid version: {}", e))
}

pub fn set_schema_version(conn: &Connection, version: u32) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT OR REPLACE INTO sys_config (key, value, updated_at) VALUES ('data_version', ?1, ?2)",
        params![version.to_string(), now],
    ).map_err(|e| format!("Failed to set version: {}", e))?;
    Ok(())
}

/// Run all pending migrations
pub fn run_migrations(conn: &mut Connection) -> Result<(), String> {
    let current = get_schema_version(conn).unwrap_or(1);

    if current < 2 {
        apply_migration(
            conn,
            2,
            "ALTER TABLE profiles ADD COLUMN extra_data TEXT;",
            "Add extra_data column",
        )?;
    }
    if current < 3 {
        // Ensure metadata table has updated_at column
        let has_updated: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('metadata') WHERE name = 'updated_at'",
                [],
                |r| r.get::<_, i32>(0),
            )
            .unwrap_or(0)
            > 0;
        if !has_updated {
            let metadata_exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('metadata') WHERE name = 'key'",
                    [],
                    |r| r.get::<_, i32>(0),
                )
                .unwrap_or(0)
                > 0;
            if !metadata_exists {
                conn.execute("CREATE TABLE metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT)", [])
                    .map_err(|e| format!("Create metadata: {}", e))?;
            } else {
                conn.execute("ALTER TABLE metadata ADD COLUMN updated_at TEXT", [])
                    .map_err(|e| format!("Add updated_at: {}", e))?;
            }
        }
        set_schema_version(conn, 3)?;
    }
    if current < 4 {
        apply_migration(
            conn,
            4,
            "ALTER TABLE trash_items ADD COLUMN original_section_type TEXT;",
            "Add original_section_type to trash_items",
        )?;
    }
    if current < 5 {
        // Add structured audit log columns (may already exist from init_schema)
        let has_entity_type: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('audit_log') WHERE name = 'entity_type'",
                [],
                |r| r.get::<_, i32>(0),
            )
            .unwrap_or(0)
            > 0;
        if !has_entity_type {
            let tx = conn.transaction().map_err(|e| format!("Begin tx: {}", e))?;
            tx.execute_batch(
                "ALTER TABLE audit_log ADD COLUMN entity_type TEXT;
                 ALTER TABLE audit_log ADD COLUMN entity_id TEXT;
                 ALTER TABLE audit_log ADD COLUMN entity_name TEXT;
                 ALTER TABLE audit_log ADD COLUMN performed_by TEXT DEFAULT 'user';",
            )
            .map_err(|e| format!("Migration 5 failed: {}", e))?;
            let now = Utc::now().timestamp();
            tx.execute(
                "INSERT INTO schema_migrations (version, applied_at, description) VALUES (?1, ?2, ?3)",
                params![5, now, "Add structured columns to audit_log"],
            ).map_err(|e| format!("Record migration 5: {}", e))?;
            tx.commit()
                .map_err(|e| format!("Commit migration 5: {}", e))?;
        } else {
            // Fresh DB: init_schema already has the columns, just mark version 5
            let now = Utc::now().timestamp();
            conn.execute(
                "INSERT OR IGNORE INTO schema_migrations (version, applied_at, description) VALUES (?1, ?2, ?3)",
                params![5, now, "audit_log columns already present (init_schema)"],
            ).ok();
        }
        set_schema_version(conn, 5)?;
    }
    if current < 6 {
        apply_migration(
            conn,
            6,
            "CREATE TABLE IF NOT EXISTS guide_embeddings (
                id TEXT PRIMARY KEY,
                guide_id TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                chunk_text TEXT NOT NULL,
                embedding BLOB NOT NULL,
                model TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_guide_embeddings_guide ON guide_embeddings(guide_id);",
            "Add guide_embeddings table for RAG vector search",
        )?;
    }
    if current < 7 {
        apply_migration(
            conn,
            7,
            "CREATE TABLE IF NOT EXISTS user_templates (
                id TEXT PRIMARY KEY,
                account_id TEXT NOT NULL,
                name TEXT NOT NULL,
                icon_id TEXT,
                properties_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_user_templates_account ON user_templates(account_id);",
            "Add user_templates table for custom object templates",
        )?;
    }
    Ok(())
}

fn apply_migration(
    conn: &mut Connection,
    version: u32,
    sql: &str,
    description: &str,
) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| format!("Begin tx: {}", e))?;
    tx.execute_batch(sql)
        .map_err(|e| format!("Migration {} failed: {}", version, e))?;
    let now = Utc::now().timestamp();
    tx.execute(
        "INSERT INTO schema_migrations (version, applied_at, description) VALUES (?1, ?2, ?3)",
        params![version, now, description],
    )
    .map_err(|e| format!("Record migration: {}", e))?;
    set_schema_version(&tx, version)?;
    tx.commit()
        .map_err(|e| format!("Commit migration: {}", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_conn() -> (Connection, TempDir) {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sys_config (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL, description TEXT);
             CREATE TABLE IF NOT EXISTS profiles (id TEXT PRIMARY KEY, name TEXT NOT NULL, data BLOB NOT NULL);
             CREATE TABLE IF NOT EXISTS trash_items (id TEXT PRIMARY KEY, item_type TEXT NOT NULL, original_id TEXT NOT NULL, data BLOB NOT NULL, deleted_at INTEGER NOT NULL, expires_at INTEGER, deleted_by TEXT DEFAULT 'user', name_snapshot TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS audit_log (id INTEGER PRIMARY KEY AUTOINCREMENT, timestamp TEXT NOT NULL, action TEXT NOT NULL, details TEXT);
             CREATE TABLE IF NOT EXISTS guide_embeddings (id TEXT PRIMARY KEY, guide_id TEXT NOT NULL, chunk_index INTEGER NOT NULL, chunk_text TEXT NOT NULL, embedding BLOB NOT NULL, model TEXT NOT NULL, created_at TEXT NOT NULL);"
        ).unwrap();
        set_schema_version(&conn, 1).unwrap();
        (conn, dir)
    }

    #[test]
    fn test_version_roundtrip() {
        let (conn, _dir) = setup_conn();
        assert_eq!(get_schema_version(&conn).unwrap(), 1);
    }

    #[test]
    fn test_run_migrations() {
        let (mut conn, _dir) = setup_conn();
        run_migrations(&mut conn).unwrap();
        assert_eq!(get_schema_version(&conn).unwrap(), CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn test_migration_v7_creates_user_templates_table() {
        let (mut conn, _dir) = setup_conn();
        run_migrations(&mut conn).unwrap();

        // Verify user_templates table exists
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='user_templates'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        // Verify index exists
        let idx_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_user_templates_account'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(idx_count, 1);

        // Verify we can insert and query
        conn.execute(
            "INSERT INTO user_templates (id, account_id, name, icon_id, properties_json, created_at)
             VALUES ('t1', 'acc1', 'Test', 'doc', '[]', '2024-01-01T00:00:00Z')",
            [],
        ).unwrap();

        let name: String = conn
            .query_row("SELECT name FROM user_templates WHERE id = 't1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(name, "Test");
    }
}
