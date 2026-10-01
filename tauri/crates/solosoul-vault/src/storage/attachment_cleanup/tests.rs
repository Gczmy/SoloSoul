//! RF-016：真实 SQLite 失败、重开及文件动作边界，仅使用合成 TempDir。

use super::*;
use crate::{ObjectRecord, VaultConfig};
use rusqlite::types::Value as SqlValue;
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::TempDir;

const ACCOUNT: &str = "rf016_account";
const OWNER: &str = "rf016_owner";
const NOW: &str = "2026-09-30T00:00:00Z";

struct Fixture {
    vault: VaultStore,
    db: Connection,
    root: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        let base = root.path().join(ACCOUNT);
        std::fs::create_dir_all(&base).unwrap();
        let config = VaultConfig::new(ACCOUNT, base).with_data_key([7; 32]);
        let vault = VaultStore::open(config).unwrap();
        let db = Connection::open(vault.base_path().join("vault.db")).unwrap();
        db.busy_timeout(Duration::from_millis(20)).unwrap();
        let fixture = Self { vault, db, root };
        fixture.save_object(
            OWNER,
            ACCOUNT,
            false,
            vec![
                fixture.attachment(OWNER, "first"),
                fixture.attachment(OWNER, "second"),
                fixture.attachment(OWNER, "retained"),
            ],
        );
        fixture
    }

    fn attachment(&self, storage: &str, id: &str) -> Value {
        let directory = self.root.path().join("attachments").join(storage).join(id);
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("fixture.bin");
        std::fs::write(&path, format!("RF016 synthetic {storage}/{id}")).unwrap();
        serde_json::json!({
            "id": id, "objectId": storage, "fileName": "fixture.bin",
            "mimeType": "application/octet-stream", "sizeBytes": 30,
            "createdAt": NOW, "deletedAt": NOW,
            "vaultPath": path.to_string_lossy(), "srcPath": null,
            "description": "synthetic", "tags": ["fixture"]
        })
    }

    fn save_object(&self, id: &str, account: &str, deleted: bool, attachments: Vec<Value>) {
        self.vault.save_object(&ObjectRecord {
            id: id.into(), account_id: account.into(), type_id: "note".into(),
            section_type: "test".into(), name: "RF016 synthetic".into(),
            properties: serde_json::json!({"ordinary": "preserve", "__attachments": attachments}),
            is_deleted: deleted, deleted_at: deleted.then(|| NOW.into()),
            created_at: NOW.into(), updated_at: NOW.into(), version: 1,
            ..Default::default()
        }).unwrap();
    }

    fn directory(&self, storage: &str, id: &str) -> PathBuf {
        self.root.path().join("attachments").join(storage).join(id)
    }

    fn queue(&self, ids: &[&str]) -> Result<Vec<AttachmentCleanupIntent>, String> {
        self.vault.queue_attachment_deletions(
            ACCOUNT,
            OWNER,
            &ids.iter().map(|id| (*id).to_string()).collect::<Vec<_>>(),
        )
    }

    fn snapshot(&self) -> Vec<Vec<Vec<SqlValue>>> {
        ["objects", "sync_hlc", "attachment_cleanup_intents"]
            .into_iter()
            .map(|table| {
                let mut statement = self
                    .db
                    .prepare(&format!("SELECT * FROM {table} ORDER BY 1,2"))
                    .unwrap();
                let count = statement.column_count();
                let rows = statement
                    .query_map([], |row| {
                        (0..count)
                            .map(|index| row.get::<_, SqlValue>(index))
                            .collect::<rusqlite::Result<Vec<_>>>()
                    })
                    .unwrap();
                rows.collect::<rusqlite::Result<Vec<_>>>().unwrap()
            })
            .collect()
    }

    fn intents(&self) -> Vec<AttachmentCleanupIntent> {
        self.vault.list_attachment_cleanup_intents(ACCOUNT).unwrap()
    }
}

fn remove_directory(path: &Path) -> Result<(), String> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("synthetic file action failure".to_string()),
    }
}

#[test]
fn rf016_queue_sql_failure_preserves_metadata_hlc_and_files() {
    let f = Fixture::new();
    let before = f.snapshot();
    let bytes = std::fs::read(f.directory(OWNER, "first").join("fixture.bin")).unwrap();
    f.db.execute_batch(
        "CREATE TRIGGER rf016_reject_object BEFORE UPDATE ON objects
         BEGIN SELECT RAISE(ABORT, 'synthetic object save failure'); END;",
    )
    .unwrap();
    assert!(f.queue(&["first"]).is_err());
    assert_eq!(f.snapshot(), before);
    assert_eq!(
        std::fs::read(f.directory(OWNER, "first").join("fixture.bin")).unwrap(),
        bytes
    );
}

