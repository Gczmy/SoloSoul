//! Schema migration runner for vault database

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use crate::storage::VaultStore;

pub const CURRENT_SCHEMA_VERSION: u32 = 26;

pub fn get_schema_version(conn: &Connection) -> Result<u32, String> {
    // P032: 区分「首次建库无 data_version 行」与「真实读取错误」——
    // QueryReturnedNoRows（optional() → None）视为版本 1（历史 unwrap_or(1)
    // 的兜底语义）；其余读取/解析错误显式传播，不再静默重跑全部迁移。
    let version: Option<String> = conn
        .query_row(
            "SELECT value FROM sys_config WHERE key = 'data_version'",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| format!("Failed to get schema version: {}", e))?;
    let Some(version) = version else {
        return Ok(1);
    };
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

/// 查询某表是否已存在某列（migration 幂等性样板）。
/// 表名/列名均为调用侧编译期常量，无注入风险。
fn has_column(conn: &Connection, table: &str, column: &str) -> bool {
    let sql = format!("SELECT COUNT(*) FROM pragma_table_info('{table}') WHERE name = '{column}'");
    conn.query_row(&sql, [], |r| r.get::<_, i32>(0))
        .unwrap_or(0)
        > 0
}

/// Run all pending migrations
pub fn run_migrations(conn: &mut Connection) -> Result<(), String> {
    // `current` 在入口读取一次，后续所有版本判断均基于该快照，
    // 与历史实现完全一致（各 migrate_vN 内 set_schema_version 不影响判断）。
    let current = get_schema_version(conn)?;

    migrate_v2(conn, current)?;
    migrate_v3(conn, current)?;
    migrate_v4(conn, current)?;
    migrate_v5(conn, current)?;
    migrate_v6(conn, current)?;
    migrate_v7(conn, current)?;
    migrate_v8(conn, current)?;
    migrate_v9(conn, current)?;
    migrate_v10(conn, current)?;
    migrate_v11(conn, current)?;
    migrate_v12(conn, current)?;
    migrate_v13(conn, current)?;
    migrate_v14(conn, current)?;
    migrate_v15(conn, current)?;
    migrate_v16(conn, current)?;
    migrate_v17(conn, current)?;
    migrate_v18(conn, current)?;
    migrate_v19(conn, current)?;
    migrate_v20(conn, current)?;
    migrate_v21(conn, current)?;
    migrate_v22(conn, current)?;
    migrate_v23(conn, current)?;
    migrate_v24(conn, current)?;
    migrate_v25(conn, current)?;
    migrate_v26(conn, current)?;

    Ok(())
}

fn migrate_v2(conn: &mut Connection, current: u32) -> Result<(), String> {
    if current < 2 {
        apply_migration(
            conn,
            2,
            "ALTER TABLE profiles ADD COLUMN extra_data TEXT;",
            "Add extra_data column",
        )?;
    }
    Ok(())
}

fn migrate_v3(conn: &mut Connection, current: u32) -> Result<(), String> {
    if current < 3 {
        // Ensure metadata table has updated_at column
        if !has_column(conn, "metadata", "updated_at") {
            if !has_column(conn, "metadata", "key") {
                conn.execute("CREATE TABLE metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT)", [])
                    .map_err(|e| format!("Create metadata: {}", e))?;
            } else {
                conn.execute("ALTER TABLE metadata ADD COLUMN updated_at TEXT", [])
                    .map_err(|e| format!("Add updated_at: {}", e))?;
            }
        }
        set_schema_version(conn, 3)?;
    }
    Ok(())
}

fn migrate_v4(conn: &mut Connection, current: u32) -> Result<(), String> {
    if current < 4 {
        if !has_column(conn, "trash_items", "original_section_type") {
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
    Ok(())
}

fn migrate_v5(conn: &mut Connection, current: u32) -> Result<(), String> {
    if current < 5 {
        // Add structured audit log columns (may already exist from init_schema)
        if !has_column(conn, "audit_log", "entity_type") {
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
    Ok(())
}

fn migrate_v6(conn: &mut Connection, current: u32) -> Result<(), String> {
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
    Ok(())
}

fn migrate_v7(conn: &mut Connection, current: u32) -> Result<(), String> {
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
    Ok(())
}

fn migrate_v8(conn: &mut Connection, current: u32) -> Result<(), String> {
    if current < 8 {
        // Idempotent: init_schema for new DBs already includes these columns.
        let has_template_id = has_column(conn, "objects", "template_id");
        let has_template_type = has_column(conn, "objects", "template_type");
        if !has_template_id && !has_template_type {
            apply_migration(
                conn,
                8,
                "ALTER TABLE objects ADD COLUMN template_id TEXT;
                 ALTER TABLE objects ADD COLUMN template_type TEXT CHECK(template_type IN ('system', 'user'));",
                "Add template_id and template_type to objects table",
            )?;
        } else {
            let now = Utc::now().timestamp();
            conn.execute(
                "INSERT OR IGNORE INTO schema_migrations (version, applied_at, description) VALUES (?1, ?2, ?3)",
                params![8, now, "template_id/template_type already present on objects (no-op)"],
            ).ok();
            set_schema_version(conn, 8)?;
        }
    }
    Ok(())
}

fn migrate_v9(conn: &mut Connection, current: u32) -> Result<(), String> {
    if current < 9 {
        apply_migration(
            conn,
            9,
            "ALTER TABLE user_templates ADD COLUMN category TEXT DEFAULT 'identity';",
            "Add category to user_templates table",
        )?;
    }
    Ok(())
}

fn migrate_v10(conn: &mut Connection, current: u32) -> Result<(), String> {
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
    Ok(())
}

fn migrate_v11(conn: &mut Connection, current: u32) -> Result<(), String> {
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
    Ok(())
}

fn migrate_v12(conn: &mut Connection, current: u32) -> Result<(), String> {
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
    Ok(())
}

fn migrate_v13(conn: &mut Connection, current: u32) -> Result<(), String> {
    if current < 13 {
        // §13 — 废弃 SensitivityMap，字段敏感度完全由模板定义
        apply_migration(
            conn,
            13,
            "DROP TABLE IF EXISTS sensitivity_map;",
            "Drop sensitivity_map — sensitivity now defined per-template",
        )?;
    }
    Ok(())
}

fn migrate_v14(conn: &mut Connection, current: u32) -> Result<(), String> {
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
    Ok(())
}

fn migrate_v15(conn: &mut Connection, current: u32) -> Result<(), String> {
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
    Ok(())
}

fn migrate_v16(conn: &mut Connection, current: u32) -> Result<(), String> {
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
    Ok(())
}

fn migrate_v17(conn: &mut Connection, current: u32) -> Result<(), String> {
    if current < 17 {
        // §17 — plugin-template compat: add contract_type_id to objects and user_templates.
        // Use two independent `pragma_table_info` booleans so the upgrade path is idempotent
        // for users with partially-migrated DBs. Each ALTER is only issued for the table
        // that does not yet have the column.
        let mut sql_parts: Vec<&str> = Vec::new();
        if !has_column(conn, "user_templates", "contract_type_id") {
            sql_parts.push("ALTER TABLE user_templates ADD COLUMN contract_type_id TEXT;");
        }
        if !has_column(conn, "objects", "contract_type_id") {
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

fn migrate_v18(conn: &mut Connection, current: u32) -> Result<(), String> {
    if current < 18 {
        if !has_column(conn, "objects", "template_hash") {
            apply_migration(
                conn,
                18,
                "ALTER TABLE objects ADD COLUMN template_hash TEXT;",
                "Add template_hash to objects table for template sync tracking",
            )?;
        } else {
            let now = Utc::now().timestamp();
            conn.execute(
                "INSERT OR IGNORE INTO schema_migrations (version, applied_at, description) VALUES (?1, ?2, ?3)",
                params![18, now, "template_hash already present on objects (no-op)"],
            ).ok();
            set_schema_version(conn, 18)?;
        }
    }
    Ok(())
}

fn migrate_v19(conn: &mut Connection, current: u32) -> Result<(), String> {
    if current < 19 {
        if !has_column(conn, "objects", "ignored_template_hash") {
            apply_migration(
                conn,
                19,
                "ALTER TABLE objects ADD COLUMN ignored_template_hash TEXT;",
                "Add ignored_template_hash to objects table for persistent sync dismissal",
            )?;
        } else {
            let now = Utc::now().timestamp();
            conn.execute(
                "INSERT OR IGNORE INTO schema_migrations (version, applied_at, description) VALUES (?1, ?2, ?3)",
                params![19, now, "ignored_template_hash already present on objects (no-op)"],
            ).ok();
            set_schema_version(conn, 19)?;
        }
    }
    Ok(())
}

fn migrate_v20(conn: &mut Connection, current: u32) -> Result<(), String> {
    if current < 20 {
        apply_migration(
            conn,
            20,
            "CREATE TABLE IF NOT EXISTS sync_conflicts (
                id TEXT PRIMARY KEY,
                table_name TEXT NOT NULL,
                record_id TEXT NOT NULL,
                local_hlc TEXT NOT NULL,
                remote_hlc TEXT NOT NULL,
                remote_data TEXT NOT NULL,
                remote_deleted INTEGER NOT NULL DEFAULT 0,
                winner TEXT NOT NULL,
                created_at TEXT NOT NULL,
                resolved INTEGER NOT NULL DEFAULT 0
            );
            CREATE UNIQUE INDEX IF NOT EXISTS idx_sync_conflicts_record ON sync_conflicts(table_name, record_id);
            CREATE INDEX IF NOT EXISTS idx_sync_conflicts_unresolved ON sync_conflicts(resolved);",
            "Add sync_conflicts table for persistent sync conflict resolution UI",
        )?;
    }
    Ok(())
}

fn migrate_v21(conn: &mut Connection, current: u32) -> Result<(), String> {
    if current < 21 {
        if !has_column(conn, "sync_conflicts", "local_data") {
            apply_migration(
                conn,
                21,
                "ALTER TABLE sync_conflicts ADD COLUMN local_data TEXT NOT NULL DEFAULT '{}';",
                "Add local_data snapshot to sync_conflicts for accurate diff UI",
            )?;
        } else {
            let now = Utc::now().timestamp();
            conn.execute(
                "INSERT OR IGNORE INTO schema_migrations (version, applied_at, description) VALUES (?1, ?2, ?3)",
                params![21, now, "local_data already present on sync_conflicts (no-op)"],
            ).ok();
            set_schema_version(conn, 21)?;
        }
    }
    Ok(())
}

fn migrate_v22(conn: &mut Connection, current: u32) -> Result<(), String> {
    if current < 22 {
        // R-3: sync_watermarks 增加页游标列——会话中断后 keyset 游标随水印持久化，
        // 等值 HLC 组尾部跨会话续传（不再从 NULL 游标重查而跳过组尾行）。
        // 表存在性守卫：部分态迁移 fixture（v17 partial state）可能无 sync 表；
        // 生产路径 storage.rs 的 CREATE TABLE IF NOT EXISTS 已含 cursor_id。
        let has_table = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sync_watermarks'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0;
        if has_table && !has_column(conn, "sync_watermarks", "cursor_id") {
            apply_migration(
                conn,
                22,
                "ALTER TABLE sync_watermarks ADD COLUMN cursor_id TEXT;",
                "R-3: persist pagination cursor alongside peer watermark",
            )?;
        } else {
            let now = Utc::now().timestamp();
            conn.execute(
                "INSERT OR IGNORE INTO schema_migrations (version, applied_at, description) VALUES (?1, ?2, ?3)",
                params![22, now, "cursor_id already present (or sync table absent) on sync_watermarks (no-op)"],
            ).ok();
            set_schema_version(conn, 22)?;
        }
    }
    Ok(())
}

/// v23 — 方案 B 阶段 3（保守退休）：为升级前创建、无 sync_hlc 行的存量行回填 HLC。
///
/// 用户决策（2026-08-04 方案 B）：回填迁移 + 保留兜底。回填必须与回退路径
/// `record_hlc_or_fallback` / keyset SQL 的 IS NULL 分支逐字节一致，否则同步排序
/// 语义改变：
///   - objects：wall = julianday(updated_at)→ms（与 keyset SQL 完全同一表达式）
///   - profiles：wall = julianday(updated_at)→ms（parse_time_ms 逐字节一致，P110 断言）
///   - trash_items：wall = deleted_at（本就是 unix ms）
///   - user_templates：wall = julianday(COALESCE(updated_at,''))→ms（NULL 回退 parse_time_ms("")=0）
///   - counter = 0，node = 规范化本地节点（与 sync 层 hex::encode(Hlc::parse_node_id_bytes) 一致）
///
/// 兜底保留：`record_hlc_or_fallback` 与 keyset IS NULL 分支不删除——未来任何直写 SQL
/// 路径产生的无 HLC 行仍可经回退同步（安全网）。
fn migrate_v23(conn: &mut Connection, current: u32) -> Result<(), String> {
    if current < 23 {
        // 表存在性守卫：部分态迁移 fixture（v17 partial state）可能无 sync 表。
        let has_hlc = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sync_hlc'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0;
        if !has_hlc {
            let now = Utc::now().timestamp();
            conn.execute(
                "INSERT OR IGNORE INTO schema_migrations (version, applied_at, description) VALUES (?1, ?2, ?3)",
                params![23, now, "sync_hlc absent (partial-state fixture) — backfill skipped (no-op)"],
            )
            .ok();
            set_schema_version(conn, 23)?;
            return Ok(());
        }

        // 读本地 sync 节点（metadata 表 base64 明文，无需 data key）。与 local_node_id()
        // 的语义一致：无节点时回退 "unknown"，再经 normalize_sync_node_id 规范化。
        let raw_node = conn
            .query_row(
                "SELECT value FROM metadata WHERE key = 'sync_node_id'",
                [],
                |r| r.get::<_, String>(0),
            )
            .ok()
            .and_then(|b64| {
                use base64::Engine as _;
                base64::engine::general_purpose::STANDARD.decode(&b64).ok()
            })
            .and_then(|v| String::from_utf8(v).ok())
            .unwrap_or_else(|| "unknown".to_string());
        let node = normalize_sync_node_id(&raw_node);
        let now = Utc::now().to_rfc3339();

        // ── 回填主体 ─────────────────────────────────────────────
        // 注意：wall 语义按**各表真实回退路径**逐字节复刻，混用会改变同步排序：
        //   - objects：keyset SQL 回退用 julianday(updated_at)→ms → 回填同表达式；
        //   - trash_items：keyset SQL 回退用 deleted_at 原值 → 回填同列；
        //   - profiles / user_templates：record_hlc_or_fallback 用 Rust parse_time_ms
        //     （chrono RFC3339→ms，julianday 浮点对部分时间戳差 1ms，不可混用）→
        //     Rust 层逐行计算（parse_time_ms 语义：解析失败/空值 → 0）。
        let tx = conn
            .transaction()
            .map_err(|e| format!("Begin tx for v23: {}", e))?;
        let mut insert = tx
            .prepare_cached(
                "INSERT OR IGNORE INTO sync_hlc (table_name, record_id, wall_time_ms, counter, node_id, updated_at) VALUES (?1, ?2, ?3, 0, ?4, ?5)",
            )
            .map_err(|e| format!("Prepare v23 insert: {}", e))?;

        // objects：julianday 表达式与 keyset SQL 回退逐字一致。
        if has_column(&tx, "objects", "updated_at") {
            tx.execute_batch(&format!(
                "INSERT OR IGNORE INTO sync_hlc (table_name, record_id, wall_time_ms, counter, node_id, updated_at)\n\
                 SELECT 'objects', o.id, CAST((julianday(o.updated_at) - 2440587.5) * 86400000.0 AS INTEGER), 0, '{node}', '{now}'\n\
                 FROM objects o LEFT JOIN sync_hlc h ON h.table_name = 'objects' AND h.record_id = o.id\n\
                 WHERE h.record_id IS NULL;"
            ))
            .map_err(|e| format!("v23 objects backfill: {}", e))?;
        }
        // trash_items：wall = deleted_at 原值。
        if has_column(&tx, "trash_items", "deleted_at") {
            tx.execute_batch(&format!(
                "INSERT OR IGNORE INTO sync_hlc (table_name, record_id, wall_time_ms, counter, node_id, updated_at)\n\
                 SELECT 'trash_items', t.id, t.deleted_at, 0, '{node}', '{now}'\n\
                 FROM trash_items t LEFT JOIN sync_hlc h ON h.table_name = 'trash_items' AND h.record_id = t.id\n\
                 WHERE h.record_id IS NULL;"
            ))
            .map_err(|e| format!("v23 trash backfill: {}", e))?;
        }
        // profiles / user_templates：Rust chrono 逐行（与 parse_time_ms 逐字节一致）。
        for (table, has_updated) in [
            ("profiles", has_column(&tx, "profiles", "updated_at")),
            (
                "user_templates",
                has_column(&tx, "user_templates", "updated_at"),
            ),
        ] {
            if !has_updated {
                continue;
            }
            let rows: Vec<(String, Option<String>)> = tx
                .prepare(&format!("SELECT id, updated_at FROM {}", table))
                .map_err(|e| format!("Prepare {} scan: {}", table, e))?
                .query_map([], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
                })
                .map_err(|e| format!("Scan {}: {}", table, e))?
                .collect::<Result<_, _>>()
                .map_err(|e| format!("Collect {}: {}", table, e))?;
            for (id, updated) in rows {
                let wall = updated.as_deref().map(parse_time_ms).unwrap_or(0);
                insert
                    .execute(params![table, id, wall, node, now])
                    .map_err(|e| format!("v23 {} backfill row {}: {}", table, id, e))?;
            }
        }
        drop(insert);
        let now_ts = Utc::now().timestamp();
        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at, description) VALUES (?1, ?2, ?3)",
            params![
                23,
                now_ts,
                "v23: backfill sync_hlc for legacy rows (conservative retirement)"
            ],
        )
        .map_err(|e| format!("Record v23: {}", e))?;
        set_schema_version(&tx, 23)?;
        tx.commit().map_err(|e| format!("Commit v23: {}", e))?;
    }
    Ok(())
}

/// v24 — sync_peers 增加 client_type（客户端类型）与 trusted_at（最近信任时间）列。
///
/// 已知设备卡片展示「客户端类型 + 设备图标」与「最近信任时间」所需（P2 前端设备同步 UI）。
/// 幂等：has_column 守卫，旧库 ALTER、新库（已含列）no-op。
fn migrate_v24(conn: &mut Connection, current: u32) -> Result<(), String> {
    if current < 24 {
        // 表存在性守卫：部分态迁移 fixture（v17 partial state）可能无 sync 表。
        let has_table = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sync_peers'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0;
        if !has_table {
            let now = Utc::now().timestamp();
            conn.execute(
                "INSERT OR IGNORE INTO schema_migrations (version, applied_at, description) VALUES (?1, ?2, ?3)",
                params![24, now, "sync_peers absent (partial-state fixture) — columns skipped (no-op)"],
            )
            .ok();
            set_schema_version(conn, 24)?;
            return Ok(());
        }
        let mut sql_parts: Vec<&str> = Vec::new();
        if !has_column(conn, "sync_peers", "client_type") {
            sql_parts.push("ALTER TABLE sync_peers ADD COLUMN client_type TEXT;");
        }
        if !has_column(conn, "sync_peers", "trusted_at") {
            sql_parts.push("ALTER TABLE sync_peers ADD COLUMN trusted_at INTEGER;");
        }
        let tx = conn
            .transaction()
            .map_err(|e| format!("Begin tx for v24: {}", e))?;
        let now = Utc::now().timestamp();
        if sql_parts.is_empty() {
            tx.execute(
                "INSERT OR IGNORE INTO schema_migrations (version, applied_at, description) VALUES (?1, ?2, ?3)",
                params![24, now, "client_type/trusted_at already present on sync_peers (no-op)"],
            )
            .map_err(|e| format!("Record v24 (no-op): {}", e))?;
        } else {
            let combined = sql_parts.join("\n");
            tx.execute_batch(&combined)
                .map_err(|e| format!("v24 ALTER failed: {}", e))?;
            tx.execute(
                "INSERT INTO schema_migrations (version, applied_at, description) VALUES (?1, ?2, ?3)",
                params![24, now, "Add client_type and trusted_at to sync_peers"],
            )
            .map_err(|e| format!("Record v24: {}", e))?;
        }
        set_schema_version(&tx, 24)?;
        tx.commit().map_err(|e| format!("Commit v24: {}", e))?;
    }
    Ok(())
}

/// v25 — sync_peers 增加 last_addr（最近一次成功同步的连接地址）列。
///
/// P1#7/#8：在线状态心跳化——成功同步即证明 LAN 可达，即使 mDNS 发现链中断，
/// known_peers 也可凭「最近 5 分钟内的 last_seen + last_addr」显示在线。
/// 幂等：has_column 守卫，旧库 ALTER、无表/已含列 no-op。
fn migrate_v25(conn: &mut Connection, current: u32) -> Result<(), String> {
    if current < 25 {
        // 表存在性守卫：部分态迁移 fixture（v17 partial state）可能无 sync 表。
        let has_table = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sync_peers'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0;
        if !has_table {
            let now = Utc::now().timestamp();
            conn.execute(
                "INSERT OR IGNORE INTO schema_migrations (version, applied_at, description) VALUES (?1, ?2, ?3)",
                params![25, now, "sync_peers absent (partial-state fixture) — last_addr skipped (no-op)"],
            )
            .ok();
            set_schema_version(conn, 25)?;
            return Ok(());
        }
        if has_column(conn, "sync_peers", "last_addr") {
            let now = Utc::now().timestamp();
            conn.execute(
                "INSERT OR IGNORE INTO schema_migrations (version, applied_at, description) VALUES (?1, ?2, ?3)",
                params![25, now, "last_addr already present on sync_peers (no-op)"],
            )
            .ok();
            set_schema_version(conn, 25)?;
        } else {
            apply_migration(
                conn,
                25,
                "ALTER TABLE sync_peers ADD COLUMN last_addr TEXT;",
                "Add last_addr to sync_peers (peer online-state heartbeat)",
            )?;
        }
    }
    Ok(())
}

/// v26 — 新增 `llm_conversations` 表（P004：LLM 会话从 profile preferences blob 迁出，
/// 按 conversation_id 行存储，避免每次保存整 blob 解密+深克隆+序列化+加密+写盘）。
///
/// 表结构：data 列为加密 JSON（与 profiles.data 同款 AES-256-GCM），updated_at 明文
/// 供排序与 HLC 回退。幂等：CREATE TABLE IF NOT EXISTS + 版本记录 INSERT OR IGNORE
/// （与 v24/v25 守卫模式一致，兼容 v23 测试降级重跑场景）。
fn migrate_v26(conn: &mut Connection, current: u32) -> Result<(), String> {
    if current < 26 {
        let has_table = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='llm_conversations'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0;
        if has_table {
            let now = Utc::now().timestamp();
            conn.execute(
                "INSERT OR IGNORE INTO schema_migrations (version, applied_at, description) VALUES (?1, ?2, ?3)",
                params![26, now, "llm_conversations already present (no-op)"],
            )
            .ok();
            set_schema_version(conn, 26)?;
        } else {
            apply_migration(
                conn,
                26,
                "CREATE TABLE IF NOT EXISTS llm_conversations (
                    id TEXT PRIMARY KEY,
                    account_id TEXT NOT NULL,
                    data BLOB NOT NULL,
                    updated_at TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_llm_conversations_account ON llm_conversations(account_id);",
                "Add llm_conversations table for per-row conversation storage (P004)",
            )?;
        }
    }
    Ok(())
}

/// 复用 storage/sync_meta.rs 的 `VaultStore::parse_time_ms`（pub(crate)，同一 crate 无循环依赖）。
fn parse_time_ms(s: &str) -> u64 {
    VaultStore::parse_time_ms(s)
}

/// 复用 storage/sync_meta.rs 的 `VaultStore::normalize_sync_node_id`（pub(crate)，同一 crate）。
fn normalize_sync_node_id(node_id: &str) -> String {
    VaultStore::normalize_sync_node_id(node_id)
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

    // ── 方案 B 阶段 3 — v23 存量 HLC 回填 ───────────────────────
    //
    // 保守退休（2026-08-04 用户决策 B）：升级前创建、无 sync_hlc 行的存量行
    // 按回退路径语义回填 HLC，回退代码保留作兜底。回填 wall/counter/node 必须
    // 与 record_hlc_or_fallback / keyset IS NULL 分支逐字节一致。

    /// 升级前遗留行回填测试：raw INSERT（不写 sync_hlc）模拟旧库数据，
    /// 降级到 v22 后重跑迁移，断言四表行均被回填且 wall 与回退语义一致。
    #[test]
    fn test_migration_v23_backfills_hlc_for_legacy_rows() {
        let (mut conn, _dir) = setup_conn();
        run_migrations(&mut conn).unwrap();
        assert_eq!(get_schema_version(&conn).unwrap(), CURRENT_SCHEMA_VERSION);

        // 迁移 fixture 的 profiles 为 v1 样式（无 updated_at），补齐列以覆盖回填。
        // execute_batch：多语句 ALTER 需逐条执行（execute 仅编译第一条）。
        conn.execute_batch(
            "ALTER TABLE profiles ADD COLUMN created_at TEXT;
             ALTER TABLE profiles ADD COLUMN updated_at TEXT;
             ALTER TABLE profiles ADD COLUMN version INTEGER DEFAULT 1;",
        )
        .unwrap();

        // ── 遗留行（raw INSERT，无 sync_hlc）──
        let obj_updated = "2024-01-15T00:00:00Z";
        conn.execute(
            "INSERT INTO objects (id, account_id, name, created_at, updated_at) VALUES ('o1', 'acc1', 'ObjA', '2024-01-01T00:00:00Z', ?1)",
            params![obj_updated],
        )
        .unwrap();
        let profile_updated = "2024-02-02T03:04:05Z";
        conn.execute(
            "INSERT INTO profiles (id, name, data, created_at, updated_at, version) VALUES ('p1', 'ProfA', X'0102', '2024-01-01T00:00:00Z', ?1, 1)",
            params![profile_updated],
        )
        .unwrap();
        // 已有 HLC 的行必须保持不动（LEFT JOIN + INSERT OR IGNORE 双保险）。
        conn.execute(
            "INSERT INTO objects (id, account_id, name, created_at, updated_at) VALUES ('o2', 'acc1', 'ObjB', '2024-01-01T00:00:00Z', '2024-03-03T00:00:00Z')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO sync_hlc (table_name, record_id, wall_time_ms, counter, node_id, updated_at) VALUES ('objects', 'o2', 1709424000000, 7, 'aabbccdd', '2024-03-03T00:00:00Z')",
            [],
        )
        .unwrap();
        let trash_deleted = 1704067200i64 * 1000; // 2024-01-01T00:00:00Z ms
        conn.execute(
            "INSERT INTO trash_items (id, item_type, original_id, data, deleted_at, deleted_by, name_snapshot) VALUES ('t1', 'object', 'o9', X'00', ?1, 'user', 'TrashA')",
            params![trash_deleted],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO user_templates (id, account_id, name, icon_id, properties_json, created_at, updated_at, category) VALUES ('u1', 'acc1', 'TplA', 'doc', '[]', '2024-01-01T00:00:00Z', NULL, 'identity')",
            [],
        )
        .unwrap();

        // ── 模拟升级前状态：降级到 v22，重跑迁移只触发 v23 ──
        // （先清掉首次 run_migrations 已记录的 v23 行，避免 apply_migration UNIQUE 冲突；
        //   生产路径每次启动只跑一次，无此场景。）
        conn.execute("DELETE FROM schema_migrations WHERE version = 23", [])
            .unwrap();
        set_schema_version(&conn, 22).unwrap();
        run_migrations(&mut conn).unwrap();
        assert_eq!(get_schema_version(&conn).unwrap(), CURRENT_SCHEMA_VERSION);

        // ── 断言：四表遗留行全部回填 ──
        let expected_obj_wall = chrono::DateTime::parse_from_rfc3339(obj_updated)
            .unwrap()
            .timestamp_millis();
        let expected_profile_wall = chrono::DateTime::parse_from_rfc3339(profile_updated)
            .unwrap()
            .timestamp_millis();
        let node = normalize_sync_node_id("unknown"); // 无 sync_node_id → "unknown" 规范化

        let (wall, counter, node_id): (i64, i64, String) = conn
            .query_row(
                "SELECT wall_time_ms, counter, node_id FROM sync_hlc WHERE table_name='objects' AND record_id='o1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(wall, expected_obj_wall, "objects 回退 wall 必须逐字节一致");
        assert_eq!(counter, 0);
        assert_eq!(node_id, node);

        let (wall, counter): (i64, i64) = conn
            .query_row(
                "SELECT wall_time_ms, counter FROM sync_hlc WHERE table_name='profiles' AND record_id='p1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            wall, expected_profile_wall,
            "profiles 回退 wall 必须逐字节一致"
        );
        assert_eq!(counter, 0);

        let (wall, counter): (i64, i64) = conn
            .query_row(
                "SELECT wall_time_ms, counter FROM sync_hlc WHERE table_name='trash_items' AND record_id='t1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(wall, trash_deleted, "trash_items wall 取 deleted_at 原值");
        assert_eq!(counter, 0);

        let (wall, counter): (i64, i64) = conn
            .query_row(
                "SELECT wall_time_ms, counter FROM sync_hlc WHERE table_name='user_templates' AND record_id='u1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            wall, 0,
            "user_templates 空 updated_at 回退 parse_time_ms(\"\")=0"
        );
        assert_eq!(counter, 0);

        // 已有 HLC 的行不被覆盖。
        let (wall, counter): (i64, i64) = conn
            .query_row(
                "SELECT wall_time_ms, counter FROM sync_hlc WHERE table_name='objects' AND record_id='o2'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(wall, 1709424000000, "已有 HLC 的行必须保持不动");
        assert_eq!(counter, 7);

        // 幂等：重复执行 run_migrations 不再产生重复回填（version 已到位，migrate_v23 跳过）。
        run_migrations(&mut conn).unwrap();
        let hlc_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sync_hlc", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            hlc_count, 5,
            "objects o1/o2 + profiles + trash + template = 5 行，重复迁移不得新增"
        );
    }

    /// 已配置 sync 节点的库：回填 node 必须取规范化本地节点（而非 "unknown"）。
    #[test]
    fn test_migration_v23_backfill_uses_stored_sync_node() {
        let (mut conn, _dir) = setup_conn();
        run_migrations(&mut conn).unwrap();
        conn.execute_batch(
            "ALTER TABLE profiles ADD COLUMN created_at TEXT;
             ALTER TABLE profiles ADD COLUMN updated_at TEXT;
             ALTER TABLE profiles ADD COLUMN version INTEGER DEFAULT 1;",
        )
        .unwrap();

        // metadata 存 base64 明文 sync_node_id（与 read_metadata/write_metadata 一致）。
        use base64::Engine as _;
        let raw_node = "node_a1b2c3d4e5f60718293a4b5c6d7e8f90";
        let b64 = base64::engine::general_purpose::STANDARD.encode(raw_node.as_bytes());
        conn.execute(
            "INSERT INTO metadata (key, value, updated_at) VALUES ('sync_node_id', ?1, '2024-01-01T00:00:00Z')",
            params![b64],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO objects (id, account_id, name, created_at, updated_at) VALUES ('o1', 'acc1', 'ObjA', '2024-01-01T00:00:00Z', '2024-01-15T00:00:00Z')",
            [],
        )
        .unwrap();

        conn.execute("DELETE FROM schema_migrations WHERE version = 23", [])
            .unwrap();
        set_schema_version(&conn, 22).unwrap();
        run_migrations(&mut conn).unwrap();

        let expected_node = normalize_sync_node_id(raw_node);
        let (node_id, wall): (String, i64) = conn
            .query_row(
                "SELECT node_id, wall_time_ms FROM sync_hlc WHERE table_name='objects' AND record_id='o1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(node_id, expected_node, "回填 node 必须取规范化本地节点");
        assert!(wall > 0);
    }

    /// v24：旧库 sync_peers 升级后自动获得 client_type / trusted_at 列，
    /// 新库（建表已含列）与部分态 fixture（无 sync 表）幂等 no-op。
    #[test]
    fn test_migration_v24_adds_peer_metadata_columns() {
        // ── 旧库路径：v15 风格 sync_peers（无 client_type/trusted_at）──
        let (mut conn, _dir) = setup_conn();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sync_peers (
                peer_node_id TEXT PRIMARY KEY,
                peer_name TEXT,
                trusted INTEGER NOT NULL DEFAULT 0,
                public_key_fingerprint TEXT,
                last_seen INTEGER,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
             );\n",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO sync_peers (peer_node_id, peer_name, trusted, created_at, updated_at) VALUES ('node_abc', 'Old Device', 1, '2024-01-01T00:00:00Z', '2024-01-01T00:00:00Z')",
            [],
        )
        .unwrap();

        // 跳过 v2-v23 直接到 v23 快照，仅触发 v24。
        conn.execute("DELETE FROM schema_migrations", []).unwrap();
        set_schema_version(&conn, 23).unwrap();
        run_migrations(&mut conn).unwrap();
        assert_eq!(get_schema_version(&conn).unwrap(), CURRENT_SCHEMA_VERSION);

        for col in &["client_type", "trusted_at"] {
            let sql = format!(
                "SELECT COUNT(*) FROM pragma_table_info('sync_peers') WHERE name = '{}'",
                col
            );
            let present: i64 = conn.query_row(&sql, [], |r| r.get(0)).unwrap();
            assert_eq!(present, 1, "sync_peers.{col} must exist after v24");
        }
        // 存量行保留，新列为 NULL（未知客户端/从未信任）。
        let (client_type, trusted_at): (Option<String>, Option<i64>) = conn
            .query_row(
                "SELECT client_type, trusted_at FROM sync_peers WHERE peer_node_id = 'node_abc'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert!(client_type.is_none());
        assert!(trusted_at.is_none());

        // 幂等：再次运行不新增 v24 记录行。
        run_migrations(&mut conn).unwrap();
        let v24_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = 24",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(v24_rows, 1, "second run must not duplicate v24 row");
    }

    /// v24 部分态 fixture（无 sync_peers 表）：迁移需跳过 ALTER（no-op）而非报错。
    #[test]
    fn test_migration_v24_skips_when_sync_peers_absent() {
        let (mut conn, _dir) = setup_conn();
        // setup_conn 不含 sync 表；跳过前序迁移直接触发 v24。
        conn.execute("DELETE FROM schema_migrations", []).unwrap();
        set_schema_version(&conn, 23).unwrap();
        run_migrations(&mut conn).unwrap();
        assert_eq!(get_schema_version(&conn).unwrap(), CURRENT_SCHEMA_VERSION);
        let v24_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = 24",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(v24_rows, 1);
    }

    /// v26：旧库升级后获得 llm_conversations 表与账号索引；新库幂等。
    #[test]
    fn test_migration_v26_creates_conversations_table() {
        let (mut conn, _dir) = setup_conn();
        conn.execute("DELETE FROM schema_migrations", []).unwrap();
        set_schema_version(&conn, 25).unwrap();
        run_migrations(&mut conn).unwrap();
        assert_eq!(get_schema_version(&conn).unwrap(), CURRENT_SCHEMA_VERSION);

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='llm_conversations'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "llm_conversations 表必须存在");

        let idx: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_llm_conversations_account'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(idx, 1, "账号索引必须存在");

        // 可写可读
        conn.execute(
            "INSERT INTO llm_conversations (id, account_id, data, updated_at) VALUES ('c1', 'acc1', X'01', '2024-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        let id: String = conn
            .query_row(
                "SELECT id FROM llm_conversations WHERE id = 'c1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(id, "c1");

        // 幂等：重复迁移不重复建表/索引
        run_migrations(&mut conn).unwrap();
        let v26_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = 26",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(v26_rows, 1);
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
