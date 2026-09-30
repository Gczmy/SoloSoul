//! RF-021 兼容验收：只经真实 ImportJob / 加密 ZIP / VaultService / SQLite。
//! 不引用新增 batch 实现；可独立注册在旧 Host 中跑红，后续保留。
use super::super::import::{run_import_job, ImportJob};
use super::rf020::{objects, package, Fixture};
use super::*;
use rusqlite::types::ValueRef;
use std::path::{Path, PathBuf};

const CORRUPT_ROWS: [(&str, &str); 4] = [
    ("name", "x'80'"),
    ("id", "x'80'"),
    ("is_deleted", "x'80'"),
    ("name", "CAST(x'80' AS TEXT)"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
enum RawCell {
    Null,
    Integer(i64),
    RealBits(u64),
    Text(Vec<u8>),
    Blob(Vec<u8>),
}

type RawRows = Vec<Vec<RawCell>>;

// get_ref 保留真实 SQLite 类型及原字节：非法 UTF-8 TEXT 也不转成 Rust String。
fn raw_table(db: &rusqlite::Connection, table: &str) -> RawRows {
    assert!(["objects", "user_templates", "object_snapshots", "sync_hlc"].contains(&table));
    let mut stmt = db
        .prepare(&format!("SELECT * FROM {table} ORDER BY 1,2"))
        .unwrap();
    let columns = stmt.column_count();
    let mut rows = stmt.query([]).unwrap();
    let mut out = Vec::new();
    while let Some(row) = rows.next().unwrap() {
        let cells = (0..columns)
            .map(|column| match row.get_ref(column).unwrap() {
                ValueRef::Null => RawCell::Null,
                ValueRef::Integer(value) => RawCell::Integer(value),
                ValueRef::Real(value) => RawCell::RealBits(value.to_bits()),
                ValueRef::Text(value) => RawCell::Text(value.to_vec()),
                ValueRef::Blob(value) => RawCell::Blob(value.to_vec()),
            })
            .collect();
        out.push(cells);
    }
    out
}

fn raw_state(f: &Fixture) -> Vec<RawRows> {
    ["objects", "user_templates", "object_snapshots", "sync_hlc"]
        .into_iter()
        .map(|table| raw_table(&f.db, table))
        .collect()
}

fn local(f: &Fixture, id: &str, name: &str) {
    f.vault
        .save_object(&ObjectRecord {
            id: id.into(),
            account_id: f.account.clone(),
            name: name.into(),
            type_id: "note".into(),
            section_type: "identity".into(),
            properties: json!({"local": name}),
            created_at: "2026-09-30T00:00:00Z".into(),
            updated_at: "2026-09-30T00:00:00Z".into(),
            ..Default::default()
        })
        .unwrap();
}

fn corrupt(f: &Fixture, id: &str, column: &str, literal: &str, deleted: bool) {
    assert!(CORRUPT_ROWS.contains(&(column, literal)));
    f.db.execute(
        &format!(
            "UPDATE objects SET is_deleted={}, {column}={literal} WHERE id=?1",
            u8::from(deleted)
        ),
        [id],
    )
    .unwrap();
}

fn clean_objects() -> serde_json::Value {
    let mut payload = objects();
    for (ordinal, value) in payload["objects"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        value["properties"] = json!({"ordinal": ordinal});
    }
    payload
}

fn request(path: &Path, strategy: ImportStrategy) -> AdvancedImportRequest {
    AdvancedImportRequest {
        selections: None,
        strategy,
        source_path: path.to_str().unwrap().into(),
        password: "export-password".into(),
        selected_attachment_ids: Some(vec![]),
        object_strategies: HashMap::new(),
        locale: "en-US".into(),
    }
}

fn select_only(req: &mut AdvancedImportRequest, id: &str) {
    req.selections = Some(vec![
        ImportSelection {
            object_id: id.into(),
            selected: true,
        },
        ImportSelection {
            object_id: "rf020-1".into(),
            selected: false,
        },
    ]);
}

fn run(f: &Fixture, req: AdvancedImportRequest) -> ImportResult {
    let path = PathBuf::from(&req.source_path);
    let source_bytes = std::fs::read(&path).unwrap();
    let job = ImportJob::prepare(f.service.clone(), &f.account, req, None, |path| {
        Path::new(path)
            .canonicalize()
            .map_err(|error| error.to_string())
    })
    .unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let result = runtime
        .block_on(run_import_job(job, ImportJob::run, || {}))
        .unwrap();
    assert_eq!(std::fs::read(path).unwrap(), source_bytes);
    result
}

fn assert_complete_counts(
    result: &ImportResult,
    objects: usize,
    templates: usize,
    snapshots: usize,
) {
    assert!(result.is_complete(), "{result:?}");
    assert_eq!(
        (
            result.object_count,
            result.template_count,
            result.snapshot_count
        ),
        (objects, templates, snapshots)
    );
    assert_eq!(
        (result.attachment_count, result.attachment_files_written),
        (0, 0)
    );
    assert!(!result.preferences_imported);
}

fn assert_not_committed(result: &ImportResult) {
    assert_eq!(result.status, ImportStatus::NotCommitted, "{result:?}");
    assert_eq!(
        (
            result.object_count,
            result.template_count,
            result.snapshot_count
        ),
        (0, 0, 0)
    );
    assert_eq!(
        (result.attachment_count, result.attachment_files_written),
        (0, 0)
    );
    assert!(!result.preferences_imported);
    assert_eq!(result.error_code.as_deref(), Some("IMPORT_FAILED"));
}

fn history(id: &str) -> serde_json::Value {
    json!([{
        "object_id":id, "timestamp":1700000000000i64, "triggered_by":"source", "diff_summary":"first",
        "data":base64::Engine::encode(&base64::engine::general_purpose::STANDARD,b"history-first")
    },{
        "object_id":id, "timestamp":1700000001000i64, "triggered_by":"source", "diff_summary":"second",
        "data":base64::Engine::encode(&base64::engine::general_purpose::STANDARD,b"history-second")
    }])
}

fn templates() -> serde_json::Value {
    json!([{
        "id":"rf021-compat-tpl-a", "accountId":"source", "name":"RF021 compatibility first",
        "properties":[], "createdAt":"2026-09-30T00:00:00Z"
    },{
        "id":"rf021-compat-tpl-b", "accountId":"source", "name":"RF021 compatibility second",
        "properties":[], "createdAt":"2026-09-30T00:00:00Z"
    }])
}

fn seed_sixty_history_for(f: &Fixture, object_id: &str) {
    for ordinal in 0..60 {
        f.vault
            .save_snapshot_at(
                object_id,
                "local",
                format!("old-{ordinal}").as_bytes(),
                "old",
                1600000000000 + ordinal,
            )
            .unwrap();
    }
    let count: i64 =
        f.db.query_row(
            "SELECT COUNT(*) FROM object_snapshots WHERE object_id=?1",
            [object_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 60);
    // 显式证明原列表 API 的 50 条窗口不能充当全量回滚证据。
    assert_eq!(f.vault.list_snapshots(object_id).unwrap().len(), 50);
}

fn seed_sixty_history(f: &Fixture) {
    seed_sixty_history_for(f, "rf020-0");
}

#[test]
fn rf021_host_unused_bad_typed_rows_do_not_block_overwrite_or_skip() {
    for strategy in [ImportStrategy::Overwrite, ImportStrategy::SkipExisting] {
        for (column, literal) in CORRUPT_ROWS {
            let f = Fixture::new();
            local(&f, "rf021-unused", "Unused local");
            corrupt(&f, "rf021-unused", column, literal, false);
            let old = raw_table(&f.db, "objects");
            let path = package(f.dir.path(), clean_objects(), false, false, false);
            let result = run(&f, request(&path, strategy));
            assert_complete_counts(&result, 2, 0, 2);
            let rows = raw_table(&f.db, "objects");
            assert_eq!(rows.len(), old.len() + 2);
            assert!(
                rows.contains(&old[0]),
                "unused {column} {literal} must retain original bytes"
            );
            for id in ["rf020-0", "rf020-1"] {
                assert_eq!(f.vault.load_object(id).unwrap().unwrap().name, "Synthetic");
            }
        }
    }
}

#[test]
fn rf021_host_only_good_selection_ignores_bad_typed_unselected_target() {
    for strategy in [ImportStrategy::Overwrite, ImportStrategy::SkipExisting] {
        for (column, literal) in CORRUPT_ROWS {
            let f = Fixture::new();
            local(&f, "rf020-1", "Unselected local");
            corrupt(&f, "rf020-1", column, literal, false);
            let old = raw_table(&f.db, "objects");
            let path = package(f.dir.path(), clean_objects(), false, false, false);
            let mut req = request(&path, strategy);
            select_only(&mut req, "rf020-0");
            let result = run(&f, req);
            assert_complete_counts(&result, 1, 0, 1);
            assert_eq!(
                f.vault.load_object("rf020-0").unwrap().unwrap().name,
                "Synthetic"
            );
            let rows = raw_table(&f.db, "objects");
            assert_eq!(rows.len(), old.len() + 1);
            assert!(
                rows.contains(&old[0]),
                "unselected target {column} {literal} was altered"
            );
        }
    }
}

#[test]
fn rf021_host_explicit_empty_selection_does_not_parse_bad_typed_rows() {
    for strategy in [
        ImportStrategy::Overwrite,
        ImportStrategy::SkipExisting,
        ImportStrategy::KeepBoth,
    ] {
        for (column, literal) in CORRUPT_ROWS {
            let f = Fixture::new();
            local(&f, "rf020-1", "Unselected local");
            corrupt(&f, "rf020-1", column, literal, false);
            let path = package(f.dir.path(), clean_objects(), false, false, false);
            let before = raw_state(&f);
            let mut req = request(&path, strategy);
            req.selections = Some(vec![]);
            assert_complete_counts(&run(&f, req), 0, 0, 0);
            assert_eq!(raw_state(&f), before, "empty selection: {column} {literal}");
        }
    }
}

#[test]
fn rf021_host_softdeleted_bad_typed_rows_do_not_enter_keepboth_active_list() {
    for strategy in [
        ImportStrategy::Overwrite,
        ImportStrategy::SkipExisting,
        ImportStrategy::KeepBoth,
    ] {
        for (column, literal) in CORRUPT_ROWS {
            let f = Fixture::new();
            local(&f, "rf021-softdeleted", "Deleted local");
            corrupt(&f, "rf021-softdeleted", column, literal, true);
            let old = raw_table(&f.db, "objects");
            // is_deleted 本身为 BLOB 时也由旧 SQL WHERE is_deleted=0 排除。
            assert!(f
                .vault
                .list_objects(&f.account, None, None, None, false, false)
                .unwrap()
                .is_empty());
            let path = package(f.dir.path(), clean_objects(), false, false, false);
            let result = run(&f, request(&path, strategy));
            assert_complete_counts(&result, 2, 0, 2);
            let rows = raw_table(&f.db, "objects");
            assert_eq!(rows.len(), old.len() + 2);
            assert!(
                rows.contains(&old[0]),
                "softdeleted {column} {literal} was altered"
            );
        }
    }
}

#[test]
fn rf021_host_keepboth_active_bad_name_stays_strict_and_zero_write() {
    for literal in ["x'80'", "CAST(x'80' AS TEXT)"] {
        let f = Fixture::new();
        local(&f, "rf021-active", "Active local");
        corrupt(&f, "rf021-active", "name", literal, false);
        assert!(f
            .vault
            .list_objects(&f.account, None, None, None, false, false)
            .is_err());
        let path = package(f.dir.path(), clean_objects(), false, false, false);
        let before = raw_state(&f);
        let result = run(&f, request(&path, ImportStrategy::KeepBoth));
        assert_not_committed(&result);
        assert_eq!(result.failure_stage, Some(ImportStage::Objects));
        assert_eq!(raw_state(&f), before, "strict active name {literal}");
    }
}

#[test]
fn rf021_host_chinese_owned_package_duplicate_id_keeps_name_history_and_counts() {
    let f = Fixture::new();
    local(&f, "rf021-name-1", "旅行（导入）");
    local(&f, "rf021-name-2", "旅行（导入） 2");
    let local_rows = raw_table(&f.db, "objects");
    let mut payload = clean_objects();
    payload["objects"][0]["name"] = json!("旅行");
    payload["objects"][0]["properties"] = json!({"ordinal":1});
    let mut duplicate = payload["objects"][0].clone();
    duplicate["properties"] = json!({"ordinal":2});
    payload["objects"]
        .as_array_mut()
        .unwrap()
        .insert(1, duplicate);
    payload["snapshots"] = history("rf020-0");
    let path = package(f.dir.path(), payload, false, false, false);
    let mut req = request(&path, ImportStrategy::KeepBoth);
    req.locale = "zh-CN".into();
    select_only(&mut req, "rf020-0");
    let result = run(&f, req);
    assert_complete_counts(&result, 1, 0, 4);
    assert!(f.vault.load_object("rf020-0").unwrap().is_none());
    assert!(f.vault.load_object("rf020-1").unwrap().is_none());
    let imported: Vec<_> = f
        .vault
        .list_objects(&f.account, None, None, None, false, false)
        .unwrap()
        .into_iter()
        .filter(|record| !record.id.starts_with("rf021-name-"))
        .collect();
    assert_eq!(imported.len(), 1);
    let stored = f.vault.load_object(&imported[0].id).unwrap().unwrap();
    assert_eq!(stored.name, "旅行（导入） 4");
    assert_eq!(stored.properties["ordinal"], 2);
    let snapshots = f.vault.list_snapshots(&stored.id).unwrap();
    assert_eq!(snapshots.len(), 4);
    let mut first = 0;
    let mut second = 0;
    for snapshot in snapshots {
        let data = f
            .vault
            .get_snapshot(snapshot["id"].as_str().unwrap())
            .unwrap()
            .unwrap();
        if data == b"history-first" {
            first += 1;
            assert_eq!(snapshot["timestamp"].as_i64(), Some(1700000000000));
        } else {
            assert_eq!(data, b"history-second");
            second += 1;
            assert_eq!(snapshot["timestamp"].as_i64(), Some(1700000001000));
        }
    }
    assert_eq!((first, second), (2, 2));
    let rows = raw_table(&f.db, "objects");
    assert_eq!(rows.len(), local_rows.len() + 1);
    assert!(local_rows.iter().all(|old| rows.contains(old)));
}

#[derive(Debug, Clone, Copy)]
enum Fault {
    Templates,
    ObjectInsert,
    ObjectUpdate,
    Snapshots,
    ObjectsHlc,
}

fn install_second_failure(f: &Fixture, fault: Fault) {
    let (table, event, condition) = match fault {
        Fault::Templates => (
            "user_templates",
            "INSERT",
            "NEW.id IN ('rf021-compat-tpl-a','rf021-compat-tpl-b')",
        ),
        Fault::ObjectInsert => ("objects", "INSERT", "NEW.id IN ('rf020-0','rf020-1')"),
        Fault::ObjectUpdate => ("objects", "UPDATE", "NEW.id IN ('rf020-0','rf020-1')"),
        Fault::Snapshots => (
            "object_snapshots",
            "INSERT",
            "NEW.object_id IN ('rf020-0','rf020-1')",
        ),
        // HLC 的实际 SQL 列为 table_name：模板 HLC 绝不能消耗对象的第 N 次计数。
        Fault::ObjectsHlc => (
            "sync_hlc",
            "INSERT",
            "NEW.table_name='objects' AND NEW.record_id IN ('rf020-0','rf020-1')",
        ),
    };
    f.db.execute_batch(&format!(
        "CREATE TABLE rf021_compat_counter(n INTEGER NOT NULL); INSERT INTO rf021_compat_counter VALUES(0);
         CREATE TRIGGER rf021_compat_fail BEFORE {event} ON {table} WHEN {condition} BEGIN
         UPDATE rf021_compat_counter SET n=n+1;
         SELECT CASE WHEN (SELECT n FROM rf021_compat_counter)=2
         THEN RAISE(ABORT,'sensitive compatibility fault') END; END;"
    )).unwrap();
}

#[test]
fn rf021_host_owned_atomic_second_write_matrix_restores_four_tables_and_sixty_history() {
    for fault in [
        Fault::Templates,
        Fault::ObjectInsert,
        Fault::ObjectUpdate,
        Fault::Snapshots,
        Fault::ObjectsHlc,
    ] {
        let f = Fixture::new();
        // HLC 场景的两个包对象都是真正的新 ID，避免已有 HLC 的 UPSERT 事件干扰证据。
        let history_owner = if matches!(fault, Fault::ObjectsHlc) {
            "rf021-history-owner"
        } else {
            "rf020-0"
        };
        local(&f, history_owner, "Original first");
        if matches!(fault, Fault::ObjectUpdate) {
            // UPDATE 故障必须命中两个既有目标；其它场景的第二对象是真实新增。
            local(&f, "rf020-1", "Original second");
        }
        seed_sixty_history_for(&f, history_owner);
        let mut payload = clean_objects();
        payload["templates"] = templates();
        payload["objects"][0]["template_id"] = json!("rf021-compat-tpl-a");
        payload["objects"][1]["template_id"] = json!("rf021-compat-tpl-b");
        let mut snapshots = history("rf020-0").as_array().unwrap().clone();
        snapshots.extend(history("rf020-1").as_array().unwrap().clone());
        payload["snapshots"] = json!(snapshots);
        let path = package(f.dir.path(), payload, false, false, false);
        install_second_failure(&f, fault);
        let before = raw_state(&f);
        let result = run(&f, request(&path, ImportStrategy::Overwrite));
        assert_not_committed(&result);
        let stage = match fault {
            Fault::Templates => ImportStage::Templates,
            Fault::Snapshots => ImportStage::Snapshots,
            _ => ImportStage::Objects,
        };
        assert_eq!(result.failure_stage, Some(stage), "{fault:?}");
        assert_eq!(
            raw_state(&f),
            before,
            "{fault:?}: every original cell/ciphertext must survive"
        );
        let counter: i64 =
            f.db.query_row("SELECT n FROM rf021_compat_counter", [], |row| row.get(0))
                .unwrap();
        assert_eq!(
            counter, 0,
            "{fault:?}: trigger DML must roll back with the batch"
        );
        assert!(!serde_json::to_string(&result)
            .unwrap()
            .contains("sensitive compatibility fault"));
        let all_history: i64 =
            f.db.query_row(
                "SELECT COUNT(*) FROM object_snapshots WHERE object_id=?1",
                [history_owner],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(all_history, 60);
        f.db.execute_batch("DROP TRIGGER rf021_compat_fail; DROP TABLE rf021_compat_counter;")
            .unwrap();
        let retry = run(&f, request(&path, ImportStrategy::Overwrite));
        assert_complete_counts(&retry, 2, 2, 4);
        assert_eq!(f.vault.list_snapshots("rf020-0").unwrap().len(), 2);
        assert_eq!(f.vault.list_snapshots("rf020-1").unwrap().len(), 2);
    }
}

#[test]
fn rf021_host_owned_history_delete_failure_retains_all_sixty_ciphertexts() {
    let f = Fixture::new();
    local(&f, "rf020-0", "Original");
    seed_sixty_history(&f);
    let mut payload = clean_objects();
    payload["objects"].as_array_mut().unwrap().truncate(1);
    payload["snapshots"] = history("rf020-0");
    let path = package(f.dir.path(), payload, false, false, false);
    f.db.execute_batch(
        "CREATE TRIGGER rf021_compat_delete_fail BEFORE DELETE ON object_snapshots
         WHEN OLD.object_id='rf020-0' BEGIN SELECT RAISE(ABORT,'history delete fault'); END;",
    )
    .unwrap();
    let before = raw_state(&f);
    let result = run(&f, request(&path, ImportStrategy::Overwrite));
    assert_not_committed(&result);
    assert_eq!(result.failure_stage, Some(ImportStage::Snapshots));
    assert_eq!(raw_state(&f), before);
    f.db.execute_batch("DROP TRIGGER rf021_compat_delete_fail;")
        .unwrap();
    assert_complete_counts(&run(&f, request(&path, ImportStrategy::Overwrite)), 1, 0, 2);
}

#[test]
fn rf021_host_owned_bad_or_empty_package_history_appends_without_losing_sixty_rows() {
    for shape in 0..3 {
        let f = Fixture::new();
        local(&f, "rf020-0", "Original");
        seed_sixty_history(&f);
        let old = raw_table(&f.db, "object_snapshots");
        let mut payload = clean_objects();
        payload["objects"].as_array_mut().unwrap().truncate(1);
        if shape == 1 {
            payload["snapshots"] = json!([]);
        }
        if shape == 2 {
            payload["snapshots"] = json!([
                {"object_id":"rf020-0","data":"not-base64!"},
                {"object_id":"rf020-0","data":""}
            ]);
        }
        let path = package(f.dir.path(), payload, false, false, false);
        assert_complete_counts(&run(&f, request(&path, ImportStrategy::Overwrite)), 1, 0, 1);
        let rows = raw_table(&f.db, "object_snapshots");
        assert_eq!(rows.len(), 61);
        assert!(
            old.iter().all(|old_row| rows.contains(old_row)),
            "shape {shape}: old ciphertext changed"
        );
    }
}

// 空批次不生成 HLC；无关的本地 HLC 类型损坏不能新增零写导入失败。
#[test]
fn rf021_host_empty_selection_and_all_skip_do_not_read_unrelated_corrupt_hlc() {
    for all_skip in [false, true] {
        let f = Fixture::new();
        let node_id = "0123456789abcdef0123456789abcdef";
        f.vault.set_sync_node_id(node_id).unwrap();
        if all_skip {
            for id in ["rf020-0", "rf020-1"] {
                local(&f, id, "Existing active object");
            }
        } else {
            local(&f, "rf021-empty-local", "Unrelated active object");
        }
        let mut statement =
            f.db.prepare("SELECT wall_time_ms FROM sync_hlc WHERE node_id=?1")
                .unwrap();
        let normal_hlc = statement
            .query_map([node_id], |row| row.get::<_, i64>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(
            !normal_hlc.is_empty(),
            "save_object must seed the actual local node HLC"
        );
        assert!(normal_hlc.iter().all(|wall_time| *wall_time > 0));
        drop(statement);
        // Fixture.db 是独立于 Vault writer 的第二个真实 SQLite connection。
        let corrupted =
            f.db.execute(
                "UPDATE sync_hlc SET wall_time_ms=x'00' WHERE node_id=?1",
                [node_id],
            )
            .unwrap();
        assert_eq!(corrupted, normal_hlc.len());
        assert!(
            f.db.query_row(
                "SELECT COALESCE(MAX(wall_time_ms), 0) FROM sync_hlc WHERE node_id=?1",
                [node_id],
                |row| row.get::<_, i64>(0),
            )
            .is_err(),
            "the actual BatchHlc typed MAX read must reject the injected BLOB"
        );
        let before = raw_state(&f);
        let path = package(f.dir.path(), clean_objects(), false, false, false);
        let mut req = request(
            &path,
            if all_skip {
                ImportStrategy::SkipExisting
            } else {
                ImportStrategy::Overwrite
            },
        );
        if !all_skip {
            req.selections = Some(vec![]);
        }
        let outcome = run(&f, req);
        assert_complete_counts(&outcome, 0, 0, 0);
        assert_eq!(outcome.failure_stage, None);
        assert_eq!(outcome.error_code, None);
        assert_eq!(
            raw_state(&f),
            before,
            "empty-selection/all-skip must retain all four business tables byte for byte"
        );
    }
}