#[test]
fn rf016_second_intent_insert_failure_rolls_back_entire_batch() {
    let f = Fixture::new();
    f.db.execute_batch(
        "CREATE TABLE rf016_insert_count (n INTEGER NOT NULL);
         INSERT INTO rf016_insert_count VALUES(0);
         CREATE TRIGGER rf016_reject_second BEFORE INSERT ON attachment_cleanup_intents BEGIN
           UPDATE rf016_insert_count SET n = n + 1;
           SELECT CASE WHEN (SELECT n FROM rf016_insert_count) = 2
             THEN RAISE(ABORT, 'synthetic second intent failure') END;
         END;",
    )
    .unwrap();
    let before = f.snapshot();
    assert!(f.queue(&["first", "second"]).is_err());
    assert_eq!(f.snapshot(), before);
    assert_eq!(
        f.db.query_row("SELECT n FROM rf016_insert_count", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    for id in ["first", "second", "retained"] {
        assert!(f.directory(OWNER, id).join("fixture.bin").is_file());
    }
}

#[test]
fn rf016_queue_updates_once_and_repeated_or_unknown_ids_do_not_create_permission() {
    let f = Fixture::new();
    let before = f.vault.load_object(OWNER).unwrap().unwrap();
    assert!(f.queue(&["unknown"]).unwrap().is_empty());
    assert_eq!(
        f.vault.load_object(OWNER).unwrap().unwrap().version,
        before.version
    );
    let queued = f.queue(&["first", "first", "unknown"]).unwrap();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].attachment_id, "first");
    assert_eq!(
        f.vault.load_object(OWNER).unwrap().unwrap().version,
        before.version + 1
    );
    assert_eq!(f.queue(&["first"]).unwrap(), queued);
    assert_eq!(
        f.vault.load_object(OWNER).unwrap().unwrap().version,
        before.version + 1
    );
    assert!(f.directory(OWNER, "first").exists());
    assert!(f.directory(OWNER, "retained").exists());
}

#[test]
fn rf016_reopen_after_metadata_commit_recovers_pending_file_cleanup() {
    let f = Fixture::new();
    let intent = f.queue(&["first"]).unwrap().remove(0);
    let version = f.vault.load_object(OWNER).unwrap().unwrap().version;
    assert!(f.directory(OWNER, "first").exists());
    f.vault.lock();
    let reopened = VaultStore::open(
        VaultConfig::new(ACCOUNT, f.root.path().join(ACCOUNT)).with_data_key([7; 32]),
    )
    .unwrap();
    assert_eq!(
        reopened.list_attachment_cleanup_intents(ACCOUNT).unwrap(),
        vec![intent.clone()]
    );
    assert!(reopened
        .run_attachment_cleanup_intent(&intent, |_stored| {
            remove_directory(&f.directory(OWNER, "first"))
        })
        .unwrap());
    assert!(!f.directory(OWNER, "first").exists());
    assert!(f.directory(OWNER, "retained").exists());
    assert!(reopened
        .list_attachment_cleanup_intents(ACCOUNT)
        .unwrap()
        .is_empty());
    assert_eq!(
        reopened.load_object(OWNER).unwrap().unwrap().version,
        version
    );
}

#[test]
fn rf016_action_error_is_fixed_persisted_and_retry_finishes() {
    let f = Fixture::new();
    let intent = f.queue(&["first"]).unwrap().remove(0);
    let result = f.vault.run_attachment_cleanup_intent(&intent, |_stored| {
        Err("PRIVATE: user home path / secret filename".into())
    });
    assert_eq!(
        result.unwrap_err(),
        "Attachment cleanup file operation failed"
    );
    let pending = f.intents().remove(0);
    assert_eq!(pending.attempts, 1);
    assert_eq!(
        pending.last_error_code.as_deref(),
        Some("file_action_failed")
    );
    assert!(f.directory(OWNER, "first").exists());
    assert!(f
        .vault
        .run_attachment_cleanup_intent(&pending, |_stored| {
            remove_directory(&f.directory(OWNER, "first"))
        })
        .unwrap());
    assert!(f.intents().is_empty());
    let called = Cell::new(false);
    assert!(f
        .vault
        .run_attachment_cleanup_intent(&pending, |_stored| {
            called.set(true);
            Ok(())
        })
        .unwrap());
    assert!(!called.get());
}

