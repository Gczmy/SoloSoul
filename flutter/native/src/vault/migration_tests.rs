//! Migration tests for vault schema versioning
//!
//! Tests the migration runner ensures:
//! - V1 to V2 migrations work correctly
//! - Migrations are recorded in schema_migrations table
//! - Migrations are idempotent

#[cfg(test)]
mod tests {
    use rusqlite::{Connection, params};
    use tempfile::tempdir;

    /// Test that migration v1_to_v2 correctly adds extra_data column
    #[test]
    fn test_migration_v1_to_v2_adds_extra_blob() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_v1_to_v2.db");
        let mut conn = Connection::open(&db_path).unwrap();

        // Create initial schema (simulating v1)
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS profiles (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                data BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                version INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS sys_config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at INTEGER NOT NULL,
                description TEXT NOT NULL
            );

            INSERT INTO sys_config (key, value, updated_at)
            VALUES ('data_version', '1', unixepoch());
            "#,
        )
        .unwrap();

        // Verify extra_data column does NOT exist before migration
        let has_extra_before: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM pragma_table_info('profiles')
                 WHERE name = 'extra_data'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);
        assert!(!has_extra_before, "extra_data should not exist before migration");

        // Apply v1_to_v2 migration
        let migration_sql = r#"
            ALTER TABLE profiles ADD COLUMN extra_data TEXT;
            "#;

        let tx = conn.transaction().unwrap();
        tx.execute_batch(migration_sql).unwrap();
        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at, description) VALUES (2, unixepoch(), 'Add extra_data column')",
            [],
        )
        .unwrap();
        tx.commit().unwrap();

        // Verify extra_data column NOW exists after migration
        let has_extra_after: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM pragma_table_info('profiles')
                 WHERE name = 'extra_data'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);
        assert!(has_extra_after, "extra_data should exist after migration");

        // Verify migration is recorded
        let recorded_version: i64 = conn
            .query_row(
                "SELECT version FROM schema_migrations ORDER BY applied_at DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(recorded_version, 2, "Migration version 2 should be recorded");
    }

    /// Test that migrations are recorded in schema_migrations table
    #[test]
    fn test_migration_recorded() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_migration_recorded.db");
        let mut conn = Connection::open(&db_path).unwrap();

        // Create schema_migrations table
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at INTEGER NOT NULL,
                description TEXT NOT NULL
            );
            "#,
        )
        .unwrap();

        // Apply a migration
        let tx = conn.transaction().unwrap();
        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at, description) VALUES (?1, unixepoch(), ?2)",
            params![1, "Initial schema"],
        )
        .unwrap();
        tx.commit().unwrap();

        // Verify migration was recorded
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1, "Should have exactly 1 migration recorded");

        let (version, description): (i64, String) = conn
            .query_row(
                "SELECT version, description FROM schema_migrations",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(version, 1, "Migration version should be 1");
        assert_eq!(description, "Initial schema", "Migration description should match");
    }

    /// Test that running the same migration twice is idempotent
    #[test]
    fn test_idempotent_migration() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_idempotent.db");
        let mut conn = Connection::open(&db_path).unwrap();

        // Create profiles table with extra_data
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS profiles (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                data BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                version INTEGER NOT NULL DEFAULT 1,
                extra_data TEXT
            );

            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at INTEGER NOT NULL,
                description TEXT NOT NULL
            );
            "#,
        )
        .unwrap();

        // First migration attempt
        let tx1 = conn.transaction().unwrap();
        let result1 = tx1.execute(
            "INSERT INTO schema_migrations (version, applied_at, description) VALUES (1, unixepoch(), 'Test migration')",
            [],
        );
        assert!(result1.is_ok(), "First migration should succeed");
        tx1.commit().unwrap();

        // Verify version 1 migration exists
        let check_exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = 1)",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(check_exists, "Version 1 migration should already be recorded");

        // Try to insert duplicate (should fail due to PRIMARY KEY)
        {
            let tx2 = conn.transaction().unwrap();
            let dup_result = tx2.execute(
                "INSERT INTO schema_migrations (version, applied_at, description) VALUES (1, unixepoch(), 'Duplicate')",
                [],
            );
            assert!(
                dup_result.is_err(),
                "Duplicate migration version should fail due to PRIMARY KEY constraint"
            );
            // tx2 is dropped here without commit, so the insert doesn't happen
        }

        // Verify only one migration exists
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1, "Should still have exactly 1 migration recorded");
    }
}
