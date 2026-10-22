use super::*;
use crate::VaultConfig;
use tempfile::TempDir;

fn fixture() -> (VaultStore, TempDir, ObjectRecord) {
    let dir = TempDir::new().unwrap();
    let vault = VaultStore::open(
        VaultConfig::new("public-account", dir.path().to_path_buf()).with_data_key([42; 32]),
    )
    .unwrap();
    let record = ObjectRecord {
        id: "public-object".into(),
        account_id: "public-account".into(),
        type_id: "identity".into(),
        name: "Public object".into(),
        properties: serde_json::json!({"name":"Public","__attachments":[]}),
        ..Default::default()
    };
    vault.save_object(&record).unwrap();
    (vault, dir, record)
}

#[test]
fn form_updates_preserve_latest_attachments_and_internal_metadata_without_forging_protection() {
    let old = serde_json::json!({"__editLocked":false,"__attachments":[{"id":"latest"}],"__deprecatedFields":{"old":"Public archived value"},"__fields":{"__group":{"type":"dynamic_group"}},"__group":["old"]});
    let mut incoming = serde_json::json!({"name":"Changed","__editLocked":true,"__attachments":[]});
    preserve_object_edit_metadata(&old, &mut incoming);
    assert_eq!(incoming["__editLocked"], false);
    assert_eq!(incoming["__attachments"], old["__attachments"]);
    assert_eq!(incoming["__deprecatedFields"], old["__deprecatedFields"]);
    assert!(incoming.get("__group").is_none());
    let mut forged = serde_json::json!({"__editLocked":true,"__attachments":[{}]});
    preserve_object_edit_metadata(&serde_json::json!({}), &mut forged);
    assert_eq!(forged, serde_json::json!({}));
}

#[test]
fn object_lock_persists_without_hiding_content_and_only_explicit_unlock_allows_writes() {
    let (vault, dir, original) = fixture();
    let locked = vault.set_object_edit_lock(&original.id, true).unwrap();
    assert!(object_edit_locked(&locked.properties));
    assert_eq!(locked.properties["name"], original.properties["name"]);
    let repeated = vault.set_object_edit_lock(&original.id, true).unwrap();
    assert_eq!(repeated.version, locked.version);
    let mut forged = locked.clone();
    forged.name = "Changed".into();
    forged
        .properties
        .as_object_mut()
        .unwrap()
        .remove(OBJECT_EDIT_LOCK_KEY);
    assert_eq!(vault.save_object(&forged).unwrap_err(), OBJECT_EDIT_LOCKED);
    assert_eq!(
        vault.delete_object(&original.id, true).unwrap_err(),
        OBJECT_EDIT_LOCKED
    );
    assert_eq!(
        vault.delete_object(&original.id, false).unwrap_err(),
        OBJECT_EDIT_LOCKED
    );
    drop(vault);
    let reopened = VaultStore::open(
        VaultConfig::new("public-account", dir.path().to_path_buf()).with_data_key([42; 32]),
    )
    .unwrap();
    let read = reopened.load_object(&original.id).unwrap().unwrap();
    assert_eq!(read.name, original.name);
    assert!(object_edit_locked(&read.properties));
    assert!(object_edit_locked(
        &reopened
            .list_objects("public-account", None, None, None, false, false)
            .unwrap()[0]
            .properties
    ));
    let mut unlocked = reopened.set_object_edit_lock(&original.id, false).unwrap();
    unlocked.name = "Changed after unlocking".into();
    reopened.save_object(&unlocked).unwrap();
    assert_eq!(
        reopened.load_object(&original.id).unwrap().unwrap().name,
        unlocked.name
    );
}

#[test]
fn object_lock_rolls_back_batch_updates_and_trash_without_partial_changes() {
    let (vault, _dir, original) = fixture();
    let second = ObjectRecord {
        id: "public-second".into(),
        name: "Second".into(),
        ..original.clone()
    };
    vault.save_object(&second).unwrap();
    vault.set_object_edit_lock(&original.id, true).unwrap();
    let changed = ObjectRecord {
        name: "Should roll back".into(),
        ..second.clone()
    };
    assert_eq!(
        vault
            .save_objects_batch(&[changed, original.clone()])
            .unwrap_err(),
        OBJECT_EDIT_LOCKED
    );
    assert_eq!(
        vault.load_object(&second.id).unwrap().unwrap().name,
        second.name
    );
    assert_eq!(
        vault
            .trash_and_soft_delete_batch(&[], &[second.id.clone(), original.id.clone()])
            .unwrap_err(),
        OBJECT_EDIT_LOCKED
    );
    assert!(!vault.load_object(&second.id).unwrap().unwrap().is_deleted);
}

#[test]
fn object_lock_rejects_foreign_pages_and_invalid_protection_does_not_fail_open() {
    let (vault, _dir, original) = fixture();
    let foreign = ObjectRecord {
        id: "foreign".into(),
        account_id: "other-account".into(),
        ..original.clone()
    };
    vault.save_object(&foreign).unwrap();
    assert!(vault.set_object_edit_lock(&foreign.id, true).is_err());
    let page = ObjectRecord {
        id: "page".into(),
        type_id: "page".into(),
        ..original
    };
    vault.save_object(&page).unwrap();
    assert!(vault.set_object_edit_lock(&page.id, true).is_err());
    assert!(object_edit_locked(
        &serde_json::json!({"__editLocked":null})
    ));
    assert!(!object_edit_locked(&serde_json::json!({})));
    assert!(!object_edit_locked(
        &serde_json::json!({"__editLocked":false})
    ));
}

#[test]
fn locked_attachment_removal_creates_no_cleanup_permission() {
    let (vault, _dir, mut record) = fixture();
    record.properties["__attachments"] = serde_json::json!([{"id":"public-attachment","fileName":"public.txt","mimeType":"text/plain","createdAt":"2026-10-23","sizeBytes":10}]);
    vault.save_object(&record).unwrap();
    vault.set_object_edit_lock(&record.id, true).unwrap();
    assert_eq!(
        vault
            .queue_attachment_deletions("public-account", &record.id, &["public-attachment".into()])
            .unwrap_err(),
        OBJECT_EDIT_LOCKED
    );
    assert!(vault
        .list_attachment_cleanup_intents("public-account")
        .unwrap()
        .is_empty());
    assert_eq!(
        vault.load_object(&record.id).unwrap().unwrap().properties["__attachments"],
        record.properties["__attachments"]
    );
}

#[test]
fn trusted_sync_propagates_explicit_unlock_without_wedging_locked_replicas() {
    let (vault, _dir, original) = fixture();
    vault.set_object_edit_lock(&original.id, true).unwrap();
    let mut remote = original;
    remote.properties["__editLocked"] = serde_json::json!(false);
    let record = crate::VaultSyncRecord {
        id: remote.id.clone(),
        table: "objects".into(),
        deleted: false,
        hlc: crate::RecordHlc {
            wall_time_ms: 2_000_000_000_000_000,
            counter: 0,
            node_id: "remote".into(),
        },
        data: serde_json::to_value(remote).unwrap(),
    };
    assert!(vault.apply_sync_record(&record, "local").unwrap());
    let mut read = vault.load_object(&record.id).unwrap().unwrap();
    assert!(!object_edit_locked(&read.properties));
    read.name = "Editable after remote unlock".into();
    vault.save_object(&read).unwrap();
}
