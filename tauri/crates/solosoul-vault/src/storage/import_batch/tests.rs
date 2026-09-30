//! RF-021：使用合成 TempDir 与真实 SQLite 故障验证导入批次边界。

use super::*;
use crate::{ObjectRecord, PropertyType, TemplateProperty, UserTemplate, VaultConfig};
use rusqlite::{types::Value as SqlValue, Connection};
use std::collections::BTreeSet;
use std::time::Duration;
use tempfile::TempDir;

const ACCOUNT: &str = "rf021_account";
const NOW: &str = "2026-09-30T00:00:00Z";

struct Fixture {
    vault: VaultStore,
    db: Connection,
    _root: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        let base = root.path().join(ACCOUNT);
        std::fs::create_dir_all(&base).unwrap();
        let vault =
            VaultStore::open(VaultConfig::new(ACCOUNT, base).with_data_key([0x21; 32])).unwrap();
        let db = Connection::open(vault.base_path().join("vault.db")).unwrap();
        db.busy_timeout(Duration::from_millis(20)).unwrap();
        Self {
            vault,
            db,
            _root: root,
        }
    }

    fn view(&self) -> ImportReadView {
        self.vault.read_import_view(ACCOUNT).unwrap()
    }

    fn commit(
        &self,
        view: &ImportReadView,
        batch: &ImportDatabaseBatch,
    ) -> Result<ImportDatabaseCommit, ImportBatchError> {
        self.vault
            .commit_import_batch(ACCOUNT, &view.revision, batch)
    }

    fn install(&self, sql: &str) {
        self.db.execute_batch(sql).unwrap();
    }

    fn with_connection<T>(&self, action: impl FnOnce(&Connection) -> T) -> T {
        let guard = self.vault.conn.lock().unwrap();
        action(guard.as_ref().unwrap())
    }

    fn seed_object(&self, id: &str, name: &str) {
        self.vault.save_object(&record(id, name)).unwrap();
    }

    fn seed_history(&self, owner: &str, count: usize) {
        for index in 0..count {
            let data = serde_json::to_vec(&serde_json::json!({
                "name": format!("RF021 old history {index}"),
                "tags": ["synthetic"],
                "properties": { "secret": format!("old secret {index}") }
            }))
            .unwrap();
            self.vault
                .save_snapshot_at(
                    owner,
                    "rf021_seed",
                    &data,
                    "synthetic existing history",
                    1_600_000_000_000 + index as i64,
                )
                .unwrap();
        }
    }

    fn raw_rows(&self, table: &str) -> Vec<Vec<SqlValue>> {
        let mut statement = self
            .db
            .prepare(&format!("SELECT * FROM {table} ORDER BY 1,2"))
            .unwrap();
        let columns = statement.column_count();
        let rows = statement
            .query_map([], |row| {
                (0..columns)
                    .map(|column| row.get::<_, SqlValue>(column))
                    .collect::<rusqlite::Result<Vec<_>>>()
            })
            .unwrap();
        rows.collect::<rusqlite::Result<Vec<_>>>().unwrap()
    }

    fn raw_state(&self) -> Vec<Vec<Vec<SqlValue>>> {
        ["objects", "user_templates", "object_snapshots", "sync_hlc"]
            .into_iter()
            .map(|table| self.raw_rows(table))
            .collect()
    }

    fn history_ids(&self, owner: &str) -> BTreeSet<String> {
        let mut statement = self
            .db
            .prepare("SELECT id FROM object_snapshots WHERE object_id = ?1 ORDER BY id")
            .unwrap();
        let rows = statement
            .query_map([owner], |row| row.get::<_, String>(0))
            .unwrap();
        rows.collect::<rusqlite::Result<BTreeSet<_>>>().unwrap()
    }

    fn fail_second(&self, event: &str, table: &str) {
        self.install(&format!(
            "CREATE TABLE rf021_fail_counter (n INTEGER NOT NULL);
             INSERT INTO rf021_fail_counter VALUES (0);
             CREATE TRIGGER rf021_fail_second BEFORE {event} ON {table} BEGIN
               UPDATE rf021_fail_counter SET n = n + 1;
               SELECT CASE WHEN (SELECT n FROM rf021_fail_counter) = 2
                 THEN RAISE(ABORT, 'RF021 synthetic second write failure') END;
             END;"
        ));
    }

    fn assert_counter_rolled_back(&self) {
        let value: i64 = self
            .db
            .query_row("SELECT n FROM rf021_fail_counter", [], |row| row.get(0))
            .unwrap();
        assert_eq!(value, 0, "第一条真实写入与触发器计数也必须回滚");
    }

    fn assert_connection_reusable(&self) {
        self.with_connection(|connection| {
            assert!(connection.is_autocommit(), "失败后不能遗留打开的事务");
            connection
                .execute_batch("BEGIN IMMEDIATE; ROLLBACK;")
                .unwrap();
            assert!(connection.is_autocommit());
        });
    }

    fn retry_without_trigger(
        &self,
        batch: &ImportDatabaseBatch,
        trigger: &str,
    ) -> ImportDatabaseCommit {
        self.install(&format!("DROP TRIGGER {trigger}"));
        self.assert_connection_reusable();
        let fresh = self.view();
        self.commit(&fresh, batch).unwrap()
    }
}

fn record(id: &str, name: &str) -> ObjectRecord {
    ObjectRecord {
        id: id.into(),
        account_id: ACCOUNT.into(),
        type_id: "note".into(),
        section_type: "rf021".into(),
        name: name.into(),
        icon_name: "document".into(),
        properties: serde_json::json!({
            "secret": "RF021 secret record data",
            "ordinary": name
        }),
        property_labels: Some(serde_json::json!({ "secret": "critical" })),
        sensitivity_level: "internal".into(),
        created_at: NOW.into(),
        updated_at: NOW.into(),
        version: 1,
        ..Default::default()
    }
}

fn template(id: &str, name: &str) -> UserTemplate {
    UserTemplate {
        id: id.into(),
        account_id: ACCOUNT.into(),
        name: name.into(),
        icon_id: Some("document".into()),
        properties: vec![TemplateProperty {
            id: "rf021_secret".into(),
            name: "RF021 secret template field".into(),
            prop_type: PropertyType::Text,
            sensitivity_level: Some("critical".into()),
            options: None,
            sensitive: None,
            deprecated_at: None,
            allowed_types: None,
            max_items: None,
            contract_field: None,
            contract_bindings: None,
        }],
        category: Some("rf021".into()),
        created_at: NOW.into(),
        updated_at: Some(NOW.into()),
        contract_type_id: None,
    }
}

fn snapshot(index: i64) -> ImportSnapshot {
    ImportSnapshot {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp_ms: 1_700_000_000_000 + index,
        triggered_by: "rf021_import".into(),
        data: serde_json::to_vec(&serde_json::json!({
            "name": format!("RF021 imported history {index}"),
            "tags": [],
            "properties": { "secret": format!("RF021 snapshot secret {index}") }
        }))
        .unwrap(),
        diff_summary: format!("synthetic imported snapshot {index}"),
    }
}

fn write(record: ObjectRecord, history: ImportHistoryChange) -> ImportObjectWrite {
    ImportObjectWrite { record, history }
}

fn batch(objects: Vec<ImportObjectWrite>) -> ImportDatabaseBatch {
    ImportDatabaseBatch {
        templates: vec![],
        objects,
    }
}

fn expect_error(
    result: Result<ImportDatabaseCommit, ImportBatchError>,
    expected: ImportBatchError,
) {
    match result {
        Err(error) => assert_eq!(error, expected),
        Ok(_) => panic!("导入批次应返回 {expected:?}"),
    }
}

fn expect_read_error(result: Result<ImportReadView, ImportBatchError>, expected: ImportBatchError) {
    match result {
        Err(error) => assert_eq!(error, expected),
        Ok(_) => panic!("导入视图应返回 {expected:?}"),
    }
}