#[test]
fn rf016_confirmation_sql_failure_leaves_intent_then_not_found_completes() {
    let f = Fixture::new();
    let intent = f.queue(&["first"]).unwrap().remove(0);
    f.db.execute_batch(
        "CREATE TRIGGER rf016_reject_confirmation BEFORE DELETE ON attachment_cleanup_intents
         BEGIN SELECT RAISE(ABORT, 'synthetic cleanup confirmation failure'); END;",
    )
    .unwrap();
    assert!(f
        .vault
        .run_attachment_cleanup_intent(&intent, |_stored| {
            remove_directory(&f.directory(OWNER, "first"))
        })
        .is_err());
    assert!(!f.directory(OWNER, "first").exists());
    assert_eq!(f.intents(), vec![intent.clone()]);
    f.db.execute_batch("DROP TRIGGER rf016_reject_confirmation")
        .unwrap();
    assert!(f
        .vault
        .run_attachment_cleanup_intent(&intent, |_stored| {
            remove_directory(&f.directory(OWNER, "first"))
        })
        .unwrap());
    assert!(f.intents().is_empty());
}

#[test]
fn rf016_new_soft_deleted_reference_prevents_file_action() {
    let f = Fixture::new();
    let entry = f.vault.load_object(OWNER).unwrap().unwrap().properties["__attachments"][0].clone();
    let intent = f.queue(&["first"]).unwrap().remove(0);
    f.save_object("restored_alias", ACCOUNT, true, vec![entry]);
    let called = Cell::new(false);
    assert!(!f
        .vault
        .run_attachment_cleanup_intent(&intent, |_stored| {
            called.set(true);
            Ok(())
        })
        .unwrap());
    assert!(!called.get());
    assert_eq!(
        f.intents()[0].last_error_code.as_deref(),
        Some("referenced")
    );
    assert_eq!(f.intents()[0].attempts, 0);
    assert!(f.directory(OWNER, "first").exists());
}

#[test]
fn rf016_all_reference_rows_are_strict_even_after_a_reference_matches() {
    let f = Fixture::new();
    let entry = f.vault.load_object(OWNER).unwrap().unwrap().properties["__attachments"][0].clone();
    let intent = f.queue(&["first"]).unwrap().remove(0);
    f.save_object("alias", ACCOUNT, false, vec![entry]);
    f.vault
        .save_object(&ObjectRecord {
            id: "bad_last".into(),
            account_id: ACCOUNT.into(),
            name: "synthetic malformed metadata".into(),
            type_id: "note".into(),
            section_type: "test".into(),
            properties: serde_json::json!({"__attachments": [{"id": "bad", "vaultPath": 42}]}),
            created_at: NOW.into(),
            updated_at: NOW.into(),
            version: 1,
            ..Default::default()
        })
        .unwrap();
    let called = Cell::new(false);
    assert!(f
        .vault
        .run_attachment_cleanup_intent(&intent, |_stored| {
            called.set(true);
            Ok(())
        })
        .is_err());
    assert!(!called.get());
    assert_eq!(
        f.intents()[0].last_error_code.as_deref(),
        Some("invalid_references")
    );
    assert!(f.directory(OWNER, "first").exists());
}

#[test]
fn rf016_restore_owner_id_preserves_distinct_storage_id() {
    let f = Fixture::new();
    let entry = f.attachment("old_storage", "restored_file");
    f.save_object("new_owner", ACCOUNT, false, vec![entry]);
    let intent = f
        .vault
        .queue_attachment_deletions(ACCOUNT, "new_owner", &["restored_file".into()])
        .unwrap()
        .remove(0);
    assert_eq!(intent.object_id, "new_owner");
    assert_eq!(intent.storage_object_id, "old_storage");
    assert!(f
        .vault
        .run_attachment_cleanup_intent(&intent, |_stored| {
            remove_directory(&f.directory("old_storage", "restored_file"))
        })
        .unwrap());
    assert!(!f.directory("old_storage", "restored_file").exists());
    assert!(f.directory(OWNER, "retained").exists());
}

#[test]
fn rf016_missing_foreign_or_invalid_requests_do_not_grant_permission() {
    let f = Fixture::new();
    let before = f.snapshot();
    assert!(f
        .vault
        .queue_attachment_deletions(ACCOUNT, "missing", &["first".into()])
        .is_err());
    assert!(f
        .vault
        .queue_attachment_deletions("other_account", OWNER, &["first".into()])
        .is_err());
    assert!(f.queue(&["first", "../escape"]).is_err());
    assert_eq!(f.snapshot(), before);
    f.save_object(
        "foreign",
        "foreign_account",
        false,
        vec![f.attachment("foreign", "foreign_file")],
    );
    assert!(f
        .vault
        .queue_attachment_deletions(ACCOUNT, "foreign", &["foreign_file".into()])
        .is_err());
    assert!(f.intents().is_empty());
    assert!(f.directory(OWNER, "first").exists());
}

