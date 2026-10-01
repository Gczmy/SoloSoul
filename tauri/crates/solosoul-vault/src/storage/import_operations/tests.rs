//! RF022 TEMP 真 SQLite 候选回归；未运行。文件动作闭包仅测许可，不冒充 Core 文件集成。
use super::*;
use crate::{ImportHistoryChange, ImportObjectWrite, ObjectRecord, VaultConfig};
use rusqlite::types::Value as SqlValue;
use tempfile::TempDir;
const ACCOUNT: &str = "rf022_account";
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
        let vault =
            VaultStore::open(VaultConfig::new(ACCOUNT, base).with_data_key([0x22; 32])).unwrap();
        let db = Connection::open(vault.base_path().join("vault.db")).unwrap();
        db.busy_timeout(std::time::Duration::from_millis(30))
            .unwrap();
        Self { vault, db, root }
    }
    fn start(&self, kind: ImportSourceKind, owners: &[&str], prefs: bool) -> ImportOperationStart {
        let steps:Vec<_>=owners.iter().enumerate().map(|(i,owner)|{
            let id=uuid::Uuid::from_u128(100+i as u128).to_string();ImportAttachmentStepPlan{entry_ordinal:i as u32,source_object_id:format!("source_{owner}"),source_attachment_id:format!("source_att_{i}"),owner_id:owner.to_string(),attachment_id:id.clone(),safe_file_name:"example.txt".into(),metadata:json!({"id":id,"objectId":owner,"fileName":"example.txt","mimeType":"text/plain","createdAt":NOW,"sizeBytes":9999}),initial_staged_proof:None}
        }).collect();
        ImportOperationStart {
            operation_id: uuid::Uuid::new_v4().to_string(),
            source_kind: kind,
            source: ImportSourceProof {
                sha256: "a".repeat(64),
                length: 321,
            },
            root_binding: self.vault.import_root_binding().unwrap(),
            request_fingerprint: "b".repeat(64),
            plan: json!({"selected":null,"strategy":"keepBoth","maps":{"source_owner":"owner"},"private":"RF022 secret proof"}),
            owners: owners
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .map(|owner| ImportAttachmentOwnerPlan {
                    owner_id: owner.into(),
                    expected_attachments: None,
                })
                .collect(),
            steps,
            preferences_required: prefs,
            source_ready: None,
        }
    }
    fn batch(&self, owners: &[&str]) -> ImportDatabaseBatch {
        ImportDatabaseBatch {
            templates: vec![],
            objects: owners
                .iter()
                .map(|owner| ImportObjectWrite {
                    record: record(owner),
                    history: ImportHistoryChange::Keep,
                })
                .collect(),
        }
    }
    fn commit(
        &self,
        start: &ImportOperationStart,
        batch: &ImportDatabaseBatch,
    ) -> Result<ImportOperationCommit, String> {
        let v = self.vault.read_import_view(ACCOUNT).unwrap();
        self.vault
            .commit_import_batch_with_operation(ACCOUNT, &v.revision, batch, start)
    }
    fn accept(&self, owners: &[&str], prefs: bool) -> (ImportOperationStart, ImportOperationLease) {
        let start = self.start(ImportSourceKind::Manual, owners, prefs);
        self.commit(&start, &self.batch(owners)).unwrap();
        let lease = self
            .vault
            .claim_import_operation(ACCOUNT, &start.operation_id, &start.root_binding)
            .unwrap();
        (start, lease)
    }
    fn stage_publish(&self, lease: &ImportOperationLease, ordinal: u32) {
        self.vault
            .confirm_import_attachment_staged(lease, ordinal, &proof(lease.epoch()))
            .unwrap();
        self.vault
            .publish_import_attachment(lease, ordinal, |_| Ok(()))
            .unwrap();
    }
    fn raw(&self) -> Vec<Vec<Vec<SqlValue>>> {
        ["objects", "user_templates", "object_snapshots", "sync_hlc"]
            .iter()
            .map(|t| {
                let mut stmt = self
                    .db
                    .prepare(&format!("SELECT * FROM {t} ORDER BY 1,2"))
                    .unwrap();
                let count = stmt.column_count();
                let rows = stmt
                    .query_map([], |r| (0..count).map(|c| r.get(c)).collect())
                    .unwrap();
                rows.collect::<rusqlite::Result<Vec<Vec<SqlValue>>>>()
                    .unwrap()
            })
            .collect()
    }
    fn count(&self, table: &str) -> i64 {
        self.db
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }
    fn corrupt_local_hlc(&self) {
        const NODE: &str = "0123456789abcdef0123456789abcdef";
        self.vault.set_sync_node_id(NODE).unwrap();
        self.vault.save_object(&record("seed")).unwrap();
        let actual: String = self
            .db
            .query_row(
                "SELECT node_id FROM sync_hlc WHERE table_name='objects' AND record_id='seed'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(actual, NODE);
        self.db.execute("INSERT INTO sync_hlc(table_name,record_id,wall_time_ms,counter,node_id,updated_at) VALUES('objects','corrupt_unused',X'00',0,?1,?2)",params![NODE,NOW]).unwrap();
        let max: rusqlite::Result<i64> = self.db.query_row(
            "SELECT MAX(wall_time_ms) FROM sync_hlc WHERE node_id=?1",
            params![NODE],
            |r| r.get(0),
        );
        assert!(matches!(
            max,
            Err(rusqlite::Error::InvalidColumnType(
                _,
                _,
                rusqlite::types::Type::Blob
            ))
        ));
    }
}
fn record(id: &str) -> ObjectRecord {
    ObjectRecord {
        id: id.into(),
        account_id: ACCOUNT.into(),
        name: "before".into(),
        type_id: "note".into(),
        section_type: "identity".into(),
        icon_name: "document".into(),
        sensitivity_level: "internal".into(),
        properties: json!({"other":"before"}),
        created_at: NOW.into(),
        updated_at: NOW.into(),
        version: 1,
        ..Default::default()
    }
}
fn proof(epoch: u64) -> ImportCiphertextProof {
    ImportCiphertextProof {
        sha256: "c".repeat(64),
        length: 100,
        plaintext_length: 42,
        stage_epoch: epoch,
    }
}
fn marker(start: &ImportOperationStart) -> ImportOwnedAttachmentMarker {
    let s = &start.steps[0];
    ImportOwnedAttachmentMarker {
        account_id: ACCOUNT.into(),
        operation_id: start.operation_id.clone(),
        entry_ordinal: s.entry_ordinal,
        owner_id: s.owner_id.clone(),
        attachment_id: s.attachment_id.clone(),
        root_binding: start.root_binding.clone(),
    }
}

