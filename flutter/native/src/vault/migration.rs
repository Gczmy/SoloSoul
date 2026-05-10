//! Schema migration runner for vault database
//!
//! Implements atomic schema migrations with version tracking.

use chrono::Utc;
use rusqlite::{params, Connection};

/// Current schema version
pub const CURRENT_SCHEMA_VERSION: u32 = 3;

/// Get the current schema version from sys_config
pub fn get_schema_version(conn: &Connection) -> Result<u32, String> {
    let version: String = conn
        .query_row(
            "SELECT value FROM sys_config WHERE key = 'data_version'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to get schema version: {}", e))?;

    version
        .parse::<u32>()
        .map_err(|e| format!("Invalid schema version format: {}", e))
}

/// Set the schema version in sys_config
pub fn set_schema_version(conn: &Connection, version: u32) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT OR REPLACE INTO sys_config (key, value, updated_at) VALUES ('data_version', ?1, ?2)",
        params![version.to_string(), now],
    )
    .map_err(|e| format!("Failed to set schema version: {}", e))?;
    Ok(())
}

/// Apply a migration atomically within a transaction
pub fn apply_migration(
    conn: &mut Connection,
    version: u32,
    sql: &str,
    description: &str,
) -> Result<(), String> {
    let tx = conn
        .transaction()
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    // Execute the migration SQL
    tx.execute_batch(sql)
        .map_err(|e| format!("Migration {} failed: {}", version, e))?;

    // Record the migration
    let now = Utc::now().timestamp();
    tx.execute(
        "INSERT INTO schema_migrations (version, applied_at, description) VALUES (?1, ?2, ?3)",
        params![version, now, description],
    )
    .map_err(|e| format!("Failed to record migration: {}", e))?;

    // Update the schema version
    tx.execute(
        "INSERT OR REPLACE INTO sys_config (key, value, updated_at) VALUES ('data_version', ?1, ?2)",
        params![version.to_string(), Utc::now().to_rfc3339()],
    )
    .map_err(|e| format!("Failed to update schema version: {}", e))?;

    tx.commit()
        .map_err(|e| format!("Failed to commit migration: {}", e))?;

    Ok(())
}

/// Migration v1 -> v2: Add extra_data column to profiles table
pub fn migrate_v1_to_v2(conn: &mut Connection) -> Result<(), String> {
    apply_migration(
        conn,
        2,
        r#"
        -- Add extra_data column for future extensibility
        ALTER TABLE profiles ADD COLUMN extra_data TEXT;
        "#,
        "Add extra_data column for extensibility",
    )
}

/// Migration v2 -> v3: Add updated_at column to metadata table
pub fn migrate_v2_to_v3(conn: &mut Connection) -> Result<(), String> {
    // Check if metadata table exists by querying if it has the key column
    let metadata_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('metadata') WHERE name = 'key'",
            [],
            |row| row.get::<_, i32>(0),
        )
        .map(|count| count > 0)
        .unwrap_or(false);

    // If metadata table doesn't exist, create it
    if !metadata_exists {
        conn.execute(
            "CREATE TABLE metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT)",
            [],
        )
        .map_err(|e| format!("Failed to create metadata table: {}", e))?;
    } else {
        // Check if updated_at column exists
        let has_updated_at: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('metadata') WHERE name = 'updated_at'",
                [],
                |row| row.get::<_, i32>(0),
            )
            .map(|count| count > 0)
            .unwrap_or(false);

        // If column doesn't exist, add it
        if !has_updated_at {
            conn.execute("ALTER TABLE metadata ADD COLUMN updated_at TEXT", [])
                .map_err(|e| format!("Failed to add updated_at column: {}", e))?;
        }
    }

    // Record migration
    let now = Utc::now().timestamp();
    conn.execute(
        "INSERT INTO schema_migrations (version, applied_at, description) VALUES (3, ?1, ?2)",
        params![now, "Add updated_at column to metadata table"],
    )
    .map_err(|e| format!("Failed to record migration: {}", e))?;

    // Update schema version
    conn.execute(
        "INSERT OR REPLACE INTO sys_config (key, value, updated_at) VALUES ('data_version', '3', ?1)",
        params![Utc::now().to_rfc3339()],
    )
    .map_err(|e| format!("Failed to update schema version: {}", e))?;

    Ok(())
}