#[test]
fn rf021_second_template_insert_failure_rolls_back_entire_database_batch() {
    let f = Fixture::new();
    f.seed_object("retained", "retain this object");
    f.seed_history("retained", 2);
    f.fail_second("INSERT", "user_templates");
    let plan = ImportDatabaseBatch {
        templates: vec![template("template_a", "A"), template("template_b", "B")],
        objects: vec![write(record("imported", "new"), ImportHistoryChange::Keep)],
    };
    let view = f.view();
    let before = f.raw_state();
    expect_error(f.commit(&view, &plan), ImportBatchError::Templates);
    assert_eq!(f.raw_state(), before);
    f.assert_counter_rolled_back();
    let committed = f.retry_without_trigger(&plan, "rf021_fail_second");
    assert_eq!(committed.template_ids.len(), 2);
    assert_eq!(committed.object_write_count, 1);
}

#[test]
fn rf021_second_object_insert_failure_rolls_back_templates_objects_histories_and_hlc() {
    let f = Fixture::new();
    f.fail_second("INSERT", "objects");
    let plan = ImportDatabaseBatch {
        templates: vec![template("template_new", "synthetic template")],
        objects: vec![
            write(
                record("object_a", "A"),
                ImportHistoryChange::Append(vec![snapshot(1)]),
            ),
            write(
                record("object_b", "B"),
                ImportHistoryChange::Append(vec![snapshot(2)]),
            ),
        ],
    };
    let view = f.view();
    let before = f.raw_state();
    expect_error(f.commit(&view, &plan), ImportBatchError::Objects);
    assert_eq!(f.raw_state(), before);
    f.assert_counter_rolled_back();
    let committed = f.retry_without_trigger(&plan, "rf021_fail_second");
    assert_eq!(committed.object_write_count, 2);
    assert_eq!(committed.object_ids.len(), 2);
}

#[test]
fn rf021_second_object_update_failure_rolls_back_both_overwrites_and_history_replacements() {
    let f = Fixture::new();
    f.seed_object("object_a", "old A");
    f.seed_object("object_b", "old B");
    f.seed_history("object_a", 2);
    f.seed_history("object_b", 3);
    f.fail_second("UPDATE", "objects");
    let plan = ImportDatabaseBatch {
        templates: vec![template("template_new", "new template")],
        objects: vec![
            write(
                record("object_a", "new A"),
                ImportHistoryChange::Replace(vec![snapshot(1)]),
            ),
            write(
                record("object_b", "new B"),
                ImportHistoryChange::Replace(vec![snapshot(2)]),
            ),
        ],
    };
    let view = f.view();
    let before = f.raw_state();
    expect_error(f.commit(&view, &plan), ImportBatchError::Objects);
    assert_eq!(f.raw_state(), before);
    f.assert_counter_rolled_back();
    f.retry_without_trigger(&plan, "rf021_fail_second");
    assert_eq!(
        f.vault.load_object("object_a").unwrap().unwrap().name,
        "new A"
    );
    assert_eq!(f.history_ids("object_a").len(), 1);
    assert_eq!(f.history_ids("object_b").len(), 1);
}

#[test]
fn rf021_second_snapshot_insert_failure_restores_previous_ciphertexts_and_all_records() {
    let f = Fixture::new();
    f.seed_object("owner", "old owner");
    f.seed_history("owner", 3);
    f.fail_second("INSERT", "object_snapshots");
    let plan = ImportDatabaseBatch {
        templates: vec![template("new_template", "new template")],
        objects: vec![write(
            record("owner", "new owner"),
            ImportHistoryChange::Replace(vec![snapshot(1), snapshot(2)]),
        )],
    };
    let view = f.view();
    let before = f.raw_state();
    expect_error(f.commit(&view, &plan), ImportBatchError::Snapshots);
    assert_eq!(f.raw_state(), before);
    f.assert_counter_rolled_back();
    let committed = f.retry_without_trigger(&plan, "rf021_fail_second");
    assert_eq!(committed.snapshot_write_count, 2);
    assert_eq!(f.history_ids("owner").len(), 2);
}

#[test]
fn rf021_second_hlc_insert_failure_rolls_back_template_and_object_writes() {
    let f = Fixture::new();
    f.seed_object("retained", "old retained object");
    f.fail_second("INSERT", "sync_hlc");
    let plan = ImportDatabaseBatch {
        templates: vec![template("new_template", "new template")],
        objects: vec![
            write(
                record("object_a", "A"),
                ImportHistoryChange::Append(vec![snapshot(1)]),
            ),
            write(record("object_b", "B"), ImportHistoryChange::Keep),
        ],
    };
    let view = f.view();
    let before = f.raw_state();
    expect_error(f.commit(&view, &plan), ImportBatchError::Hlc);
    assert_eq!(f.raw_state(), before);
    f.assert_counter_rolled_back();
    let committed = f.retry_without_trigger(&plan, "rf021_fail_second");
    assert_eq!(committed.template_ids.len(), 1);
    assert_eq!(committed.object_write_count, 2);
}

#[test]
fn rf021_history_delete_failure_preserves_overwritten_object_and_its_old_history() {
    let f = Fixture::new();
    f.seed_object("owner", "old owner");
    f.seed_history("owner", 3);
    f.install(
        "CREATE TRIGGER rf021_fail_delete BEFORE DELETE ON object_snapshots
         BEGIN SELECT RAISE(ABORT, 'RF021 synthetic history delete failure'); END;",
    );
    let plan = ImportDatabaseBatch {
        templates: vec![template("new_template", "new template")],
        objects: vec![write(
            record("owner", "new owner"),
            ImportHistoryChange::Replace(vec![snapshot(1)]),
        )],
    };
    let view = f.view();
    let before = f.raw_state();
    expect_error(f.commit(&view, &plan), ImportBatchError::Snapshots);
    assert_eq!(f.raw_state(), before);
    f.retry_without_trigger(&plan, "rf021_fail_delete");
    assert_eq!(f.history_ids("owner").len(), 1);
}

