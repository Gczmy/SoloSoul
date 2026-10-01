//! RF-024：真实加密包、SQLite 事务、会话与持久恢复管线回归。
use super::*;
use serde_json::{json, Value};
use solosoul_vault::{ObjectRecord, VaultStore};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use zip::{write::SimpleFileOptions, ZipWriter};
const PASSWORD: &str = "RF024-package-password";
const MASTER: &str = "password123";
struct Fixture {
    service: Arc<VaultService>,
    session: VaultSession,
    vault: Arc<VaultStore>,
    account: String,
    db: rusqlite::Connection,
    dir: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::TempDir::new().unwrap();
        let service = Arc::new(VaultService::with_base_path(dir.path().join("vault")));
        let account = service.create_account("RF024", MASTER, None).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let session = service.capture_session(&account).unwrap();
        let vault = service.get_vault_store().unwrap();
        let db = rusqlite::Connection::open(vault.base_path().join("vault.db")).unwrap();
        Self {
            service,
            session,
            vault,
            account,
            db,
            dir,
        }
    }
    fn run(&self, path: &Path, options: ImportOptions, id: &str) -> ImportOutcome {
        execute_encrypted_import(
            &self.service,
            &self.session,
            EncryptedImportRequest {
                source_path: path.to_string_lossy().into_owned(),
                password: Zeroizing::new(PASSWORD.into()),
                options,
                operation: Some((id.into(), ImportSourceKind::Manual)),
            },
            None,
        )
        .unwrap()
    }
    fn options(strategy: AdvancedImportStrategy) -> ImportOptions {
        ImportOptions {
            strategy,
            locale: "en-US".into(),
            ..Default::default()
        }
    }
    fn count(&self) -> usize {
        self.db
            .query_row("SELECT COUNT(*) FROM objects", [], |row| row.get(0))
            .unwrap()
    }
    fn local(&self) {
        self.vault
            .save_object(&ObjectRecord {
                id: "rf024-0".into(),
                account_id: self.account.clone(),
                name: "Local".into(),
                type_id: "note".into(),
                section_type: "identity".into(),
                properties: json!({"local":true,"__attachments":[]}),
                created_at: "2026-01-01T00:00:00Z".into(),
                updated_at: "2026-01-01T00:00:00Z".into(),
                version: 1,
                ..Default::default()
            })
            .unwrap();
        self.vault
            .save_snapshot_at("rf024-0", "local", b"local history", "local diff", 10)
            .unwrap();
    }
}
fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn payload() -> Value {
    let enc =
        |bytes: &[u8]| base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes);
    json!({"objects":(0..2).map(|n|json!({"id":format!("rf024-{n}"),"name":format!("Source {n}"),"type_id":"note","section_type":"identity","contract_type_id":"rf024-contract","parent_id":if n==1 {Some("rf024-0")}else{None},"children_ids":if n==0 {vec!["rf024-1"]}else{vec![]},"properties":{"title":format!("value {n}"),"relation":{"type":"relation","targetId":"rf024-0"},"__attachments":[{"id":format!("a{n}"),"objectId":format!("rf024-{n}"),"fileName":"sample.txt","mimeType":"text/plain","sizeBytes":7,"createdAt":"2026-01-01T00:00:00Z"}]}})).collect::<Vec<_>>(),"snapshots":[{"object_id":"rf024-0","timestamp":20,"triggered_by":"edit","diff_summary":"first","data":enc(b"source-0:first")},{"object_id":"rf024-0","timestamp":30,"triggered_by":"edit","diff_summary":"second","data":enc(b"source-0:second")},{"object_id":"rf024-1","timestamp":40,"data":enc(b"source-1:only")}],"templates":[]})
}
fn package(dir: &Path, payload: Value, attachments: bool, preferences: bool) -> PathBuf {
    let path = dir.join(format!("{}.solosoul", id()));
    let salt = solosoul_crypto::kdf::generate_salt();
    let cfg = solosoul_crypto::kdf::KdfConfig::development();
    let key = solosoul_crypto::kdf::derive_export_key(PASSWORD, &salt, &cfg).unwrap();
    let mut zip = ZipWriter::new(std::fs::File::create(&path).unwrap());
    let options = SimpleFileOptions::default();
    zip.start_file("manifest.json", options).unwrap();
    zip.write_all(json!({"version":"2.0","salt_hex":hex::encode(salt),"has_attachments":attachments,"extra_files":if preferences{vec!["preferences.enc"]}else{vec![]},"kdf":super::super::kdf_to_manifest_value(&cfg)}).to_string().as_bytes()).unwrap();
    zip.start_file("payload.enc", options).unwrap();
    let bytes = serde_json::to_vec(&payload).unwrap();
    solosoul_crypto::cipher::encrypt_chunked_stream(
        &key,
        bytes.len() as u64,
        &mut std::io::Cursor::new(bytes),
        &mut zip,
    )
    .unwrap();
    if attachments {
        let att_key =
            solosoul_crypto::hkdf_ext::derive_hkdf_key(&key, &salt, b"solosoul:attachments:v1")
                .unwrap();
        for n in 0..2 {
            zip.start_file(format!("attachments/rf024-{n}/a{n}.enc"), options)
                .unwrap();
            solosoul_crypto::cipher::encrypt_chunked_stream(
                &att_key,
                7,
                &mut std::io::Cursor::new(b"fixture"),
                &mut zip,
            )
            .unwrap();
        }
    }
    if preferences {
        let prefs_key =
            solosoul_crypto::hkdf_ext::derive_hkdf_key(&key, &salt, b"solosoul:preferences:v1")
                .unwrap();
        let encrypted =
            solosoul_crypto::cipher::encrypt_to_bytes(&prefs_key, b"{\"rf024\":true}", None)
                .unwrap();
        zip.start_file("preferences.enc", options).unwrap();
        zip.write_all(&encrypted).unwrap();
    }
    zip.finish().unwrap();
    path
}
fn assert_complete(outcome: &ImportOutcome, objects: usize, snapshots: usize, attachments: usize) {
    assert_eq!(outcome.status, ImportStatus::Complete);
    assert_eq!(outcome.object_count, objects);
    assert_eq!(outcome.snapshot_count, snapshots);
    assert_eq!(outcome.attachment_count, attachments);
    assert!(outcome.error_code.is_none());
}
#[test]
fn rf024_advanced_three_strategies_keep_history_references_preferences_and_local_data() {
    for strategy in [
        AdvancedImportStrategy::SkipExisting,
        AdvancedImportStrategy::Overwrite,
        AdvancedImportStrategy::KeepBoth,
    ] {
        let f = Fixture::new();
        f.local();
        let path = package(f.dir.path(), payload(), true, true);
        let op = id();
        let outcome = f.run(&path, Fixture::options(strategy), &op);
        assert!(outcome.preferences_imported);
        assert_eq!(
            f.vault.load_profile(&f.account).unwrap().unwrap().data,
            b"{\"rf024\":true}"
        );
        match strategy {
            AdvancedImportStrategy::SkipExisting => {
                assert_complete(&outcome, 1, 1, 1);
                assert_eq!(
                    f.vault.load_object("rf024-0").unwrap().unwrap().name,
                    "Local"
                );
                assert_eq!(f.vault.list_snapshots("rf024-0").unwrap().len(), 1);
            }
            AdvancedImportStrategy::Overwrite => {
                assert_complete(&outcome, 2, 3, 2);
                assert_eq!(
                    f.vault.load_object("rf024-0").unwrap().unwrap().name,
                    "Source 0"
                );
                let history = f.vault.list_snapshots("rf024-0").unwrap();
                assert_eq!(history.len(), 2);
                assert_eq!(
                    f.vault
                        .get_snapshot(history[0]["id"].as_str().unwrap())
                        .unwrap()
                        .unwrap(),
                    b"source-0:second"
                );
            }
            AdvancedImportStrategy::KeepBoth => {
                assert_complete(&outcome, 2, 3, 2);
                assert_eq!(f.count(), 3);
                let records = f.vault.list_object_records(&f.account).unwrap();
                let root = records
                    .iter()
                    .find(|o| o.name == "Source 0 (Imported)")
                    .unwrap();
                let child = records
                    .iter()
                    .find(|o| o.name == "Source 1 (Imported)")
                    .unwrap();
                assert_ne!(root.id, "rf024-0");
                assert_eq!(child.parent_id.as_deref(), Some(root.id.as_str()));
                assert_eq!(root.children_ids, vec![child.id.clone()]);
                assert_eq!(child.properties["relation"]["targetId"], root.id);
                assert_eq!(f.vault.list_snapshots(&root.id).unwrap().len(), 2);
                assert_eq!(
                    f.vault.load_object("rf024-0").unwrap().unwrap().name,
                    "Local"
                );
            }
        }
        let repeated = f.run(&path, Fixture::options(strategy), &op);
        assert_complete(
            &repeated,
            outcome.object_count,
            outcome.snapshot_count,
            outcome.attachment_count,
        );
        assert!(path.exists());
    }
}
#[test]
fn rf024_selections_overrides_and_none_empty_attachment_scopes_remain_distinct() {
    for attachments in [None, Some(vec![]), Some(vec!["a0".into()])] {
        let f = Fixture::new();
        f.local();
        let path = package(f.dir.path(), payload(), true, false);
        let mut options = Fixture::options(AdvancedImportStrategy::SkipExisting);
        options.selections = Some(vec![
            ImportSelection {
                object_id: "rf024-0".into(),
                selected: true,
            },
            ImportSelection {
                object_id: "rf024-1".into(),
                selected: false,
            },
        ]);
        options
            .object_strategies
            .insert("rf024-0".into(), AdvancedImportStrategy::Overwrite);
        options.selected_attachment_ids = attachments.clone();
        let result = f.run(&path, options, &id());
        assert_complete(
            &result,
            1,
            2,
            if attachments.as_ref().is_some_and(Vec::is_empty) {
                0
            } else {
                1
            },
        );
        assert_eq!(f.count(), 1);
        assert_eq!(
            f.vault.load_object("rf024-0").unwrap().unwrap().name,
            "Source 0"
        );
    }
    let f = Fixture::new();
    let path = package(f.dir.path(), payload(), true, false);
    let mut options = Fixture::options(AdvancedImportStrategy::Overwrite);
    options.selections = Some(vec![]);
    assert_complete(&f.run(&path, options, &id()), 0, 0, 0);
    assert_eq!(f.count(), 0);
}
#[test]
fn rf024_database_failure_rolls_back_journal_history_objects_then_sameid_retries() {
    let f = Fixture::new();
    let path = package(f.dir.path(), payload(), true, false);
    let operation = id();
    f.db.execute_batch("CREATE TRIGGER rf024_reject BEFORE INSERT ON objects WHEN NEW.id='rf024-1' BEGIN SELECT RAISE(ABORT,'synthetic secret database detail'); END;").unwrap();
    let options = Fixture::options(AdvancedImportStrategy::Overwrite);
    let outcome = f.run(&path, options.clone(), &operation);
    assert_eq!(outcome.status, ImportStatus::NotCommitted);
    assert_eq!(
        outcome.object_count + outcome.snapshot_count + outcome.attachment_count,
        0
    );
    assert_eq!(f.count(), 0);
    assert!(f
        .vault
        .load_import_operation(&f.account, &operation)
        .unwrap()
        .is_none());
    assert_eq!(outcome.error_code.as_deref(), Some("IMPORT_FAILED"));
    f.db.execute_batch("DROP TRIGGER rf024_reject;").unwrap();
    assert_complete(&f.run(&path, options, &operation), 2, 3, 2);
}
#[test]
fn rf024_published_metadata_failure_reopens_resumes_without_source_and_never_recounts() {
    let f = Fixture::new();
    let path = package(f.dir.path(), payload(), true, false);
    let operation = id();
    f.db.execute_batch("CREATE TRIGGER rf024_meta BEFORE UPDATE OF properties ON objects BEGIN SELECT RAISE(ABORT,'synthetic secret metadata detail'); END;").unwrap();
    let outcome = f.run(
        &path,
        Fixture::options(AdvancedImportStrategy::Overwrite),
        &operation,
    );
    assert_eq!(outcome.status, ImportStatus::Partial);
    assert_eq!(outcome.object_count, 2);
    assert_eq!(outcome.snapshot_count, 3);
    assert_eq!(outcome.attachment_files_written, 2);
    assert_eq!(outcome.attachment_count, 0);
    assert_eq!(outcome.failure_stage, Some(ImportStage::Attachments));
    f.db.execute_batch("DROP TRIGGER rf024_meta;").unwrap();
    std::fs::remove_file(&path).unwrap();
    let base = f.service.base_path().to_path_buf();
    let account = f.account.clone();
    let Fixture {
        service,
        session,
        vault,
        db,
        dir,
        ..
    } = f;
    drop(session);
    drop(service);
    drop(vault);
    drop(db);
    let service = VaultService::with_base_path(base);
    service.unlock(&account, MASTER).unwrap();
    let session = service.capture_session(&account).unwrap();
    for _ in 0..2 {
        assert_complete(
            &resume_encrypted_import(&service, &session, &operation, None, None, None).unwrap(),
            2,
            3,
            2,
        );
    }
    let audits = session.vault().list_audit_log(100).unwrap();
    assert_eq!(
        audits
            .iter()
            .filter(|entry| entry.action_type == "import_execute")
            .count(),
        1
    );
    drop(session);
    drop(service);
    drop(dir);
}
#[test]
fn rf024_session_invalidated_during_plan_never_commits_into_reunlocked_account() {
    let f = Fixture::new();
    let path = package(f.dir.path(), payload(), false, false);
    let svc = f.service.clone();
    let once = Arc::new(AtomicBool::new(false));
    let seen = once.clone();
    let progress: Arc<dyn Fn(u8) + Send + Sync> = Arc::new(move |_| {
        if !seen.swap(true, Ordering::SeqCst) {
            svc.lock();
        }
    });
    let operation = id();
    let result = execute_encrypted_import(
        &f.service,
        &f.session,
        EncryptedImportRequest {
            source_path: path.to_string_lossy().into_owned(),
            password: Zeroizing::new(PASSWORD.into()),
            options: Fixture::options(AdvancedImportStrategy::Overwrite),
            operation: Some((operation.clone(), ImportSourceKind::Manual)),
        },
        Some(progress),
    )
    .unwrap();
    assert!(once.load(Ordering::SeqCst));
    assert_eq!(result.status, ImportStatus::NotCommitted);
    assert_eq!(f.count(), 0);
    f.service.unlock(&f.account, MASTER).unwrap();
    let current = f.service.capture_session(&f.account).unwrap();
    assert!(current
        .vault()
        .load_import_operation(&f.account, &operation)
        .unwrap()
        .is_none());
    assert!(path.exists());
}
#[test]
fn rf024_wrong_password_is_sanitized_and_plaintext_temporaries_are_removed() {
    let f = Fixture::new();
    let path = package(f.dir.path(), payload(), true, true);
    let result = execute_encrypted_import(
        &f.service,
        &f.session,
        EncryptedImportRequest {
            source_path: path.to_string_lossy().into_owned(),
            password: Zeroizing::new("wrong password".into()),
            options: Fixture::options(AdvancedImportStrategy::Overwrite),
            operation: Some((id(), ImportSourceKind::Manual)),
        },
        None,
    )
    .unwrap();
    assert_eq!(result.status, ImportStatus::NotCommitted);
    assert_eq!(result.error_code.as_deref(), Some("DECRYPT_FAILED"));
    assert_eq!(f.count(), 0);
    assert!(!std::fs::read_dir(f.service.base_path())
        .unwrap()
        .flatten()
        .any(|entry| entry
            .file_name()
            .to_string_lossy()
            .starts_with(IMPORT_TMP_PREFIX)));
    assert!(path.exists());
}
#[test]
fn rf024_legacy_cli_duplicate_write_count_and_keep_history_survive_shared_commit() {
    let mut p = payload();
    p["objects"] = json!([p["objects"][0].clone(), p["objects"][0].clone()]);
    p["objects"][1]["name"] = json!("Last duplicate");
    let f = Fixture::new();
    let path = package(f.dir.path(), p.clone(), false, false);
    let key = f.service.attachment_key_for_session(&f.session).unwrap();
    let operation = id();
    let outcome = super::super::import_vault_resumable(
        &f.service,
        &f.session,
        &operation,
        &path,
        PASSWORD,
        super::super::ImportStrategy::SkipExisting,
        f.service.base_path(),
        &key,
    )
    .unwrap();
    assert!(outcome.complete);
    assert_eq!(outcome.object_write_count, 2);
    assert_eq!(f.count(), 1);
    assert_eq!(
        f.vault.load_object("rf024-0").unwrap().unwrap().name,
        "Last duplicate"
    );
    assert!(f.vault.list_snapshots("rf024-0").unwrap().is_empty());
    let again = super::super::resume_vault_import(
        &f.service,
        &f.session,
        &operation,
        None,
        None,
        f.service.base_path(),
        &key,
    )
    .unwrap();
    assert_eq!(again.object_write_count, 2);
    let advanced = Fixture::new();
    let path = package(advanced.dir.path(), p, false, false);
    let outcome = advanced.run(
        &path,
        Fixture::options(AdvancedImportStrategy::SkipExisting),
        &id(),
    );
    assert_complete(&outcome, 1, 2, 0);
    assert_eq!(
        advanced.vault.load_object("rf024-0").unwrap().unwrap().name,
        "Source 0"
    );
}
#[test]
fn rf024_native_cloud_identity_rejects_foreign_paths_and_binds_account_device_hlc() {
    let f = Fixture::new();
    let source = package(f.dir.path(), payload(), false, false);
    assert!(source::cloud_import_source_identity(&f.service, &f.session, &source).is_err());
    let incoming = f
        .service
        .base_path()
        .join("cloud_sync_incoming")
        .join(&f.account)
        .join("remote-device/123-0.solosoul");
    std::fs::create_dir_all(incoming.parent().unwrap()).unwrap();
    std::fs::copy(&source, &incoming).unwrap();
    assert_eq!(
        source::cloud_import_source_identity(&f.service, &f.session, &incoming).unwrap(),
        ("remote-device".into(), "123-0".into())
    );
    let result = execute_encrypted_import(
        &f.service,
        &f.session,
        EncryptedImportRequest {
            source_path: incoming.to_string_lossy().into_owned(),
            password: Zeroizing::new(PASSWORD.into()),
            options: Fixture::options(AdvancedImportStrategy::SkipExisting),
            operation: Some((id(), ImportSourceKind::Cloud)),
        },
        None,
    )
    .unwrap();
    assert_complete(&result, 2, 3, 0);
    let invalid = execute_encrypted_import(
        &f.service,
        &f.session,
        EncryptedImportRequest {
            source_path: source.to_string_lossy().into_owned(),
            password: Zeroizing::new(PASSWORD.into()),
            options: Fixture::options(AdvancedImportStrategy::Overwrite),
            operation: Some(("not-an-operation-id".into(), ImportSourceKind::Manual)),
        },
        None,
    )
    .unwrap_err();
    assert!(matches!(invalid,ImportFailure::Code{ref code,..} if code=="INVALID_OPERATION_ID"));
}