#[test]
fn rf016_forged_or_expired_permission_cannot_run_action_and_lock_rejects_retry() {
    let f = Fixture::new();
    let intent = f.queue(&["first"]).unwrap().remove(0);
    let called = Cell::new(false);
    let mut forged = intent.clone();
    forged.storage_object_id = "somewhere_else".into();
    assert!(f
        .vault
        .run_attachment_cleanup_intent(&forged, |_stored| {
            called.set(true);
            Ok(())
        })
        .is_err());
    forged = intent.clone();
    forged.created_at += 1;
    assert!(f
        .vault
        .run_attachment_cleanup_intent(&forged, |_stored| {
            called.set(true);
            Ok(())
        })
        .is_err());
    f.vault.lock();
    assert!(f
        .vault
        .run_attachment_cleanup_intent(&intent, |_stored| {
            called.set(true);
            Ok(())
        })
        .is_err());
    assert!(!called.get());
    assert!(f.directory(OWNER, "first").exists());
}

#[test]
fn rf016_immediate_transaction_prevents_another_connection_writing_during_action() {
    let f = Fixture::new();
    let intent = f.queue(&["first"]).unwrap().remove(0);
    let blocked = Cell::new(false);
    assert!(f
        .vault
        .run_attachment_cleanup_intent(&intent, |_stored| {
            let other = Connection::open(f.vault.base_path().join("vault.db")).unwrap();
            other.busy_timeout(Duration::from_millis(5)).unwrap();
            let error = other
                .execute(
                    "UPDATE objects SET name = 'outside write' WHERE id = ?1",
                    [OWNER],
                )
                .unwrap_err();
            blocked.set(error.sqlite_error_code() == Some(rusqlite::ErrorCode::DatabaseBusy));
            remove_directory(&f.directory(OWNER, "first"))
        })
        .unwrap());
    assert!(blocked.get());
    assert!(f.intents().is_empty());
}

#[test]
fn rf016_pending_permission_survives_owner_disappearance_and_bad_metadata_never_queues() {
    let f = Fixture::new();
    let intent = f.queue(&["first"]).unwrap().remove(0);
    f.db.execute("DELETE FROM objects WHERE id = ?1", [OWNER])
        .unwrap();
    assert_eq!(f.queue(&["first"]).unwrap(), vec![intent.clone()]);
    assert!(f
        .vault
        .run_attachment_cleanup_intent(&intent, |_stored| {
            remove_directory(&f.directory(OWNER, "first"))
        })
        .unwrap());
    f.vault
        .save_object(&ObjectRecord {
            id: "bad_array".into(),
            account_id: ACCOUNT.into(),
            type_id: "note".into(),
            section_type: "test".into(),
            name: "bad array".into(),
            properties: serde_json::json!({"__attachments": "not an array"}),
            created_at: NOW.into(),
            updated_at: NOW.into(),
            version: 1,
            ..Default::default()
        })
        .unwrap();
    let before = f.snapshot();
    assert!(f
        .vault
        .queue_attachment_deletions(ACCOUNT, "bad_array", &["anything".into()])
        .is_err());
    assert_eq!(f.snapshot(), before);
}

#[test]
fn rf016_schema_26_upgrades_idempotently_with_only_local_intent_fields() {
    let mut conn = Connection::open_in_memory().unwrap();
    VaultStore::init_schema(&conn).unwrap();
    crate::migration::set_schema_version(&conn, 26).unwrap();
    crate::migration::run_migrations(&mut conn).unwrap();
    assert_eq!(
        crate::migration::get_schema_version(&conn).unwrap(),
        crate::migration::CURRENT_SCHEMA_VERSION
    );
    crate::migration::run_migrations(&mut conn).unwrap();
    assert_eq!(
        conn.query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = 27",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap(),
        1
    );
    assert_eq!(
        conn.query_row(
            "SELECT COUNT(*) FROM attachment_cleanup_intents",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap(),
        0
    );
    let mut statement = conn
        .prepare("PRAGMA table_info(attachment_cleanup_intents)")
        .unwrap();
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(
        columns,
        vec![
            "account_id",
            "object_id",
            "storage_object_id",
            "attachment_id",
            "created_at",
            "attempts",
            "last_error_code",
        ]
    );
}

#[test]
fn rf016_failed_schema_27_transaction_preserves_schema_26() {
    let mut conn = Connection::open_in_memory().unwrap();
    VaultStore::init_schema(&conn).unwrap();
    crate::migration::set_schema_version(&conn, 26).unwrap();
    conn.execute_batch(
        "CREATE TRIGGER rf016_reject_migration BEFORE INSERT ON schema_migrations
         WHEN NEW.version = 27 BEGIN SELECT RAISE(ABORT, 'synthetic migration failure'); END;",
    )
    .unwrap();
    assert!(crate::migration::run_migrations(&mut conn).is_err());
    assert_eq!(crate::migration::get_schema_version(&conn).unwrap(), 26);
    assert_eq!(conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'attachment_cleanup_intents'",
        [], |row| row.get::<_, i64>(0),
    ).unwrap(), 0);
    conn.execute_batch("DROP TRIGGER rf016_reject_migration")
        .unwrap();
    crate::migration::run_migrations(&mut conn).unwrap();
    assert_eq!(
        crate::migration::get_schema_version(&conn).unwrap(),
        crate::migration::CURRENT_SCHEMA_VERSION
    );
}

