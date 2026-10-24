use super::*;
use serde_json::json;
use solosoul_vault::{ObjectRecord, VaultConfig, VaultStore};
use std::sync::{Arc, Barrier};

fn fixture() -> (VaultStore, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().unwrap();
    let vault = VaultStore::open(
        VaultConfig::new("public-account", dir.path().to_path_buf()).with_data_key([42; 32]),
    )
    .unwrap();
    vault.save_object(&ObjectRecord {
        id: "public-object".into(),
        account_id: "public-account".into(),
        type_id: "identity".into(),
        name: "Public object".into(),
        properties: json!({"value":"keep","__attachments":[
            {"id":"a","objectId":"public-object","fileName":"a.txt","mimeType":"text/plain","sizeBytes":1,"createdAt":"public-time"},
            {"id":"b","objectId":"public-object","fileName":"b.txt","mimeType":"text/plain","sizeBytes":1,"createdAt":"public-time"}
        ]}),
        version: 1,
        ..Default::default()
    }).unwrap();
    (vault, dir)
}

#[test]
fn gui_attachment_transactions_preserve_concurrent_rename_and_soft_delete() {
    let (vault, _dir) = fixture();
    let vault = Arc::new(vault);
    let start = Arc::new(Barrier::new(3));
    let workers: Vec<_> = [0, 1]
        .into_iter()
        .map(|index| {
            let vault = vault.clone();
            let start = start.clone();
            std::thread::spawn(move || {
                start.wait();
                mutate_attachments(&vault, "public-object", |atts| {
                    if index == 0 {
                        atts[0].file_name = "renamed.txt".into();
                    } else {
                        atts[1].deleted_at = Some("deleted-time".into());
                    }
                    Ok(())
                })
                .unwrap();
            })
        })
        .collect();
    start.wait();
    for worker in workers {
        worker.join().unwrap();
    }
    let saved = vault.load_object("public-object").unwrap().unwrap();
    let atts = load_attachments(&saved.properties);
    assert_eq!(atts[0].file_name, "renamed.txt");
    assert_eq!(atts[1].deleted_at.as_deref(), Some("deleted-time"));
    assert_eq!(saved.properties["value"], "keep");
    assert_eq!(saved.version, 3);
}

#[test]
fn gui_attachment_errors_and_lock_refusal_do_not_commit_partial_metadata() {
    let (vault, _dir) = fixture();
    let before = vault.load_object("public-object").unwrap().unwrap();
    assert_eq!(
        mutate_attachments(&vault, "public-object", |atts| {
            atts.clear();
            Err("Attachment not found".into())
        })
        .unwrap_err(),
        "Attachment not found"
    );
    assert_eq!(
        vault
            .load_object("public-object")
            .unwrap()
            .unwrap()
            .properties,
        before.properties
    );
    let locked = vault.set_object_edit_lock("public-object", true).unwrap();
    assert_eq!(
        mutate_attachments(&vault, "public-object", |atts| {
            atts.clear();
            Ok(())
        })
        .unwrap_err(),
        solosoul_vault::storage::OBJECT_EDIT_LOCKED
    );
    let after = vault.load_object("public-object").unwrap().unwrap();
    assert_eq!(after.properties, locked.properties);
    assert_eq!(after.version, locked.version);
}