#[test]
fn rf022_business_and_journal_sql_failure_roll_back_together() {
    let f = Fixture::new();
    f.db.execute_batch("CREATE TRIGGER fail_journal BEFORE INSERT ON import_operations BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    let before = f.raw();
    let s = f.start(ImportSourceKind::Manual, &["owner"], false);
    assert_eq!(f.commit(&s, &f.batch(&["owner"])).unwrap_err(), WRITE);
    assert_eq!(f.raw(), before);
    assert_eq!(f.count("import_operations"), 0);
    assert_eq!(f.count("import_attachment_steps"), 0);
}
#[test]
fn rf022_step_insert_failure_rolls_back_business_and_parent() {
    let f = Fixture::new();
    f.db.execute_batch("CREATE TRIGGER fail_step BEFORE INSERT ON import_attachment_steps BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    let before = f.raw();
    let s = f.start(ImportSourceKind::Manual, &["owner"], false);
    assert_eq!(f.commit(&s, &f.batch(&["owner"])).unwrap_err(), WRITE);
    assert_eq!(f.raw(), before);
    assert_eq!(f.count("import_operations"), 0);
}
#[test]
fn rf022_business_sql_error_keeps_exact_batch_stage_code() {
    let f = Fixture::new();
    f.db.execute_batch("CREATE TRIGGER fail_object BEFORE INSERT ON objects BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    let s = f.start(ImportSourceKind::Manual, &["owner"], false);
    assert_eq!(
        f.commit(&s, &f.batch(&["owner"])).unwrap_err(),
        "import_batch_objects_failed"
    );
    assert_eq!(f.count("import_operations"), 0);
}
#[test]
fn rf022_empty_new_and_old_batches_do_not_read_unused_corrupt_hlc() {
    let f = Fixture::new();
    f.corrupt_local_hlc();
    let before = f.raw();
    let s = f.start(ImportSourceKind::Manual, &[], false);
    assert!(f.commit(&s, &ImportDatabaseBatch::default()).is_ok());
    assert_eq!(f.raw(), before);
    let view = f.vault.read_import_view(ACCOUNT).unwrap();
    assert_eq!(
        f.vault
            .commit_import_batch(ACCOUNT, &view.revision, &ImportDatabaseBatch::default())
            .unwrap(),
        ImportDatabaseCommit::default()
    );
    assert_eq!(f.count("import_operations"), 1);
    assert_eq!(f.raw(), before);
}
#[test]
fn rf022_nonempty_hlc_failure_has_zero_business_and_journal_writes() {
    let f = Fixture::new();
    f.corrupt_local_hlc();
    let before = f.raw();
    let s = f.start(ImportSourceKind::Manual, &["owner"], false);
    assert_eq!(
        f.commit(&s, &f.batch(&["owner"])).unwrap_err(),
        "import_batch_hlc_failed"
    );
    assert_eq!(f.raw(), before);
    assert_eq!(f.count("import_operations"), 0);
}
#[test]
fn rf022_stale_foreign_store_account_and_locked_starts_reject() {
    let f = Fixture::new();
    let s = f.start(ImportSourceKind::Manual, &[], false);
    let view = f.vault.read_import_view(ACCOUNT).unwrap();
    f.vault.save_object(&record("extra")).unwrap();
    assert_eq!(
        f.vault
            .commit_import_batch_with_operation(
                ACCOUNT,
                &view.revision,
                &ImportDatabaseBatch::default(),
                &s
            )
            .unwrap_err(),
        "import_batch_stale_view"
    );
    let other = Fixture::new();
    let other_view = other.vault.read_import_view(ACCOUNT).unwrap();
    assert_eq!(
        f.vault
            .commit_import_batch_with_operation(
                ACCOUNT,
                &other_view.revision,
                &ImportDatabaseBatch::default(),
                &s
            )
            .unwrap_err(),
        "import_batch_wrong_store"
    );
    let view = f.vault.read_import_view(ACCOUNT).unwrap();
    assert_eq!(
        f.vault
            .commit_import_batch_with_operation(
                "foreign",
                &view.revision,
                &ImportDatabaseBatch::default(),
                &s
            )
            .unwrap_err(),
        "import_batch_account_mismatch"
    );
    f.vault.lock();
    assert!(f
        .vault
        .commit_import_batch_with_operation(
            ACCOUNT,
            &view.revision,
            &ImportDatabaseBatch::default(),
            &s
        )
        .is_err());
    assert_eq!(f.count("import_operations"), 0);
}
#[test]
fn rf022_same_id_uses_first_maps_without_replaying_batch() {
    let f = Fixture::new();
    let s = f.start(ImportSourceKind::Manual, &["owner"], false);
    let first = f.commit(&s, &f.batch(&["owner"])).unwrap();
    let before = f.raw();
    let mut changed = s.clone();
    changed.plan = json!({"maps":{"source_owner":"other_private_uuid"}});
    changed.steps[0].attachment_id = uuid::Uuid::new_v4().to_string();
    changed.steps[0].metadata["id"] = json!(changed.steps[0].attachment_id);
    let retry = f.commit(&changed, &f.batch(&["another"])).unwrap();
    assert!(retry.already_committed);
    assert_eq!(retry.operation.start, first.operation.start);
    assert_eq!(f.raw(), before);
}
#[test]
fn rf022_same_id_other_source_or_options_or_root_reject() {
    let f = Fixture::new();
    let s = f.start(ImportSourceKind::Manual, &[], false);
    f.commit(&s, &ImportDatabaseBatch::default()).unwrap();
    for field in 0..3 {
        let mut changed = s.clone();
        match field {
            0 => changed.source.sha256 = "d".repeat(64),
            1 => changed.request_fingerprint = "e".repeat(64),
            _ => changed.root_binding = "f".repeat(64),
        };
        assert_eq!(
            f.commit(&changed, &ImportDatabaseBatch::default())
                .unwrap_err(),
            IDENTITY
        )
    }
    assert_eq!(f.count("import_operations"), 1);
}
#[test]
fn rf022_cloud_lookup_and_concurrent_style_first_commit_deduplicate() {
    let f = Fixture::new();
    let s = f.start(ImportSourceKind::Cloud, &[], false);
    let first = f.commit(&s, &ImportDatabaseBatch::default()).unwrap();
    let mut another = s.clone();
    another.operation_id = uuid::Uuid::new_v4().to_string();
    assert_eq!(
        f.commit(&another, &ImportDatabaseBatch::default())
            .unwrap()
            .operation
            .start
            .operation_id,
        first.operation.start.operation_id
    );
    assert_eq!(f.count("import_operations"), 1);
    let lease = f
        .vault
        .claim_import_operation(ACCOUNT, &s.operation_id, &s.root_binding)
        .unwrap();
    f.vault.complete_import_operation(&lease).unwrap();
    assert!(f.vault.list_import_operations(ACCOUNT).unwrap().is_empty());
    assert_eq!(
        f.vault
            .find_cloud_import_operation(
                ACCOUNT,
                &s.source,
                &s.request_fingerprint,
                &s.root_binding
            )
            .unwrap()
            .unwrap()
            .phase,
        ImportOperationPhase::Complete
    );
}
#[test]
fn rf022_new_manual_fresh_same_source_keeps_distinct_operation() {
    let f = Fixture::new();
    let s = f.start(ImportSourceKind::Manual, &[], false);
    f.commit(&s, &ImportDatabaseBatch::default()).unwrap();
    let mut fresh = s.clone();
    fresh.operation_id = uuid::Uuid::new_v4().to_string();
    assert!(
        !f.commit(&fresh, &ImportDatabaseBatch::default())
            .unwrap()
            .already_committed
    );
    assert_eq!(f.count("import_operations"), 2);
}
#[test]
fn rf022_global_epoch_rejects_all_old_worker_mutations() {
    let f = Fixture::new();
    let (s, old) = f.accept(&["owner"], true);
    let new = f
        .vault
        .claim_import_operation(ACCOUNT, &s.operation_id, &s.root_binding)
        .unwrap();
    assert_eq!(new.epoch(), old.epoch() + 1);
    assert_eq!(
        f.vault
            .confirm_import_attachment_staged(&old, 0, &proof(old.epoch()))
            .unwrap_err(),
        LEASE
    );
    let mut called = false;
    assert_eq!(
        f.vault
            .publish_import_attachment(&old, 0, |_| {
                called = true;
                Ok(())
            })
            .unwrap_err(),
        LEASE
    );
    assert!(!called);
    assert_eq!(
        f.vault
            .commit_import_attachment_metadata(&old, "owner")
            .unwrap_err(),
        LEASE
    );
    assert_eq!(
        f.vault
            .commit_import_preferences(
                &old,
                &Profile::new_with_id(ACCOUNT, ACCOUNT, b"{}".to_vec())
            )
            .unwrap_err(),
        LEASE
    );
    assert_eq!(f.vault.complete_import_operation(&old).unwrap_err(), LEASE);
}
#[test]
fn rf022_new_epoch_reuses_previously_confirmed_stage() {
    let f = Fixture::new();
    let (s, old) = f.accept(&["owner"], false);
    f.vault
        .confirm_import_attachment_staged(&old, 0, &proof(old.epoch()))
        .unwrap();
    let new = f
        .vault
        .claim_import_operation(ACCOUNT, &s.operation_id, &s.root_binding)
        .unwrap();
    let mut observed = 0;
    f.vault
        .publish_import_attachment(&new, 0, |step| {
            observed = step.staged_proof.as_ref().unwrap().stage_epoch;
            Ok(())
        })
        .unwrap();
    assert_eq!(observed, old.epoch());
}
#[test]
fn rf022_publish_callback_and_journal_update_failure_leave_staged() {
    let f = Fixture::new();
    let (s, lease) = f.accept(&["owner"], false);
    f.vault
        .confirm_import_attachment_staged(&lease, 0, &proof(lease.epoch()))
        .unwrap();
    assert_eq!(
        f.vault
            .publish_import_attachment(&lease, 0, |_| Err("file_action_failed".into()))
            .unwrap_err(),
        "file_action_failed"
    );
    f.db.execute_batch("CREATE TRIGGER fail_publish BEFORE UPDATE ON import_attachment_steps WHEN NEW.phase='published' BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    let mut called = false;
    assert_eq!(
        f.vault
            .publish_import_attachment(&lease, 0, |_| {
                called = true;
                Ok(())
            })
            .unwrap_err(),
        WRITE
    );
    assert!(called);
    assert_eq!(
        f.vault
            .load_import_operation(ACCOUNT, &s.operation_id)
            .unwrap()
            .unwrap()
            .steps[0]
            .phase,
        ImportAttachmentPhase::Staged
    );
    f.db.execute_batch("DROP TRIGGER fail_publish").unwrap();
    assert!(f
        .vault
        .publish_import_attachment(&lease, 0, |_| Ok(()))
        .unwrap());
    assert!(!f
        .vault
        .publish_import_attachment(&lease, 0, |_| panic!("already published action repeated"))
        .unwrap());
}
#[test]
fn rf022_metadata_preserves_new_other_fields_and_uses_actual_plaintext_length() {
    let f = Fixture::new();
    let (s, lease) = f.accept(&["owner"], false);
    f.stage_publish(&lease, 0);
    let mut current = f.vault.load_object("owner").unwrap().unwrap();
    current.name = "latest user name".into();
    current.properties["other"] = json!("latest edit");
    f.vault.save_object(&current).unwrap();
    let old_version = current.version;
    let done = f
        .vault
        .commit_import_attachment_metadata(&lease, "owner")
        .unwrap();
    assert_eq!(done.attachment_count, 1);
    let current = f.vault.load_object("owner").unwrap().unwrap();
    assert_eq!(current.name, "latest user name");
    assert_eq!(current.properties["other"], "latest edit");
    assert_eq!(current.properties["__attachments"][0]["sizeBytes"], 42);
    assert_eq!(current.version, old_version + 1);
    let before = f.raw();
    f.vault
        .commit_import_attachment_metadata(&lease, "owner")
        .unwrap();
    assert_eq!(f.raw(), before);
    assert_eq!(
        f.vault
            .load_import_operation(ACCOUNT, &s.operation_id)
            .unwrap()
            .unwrap()
            .attachment_count,
        1
    );
}
#[test]
fn rf022_metadata_requires_all_zip_published_then_detects_attachment_baseline_conflict() {
    let f = Fixture::new();
    let (_, lease) = f.accept(&["owner", "second"], false);
    f.stage_publish(&lease, 0);
    assert_eq!(
        f.vault
            .commit_import_attachment_metadata(&lease, "owner")
            .unwrap_err(),
        STATE
    );
    f.stage_publish(&lease, 1);
    let mut current = f.vault.load_object("owner").unwrap().unwrap();
    current.properties["__attachments"] = json!([]);
    f.vault.save_object(&current).unwrap();
    let before = f.raw();
    assert_eq!(
        f.vault
            .commit_import_attachment_metadata(&lease, "owner")
            .unwrap_err(),
        "import_operation_attachment_conflict"
    );
    assert_eq!(f.raw(), before);
}
#[test]
fn rf022_owner_deleted_foreign_or_missing_cannot_activate() {
    for mode in 0..3 {
        let f = Fixture::new();
        let (_, lease) = f.accept(&["owner"], false);
        f.stage_publish(&lease, 0);
        match mode {
            0 => {
                f.db.execute("UPDATE objects SET is_deleted=1 WHERE id='owner'", [])
                    .unwrap();
            }
            1 => {
                f.db.execute(
                    "UPDATE objects SET account_id='foreign' WHERE id='owner'",
                    [],
                )
                .unwrap();
            }
            _ => {
                f.db.execute("DELETE FROM objects WHERE id='owner'", [])
                    .unwrap();
            }
        }
        let before = f.raw();
        assert!(f
            .vault
            .commit_import_attachment_metadata(&lease, "owner")
            .is_err());
        assert_eq!(f.raw(), before);
    }
}
#[test]
fn rf022_metadata_sql_failure_rolls_back_properties_hlc_steps_and_counts() {
    let f = Fixture::new();
    let (s, lease) = f.accept(&["owner"], false);
    f.stage_publish(&lease, 0);
    f.db.execute_batch("CREATE TRIGGER fail_meta BEFORE UPDATE ON import_attachment_steps WHEN NEW.phase='metadataCommitted' BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    let before = f.raw();
    assert_eq!(
        f.vault
            .commit_import_attachment_metadata(&lease, "owner")
            .unwrap_err(),
        WRITE
    );
    assert_eq!(f.raw(), before);
    let op = f
        .vault
        .load_import_operation(ACCOUNT, &s.operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(op.attachment_count, 0);
    assert_eq!(op.steps[0].phase, ImportAttachmentPhase::Published);
}
#[test]
fn rf022_preferences_save_and_phase_rollback_together_and_complete_is_guarded() {
    let f = Fixture::new();
    let (s, lease) = f.accept(&[], true);
    assert_eq!(
        f.vault.complete_import_operation(&lease).unwrap_err(),
        STATE
    );
    f.db.execute_batch("CREATE TRIGGER fail_prefs BEFORE UPDATE ON import_operations WHEN NEW.phase='preferences' BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    let profile = Profile::new_with_id(
        ACCOUNT,
        ACCOUNT,
        b"{\"preferences\":{\"theme\":\"dark\"}}".to_vec(),
    );
    let before = f.raw();
    assert_eq!(
        f.vault
            .commit_import_preferences(&lease, &profile)
            .unwrap_err(),
        WRITE
    );
    assert_eq!(f.count("profiles"), 0);
    assert_eq!(f.raw(), before);
    assert!(
        !f.vault
            .load_import_operation(ACCOUNT, &s.operation_id)
            .unwrap()
            .unwrap()
            .preferences_imported
    );
    f.db.execute_batch("DROP TRIGGER fail_prefs").unwrap();
    f.vault.commit_import_preferences(&lease, &profile).unwrap();
    let done = f.vault.complete_import_operation(&lease).unwrap();
    assert_eq!(done.phase, ImportOperationPhase::Complete);
    let before = f.raw();
    f.vault.complete_import_operation(&lease).unwrap();
    assert_eq!(f.raw(), before);
    assert!(
        f.commit(&s, &f.batch(&["should_not_exist"]))
            .unwrap()
            .already_committed
    );
    assert!(f.vault.load_object("should_not_exist").unwrap().is_none());
}
#[test]
fn rf022_close_reopen_requires_new_native_lease_and_retains_confirmed_steps() {
    let f = Fixture::new();
    let (s, lease) = f.accept(&["owner"], false);
    f.vault
        .confirm_import_attachment_staged(&lease, 0, &proof(lease.epoch()))
        .unwrap();
    let path = f.vault.base_path().to_path_buf();
    f.vault.lock();
    let reopened =
        VaultStore::open(VaultConfig::new(ACCOUNT, path).with_data_key([0x22; 32])).unwrap();
    assert_eq!(
        reopened
            .publish_import_attachment(&lease, 0, |_| panic!("foreign store callback"))
            .unwrap_err(),
        LEASE
    );
    let new = reopened
        .claim_import_operation(ACCOUNT, &s.operation_id, &s.root_binding)
        .unwrap();
    assert!(reopened
        .publish_import_attachment(&new, 0, |step| {
            assert_eq!(
                step.staged_proof.as_ref().unwrap().stage_epoch,
                lease.epoch()
            );
            Ok(())
        })
        .unwrap());
}
#[test]
fn rf022_recovery_ready_initial_steps_accept_atomically_or_roll_back() {
    let f = Fixture::new();
    let mut s = f.start(ImportSourceKind::Recovery, &["owner"], true);
    s.source_ready = Some(json!({"payload":{"objects":[]},"preferences":{"preferences":{}}}));
    s.steps[0].initial_staged_proof = Some(proof(0));
    let accepted = f.commit(&s, &f.batch(&["owner"])).unwrap();
    assert_eq!(
        accepted.operation.steps[0].phase,
        ImportAttachmentPhase::Staged
    );
    assert_eq!(
        accepted.operation.steps[0]
            .staged_proof
            .as_ref()
            .unwrap()
            .stage_epoch,
        0
    );
    let other = Fixture::new();
    let mut s = other.start(ImportSourceKind::Recovery, &["owner"], false);
    s.source_ready = Some(json!({"payload":{}}));
    s.steps[0].initial_staged_proof = Some(proof(0));
    other.db.execute_batch("CREATE TRIGGER fail_recovery BEFORE INSERT ON import_operations BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    let before = other.raw();
    assert!(other.commit(&s, &other.batch(&["owner"])).is_err());
    assert_eq!(other.raw(), before);
    assert_eq!(other.count("import_operations"), 0);
}
#[test]
fn rf022_recovery_missing_ready_or_partial_initial_proofs_reject() {
    let f = Fixture::new();
    let mut s = f.start(ImportSourceKind::Recovery, &["owner"], false);
    assert_eq!(f.commit(&s, &f.batch(&["owner"])).unwrap_err(), INVALID);
    s.source_ready = Some(json!({"payload":{}}));
    assert_eq!(f.commit(&s, &f.batch(&["owner"])).unwrap_err(), INVALID);
    assert_eq!(f.count("objects"), 0);
    assert_eq!(f.count("import_operations"), 0);
}
#[test]
fn rf022_orphan_guard_unknown_foreign_pending_and_latest_completed_reference_protect() {
    let f = Fixture::new();
    let (s, lease) = f.accept(&["owner"], false);
    let m = marker(&s);
    assert!(f
        .vault
        .with_import_orphan_delete_guard(ACCOUNT, &m, || -> Result<(), String> {
            panic!("pending deletion")
        })
        .unwrap()
        .is_none());
    let mut foreign = m.clone();
    foreign.account_id = "foreign".into();
    assert!(f
        .vault
        .with_import_orphan_delete_guard(ACCOUNT, &foreign, || -> Result<(), String> {
            panic!("foreign deletion")
        })
        .unwrap()
        .is_none());
    let mut unknown = m.clone();
    unknown.operation_id = uuid::Uuid::new_v4().to_string();
    assert!(f
        .vault
        .with_import_orphan_delete_guard(ACCOUNT, &unknown, || -> Result<(), String> {
            panic!("unknown deletion")
        })
        .unwrap()
        .is_none());
    // 模拟旧 scanner 在 metadata 前已缓存空引用：guard 必须读取最新 DB，不接受缓存参数。
    f.stage_publish(&lease, 0);
    f.vault
        .commit_import_attachment_metadata(&lease, "owner")
        .unwrap();
    f.vault.complete_import_operation(&lease).unwrap();
    assert!(f
        .vault
        .with_import_orphan_delete_guard(ACCOUNT, &m, || -> Result<(), String> {
            panic!("new reference deleted")
        })
        .unwrap()
        .is_none());
    let mut obj = f.vault.load_object("owner").unwrap().unwrap();
    obj.properties
        .as_object_mut()
        .unwrap()
        .remove("__attachments");
    f.vault.save_object(&obj).unwrap();
    assert_eq!(
        f.vault
            .with_import_orphan_delete_guard(ACCOUNT, &m, || Ok("deleted"))
            .unwrap(),
        Some("deleted")
    );
}
#[test]
fn rf022_orphan_guard_bad_latest_reference_fails_closed() {
    let f = Fixture::new();
    let (s, lease) = f.accept(&["owner"], false);
    f.stage_publish(&lease, 0);
    f.vault
        .commit_import_attachment_metadata(&lease, "owner")
        .unwrap();
    f.vault.complete_import_operation(&lease).unwrap();
    f.db.execute(
        "UPDATE objects SET properties='broken-json' WHERE id='owner'",
        [],
    )
    .unwrap();
    assert!(f
        .vault
        .with_import_orphan_delete_guard(ACCOUNT, &marker(&s), || -> Result<(), String> {
            panic!("bad reference deleted")
        })
        .is_err());
}
#[test]
fn rf022_sensitive_plan_result_steps_are_encrypted_and_tampered_phase_rejects() {
    let f = Fixture::new();
    let (s, _) = f.accept(&["owner"], false);
    let p: String =
        f.db.query_row("SELECT plan_enc FROM import_operations", [], |r| r.get(0))
            .unwrap();
    let q: String =
        f.db.query_row("SELECT result_enc FROM import_operations", [], |r| r.get(0))
            .unwrap();
    let step: String =
        f.db.query_row("SELECT step_enc FROM import_attachment_steps", [], |r| {
            r.get(0)
        })
        .unwrap();
    for v in [p, q, step] {
        assert!(v.starts_with("solo:"));
        assert!(!v.contains("RF022 secret proof"));
        assert!(!v.contains("example.txt"));
        assert!(!v.contains(&s.source.sha256));
    }
    f.db.execute("UPDATE import_operations SET phase='complete'", [])
        .unwrap();
    assert!(f
        .vault
        .load_import_operation(ACCOUNT, &s.operation_id)
        .is_err());
}
#[test]
fn rf022_pending_rekey_is_rejected_with_zero_changes() {
    let f = Fixture::new();
    let (s, _) = f.accept(&[], false);
    let before = f.raw();
    let cipher: String =
        f.db.query_row("SELECT plan_enc FROM import_operations", [], |r| r.get(0))
            .unwrap();
    assert_eq!(
        f.vault
            .reencrypt_all(
                &DataEncryptionKey::new([0x22; 32]),
                &DataEncryptionKey::new([0x33; 32])
            )
            .unwrap_err(),
        "IMPORT_OPERATIONS_PENDING"
    );
    assert_eq!(f.raw(), before);
    assert_eq!(
        f.db.query_row("SELECT plan_enc FROM import_operations", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        cipher
    );
    assert!(f
        .vault
        .load_import_operation(ACCOUNT, &s.operation_id)
        .is_ok());
}
#[test]
fn rf022_complete_and_abandoned_journal_rekey_remain_readable() {
    let f = Fixture::new();
    let (s, lease) = f.accept(&[], false);
    f.vault.complete_import_operation(&lease).unwrap();
    let (other, other_lease) = f.accept(&[], false);
    f.vault.abandon_import_operation(&other_lease).unwrap();
    f.vault
        .reencrypt_all(
            &DataEncryptionKey::new([0x22; 32]),
            &DataEncryptionKey::new([0x33; 32]),
        )
        .unwrap();
    f.vault.set_data_key(DataEncryptionKey::new([0x33; 32]));
    assert_eq!(
        f.vault
            .load_import_operation(ACCOUNT, &s.operation_id)
            .unwrap()
            .unwrap()
            .phase,
        ImportOperationPhase::Complete
    );
    assert_eq!(
        f.vault
            .load_import_operation(ACCOUNT, &other.operation_id)
            .unwrap()
            .unwrap()
            .phase,
        ImportOperationPhase::Abandoned
    );
}
#[test]
fn rf022_wrong_account_root_and_marker_do_not_acquire_or_read() {
    let f = Fixture::new();
    let (s, _) = f.accept(&[], false);
    assert_eq!(
        f.vault
            .load_import_operation("foreign", &s.operation_id)
            .unwrap_err(),
        IDENTITY
    );
    assert_eq!(
        f.vault
            .claim_import_operation(ACCOUNT, &s.operation_id, &"f".repeat(64))
            .unwrap_err(),
        IDENTITY
    );
    assert_eq!(
        f.vault
            .find_cloud_import_operation(
                "foreign",
                &s.source,
                &s.request_fingerprint,
                &s.root_binding
            )
            .unwrap_err(),
        IDENTITY
    );
    assert!(f.root.path().exists());
}

#[test]
fn rf022_journal_only_complete_probe_tracks_reencrypt_key() {
    let f = Fixture::new();
    let (start, lease) = f.accept(&[], false);
    f.vault.complete_import_operation(&lease).unwrap();
    for table in [
        "profiles",
        "objects",
        "trash_items",
        "user_templates",
        "object_snapshots",
        "audit_log",
        "llm_conversations",
        "sync_conflicts",
    ] {
        assert_eq!(f.count(table), 0, "{table}");
    }
    assert_eq!(f.count("import_operations"), 1);
    assert_eq!(f.count("import_attachment_steps"), 0);
    let db_path = f.vault.base_path().join("vault.db");
    let old = DataEncryptionKey::new([0x22; 32]);
    let new = DataEncryptionKey::new([0x33; 32]);
    assert!(crate::probe_data_key(&db_path, &old).unwrap());
    assert!(!crate::probe_data_key(&db_path, &new).unwrap());
    f.vault.reencrypt_all(&old, &new).unwrap();
    assert!(!crate::probe_data_key(&db_path, &old).unwrap());
    assert!(crate::probe_data_key(&db_path, &new).unwrap());
    let reopened = VaultStore::open(
        VaultConfig::new(ACCOUNT, f.vault.base_path().to_path_buf()).with_data_key([0x33; 32]),
    )
    .unwrap();
    assert_eq!(
        reopened
            .load_import_operation(ACCOUNT, &start.operation_id)
            .unwrap()
            .unwrap()
            .phase,
        ImportOperationPhase::Complete
    );
}