#[test]
fn rf021_actual_deferred_foreign_key_commit_failure_rolls_back_and_connection_recovers() {
    let f = Fixture::new();
    f.seed_object("owner", "old owner");
    f.seed_history("owner", 2);
    // foreign_keys 是连接级选项：必须在真正提交批次的 Vault 连接上启用。
    f.with_connection(|connection| {
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON;
                 CREATE TABLE rf021_commit_parents (id INTEGER PRIMARY KEY);
                 CREATE TABLE rf021_commit_children (
                   id INTEGER PRIMARY KEY,
                   parent_id INTEGER NOT NULL REFERENCES rf021_commit_parents(id)
                     DEFERRABLE INITIALLY DEFERRED
                 );
                 CREATE TRIGGER rf021_fail_commit AFTER UPDATE ON objects BEGIN
                   INSERT INTO rf021_commit_children (id, parent_id) VALUES (1, 999);
                 END;",
            )
            .unwrap();
        assert_eq!(
            connection
                .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
    });
    let plan = ImportDatabaseBatch {
        templates: vec![template("new_template", "new template")],
        objects: vec![write(
            record("owner", "new owner"),
            ImportHistoryChange::Replace(vec![snapshot(1)]),
        )],
    };
    let view = f.view();
    let before = f.raw_state();
    expect_error(f.commit(&view, &plan), ImportBatchError::Commit);
    assert_eq!(f.raw_state(), before);
    assert_eq!(
        f.db.query_row("SELECT COUNT(*) FROM rf021_commit_children", [], |row| row
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        0
    );
    f.assert_connection_reusable();
    let committed = f.retry_without_trigger(&plan, "rf021_fail_commit");
    assert_eq!(committed.object_write_count, 1);
    assert_eq!(
        f.vault.load_object("owner").unwrap().unwrap().name,
        "new owner"
    );
}

#[test]
fn rf021_overwrite_failure_preserves_all_sixty_old_snapshots_byte_for_byte() {
    let f = Fixture::new();
    f.seed_object("owner", "old owner");
    f.seed_history("owner", 60);
    assert_eq!(f.history_ids("owner").len(), 60);
    // 查询 UI 历史只取 50 条，回滚比较必须读取表里的全部密文行。
    assert_eq!(f.vault.list_snapshots("owner").unwrap().len(), 50);
    f.fail_second("INSERT", "object_snapshots");
    let plan = batch(vec![write(
        record("owner", "new owner"),
        ImportHistoryChange::Replace(vec![snapshot(1), snapshot(2)]),
    )]);
    let view = f.view();
    let before = f.raw_state();
    let old_ciphertexts = f.raw_rows("object_snapshots");
    expect_error(f.commit(&view, &plan), ImportBatchError::Snapshots);
    assert_eq!(f.raw_rows("object_snapshots"), old_ciphertexts);
    assert_eq!(f.history_ids("owner").len(), 60);
    assert_eq!(f.raw_state(), before);
    f.assert_counter_rolled_back();
    f.retry_without_trigger(&plan, "rf021_fail_second");
    assert_eq!(f.history_ids("owner").len(), 2);
}

#[test]
fn rf021_same_connection_dml_invalidates_prepared_view_without_import_writes() {
    let f = Fixture::new();
    f.seed_object("owner", "old owner");
    let view = f.view();
    f.with_connection(|connection| {
        connection
            .execute(
                "UPDATE objects SET name = 'concurrent same connection' WHERE id = 'owner'",
                [],
            )
            .unwrap();
    });
    let concurrent = f.raw_state();
    let plan = ImportDatabaseBatch {
        templates: vec![template("new_template", "new template")],
        objects: vec![write(
            record("owner", "prepared overwrite"),
            ImportHistoryChange::Keep,
        )],
    };
    expect_error(f.commit(&view, &plan), ImportBatchError::StaleView);
    assert_eq!(f.raw_state(), concurrent);
    let fresh = f.view();
    f.commit(&fresh, &plan).unwrap();
    assert_eq!(
        f.vault.load_object("owner").unwrap().unwrap().name,
        "prepared overwrite"
    );
}

#[test]
fn rf021_second_connection_dml_invalidates_view_and_retains_concurrent_change() {
    let f = Fixture::new();
    f.seed_object("owner", "old owner");
    let view = f.view();
    f.db.execute(
        "UPDATE objects SET name = 'concurrent other connection' WHERE id = 'owner'",
        [],
    )
    .unwrap();
    let concurrent = f.raw_state();
    let plan = batch(vec![write(
        record("owner", "prepared overwrite"),
        ImportHistoryChange::Keep,
    )]);
    expect_error(f.commit(&view, &plan), ImportBatchError::StaleView);
    assert_eq!(f.raw_state(), concurrent);
    assert_eq!(
        f.vault.load_object("owner").unwrap().unwrap().name,
        "concurrent other connection"
    );
    let fresh = f.view();
    f.commit(&fresh, &plan).unwrap();
}

#[test]
fn rf021_other_store_revision_and_wrong_account_cannot_authorize_a_batch() {
    let first = Fixture::new();
    let second = Fixture::new();
    let view = first.view();
    let plan = batch(vec![write(
        record("new_object", "new object"),
        ImportHistoryChange::Keep,
    )]);
    let first_before = first.raw_state();
    let second_before = second.raw_state();
    expect_error(second.commit(&view, &plan), ImportBatchError::WrongStore);
    expect_error(
        first
            .vault
            .commit_import_batch("another_account", &view.revision, &plan),
        ImportBatchError::AccountMismatch,
    );
    expect_read_error(
        first.vault.read_import_view("another_account"),
        ImportBatchError::AccountMismatch,
    );
    assert_eq!(first.raw_state(), first_before);
    assert_eq!(second.raw_state(), second_before);
}

#[test]
fn rf021_locking_store_revokes_view_read_and_commit_permission() {
    let f = Fixture::new();
    f.seed_object("owner", "old owner");
    let view = f.view();
    let before = f.raw_state();
    let plan = batch(vec![write(
        record("owner", "new owner"),
        ImportHistoryChange::Keep,
    )]);
    f.vault.lock();
    expect_error(f.commit(&view, &plan), ImportBatchError::Locked);
    expect_read_error(f.vault.read_import_view(ACCOUNT), ImportBatchError::Locked);
    assert!(f.vault.load_import_view_object(&view, "owner").is_err());
    assert!(f
        .vault
        .list_import_view_active_objects(&view, &BTreeSet::new())
        .is_err());
    assert_eq!(f.raw_state(), before);
}

#[test]
fn rf021_plan_cannot_overwrite_an_existing_foreign_owner_or_write_foreign_account() {
    let f = Fixture::new();
    let mut foreign = record("foreign_owner", "foreign owner");
    foreign.account_id = "another_account".into();
    f.vault.save_object(&foreign).unwrap();
    let mut existing_foreign_template =
        template("existing_foreign_template", "foreign retained template");
    existing_foreign_template.account_id = "another_account".into();
    f.vault
        .save_user_template(&existing_foreign_template)
        .unwrap();
    let view = f.view();
    assert!(f
        .vault
        .list_import_view_object_metadata(&view)
        .unwrap()
        .iter()
        .all(|object| object.id != "foreign_owner"));
    assert!(f
        .vault
        .load_import_view_object(&view, "foreign_owner")
        .unwrap()
        .is_none());
    assert!(f
        .vault
        .list_import_view_user_templates(&view)
        .unwrap()
        .iter()
        .all(|template| template.id != "existing_foreign_template"));
    assert!(f
        .vault
        .load_import_view_user_template(&view, "existing_foreign_template")
        .unwrap()
        .is_none());
    let before = f.raw_state();
    let mut foreign_record = record("new_foreign", "foreign record");
    foreign_record.account_id = "another_account".into();
    let mut foreign_template = template("foreign_template", "foreign template");
    foreign_template.account_id = "another_account".into();
    let plans = vec![
        batch(vec![write(
            record("foreign_owner", "cannot claim this owner"),
            ImportHistoryChange::Keep,
        )]),
        batch(vec![write(foreign_record, ImportHistoryChange::Keep)]),
        ImportDatabaseBatch {
            templates: vec![foreign_template],
            objects: vec![],
        },
        ImportDatabaseBatch {
            templates: vec![template(
                "existing_foreign_template",
                "cannot claim foreign template",
            )],
            objects: vec![],
        },
    ];
    for plan in plans {
        let fresh = f.view();
        expect_error(f.commit(&fresh, &plan), ImportBatchError::AccountMismatch);
        assert_eq!(f.raw_state(), before);
    }
}

#[test]
fn rf021_new_template_plan_rejects_existing_or_duplicate_template_ids() {
    let f = Fixture::new();
    f.vault
        .save_user_template(&template("existing", "retained template"))
        .unwrap();
    let before = f.raw_state();
    let plans = vec![
        ImportDatabaseBatch {
            templates: vec![template("existing", "cannot overwrite")],
            objects: vec![write(record("new", "new"), ImportHistoryChange::Keep)],
        },
        ImportDatabaseBatch {
            templates: vec![template("duplicate", "A"), template("duplicate", "B")],
            objects: vec![],
        },
    ];
    for plan in plans {
        let fresh = f.view();
        expect_error(f.commit(&fresh, &plan), ImportBatchError::InvalidPlan);
        assert_eq!(f.raw_state(), before);
    }
}

#[test]
fn rf021_snapshot_ids_are_unique_across_owners_and_cannot_reuse_existing_history_id() {
    let f = Fixture::new();
    f.seed_object("owner_a", "old A");
    f.seed_object("owner_b", "old B");
    f.seed_history("owner_a", 1);
    let existing_id = f.history_ids("owner_a").into_iter().next().unwrap();
    let first = snapshot(1);
    let mut repeated = snapshot(2);
    repeated.id = first.id.clone();
    let mut collision = snapshot(3);
    collision.id = existing_id;
    let plans = vec![
        batch(vec![
            write(
                record("owner_a", "new A"),
                ImportHistoryChange::Append(vec![first]),
            ),
            write(
                record("owner_b", "new B"),
                ImportHistoryChange::Append(vec![repeated]),
            ),
        ]),
        batch(vec![write(
            record("owner_b", "new B"),
            ImportHistoryChange::Append(vec![collision]),
        )]),
    ];
    let before = f.raw_state();
    for plan in plans {
        let fresh = f.view();
        expect_error(f.commit(&fresh, &plan), ImportBatchError::InvalidPlan);
        assert_eq!(f.raw_state(), before);
    }
}

#[test]
fn rf021_metadata_view_ignores_bad_properties_but_strict_active_reads_reject_them() {
    let f = Fixture::new();
    f.seed_object("active", "active bad properties");
    f.seed_object("deleted", "deleted bad properties");
    f.db.execute_batch(
        "UPDATE objects SET properties = 'not a valid JSON object' WHERE id IN ('active', 'deleted');
         UPDATE objects SET is_deleted = 1, deleted_at = '2026-09-30T00:00:00Z' WHERE id = 'deleted';",
    ).unwrap();
    let view = f.view();
    let metadata = f.vault.list_import_view_object_metadata(&view).unwrap();
    assert_eq!(metadata.len(), 1);
    assert_eq!(metadata[0].id, "active");
    assert!(metadata[0].properties.is_null());
    assert!(metadata[0].property_labels.is_none());
    assert!(f.vault.load_import_view_object(&view, "active").is_err());
    assert!(f.vault.load_import_view_object(&view, "deleted").is_err());
    assert!(f
        .vault
        .list_import_view_active_objects(&view, &BTreeSet::new())
        .is_err());
    // 软删的坏记录不参与活动对象列表；排除活动坏行之后应可正常列出空列表。
    let shadowed = BTreeSet::from(["active".to_string()]);
    assert!(f
        .vault
        .list_import_view_active_objects(&view, &shadowed)
        .unwrap()
        .is_empty());
}

#[test]
fn rf021_shadowed_corrupt_active_object_does_not_block_strict_remaining_list() {
    let f = Fixture::new();
    f.seed_object("shadowed", "shadowed bad record");
    f.seed_object("retained", "valid retained record");
    f.db.execute(
        "UPDATE objects SET properties = 'corrupt synthetic data' WHERE id = 'shadowed'",
        [],
    )
    .unwrap();
    let view = f.view();
    assert!(f
        .vault
        .list_import_view_active_objects(&view, &BTreeSet::new())
        .is_err());
    let shadowed = BTreeSet::from(["shadowed".to_string()]);
    let remaining = f
        .vault
        .list_import_view_active_objects(&view, &shadowed)
        .unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, "retained");
    assert_eq!(remaining[0].name, "valid retained record");
    assert!(f
        .vault
        .load_import_view_object(&view, "missing")
        .unwrap()
        .is_none());
}