/// Run all pending migrations
pub fn run_migrations(conn: &mut Connection) -> Result<(), String> {
    let current_version = get_schema_version(conn)?;

    if current_version < 2 {
        migrate_v1_to_v2(conn)?;
    }

    if current_version < 3 {
        migrate_v2_to_v3(conn)?;
    }

    let _final_version = get_schema_version(conn)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn create_test_db() -> (Connection, PathBuf) {
        let temp_dir =
            std::env::temp_dir().join(format!("solosoul_migration_test_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).ok();
        let db_path = temp_dir.join("test.db");
        let conn = Connection::open(&db_path).unwrap();
        (conn, temp_dir)
    }

    fn cleanup(temp_dir: PathBuf) {
        std::fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_get_schema_version_default() {
        let (conn, temp_dir) = create_test_db();

        // Initialize schema with sys_config
        conn.execute_batch(
            r#"
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
            "#,
        )
        .unwrap();

        // Set initial version
        set_schema_version(&conn, 1).unwrap();

        let version = get_schema_version(&conn).unwrap();
        assert_eq!(version, 1);

        cleanup(temp_dir);
    }

    #[test]
    fn test_apply_migration_atomic() {
        let (mut conn, temp_dir) = create_test_db();

        // Initialize schema
        conn.execute_batch(
            r#"
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
            CREATE TABLE IF NOT EXISTS profiles (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                data BLOB NOT NULL
            );
            "#,
        )
        .unwrap();

        set_schema_version(&conn, 1).unwrap();

        // Apply a migration
        let sql = "ALTER TABLE profiles ADD COLUMN extra_data TEXT;";
        apply_migration(&mut conn, 2, sql, "Add extra_data column").unwrap();

        // Verify version updated
        assert_eq!(get_schema_version(&conn).unwrap(), 2);

        // Verify migration recorded
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = 2",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        // Verify column exists
        let column_exists: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('profiles') WHERE name = 'extra_data'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(column_exists, 1);

        cleanup(temp_dir);
    }

    #[test]
    fn test_run_migrations_v1_to_v2() {
        let (mut conn, temp_dir) = create_test_db();

        // Initialize schema (simulating v1 setup)
        conn.execute_batch(
            r#"
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
            CREATE TABLE IF NOT EXISTS profiles (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                data BLOB NOT NULL
            );
            "#,
        )
        .unwrap();

        set_schema_version(&conn, 1).unwrap();

        // Run migrations
        run_migrations(&mut conn).unwrap();

        // Verify final version
        assert_eq!(get_schema_version(&conn).unwrap(), 3);

        // Verify extra_data column exists
        let column_exists: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('profiles') WHERE name = 'extra_data'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(column_exists, 1);

        cleanup(temp_dir);
    }

    #[test]
    fn test_run_migrations_v2_to_v3() {
        let (mut conn, temp_dir) = create_test_db();

        // Initialize schema (simulating v2 setup)
        conn.execute_batch(
            r#"
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
            CREATE TABLE IF NOT EXISTS profiles (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                data BLOB NOT NULL,
                extra_data TEXT
            );
            CREATE TABLE IF NOT EXISTS metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            "#,
        )
        .unwrap();

        set_schema_version(&conn, 2).unwrap();

        // Run migrations
        run_migrations(&mut conn).unwrap();

        // Verify final version
        assert_eq!(get_schema_version(&conn).unwrap(), 3);

        // Verify updated_at column exists in metadata
        let column_exists: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('metadata') WHERE name = 'updated_at'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(column_exists, 1);

        cleanup(temp_dir);
    }

    #[test]
    fn test_migration_idempotent() {
        let (mut conn, temp_dir) = create_test_db();

        // Initialize schema
        conn.execute_batch(
            r#"
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
            CREATE TABLE IF NOT EXISTS profiles (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                data BLOB NOT NULL,
                extra_data TEXT
            );
            "#,
        )
        .unwrap();

        // Set version to 3 (already at current)
        set_schema_version(&conn, 3).unwrap();

        // Run migrations - should be no-op since already at current
        run_migrations(&mut conn).unwrap();

        // Verify still at version 3
        assert_eq!(get_schema_version(&conn).unwrap(), 3);

        cleanup(temp_dir);
    }
}
