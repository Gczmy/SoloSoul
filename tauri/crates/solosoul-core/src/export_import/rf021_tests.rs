//! RF-021 Core 真实 ZIP/SQLite 接线候选；仅 TEMP，尚未执行。
use super::*;
use rusqlite::{types::Value as SqlValue, Connection};
use serde_json::{json, Value};
use std::sync::Arc;

const PASSWORD: &str = "rf021-vault-password";
const PACKAGE_PASSWORD: &str = "RF021-Export-Password1";
const NOW: &str = "2026-09-30T12:00:00Z";

struct Fixture {
    vault: Arc<VaultStore>,
    service: crate::VaultService,
    db: Connection,
    account_id: String,
    dir: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::TempDir::new().unwrap();
        let service = crate::VaultService::with_base_path(dir.path().to_path_buf());
        let account = service.create_account("RF021", PASSWORD, None).unwrap();
        let account_id = account["id"].as_str().unwrap().to_string();
        let vault = service.get_vault_store().unwrap();
        let db = Connection::open(dir.path().join(&account_id).join("vault.db")).unwrap();
        Self {
            vault,
            service,
            db,
            account_id,
            dir,
        }
    }

    fn record(&self, id: &str, name: &str) -> ObjectRecord {
        let value = json!({"objects": [object(id, name)]});
        let (mut records, _, _) = build_import_records(
            &self.account_id,
            &value,
            &HashMap::new(),
            &HashSet::new(),
            &HashSet::new(),
            ImportStrategy::Overwrite,
            NOW,
        );
        records.remove(0)
    }

    fn seed(&self, id: &str, name: &str) {
        self.vault.save_object(&self.record(id, name)).unwrap();
    }

    fn package(&self, payload: &Value, extra: &[(&str, Vec<u8>)], attachments: bool) -> PathBuf {
        let path = self
            .dir
            .path()
            .join(format!("{}.solosoul", uuid::Uuid::new_v4()));
        let salt = solosoul_crypto::kdf::generate_salt();
        // 显式测试级参数，避免依赖进程全局环境。
        let cfg = KdfConfig::development();
        let key = derive_export_key_cfg(PACKAGE_PASSWORD, &salt, &cfg).unwrap();
        let manifest = json!({
            "version": "2.0", "salt_hex": hex::encode(salt),
            "has_attachments": attachments, "has_templates": true,
            "object_count": payload["objects"].as_array().map_or(0, Vec::len),
            "extra_files": extra.iter().filter(|(name, _)| *name == "preferences.enc")
                .map(|(name, _)| *name).collect::<Vec<_>>(),
            "kdf": kdf_to_manifest_value(&cfg),
        });
        let mut zip = ZipWriter::new(File::create(&path).unwrap());
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("manifest.json", options).unwrap();
        zip.write_all(manifest.to_string().as_bytes()).unwrap();
        zip.start_file("payload.enc", options).unwrap();
        let bytes = serde_json::to_vec(payload).unwrap();
        solosoul_crypto::cipher::encrypt_chunked_stream(
            &key,
            bytes.len() as u64,
            &mut std::io::Cursor::new(bytes),
            &mut zip,
        )
        .unwrap();
        for (name, bytes) in extra {
            zip.start_file(*name, options).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
        path
    }

    fn import(&self, payload: &Value, strategy: ImportStrategy) -> Result<usize, ExportError> {
        let path = self.package(payload, &[], false);
        import_vault(
            &self.vault,
            &self.account_id,
            &path,
            PACKAGE_PASSWORD,
            strategy,
            self.dir.path(),
            None,
        )
    }

    fn raw_table(&self, table: &str) -> Vec<Vec<SqlValue>> {
        // 表名只有测试中的固定白名单，扫描全部行/列，不使用 LIMIT 50。
        assert!(matches!(
            table,
            "objects" | "user_templates" | "object_snapshots" | "sync_hlc"
        ));
        let mut statement = self
            .db
            .prepare(&format!("SELECT * FROM {table} ORDER BY 1, 2"))
            .unwrap();
        let columns = statement.column_count();
        let rows = statement
            .query_map([], |row| {
                (0..columns)
                    .map(|i| row.get(i))
                    .collect::<rusqlite::Result<Vec<SqlValue>>>()
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        rows
    }

    fn raw_database(&self) -> Vec<Vec<Vec<SqlValue>>> {
        ["objects", "user_templates", "object_snapshots", "sync_hlc"]
            .into_iter()
            .map(|table| self.raw_table(table))
            .collect()
    }

    fn fail_nth(&self, table: &str, event: &str, condition: &str, nth: usize) {
        assert!(matches!(table, "objects" | "user_templates" | "sync_hlc"));
        assert!(matches!(event, "INSERT" | "UPDATE"));
        self.db
            .execute_batch(&format!(
                "CREATE TABLE rf021_counter(n INTEGER NOT NULL);
             INSERT INTO rf021_counter VALUES(0);
             CREATE TRIGGER rf021_failure BEFORE {event} ON {table} {condition} BEGIN
               UPDATE rf021_counter SET n=n+1;
               SELECT CASE WHEN (SELECT n FROM rf021_counter)={nth}
                 THEN RAISE(ABORT,'rf021_real_sql_failure') END;
             END;"
            ))
            .unwrap();
    }

    fn assert_counter_rolled_back(&self) {
        let n: i64 = self
            .db
            .query_row("SELECT n FROM rf021_counter", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            n, 0,
            "trigger side effects must roll back with the full batch"
        );
    }

    fn remove_failure(&self) {
        self.db
            .execute_batch("DROP TRIGGER rf021_failure; DROP TABLE rf021_counter;")
            .unwrap();
    }
}

fn object(id: &str, name: &str) -> Value {
    json!({
        "id": id, "name": name, "type_id": "note", "section_type": "identity",
        "properties": {"value": name}, "sensitivity_level": "internal", "version": 7,
        "created_at": NOW, "tags": [],
    })
}

fn template(id: &str, name: &str) -> Value {
    json!({
        "id": id, "accountId": "source-account", "name": name, "iconId": null,
        "properties": [], "category": null, "contractTypeId": null,
        "createdAt": NOW, "updatedAt": null,
    })
}

fn batch_payload() -> Value {
    let mut one = object("rf021-one", "One");
    let mut two = object("rf021-two", "Two");
    one["template_id"] = json!("source-one");
    two["template_id"] = json!("source-two");
    json!({
        "templates": [template("source-one", "RF021 Template One"), template("source-two", "RF021 Template Two")],
        "objects": [one, two],
    })
}

#[test]
fn rf021_core_second_template_failure_rolls_back_all_tables_and_retries() {
    let f = Fixture::new();
    f.fail_nth(
        "user_templates",
        "INSERT",
        "WHEN NEW.id LIKE 'imported:%'",
        2,
    );
    let before = f.raw_database();
    let error = f
        .import(&batch_payload(), ImportStrategy::Overwrite)
        .unwrap_err();
    assert_eq!(error.to_string(), "import_batch_templates_failed");
    assert_eq!(f.raw_database(), before);
    f.assert_counter_rolled_back();
    f.remove_failure();
    assert_eq!(
        f.import(&batch_payload(), ImportStrategy::Overwrite)
            .unwrap(),
        2
    );
}

#[test]
fn rf021_core_second_object_insert_failure_rolls_back_templates_and_hlc() {
    let f = Fixture::new();
    f.fail_nth("objects", "INSERT", "WHEN NEW.id LIKE 'rf021-%'", 2);
    let before = f.raw_database();
    assert_eq!(
        f.import(&batch_payload(), ImportStrategy::Overwrite)
            .unwrap_err()
            .to_string(),
        "import_batch_objects_failed"
    );
    assert_eq!(f.raw_database(), before);
    f.assert_counter_rolled_back();
    f.remove_failure();
    assert_eq!(
        f.import(&batch_payload(), ImportStrategy::Overwrite)
            .unwrap(),
        2
    );
}

#[test]
fn rf021_core_second_object_update_failure_keeps_all_sixty_old_snapshots() {
    let f = Fixture::new();
    f.seed("rf021-one", "Old One");
    f.seed("rf021-two", "Old Two");
    for i in 0..60 {
        f.vault
            .save_snapshot_at(
                "rf021-one",
                "test",
                format!("old-{i}").as_bytes(),
                "old",
                1000 + i,
            )
            .unwrap();
    }
    assert_eq!(f.raw_table("object_snapshots").len(), 60);
    f.fail_nth("objects", "UPDATE", "WHEN NEW.id LIKE 'rf021-%'", 2);
    let before = f.raw_database();
    assert!(f
        .import(&batch_payload(), ImportStrategy::Overwrite)
        .is_err());
    assert_eq!(f.raw_database(), before);
    f.assert_counter_rolled_back();
}

#[test]
fn rf021_core_second_hlc_failure_rolls_back_new_templates_and_objects() {
    let f = Fixture::new();
    f.fail_nth("sync_hlc", "INSERT", "", 2);
    let before = f.raw_database();
    assert_eq!(
        f.import(&batch_payload(), ImportStrategy::Overwrite)
            .unwrap_err()
            .to_string(),
        "import_batch_hlc_failed"
    );
    assert_eq!(f.raw_database(), before);
    f.assert_counter_rolled_back();
    f.remove_failure();
    assert_eq!(
        f.import(&batch_payload(), ImportStrategy::Overwrite)
            .unwrap(),
        2
    );
}

#[test]
fn rf021_core_skip_uses_batch_initial_active_membership_and_counts_duplicate_new_id() {
    let f = Fixture::new();
    f.seed("rf021-active", "Local");
    let mut deleted = f.record("rf021-deleted", "Trash");
    deleted.is_deleted = true;
    deleted.deleted_at = Some(NOW.to_string());
    f.vault.save_object(&deleted).unwrap();
    let payload = json!({"objects": [
        object("rf021-active", "Ignored"), object("rf021-deleted", "Restored"),
        object("rf021-new", "First"), object("rf021-new", "Last"),
    ]});
    assert_eq!(f.import(&payload, ImportStrategy::SkipExisting).unwrap(), 3);
    assert_eq!(
        f.vault.load_object("rf021-active").unwrap().unwrap().name,
        "Local"
    );
    let restored = f.vault.load_object("rf021-deleted").unwrap().unwrap();
    assert_eq!(restored.name, "Restored");
    assert!(!restored.is_deleted);
    assert_eq!(
        f.vault.load_object("rf021-new").unwrap().unwrap().name,
        "Last"
    );
}

#[test]
fn rf021_core_overwrite_and_merge_keep_history_and_ignore_package_history() {
    for strategy in [ImportStrategy::Overwrite, ImportStrategy::Merge] {
        let f = Fixture::new();
        f.seed("rf021-one", "Old");
        for i in 0..60 {
            f.vault
                .save_snapshot_at("rf021-one", "test", &[i as u8], "old", 1000 + i)
                .unwrap();
        }
        let old_history = f.raw_table("object_snapshots");
        let mut first = object("rf021-one", "First");
        first["snapshots"] = json!([{"data": "bmV3", "timestamp": 9999}]);
        let payload = json!({"objects": [first, object("rf021-one", "Last")]});
        assert_eq!(f.import(&payload, strategy).unwrap(), 2);
        assert_eq!(
            f.vault.load_object("rf021-one").unwrap().unwrap().name,
            "Last"
        );
        assert_eq!(f.raw_table("object_snapshots"), old_history);
    }
}

#[test]
fn rf021_core_template_hash_dedupe_and_repeated_original_id_last_mapping() {
    let f = Fixture::new();
    let mut one = object("rf021-one", "One");
    one["template_id"] = json!("same-source");
    let payload = json!({
        "templates": [
            template("same-source", "RF021 Original Content"),
            template("alias-source", "RF021 Original Content"),
            template("same-source", "RF021 Last Content"),
        ],
        "objects": [one],
    });
    let before = f.vault.count_user_templates(&f.account_id).unwrap();
    assert_eq!(f.import(&payload, ImportStrategy::Overwrite).unwrap(), 1);
    assert_eq!(
        f.vault.count_user_templates(&f.account_id).unwrap(),
        before + 2
    );
    let expected: UserTemplate =
        serde_json::from_value(template("same-source", "RF021 Last Content")).unwrap();
    let expected_id = imported_template_id("same-source", &user_template_content_hash(&expected));
    let actual = f.vault.load_object("rf021-one").unwrap().unwrap();
    assert_eq!(actual.template_id.as_deref(), Some(expected_id.as_str()));
    assert_eq!(actual.properties["__templateName"], "RF021 Last Content");
}

#[test]
fn rf021_core_seed_template_content_reuses_original_local_id() {
    let f = Fixture::new();
    let mut local: UserTemplate =
        serde_json::from_value(template("local-seed", "RF021 Seed")).unwrap();
    local.account_id = f.account_id.clone();
    f.vault.save_user_template(&local).unwrap();
    let mut one = object("rf021-one", "One");
    one["template_id"] = json!("foreign-source");
    let payload =
        json!({"templates": [template("foreign-source", "RF021 Seed")], "objects": [one]});
    let before = f.vault.count_user_templates(&f.account_id).unwrap();
    assert_eq!(f.import(&payload, ImportStrategy::Overwrite).unwrap(), 1);
    assert_eq!(f.vault.count_user_templates(&f.account_id).unwrap(), before);
    assert_eq!(
        f.vault
            .load_object("rf021-one")
            .unwrap()
            .unwrap()
            .template_id
            .as_deref(),
        Some("local-seed")
    );
}

#[test]
fn rf021_core_invalid_templates_warn_skip_without_decoding_unused_bad_local_template() {
    let f = Fixture::new();
    let mut local: UserTemplate =
        serde_json::from_value(template("rf021-corrupt", "RF021 Bad")).unwrap();
    local.account_id = f.account_id.clone();
    f.vault.save_user_template(&local).unwrap();
    f.db.execute(
        "UPDATE user_templates SET properties_json='bad-ciphertext' WHERE id=?1",
        ["rf021-corrupt"],
    )
    .unwrap();
    let payload =
        json!({"templates": [{"id": "invalid-only"}], "objects": [object("rf021-one", "One")]});
    assert_eq!(f.import(&payload, ImportStrategy::Overwrite).unwrap(), 1);
    assert!(f.vault.load_user_template("rf021-corrupt").is_err());
}

#[test]
fn rf021_core_valid_package_template_propagates_frozen_local_template_read_error() {
    let f = Fixture::new();
    let mut local: UserTemplate =
        serde_json::from_value(template("rf021-corrupt", "RF021 Bad")).unwrap();
    local.account_id = f.account_id.clone();
    f.vault.save_user_template(&local).unwrap();
    f.db.execute(
        "UPDATE user_templates SET properties_json='bad-ciphertext' WHERE id=?1",
        ["rf021-corrupt"],
    )
    .unwrap();
    let before = f.raw_database();
    assert!(f
        .import(&batch_payload(), ImportStrategy::Overwrite)
        .is_err());
    assert_eq!(f.raw_database(), before);
}

#[test]
fn rf021_core_metadata_skip_does_not_decode_unused_bad_object_properties_or_labels() {
    let f = Fixture::new();
    f.seed("rf021-active", "Local");
    f.db.execute("UPDATE objects SET properties='bad-ciphertext', property_labels='bad-ciphertext' WHERE id=?1", ["rf021-active"]).unwrap();
    let payload =
        json!({"objects": [object("rf021-active", "Ignored"), object("rf021-new", "New")]});
    assert_eq!(f.import(&payload, ImportStrategy::SkipExisting).unwrap(), 1);
    assert!(f.vault.load_object("rf021-active").is_err());
    assert_eq!(
        f.vault.load_object("rf021-new").unwrap().unwrap().name,
        "New"
    );
}

#[test]
fn rf021_core_prepare_does_not_write_and_same_connection_change_rejects_commit() {
    let f = Fixture::new();
    let before = f.raw_database();
    let prepared = prepare_import_database(
        &f.vault,
        &f.account_id,
        &batch_payload(),
        ImportStrategy::Overwrite,
        NOW,
    )
    .unwrap();
    assert_eq!(f.raw_database(), before);
    assert_eq!(prepared.batch.objects.len(), 2);
    assert!(prepared
        .batch
        .objects
        .iter()
        .all(|write| matches!(write.history, ImportHistoryChange::Keep)));
    f.seed("rf021-concurrent", "Concurrent");
    let after_concurrent = f.raw_database();
    let error = commit_import_database(&ImportTarget::Direct(&f.vault), &f.account_id, &prepared)
        .unwrap_err();
    assert_eq!(error.to_string(), "import_batch_stale_view");
    assert_eq!(f.raw_database(), after_concurrent);
}

#[test]
fn rf021_core_second_connection_change_rejects_old_prepared_revision() {
    let f = Fixture::new();
    f.seed("rf021-concurrent", "Before");
    let prepared = prepare_import_database(
        &f.vault,
        &f.account_id,
        &batch_payload(),
        ImportStrategy::Overwrite,
        NOW,
    )
    .unwrap();
    f.db.execute(
        "UPDATE objects SET name='After' WHERE id=?1",
        ["rf021-concurrent"],
    )
    .unwrap();
    let before = f.raw_database();
    assert_eq!(
        commit_import_database(&ImportTarget::Direct(&f.vault), &f.account_id, &prepared)
            .unwrap_err()
            .to_string(),
        "import_batch_stale_view"
    );
    assert_eq!(f.raw_database(), before);
}

#[test]
fn rf021_core_old_session_cannot_commit_prepared_batch_after_same_account_reunlock() {
    let f = Fixture::new();
    let session = f.service.capture_session(&f.account_id).unwrap();
    let prepared = prepare_import_database(
        session.vault(),
        &f.account_id,
        &batch_payload(),
        ImportStrategy::Overwrite,
        NOW,
    )
    .unwrap();
    f.service.lock();
    f.service.unlock(&f.account_id, PASSWORD).unwrap();
    let before = f.raw_database();
    let target = ImportTarget::Session {
        service: &f.service,
        session: &session,
    };
    assert!(commit_import_database(&target, &f.account_id, &prepared).is_err());
    assert_eq!(f.raw_database(), before);
}

#[test]
fn rf021_core_expired_session_is_rejected_before_opening_package() {
    let f = Fixture::new();
    let session = f.service.capture_session(&f.account_id).unwrap();
    f.service.lock();
    f.service.unlock(&f.account_id, PASSWORD).unwrap();
    let target = ImportTarget::Session {
        service: &f.service,
        session: &session,
    };
    let error = import_vault_into(
        &target,
        &f.account_id,
        &f.dir.path().join("does-not-exist.solosoul"),
        PACKAGE_PASSWORD,
        ImportStrategy::Overwrite,
        f.dir.path(),
        None,
    )
    .unwrap_err();
    assert_eq!(error.to_string(), "Vault session is no longer current");
}

#[test]
fn rf021_core_database_success_preserves_encryption_and_orders_template_object_hlc() {
    let f = Fixture::new();
    assert_eq!(
        f.import(&batch_payload(), ImportStrategy::Overwrite)
            .unwrap(),
        2
    );
    let one = f.vault.load_object("rf021-one").unwrap().unwrap();
    assert_eq!(one.properties["value"], "One");
    let template_id = one.template_id.unwrap();
    let template_hlc = f
        .vault
        .get_record_hlc("user_templates", &template_id)
        .unwrap()
        .unwrap();
    let object_hlc = f
        .vault
        .get_record_hlc("objects", "rf021-one")
        .unwrap()
        .unwrap();
    assert!(object_hlc.wall_time_ms > template_hlc.wall_time_ms);
    let encrypted: String =
        f.db.query_row(
            "SELECT properties FROM objects WHERE id=?1",
            ["rf021-one"],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!encrypted.contains("\"value\""));
    let second_hlc = f
        .vault
        .get_record_hlc("objects", "rf021-two")
        .unwrap()
        .unwrap();
    assert!(second_hlc.wall_time_ms > object_hlc.wall_time_ms);
}

#[test]
fn rf021_core_preferences_failure_keeps_old_err_contract_after_database_commit() {
    let f = Fixture::new();
    let path = f.package(
        &batch_payload(),
        &[("preferences.enc", b"not-a-ciphertext".to_vec())],
        false,
    );
    assert!(import_vault(
        &f.vault,
        &f.account_id,
        &path,
        PACKAGE_PASSWORD,
        ImportStrategy::Overwrite,
        f.dir.path(),
        None
    )
    .is_err());
    assert!(f.vault.load_object("rf021-one").unwrap().is_some());
    assert!(f.vault.load_object("rf021-two").unwrap().is_some());
}

#[test]
fn rf021_core_attachment_failure_keeps_old_err_contract_after_database_commit() {
    let f = Fixture::new();
    let mut payload = batch_payload();
    payload["objects"][0]["properties"]["__attachments"] = json!([{
        "id": "attachment-one", "objectId": "rf021-one", "fileName": "one.txt",
        "mimeType": "text/plain", "sizeBytes": 4, "createdAt": NOW,
    }]);
    let path = f.package(
        &payload,
        &[(
            "attachments/rf021-one/attachment-one.enc",
            b"broken".to_vec(),
        )],
        true,
    );
    assert!(import_vault(
        &f.vault,
        &f.account_id,
        &path,
        PACKAGE_PASSWORD,
        ImportStrategy::Overwrite,
        f.dir.path(),
        None
    )
    .is_err());
    assert!(f.vault.load_object("rf021-one").unwrap().is_some());
}

#[test]
fn rf021_core_template_only_import_commits_template_and_returns_zero_object_writes() {
    let f = Fixture::new();
    let count = f.vault.count_user_templates(&f.account_id).unwrap();
    let payload =
        json!({"templates": [template("source-only", "RF021 Template Only")], "objects": []});
    assert_eq!(f.import(&payload, ImportStrategy::SkipExisting).unwrap(), 0);
    assert_eq!(
        f.vault.count_user_templates(&f.account_id).unwrap(),
        count + 1
    );
}

#[test]
fn rf021_core_wrong_account_prepare_returns_error_without_writes() {
    let f = Fixture::new();
    let before = f.raw_database();
    assert!(prepare_import_database(
        &f.vault,
        "wrong-account",
        &batch_payload(),
        ImportStrategy::Overwrite,
        NOW
    )
    .is_err());
    assert_eq!(f.raw_database(), before);
}

/// 只供新增坏 UTF-8 回归：ValueRef 保留 SQLite 类型及原字节，禁止先转 owned String。
#[derive(Debug, PartialEq, Eq)]
enum MetadataRawCell {
    Null,
    Integer(i64),
    RealBits(u64),
    Text(Vec<u8>),
    Blob(Vec<u8>),
}

fn metadata_raw_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Vec<MetadataRawCell>> {
    (0..row.as_ref().column_count())
        .map(|column| {
            Ok(match row.get_ref(column)? {
                rusqlite::types::ValueRef::Null => MetadataRawCell::Null,
                rusqlite::types::ValueRef::Integer(value) => MetadataRawCell::Integer(value),
                rusqlite::types::ValueRef::Real(value) => {
                    MetadataRawCell::RealBits(value.to_bits())
                }
                rusqlite::types::ValueRef::Text(bytes) => MetadataRawCell::Text(bytes.to_vec()),
                rusqlite::types::ValueRef::Blob(bytes) => MetadataRawCell::Blob(bytes.to_vec()),
            })
        })
        .collect()
}

fn metadata_raw_table(db: &Connection, table: &str) -> Vec<Vec<MetadataRawCell>> {
    assert!(matches!(
        table,
        "objects" | "user_templates" | "object_snapshots" | "sync_hlc"
    ));
    let mut statement = db
        .prepare(&format!("SELECT * FROM {table} ORDER BY 1,2"))
        .unwrap();
    let rows = statement.query_map([], metadata_raw_row).unwrap();
    rows.map(Result::unwrap).collect()
}

fn metadata_raw_database(db: &Connection) -> Vec<Vec<Vec<MetadataRawCell>>> {
    ["objects", "user_templates", "object_snapshots", "sync_hlc"]
        .into_iter()
        .map(|table| metadata_raw_table(db, table))
        .collect()
}

// 跨 API 兼容：仅 SkipExisting 消费冻结的旧 active metadata 严格查询。
// SQL 不解密 properties/labels，也不解析 children；其它策略不读取无关 metadata。
#[test]
fn rf021_core_skip_strict_active_metadata_keeps_old_error_and_zero_writes() {
    let columns = [
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
    ];
    for column in columns {
        let f = Fixture::new();
        f.seed("rf021-corrupt-metadata", "Unused local");
        // template_type 有 CHECK；仅外部测试连接注入时暂停约束，读取/导入前恢复。
        if column == "template_type" {
            f.db.execute_batch("PRAGMA ignore_check_constraints=ON")
                .unwrap();
        }
        let injected = f.db.execute(
            &format!("UPDATE objects SET {column}=x'80' WHERE id=?1"),
            ["rf021-corrupt-metadata"],
        );
        if column == "template_type" {
            f.db.execute_batch("PRAGMA ignore_check_constraints=OFF")
                .unwrap();
        }
        injected.unwrap();
        let expected = f
            .vault
            .list_object_metadata(&f.account_id, None, None, false, false)
            .unwrap_err();
        let before = metadata_raw_database(&f.db);
        let error = f
            .import(
                &json!({"objects": [object("rf021-new", "New")]}),
                ImportStrategy::SkipExisting,
            )
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            expected,
            "strict active metadata column {column}"
        );
        assert_eq!(
            metadata_raw_database(&f.db),
            before,
            "metadata read error must not commit: {column}"
        );
    }
    // 不同于 BLOB 的真实非法 UTF-8 TEXT，也必须保留原 strict parser 错误。
    let f = Fixture::new();
    f.seed("rf021-invalid-utf8", "Unused local");
    f.db.execute(
        "UPDATE objects SET name=CAST(x'80' AS TEXT) WHERE id=?1",
        ["rf021-invalid-utf8"],
    )
    .unwrap();
    let expected = f
        .vault
        .list_object_metadata(&f.account_id, None, None, false, false)
        .unwrap_err();
    let before = metadata_raw_database(&f.db);
    assert_eq!(
        f.import(
            &json!({"objects": [object("rf021-new", "New")]}),
            ImportStrategy::SkipExisting
        )
        .unwrap_err()
        .to_string(),
        expected
    );
    assert_eq!(metadata_raw_database(&f.db), before);
}

#[test]
fn rf021_core_overwrite_and_merge_ignore_unused_typed_metadata_errors() {
    for strategy in [ImportStrategy::Overwrite, ImportStrategy::Merge] {
        for column in [
            "id",
            "name",
            "is_deleted",
            "type_id",
            "sensitivity_level",
            "created_at",
        ] {
            let f = Fixture::new();
            f.seed("rf021-corrupt-metadata", "Unused local");
            let bad_rowid: i64 =
                f.db.query_row(
                    "SELECT rowid FROM objects WHERE id=?1",
                    ["rf021-corrupt-metadata"],
                    |row| row.get(0),
                )
                .unwrap();
            f.db.execute(
                &format!("UPDATE objects SET {column}=x'80' WHERE id=?1"),
                ["rf021-corrupt-metadata"],
            )
            .unwrap();
            let before_bad =
                f.db.query_row(
                    "SELECT * FROM objects WHERE rowid=?1",
                    [bad_rowid],
                    metadata_raw_row,
                )
                .unwrap();
            assert_eq!(
                f.import(&json!({"objects": [object("rf021-new", "New")]}), strategy)
                    .unwrap(),
                1,
                "unused {column} must not block {strategy:?}"
            );
            assert_eq!(
                f.vault.load_object("rf021-new").unwrap().unwrap().name,
                "New"
            );
            let after_bad =
                f.db.query_row(
                    "SELECT * FROM objects WHERE rowid=?1",
                    [bad_rowid],
                    metadata_raw_row,
                )
                .unwrap();
            assert_eq!(
                after_bad, before_bad,
                "unrelated bad row must not be repaired/overwritten: {column}"
            );
        }
    }
}

#[test]
fn rf021_core_skip_ignores_corrupt_soft_deleted_and_other_excluded_metadata() {
    for (column, literal, deleted) in [
        ("name", "x'80'", "1"),
        ("name", "CAST(x'80' AS TEXT)", "1"),
        ("id", "x'80'", "1"),
        ("type_id", "x'80'", "1"),
        ("is_deleted", "x'80'", "0"),
        ("is_deleted", "CAST(x'80' AS TEXT)", "0"),
    ] {
        let f = Fixture::new();
        f.seed("rf021-excluded", "Excluded local");
        f.db.execute(
            &format!("UPDATE objects SET is_deleted={deleted}, {column}={literal} WHERE id=?1"),
            ["rf021-excluded"],
        )
        .unwrap();
        // 使用旧查询作为边界证据：WHERE is_deleted=0 本就不读取该坏行。
        assert!(f
            .vault
            .list_object_metadata(&f.account_id, None, None, false, false)
            .unwrap()
            .is_empty());
        let old_rows = metadata_raw_table(&f.db, "objects");
        assert_eq!(
            f.import(
                &json!({"objects": [object("rf021-new", "New")]}),
                ImportStrategy::SkipExisting
            )
            .unwrap(),
            1
        );
        let rows = metadata_raw_table(&f.db, "objects");
        assert_eq!(rows.len(), old_rows.len() + 1);
        assert!(
            rows.contains(&old_rows[0]),
            "excluded row must retain exact raw bytes"
        );
    }
}

#[test]
fn rf021_core_skip_metadata_stays_independent_of_bad_properties_labels_and_children() {
    for column in ["properties", "property_labels", "children_ids"] {
        let f = Fixture::new();
        f.seed("rf021-existing", "Local");
        f.db.execute(
            &format!("UPDATE objects SET {column}='bad-ciphertext-or-json' WHERE id=?1"),
            ["rf021-existing"],
        )
        .unwrap();
        assert!(f.vault.load_object("rf021-existing").is_err());
        let old_rows = metadata_raw_table(&f.db, "objects");
        assert_eq!(f.import(&json!({"objects": [object("rf021-existing", "Must skip"), object("rf021-new", "New")]}), ImportStrategy::SkipExisting).unwrap(), 1);
        assert!(
            metadata_raw_table(&f.db, "objects").contains(&old_rows[0]),
            "metadata-only Skip cannot alter bad {column}"
        );
        assert_eq!(
            f.vault.load_object("rf021-new").unwrap().unwrap().name,
            "New"
        );
    }
}

#[test]
fn rf021_core_frozen_active_metadata_survives_live_change_but_commit_is_stale() {
    let f = Fixture::new();
    f.seed("rf021-existing", "Before");
    let before_prepare = metadata_raw_database(&f.db);
    let prepared = prepare_import_database(
        &f.vault,
        &f.account_id,
        &json!({"objects": [object("rf021-existing", "Must skip"), object("rf021-new", "New")]}),
        ImportStrategy::SkipExisting,
        NOW,
    )
    .unwrap();
    assert_eq!(metadata_raw_database(&f.db), before_prepare);
    assert_eq!(prepared.batch.objects.len(), 1);
    assert_eq!(prepared.batch.objects[0].record.id, "rf021-new");
    f.db.execute(
        "UPDATE objects SET name='After', is_deleted=1 WHERE id=?1",
        ["rf021-existing"],
    )
    .unwrap();
    assert!(f
        .vault
        .list_object_metadata(&f.account_id, None, None, false, false)
        .unwrap()
        .is_empty());
    let frozen = f
        .vault
        .list_import_view_object_metadata(&prepared.view)
        .unwrap();
    assert_eq!(frozen.len(), 1);
    assert_eq!(frozen[0].id, "rf021-existing");
    assert_eq!(frozen[0].name, "Before");
    assert!(!frozen[0].is_deleted);
    let after_concurrent = metadata_raw_database(&f.db);
    assert_eq!(
        commit_import_database(&ImportTarget::Direct(&f.vault), &f.account_id, &prepared)
            .unwrap_err()
            .to_string(),
        "import_batch_stale_view"
    );
    assert_eq!(metadata_raw_database(&f.db), after_concurrent);
    assert!(f.vault.load_object("rf021-new").unwrap().is_none());
}

// 全部 Skip 的真实公共入口不得读取没有写操作所需的本地 HLC。
#[test]
fn rf021_core_all_skip_avoids_unused_corrupt_local_hlc_and_keeps_four_tables() {
    let f = Fixture::new();
    // 使用 32 位 hex node，公开 getter 与 save_object 生成的规范化 node 相同。
    f.vault
        .set_sync_node_id("0123456789abcdef0123456789abcdef")
        .unwrap();
    f.seed("rf021-empty-hlc-one", "Local One");
    f.seed("rf021-empty-hlc-two", "Local Two");
    for id in ["rf021-empty-hlc-one", "rf021-empty-hlc-two"] {
        f.vault
            .save_snapshot_at(id, "test", b"existing-history", "old", 1000)
            .unwrap();
    }
    let local_node = f.vault.get_sync_node_id().unwrap().unwrap();
    for id in ["rf021-empty-hlc-one", "rf021-empty-hlc-two"] {
        let seeded_hlc = f.vault.get_record_hlc("objects", id).unwrap().unwrap();
        assert_eq!(seeded_hlc.node_id, local_node);
        assert!(seeded_hlc.wall_time_ms > 0);
    }
    // f.db 是 Fixture 打开的第二个真实 SQLite 连接；先 seed 正常 HLC 再注入 BLOB。
    assert!(
        f.db.execute(
            "UPDATE sync_hlc SET wall_time_ms=x'00' WHERE node_id=?1",
            [&local_node],
        )
        .unwrap()
            >= 2
    );
    // 与 BatchHlc::new 相同的 typed MAX 查询确实无法将 BLOB 读为整数。
    // 这是故障前提，不直接调用内部 batch，也不替代真实 import_vault 接线。
    let typed_error =
        f.db.query_row(
            "SELECT COALESCE(MAX(wall_time_ms), 0) FROM sync_hlc WHERE node_id=?1",
            [&local_node],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_err();
    assert!(matches!(
        typed_error,
        rusqlite::Error::InvalidColumnType(_, _, _)
    ));
    let before = metadata_raw_database(&f.db);
    // 包中没有 templates；两个对象 ID 都是已存在的 active 对象，计划应完全为空。
    let payload = json!({
        "objects": [
            object("rf021-empty-hlc-one", "Must Skip One"),
            object("rf021-empty-hlc-two", "Must Skip Two"),
        ],
    });
    assert_eq!(f.import(&payload, ImportStrategy::SkipExisting).unwrap(), 0);
    assert_eq!(
        metadata_raw_database(&f.db),
        before,
        "zero-write Core import must preserve all four business tables, including HLC BLOB bytes"
    );
}