#[test]
fn rf021_bad_template_properties_are_deferred_until_strict_read_and_object_only_commit_succeeds() {
    let f = Fixture::new();
    f.vault
        .save_user_template(&template("bad_template", "retained template"))
        .unwrap();
    f.db.execute(
        "UPDATE user_templates SET properties_json = 'not template JSON' WHERE id = 'bad_template'",
        [],
    )
    .unwrap();
    let before = f.raw_state();
    let view = f.view();
    assert!(f.vault.list_import_view_user_templates(&view).is_err());
    assert!(f
        .vault
        .load_import_view_user_template(&view, "bad_template")
        .is_err());
    assert!(f
        .vault
        .load_import_view_user_template(&view, "missing")
        .unwrap()
        .is_none());
    assert_eq!(f.raw_state(), before);
    let committed = f
        .commit(
            &view,
            &batch(vec![write(
                record("object_only", "object only"),
                ImportHistoryChange::Keep,
            )]),
        )
        .unwrap();
    assert_eq!(committed.object_write_count, 1);
    assert_eq!(
        f.vault.load_object("object_only").unwrap().unwrap().name,
        "object only"
    );
}

#[test]
fn rf021_invalid_empty_snapshot_or_duplicate_snapshot_within_owner_is_zero_commit() {
    let f = Fixture::new();
    f.seed_object("owner", "old owner");
    let mut empty = snapshot(1);
    empty.data.clear();
    let first = snapshot(2);
    let mut repeated = snapshot(3);
    repeated.id = first.id.clone();
    let plans = vec![
        batch(vec![write(
            record("owner", "new owner"),
            ImportHistoryChange::Replace(vec![empty]),
        )]),
        batch(vec![write(
            record("owner", "new owner"),
            ImportHistoryChange::Replace(vec![first, repeated]),
        )]),
    ];
    let before = f.raw_state();
    for plan in plans {
        let fresh = f.view();
        expect_error(f.commit(&fresh, &plan), ImportBatchError::InvalidPlan);
        assert_eq!(f.raw_state(), before);
    }
}

#[test]
fn rf021_ordered_duplicate_objects_preserve_final_history_identity_encryption_and_hlc() {
    let f = Fixture::new();
    f.seed_object("owner", "old owner");
    f.seed_history("owner", 1);
    f.install(
        "CREATE TABLE rf021_hlc_order (
           seq INTEGER PRIMARY KEY AUTOINCREMENT, table_name TEXT, record_id TEXT,
           wall INTEGER, counter INTEGER, node TEXT
         );
         CREATE TRIGGER rf021_hlc_insert AFTER INSERT ON sync_hlc BEGIN
           INSERT INTO rf021_hlc_order(table_name, record_id, wall, counter, node)
             VALUES(NEW.table_name, NEW.record_id, NEW.wall_time_ms, NEW.counter, NEW.node_id);
         END;
         CREATE TRIGGER rf021_hlc_update AFTER UPDATE ON sync_hlc BEGIN
           INSERT INTO rf021_hlc_order(table_name, record_id, wall, counter, node)
             VALUES(NEW.table_name, NEW.record_id, NEW.wall_time_ms, NEW.counter, NEW.node_id);
         END;",
    );
    let old_history = f.history_ids("owner");
    let removed = snapshot(1);
    let removed_id = removed.id.clone();
    let second = snapshot(2);
    let second_id = second.id.clone();
    let second_data = second.data.clone();
    let third = snapshot(3);
    let third_id = third.id.clone();
    let last = snapshot(4);
    let last_id = last.id.clone();
    let mut final_record = record("owner", "final name");
    final_record.template_id = Some("new_template".into());
    final_record.version = 5;
    let plan = ImportDatabaseBatch {
        templates: vec![template("new_template", "new template")],
        objects: vec![
            write(record("owner", "first"), ImportHistoryChange::Keep),
            write(
                record("owner", "append"),
                ImportHistoryChange::Append(vec![removed]),
            ),
            write(
                record("owner", "replace"),
                ImportHistoryChange::Replace(vec![second, third]),
            ),
            write(
                record("owner", "append final"),
                ImportHistoryChange::Append(vec![last]),
            ),
            write(final_record, ImportHistoryChange::Keep),
        ],
    };
    let view = f.view();
    let previous_max: i64 =
        f.db.query_row(
            "SELECT COALESCE(MAX(wall_time_ms), 0) FROM sync_hlc",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let committed = f.commit(&view, &plan).unwrap();
    assert_eq!(committed.object_write_count, 5);
    assert_eq!(committed.object_ids, BTreeSet::from(["owner".to_string()]));
    assert_eq!(
        committed.template_ids,
        BTreeSet::from(["new_template".to_string()])
    );
    assert_eq!(committed.snapshot_write_count, 4);
    let final_ids = BTreeSet::from([second_id.clone(), third_id, last_id]);
    assert_eq!(committed.snapshot_ids, final_ids);
    assert_eq!(f.history_ids("owner"), final_ids);
    assert!(old_history.is_disjoint(&f.history_ids("owner")));
    assert!(f.vault.get_snapshot(&removed_id).unwrap().is_none());
    assert_eq!(
        f.vault.get_snapshot(&second_id).unwrap().unwrap(),
        second_data
    );
    let saved = f.vault.load_object("owner").unwrap().unwrap();
    assert_eq!(saved.name, "final name");
    assert_eq!(saved.version, 5);
    assert_eq!(saved.template_id.as_deref(), Some("new_template"));
    assert_eq!(saved.properties["secret"], "RF021 secret record data");
    assert_eq!(saved.properties["__templateName"], "new template");
    assert_eq!(
        f.vault
            .load_user_template("new_template")
            .unwrap()
            .unwrap()
            .properties[0]
            .name,
        "RF021 secret template field"
    );
    let properties: String =
        f.db.query_row(
            "SELECT properties FROM objects WHERE id = 'owner'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let template_properties: String =
        f.db.query_row(
            "SELECT properties_json FROM user_templates WHERE id = 'new_template'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let encrypted_history: Vec<u8> =
        f.db.query_row(
            "SELECT data FROM object_snapshots WHERE id = ?1",
            [&second_id],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!properties.contains("RF021 secret record data"));
    assert!(!template_properties.contains("RF021 secret template field"));
    assert_ne!(encrypted_history, second_data);
    let clocks: Vec<(i64, i64, String)> = {
        let mut statement =
            f.db.prepare("SELECT wall, counter, node FROM rf021_hlc_order ORDER BY seq")
                .unwrap();
        let rows = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap();
        rows.collect::<rusqlite::Result<_>>().unwrap()
    };
    assert_eq!(clocks.len(), 6, "一条模板和五次对象写入各自关联 HLC");
    assert!(clocks[0].0 > previous_max);
    for pair in clocks.windows(2) {
        assert!(pair[0] < pair[1], "批内 HLC 应按实际写入顺序递增");
    }
    let after = f.raw_state();
    expect_error(f.commit(&view, &plan), ImportBatchError::StaleView);
    assert_eq!(f.raw_state(), after);
}

#[test]
fn rf021_reopened_same_account_and_key_cannot_reuse_old_store_revision_or_view() {
    let f = Fixture::new();
    f.seed_object("owner", "before reopen");
    let old_view = f.view();
    let base = f.vault.base_path().to_path_buf();
    let plan = batch(vec![write(
        record("owner", "after reopen"),
        ImportHistoryChange::Keep,
    )]);
    f.vault.lock();
    let reopened =
        VaultStore::open(VaultConfig::new(ACCOUNT, base).with_data_key([0x21; 32])).unwrap();
    let before = f.raw_state();
    expect_error(
        reopened.commit_import_batch(ACCOUNT, &old_view.revision, &plan),
        ImportBatchError::WrongStore,
    );
    match reopened.load_import_view_object(&old_view, "owner") {
        Err(error) => assert_eq!(error, ImportBatchError::WrongStore.to_string()),
        Ok(_) => panic!("同账户、同路径、同密钥重开后不能读取原实例的冻结对象"),
    }
    match reopened.list_import_view_active_objects(&old_view, &BTreeSet::new()) {
        Err(error) => assert_eq!(error, ImportBatchError::WrongStore.to_string()),
        Ok(_) => panic!("重开实例不能借用原实例的冻结活动对象列表"),
    }
    assert_eq!(f.raw_state(), before);
    let fresh = reopened.read_import_view(ACCOUNT).unwrap();
    assert_eq!(
        reopened
            .load_import_view_object(&fresh, "owner")
            .unwrap()
            .unwrap()
            .name,
        "before reopen"
    );
    let committed = reopened
        .commit_import_batch(ACCOUNT, &fresh.revision, &plan)
        .unwrap();
    assert_eq!(committed.object_write_count, 1);
    assert_eq!(
        reopened.load_object("owner").unwrap().unwrap().name,
        "after reopen"
    );
}

#[test]
fn rf021_view_keeps_load_and_list_distinct_children_and_property_label_parsing_boundaries() {
    let f = Fixture::new();
    f.seed_object("owner", "retained object");
    f.db.execute(
        "UPDATE objects SET children_ids = 'not children JSON' WHERE id = 'owner'",
        [],
    )
    .unwrap();
    let children_view = f.view();
    let metadata = f
        .vault
        .list_import_view_object_metadata(&children_view)
        .unwrap();
    assert_eq!(metadata.len(), 1);
    assert_eq!(metadata[0].id, "owner");
    assert!(f
        .vault
        .load_import_view_object(&children_view, "owner")
        .is_err());
    // 原 list_objects 不解析 children_ids；冻结列表必须保留这一边界。
    let summaries = f
        .vault
        .list_import_view_active_objects(&children_view, &BTreeSet::new())
        .unwrap();
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].id, "owner");
    assert_eq!(summaries[0].name, "retained object");

    // 恢复 children_ids，单独破坏 property_labels，避免上一个错误掩盖此断言。
    f.db.execute(
        "UPDATE objects SET children_ids = '[]', property_labels = 'not labels JSON'
             WHERE id = 'owner'",
        [],
    )
    .unwrap();
    let labels_view = f.view();
    let metadata = f
        .vault
        .list_import_view_object_metadata(&labels_view)
        .unwrap();
    assert_eq!(metadata.len(), 1);
    assert_eq!(metadata[0].id, "owner");
    assert!(f
        .vault
        .load_import_view_object(&labels_view, "owner")
        .is_err());
    assert!(f
        .vault
        .list_import_view_active_objects(&labels_view, &BTreeSet::new())
        .is_err());
}

