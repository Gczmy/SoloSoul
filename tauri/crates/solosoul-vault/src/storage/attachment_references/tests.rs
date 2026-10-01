//! RF903：真实 SQLite、透明加密与物理文件动作回归候选。未运行。
//! Core 的 SOLC2 全 AEAD/会话/maintenance/文件身份验证由其真实集成回归覆盖。
use super::*;
use crate::encryption::{encrypt_field, encrypt_text_field};
use crate::{
    ImportAttachmentOwnerPlan, ImportAttachmentStepPlan, ImportCiphertextProof,
    ImportDatabaseBatch, ImportHistoryChange, ImportObjectWrite, ImportOperationLease,
    ImportOperationStart, ImportSourceKind, ImportSourceProof, ObjectRecord, Profile, RecordHlc,
    SyncConflictBatchEntry, TrashItem, VaultConfig,
};
use rusqlite::params;
use serde_json::json;
use std::cell::Cell;
use tempfile::TempDir;

const ACCOUNT_ID: &str = "rf903_account";
const OWNER: &str = "original_owner";
const ATTACHMENT_ID: &str = "00000000-0000-4000-8000-000000000903";
const NOW: &str = "2026-10-01T00:00:00Z";
const KEY: [u8; 32] = [0x93; 32];

struct Fixture {
    vault: VaultStore,
    db: Connection,
    root: TempDir,
}
impl Fixture {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        let owner = VaultRootOwner::acquire(root.path()).unwrap();
        let base = owner.root().join(ACCOUNT_ID);
        std::fs::create_dir_all(&base).unwrap();
        let vault =
            VaultStore::open_owned(VaultConfig::new(ACCOUNT_ID, base).with_data_key(KEY), owner)
                .unwrap();
        let db = Connection::open(vault.base_path().join("vault.db")).unwrap();
        db.busy_timeout(std::time::Duration::from_millis(30))
            .unwrap();
        Self { vault, db, root }
    }
    fn candidate(&self) -> AttachmentReferenceCandidate {
        let dir = self
            .vault
            .root_owner()
            .root()
            .join("attachments")
            .join(OWNER)
            .join(ATTACHMENT_ID);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("example.txt");
        std::fs::write(
            &path,
            b"physical file survives unless the real action succeeds",
        )
        .unwrap();
        AttachmentReferenceCandidate {
            storage_object_id: OWNER.into(),
            attachment_id: ATTACHMENT_ID.into(),
            canonical_path: path.canonicalize().unwrap(),
            literal_aliases: vec![],
        }
    }
    fn protect(&self, c: &AttachmentReferenceCandidate) {
        let view = self
            .vault
            .read_attachment_reference_view(ACCOUNT_ID)
            .unwrap();
        let called = Cell::new(false);
        let outcome = self
            .vault
            .with_unreferenced_attachment_guard(ACCOUNT_ID, &view, c, || {
                called.set(true);
                std::fs::remove_file(&c.canonical_path).map_err(|e| e.to_string())
            })
            .unwrap();
        assert_eq!(outcome, None);
        assert!(!called.get());
        assert!(c.canonical_path.exists());
    }
    fn record(&self, id: &str, properties: Value) -> ObjectRecord {
        ObjectRecord {
            id: id.into(),
            account_id: ACCOUNT_ID.into(),
            type_id: "note".into(),
            section_type: "identity".into(),
            name: "real encrypted object".into(),
            icon_name: "document".into(),
            sensitivity_level: "internal".into(),
            properties,
            created_at: NOW.into(),
            updated_at: NOW.into(),
            version: 1,
            ..Default::default()
        }
    }
    fn snapshot(&self, id: &str, properties: Value, when: i64) {
        self.vault
            .save_snapshot_at(
                id,
                "user_edit",
                &serde_json::to_vec(&json!({"name":"snapshot","properties":properties})).unwrap(),
                "historical",
                when,
            )
            .unwrap();
    }
    fn trash(id: &str, kind: &str, properties: Value, when: i64) -> TrashItem {
        TrashItem {
            id: id.into(),
            item_type: kind.into(),
            original_id: OWNER.into(),
            original_parent_id: None,
            original_section_type: Some("identity".into()),
            original_sort_order: None,
            data: serde_json::to_vec(&json!({"id":OWNER,"properties":properties})).unwrap(),
            deleted_at: when,
            expires_at: None,
            deleted_by: ACCOUNT_ID.into(),
            name_snapshot: "recoverable".into(),
            icon_snapshot: None,
        }
    }
    fn conflict(&self, table: &str, local: Value, remote: Value, remote_deleted: bool) {
        self.vault
            .save_sync_conflicts_batch(&[SyncConflictBatchEntry {
                table: table.into(),
                record_id: format!("record_{table}"),
                local_hlc: RecordHlc {
                    wall_time_ms: 1,
                    counter: 0,
                    node_id: "local".into(),
                },
                remote_hlc: RecordHlc {
                    wall_time_ms: 2,
                    counter: 0,
                    node_id: "remote".into(),
                },
                local_data: local,
                remote_data: remote,
                remote_deleted,
            }])
            .unwrap();
    }
    fn accept(&self, prefs: bool) -> (ImportOperationStart, ImportOperationLease) {
        let start = ImportOperationStart {
            operation_id: uuid::Uuid::new_v4().to_string(),
            source_kind: ImportSourceKind::Manual,
            source: ImportSourceProof {
                sha256: "a".repeat(64),
                length: 100,
            },
            root_binding: self.vault.import_root_binding().unwrap(),
            request_fingerprint: "b".repeat(64),
            plan: json!({"preferences":{"kind":if prefs {"package"} else {"none"}}}),
            owners: vec![ImportAttachmentOwnerPlan {
                owner_id: OWNER.into(),
                expected_attachments: None,
            }],
            steps: vec![ImportAttachmentStepPlan {
                entry_ordinal: 0,
                source_object_id: "source_owner".into(),
                source_attachment_id: "source_attachment".into(),
                owner_id: OWNER.into(),
                attachment_id: ATTACHMENT_ID.into(),
                safe_file_name: "example.txt".into(),
                metadata: meta(ATTACHMENT_ID),
                initial_staged_proof: None,
            }],
            preferences_required: prefs,
            source_ready: None,
        };
        let view = self.vault.read_import_view(ACCOUNT_ID).unwrap();
        let batch = ImportDatabaseBatch {
            objects: vec![ImportObjectWrite {
                record: self.record(OWNER, json!({})),
                history: ImportHistoryChange::Keep,
            }],
            templates: vec![],
        };
        self.vault
            .commit_import_batch_with_operation(ACCOUNT_ID, &view.revision, &batch, &start)
            .unwrap();
        let lease = self
            .vault
            .claim_import_operation(ACCOUNT_ID, &start.operation_id, &start.root_binding)
            .unwrap();
        (start, lease)
    }
    fn publish(
        &self,
        lease: &ImportOperationLease,
        c: &AttachmentReferenceCandidate,
        activate: bool,
    ) {
        self.vault
            .confirm_import_attachment_staged(
                lease,
                0,
                &ImportCiphertextProof {
                    sha256: "c".repeat(64),
                    length: 100,
                    plaintext_length: 9,
                    stage_epoch: lease.epoch(),
                },
            )
            .unwrap();
        assert!(self
            .vault
            .publish_import_attachment(lease, 0, |_| std::fs::write(
                &c.canonical_path,
                b"published"
            )
            .map_err(|e| e.to_string()))
            .unwrap());
        if activate {
            self.vault
                .commit_import_attachment_metadata(lease, OWNER)
                .unwrap();
        }
    }
}
fn meta(id: &str) -> Value {
    json!({"id":id,"objectId":OWNER,"fileName":"example.txt","mimeType":"text/plain","sizeBytes":9,"createdAt":NOW})
}
fn refs() -> Value {
    json!({"__attachments":[meta(ATTACHMENT_ID)]})
}

