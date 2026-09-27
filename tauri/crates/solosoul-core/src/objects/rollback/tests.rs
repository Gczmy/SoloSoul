use super::*;
use rusqlite::{types::ValueRef, Connection};
use serde_json::{json, Value};
use solosoul_vault::VaultConfig;
use tempfile::TempDir;

// 所有数据、密钥和故障只存在于本测试新建的临时 Vault。
struct Fixture {
    vault: VaultStore,
    db: Connection,
    _dir: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let vault = VaultStore::open(
            VaultConfig::new("rf008-account", dir.path().to_path_buf()).with_data_key([0x28; 32]),
        )
        .unwrap();
        let db = Connection::open(dir.path().join("vault.db")).unwrap();
        let fixture = Self {
            vault,
            db,
            _dir: dir,
        };
        fixture.vault.save_object(&record("object-a")).unwrap();
        fixture.vault.save_object(&record("object-b")).unwrap();
        fixture
    }

    fn snapshot(&self, owner: &str, value: &Value) -> String {
        self.snapshot_bytes(owner, &serde_json::to_vec(value).unwrap())
    }

    fn snapshot_bytes(&self, owner: &str, bytes: &[u8]) -> String {
        self.vault
            .save_snapshot_at(owner, "synthetic", bytes, "old", 1_700_000_000_000)
            .unwrap();
        self.db
            .query_row(
                "SELECT id FROM object_snapshots WHERE object_id = ?1 ORDER BY rowid DESC LIMIT 1",
                [owner],
                |row| row.get(0),
            )
            .unwrap()
    }

    fn dump(&self, table: &str) -> Value {
        // 表名全部由本文件固定给出；保留密文原字节可发现任何意外重写。
        let mut statement = self
            .db
            .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
            .unwrap();
        let columns = statement.column_count();
        let rows = statement
            .query_map([], |row| {
                (0..columns)
                    .map(|index| {
                        Ok(match row.get_ref(index)? {
                            ValueRef::Null => Value::Null,
                            ValueRef::Integer(value) => json!(value),
                            ValueRef::Real(value) => json!(value),
                            ValueRef::Text(value) => json!(String::from_utf8_lossy(value)),
                            ValueRef::Blob(value) => json!(value),
                        })
                    })
                    .collect::<rusqlite::Result<Vec<_>>>()
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        json!(rows)
    }

    fn state(&self) -> Value {
        json!({
            "objects": self.dump("objects"),
            "hlc": self.dump("sync_hlc"),
            "history": self.dump("object_snapshots"),
            "audit": self.dump("audit_log"),
            "config": self.dump("sys_config"),
            "historyCounts": self.vault.count_snapshots_batch(&[
                "object-a".into(), "object-b".into(), "missing-target".into(), String::new(),
            ]).unwrap(),
        })
    }

    fn history_count(&self) -> usize {
        self.db
            .query_row("SELECT count(*) FROM object_snapshots", [], |row| {
                row.get(0)
            })
            .unwrap()
    }

    fn audit_count(&self) -> usize {
        self.db
            .query_row("SELECT count(*) FROM audit_log", [], |row| row.get(0))
            .unwrap()
    }

    fn fail_history(&self) {
        self.db.execute_batch("CREATE TRIGGER rf008_history_failure BEFORE INSERT ON object_snapshots
            WHEN NEW.triggered_by = 'rollback' BEGIN SELECT RAISE(ABORT, 'synthetic history failure'); END;").unwrap();
    }

    fn fail_audit(&self) {
        self.db.execute_batch("CREATE TRIGGER rf008_audit_failure BEFORE INSERT ON audit_log
            WHEN NEW.action = 'object_rollback' BEGIN SELECT RAISE(ABORT, 'synthetic audit failure'); END;").unwrap();
    }

    fn assert_rejected(
        &self,
        object: &str,
        snapshot: &str,
        stage: RollbackErrorStage,
    ) -> RollbackError {
        let before = self.state();
        let error = rollback_object(&self.vault, object, snapshot).unwrap_err();
        assert_eq!(error.stage, stage);
        assert_eq!(
            self.state(),
            before,
            "rejection must not write any object, HLC, history or audit"
        );
        error
    }
}

fn record(id: &str) -> ObjectRecord {
    ObjectRecord {
        id: id.into(),
        account_id: "rf008-account".into(),
        type_id: "identity-card".into(),
        section_type: "original-section".into(),
        name: "当前合成对象".into(),
        icon_name: "original-icon".into(),
        parent_id: Some("original-parent".into()),
        children_ids: vec!["original-child".into()],
        properties: json!({"title": "current", "__fields": {"title": {"name": "当前字段", "type": "text"}}}),
        property_labels: Some(json!({"title": "critical", "currentOnly": "sensitive"})),
        sensitivity_level: "sensitive".into(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec!["current-tag".into()],
        template_id: Some("already-deleted-template".into()),
        template_type: Some("user".into()),
        contract_type_id: Some("original-contract".into()),
        template_hash: Some("original-template-hash".into()),
        ignored_template_hash: Some("original-ignore-hash".into()),
        created_at: "2024-01-02T03:04:05Z".into(),
        updated_at: "2025-01-02T03:04:05Z".into(),
        version: 41,
    }
}

fn restored_input() -> Value {
    json!({
        "name": "恢复的合成对象 🌙", "tags": ["first", 7, false, "second"],
        "properties": {"title": "restored", "__fields": {"title": {"name": "历史字段", "type": "text"}}},
        "propertyLabels": {"title": "internal"},
    })
}

#[test]
fn rf008_object_or_hlc_failure_rolls_back_every_write_and_connection_recovers() {
    for table in ["objects", "sync_hlc"] {
        let fixture = Fixture::new();
        let source = fixture.snapshot("object-a", &restored_input());
        fixture
            .db
            .execute_batch(&format!(
                "CREATE TRIGGER rf008_save_failure BEFORE INSERT ON {table}
             BEGIN SELECT RAISE(ABORT, 'synthetic object save failure'); END;"
            ))
            .unwrap();
        let error = fixture.assert_rejected("object-a", &source, RollbackErrorStage::ObjectSave);
        assert!(error.message.contains("synthetic object save failure"));
        fixture
            .db
            .execute_batch("DROP TRIGGER rf008_save_failure")
            .unwrap();
        let outcome = rollback_object(&fixture.vault, "object-a", &source).unwrap();
        assert!(outcome.snapshot_error.is_none() && outcome.audit_error.is_none());
        assert_eq!(
            fixture
                .vault
                .load_object("object-a")
                .unwrap()
                .unwrap()
                .version,
            42
        );
    }
}

#[test]
fn rf008_history_failure_still_audits_the_saved_object() {
    let fixture = Fixture::new();
    let source = fixture.snapshot("object-a", &restored_input());
    let history = fixture.history_count();
    let audit = fixture.audit_count();
    fixture.fail_history();
    let outcome = rollback_object(&fixture.vault, "object-a", &source).unwrap();
    assert!(outcome
        .snapshot_error
        .as_deref()
        .unwrap()
        .contains("synthetic history failure"));
    assert!(outcome.audit_error.is_none());
    assert_eq!(outcome.record.version, 42);
    assert_eq!(
        fixture.vault.load_object("object-a").unwrap().unwrap().name,
        "恢复的合成对象 🌙"
    );
    assert_eq!(fixture.history_count(), history);
    assert_eq!(fixture.audit_count(), audit + 1);
    assert_eq!(
        fixture.vault.list_audit_log(1).unwrap()[0].action_type,
        "object_rollback"
    );
}

#[test]
fn rf008_audit_failure_keeps_saved_object_and_restored_history() {
    let fixture = Fixture::new();
    let source = fixture.snapshot("object-a", &restored_input());
    let history = fixture.history_count();
    let audit = fixture.audit_count();
    fixture.fail_audit();
    let outcome = rollback_object(&fixture.vault, "object-a", &source).unwrap();
    assert!(outcome.snapshot_error.is_none());
    assert!(outcome
        .audit_error
        .as_deref()
        .unwrap()
        .contains("synthetic audit failure"));
    assert_eq!(
        fixture
            .vault
            .load_object("object-a")
            .unwrap()
            .unwrap()
            .version,
        42
    );
    assert_eq!(fixture.history_count(), history + 1);
    assert_eq!(fixture.audit_count(), audit);
    let generated_id: String = fixture
        .db
        .query_row(
            "SELECT id FROM object_snapshots WHERE triggered_by = 'rollback'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let generated: Value =
        serde_json::from_slice(&fixture.vault.get_snapshot(&generated_id).unwrap().unwrap())
            .unwrap();
    assert_eq!(generated["name"], "恢复的合成对象 🌙");
    assert_eq!(generated["propertyLabels"], json!({"title": "internal"}));
}

#[test]
fn rf008_reports_both_independent_post_save_failures() {
    let fixture = Fixture::new();
    let source = fixture.snapshot("object-a", &restored_input());
    let history = fixture.history_count();
    let audit = fixture.audit_count();
    fixture.fail_history();
    fixture.fail_audit();
    let outcome = rollback_object(&fixture.vault, "object-a", &source).unwrap();
    assert!(outcome
        .snapshot_error
        .as_deref()
        .unwrap()
        .contains("synthetic history failure"));
    assert!(outcome
        .audit_error
        .as_deref()
        .unwrap()
        .contains("synthetic audit failure"));
    assert_eq!(
        fixture
            .vault
            .load_object("object-a")
            .unwrap()
            .unwrap()
            .version,
        42
    );
    assert_eq!(fixture.history_count(), history);
    assert_eq!(fixture.audit_count(), audit);
}

#[test]
fn rf008_invalid_owner_missing_data_bad_json_and_version_overflow_are_zero_write() {
    let fixture = Fixture::new();
    let error = fixture.assert_rejected(
        "object-a",
        "no-snapshot",
        RollbackErrorStage::SnapshotNotFound,
    );
    assert_eq!(error.to_string(), "Snapshot not found");
    let foreign = fixture.snapshot(
        "object-b",
        &json!({"objectId": "object-a", "name": "forged-owner"}),
    );
    let error = fixture.assert_rejected("object-a", &foreign, RollbackErrorStage::Ownership);
    assert_eq!(error.to_string(), "Snapshot does not belong to object");
    let empty_owner = fixture.snapshot("", &json!({"objectId": "object-a"}));
    fixture.assert_rejected("", &empty_owner, RollbackErrorStage::Ownership);
    let missing_target = fixture.snapshot("missing-target", &restored_input());
    let error = fixture.assert_rejected(
        "missing-target",
        &missing_target,
        RollbackErrorStage::ObjectNotFound,
    );
    assert_eq!(error.to_string(), "Object not found");
    let malformed = fixture.snapshot_bytes("object-a", b"{invalid synthetic JSON");
    let error = fixture.assert_rejected("object-a", &malformed, RollbackErrorStage::SnapshotParse);
    assert!(error.to_string().starts_with("Parse: "));
    let mut maximum = record("object-a");
    maximum.version = u32::MAX;
    fixture.vault.save_object(&maximum).unwrap();
    let source = fixture.snapshot("object-a", &restored_input());
    fixture.assert_rejected("object-a", &source, RollbackErrorStage::Version);
}

#[test]
fn rf008_malformed_labels_reject_before_writes_without_alias_fallback() {
    let fixture = Fixture::new();
    for (key, value) in [
        ("propertyLabels", json!([])),
        ("property_labels", json!("bad")),
        ("propertyLabels", json!(false)),
        ("property_labels", json!(7)),
    ] {
        let mut input = restored_input();
        input.as_object_mut().unwrap().remove("propertyLabels");
        input[key] = value;
        if key == "propertyLabels" {
            input["property_labels"] = json!({"title": "public"});
        }
        let source = fixture.snapshot("object-a", &input);
        fixture.assert_rejected("object-a", &source, RollbackErrorStage::Labels);
    }
}

#[test]
fn rf008_restores_both_label_formats_preserves_other_fields_and_writes_one_history_and_audit() {
    for key in ["propertyLabels", "property_labels"] {
        let fixture = Fixture::new();
        let original = fixture.vault.load_object("object-a").unwrap().unwrap();
        let other = serde_json::to_value(fixture.vault.load_object("object-b").unwrap()).unwrap();
        let mut input = restored_input();
        let labels = input
            .as_object_mut()
            .unwrap()
            .remove("propertyLabels")
            .unwrap();
        input[key] = labels;
        // 快照中额外的旧字段不属于本次恢复范围。
        input["sectionType"] = json!("forged-section");
        input["id"] = json!("object-b");
        input["version"] = json!(3);
        input["isDeleted"] = json!(true);
        let source = fixture.snapshot("object-a", &input);
        let history = fixture.history_count();
        let audit = fixture.audit_count();
        let outcome = rollback_object(&fixture.vault, "object-a", &source).unwrap();
        assert!(outcome.snapshot_error.is_none() && outcome.audit_error.is_none());
        let actual = fixture.vault.load_object("object-a").unwrap().unwrap();
        assert_eq!(
            serde_json::to_value(&outcome.record).unwrap(),
            serde_json::to_value(&actual).unwrap()
        );
        assert_eq!(actual.name, "恢复的合成对象 🌙");
        assert_eq!(actual.tags_json, ["first", "second"]);
        assert_eq!(actual.properties, input["properties"]);
        assert_eq!(actual.property_labels, Some(json!({"title": "internal"})));
        assert_eq!(actual.version, original.version + 1);
        assert_ne!(actual.updated_at, original.updated_at);
        chrono::DateTime::parse_from_rfc3339(&actual.updated_at).unwrap();
        let mut expected = original;
        expected.name = actual.name.clone();
        expected.tags_json = actual.tags_json.clone();
        expected.properties = actual.properties.clone();
        expected.property_labels = actual.property_labels.clone();
        expected.version = actual.version;
        expected.updated_at = actual.updated_at.clone();
        assert_eq!(
            serde_json::to_value(&actual).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
        assert_eq!(
            serde_json::to_value(fixture.vault.load_object("object-b").unwrap()).unwrap(),
            other
        );
        assert_eq!(fixture.history_count(), history + 1);
        assert_eq!(fixture.audit_count(), audit + 1);
        let entry = fixture
            .vault
            .list_snapshots("object-a")
            .unwrap()
            .into_iter()
            .find(|entry| entry["triggeredBy"] == "rollback")
            .unwrap();
        assert_eq!(entry["diffSummary"], "diff_rollback");
        let generated_id = entry["id"].as_str().unwrap();
        let generated: Value =
            serde_json::from_slice(&fixture.vault.get_snapshot(generated_id).unwrap().unwrap())
                .unwrap();
        assert_eq!(
            generated,
            json!({"name": actual.name, "tags": ["first", "second"], "properties": actual.properties, "propertyLabels": {"title": "internal"}})
        );
        let log = fixture.vault.list_audit_log(1).unwrap().remove(0);
        assert_eq!(log.action_type, "object_rollback");
        assert_eq!(log.entity_type, "object");
        assert_eq!(log.entity_id.as_deref(), Some("object-a"));
        assert_eq!(log.entity_name.as_deref(), Some("恢复的合成对象 🌙"));
        assert_eq!(log.performed_by, "user");
        assert_eq!(
            serde_json::from_str::<Value>(log.details.as_deref().unwrap()).unwrap(),
            json!({"section": "original-section", "snapshot": source})
        );
        let second = rollback_object(&fixture.vault, "object-a", generated_id).unwrap();
        assert_eq!(
            second.record.property_labels,
            Some(json!({"title": "internal"}))
        );
        assert_eq!(second.record.version, 43);
    }
}

#[test]
fn rf008_missing_null_empty_and_camel_priority_labels_keep_legacy_field_semantics() {
    for (input, labels) in [
        (
            json!({}),
            Some(json!({"title": "critical", "currentOnly": "sensitive"})),
        ),
        (
            json!({"propertyLabels": null, "property_labels": {"title": "public"}}),
            None,
        ),
        (json!({"property_labels": null}), None),
        (json!({"propertyLabels": {}}), Some(json!({}))),
        (
            json!({"propertyLabels": {"title": "sensitive"}, "property_labels": false}),
            Some(json!({"title": "sensitive"})),
        ),
    ] {
        let fixture = Fixture::new();
        let mut input = input;
        input["name"] = json!(false);
        input["tags"] = json!(null);
        input["properties"] = json!(null);
        let source = fixture.snapshot("object-a", &input);
        let outcome = rollback_object(&fixture.vault, "object-a", &source).unwrap();
        assert_eq!(outcome.record.property_labels, labels);
        assert_eq!(outcome.record.name, record("object-a").name);
        assert_eq!(outcome.record.tags_json, record("object-a").tags_json);
        assert_eq!(outcome.record.properties, record("object-a").properties);
        let generated = fixture
            .vault
            .list_snapshots("object-a")
            .unwrap()
            .into_iter()
            .find(|entry| entry["triggeredBy"] == "rollback")
            .unwrap();
        let bytes = fixture
            .vault
            .get_snapshot(generated["id"].as_str().unwrap())
            .unwrap()
            .unwrap();
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            value.get("propertyLabels"),
            Some(&labels.unwrap_or(Value::Null))
        );
    }
    // 既有 properties 恢复只区分 null，不因共享用例额外收紧历史格式。
    let fixture = Fixture::new();
    let source = fixture.snapshot("object-a", &json!({"properties": ["legacy", 7]}));
    assert_eq!(
        rollback_object(&fixture.vault, "object-a", &source)
            .unwrap()
            .record
            .properties,
        json!(["legacy", 7])
    );
}