// ── RF021 按需模板视图修订（真实 SQLite，TEMP 候选未执行） ──

#[test]
fn rf021_template_view_freezes_order_values_deletion_and_later_insert() {
    let f = Fixture::new();
    let mut first = template("first", "Frozen first");
    first.created_at = "2026-09-01T00:00:00Z".into();
    let mut second = template("second", "Frozen second");
    second.created_at = "2026-09-02T00:00:00Z".into();
    f.vault.save_user_template(&second).unwrap();
    f.vault.save_user_template(&first).unwrap();
    let view = f.view();
    let frozen = f.vault.list_import_view_user_templates(&view).unwrap();
    assert_eq!(
        frozen
            .iter()
            .map(|template| template.id.as_str())
            .collect::<Vec<_>>(),
        vec!["first", "second"]
    );
    first.name = "Live changed".into();
    first.properties[0].name = "Live changed field".into();
    f.vault.save_user_template(&first).unwrap();
    f.db.execute("DELETE FROM user_templates WHERE id='second'", [])
        .unwrap();
    f.vault
        .save_user_template(&template("later", "Later"))
        .unwrap();
    let before = f.raw_state();
    let from_view = f.vault.list_import_view_user_templates(&view).unwrap();
    assert_eq!(
        serde_json::to_value(from_view).unwrap(),
        serde_json::to_value(&frozen).unwrap()
    );
    let old = f
        .vault
        .load_import_view_user_template(&view, "first")
        .unwrap()
        .unwrap();
    assert_eq!(old.name, "Frozen first");
    assert_eq!(old.properties[0].name, "RF021 secret template field");
    assert!(f
        .vault
        .load_import_view_user_template(&view, "second")
        .unwrap()
        .is_some());
    assert!(f
        .vault
        .load_import_view_user_template(&view, "later")
        .unwrap()
        .is_none());
    expect_error(
        f.commit(
            &view,
            &batch(vec![write(
                record("not_written", "stale"),
                ImportHistoryChange::Keep,
            )]),
        ),
        ImportBatchError::StaleView,
    );
    assert_eq!(f.raw_state(), before);
}

#[test]
fn rf021_template_view_frozen_error_does_not_change_after_live_repair() {
    let f = Fixture::new();
    f.vault
        .save_user_template(&template("broken", "Before"))
        .unwrap();
    f.db.execute(
        "UPDATE user_templates SET properties_json='solo:not-base64!' WHERE id='broken'",
        [],
    )
    .unwrap();
    let bad_view = f.view();
    let error = f
        .vault
        .load_import_view_user_template(&bad_view, "broken")
        .unwrap_err();
    f.vault
        .save_user_template(&template("broken", "Repaired"))
        .unwrap();
    assert_eq!(
        f.vault
            .load_import_view_user_template(&bad_view, "broken")
            .unwrap_err(),
        error
    );
    assert!(f.vault.list_import_view_user_templates(&bad_view).is_err());
    let fresh = f.view();
    assert_eq!(
        f.vault
            .load_import_view_user_template(&fresh, "broken")
            .unwrap()
            .unwrap()
            .name,
        "Repaired"
    );
}

