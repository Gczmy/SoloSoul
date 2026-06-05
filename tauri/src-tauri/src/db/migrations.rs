//! Database migration framework.
//!
//! Migrations are applied in order using a `_migrations` tracking table.
//! Each migration runs inside a transaction for safety.

use rusqlite::Connection;
use tracing::{error, info};

/// A single database migration.
pub struct Migration {
    pub version: u32,
    pub name: &'static str,
    pub sql: &'static str,
}

/// All migrations in version order.
///
/// When adding a new migration, append it to this array with the next
/// sequential version number. Never reorder or delete existing entries.
pub const MIGRATIONS: &[Migration] = &[
    // v1: Initial schema — core tables
    Migration {
        version: 1,
        name: "initial_schema",
        sql: r#"
CREATE TABLE IF NOT EXISTS profiles (
    id          TEXT PRIMARY KEY,
    account_id  TEXT NOT NULL,
    name        TEXT NOT NULL,
    data        BLOB,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    version     INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS objects (
    id               TEXT PRIMARY KEY,
    account_id       TEXT NOT NULL,
    name             TEXT NOT NULL,
    collection_type  TEXT NOT NULL,
    properties       BLOB,
    sensitivity_level TEXT NOT NULL DEFAULT 'private',
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL,
    deleted_at       TEXT
);

CREATE TABLE IF NOT EXISTS settings (
    key        TEXT PRIMARY KEY,
    value      TEXT,
    account_id TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sensitivity_map (
    field_id       TEXT PRIMARY KEY,
    level          TEXT NOT NULL,
    last_modified  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS operation_log (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id   TEXT NOT NULL,
    action       TEXT NOT NULL,
    target_type  TEXT NOT NULL,
    target_id    TEXT,
    detail       TEXT,
    timestamp    TEXT NOT NULL
);
"#,
    },
];

/// Apply all pending migrations to the database.
///
/// Creates the `_migrations` tracking table if it does not exist, then
/// iterates through [`MIGRATIONS`] in order, skipping any that have
/// already been applied. Each migration runs inside its own transaction.
pub fn run_migrations(conn: &Connection) -> Result<(), String> {
    // Ensure the tracking table exists before we do anything else.
    conn.execute(
        "CREATE TABLE IF NOT EXISTS _migrations (
            version     INTEGER PRIMARY KEY,
            name        TEXT NOT NULL,
            applied_at  TEXT NOT NULL
        )",
        [],
    )
    .map_err(|e| format!("Failed to create _migrations table: {e}"))?;

    // Collect versions that have already been applied.
    let mut stmt = conn
        .prepare("SELECT version FROM _migrations ORDER BY version")
        .map_err(|e| format!("Failed to query _migrations: {e}"))?;
    let applied_versions: Vec<u32> = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| format!("Failed to read _migrations: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    for migration in MIGRATIONS {
        if applied_versions.contains(&migration.version) {
            continue;
        }

        info!(
            version = migration.version,
            name = migration.name,
            "Applying database migration"
        );

        conn.execute("BEGIN", [])
            .map_err(|e| format!("Failed to begin transaction: {e}"))?;

        let result = apply_single(conn, migration);

        match result {
            Ok(()) => {
                conn.execute("COMMIT", [])
                    .map_err(|e| format!("Failed to commit transaction: {e}"))?;
                info!(
                    version = migration.version,
                    name = migration.name,
                    "Migration applied successfully"
                );
            }
            Err(e) => {
                // Best-effort rollback; if this fails we still return the
                // original error because nothing else can be done.
                if let Err(rollback_err) = conn.execute("ROLLBACK", []) {
                    error!("Rollback failed after migration error: {rollback_err}");
                }
                error!(
                    version = migration.version,
                    name = migration.name,
                    error = %e,
                    "Migration failed, transaction rolled back"
                );
                return Err(e);
            }
        }
    }

    Ok(())
}

/// Execute a single migration's SQL and record it in `_migrations`.
fn apply_single(conn: &Connection, migration: &Migration) -> Result<(), String> {
    conn.execute_batch(migration.sql)
        .map_err(|e| {
            format!(
                "Migration v{} ({}) SQL error: {e}",
                migration.version, migration.name,
            )
        })?;

    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO _migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![migration.version, migration.name, now],
    )
    .map_err(|e| {
        format!(
            "Failed to record migration v{}: {e}",
            migration.version,
        )
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create an in-memory database and run migrations on it.
    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().expect("failed to open in-memory db");
        run_migrations(&conn).expect("migrations should succeed on fresh db");
        conn
    }

    #[test]
    fn test_run_migrations_creates_tables() {
        let conn = setup_test_db();

        // Verify every table from v1 exists.
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap();
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"_migrations".to_string()));
        assert!(tables.contains(&"profiles".to_string()));
        assert!(tables.contains(&"objects".to_string()));
        assert!(tables.contains(&"settings".to_string()));
        assert!(tables.contains(&"sensitivity_map".to_string()));
        assert!(tables.contains(&"operation_log".to_string()));
    }

    #[test]
    fn test_migrations_are_idempotent() {
        let conn = setup_test_db();

        // Running migrations again should succeed without errors.
        run_migrations(&conn).expect("re-running migrations should be idempotent");

        // The _migrations table should still have exactly one row per migration.
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM _migrations", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, MIGRATIONS.len() as i64);
    }

    #[test]
    fn test_migration_records_metadata() {
        let conn = setup_test_db();

        let mut stmt = conn
            .prepare("SELECT version, name, applied_at FROM _migrations ORDER BY version")
            .unwrap();
        let rows: Vec<(u32, String, String)> = stmt
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(rows.len(), MIGRATIONS.len());
        for (i, (version, name, applied_at)) in rows.iter().enumerate() {
            assert_eq!(*version, MIGRATIONS[i].version);
            assert_eq!(name, MIGRATIONS[i].name);
            assert!(!applied_at.is_empty());
        }
    }
}
