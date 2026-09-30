//! RF-021 Host 候选：真实加密包、实际 VaultStore 与第二 SQLite 连接。
//! TEMP 草稿，未执行；底层 COMMIT 故障/wrong-store 等由 Vault 回归负责。
use super::rf020::{objects, package, Fixture};
use super::*;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

fn run(
    f: &Fixture,
    path: &Path,
    strategy: ImportStrategy,
    progress: Option<Arc<dyn Fn(u8) + Send + Sync>>,
) -> ImportResult {
    let service = f.service.read().unwrap();
    let session = service.capture_session(&f.account).unwrap();
    super::super::import::import_execute_for_session(
        &service,
        &session,
        path.to_string_lossy().into_owned(),
        Zeroizing::new("export-password".into()),
        strategy,
        None,
        None,
        HashMap::new(),
        "en-US",
        progress,
    )
    .unwrap()
}

fn raw_state(db: &rusqlite::Connection) -> Vec<Vec<Vec<rusqlite::types::Value>>> {
    ["objects", "user_templates", "object_snapshots", "sync_hlc"]
        .into_iter()
        .map(|table| {
            let mut statement = db
                .prepare(&format!("SELECT * FROM {table} ORDER BY 1,2"))
                .unwrap();
            let count = statement.column_count();
            let rows = statement
                .query_map([], |row| {
                    (0..count)
                        .map(|column| row.get(column))
                        .collect::<rusqlite::Result<Vec<rusqlite::types::Value>>>()
                })
                .unwrap();
            rows.map(Result::unwrap).collect()
        })
        .collect()
}

