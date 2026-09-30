use super::*;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

pub(crate) struct Fixture {
    pub service: Arc<RwLock<solosoul_core::VaultService>>,
    pub vault: Arc<VaultStore>,
    pub account: String,
    pub db: rusqlite::Connection,
    // Windows 上先关闭所有数据库句柄，再清理目录。
    pub dir: TempDir,
}

impl Fixture {
    pub(crate) fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let service = Arc::new(RwLock::new(solosoul_core::VaultService::with_base_path(
            dir.path().join("vault"),
        )));
        let account = format!("acc_{}", Uuid::new_v4().simple());
        service
            .read()
            .unwrap()
            .create_account_with_id(&account, "Test", "password123", None)
            .unwrap();
        let vault = service.read().unwrap().get_vault_store().unwrap();
        let db = rusqlite::Connection::open(vault.base_path().join("vault.db")).unwrap();
        Self {
            dir,
            service,
            vault,
            account,
            db,
        }
    }

    pub(crate) fn reject_nth_object_write(&self, nth: usize) {
        self.db
            .execute_batch(&format!(
                "CREATE TABLE rf020_writes(n INTEGER); INSERT INTO rf020_writes VALUES(0);
             CREATE TRIGGER rf020_reject BEFORE INSERT ON objects
             WHEN NEW.id LIKE 'rf020-%' BEGIN
             UPDATE rf020_writes SET n=n+1;
             SELECT CASE WHEN (SELECT n FROM rf020_writes)={nth}
             THEN RAISE(ABORT, 'sensitive injected database detail') END; END;"
            ))
            .unwrap();
    }

    fn run(&self, path: &Path) -> ImportResult {
        import_execute_internal(
            self.service.read().unwrap(),
            self.account.clone(),
            path.to_string_lossy().into_owned(),
            Zeroizing::new("export-password".into()),
            ImportStrategy::Overwrite,
            None,
            None,
            HashMap::new(),
            "en-US",
            None,
        )
        .unwrap()
    }

    fn object_count(&self) -> usize {
        self.db
            .query_row(
                "SELECT COUNT(*) FROM objects WHERE id LIKE 'rf020-%'",
                [],
                |r| r.get(0),
            )
            .unwrap()
    }

    fn linked_attachments(&self) -> usize {
        (0..2)
            .filter_map(|n| self.vault.load_object(&format!("rf020-{n}")).unwrap())
            .flat_map(|o| {
                o.properties["__attachments"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
            })
            .filter(|a| {
                a["vaultPath"]
                    .as_str()
                    .is_some_and(|p| Path::new(p).is_file())
            })
            .count()
    }
}

pub(crate) fn objects() -> serde_json::Value {
    json!({"objects": (0..2).map(|n| json!({
        "id": format!("rf020-{n}"), "name": "Synthetic", "type_id": "note",
        "section_type": "identity", "properties": {"__attachments": [{
            "id": format!("a{n}"), "objectId": format!("rf020-{n}"),
            "fileName": "sample.txt", "mimeType": "text/plain", "sizeBytes": 7,
            "createdAt": "2026-09-25T00:00:00Z"
        }]}
    })).collect::<Vec<_>>()})
}

pub(crate) fn package(
    dir: &Path,
    payload: serde_json::Value,
    attachments: bool,
    corrupt_second: bool,
    preferences: bool,
) -> PathBuf {
    let path = dir.join("incoming.solosoul");
    let salt = solosoul_crypto::kdf::generate_salt();
    let key = derive_export_key_cfg(
        "export-password",
        &salt,
        &solosoul_crypto::kdf::KdfConfig::balanced(),
    )
    .unwrap();
    let mut zip = ZipWriter::new(File::create(&path).unwrap());
    let options = SimpleFileOptions::default();
    zip.start_file("manifest.json", options).unwrap();
    zip.write_all(
        json!({"version":"2.0", "salt_hex": hex::encode(salt), "has_attachments":attachments,
            "extra_files": if preferences { vec!["preferences.enc"] } else { vec![] }
        })
        .to_string()
        .as_bytes(),
    )
    .unwrap();
    zip.start_file("payload.enc", options).unwrap();
    let payload = serde_json::to_vec(&payload).unwrap();
    solosoul_crypto::cipher::encrypt_chunked_stream(
        &key,
        payload.len() as u64,
        &mut std::io::Cursor::new(payload),
        &mut zip,
    )
    .unwrap();
    if attachments {
        let att_key =
            solosoul_crypto::hkdf_ext::derive_hkdf_key(&key, &salt, b"solosoul:attachments:v1")
                .unwrap();
        for n in 0..2 {
            zip.start_file(format!("attachments/rf020-{n}/a{n}.enc"), options)
                .unwrap();
            if n == 1 && corrupt_second {
                zip.write_all(b"invalid encrypted attachment").unwrap();
            } else {
                solosoul_crypto::cipher::encrypt_chunked_stream(
                    &att_key,
                    7,
                    &mut std::io::Cursor::new(b"fixture"),
                    &mut zip,
                )
                .unwrap();
            }
        }
    }
    // preferences=true 时声明但缺少条目，覆盖此前被 if let Ok 吞掉的读取失败。
    zip.finish().unwrap();
    path
}