#[test]
fn rf016_soft_deleted_owner_preserves_gui_permission_and_action_receives_database_intent() {
    let f = Fixture::new();
    let mut entry = f.attachment("old_storage", "soft_file");
    entry["vaultPath"] = Value::String("/legacy/path/not-used-to-delete".into());
    f.save_object("soft_owner", ACCOUNT, true, vec![entry]);
    let intent = f
        .vault
        .queue_attachment_deletions(ACCOUNT, "soft_owner", &["soft_file".into()])
        .unwrap()
        .remove(0);
    // 调用方展示快照上的诊断字段不成为执行参数；action 收到数据库原值。
    let mut caller_snapshot = intent.clone();
    caller_snapshot.attempts = 99;
    caller_snapshot.last_error_code = Some("caller supplied detail".into());
    assert!(f
        .vault
        .run_attachment_cleanup_intent(&caller_snapshot, |stored| {
            assert_eq!(stored, &intent);
            assert_eq!(stored.object_id, "soft_owner");
            remove_directory(&f.directory(&stored.storage_object_id, &stored.attachment_id))
        })
        .unwrap());
    assert!(!f.directory("old_storage", "soft_file").exists());
    let owner = f.vault.load_object("soft_owner").unwrap().unwrap();
    assert!(owner.is_deleted);
    assert!(owner.properties["__attachments"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn rf016_case_alias_and_source_path_reference_prevent_file_action() {
    let f = Fixture::new();
    let intent = f.queue(&["first"]).unwrap().remove(0);
    let mut entry = f.attachment("other_storage", "other_attachment");
    entry["srcPath"] = Value::String(format!(
        "C:\\\\legacy\\\\attachments\\\\{}\\\\FIRST\\\\fixture.bin",
        OWNER.to_ascii_uppercase()
    ));
    f.save_object("source_alias", ACCOUNT, false, vec![entry]);
    let called = Cell::new(false);
    assert!(!f
        .vault
        .run_attachment_cleanup_intent(&intent, |_stored| {
            called.set(true);
            Ok(())
        })
        .unwrap());
    assert!(!called.get());
    assert_eq!(
        f.intents()[0].last_error_code.as_deref(),
        Some("referenced")
    );
    assert!(f.directory(OWNER, "first").exists());
}

#[test]
fn rf016_missing_required_null_and_duplicate_metadata_never_grant_permission() {
    let f = Fixture::new();
    for (owner, metadata) in [
        ("missing_required", serde_json::json!([{"id": "target"}])),
        (
            "null_required",
            serde_json::json!([{
                "id": "target", "fileName": null, "mimeType": "test",
                "sizeBytes": 1, "createdAt": NOW
            }]),
        ),
        (
            "duplicate_metadata",
            Value::Array(vec![
                f.attachment("duplicate_metadata", "target"),
                f.attachment("duplicate_metadata", "target"),
            ]),
        ),
    ] {
        f.vault
            .save_object(&ObjectRecord {
                id: owner.into(),
                account_id: ACCOUNT.into(),
                name: "synthetic invalid".into(),
                type_id: "note".into(),
                section_type: "test".into(),
                properties: serde_json::json!({"__attachments": metadata}),
                created_at: NOW.into(),
                updated_at: NOW.into(),
                version: 1,
                ..Default::default()
            })
            .unwrap();
        let before = f.snapshot();
        assert!(f
            .vault
            .queue_attachment_deletions(ACCOUNT, owner, &["target".into()],)
            .is_err());
        assert_eq!(f.snapshot(), before);
    }
    assert!(f.intents().is_empty());
}