fn record(f: &Fixture, id: &str, name: &str) -> ObjectRecord {
    ObjectRecord {
        id: id.into(),
        account_id: f.account.clone(),
        name: name.into(),
        type_id: "note".into(),
        section_type: "identity".into(),
        icon_name: "document".into(),
        parent_id: None,
        children_ids: vec![],
        properties: json!({"local":true}),
        property_labels: None,
        sensitivity_level: "internal".into(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: None,
        template_type: None,
        contract_type_id: None,
        template_hash: None,
        ignored_template_hash: None,
        created_at: "2026-09-25T00:00:00Z".into(),
        updated_at: "2026-09-25T00:00:00Z".into(),
        version: 1,
    }
}

fn templates() -> serde_json::Value {
    json!([{
        "id":"rf021-tpl-a","accountId":"source","name":"RF021 first template","properties":[],"createdAt":"2026-09-25T00:00:00Z"
    },{
        "id":"rf021-tpl-b","accountId":"source","name":"RF021 second template","properties":[],"createdAt":"2026-09-25T00:00:00Z"
    }])
}

fn history(id: &str) -> serde_json::Value {
    json!([{
        "object_id":id,"timestamp":1700000000000i64,"triggered_by":"source","diff_summary":"first",
        "data":base64::Engine::encode(&base64::engine::general_purpose::STANDARD,b"history-first")
    },{
        "object_id":id,"timestamp":1700000001000i64,"triggered_by":"source","diff_summary":"second",
        "data":base64::Engine::encode(&base64::engine::general_purpose::STANDARD,b"history-second")
    }])
}

fn assert_zero(outcome: &ImportResult) {
    assert_eq!(outcome.status, ImportStatus::NotCommitted);
    assert_eq!(outcome.object_count, 0);
    assert_eq!(outcome.template_count, 0);
    assert_eq!(outcome.snapshot_count, 0);
    assert_eq!(outcome.attachment_count, 0);
    assert_eq!(outcome.attachment_files_written, 0);
    assert!(!outcome.preferences_imported);
    assert_eq!(outcome.error_code.as_deref(), Some("IMPORT_FAILED"));
}

#[test]
fn rf021_host_database_fault_rolls_back_every_table_and_fresh_retry_succeeds() {
    for (table, stage) in [
        ("user_templates", ImportStage::Templates),
        ("objects", ImportStage::Objects),
        ("object_snapshots", ImportStage::Snapshots),
        ("sync_hlc", ImportStage::Objects),
    ] {
        let f = Fixture::new();
        f.db.execute_batch(&format!("CREATE TABLE rf021_counter(n INTEGER NOT NULL);INSERT INTO rf021_counter VALUES(0);
            CREATE TRIGGER rf021_fail BEFORE INSERT ON {table} BEGIN
              UPDATE rf021_counter SET n=n+1;
              SELECT CASE WHEN (SELECT n FROM rf021_counter)=2 THEN RAISE(ABORT,'sensitive injected database detail') END;END;")).unwrap();
        let mut payload = objects();
        payload["templates"] = templates();
        payload["objects"][0]["template_id"] = json!("rf021-tpl-a");
        payload["objects"][1]["template_id"] = json!("rf021-tpl-b");
        let mut snapshots = history("rf020-0").as_array().unwrap().clone();
        snapshots.extend(history("rf020-1").as_array().unwrap().clone());
        payload["snapshots"] = json!(snapshots);
        let path = package(f.dir.path(), payload, false, false, false);
        let before = raw_state(&f.db);
        let outcome = run(&f, &path, ImportStrategy::Overwrite, None);
        assert_zero(&outcome);
        assert_eq!(outcome.failure_stage, Some(stage));
        assert_eq!(raw_state(&f.db), before);
        let counter: i64 =
            f.db.query_row("SELECT n FROM rf021_counter", [], |row| row.get(0))
                .unwrap();
        assert_eq!(counter, 0);
        assert!(!serde_json::to_string(&outcome)
            .unwrap()
            .contains("sensitive injected"));
        f.db.execute_batch("DROP TRIGGER rf021_fail;DROP TABLE rf021_counter;")
            .unwrap();
        let retry = run(&f, &path, ImportStrategy::Overwrite, None)
            .require_complete()
            .unwrap();
        assert_eq!(retry.object_count, 2);
        assert_eq!(retry.template_count, 2);
        assert_eq!(retry.snapshot_count, 4);
    }
}

#[test]
fn rf021_host_history_sql_failure_preserves_all_sixty_old_rows() {
    for delete in [false, true] {
        let f = Fixture::new();
        f.vault.save_object(&record(&f, "rf020-0", "Old")).unwrap();
        for index in 0..60 {
            f.vault
                .save_snapshot("rf020-0", "local", format!("old-{index}").as_bytes(), "old")
                .unwrap();
        }
        if delete {
            f.db.execute_batch("CREATE TRIGGER rf021_fail BEFORE DELETE ON object_snapshots BEGIN SELECT RAISE(ABORT,'history delete fault');END;").unwrap();
        } else {
            f.db.execute_batch("CREATE TABLE rf021_counter(n INTEGER NOT NULL);INSERT INTO rf021_counter VALUES(0);
                CREATE TRIGGER rf021_fail BEFORE INSERT ON object_snapshots BEGIN UPDATE rf021_counter SET n=n+1;
                SELECT CASE WHEN (SELECT n FROM rf021_counter)=2 THEN RAISE(ABORT,'second history fault') END;END;").unwrap();
        }
        let mut payload = objects();
        payload["objects"].as_array_mut().unwrap().truncate(1);
        payload["templates"] = templates();
        payload["snapshots"] = history("rf020-0");
        let path = package(f.dir.path(), payload, false, false, false);
        let before = raw_state(&f.db);
        let outcome = run(&f, &path, ImportStrategy::Overwrite, None);
        assert_zero(&outcome);
        assert_eq!(outcome.failure_stage, Some(ImportStage::Snapshots));
        assert_eq!(raw_state(&f.db), before);
        let count: i64 =
            f.db.query_row(
                "SELECT COUNT(*) FROM object_snapshots WHERE object_id='rf020-0'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 60);
        assert_eq!(f.vault.load_object("rf020-0").unwrap().unwrap().name, "Old");
    }
}

#[test]
fn rf021_host_concurrent_dml_after_plan_rejects_import_and_preserves_writer() {
    for external in [false, true] {
        let f = Fixture::new();
        let sentinel = record(&f, "rf021-sentinel", "Before");
        f.vault.save_object(&sentinel).unwrap();
        let fired = Arc::new(AtomicBool::new(false));
        let expected = Arc::new(std::sync::Mutex::new(None));
        let fired_cb = fired.clone();
        let expected_cb = expected.clone();
        let db_path = f.vault.base_path().join("vault.db");
        let vault = f.vault.clone();
        let mut changed = sentinel;
        changed.name = "Concurrent".into();
        let callback: Arc<dyn Fn(u8) + Send + Sync> = Arc::new(move |progress| {
            if progress != 80 || fired_cb.swap(true, Ordering::SeqCst) {
                return;
            }
            if external {
                let db = rusqlite::Connection::open(&db_path).unwrap();
                assert_eq!(
                    db.execute(
                        "UPDATE objects SET name='Concurrent' WHERE id='rf021-sentinel'",
                        []
                    )
                    .unwrap(),
                    1
                );
            } else {
                vault.save_object(&changed).unwrap();
            }
            let db = rusqlite::Connection::open(&db_path).unwrap();
            *expected_cb.lock().unwrap() = Some(raw_state(&db));
        });
        let mut payload = objects();
        payload["templates"] = templates();
        let path = package(f.dir.path(), payload, false, false, false);
        let outcome = run(&f, &path, ImportStrategy::Overwrite, Some(callback));
        assert!(fired.load(Ordering::SeqCst));
        assert_zero(&outcome);
        assert_eq!(outcome.failure_stage, Some(ImportStage::Objects));
        assert_eq!(
            &raw_state(&f.db),
            expected.lock().unwrap().as_ref().unwrap()
        );
        assert_eq!(
            f.vault.load_object("rf021-sentinel").unwrap().unwrap().name,
            "Concurrent"
        );
        assert!(f.vault.load_object("rf020-0").unwrap().is_none());
        assert!(f.vault.load_user_template("rf021-tpl-a").unwrap().is_none());
    }
}

#[test]
fn rf021_host_strict_load_and_list_have_existing_error_boundaries() {
    for column in ["properties", "property_labels", "children_ids"] {
        for strategy in [ImportStrategy::Overwrite, ImportStrategy::SkipExisting] {
            let f = Fixture::new();
            f.vault.save_object(&record(&f, "rf020-0", "Old")).unwrap();
            f.db.execute(
                &format!("UPDATE objects SET {column}='solo:not-base64!' WHERE id='rf020-0'"),
                [],
            )
            .unwrap();
            let path = package(f.dir.path(), objects(), false, false, false);
            let before = raw_state(&f.db);
            let outcome = run(&f, &path, strategy, None);
            assert_zero(&outcome);
            assert_eq!(raw_state(&f.db), before);
        }
    }
    for (column, deleted, success) in [
        ("properties", false, false),
        ("property_labels", false, false),
        ("children_ids", false, true),
        ("properties", true, true),
    ] {
        let f = Fixture::new();
        let mut local = record(&f, "rf021-unrelated", "Other");
        local.is_deleted = deleted;
        f.vault.save_object(&local).unwrap();
        f.db.execute(
            &format!("UPDATE objects SET {column}='solo:not-base64!' WHERE id='rf021-unrelated'"),
            [],
        )
        .unwrap();
        let path = package(f.dir.path(), objects(), false, false, false);
        let before = raw_state(&f.db);
        let outcome = run(&f, &path, ImportStrategy::KeepBoth, None);
        if success {
            assert_eq!(outcome.require_complete().unwrap().object_count, 2);
        } else {
            assert_zero(&outcome);
            assert_eq!(raw_state(&f.db), before);
        }
    }
}

#[test]
fn rf021_host_template_reads_are_on_demand_and_bad_reference_retains_snapshot_timing() {
    for shape in [0, 1, 2, 3] {
        let f = Fixture::new();
        let now = "2026-09-25T00:00:00Z";
        f.vault
            .save_user_template(&UserTemplate {
                id: "rf021-bad-template".into(),
                account_id: f.account.clone(),
                name: "Local bad template name".into(),
                icon_id: None,
                properties: vec![],
                category: None,
                contract_type_id: None,
                created_at: now.into(),
                updated_at: None,
            })
            .unwrap();
        f.db.execute("UPDATE user_templates SET properties_json='solo:not-base64!' WHERE id='rf021-bad-template'",[]).unwrap();
        let mut payload = objects();
        payload["objects"].as_array_mut().unwrap().truncate(1);
        payload["objects"][0]["template_id"] = json!("rf021-bad-template");
        payload["objects"][0]["properties"] =
            json!({"__fields":{"input":{"name":"Kept","type":"text"}},"__templateName":"incoming"});
        payload["objects"][0]["property_labels"] = json!({"input":"critical"});
        match shape {
            0 => {}
            1 => payload["templates"] = serde_json::Value::Null,
            2 => payload["templates"] = json!([]),
            _ => payload["templates"] = templates(),
        };
        let path = package(f.dir.path(), payload, false, false, false);
        let before = raw_state(&f.db);
        let outcome = run(&f, &path, ImportStrategy::Overwrite, None);
        if shape == 3 {
            assert_zero(&outcome);
            assert_eq!(outcome.failure_stage, Some(ImportStage::Templates));
            assert_eq!(raw_state(&f.db), before);
            continue;
        }
        assert_eq!(outcome.require_complete().unwrap().object_count, 1);
        let stored = f.vault.load_object("rf020-0").unwrap().unwrap();
        assert_eq!(stored.template_id.as_deref(), Some("rf021-bad-template"));
        assert_eq!(
            stored.properties["__templateName"],
            "Local bad template name"
        );
        assert_eq!(stored.properties["__fields"]["input"]["name"], "Kept");
        assert_eq!(stored.property_labels, Some(json!({"input":"critical"})));
        let snapshots = f.vault.list_snapshots("rf020-0").unwrap();
        assert_eq!(snapshots.len(), 1);
        let data = f
            .vault
            .get_snapshot(snapshots[0]["id"].as_str().unwrap())
            .unwrap()
            .unwrap();
        let snapshot: serde_json::Value = serde_json::from_slice(&data).unwrap();
        assert_eq!(snapshot["properties"]["__templateName"], "incoming");
    }
}

#[test]
fn rf021_host_duplicate_ids_preserve_sequential_strategy_name_and_write_count() {
    for strategy in [
        ImportStrategy::Overwrite,
        ImportStrategy::SkipExisting,
        ImportStrategy::KeepBoth,
    ] {
        let f = Fixture::new();
        let mut payload = objects();
        payload["objects"][1] = payload["objects"][0].clone();
        payload["objects"][0]["properties"] = json!({"ordinal":1});
        payload["objects"][1]["properties"] = json!({"ordinal":2});
        let path = package(f.dir.path(), payload, false, false, false);
        let outcome = run(&f, &path, strategy, None).require_complete().unwrap();
        assert_eq!(outcome.object_count, 1);
        let records = f.vault.list_object_records(&f.account).unwrap();
        let stored = &records[0];
        assert_eq!(records.len(), 1);
        assert_eq!(
            stored.properties["ordinal"],
            if strategy == ImportStrategy::SkipExisting {
                1
            } else {
                2
            }
        );
        if strategy == ImportStrategy::KeepBoth {
            assert_ne!(stored.id, "rf020-0");
            assert_eq!(stored.name, "Synthetic (Imported) 2");
        } else {
            assert_eq!(stored.id, "rf020-0");
            assert_eq!(stored.name, "Synthetic");
        }
        let count: usize =
            f.db.query_row(
                "SELECT COUNT(*) FROM object_snapshots WHERE object_id=?1",
                [&stored.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            count,
            if strategy == ImportStrategy::SkipExisting {
                1
            } else {
                2
            }
        );
        assert_eq!(outcome.snapshot_count, count);
    }
}

#[test]
fn rf021_host_valid_history_replaces_and_missing_empty_bad_history_only_appends() {
    for shape in [0, 1, 2, 3, 4] {
        let f = Fixture::new();
        f.vault.save_object(&record(&f, "rf020-0", "Old")).unwrap();
        f.vault
            .save_snapshot("rf020-0", "local", b"local-old", "old")
            .unwrap();
        let old_id = f.vault.list_snapshots("rf020-0").unwrap()[0]["id"]
            .as_str()
            .unwrap()
            .to_string();
        let mut payload = objects();
        payload["objects"].as_array_mut().unwrap().truncate(1);
        match shape {
            0 => {}
            1 => payload["snapshots"] = json!([]),
            2 => {
                payload["snapshots"] = json!([{"object_id":"rf020-0","data":"not-valid-base64"},{"object_id":"rf020-0","data":""}])
            }
            _ => {
                let mut snaps = history("rf020-0").as_array().unwrap().clone();
                snaps.push(json!({"object_id":"rf020-0","data":"bad"}));
                payload["snapshots"] = json!(snaps);
            }
        }
        if shape == 4 {
            let duplicate = payload["objects"][0].clone();
            payload["objects"].as_array_mut().unwrap().push(duplicate);
        }
        let path = package(f.dir.path(), payload, false, false, false);
        let outcome = run(&f, &path, ImportStrategy::Overwrite, None)
            .require_complete()
            .unwrap();
        let count: usize =
            f.db.query_row(
                "SELECT COUNT(*) FROM object_snapshots WHERE object_id='rf020-0'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);
        assert_eq!(
            outcome.snapshot_count,
            if shape < 3 {
                1
            } else if shape == 4 {
                4
            } else {
                2
            }
        );
        assert_eq!(f.vault.get_snapshot(&old_id).unwrap().is_some(), shape < 3);
        if shape >= 3 {
            let snapshots = f.vault.list_snapshots("rf020-0").unwrap();
            let timestamps: Vec<i64> = snapshots
                .iter()
                .map(|snap| snap["timestamp"].as_i64().unwrap())
                .collect();
            assert_eq!(timestamps, vec![1700000001000, 1700000000000]);
            assert_eq!(
                f.vault
                    .get_snapshot(snapshots[0]["id"].as_str().unwrap())
                    .unwrap()
                    .unwrap(),
                b"history-second"
            );
        }
    }
}

#[test]
fn rf021_host_shadow_name_excludes_bad_original_raw_without_weakening_strict_list() {
    let f = Fixture::new();
    let local = record(&f, "rf021-shadowed", "Old");
    f.vault.save_object(&local).unwrap();
    f.db.execute(
        "UPDATE objects SET properties='solo:not-base64!' WHERE id='rf021-shadowed'",
        [],
    )
    .unwrap();
    let view = f.vault.read_import_view(&f.account).unwrap();
    let mut shadow = HashMap::new();
    let mut replacement = local;
    replacement.name = "Copy (Imported)".into();
    shadow.insert(replacement.id.clone(), replacement);
    // 该直接 helper 边界并不声称真实包 Overwrite 坏目标能成功（前面的 strict load 测试证明拒绝）。
    let name =
        super::super::import::rf021_unique_shadow_name(&f.vault, &view, &shadow, "Copy", "en-US")
            .unwrap();
    assert_eq!(name, "Copy (Imported) 2");
    assert!(super::super::import::rf021_unique_shadow_name(
        &f.vault,
        &view,
        &HashMap::new(),
        "Copy",
        "en-US"
    )
    .is_err());
}
