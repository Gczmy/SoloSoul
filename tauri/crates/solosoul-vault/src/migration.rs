//! Schema migration runner for vault database

use chrono::Utc;
use rusqlite::{params, Connection};

pub const CURRENT_SCHEMA_VERSION: u32 = 17;

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
        let has_section_type: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('trash_items') WHERE name = 'original_section_type'",
                [],
                |r| r.get::<_, i32>(0),
            )
            .unwrap_or(0)
            > 0;
        if !has_section_type {
            apply_migration(
                conn,
                4,
                "ALTER TABLE trash_items ADD COLUMN original_section_type TEXT;",
                "Add original_section_type to trash_items",
            )?;
        } else {
            let now = Utc::now().timestamp();
            conn.execute(
                "INSERT INTO schema_migrations (version, applied_at, description) VALUES (?1, ?2, ?3)",
                params![4, now, "Add original_section_type to trash_items (already present)"],
            ).ok();
            set_schema_version(conn, 4)?;
        }
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
                contract_type_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_user_templates_account ON user_templates(account_id);",
            "Add user_templates table for custom object templates",
        )?;
    }
    if current < 8 {
        apply_migration(
            conn,
            8,
            "ALTER TABLE objects ADD COLUMN template_id TEXT;
             ALTER TABLE objects ADD COLUMN template_type TEXT CHECK(template_type IN ('system', 'user'));",
            "Add template_id and template_type to objects table",
        )?;
    }
    if current < 9 {
        apply_migration(
            conn,
            9,
            "ALTER TABLE user_templates ADD COLUMN category TEXT DEFAULT 'identity';",
            "Add category to user_templates table",
        )?;
    }
    if current < 10 {
        apply_migration(
            conn,
            10,
            "CREATE TABLE IF NOT EXISTS sensitivity_map (
                field_id       TEXT PRIMARY KEY,
                level          TEXT NOT NULL,
                template_id    TEXT,
                last_modified  TEXT NOT NULL
            );",
            "Add sensitivity_map table for field-level sensitivity persistence",
        )?;
    }
    if current < 11 {
        apply_migration(
            conn,
            11,
            "CREATE TABLE trash_items_new (
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
             INSERT INTO trash_items_new (
                id, item_type, original_id, original_parent_id, original_section_type,
                original_sort_order, data, deleted_at, expires_at, deleted_by,
                name_snapshot, icon_snapshot
             ) SELECT
                id, item_type, original_id, original_parent_id, original_section_type,
                original_sort_order, data, deleted_at, expires_at, deleted_by,
                name_snapshot, icon_snapshot
             FROM trash_items;
             DROP TABLE trash_items;
             ALTER TABLE trash_items_new RENAME TO trash_items;
             CREATE INDEX idx_trash_expires ON trash_items(expires_at);
             CREATE INDEX idx_trash_deleted_at ON trash_items(deleted_at);
             CREATE INDEX idx_trash_type ON trash_items(item_type);",
            "Recreate trash_items without restrictive CHECK constraint",
        )?;
    }
    if current < 12 {
        // §12 — 彻底重建 trash_items，丢弃全部旧数据（软件尚未分发，旧数据无保留价值）
        apply_migration(
            conn,
            12,
            "DROP TABLE IF EXISTS trash_items;
             CREATE TABLE trash_items (
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
             CREATE INDEX idx_trash_expires ON trash_items(expires_at);
             CREATE INDEX idx_trash_deleted_at ON trash_items(deleted_at);
             CREATE INDEX idx_trash_type ON trash_items(item_type);",
            "Rebuild trash_items from scratch — discard all legacy trash data",
        )?;
    }
    if current < 13 {
        // §13 — 废弃 SensitivityMap，字段敏感度完全由模板定义
        apply_migration(
            conn,
            13,
            "DROP TABLE IF EXISTS sensitivity_map;",
            "Drop sensitivity_map — sensitivity now defined per-template",
        )?;
    }
    if current < 14 {
        // §14 — TemplateProperty 支持 deprecated_at 字段（properties_json 是自由 JSON，无需表结构变更）
        let now = Utc::now().timestamp();
        conn.execute(
            "INSERT INTO schema_migrations (version, applied_at, description) VALUES (?1, ?2, ?3)",
            params![
                14,
                now,
                "Add deprecatedAt support to TemplateProperty (properties_json is free-form JSON)"
            ],
        )
        .ok();
        set_schema_version(conn, 14)?;
    }
    if current < 15 {
        apply_migration(
            conn,
            15,
            "CREATE TABLE IF NOT EXISTS sync_peers (
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
             );",
            "Add sync peer, watermark and HLC tables",
        )?;
    }
    if current < 16 {
        apply_migration(
            conn,
            16,
            "CREATE TABLE IF NOT EXISTS sync_tombstones (
                table_name TEXT NOT NULL,
                record_id TEXT NOT NULL,
                wall_time_ms INTEGER NOT NULL,
                counter INTEGER NOT NULL,
                node_id TEXT NOT NULL,
                deleted_by_node_id TEXT NOT NULL,
                created_at TEXT NOT NULL,
                PRIMARY KEY (table_name, record_id)
             );",
            "Add sync tombstones table",
        )?;
    }
    if current < 17 {
        // §17 — plugin-template compat: add contract_type_id to objects and user_templates.
        // Use two independent `pragma_table_info` booleans so the upgrade path is idempotent
        // for users with partially-migrated DBs. Each ALTER is only issued for the table
        // that does not yet have the column.
        let mut sql_parts: Vec<&str> = Vec::new();
        let has_utpl_ctid: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('user_templates') WHERE name = 'contract_type_id'",
                [],
                |r| r.get::<_, i32>(0),
            )
            .unwrap_or(0)
            > 0;
        if !has_utpl_ctid {
            sql_parts.push("ALTER TABLE user_templates ADD COLUMN contract_type_id TEXT;");
        }
        let has_objects_ctid: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('objects') WHERE name = 'contract_type_id'",
                [],
                |r| r.get::<_, i32>(0),
            )
            .unwrap_or(0)
            > 0;
        if !has_objects_ctid {
            sql_parts.push("ALTER TABLE objects ADD COLUMN contract_type_id TEXT;");
        }
        let tx = conn
            .transaction()
            .map_err(|e| format!("Begin tx for v17: {}", e))?;
        let now = Utc::now().timestamp();
        if sql_parts.is_empty() {
            tx.execute(
                "INSERT OR IGNORE INTO schema_migrations (version, applied_at, description) VALUES (?1, ?2, ?3)",
                params![17, now, "contract_type_id already present on objects/user_templates (no-op)"],
            )
            .map_err(|e| format!("Record v17 (no-op): {}", e))?;
        } else {
            let combined = sql_parts.join("\n");
            tx.execute_batch(&combined)
                .map_err(|e| format!("v17 ALTER failed: {}", e))?;
            tx.execute(
                "INSERT INTO schema_migrations (version, applied_at, description) VALUES (?1, ?2, ?3)",
                params![17, now, "Add contract_type_id to objects and user_templates (plugin-template compat)"],
            )
            .map_err(|e| format!("Record v17: {}", e))?;
        }
        set_schema_version(&tx, 17)?;
        tx.commit().map_err(|e| format!("Commit v17: {}", e))?;
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

    /// Reusable v16-ish DDL template; `contract_type_id TEXT,` placeholder
    /// slots are conditionally filled by `setup_v16_partial_state` to build
    /// each of the 4 partial-state facets.
    ///
    /// Two comment lines (`/*UTPL_CTID*/`, `/*OBJECTS_CTID*/`) mark where the
    /// `contract_type_id TEXT,` line is conditionally inserted.
    const HELPERS_PARTIAL_V16_SQL: &str = r#"CREATE TABLE IF NOT EXISTS sys_config (
    key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL, description TEXT
);
CREATE TABLE IF NOT EXISTS user_templates (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    name TEXT NOT NULL,
    icon_id TEXT,
    properties_json TEXT NOT NULL,
/*UTPL_CTID*/    created_at TEXT NOT NULL,
    updated_at TEXT,
    category TEXT DEFAULT 'identity'
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
/*OBJECTS_CTID*/    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER DEFAULT 1
);
"#;

    fn setup_conn() -> (Connection, TempDir) {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sys_config (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL, description TEXT);
             CREATE TABLE IF NOT EXISTS profiles (id TEXT PRIMARY KEY, name TEXT NOT NULL, data BLOB NOT NULL);
             CREATE TABLE IF NOT EXISTS trash_items (
                id TEXT PRIMARY KEY,
                item_type TEXT NOT NULL,
                original_id TEXT NOT NULL,
                original_parent_id TEXT,
                original_sort_order INTEGER,
                data BLOB NOT NULL,
                deleted_at INTEGER NOT NULL,
                expires_at INTEGER,
                deleted_by TEXT DEFAULT 'user',
                name_snapshot TEXT NOT NULL,
                icon_snapshot TEXT
             );
             CREATE TABLE IF NOT EXISTS audit_log (id INTEGER PRIMARY KEY AUTOINCREMENT, timestamp TEXT NOT NULL, action TEXT NOT NULL, details TEXT);
             CREATE TABLE IF NOT EXISTS guide_embeddings (id TEXT PRIMARY KEY, guide_id TEXT NOT NULL, chunk_index INTEGER NOT NULL, chunk_text TEXT NOT NULL, embedding BLOB NOT NULL, model TEXT NOT NULL, created_at TEXT NOT NULL);
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
            CREATE INDEX IF NOT EXISTS idx_objects_account ON objects(account_id);"
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
            .query_row("SELECT name FROM user_templates WHERE id = 't1'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(name, "Test");
    }

    #[test]
    fn test_migration_v11_and_v12_rebuild_trash_items() {
        let (mut conn, _dir) = setup_conn();
        // Insert old-format trash items (v1 schema, no original_section_type yet)
        conn.execute(
            "INSERT INTO trash_items (id, item_type, original_id, original_parent_id, original_sort_order, data, deleted_at, expires_at, deleted_by, name_snapshot, icon_snapshot)
             VALUES ('t1', 'page', 'orig1', 'parent1', 1, X'0102', 1000, 2000, 'user', 'Page A', NULL)",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO trash_items (id, item_type, original_id, original_parent_id, original_sort_order, data, deleted_at, expires_at, deleted_by, name_snapshot, icon_snapshot)
             VALUES ('t2', 'object', 'orig2', NULL, NULL, X'0304', 3000, 4000, 'user', 'Obj B', 'icon2')",
            [],
        ).unwrap();

        run_migrations(&mut conn).unwrap();
        assert_eq!(get_schema_version(&conn).unwrap(), CURRENT_SCHEMA_VERSION);

        // v12 discards all legacy trash data — clean slate
        let count_after_v12: i64 = conn
            .query_row("SELECT COUNT(*) FROM trash_items", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count_after_v12, 0);

        // Verify new table accepts 'template' without CHECK constraint
        conn.execute(
            "INSERT INTO trash_items (id, item_type, original_id, data, deleted_at, deleted_by, name_snapshot)
             VALUES ('t3', 'template', 'tpl1', X'00', 5000, 'user', 'Template C')",
            [],
        ).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM trash_items WHERE item_type = 'template'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    // ── §30 plugin-template Stage 3 — v17 idempotency + partial-state ───
    //
    // Stage 3 backfills the missing acceptance tests for the v17 idempotent
    // ALTER block. These tests guarantee Stage 1+2 will not silently regress
    // if a future migration accidentally lets v17 re-execute on every
    // VaultStore::open() and re-emit duplicate schema_migrations rows.

    /// Build a v16-ish connection with independent control over whether
    /// `objects` and `user_templates` already carry `contract_type_id`.
    fn setup_v16_partial_state(
        has_utpl_ctid: bool,
        has_objects_ctid: bool,
    ) -> (Connection, TempDir) {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test_v17.db");
        let conn = Connection::open(&db_path).unwrap();
        let sql = HELPERS_PARTIAL_V16_SQL
            .replace(
                "/*UTPL_CTID*/",
                if has_utpl_ctid {
                    "    contract_type_id TEXT,\n"
                } else {
                    ""
                },
            )
            .replace(
                "/*OBJECTS_CTID*/",
                if has_objects_ctid {
                    "    contract_type_id TEXT,\n"
                } else {
                    ""
                },
            );
        conn.execute_batch(&sql).unwrap();
        set_schema_version(&conn, 16).unwrap();
        (conn, dir)
    }

    #[test]
    fn test_migration_v17_idempotent_run_twice() {
        let (mut conn, _dir) = setup_conn();
        run_migrations(&mut conn).unwrap();
        assert_eq!(get_schema_version(&conn).unwrap(), CURRENT_SCHEMA_VERSION);

        let v17_rows_1: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = 17",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(v17_rows_1, 1, "first run must record exactly one v17 row");

        for tbl in &["objects", "user_templates"] {
            let sql = format!(
                r#"SELECT "notnull", ((dflt_value IS NULL) OR (dflt_value = '')) FROM pragma_table_info('{}') WHERE name = 'contract_type_id'"#,
                tbl
            );
            let (notnull, dflt_null): (i64, i64) = conn
                .query_row(&sql, [], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))
                .unwrap();
            assert_eq!(notnull, 0, "{}.contract_type_id must be nullable", tbl);
            assert_eq!(
                dflt_null, 1,
                "{}.contract_type_id must have NULL-or-empty default (Option B contract)",
                tbl
            );
        }

        run_migrations(&mut conn).unwrap();
        assert_eq!(get_schema_version(&conn).unwrap(), CURRENT_SCHEMA_VERSION);
        let v17_rows_2: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = 17",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            v17_rows_2, 1,
            "second run_migrations MUST NOT add a duplicate v17 schema_migrations row (got {})",
            v17_rows_2
        );
    }

    #[test]
    fn test_migration_v17_partial_state() {
        let facets: &[(&str, bool, bool)] = &[
            ("both missing (fresh install)", false, false),
            ("user_templates has, objects missing", true, false),
            ("user_templates missing, objects has", false, true),
            ("both columns already present", true, true),
        ];
        for (label, has_utpl, has_objects) in facets.iter() {
            let (mut conn, _dir) = setup_v16_partial_state(*has_utpl, *has_objects);
            assert_eq!(
                get_schema_version(&conn).unwrap(),
                16,
                "facet `{}`: helper must leave conn at v16 before run_migrations",
                label
            );

            run_migrations(&mut conn).unwrap();
            assert_eq!(
                get_schema_version(&conn).unwrap(),
                CURRENT_SCHEMA_VERSION,
                "facet `{}`: run_migrations must end at v17",
                label
            );

            for tbl in &["objects", "user_templates"] {
                let sql = format!(
                    "SELECT COUNT(*) FROM pragma_table_info('{}') WHERE name = 'contract_type_id'",
                    tbl
                );
                let present: i64 = conn.query_row(&sql, [], |r| r.get(0)).unwrap();
                assert_eq!(
                    present, 1,
                    "facet `{}`: {}.contract_type_id must exist after v17",
                    label, tbl
                );
            }

            let v17_rows: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM schema_migrations WHERE version = 17",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(
                v17_rows, 1,
                "facet `{}`: schema_migrations must have exactly one v17 row (got {})",
                label, v17_rows
            );
        }
    }
}