#[test]
fn rf021_unused_bad_template_columns_including_name_do_not_block_object_only_batch() {
    for column in [
        "name",
        "icon_id",
        "category",
        "contract_type_id",
        "created_at",
        "updated_at",
    ] {
        let f = Fixture::new();
        f.vault
            .save_user_template(&template("bad_column", "Unused"))
            .unwrap();
        f.db.execute(
            &format!("UPDATE user_templates SET {column}=zeroblob(4) WHERE id='bad_column'"),
            [],
        )
        .unwrap();
        let view = f.view();
        let before = f.raw_state();
        assert!(f.vault.list_import_view_user_templates(&view).is_err());
        assert!(f
            .vault
            .load_import_view_user_template(&view, "bad_column")
            .is_err());
        assert_eq!(f.raw_state(), before);
        let committed = f
            .commit(
                &view,
                &batch(vec![write(
                    record("object_only", "No template"),
                    ImportHistoryChange::Keep,
                )]),
            )
            .unwrap();
        assert_eq!(committed.object_write_count, 1);
        assert_eq!(
            f.vault.load_object("object_only").unwrap().unwrap().name,
            "No template"
        );
        let bad: Vec<u8> =
            f.db.query_row(
                &format!("SELECT CAST({column} AS BLOB) FROM user_templates WHERE id='bad_column'"),
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(bad, vec![0; 4]);
    }
}

#[test]
fn rf021_template_accessors_reject_other_store_account_locked_and_reopened_store() {
    let first = Fixture::new();
    let second = Fixture::new();
    first
        .vault
        .save_user_template(&template("owner_template", "Owner"))
        .unwrap();
    let view = first.view();
    assert_eq!(
        second
            .vault
            .list_import_view_user_templates(&view)
            .unwrap_err(),
        ImportBatchError::WrongStore.to_string()
    );
    assert_eq!(
        second
            .vault
            .load_import_view_user_template(&view, "owner_template")
            .unwrap_err(),
        ImportBatchError::WrongStore.to_string()
    );
    let foreign_base = second._root.path().join("actual_other_account");
    std::fs::create_dir_all(&foreign_base).unwrap();
    let foreign_store = VaultStore::open(
        VaultConfig::new("actual_other_account", foreign_base).with_data_key([0x22; 32]),
    )
    .unwrap();
    let wrong_account = foreign_store
        .read_import_view("actual_other_account")
        .unwrap();
    assert_eq!(
        first
            .vault
            .list_import_view_user_templates(&wrong_account)
            .unwrap_err(),
        ImportBatchError::AccountMismatch.to_string()
    );
    assert_eq!(
        first
            .vault
            .load_import_view_user_template(&wrong_account, "owner_template")
            .unwrap_err(),
        ImportBatchError::AccountMismatch.to_string()
    );
    let base = first.vault.base_path().to_path_buf();
    first.vault.lock();
    assert_eq!(
        first
            .vault
            .list_import_view_user_templates(&view)
            .unwrap_err(),
        ImportBatchError::Locked.to_string()
    );
    assert_eq!(
        first
            .vault
            .load_import_view_user_template(&view, "owner_template")
            .unwrap_err(),
        ImportBatchError::Locked.to_string()
    );
    assert_eq!(
        first
            .vault
            .load_import_view_user_template(&view, "missing")
            .unwrap_err(),
        ImportBatchError::Locked.to_string()
    );
    let reopened =
        VaultStore::open(VaultConfig::new(ACCOUNT, base).with_data_key([0x21; 32])).unwrap();
    assert_eq!(
        reopened.list_import_view_user_templates(&view).unwrap_err(),
        ImportBatchError::WrongStore.to_string()
    );
    assert_eq!(
        reopened
            .load_import_view_user_template(&view, "owner_template")
            .unwrap_err(),
        ImportBatchError::WrongStore.to_string()
    );
    let fresh = reopened.read_import_view(ACCOUNT).unwrap();
    assert_eq!(
        reopened
            .load_import_view_user_template(&fresh, "owner_template")
            .unwrap()
            .unwrap()
            .name,
        "Owner"
    );
    let empty = second.view();
    second.vault.lock();
    assert_eq!(
        second
            .vault
            .list_import_view_user_templates(&empty)
            .unwrap_err(),
        ImportBatchError::Locked.to_string()
    );
}

#[test]
fn rf021_template_raw_mapper_preserves_decryption_priority_and_invalid_utf8() {
    for bad_name in ["zeroblob(4)", "CAST(X'80' AS TEXT)"] {
        let f = Fixture::new();
        f.vault
            .save_user_template(&template("both_bad", "Original"))
            .unwrap();
        f.db.execute(&format!("UPDATE user_templates SET name={bad_name},properties_json='solo:not-base64!' WHERE id='both_bad'"),[]).unwrap();
        let view = f.view();
        assert!(f
            .vault
            .load_user_template("both_bad")
            .unwrap_err()
            .contains("Template properties decryption failed"));
        assert!(f
            .vault
            .load_import_view_user_template(&view, "both_bad")
            .unwrap_err()
            .contains("Template properties decryption failed"));
        f.db.execute(
            "UPDATE user_templates SET properties_json='[]' WHERE id='both_bad'",
            [],
        )
        .unwrap();
        let fresh = f.view();
        let direct = f.vault.load_user_template("both_bad").unwrap_err();
        let frozen = f
            .vault
            .load_import_view_user_template(&fresh, "both_bad")
            .unwrap_err();
        assert_eq!(frozen, direct);
        assert!(!frozen.contains("Template properties decryption failed"));
        // 无 UTF-8 提前转换：即使坏 name 是 SQLite Text bytes，纯对象计划不解析它。
        assert_eq!(
            f.commit(
                &fresh,
                &batch(vec![write(
                    record("object_only", "No template"),
                    ImportHistoryChange::Keep
                )])
            )
            .unwrap()
            .object_write_count,
            1
        );
    }
}

#[test]
fn rf021_referenced_bad_template_name_keeps_old_no_fallback_and_snapshot_input() {
    let f = Fixture::new();
    f.vault
        .save_user_template(&template("bad_name", "Originally valid"))
        .unwrap();
    f.db.execute(
        "UPDATE user_templates SET name=zeroblob(4) WHERE id='bad_name'",
        [],
    )
    .unwrap();
    let view = f.view();
    assert!(f
        .vault
        .load_import_view_user_template(&view, "bad_name")
        .is_err());
    let mut imported = record("owner", "Imported");
    imported.template_id = Some("bad_name".into());
    imported.properties = serde_json::json!({"__templateName":"incoming","input":"kept"});
    let mut initial = snapshot(1);
    initial.data = serde_json::to_vec(&imported).unwrap();
    let snapshot_id = initial.id.clone();
    let committed = f
        .commit(
            &view,
            &batch(vec![write(
                imported,
                ImportHistoryChange::Append(vec![initial]),
            )]),
        )
        .unwrap();
    assert_eq!(committed.object_write_count, 1);
    assert_eq!(committed.snapshot_write_count, 1);
    assert_eq!(
        f.vault.load_object("owner").unwrap().unwrap().properties["__templateName"],
        "incoming"
    );
    let data = f.vault.get_snapshot(&snapshot_id).unwrap().unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&data).unwrap()["properties"]["__templateName"],
        "incoming"
    );
}

#[test]
fn rf021_new_batch_template_name_matches_prepared_record_and_initial_snapshot() {
    let f = Fixture::new();
    let view = f.view();
    let added = template("new_template", "New batch template");
    let mut imported = record("owner", "Imported");
    imported.template_id = Some(added.id.clone());
    // 与 Host 的成功模板继承一致：保存前 record 已写入本批模板名。
    imported.properties = serde_json::json!({"__templateName":added.name.clone()});
    let mut initial = snapshot(1);
    initial.data = serde_json::to_vec(&imported).unwrap();
    let snapshot_id = initial.id.clone();
    let committed = f
        .commit(
            &view,
            &ImportDatabaseBatch {
                templates: vec![added],
                objects: vec![write(imported, ImportHistoryChange::Append(vec![initial]))],
            },
        )
        .unwrap();
    assert_eq!(committed.template_ids.len(), 1);
    assert_eq!(committed.object_write_count, 1);
    assert_eq!(
        f.vault.load_object("owner").unwrap().unwrap().properties["__templateName"],
        "New batch template"
    );
    let data = f.vault.get_snapshot(&snapshot_id).unwrap().unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&data).unwrap()["properties"]["__templateName"],
        "New batch template"
    );
}

// ── RF021 metadata/unused typed error 兼容修订（真实 SQLite，TEMP 未执行） ──

