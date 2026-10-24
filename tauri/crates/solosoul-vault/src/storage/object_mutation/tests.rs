use super::*;
use crate::VaultConfig;
use serde_json::json;
use std::sync::{Arc, Barrier};

fn fixture() -> (VaultStore, tempfile::TempDir, ObjectRecord) {
    let dir = tempfile::TempDir::new().unwrap();
    let vault = VaultStore::open(
        VaultConfig::new("public-account", dir.path().to_path_buf()).with_data_key([42; 32]),
    )
    .unwrap();
    let record = ObjectRecord {
        id: "public-object".into(),
        account_id: "public-account".into(),
        type_id: "identity".into(),
        name: "Public object".into(),
        properties: json!({"value":"original","__attachments":[]}),
        version: 1,
        ..Default::default()
    };
    vault.save_object(&record).unwrap();
    (vault, dir, record)
}

#[test]
fn concurrent_attachment_edits_keep_both_references_and_advance_versions_and_hlc() {
    let (vault, _dir, original) = fixture();
    let before = vault
        .get_record_hlc("objects", &original.id)
        .unwrap()
        .unwrap();
    let vault = Arc::new(vault);
    let start = Arc::new(Barrier::new(3));
    let threads: Vec<_> = ["attachment-a", "attachment-b"]
        .into_iter()
        .map(|id| {
            let vault = vault.clone();
            let start = start.clone();
            std::thread::spawn(move || {
                start.wait();
                vault
                    .mutate_object("public-object", |record| -> Result<_, String> {
                        record.properties["__attachments"]
                            .as_array_mut()
                            .unwrap()
                            .push(json!({"id":id}));
                        Ok(true)
                    })
                    .unwrap()
                    .unwrap()
                    .version
            })
        })
        .collect();
    start.wait();
    let mut versions: Vec<_> = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect();
    versions.sort();
    assert_eq!(versions, [2, 3]);
    let saved = vault.load_object(&original.id).unwrap().unwrap();
    let mut ids: Vec<_> = saved.properties["__attachments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|attachment| attachment["id"].as_str().unwrap())
        .collect();
    ids.sort();
    assert_eq!(ids, ["attachment-a", "attachment-b"]);
    assert_eq!(saved.version, 3);
    let after = vault
        .get_record_hlc("objects", &original.id)
        .unwrap()
        .unwrap();
    assert!(after.wall_time_ms > before.wall_time_ms);
}

#[test]
fn rejected_edits_noops_missing_records_and_identity_changes_do_not_commit() {
    let (vault, _dir, original) = fixture();
    let before = vault.get_record_hlc("objects", &original.id).unwrap();
    assert!(matches!(
        vault.mutate_object(&original.id, |record| {
            record.name = "Rejected".into();
            Err::<bool, _>("invalid edit".to_string())
        }),
        Err(ObjectMutationError::Edit(_))
    ));
    assert!(vault
        .mutate_object(&original.id, |record| -> Result<_, String> {
            record.name = "Discarded no-op".into();
            Ok(false)
        })
        .unwrap()
        .is_none());
    assert!(vault
        .mutate_object("missing", |_| -> Result<bool, String> {
            panic!("missing object")
        })
        .unwrap()
        .is_none());
    for field in ["id", "account", "created"] {
        assert!(matches!(
            vault.mutate_object(&original.id, |record| -> Result<_, String> {
                match field {
                    "id" => record.id = "forged".into(),
                    "account" => record.account_id = "other-account".into(),
                    _ => record.created_at = "changed".into(),
                }
                Ok(true)
            }),
            Err(ObjectMutationError::Write(_))
        ));
    }
    assert_eq!(
        vault.load_object(&original.id).unwrap().unwrap().name,
        original.name
    );
    assert_eq!(
        vault.get_record_hlc("objects", &original.id).unwrap(),
        before
    );
}

#[test]
fn hlc_failure_rolls_back_the_object_and_preserves_the_previous_clock() {
    let (vault, _dir, original) = fixture();
    let before = vault.get_record_hlc("objects", &original.id).unwrap();
    vault
        .conn
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .execute_batch(
            "CREATE TRIGGER public_hlc_failure BEFORE INSERT ON sync_hlc
         WHEN NEW.table_name='objects' BEGIN SELECT RAISE(ABORT, 'public failure'); END;",
        )
        .unwrap();
    let error = vault
        .mutate_object(&original.id, |record| -> Result<_, String> {
            record.name = "Must roll back".into();
            Ok(true)
        })
        .unwrap_err();
    assert!(matches!(error, ObjectMutationError::Write(_)));
    let saved = vault.load_object(&original.id).unwrap().unwrap();
    assert_eq!(saved.name, original.name);
    assert_eq!(saved.version, original.version);
    assert_eq!(
        vault.get_record_hlc("objects", &original.id).unwrap(),
        before
    );
}

#[test]
fn edit_cannot_forge_protection_or_overwrite_a_concurrently_locked_record() {
    let (vault, _dir, original) = fixture();
    let edited = vault
        .mutate_object(&original.id, |record| -> Result<_, String> {
            record.properties[OBJECT_EDIT_LOCK_KEY] = json!(true);
            Ok(true)
        })
        .unwrap()
        .unwrap();
    assert!(!object_edit_locked(&edited.properties));
    let locked = vault.set_object_edit_lock(&original.id, true).unwrap();
    let before = vault.get_record_hlc("objects", &original.id).unwrap();
    let result = vault.mutate_object(&original.id, |record| -> Result<_, String> {
        record.name = "Must not change".into();
        record.properties[OBJECT_EDIT_LOCK_KEY] = json!(false);
        Ok(true)
    });
    assert_eq!(String::from(result.unwrap_err()), OBJECT_EDIT_LOCKED);
    let saved = vault.load_object(&original.id).unwrap().unwrap();
    assert_eq!(saved.name, original.name);
    assert_eq!(saved.version, locked.version);
    assert!(object_edit_locked(&saved.properties));
    assert_eq!(
        vault.get_record_hlc("objects", &original.id).unwrap(),
        before
    );
}

#[test]
fn lock_waits_for_the_whole_mutation_and_preserves_its_committed_content() {
    let (vault, _dir, original) = fixture();
    let vault = Arc::new(vault);
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let edit = {
        let vault = vault.clone();
        std::thread::spawn(move || {
            vault
                .mutate_object("public-object", |record| -> Result<_, String> {
                    record.name = "Committed before lock".into();
                    entered_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    Ok(true)
                })
                .unwrap()
                .unwrap()
        })
    };
    entered_rx.recv().unwrap();
    let lock = {
        let vault = vault.clone();
        std::thread::spawn(move || vault.set_object_edit_lock("public-object", true).unwrap())
    };
    release_tx.send(()).unwrap();
    assert_eq!(edit.join().unwrap().version, original.version + 1);
    let locked = lock.join().unwrap();
    assert_eq!(locked.version, original.version + 2);
    assert_eq!(locked.name, "Committed before lock");
    assert!(object_edit_locked(&locked.properties));
}

#[test]
fn version_overflow_does_not_write_or_advance_hlc() {
    let (vault, _dir, mut original) = fixture();
    original.version = u32::MAX;
    vault.save_object(&original).unwrap();
    let before = vault.get_record_hlc("objects", &original.id).unwrap();
    assert!(matches!(
        vault.mutate_object(&original.id, |record| -> Result<_, String> {
            record.name = "Must not commit".into();
            Ok(true)
        }),
        Err(ObjectMutationError::VersionOverflow)
    ));
    assert_eq!(
        vault.load_object(&original.id).unwrap().unwrap().name,
        original.name
    );
    assert_eq!(
        vault.get_record_hlc("objects", &original.id).unwrap(),
        before
    );
}

#[test]
fn independent_connections_reload_under_the_write_transaction() {
    let (vault, dir, original) = fixture();
    let first = Arc::new(vault);
    let second = Arc::new(
        VaultStore::open_owned(
            VaultConfig::new("public-account", dir.path().to_path_buf()).with_data_key([42; 32]),
            first.root_owner(),
        )
        .unwrap(),
    );
    let start = Arc::new(Barrier::new(3));
    let workers: Vec<_> = [first.clone(), second.clone()]
        .into_iter()
        .enumerate()
        .map(|(index, vault)| {
            let start = start.clone();
            std::thread::spawn(move || {
                start.wait();
                vault
                    .mutate_object("public-object", |record| -> Result<_, String> {
                        record.properties["__attachments"]
                            .as_array_mut()
                            .unwrap()
                            .push(json!({"id":index}));
                        Ok(true)
                    })
                    .unwrap()
                    .unwrap()
            })
        })
        .collect();
    start.wait();
    let mut versions: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap().version)
        .collect();
    versions.sort();
    assert_eq!(versions, [original.version + 1, original.version + 2]);
    let saved = first.load_object(&original.id).unwrap().unwrap();
    assert_eq!(
        saved.properties["__attachments"].as_array().unwrap().len(),
        2
    );
}