#[test]
fn rf020_object_failure_counts_match_database() {
    for nth in [1, 2] {
        let f = Fixture::new();
        f.reject_nth_object_write(nth);
        let path = package(f.dir.path(), objects(), false, false, false);
        let outcome = f.run(&path);
        assert_eq!(outcome.object_count, 0);
        assert_eq!(outcome.template_count, 0);
        assert_eq!(outcome.snapshot_count, 0);
        assert_eq!(outcome.object_count, f.object_count());
        assert_eq!(outcome.status, ImportStatus::NotCommitted);
        assert_eq!(outcome.failure_stage, Some(ImportStage::Objects));
        assert!(!serde_json::to_string(&outcome)
            .unwrap()
            .contains("sensitive injected"));
        assert!(outcome.require_complete().is_err());
        assert!(path.exists());
    }
}

#[test]
fn rf020_attachment_failures_count_only_linked_metadata() {
    for corrupt_file in [true, false] {
        let f = Fixture::new();
        // 先提交两个对象，第 4 次写对象对应第二个附件元数据写回。
        if !corrupt_file {
            f.reject_nth_object_write(4);
        }
        let path = package(f.dir.path(), objects(), true, corrupt_file, false);
        let outcome = f.run(&path);
        assert_eq!(outcome.status, ImportStatus::Partial);
        assert_eq!(outcome.failure_stage, Some(ImportStage::Attachments));
        assert_eq!(outcome.object_count, f.object_count());
        assert_eq!(outcome.attachment_count, f.linked_attachments());
        assert_eq!(outcome.attachment_count, if corrupt_file { 0 } else { 1 });
        assert_eq!(
            outcome.attachment_files_written,
            if corrupt_file { 1 } else { 2 }
        );
        assert!(path.exists());
    }
}

#[test]
fn rf020_database_snapshot_failure_is_atomic_and_preferences_remain_partial() {
    for snapshots in [true, false] {
        let f = Fixture::new();
        if snapshots {
            f.db.execute_batch(
                "CREATE TRIGGER rf020_reject_snapshot BEFORE INSERT ON object_snapshots
                BEGIN SELECT RAISE(ABORT, 'snapshot fault'); END;",
            )
            .unwrap();
        }
        let path = package(f.dir.path(), objects(), false, false, !snapshots);
        let outcome = f.run(&path);
        assert_eq!(
            outcome.status,
            if snapshots {
                ImportStatus::NotCommitted
            } else {
                ImportStatus::Partial
            }
        );
        assert_eq!(
            outcome.failure_stage,
            Some(if snapshots {
                ImportStage::Snapshots
            } else {
                ImportStage::Preferences
            })
        );
        assert_eq!(outcome.object_count, f.object_count());
        assert_eq!(outcome.object_count, if snapshots { 0 } else { 2 });
        assert_eq!(outcome.snapshot_count, if snapshots { 0 } else { 2 });
        assert!(!outcome.preferences_imported);
    }
}

#[test]
fn rf020_success_keeps_counts_and_reports_complete() {
    let f = Fixture::new();
    let path = package(f.dir.path(), objects(), true, false, false);
    let outcome = f.run(&path).require_complete().unwrap();
    assert_eq!(outcome.object_count, 2);
    assert_eq!(outcome.attachment_count, 2);
    assert_eq!(outcome.attachment_count, f.linked_attachments());
    assert_eq!(outcome.snapshot_count, 2);
    assert_eq!(outcome.failure_stage, None);
    assert_eq!(outcome.error_code, None);
}

#[test]
fn rf020_duplicate_object_ids_count_committed_records_once() {
    let f = Fixture::new();
    let mut payload = objects();
    payload["objects"][1] = payload["objects"][0].clone();
    let path = package(f.dir.path(), payload, false, false, false);
    let outcome = f.run(&path).require_complete().unwrap();
    assert_eq!(outcome.object_count, 1);
    assert_eq!(outcome.object_count, f.object_count());
}

#[test]
fn rf020_cloud_request_accepts_all_or_explicit_selection() {
    for selections in [serde_json::Value::Null, json!([])] {
        let req: AdvancedImportRequest = serde_json::from_value(json!({
            "selections": selections, "strategy":"skipExisting", "sourcePath":"fixture.solosoul",
            "password":"export-password", "selectedAttachmentIds":null, "objectStrategies":{}
        }))
        .unwrap();
        assert_eq!(req.selections.is_none(), selections.is_null());
    }
}

#[test]
fn rf020_template_failure_is_atomic_and_decryption_is_uncommitted() {
    let f = Fixture::new();
    let mut payload = objects();
    let template = |id: &str| {
        json!({
        "id": id, "accountId":"source", "name": id, "properties":[],
        "createdAt":"2026-09-25T00:00:00Z"
        })
    };
    payload["templates"] = json!([template("first"), template("second")]);
    f.db.execute_batch(
        "CREATE TRIGGER rf020_template BEFORE INSERT ON user_templates
        WHEN NEW.id='second' BEGIN SELECT RAISE(ABORT, 'template fault'); END;",
    )
    .unwrap();
    let path = package(f.dir.path(), payload, false, false, false);
    let outcome = f.run(&path);
    assert_eq!(outcome.status, ImportStatus::NotCommitted);
    assert_eq!(outcome.template_count, 0);
    assert_eq!(outcome.object_count, 0);
    assert_eq!(outcome.failure_stage, Some(ImportStage::Templates));
    assert!(f.vault.load_user_template("first").unwrap().is_none());
    assert!(f.vault.load_user_template("second").unwrap().is_none());
    std::fs::write(&path, b"not a package").unwrap();
    let outcome = f.run(&path);
    assert_eq!(outcome.status, ImportStatus::NotCommitted);
    assert_eq!(outcome.failure_stage, Some(ImportStage::Preparation));
}