#[test]
fn rf021_host_load_and_commit_good_ignore_unused_active_or_deleted_bad_name() {
    for deleted in [false, true] {
        let f = Fixture::new();
        f.seed_object("good", "Valid target");
        f.seed_object("unused", "Unused old name");
        f.db.execute(
            "UPDATE objects SET name=zeroblob(4),is_deleted=?1 WHERE id='unused'",
            [deleted as i32],
        )
        .unwrap();
        let view = f.view();
        let existing = f
            .vault
            .load_import_view_object(&view, "good")
            .unwrap()
            .unwrap();
        assert_eq!(existing.name, "Valid target");
        assert!(f.vault.load_import_view_object(&view, "unused").is_err());
        let metadata = f.vault.list_import_view_object_metadata(&view);
        let full = f
            .vault
            .list_import_view_active_objects(&view, &BTreeSet::new());
        if deleted {
            assert_eq!(metadata.unwrap().len(), 1);
            assert_eq!(full.unwrap().len(), 1);
        } else {
            assert!(metadata.is_err());
            assert!(full.is_err());
        }
        let original_rows = f.raw_rows("objects");
        let original_bad = original_rows
            .iter()
            .find(|row| row[0] == SqlValue::Text("unused".into()))
            .unwrap()
            .clone();
        let committed = f
            .commit(
                &view,
                &batch(vec![write(
                    record("good", "Updated valid target"),
                    ImportHistoryChange::Keep,
                )]),
            )
            .unwrap();
        assert_eq!(committed.object_write_count, 1);
        assert_eq!(
            f.vault.load_object("good").unwrap().unwrap().name,
            "Updated valid target"
        );
        assert!(
            f.raw_rows("objects").contains(&original_bad),
            "unused bad record must stay byte-identical"
        );
    }
}

#[test]
fn rf021_active_full_list_defers_bad_name_and_unmatchable_blob_id_without_hiding_error() {
    for column in ["name", "id"] {
        let f = Fixture::new();
        f.seed_object("good", "Good");
        f.seed_object("bad", "Bad");
        f.db.execute(
            &format!("UPDATE objects SET {column}=zeroblob(4) WHERE id='bad'"),
            [],
        )
        .unwrap();
        let before = f.raw_state();
        let view = f.view();
        assert_eq!(
            f.vault
                .load_import_view_object(&view, "good")
                .unwrap()
                .unwrap()
                .name,
            "Good"
        );
        let bad = f.vault.load_import_view_object(&view, "bad");
        if column == "id" {
            assert!(bad.unwrap().is_none());
        } else {
            assert!(bad.is_err());
        }
        assert!(f
            .vault
            .list_import_view_active_objects(&view, &BTreeSet::new())
            .is_err());
        let shadowed = BTreeSet::from(["bad".into()]);
        let remaining = f.vault.list_import_view_active_objects(&view, &shadowed);
        if column == "id" {
            assert!(
                remaining.is_err(),
                "unmatchable ID may not be hidden by a normal String shadow"
            );
        } else {
            let remaining = remaining.unwrap();
            assert_eq!(remaining.len(), 1);
            assert_eq!(remaining[0].id, "good");
        }
        assert_eq!(f.raw_state(), before);
    }
}

#[test]
fn rf021_active_metadata_preserves_all_reachable_typed_column_errors_without_defaulting() {
    // 第7列 is_deleted 由原 SQL =0 决定 membership；非数值坏类型不满足谓词。
    // 其余14列在 active 行上都必须保持旧严格类型错误，包含 nullable String 列。
    for column in [
        "id",
        "name",
        "type_id",
        "section_type",
        "sensitivity_level",
        "created_at",
        "updated_at",
        "template_id",
        "template_type",
        "contract_type_id",
        "template_hash",
        "ignored_template_hash",
        "icon_name",
        "parent_id",
    ] {
        let f = Fixture::new();
        f.seed_object("bad", "Original");
        // 模拟已有损坏 SQLite：只在合成第二连接关闭 CHECK，否则 template_type 的
        // system/user 约束会在装配 Blob 时拒绝，测试不能到达旧/新 metadata mapper。
        f.db.execute_batch("PRAGMA ignore_check_constraints=ON;")
            .unwrap();
        f.db.execute(
            &format!("UPDATE objects SET {column}=zeroblob(4) WHERE id='bad'"),
            [],
        )
        .unwrap();
        let direct = f
            .vault
            .list_object_metadata(ACCOUNT, None, None, false, false)
            .unwrap_err();
        let view = f.view();
        let before = f.raw_state();
        let frozen = f.vault.list_import_view_object_metadata(&view).unwrap_err();
        assert_eq!(
            frozen, direct,
            "{column}: original SQL/mapper error must be preserved"
        );
        assert_eq!(f.raw_state(), before);
        let original_rows = f.raw_rows("objects");
        assert_eq!(
            f.commit(
                &view,
                &batch(vec![write(
                    record("object_only", "Imported"),
                    ImportHistoryChange::Keep
                )])
            )
            .unwrap()
            .object_write_count,
            1
        );
        let after = f.raw_rows("objects");
        assert!(
            original_rows.iter().all(|row| after.contains(row)),
            "unused typed error must not mutate old rows"
        );
    }
}

#[test]
fn rf021_metadata_never_reads_properties_labels_children_or_unrequested_tags() {
    let f = Fixture::new();
    f.seed_object("owner", "Owner");
    f.db.execute_batch("UPDATE objects SET properties=zeroblob(4),property_labels=zeroblob(4),children_ids=zeroblob(4),tags_json=zeroblob(4) WHERE id='owner';").unwrap();
    let view = f.view();
    let direct = f
        .vault
        .list_object_metadata(ACCOUNT, None, None, false, false)
        .unwrap();
    let frozen = f.vault.list_import_view_object_metadata(&view).unwrap();
    assert_eq!(
        serde_json::to_value(&frozen).unwrap(),
        serde_json::to_value(direct).unwrap()
    );
    assert_eq!(frozen.len(), 1);
    assert_eq!(frozen[0].id, "owner");
    assert!(frozen[0].properties.is_null());
    assert!(frozen[0].property_labels.is_none());
    assert!(frozen[0].tags.is_empty());
    assert!(!frozen[0].has_attachments);
    assert!(frozen[0].sensitivity_levels.is_empty());
    assert!(f.vault.load_import_view_object(&view, "owner").is_err());
    assert!(f
        .vault
        .list_import_view_active_objects(&view, &BTreeSet::new())
        .is_err());
}

#[test]
fn rf021_metadata_original_active_sql_excludes_deleted_bad_rows_and_bad_is_deleted_type() {
    let f = Fixture::new();
    f.seed_object("active", "Active");
    f.seed_object("deleted", "Deleted");
    f.seed_object("non_numeric_flag", "Flag");
    f.db.execute_batch(
        "UPDATE objects SET name=zeroblob(4),is_deleted=1 WHERE id='deleted';
        UPDATE objects SET name=zeroblob(4),is_deleted=zeroblob(4) WHERE id='non_numeric_flag';",
    )
    .unwrap();
    let view = f.view();
    let direct = f
        .vault
        .list_object_metadata(ACCOUNT, None, None, false, false)
        .unwrap();
    let frozen = f.vault.list_import_view_object_metadata(&view).unwrap();
    assert_eq!(
        serde_json::to_value(&frozen).unwrap(),
        serde_json::to_value(direct).unwrap()
    );
    assert_eq!(frozen.len(), 1);
    assert_eq!(frozen[0].id, "active");
    assert!(f.vault.load_import_view_object(&view, "deleted").is_err());
    assert!(f
        .vault
        .load_import_view_object(&view, "non_numeric_flag")
        .is_err());
}

#[test]
fn rf021_metadata_freezes_errors_after_live_repair_and_rejects_stale_commit() {
    let f = Fixture::new();
    f.seed_object("owner", "Original");
    f.db.execute(
        "UPDATE objects SET icon_name=zeroblob(4) WHERE id='owner'",
        [],
    )
    .unwrap();
    let view = f.view();
    let error = f.vault.list_import_view_object_metadata(&view).unwrap_err();
    f.db.execute(
        "UPDATE objects SET icon_name='repaired' WHERE id='owner'",
        [],
    )
    .unwrap();
    let before = f.raw_state();
    assert_eq!(
        f.vault.list_import_view_object_metadata(&view).unwrap_err(),
        error
    );
    let fresh = f.view();
    assert_eq!(
        f.vault.list_import_view_object_metadata(&fresh).unwrap()[0].icon_name,
        "repaired"
    );
    expect_error(
        f.commit(
            &view,
            &batch(vec![write(
                record("owner", "Stale"),
                ImportHistoryChange::Keep,
            )]),
        ),
        ImportBatchError::StaleView,
    );
    assert_eq!(f.raw_state(), before);
}