#[test]
fn rf903_current_and_soft_deleted_objects_both_protect_the_real_file() {
    for soft in [false, true] {
        let f = Fixture::new();
        let c = f.candidate();
        f.vault.save_object(&f.record(OWNER, refs())).unwrap();
        if soft {
            f.vault.delete_object(OWNER, true).unwrap();
        }
        assert_eq!(
            f.vault
                .read_attachment_reference_view(ACCOUNT_ID)
                .unwrap()
                .stats()
                .object_rows,
            1
        );
        f.protect(&c);
    }
}
#[test]
fn rf903_object_and_page_trash_501_are_not_truncated_by_ui_limit() {
    for kind in ["object", "page"] {
        let f = Fixture::new();
        let c = f.candidate();
        let tx = f.db.unchecked_transaction().unwrap();
        let key = DataEncryptionKey(KEY);
        for index in 0..501 {
            let props = if index == 0 { refs() } else { json!({}) };
            let item = Fixture::trash(&format!("trash_{index:04}"), kind, props, index);
            VaultStore::save_trash_item_tx(&tx, &key, &item).unwrap();
        }
        tx.commit().unwrap();
        let ui = f.vault.list_trash_items(None, None).unwrap();
        assert_eq!(ui.len(), 500);
        assert!(ui.iter().all(|item| item.id != "trash_0000"));
        assert_eq!(
            f.vault
                .read_attachment_reference_view(ACCOUNT_ID)
                .unwrap()
                .stats()
                .trash_rows,
            501
        );
        f.protect(&c);
    }
}
#[test]
fn rf903_snapshot_51_and_history_without_current_object_are_protected() {
    let f = Fixture::new();
    let c = f.candidate();
    for index in 0..51 {
        f.snapshot(OWNER, if index == 0 { refs() } else { json!({}) }, index);
    }
    let ui = f.vault.list_snapshots(OWNER).unwrap();
    assert_eq!(ui.len(), 50);
    assert!(ui.iter().all(|v| v["timestamp"] != json!(0)));
    assert!(f.vault.load_object(OWNER).unwrap().is_none());
    assert_eq!(
        f.vault
            .read_attachment_reference_view(ACCOUNT_ID)
            .unwrap()
            .stats()
            .snapshot_rows,
        51
    );
    f.protect(&c);
}
#[test]
fn rf903_restore_changing_owner_and_attachment_ids_preserves_path_aliases() {
    let f = Fixture::new();
    let mut c = f.candidate();
    let legacy =
        r"C:\old-root\attachments\original_owner\00000000-0000-4000-8000-000000000903\example.txt";
    let mut changed = meta("new_attachment_id");
    changed["objectId"] = json!("restored_owner_1234");
    changed["vaultPath"] = json!(legacy);
    f.vault
        .save_object(&f.record("restored_owner_1234", json!({"__attachments":[changed]})))
        .unwrap();
    f.protect(&c);
    // 两个 ID 与目录布局都变化，调用者提供已验证物理候选对应的原字面别名。
    c.literal_aliases
        .push(r"C:\historical-native\legacy-files\original.bin".into());
    f.vault
        .save_object(&f.record(
            "restored_owner_1234",
            json!({"src_path":"c:/HISTORICAL-native/legacy-files/original.bin"}),
        ))
        .unwrap();
    f.protect(&c);
    assert!(f.root.path().exists());
}
#[test]
fn rf903_legacy_snake_metadata_and_extended_windows_paths_preserve_references() {
    let f = Fixture::new();
    let c = f.candidate();
    let att = json!({"id":"new_id","object_id":"new_owner","file_name":"example.txt","mime_type":"text/plain","size_bytes":9,"created_at":NOW,"src_path":c.canonical_path.to_string_lossy().replace('\\',"/")});
    f.vault
        .save_object(&f.record("new_owner", json!({"__attachments":[att]})))
        .unwrap();
    f.protect(&c);
}
#[test]
fn rf903_encrypted_profile_backup_unified_objects_preserves_old_attachment() {
    let f = Fixture::new();
    let c = f.candidate();
    let data = json!({"unified_objects":[{"id":"old_owner","properties":refs()}]});
    f.vault
        .save_profile(&Profile::new_with_id(
            "historical_backup",
            "backup",
            serde_json::to_vec(&data).unwrap(),
        ))
        .unwrap();
    assert_eq!(
        f.vault
            .read_attachment_reference_view(ACCOUNT_ID)
            .unwrap()
            .stats()
            .profile_rows,
        1
    );
    f.protect(&c);
}
#[test]
fn rf903_unresolved_object_conflict_protects_both_recoverable_sides() {
    for remote_side in [false, true] {
        let f = Fixture::new();
        let c = f.candidate();
        let good = json!({"id":OWNER,"properties":refs()});
        let empty = json!({"id":OWNER,"properties":{}});
        let (local, remote) = if remote_side {
            (empty, good)
        } else {
            (good, empty)
        };
        f.conflict("objects", local, remote, false);
        assert_eq!(
            f.vault
                .read_attachment_reference_view(ACCOUNT_ID)
                .unwrap()
                .stats()
                .conflict_rows,
            1
        );
        f.protect(&c);
    }
}
#[test]
fn rf903_trash_and_profile_conflict_wire_payloads_are_fully_decoded() {
    for table in ["trash_items", "profiles"] {
        let f = Fixture::new();
        let c = f.candidate();
        let bytes = serde_json::to_vec(&json!({"properties":refs()})).unwrap();
        let payload = if table == "trash_items" {
            json!({"item_type":"page","data":bytes})
        } else {
            json!({"data":base64::engine::general_purpose::STANDARD.encode(serde_json::to_vec(&json!({"unified_objects":[{"properties":refs()}]})).unwrap())})
        };
        f.conflict(table, Value::Null, payload, false);
        f.protect(&c);
    }
}
#[test]
fn rf903_explicit_conflict_tombstone_has_no_recoverable_payload() {
    let f = Fixture::new();
    let c = f.candidate();
    f.conflict("objects", Value::Null, Value::Null, true);
    let view = f.vault.read_attachment_reference_view(ACCOUNT_ID).unwrap();
    assert_eq!(
        f.vault
            .with_unreferenced_attachment_guard(ACCOUNT_ID, &view, &c, || std::fs::remove_file(
                &c.canonical_path
            )
            .map_err(|e| e.to_string()))
            .unwrap(),
        Some(())
    );
    assert!(!c.canonical_path.exists());
}
#[test]
fn rf903_unknown_trash_or_conflict_whole_scan_fails_closed() {
    for table in ["trash", "conflict"] {
        let f = Fixture::new();
        let c = f.candidate();
        if table == "trash" {
            f.vault
                .save_trash_item(&Fixture::trash("unknown", "unknown_recovery", json!({}), 0))
                .unwrap();
        } else {
            f.conflict("unknown_recovery", json!({}), json!({}), false);
        }
        assert_eq!(
            f.vault
                .read_attachment_reference_view(ACCOUNT_ID)
                .err()
                .as_deref(),
            Some(INVALID)
        );
        assert!(c.canonical_path.exists());
    }
}
#[test]
fn rf903_bad_ciphertext_or_json_is_never_downgraded_to_empty_references() {
    for source in [
        "objects",
        "trash_items",
        "object_snapshots",
        "profiles",
        "sync_conflicts",
    ] {
        let f = Fixture::new();
        let c = f.candidate();
        // 验证已有引用后仍必须检查其余行，而不是首次 found 就提前授权/成功。
        f.vault
            .save_object(&f.record("a_first_reference", refs()))
            .unwrap();
        match source {
            "objects" => {
                f.vault.save_object(&f.record("z_bad", json!({}))).unwrap();
                f.db.execute(
                    "UPDATE objects SET properties='solo:AAAA' WHERE id='z_bad'",
                    [],
                )
                .unwrap();
            }
            "trash_items" => {
                f.vault
                    .save_trash_item(&Fixture::trash("z_bad", "object", json!({}), 0))
                    .unwrap();
                f.db.execute(
                    "UPDATE trash_items SET data=X'534F4C4F' WHERE id='z_bad'",
                    [],
                )
                .unwrap();
            }
            "object_snapshots" => {
                f.vault
                    .save_snapshot(OWNER, "user_edit", b"not-json", "invalid historical data")
                    .unwrap();
            }
            "profiles" => {
                f.vault
                    .save_profile(&Profile::new_with_id(
                        "bad",
                        "bad",
                        b"opaque unknown legacy profile".to_vec(),
                    ))
                    .unwrap();
            }
            _ => {
                f.conflict(
                    "objects",
                    json!({"properties":{}}),
                    json!({"properties":{}}),
                    false,
                );
                f.db.execute("UPDATE sync_conflicts SET remote_data='solo:AAAA'", [])
                    .unwrap();
            }
        }
        assert_eq!(
            f.vault
                .read_attachment_reference_view(ACCOUNT_ID)
                .err()
                .as_deref(),
            Some(INVALID)
        );
        assert!(c.canonical_path.exists());
    }
}
#[test]
fn rf903_duplicate_json_and_conflicting_alias_fields_fail_closed() {
    let f = Fixture::new();
    let c = f.candidate();
    f.vault.save_object(&f.record(OWNER, json!({}))).unwrap();
    f.db.execute(
        "UPDATE objects SET properties=?1 WHERE id=?2",
        params![r#"{"__attachments":[],"__attachments":[]}"#, OWNER],
    )
    .unwrap();
    assert_eq!(
        f.vault
            .read_attachment_reference_view(ACCOUNT_ID)
            .err()
            .as_deref(),
        Some(INVALID)
    );
    let mut att = meta(ATTACHMENT_ID);
    att["file_name"] = json!("different.txt");
    f.vault
        .save_object(&f.record(OWNER, json!({"__attachments":[att]})))
        .unwrap();
    assert_eq!(
        f.vault
            .read_attachment_reference_view(ACCOUNT_ID)
            .err()
            .as_deref(),
        Some(INVALID)
    );
    assert!(c.canonical_path.exists());
}
#[test]
fn rf903_new_reference_and_invalid_late_change_return_fixed_view_changed() {
    for corrupt in [false, true] {
        let f = Fixture::new();
        let c = f.candidate();
        let view = f.vault.read_attachment_reference_view(ACCOUNT_ID).unwrap();
        f.vault.save_object(&f.record(OWNER, refs())).unwrap();
        if corrupt {
            f.db.execute("UPDATE objects SET properties='bad JSON'", [])
                .unwrap();
        }
        let called = Cell::new(false);
        let error = f
            .vault
            .with_unreferenced_attachment_guard(ACCOUNT_ID, &view, &c, || {
                called.set(true);
                Ok(())
            })
            .unwrap_err();
        assert_eq!(error, CHANGED);
        assert!(!called.get());
        assert!(c.canonical_path.exists());
    }
}
#[test]
fn rf903_view_is_bound_to_account_store_and_current_unlocked_key() {
    let f = Fixture::new();
    let c = f.candidate();
    let view = f.vault.read_attachment_reference_view(ACCOUNT_ID).unwrap();
    let another = VaultStore::open_owned(
        VaultConfig::new(ACCOUNT_ID, f.vault.base_path().to_path_buf()).with_data_key(KEY),
        f.vault.root_owner(),
    )
    .unwrap();
    assert_eq!(
        another
            .with_unreferenced_attachment_guard(ACCOUNT_ID, &view, &c, || Ok(()))
            .unwrap_err(),
        CHANGED
    );
    assert_eq!(
        f.vault
            .with_unreferenced_attachment_guard("other_account", &view, &c, || Ok(()))
            .unwrap_err(),
        ACCOUNT
    );
    f.vault.lock();
    assert_eq!(
        f.vault
            .with_unreferenced_attachment_guard(ACCOUNT_ID, &view, &c, || Ok(()))
            .unwrap_err(),
        LOCKED
    );
    assert!(c.canonical_path.exists());
}
#[test]
fn rf903_final_immediate_guard_blocks_other_writer_and_only_counts_real_success() {
    let f = Fixture::new();
    let c = f.candidate();
    let view = f.vault.read_attachment_reference_view(ACCOUNT_ID).unwrap();
    let before: i64 =
        f.db.query_row("SELECT COUNT(*) FROM sys_config", [], |r| r.get(0))
            .unwrap();
    let len = std::fs::metadata(&c.canonical_path).unwrap().len();
    let result=f.vault.with_unreferenced_attachment_guard(ACCOUNT_ID,&view,&c,|| {
        let error=f.db.execute("INSERT INTO sys_config(key,value) VALUES('rf903_other_writer','blocked')",[]).unwrap_err();
        assert!(matches!(error,rusqlite::Error::SqliteFailure(ref e,_) if matches!(e.code,rusqlite::ErrorCode::DatabaseBusy|rusqlite::ErrorCode::DatabaseLocked)));
        std::fs::remove_file(&c.canonical_path).map_err(|e|e.to_string())?;
        Ok(len)
    }).unwrap();
    assert_eq!(result, Some(len));
    assert!(!c.canonical_path.exists());
    let after: i64 =
        f.db.query_row("SELECT COUNT(*) FROM sys_config", [], |r| r.get(0))
            .unwrap();
    assert_eq!(after, before);
    let c = f.candidate();
    let view = f.vault.read_attachment_reference_view(ACCOUNT_ID).unwrap();
    let error = f
        .vault
        .with_unreferenced_attachment_guard(ACCOUNT_ID, &view, &c, || {
            std::fs::remove_file(c.canonical_path.join("not-a-directory"))
                .map_err(|e| e.to_string())
        })
        .unwrap_err();
    assert!(!error.is_empty());
    assert!(c.canonical_path.exists());
}
#[test]
fn rf903_oversize_source_row_is_rejected_without_truncation_or_file_action() {
    let f = Fixture::new();
    let c = f.candidate();
    f.vault
        .save_snapshot(OWNER, "user_edit", b"{}", "bounded")
        .unwrap();
    f.db.execute(
        "UPDATE object_snapshots SET data=zeroblob(?1)",
        [(MAX_ROW_BYTES + 1) as i64],
    )
    .unwrap();
    assert_eq!(
        f.vault
            .read_attachment_reference_view(ACCOUNT_ID)
            .err()
            .as_deref(),
        Some(LIMIT)
    );
    assert!(c.canonical_path.exists());
}
#[test]
fn rf903_pending_journal_protects_published_not_yet_activated_files() {
    let f = Fixture::new();
    let c = f.candidate();
    let (_, lease) = f.accept(false);
    f.publish(&lease, &c, false);
    assert!(f
        .vault
        .load_object(OWNER)
        .unwrap()
        .unwrap()
        .properties
        .get("__attachments")
        .is_none());
    assert_eq!(
        f.vault
            .read_attachment_reference_view(ACCOUNT_ID)
            .unwrap()
            .stats()
            .journal_rows,
        1
    );
    f.protect(&c);
}
#[test]
fn rf903_complete_journal_is_not_a_permanent_reference_after_real_owner_purge() {
    let f = Fixture::new();
    let c = f.candidate();
    let (_, lease) = f.accept(false);
    f.publish(&lease, &c, true);
    f.vault.complete_import_operation(&lease).unwrap();
    f.protect(&c); // 仍活 metadata 必须保留，即使 journal 已 Complete。
    f.vault.delete_object(OWNER, false).unwrap();
    f.vault.delete_snapshots(OWNER).unwrap();
    let view = f.vault.read_attachment_reference_view(ACCOUNT_ID).unwrap();
    assert_eq!(view.stats().journal_rows, 1);
    assert_eq!(view.stats().object_rows, 0);
    assert_eq!(
        f.vault
            .with_unreferenced_attachment_guard(ACCOUNT_ID, &view, &c, || std::fs::remove_file(
                &c.canonical_path
            )
            .map_err(|e| e.to_string()))
            .unwrap(),
        Some(())
    );
    assert!(!c.canonical_path.exists());
}
#[test]
fn rf903_abandoned_activated_metadata_protects_but_unactivated_plan_does_not() {
    for activated in [false, true] {
        let f = Fixture::new();
        let c = f.candidate();
        let (_, lease) = f.accept(false);
        f.publish(&lease, &c, activated);
        f.vault.abandon_import_operation(&lease).unwrap();
        if activated {
            f.protect(&c);
        } else {
            let view = f.vault.read_attachment_reference_view(ACCOUNT_ID).unwrap();
            assert_eq!(
                f.vault
                    .with_unreferenced_attachment_guard(ACCOUNT_ID, &view, &c, || {
                        std::fs::remove_file(&c.canonical_path).map_err(|e| e.to_string())
                    })
                    .unwrap(),
                Some(())
            );
            assert!(!c.canonical_path.exists());
        }
    }
}
#[test]
fn rf903_corrupt_journal_and_orphan_step_fail_closed() {
    for step in [false, true] {
        let f = Fixture::new();
        let c = f.candidate();
        let (start, _) = f.accept(false);
        if step {
            f.db.execute(
                "UPDATE import_attachment_steps SET step_enc='solo:AAAA' WHERE operation_id=?1",
                [&start.operation_id],
            )
            .unwrap();
        } else {
            f.db.execute(
                "UPDATE import_operations SET plan_enc='solo:AAAA' WHERE operation_id=?1",
                [&start.operation_id],
            )
            .unwrap();
        }
        assert_eq!(
            f.vault
                .read_attachment_reference_view(ACCOUNT_ID)
                .err()
                .as_deref(),
            Some(INVALID)
        );
        assert!(c.canonical_path.exists());
    }
    let f = Fixture::new();
    let c = f.candidate();
    f.db.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
    f.db.execute("INSERT INTO import_attachment_steps(operation_id,entry_ordinal,phase,step_enc) VALUES(?1,0,'planned','solo:AAAA')",[uuid::Uuid::new_v4().to_string()]).unwrap();
    assert_eq!(
        f.vault
            .read_attachment_reference_view(ACCOUNT_ID)
            .err()
            .as_deref(),
        Some(INVALID)
    );
    assert!(c.canonical_path.exists());
}
#[test]
fn rf903_source_dependent_pending_preferences_refuse_unknown_reference_set() {
    let f = Fixture::new();
    let c = f.candidate();
    f.accept(true);
    assert_eq!(
        f.vault
            .read_attachment_reference_view(ACCOUNT_ID)
            .err()
            .as_deref(),
        Some(UNAVAILABLE)
    );
    assert!(c.canonical_path.exists());
}
#[test]
fn rf903_ready_pending_preferences_are_strictly_decoded_and_protected() {
    let f = Fixture::new();
    let c = f.candidate();
    let mut start = ImportOperationStart {
        operation_id: uuid::Uuid::new_v4().to_string(),
        source_kind: ImportSourceKind::Recovery,
        source: ImportSourceProof {
            sha256: "a".repeat(64),
            length: 100,
        },
        root_binding: f.vault.import_root_binding().unwrap(),
        request_fingerprint: "b".repeat(64),
        plan: json!({"preferences":{"kind":"ready","data":serde_json::to_vec(&json!({"unified_objects":[{"properties":refs()}]})).unwrap()}}),
        owners: vec![],
        steps: vec![],
        preferences_required: true,
        source_ready: Some(json!({"version":1,"sourceIndependent":true,"preferencesReady":true})),
    };
    let old = f.vault.read_import_view(ACCOUNT_ID).unwrap();
    f.vault
        .commit_import_batch_with_operation(
            ACCOUNT_ID,
            &old.revision,
            &ImportDatabaseBatch::default(),
            &start,
        )
        .unwrap();
    f.protect(&c);
    // 同结构、坏 ready JSON 也不能被通用字符串 walker 当作无引用。
    start.operation_id = uuid::Uuid::new_v4().to_string();
    start.plan["preferences"]["data"] = json!([255, 0]);
    let old = f.vault.read_import_view(ACCOUNT_ID).unwrap();
    f.vault
        .commit_import_batch_with_operation(
            ACCOUNT_ID,
            &old.revision,
            &ImportDatabaseBatch::default(),
            &start,
        )
        .unwrap();
    assert_eq!(
        f.vault
            .read_attachment_reference_view(ACCOUNT_ID)
            .err()
            .as_deref(),
        Some(INVALID)
    );
    assert!(c.canonical_path.exists());
}
#[test]
fn rf903_authentication_is_real_and_ciphertext_body_tamper_fails_closed() {
    let f = Fixture::new();
    let c = f.candidate();
    let mut encrypted = encrypt_field(
        &DataEncryptionKey(KEY),
        &serde_json::to_vec(&json!({"properties":refs()})).unwrap(),
    )
    .unwrap();
    let last = encrypted.len() - 1;
    encrypted[last] ^= 1;
    f.vault
        .save_snapshot(OWNER, "user_edit", b"{}", "tampered")
        .unwrap();
    f.db.execute("UPDATE object_snapshots SET data=?1", [encrypted])
        .unwrap();
    assert_eq!(
        f.vault
            .read_attachment_reference_view(ACCOUNT_ID)
            .err()
            .as_deref(),
        Some(INVALID)
    );
    assert!(c.canonical_path.exists());
    // 非 AEAD 的 solo:base64(JSON) 必须被拒绝，不能触发旧透明解密的 plaintext fallback。
    f.db.execute("DELETE FROM object_snapshots", []).unwrap();
    let fake = format!(
        "solo:{}",
        base64::engine::general_purpose::STANDARD.encode(b"{}")
    );
    f.vault.save_object(&f.record(OWNER, json!({}))).unwrap();
    f.db.execute("UPDATE objects SET properties=?1", [fake])
        .unwrap();
    assert_eq!(
        f.vault
            .read_attachment_reference_view(ACCOUNT_ID)
            .err()
            .as_deref(),
        Some(INVALID)
    );
    assert!(c.canonical_path.exists());
}
#[test]
fn rf903_journal_unknown_fields_and_bad_remote_null_cannot_hide_recovery_references() {
    let f = Fixture::new();
    let c = f.candidate();
    let (start, _) = f.accept(false);
    let mut envelope = json!({"account":ACCOUNT_ID,"id":start.operation_id,"payload":start});
    envelope["payload"]["unrecognizedRecovery"] = json!({"attachments":[ATTACHMENT_ID]});
    let encoded = encrypt_text_field(
        &DataEncryptionKey(KEY),
        &serde_json::to_string(&envelope).unwrap(),
    )
    .unwrap();
    f.db.execute("UPDATE import_operations SET plan_enc=?1", [encoded])
        .unwrap();
    assert_eq!(
        f.vault
            .read_attachment_reference_view(ACCOUNT_ID)
            .err()
            .as_deref(),
        Some(INVALID)
    );
    assert!(c.canonical_path.exists());
    let f = Fixture::new();
    let c = f.candidate();
    f.conflict("objects", Value::Null, Value::Null, false);
    assert_eq!(
        f.vault
            .read_attachment_reference_view(ACCOUNT_ID)
            .err()
            .as_deref(),
        Some(INVALID)
    );
    assert!(c.canonical_path.exists());
}

#[test]
fn rf903_local_profile_byte_array_and_remote_conversation_envelopes_protect_refs() {
    for table in ["profiles", "llm_conversations"] {
        let f = Fixture::new();
        let c = f.candidate();
        let content = if table == "profiles" {
            json!({"unified_objects":[{"properties":refs()}]})
        } else {
            json!({"id":"conversation","messages":[{"properties":refs()}]})
        };
        let bytes = serde_json::to_vec(&content).unwrap();
        let payload = if table == "profiles" {
            json!({"id":"profile","data":bytes})
        } else {
            let cipher = encrypt_field(&DataEncryptionKey(KEY), &bytes).unwrap();
            json!({"id":"conversation","accountId":ACCOUNT_ID,"updatedAt":NOW,"data":base64::engine::general_purpose::STANDARD.encode(cipher)})
        };
        f.conflict(
            table,
            if table == "profiles" {
                payload.clone()
            } else {
                content
            },
            payload,
            false,
        );
        f.protect(&c);
    }
}
#[test]
fn rf903_unknown_snapshot_envelope_fails_closed() {
    let f = Fixture::new();
    let c = f.candidate();
    let unknown = json!({"futureRecovery":{"data":base64::engine::general_purpose::STANDARD.encode(serde_json::to_vec(&refs()).unwrap())}});
    let bytes = serde_json::to_vec(&unknown).unwrap();
    f.vault
        .save_snapshot(OWNER, "user_edit", &bytes, "unknown future envelope")
        .unwrap();
    assert_eq!(
        f.vault
            .read_attachment_reference_view(ACCOUNT_ID)
            .err()
            .as_deref(),
        Some(INVALID)
    );
    assert!(c.canonical_path.exists());
}
#[test]
fn rf903_metadata_only_import_objects_preserve_alias_before_any_file_step() {
    let f = Fixture::new();
    let c = f.candidate();
    let mut att = meta("imported_alias_id");
    att["objectId"] = json!("imported_owner");
    att["vaultPath"] = json!(c.canonical_path.to_string_lossy());
    let start = ImportOperationStart {
        operation_id: uuid::Uuid::new_v4().to_string(),
        source_kind: ImportSourceKind::Manual,
        source: ImportSourceProof {
            sha256: "a".repeat(64),
            length: 100,
        },
        root_binding: f.vault.import_root_binding().unwrap(),
        request_fingerprint: "b".repeat(64),
        plan: json!({"preferences":{"kind":"none"}}),
        owners: vec![],
        steps: vec![],
        preferences_required: false,
        source_ready: None,
    };
    let view = f.vault.read_import_view(ACCOUNT_ID).unwrap();
    let batch = ImportDatabaseBatch {
        templates: vec![],
        objects: vec![ImportObjectWrite {
            record: f.record("imported_owner", json!({"__attachments":[att]})),
            history: ImportHistoryChange::Keep,
        }],
    };
    f.vault
        .commit_import_batch_with_operation(ACCOUNT_ID, &view.revision, &batch, &start)
        .unwrap();
    let lease = f
        .vault
        .claim_import_operation(ACCOUNT_ID, &start.operation_id, &start.root_binding)
        .unwrap();
    f.vault.complete_import_operation(&lease).unwrap();
    f.protect(&c);
}
#[test]
fn rf903_known_empty_profile_does_not_disable_historical_orphan_cleanup() {
    let f = Fixture::new();
    let c = f.candidate();
    f.vault
        .save_profile(&Profile::new_with_id("empty", "empty", vec![]))
        .unwrap();
    let view = f.vault.read_attachment_reference_view(ACCOUNT_ID).unwrap();
    assert_eq!(
        f.vault
            .with_unreferenced_attachment_guard(ACCOUNT_ID, &view, &c, || std::fs::remove_file(
                &c.canonical_path
            )
            .map_err(|e| e.to_string()))
            .unwrap(),
        Some(())
    );
    assert!(!c.canonical_path.exists());
}

#[test]
fn rf903_page_child_trash_parent_fields_match_actual_gui_writer_contract() {
    let f = Fixture::new();
    let c = f.candidate();
    // Host build_page_delete_trash_items 的真实已发布合同：仅 child 加这两个恢复字段。
    // 此 crate 回归使用真实 Vault 加密存储；不冒充执行 Host 私有 producer。
    let mut item = Fixture::trash("page_child", "object", refs(), 0);
    let mut value: Value = serde_json::from_slice(&item.data).unwrap();
    value["parentPageName"] = json!("recoverable parent page");
    value["parentPageIcon"] = json!("folder");
    item.data = serde_json::to_vec(&value).unwrap();
    f.vault.save_trash_item(&item).unwrap();
    f.protect(&c);
    for bad in [json!({"unknown":"wrapper"}), json!(null)] {
        value["parentPageName"] = bad;
        let encoded = encrypt_field(
            &DataEncryptionKey(KEY),
            &serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();
        f.db.execute(
            "UPDATE trash_items SET data=?1 WHERE id='page_child'",
            [encoded],
        )
        .unwrap();
        assert_eq!(
            f.vault
                .read_attachment_reference_view(ACCOUNT_ID)
                .err()
                .as_deref(),
            Some(INVALID)
        );
        assert!(c.canonical_path.exists());
    }
}
#[test]
fn rf903_profile_custom_json_remains_free_but_recoverable_refs_are_protected() {
    let f = Fixture::new();
    let c = f.candidate();
    let custom = json!({"custom":{"value":"free text","nested":{"__attachments":[meta(ATTACHMENT_ID)]}},"anotherFreeRoot":[1,true,{"value":"content"}]});
    f.vault
        .save_profile(&Profile::new_with_id(
            "custom_profile",
            "custom",
            serde_json::to_vec(&custom).unwrap(),
        ))
        .unwrap();
    f.protect(&c);
    let mut alias = meta("new_id");
    alias["vault_path"] = json!(c.canonical_path.to_string_lossy());
    let custom = json!({"arbitrary":{"__attachments":[alias]}});
    f.vault
        .save_profile(&Profile::new_with_id(
            "custom_profile",
            "custom",
            serde_json::to_vec(&custom).unwrap(),
        ))
        .unwrap();
    f.protect(&c);
    f.vault
        .save_profile(&Profile::new_with_id(
            "custom_profile",
            "custom",
            b"not valid profile JSON".to_vec(),
        ))
        .unwrap();
    assert_eq!(
        f.vault
            .read_attachment_reference_view(ACCOUNT_ID)
            .err()
            .as_deref(),
        Some(INVALID)
    );
    assert!(c.canonical_path.exists());
}
#[test]
fn rf903_real_v21_migration_missing_local_history_keeps_current_reference() {
    let mut f = Fixture::new();
    let c = f.candidate();
    f.vault.save_object(&f.record(OWNER, refs())).unwrap();
    f.conflict(
        "objects",
        json!({"properties":{}}),
        json!({"properties":{}}),
        false,
    );
    // 真实 schema migration 重新建立 local_data DEFAULT '{}'，不靠 mocked parser。
    f.db.execute_batch("ALTER TABLE sync_conflicts DROP COLUMN local_data; DELETE FROM schema_migrations WHERE version>=21; UPDATE sys_config SET value='20' WHERE key='data_version';").unwrap();
    crate::migration::run_migrations(&mut f.db).unwrap();
    let local: String =
        f.db.query_row("SELECT local_data FROM sync_conflicts", [], |r| r.get(0))
            .unwrap();
    assert_eq!(local, "{}");
    f.protect(&c);
}
#[test]
fn rf903_missing_local_history_empty_sentinel_does_not_relax_remote_validation() {
    for local in ["", "{}"] {
        let f = Fixture::new();
        let c = f.candidate();
        f.vault.save_object(&f.record(OWNER, refs())).unwrap();
        f.conflict("objects", Value::Null, json!({"properties":{}}), false);
        f.db.execute("UPDATE sync_conflicts SET local_data=?1", [local])
            .unwrap();
        f.protect(&c);
        // remote '{}' 是缺完整可恢复对象的坏 payload，绝不能借本地哨兵规则当作空引用。
        let encoded = encrypt_text_field(&DataEncryptionKey(KEY), "{}").unwrap();
        f.db.execute("UPDATE sync_conflicts SET remote_data=?1", [encoded])
            .unwrap();
        assert_eq!(
            f.vault
                .read_attachment_reference_view(ACCOUNT_ID)
                .err()
                .as_deref(),
            Some(INVALID)
        );
        assert!(c.canonical_path.exists());
    }
}

#[test]
fn rf903_normal_none_labels_and_authenticated_empty_labels_are_not_bad_data() {
    for encrypted_empty in [false, true] {
        let f = Fixture::new();
        let c = f.candidate();
        let record = f.record(OWNER, json!({}));
        assert!(record.property_labels.is_none());
        f.vault.save_object(&record).unwrap();
        let stored: String =
            f.db.query_row(
                "SELECT property_labels FROM objects WHERE id=?1",
                [OWNER],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stored, ""); // 真实默认 None writer 采用 TEXT ''，不是 SQL NULL。
        if encrypted_empty {
            let cipher = solosoul_crypto::aes::encrypt_blob(&KEY, b"").unwrap();
            let encoded = format!(
                "solo:{}",
                base64::engine::general_purpose::STANDARD.encode(cipher.as_slice())
            );
            f.db.execute("UPDATE objects SET property_labels=?1", [encoded])
                .unwrap();
            assert!(f
                .vault
                .load_object(OWNER)
                .unwrap()
                .unwrap()
                .property_labels
                .is_none());
        }
        let view = f.vault.read_attachment_reference_view(ACCOUNT_ID).unwrap();
        assert_eq!(view.stats().object_rows, 1);
        assert_eq!(
            f.vault
                .with_unreferenced_attachment_guard(ACCOUNT_ID, &view, &c, || std::fs::remove_file(
                    &c.canonical_path
                )
                .map_err(|e| e.to_string()))
                .unwrap(),
            Some(())
        );
        assert!(!c.canonical_path.exists());
    }
}
#[test]
fn rf903_nonempty_invalid_labels_remain_fail_closed() {
    for invalid in [
        "not JSON",
        "solo:AAAA",
        "[]",
        r#""string labels are invalid""#,
    ] {
        let f = Fixture::new();
        let c = f.candidate();
        f.vault.save_object(&f.record(OWNER, json!({}))).unwrap();
        f.db.execute("UPDATE objects SET property_labels=?1", [invalid])
            .unwrap();
        assert_eq!(
            f.vault
                .read_attachment_reference_view(ACCOUNT_ID)
                .err()
                .as_deref(),
            Some(INVALID)
        );
        assert!(c.canonical_path.exists());
    }
}
