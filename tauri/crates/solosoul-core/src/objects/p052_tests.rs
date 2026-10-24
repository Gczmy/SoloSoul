//! P052：实际 Core 用例的阶段交错；仅使用独立的公开合成 Vault。
use super::*;
use serde_json::json;
use solosoul_vault::VaultConfig;
use std::sync::{mpsc, Arc, Barrier};

const ACCOUNT: &str = "public-account";
const OBJECT: &str = "public-object";

fn fixture() -> (VaultStore, tempfile::TempDir, ObjectRecord) {
    let dir = tempfile::TempDir::new().unwrap();
    let vault = VaultStore::open(
        VaultConfig::new(ACCOUNT, dir.path().join("vault")).with_data_key([42; 32]),
    )
    .unwrap();
    let record = ObjectRecord {
        id: OBJECT.into(),
        account_id: ACCOUNT.into(),
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
fn stale_editor_draft_keeps_attachment_written_after_its_read_and_snapshot_uses_latest_metadata() {
    let (vault, dir, _) = fixture();
    let vault = Arc::new(vault);
    let source = dir.path().join("public.txt");
    std::fs::write(&source, b"public attachment").unwrap();
    let base = dir.path().join("vault");
    let (read_tx, read_rx) = mpsc::channel();
    let (written_tx, written_rx) = mpsc::channel();
    let worker = {
        let vault = vault.clone();
        std::thread::spawn(move || {
            read_rx.recv().unwrap();
            let attachment =
                add_attachments(&vault, ACCOUNT, OBJECT, &source, &base, None).unwrap();
            written_tx.send(attachment).unwrap();
        })
    };
    let mut draft = vault.load_object(OBJECT).unwrap().unwrap();
    assert!(load_attachments(&draft.properties).is_empty());
    read_tx.send(()).unwrap();
    let attachment = written_rx.recv().unwrap();
    assert_eq!(vault.load_object(OBJECT).unwrap().unwrap().version, 2);
    draft.name = "Edited after attachment".into();
    update_object(&vault, &mut draft).unwrap();
    worker.join().unwrap();
    let saved = vault.load_object(OBJECT).unwrap().unwrap();
    assert_eq!(saved.name, draft.name);
    assert_eq!(saved.version, 3);
    assert_eq!(load_attachments(&saved.properties)[0].id, attachment.id);
    assert_eq!(draft.properties, saved.properties);
    assert_eq!(
        std::fs::read(attachment.vault_path.unwrap()).unwrap(),
        b"public attachment"
    );
    let snapshots = vault.list_snapshots(OBJECT).unwrap();
    let bytes = vault
        .get_snapshot(snapshots[0]["id"].as_str().unwrap())
        .unwrap()
        .unwrap();
    let snapshot: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        snapshot["properties"]["__attachments"][0]["id"],
        attachment.id
    );
}

#[test]
fn stale_form_does_not_undo_new_structure_and_lock_failure_preserves_the_draft() {
    let (vault, _dir, mut draft) = fixture();
    vault
        .mutate_object(OBJECT, |record| -> Result<_, String> {
            record.children_ids.push("new-child".into());
            record.template_hash = Some("new-template-hash".into());
            Ok(true)
        })
        .unwrap();
    draft.name = "New name".into();
    update_object(&vault, &mut draft).unwrap();
    assert_eq!(draft.children_ids, ["new-child"]);
    assert_eq!(draft.template_hash.as_deref(), Some("new-template-hash"));
    vault.set_object_edit_lock(OBJECT, true).unwrap();
    draft.name = "Unsaved draft".into();
    let before = serde_json::to_value(&draft).unwrap();
    assert_eq!(
        update_object(&vault, &mut draft).unwrap_err(),
        solosoul_vault::storage::OBJECT_EDIT_LOCKED
    );
    assert_eq!(serde_json::to_value(&draft).unwrap(), before);
    assert_eq!(vault.load_object(OBJECT).unwrap().unwrap().name, "New name");
}

#[test]
fn concurrent_core_rename_and_delete_keep_both_attachment_changes() {
    let (vault, _dir, mut record) = fixture();
    record.properties["__attachments"] = json!([
        {"id":"a","objectId":OBJECT,"fileName":"a.txt","mimeType":"text/plain","sizeBytes":1,"createdAt":"public-time"},
        {"id":"b","objectId":OBJECT,"fileName":"b.txt","mimeType":"text/plain","sizeBytes":1,"createdAt":"public-time"}
    ]);
    vault.save_object(&record).unwrap();
    let vault = Arc::new(vault);
    let start = Arc::new(Barrier::new(3));
    let rename = {
        let vault = vault.clone();
        let start = start.clone();
        std::thread::spawn(move || {
            start.wait();
            rename_attachment(&vault, ACCOUNT, OBJECT, "a", "renamed.txt").unwrap();
        })
    };
    let delete = {
        let vault = vault.clone();
        let start = start.clone();
        std::thread::spawn(move || {
            start.wait();
            soft_delete_attachment(&vault, ACCOUNT, OBJECT, "b").unwrap();
        })
    };
    start.wait();
    rename.join().unwrap();
    delete.join().unwrap();
    let saved = vault.load_object(OBJECT).unwrap().unwrap();
    let attachments = load_attachments(&saved.properties);
    assert_eq!(
        attachments
            .iter()
            .find(|att| att.id == "a")
            .unwrap()
            .file_name,
        "renamed.txt"
    );
    assert!(attachments
        .iter()
        .find(|att| att.id == "b")
        .unwrap()
        .deleted_at
        .is_some());
    assert_eq!(saved.version, 3);
}

#[test]
fn metadata_failure_does_not_publish_a_reference_or_remove_files() {
    let (vault, dir, original) = fixture();
    let base = dir.path().join("vault");
    let source = dir.path().join("public.txt");
    std::fs::write(&source, b"keep public source").unwrap();
    let database = rusqlite::Connection::open(base.join("vault.db")).unwrap();
    database.execute_batch("CREATE TRIGGER public_save_failure BEFORE UPDATE ON objects BEGIN SELECT RAISE(ABORT,'public failure'); END;").unwrap();
    assert!(add_attachments(&vault, ACCOUNT, OBJECT, &source, &base, None).is_err());
    assert_eq!(std::fs::read(&source).unwrap(), b"keep public source");
    let copied_files: usize = std::fs::read_dir(base.join("attachments").join(OBJECT))
        .unwrap()
        .map(|entry| std::fs::read_dir(entry.unwrap().path()).unwrap().count())
        .sum();
    // 文件复制与数据库仍是独立步骤；沿用失败副本保留语义，避免在提交结果异常时误删实体。
    assert_eq!(copied_files, 1);
    let saved = vault.load_object(OBJECT).unwrap().unwrap();
    assert_eq!(saved.version, original.version);
    assert!(load_attachments(&saved.properties).is_empty());
}

#[test]
fn a_foreign_account_draft_cannot_update_the_current_object() {
    let (vault, _dir, mut draft) = fixture();
    draft.account_id = "other-account".into();
    draft.name = "Foreign draft".into();
    assert!(update_object(&vault, &mut draft).is_err());
    let saved = vault.load_object(OBJECT).unwrap().unwrap();
    assert_eq!(saved.account_id, ACCOUNT);
    assert_eq!(saved.name, "Public object");
    assert_eq!(saved.version, 1);
}