#[test]
fn rf021_metadata_freezes_sort_values_and_active_membership_across_live_changes() {
    let f = Fixture::new();
    f.seed_object("a", "A");
    f.seed_object("b", "B");
    let view = f.view();
    let frozen = f.vault.list_import_view_object_metadata(&view).unwrap();
    assert_eq!(
        frozen
            .iter()
            .map(|summary| summary.id.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b"]
    );
    f.db.execute_batch("UPDATE objects SET name='Changed',is_deleted=1 WHERE id='a'; DELETE FROM objects WHERE id='b';").unwrap();
    f.seed_object("later", "Later");
    assert_eq!(
        serde_json::to_value(f.vault.list_import_view_object_metadata(&view).unwrap()).unwrap(),
        serde_json::to_value(&frozen).unwrap()
    );
    let fresh = f.view();
    let current = f.vault.list_import_view_object_metadata(&fresh).unwrap();
    assert_eq!(current.len(), 1);
    assert_eq!(current[0].id, "later");
}

#[test]
fn rf021_metadata_accessor_rejects_other_store_account_locked_empty_and_reopened_view() {
    let first = Fixture::new();
    let second = Fixture::new();
    let view = first.view();
    assert_eq!(
        second
            .vault
            .list_import_view_object_metadata(&view)
            .unwrap_err(),
        ImportBatchError::WrongStore.to_string()
    );
    let foreign_base = second._root.path().join("actual_metadata_other_account");
    std::fs::create_dir_all(&foreign_base).unwrap();
    let foreign = VaultStore::open(
        VaultConfig::new("actual_metadata_other_account", foreign_base).with_data_key([0x22; 32]),
    )
    .unwrap();
    let foreign_view = foreign
        .read_import_view("actual_metadata_other_account")
        .unwrap();
    assert_eq!(
        first
            .vault
            .list_import_view_object_metadata(&foreign_view)
            .unwrap_err(),
        ImportBatchError::AccountMismatch.to_string()
    );
    let base = first.vault.base_path().to_path_buf();
    first.vault.lock();
    assert_eq!(
        first
            .vault
            .list_import_view_object_metadata(&view)
            .unwrap_err(),
        ImportBatchError::Locked.to_string()
    );
    let reopened =
        VaultStore::open(VaultConfig::new(ACCOUNT, base).with_data_key([0x21; 32])).unwrap();
    assert_eq!(
        reopened
            .list_import_view_object_metadata(&view)
            .unwrap_err(),
        ImportBatchError::WrongStore.to_string()
    );
    assert!(reopened
        .list_import_view_object_metadata(&reopened.read_import_view(ACCOUNT).unwrap())
        .unwrap()
        .is_empty());
}

#[test]
fn rf021_shared_metadata_mapper_preserves_optional_tags_filter_and_deleted_query() {
    let f = Fixture::new();
    f.seed_object("active", "Active");
    f.seed_object("deleted", "Deleted");
    f.db.execute_batch(
        "UPDATE objects SET tags_json='[\"alpha\",\"beta\"]' WHERE id='active';
        UPDATE objects SET tags_json='bad tags',is_deleted=1 WHERE id='deleted';",
    )
    .unwrap();
    let active = f
        .vault
        .list_object_metadata_with_tags(ACCOUNT, Some("note"), None, false, false)
        .unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].tags, vec!["alpha", "beta"]);
    let deleted = f
        .vault
        .list_object_metadata_with_tags(ACCOUNT, None, None, false, true)
        .unwrap();
    assert_eq!(deleted.len(), 1);
    assert_eq!(deleted[0].id, "deleted");
    assert!(deleted[0].tags.is_empty());
    let view = f.view();
    assert!(f.vault.list_import_view_object_metadata(&view).unwrap()[0]
        .tags
        .is_empty());
}

// RF-021：空批不得新增 node/HLC 读取依赖；非空批仍严格拒绝坏 HLC。
fn rf021_fixture_with_bad_local_hlc() -> Fixture {
    let f = Fixture::new();
    let node = "0123456789abcdef0123456789abcdef";
    f.vault.set_sync_node_id(node).unwrap();
    f.vault
        .save_user_template(&template("empty_hlc_template", "old template"))
        .unwrap();
    f.seed_object("empty_hlc_owner", "old owner");
    f.seed_history("empty_hlc_owner", 2);
    assert_eq!(f.vault.get_sync_node_id().unwrap().as_deref(), Some(node));
    let actual_node: String =
        f.db.query_row(
            "SELECT node_id FROM sync_hlc
             WHERE table_name = 'objects' AND record_id = 'empty_hlc_owner'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(actual_node, node, "注入必须命中真实本地节点");
    f.db.execute(
        "INSERT INTO sync_hlc
             (table_name, record_id, wall_time_ms, counter, node_id, updated_at)
             VALUES ('objects', 'empty_hlc_unused', X'00', 0, ?1, ?2)",
        params![node, NOW],
    )
    .unwrap();
    let actual_max: rusqlite::Result<i64> = f.db.query_row(
        "SELECT COALESCE(MAX(wall_time_ms), 0) FROM sync_hlc WHERE node_id = ?1",
        [node],
        |row| row.get(0),
    );
    assert!(
        matches!(
            actual_max,
            Err(rusqlite::Error::InvalidColumnType(
                0,
                _,
                rusqlite::types::Type::Blob
            ))
        ),
        "真实 SQLite 同节点 MAX 必须返回 BLOB 并使 i64 读取失败"
    );
    f
}

#[test]
fn rf021_empty_batch_bad_local_hlc_succeeds_without_changing_four_tables() {
    let f = rf021_fixture_with_bad_local_hlc();
    let view = f.view();
    let before = f.raw_state();
    assert!(before.iter().all(|rows| !rows.is_empty()));
    let committed = f
        .commit(&view, &ImportDatabaseBatch::default())
        .expect("没有模板/对象写入的批次不应读取无关坏 HLC");
    assert_eq!(committed, ImportDatabaseCommit::default());
    assert_eq!(f.raw_state(), before, "四张业务表全部原始值必须保留");
    f.assert_connection_reusable();
}

#[test]
fn rf021_nonempty_batch_bad_local_hlc_still_rejects_without_any_write() {
    let f = rf021_fixture_with_bad_local_hlc();
    let view = f.view();
    let before = f.raw_state();
    let plan = ImportDatabaseBatch {
        templates: vec![template("new_hlc_template", "new template")],
        objects: vec![write(
            record("new_hlc_object", "new object"),
            ImportHistoryChange::Append(vec![snapshot(1)]),
        )],
    };
    expect_error(f.commit(&view, &plan), ImportBatchError::Hlc);
    assert_eq!(f.raw_state(), before, "模板/对象/历史/HLC不能部分写入");
    f.assert_connection_reusable();
}

#[test]
fn rf021_empty_batch_bad_local_hlc_retains_revision_account_store_and_lock_guards() {
    let empty = ImportDatabaseBatch::default();
    {
        let f = rf021_fixture_with_bad_local_hlc();
        let stale = f.view();
        f.db.execute(
            "UPDATE objects SET name = 'concurrent external edit'
                 WHERE id = 'empty_hlc_owner'",
            [],
        )
        .unwrap();
        let concurrent = f.raw_state();
        expect_error(f.commit(&stale, &empty), ImportBatchError::StaleView);
        assert_eq!(f.raw_state(), concurrent);
        f.assert_connection_reusable();
    }
    {
        let first = rf021_fixture_with_bad_local_hlc();
        let second = rf021_fixture_with_bad_local_hlc();
        let view = first.view();
        let first_before = first.raw_state();
        let second_before = second.raw_state();
        expect_error(second.commit(&view, &empty), ImportBatchError::WrongStore);
        expect_error(
            first
                .vault
                .commit_import_batch("another_account", &view.revision, &empty),
            ImportBatchError::AccountMismatch,
        );
        assert_eq!(first.raw_state(), first_before);
        assert_eq!(second.raw_state(), second_before);
        first.assert_connection_reusable();
        second.assert_connection_reusable();
    }
    {
        let f = rf021_fixture_with_bad_local_hlc();
        let view = f.view();
        let before = f.raw_state();
        f.vault.lock();
        expect_error(f.commit(&view, &empty), ImportBatchError::Locked);
        assert_eq!(f.raw_state(), before);
    }
}
