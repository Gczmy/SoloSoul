use super::super::{mutate_object_fields, BackendError, Code, UpdateObjectInput};
use super::setup_vault;
use crate::commands::error::BackendErrorStage as Stage;
use serde_json::json;
use solosoul_vault::{ObjectRecord, VaultStore};

fn seed(vault: &VaultStore) -> ObjectRecord {
    let record = ObjectRecord {
        id: "public-object".into(),
        account_id: "test_account".into(),
        type_id: "identity".into(),
        name: "Original".into(),
        sensitivity_level: "internal".into(),
        properties: json!({
            "value":"original","__attachments":[{"id":"latest-attachment"}],
            "__fields":{"value":{"name":"Public field","type":"text"}},
            "__templateName":"Public template"
        }),
        version: 1,
        ..Default::default()
    };
    vault.save_object(&record).unwrap();
    record
}

fn input() -> UpdateObjectInput {
    UpdateObjectInput {
        name: "Edited".into(),
        properties: json!({"value":"edited","__attachments":[],"__editLocked":true}),
        sensitivity_level: Some("public".into()),
        icon_name: Some("star".into()),
    }
}

#[test]
fn gui_edit_uses_committed_attachment_metadata_and_cannot_forge_lock_state() {
    let (vault, _dir) = setup_vault();
    seed(&vault);
    let (record, old_sensitivity) = mutate_object_fields(&vault, "public-object", input()).unwrap();
    assert_eq!(old_sensitivity, "internal");
    assert_eq!(record.version, 2);
    assert_eq!(record.name, "Edited");
    assert_eq!(record.properties["value"], "edited");
    assert_eq!(
        record.properties["__attachments"][0]["id"],
        "latest-attachment"
    );
    assert_eq!(record.properties["__templateName"], "Public template");
    assert!(record.properties["__fields"].is_object());
    assert!(!solosoul_vault::storage::object_edit_locked(
        &record.properties
    ));
    assert_eq!(
        vault
            .load_object("public-object")
            .unwrap()
            .unwrap()
            .properties,
        record.properties
    );
}

#[test]
fn missing_and_locked_objects_keep_existing_typed_error_stages() {
    let (vault, _dir) = setup_vault();
    assert_eq!(
        mutate_object_fields(&vault, "missing", input())
            .unwrap_err()
            .code,
        Code::ObjectNotFound
    );
    seed(&vault);
    let locked = vault.set_object_edit_lock("public-object", true).unwrap();
    assert_eq!(
        mutate_object_fields(&vault, "public-object", input()).unwrap_err(),
        BackendError::new(Code::ObjectLocked).at(Stage::Write)
    );
    assert_eq!(
        vault.load_object("public-object").unwrap().unwrap().version,
        locked.version
    );
}

#[test]
fn read_and_write_failures_are_distinct_and_failed_commit_does_not_change_the_object() {
    let (vault, dir) = setup_vault();
    let original = seed(&vault);
    let db = rusqlite::Connection::open(dir.path().join("vault.db")).unwrap();
    db.execute_batch("CREATE TRIGGER public_write_failure BEFORE INSERT ON sync_hlc WHEN NEW.table_name='objects' BEGIN SELECT RAISE(ABORT,'public failure'); END;").unwrap();
    let error = mutate_object_fields(&vault, "public-object", input()).unwrap_err();
    assert_eq!(error.code, Code::ObjectWriteFailed);
    assert_eq!(error.safe_details.unwrap().stage, Stage::Write);
    let saved = vault.load_object("public-object").unwrap().unwrap();
    assert_eq!(saved.name, original.name);
    assert_eq!(saved.version, original.version);
    db.execute(
        "UPDATE objects SET properties='corrupted' WHERE id='public-object'",
        [],
    )
    .unwrap();
    let error = mutate_object_fields(&vault, "public-object", input()).unwrap_err();
    assert_eq!(error.code, Code::ObjectReadFailed);
    assert_eq!(error.safe_details.unwrap().stage, Stage::Read);
}
