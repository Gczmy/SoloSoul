//! RF-022 真实加密 ZIP / SQLite / 新进程检查点回归；Root 执行，候选未验证。
use super::*;
use rusqlite::Connection;
use serde_json::{json, Value};
use std::process::Command;
use std::sync::Arc;

const PASSWORD: &str = "rf022-vault-password";
const PACKAGE_PASSWORD: &str = "RF022-Export-Password1";
const NOW: &str = "2026-09-30T22:00:00Z";
const OWNER: &str = "rf022_owner";
const SECOND_OWNER: &str = "rf022_second_owner";

struct Fixture {
    dir: tempfile::TempDir,
    service: crate::VaultService,
    account: String,
    vault: Arc<VaultStore>,
    db: Connection,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::TempDir::new().unwrap();
        let service = crate::VaultService::with_base_path(dir.path().into());
        let account = service.create_account("RF022", PASSWORD, None).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let vault = service.get_vault_store().unwrap();
        let db = Connection::open(vault.base_path().join("vault.db")).unwrap();
        Self {
            dir,
            service,
            account,
            vault,
            db,
        }
    }
    fn session(&self) -> crate::VaultSession {
        self.service.capture_session(&self.account).unwrap()
    }
    fn key(&self) -> Zeroizing<[u8; 32]> {
        self.service
            .attachment_key_for_session(&self.session())
            .unwrap()
    }
    fn payload(&self, second_owner: bool) -> Value {
        let mut objects = vec![source_object(OWNER, &["att_a", "att_b"])];
        if second_owner {
            objects.push(source_object(SECOND_OWNER, &["att_c"]));
        }
        json!({"objects":objects})
    }
    fn package(&self, payload: &Value, prefs: Option<&[u8]>) -> PathBuf {
        write_package(self.dir.path(), payload, prefs)
    }
    fn owned(&self, path: &Path) -> OwnedImportPackage {
        OwnedImportPackage::capture(path, self.dir.path()).unwrap()
    }
    fn prepare(
        &self,
        owned: &OwnedImportPackage,
        operation_id: &str,
        kind: ImportSourceKind,
        selection: Option<&HashSet<String>>,
        map: HashMap<String, String>,
    ) -> (ImportReadView, ImportDatabaseBatch, ImportOperationStart) {
        let opened = owned.decrypt(PACKAGE_PASSWORD, self.dir.path()).unwrap();
        let mut prepared = super::super::prepare_import_database(
            &self.vault,
            &self.account,
            &opened.payload,
            ImportStrategy::Overwrite,
            NOW,
        )
        .unwrap();
        for write in &mut prepared.batch.objects {
            if let Some(id) = map.get(&write.record.id) {
                write.record.id = id.clone();
            }
        }
        let start = prepare_import_operation(
            operation_id,
            kind,
            json!({"strategy":"overwrite","selection":selection.map(|s| {
            let mut ids: Vec<_> = s.iter().cloned().collect(); ids.sort(); ids
        }),"locale":"en-US"}),
            owned,
            &opened.payload,
            &prepared.imported_object_ids,
            &map,
            selection,
            NOW,
            self.dir.path(),
            &self.vault,
            &prepared.view,
            &mut prepared.batch,
            opened.has_attachments,
            opened.has_preferences,
        )
        .unwrap();
        (prepared.view, prepared.batch, start)
    }
    fn commit(
        &self,
        view: &ImportReadView,
        batch: &ImportDatabaseBatch,
        start: &ImportOperationStart,
    ) -> ImportOperationRecord {
        self.service
            .with_session(&self.session(), |vault| {
                vault.commit_import_batch_with_operation(
                    &self.account,
                    &view.revision,
                    batch,
                    start,
                )
            })
            .unwrap()
            .operation
    }
    fn run(
        &self,
        id: &str,
        owned: Option<&OwnedImportPackage>,
        password: Option<&str>,
        counts: &mut AttachmentImportProgress,
    ) -> Result<ImportOperationRecord, ExportError> {
        resume_import_operation(
            &self.service,
            &self.session(),
            id,
            self.dir.path(),
            owned,
            password,
            &self.key(),
            None,
            counts,
        )
    }
    fn reopen(&self) -> crate::VaultService {
        self.service.lock();
        let fresh = crate::VaultService::with_base_path(self.dir.path().into());
        fresh
            .unlock_secure(&self.account, &Zeroizing::new(PASSWORD.to_owned()))
            .unwrap();
        fresh
    }
}

fn source_object(id: &str, attachments: &[&str]) -> Value {
    json!({"id":id,"name":"Imported record","type_id":"note","section_type":"identity", "created_at":NOW,
        "properties":{"body":"source body", "__attachments":attachments.iter().map(|id| json!({
            "id":id,"objectId":"ignored source metadata owner", "fileName":format!("{id}.txt"),
            "mimeType":"text/plain","sizeBytes":999999,"createdAt":NOW,
            "srcPath":"C:/source-only/secret.txt","vaultPath":"C:/source-only/secret.txt",
            "description":"old description","tags":["old"]})).collect::<Vec<_>>()}})
}

fn write_package(root: &Path, payload: &Value, prefs: Option<&[u8]>) -> PathBuf {
    let path = root.join(format!("{}.solosoul", uuid::Uuid::new_v4()));
    let salt = solosoul_crypto::kdf::generate_salt();
    let config = KdfConfig::development();
    let key = derive_export_key_cfg(PACKAGE_PASSWORD, &salt, &config).unwrap();
    let attachment_key =
        solosoul_crypto::hkdf_ext::derive_hkdf_key(&key, &salt, b"solosoul:attachments:v1")
            .unwrap();
    let manifest = json!({"version":"2.0", "salt_hex":hex::encode(salt),"has_attachments":true,
        "extra_files":if prefs.is_some(){vec!["preferences.enc"]}else{vec![]},"kdf":kdf_to_manifest_value(&config)});
    let mut zip = ZipWriter::new(File::create(&path).unwrap());
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("manifest.json", options).unwrap();
    zip.write_all(manifest.to_string().as_bytes()).unwrap();
    zip.start_file("payload.enc", options).unwrap();
    let bytes = serde_json::to_vec(payload).unwrap();
    solosoul_crypto::cipher::encrypt_chunked_stream(
        &key,
        bytes.len() as u64,
        &mut std::io::Cursor::new(bytes),
        &mut zip,
    )
    .unwrap();
    let mut emitted = HashSet::new();
    for object in payload["objects"].as_array().unwrap() {
        let owner = object["id"].as_str().unwrap();
        for att in object["properties"]["__attachments"].as_array().unwrap() {
            let id = att["id"].as_str().unwrap();
            if !emitted.insert((owner.to_owned(), id.to_owned())) {
                continue;
            }
            zip.start_file(format!("attachments/{owner}/{id}.enc"), options)
                .unwrap();
            let bytes = format!("content of {id}").into_bytes();
            solosoul_crypto::cipher::encrypt_chunked_stream(
                &attachment_key,
                bytes.len() as u64,
                &mut std::io::Cursor::new(bytes),
                &mut zip,
            )
            .unwrap();
        }
    }
    if let Some(prefs) = prefs {
        let prefs_key =
            solosoul_crypto::hkdf_ext::derive_hkdf_key(&key, &salt, b"solosoul:preferences:v1")
                .unwrap();
        let encrypted = solosoul_crypto::cipher::encrypt_to_bytes(&prefs_key, prefs, None).unwrap();
        zip.start_file("preferences.enc", options).unwrap();
        zip.write_all(&encrypted).unwrap();
    }
    zip.finish().unwrap();
    path
}

fn op_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn final_file(root: &Path, step: &ImportAttachmentStepPlan) -> PathBuf {
    root.join("attachments")
        .join(&step.owner_id)
        .join(&step.attachment_id)
        .join(&step.safe_file_name)
}

fn resumed(
    service: &crate::VaultService,
    account: &str,
    root: &Path,
    id: &str,
    owned: Option<&OwnedImportPackage>,
    counts: &mut AttachmentImportProgress,
) -> Result<ImportOperationRecord, ExportError> {
    let session = service.capture_session(account).unwrap();
    let key = service.attachment_key_for_session(&session).unwrap();
    resume_import_operation(
        service,
        &session,
        id,
        root,
        owned,
        owned.map(|_| PACKAGE_PASSWORD),
        &key,
        None,
        counts,
    )
}

fn assert_attachments(service: &crate::VaultService, owner: &str, expected: usize) {
    let vault = service.get_vault_store().unwrap();
    let object = vault.load_object(owner).unwrap().unwrap();
    let atts = load_attachments(&object.properties);
    assert_eq!(atts.len(), expected);
    let key = service
        .attachment_key_for_session(&service.capture_session(&object.account_id).unwrap())
        .unwrap();
    for attachment in atts {
        assert!(!attachment.file_name.contains('/'));
        assert!(!attachment.file_name.contains('\\'));
        assert_eq!(attachment.description, None);
        assert!(attachment.tags.is_empty());
        let path = Path::new(attachment.vault_path.as_deref().unwrap());
        assert_eq!(attachment.src_path, attachment.vault_path);
        assert!(crate::attachment_crypto::is_encrypted_file(path));
        let bytes = crate::attachment_crypto::read_file_decrypted(&key, path, 1000).unwrap();
        assert_eq!(attachment.size_bytes, bytes.len() as u64);
        assert!(std::str::from_utf8(&bytes)
            .unwrap()
            .starts_with("content of "));
    }
}

#[test]
fn rf022_owned_proof_and_parsing_use_same_ciphertext_after_original_path_replaced() {
    let fixture = Fixture::new();
    let payload = fixture.payload(false);
    let source = fixture.package(&payload, None);
    let owned = fixture.owned(&source);
    let original = owned.source_proof().clone();
    std::fs::write(&source, b"different original file after Native capture").unwrap();
    assert_eq!(owned.source_proof(), &original);
    assert_eq!(
        owned
            .decrypt(PACKAGE_PASSWORD, fixture.dir.path())
            .unwrap()
            .payload,
        payload
    );
    let changed = fixture.owned(&source);
    assert_ne!(changed.source_proof(), &original);
}

#[test]
fn rf022_plan_freezes_zip_order_safe_names_selected_ids_and_keepboth_target_map() {
    let fixture = Fixture::new();
    let payload = fixture.payload(true);
    let source = fixture.package(&payload, None);
    let owned = fixture.owned(&source);
    let ids = HashSet::from(["att_b".to_owned(), "att_c".to_owned()]);
    let new_id = uuid::Uuid::new_v4().to_string();
    let map = HashMap::from([(OWNER.to_owned(), new_id.clone())]);
    let (_, batch, start) =
        fixture.prepare(&owned, &op_id(), ImportSourceKind::Manual, Some(&ids), map);
    assert_eq!(start.steps.len(), 2);
    assert!(start.steps[0].entry_ordinal < start.steps[1].entry_ordinal);
    assert_eq!(start.steps[0].source_attachment_id, "att_b");
    assert_eq!(start.steps[0].owner_id, new_id);
    assert_eq!(start.steps[1].source_attachment_id, "att_c");
    for step in &start.steps {
        assert!(uuid::Uuid::parse_str(&step.attachment_id).is_ok());
        assert!(!step.safe_file_name.contains('/'));
    }
    for write in batch.objects {
        assert!(write.record.properties.get("__attachments").is_none());
    }
    assert!(!serde_json::to_string(&start)
        .unwrap()
        .contains(PACKAGE_PASSWORD));
}

#[test]
fn rf022_payload_duplicate_attachment_metadata_uses_last_value_and_unsafe_name_rejects_before_commit(
) {
    let fixture = Fixture::new();
    let mut payload = fixture.payload(false);
    let mut duplicate = payload["objects"][0]["properties"]["__attachments"][0].clone();
    duplicate["fileName"] = json!("last-source-name.txt");
    payload["objects"][0]["properties"]["__attachments"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    let source = fixture.package(&payload, None);
    let owned = fixture.owned(&source);
    let id = op_id();
    let (_, _, start) =
        fixture.prepare(&owned, &id, ImportSourceKind::Manual, None, HashMap::new());
    assert_eq!(start.steps.len(), 2);
    assert_eq!(start.steps[0].safe_file_name, "last-source-name.txt");
    let mut opened = owned.decrypt(PACKAGE_PASSWORD, fixture.dir.path()).unwrap();
    opened.payload["objects"][0]["properties"]["__attachments"][2]["fileName"] =
        json!("../escape.txt");
    let mut prepared = super::super::prepare_import_database(
        &fixture.vault,
        &fixture.account,
        &opened.payload,
        ImportStrategy::Overwrite,
        NOW,
    )
    .unwrap();
    let failure = prepare_import_operation(
        &id,
        ImportSourceKind::Manual,
        json!({}),
        &owned,
        &opened.payload,
        &prepared.imported_object_ids,
        &HashMap::new(),
        None,
        NOW,
        fixture.dir.path(),
        &fixture.vault,
        &prepared.view,
        &mut prepared.batch,
        true,
        false,
    )
    .unwrap_err();
    assert!(failure.to_string().contains("附件文件名无效"));
    assert!(fixture.vault.load_object(OWNER).unwrap().is_none());
    assert!(fixture
        .vault
        .load_import_operation(&fixture.account, &id)
        .unwrap()
        .is_none());
}

#[test]
fn rf022_database_failure_rolls_back_records_and_operation_together() {
    let fixture = Fixture::new();
    let source = fixture.package(&fixture.payload(false), None);
    let owned = fixture.owned(&source);
    let id = op_id();
    fixture.db.execute_batch("CREATE TRIGGER rf022_abort BEFORE INSERT ON objects BEGIN SELECT RAISE(ABORT,'rf022 injected object write'); END;").unwrap();
    let (view, batch, start) =
        fixture.prepare(&owned, &id, ImportSourceKind::Cli, None, HashMap::new());
    let error = fixture
        .vault
        .commit_import_batch_with_operation(&fixture.account, &view.revision, &batch, &start)
        .unwrap_err();
    assert_eq!(error, "import_batch_objects_failed");
    assert!(fixture.vault.load_object(OWNER).unwrap().is_none());
    assert!(fixture
        .vault
        .load_import_operation(&fixture.account, &id)
        .unwrap()
        .is_none());
}

#[test]
fn rf022_same_operation_complete_is_idempotent_and_new_fresh_same_package_has_distinct_ids() {
    let fixture = Fixture::new();
    let source = fixture.package(&fixture.payload(false), None);
    let owned = fixture.owned(&source);
    let id = op_id();
    let (view, batch, start) =
        fixture.prepare(&owned, &id, ImportSourceKind::Cli, None, HashMap::new());
    fixture.commit(&view, &batch, &start);
    let mut counts = AttachmentImportProgress::default();
    let first = fixture
        .run(&id, Some(&owned), Some(PACKAGE_PASSWORD), &mut counts)
        .unwrap();
    assert_eq!(counts.written_file_count, 2);
    assert_eq!(counts.committed_count, 2);
    assert_eq!(first.phase, ImportOperationPhase::Complete);
    let second = fixture.run(&id, None, None, &mut counts).unwrap();
    assert_eq!(first.attachment_count, second.attachment_count);
    assert_eq!(counts.written_file_count, 2);
    assert_eq!(counts.committed_count, 2);
    assert_attachments(&fixture.service, OWNER, 2);
    let (view2, batch2, start2) =
        fixture.prepare(&owned, &id, ImportSourceKind::Cli, None, HashMap::new());
    assert_ne!(start2.steps[0].attachment_id, start.steps[0].attachment_id);
    let commit = fixture
        .vault
        .commit_import_batch_with_operation(&fixture.account, &view2.revision, &batch2, &start2)
        .unwrap();
    assert!(commit.already_committed);
    assert_eq!(commit.operation.start.steps, start.steps);
    let new_owner = uuid::Uuid::new_v4().to_string();
    let new_id = op_id();
    let (view3, batch3, start3) = fixture.prepare(
        &owned,
        &new_id,
        ImportSourceKind::Manual,
        None,
        HashMap::from([(OWNER.to_owned(), new_owner.clone())]),
    );
    fixture.commit(&view3, &batch3, &start3);
    fixture
        .run(&new_id, Some(&owned), Some(PACKAGE_PASSWORD), &mut counts)
        .unwrap();
    assert_ne!(start3.steps[0].attachment_id, start.steps[0].attachment_id);
    assert_attachments(&fixture.service, OWNER, 2);
    assert_attachments(&fixture.service, &new_owner, 2);
}

#[test]
fn rf022_metadata_sql_failure_then_reopen_without_source_restores_published_set_once() {
    let fixture = Fixture::new();
    let source = fixture.package(&fixture.payload(false), None);
    let owned = fixture.owned(&source);
    let id = op_id();
    let (view, batch, start) =
        fixture.prepare(&owned, &id, ImportSourceKind::Manual, None, HashMap::new());
    fixture.commit(&view, &batch, &start);
    fixture.db.execute_batch("CREATE TRIGGER rf022_block_metadata BEFORE UPDATE OF properties ON objects BEGIN SELECT RAISE(ABORT,'rf022 metadata failure'); END;").unwrap();
    let mut counts = AttachmentImportProgress::default();
    assert!(fixture
        .run(&id, Some(&owned), Some(PACKAGE_PASSWORD), &mut counts)
        .is_err());
    assert_eq!(counts.written_file_count, 2);
    assert_eq!(counts.committed_count, 0);
    let pending = fixture
        .vault
        .load_import_operation(&fixture.account, &id)
        .unwrap()
        .unwrap();
    assert!(pending
        .steps
        .iter()
        .all(|step| step.phase == ImportAttachmentPhase::Published));
    assert!(fixture
        .vault
        .load_object(OWNER)
        .unwrap()
        .unwrap()
        .properties
        .get("__attachments")
        .is_none());
    fixture
        .db
        .execute_batch("DROP TRIGGER rf022_block_metadata;")
        .unwrap();
    drop(owned);
    std::fs::remove_file(&source).unwrap();
    let reopened = fixture.reopen();
    let result = resumed(
        &reopened,
        &fixture.account,
        fixture.dir.path(),
        &id,
        None,
        &mut counts,
    )
    .unwrap();
    assert_eq!(result.phase, ImportOperationPhase::Complete);
    assert_eq!(counts.written_file_count, 2);
    assert_eq!(counts.committed_count, 2);
    assert_attachments(&reopened, OWNER, 2);
    assert_eq!(
        resumed(
            &reopened,
            &fixture.account,
            fixture.dir.path(),
            &id,
            None,
            &mut counts
        )
        .unwrap()
        .attachment_count,
        2
    );
}

#[test]
fn rf022_file_publication_survives_phase_sql_failure_and_counts_real_file_before_metadata() {
    let fixture = Fixture::new();
    let source = fixture.package(&fixture.payload(false), None);
    let owned = fixture.owned(&source);
    let id = op_id();
    let selected = HashSet::from(["att_a".to_owned()]);
    let (view, batch, start) = fixture.prepare(
        &owned,
        &id,
        ImportSourceKind::Manual,
        Some(&selected),
        HashMap::new(),
    );
    fixture.commit(&view, &batch, &start);
    fixture.db.execute_batch("CREATE TRIGGER rf022_block_published BEFORE UPDATE OF phase ON import_attachment_steps WHEN NEW.phase='published' BEGIN SELECT RAISE(ABORT,'rf022 publish phase failure'); END;").unwrap();
    let mut counts = AttachmentImportProgress::default();
    assert!(fixture
        .run(&id, Some(&owned), Some(PACKAGE_PASSWORD), &mut counts)
        .is_err());
    assert_eq!(counts.written_file_count, 1);
    assert_eq!(counts.committed_count, 0);
    let operation = fixture
        .vault
        .load_import_operation(&fixture.account, &id)
        .unwrap()
        .unwrap();
    assert_eq!(operation.steps[0].phase, ImportAttachmentPhase::Staged);
    let final_path = final_file(fixture.dir.path(), &start.steps[0]);
    assert!(final_path.exists());
    assert!(!attempt_directory(
        fixture.dir.path(),
        &id,
        operation.steps[0]
            .staged_proof
            .as_ref()
            .unwrap()
            .stage_epoch,
        u64::from(start.steps[0].entry_ordinal)
    )
    .unwrap()
    .join("attachment")
    .exists());
    fixture
        .db
        .execute_batch("DROP TRIGGER rf022_block_published;")
        .unwrap();
    drop(owned);
    std::fs::remove_file(source).unwrap();
    let fresh = fixture.reopen();
    let result = resumed(
        &fresh,
        &fixture.account,
        fixture.dir.path(),
        &id,
        None,
        &mut counts,
    )
    .unwrap();
    assert_eq!(result.phase, ImportOperationPhase::Complete);
    assert_eq!(counts.written_file_count, 1);
    assert_eq!(counts.committed_count, 1);
    assert_attachments(&fresh, OWNER, 1);
}

#[test]
fn rf022_second_attachment_collision_preserves_first_and_retry_resumes_frozen_map_without_reimport()
{
    let fixture = Fixture::new();
    let source = fixture.package(&fixture.payload(false), None);
    let owned = fixture.owned(&source);
    let id = op_id();
    let (view, batch, start) =
        fixture.prepare(&owned, &id, ImportSourceKind::Cloud, None, HashMap::new());
    fixture.commit(&view, &batch, &start);
    let blocked = final_file(fixture.dir.path(), &start.steps[1])
        .parent()
        .unwrap()
        .to_owned();
    std::fs::create_dir_all(&blocked).unwrap();
    std::fs::write(blocked.join("foreign"), b"never overwrite").unwrap();
    let mut counts = AttachmentImportProgress::default();
    let failure = fixture
        .run(&id, Some(&owned), Some(PACKAGE_PASSWORD), &mut counts)
        .unwrap_err();
    assert_eq!(failure.to_string(), "import_target_collision");
    assert_eq!(counts.written_file_count, 1);
    assert_eq!(counts.committed_count, 0);
    assert_eq!(
        std::fs::read(blocked.join("foreign")).unwrap(),
        b"never overwrite"
    );
    let before = fixture.vault.load_object(OWNER).unwrap().unwrap();
    let pending = fixture
        .vault
        .load_import_operation(&fixture.account, &id)
        .unwrap()
        .unwrap();
    assert_eq!(pending.steps[0].phase, ImportAttachmentPhase::Published);
    assert_eq!(pending.steps[1].phase, ImportAttachmentPhase::Staged);
    std::fs::remove_file(blocked.join("foreign")).unwrap();
    std::fs::remove_dir(&blocked).unwrap();
    drop(owned);
    std::fs::remove_file(&source).unwrap();
    let fresh = fixture.reopen();
    resumed(
        &fresh,
        &fixture.account,
        fixture.dir.path(),
        &id,
        None,
        &mut counts,
    )
    .unwrap();
    assert_eq!(counts.written_file_count, 2);
    assert_eq!(counts.committed_count, 2);
    let after = fresh
        .get_vault_store()
        .unwrap()
        .load_object(OWNER)
        .unwrap()
        .unwrap();
    assert_eq!(after.name, before.name);
    assert_attachments(&fresh, OWNER, 2);
}

#[test]
fn rf022_source_changed_and_old_session_are_rejected_before_attachment_publication() {
    let fixture = Fixture::new();
    let source = fixture.package(&fixture.payload(false), None);
    let owned = fixture.owned(&source);
    let id = op_id();
    let (view, batch, start) =
        fixture.prepare(&owned, &id, ImportSourceKind::Manual, None, HashMap::new());
    fixture.commit(&view, &batch, &start);
    let another = fixture.package(&fixture.payload(true), None);
    let changed = fixture.owned(&another);
    let mut counts = AttachmentImportProgress::default();
    assert_eq!(
        fixture
            .run(&id, Some(&changed), Some(PACKAGE_PASSWORD), &mut counts)
            .unwrap_err()
            .to_string(),
        "import_source_changed"
    );
    assert_eq!(counts.written_file_count, 0);
    let old = fixture.session();
    let key = fixture.key();
    fixture.service.lock();
    let result = resume_import_operation(
        &fixture.service,
        &old,
        &id,
        fixture.dir.path(),
        Some(&owned),
        Some(PACKAGE_PASSWORD),
        &key,
        None,
        &mut counts,
    );
    assert!(result.is_err());
    for step in &start.steps {
        assert!(!final_file(fixture.dir.path(), step).exists());
    }
}

#[test]
fn rf022_epoch_reclaim_rejects_old_worker_confirm_and_publish() {
    let fixture = Fixture::new();
    let source = fixture.package(&fixture.payload(false), None);
    let owned = fixture.owned(&source);
    let id = op_id();
    let (view, batch, start) =
        fixture.prepare(&owned, &id, ImportSourceKind::Manual, None, HashMap::new());
    fixture.commit(&view, &batch, &start);
    let first = fixture
        .vault
        .claim_import_operation(&fixture.account, &id, &start.root_binding)
        .unwrap();
    let opened = owned.decrypt(PACKAGE_PASSWORD, fixture.dir.path()).unwrap();
    let proof = stage_attachment(
        &owned,
        &opened,
        fixture.dir.path(),
        &fixture.account,
        &id,
        &start.root_binding,
        first.epoch(),
        &start.steps[0],
        &fixture.key(),
    )
    .unwrap();
    let second = fixture
        .vault
        .claim_import_operation(&fixture.account, &id, &start.root_binding)
        .unwrap();
    assert!(second.epoch() > first.epoch());
    assert!(fixture
        .vault
        .confirm_import_attachment_staged(&first, start.steps[0].entry_ordinal, &proof)
        .is_err());
    let fired = std::cell::Cell::new(false);
    assert!(fixture
        .vault
        .publish_import_attachment(&first, start.steps[0].entry_ordinal, |_| {
            fired.set(true);
            Ok(())
        })
        .is_err());
    assert!(!fired.get());
    assert!(fixture
        .vault
        .confirm_import_attachment_staged(&second, start.steps[0].entry_ordinal, &proof)
        .is_err());
    assert!(!final_file(fixture.dir.path(), &start.steps[0]).exists());
}

#[test]
fn rf022_new_current_key_must_authenticate_confirmed_stage_before_publish() {
    let fixture = Fixture::new();
    let source = fixture.package(&fixture.payload(false), None);
    let owned = fixture.owned(&source);
    let id = op_id();
    let (view, batch, start) =
        fixture.prepare(&owned, &id, ImportSourceKind::Manual, None, HashMap::new());
    fixture.commit(&view, &batch, &start);
    let lease = fixture
        .vault
        .claim_import_operation(&fixture.account, &id, &start.root_binding)
        .unwrap();
    let opened = owned.decrypt(PACKAGE_PASSWORD, fixture.dir.path()).unwrap();
    let proof = stage_attachment(
        &owned,
        &opened,
        fixture.dir.path(),
        &fixture.account,
        &id,
        &start.root_binding,
        lease.epoch(),
        &start.steps[0],
        &fixture.key(),
    )
    .unwrap();
    fixture
        .vault
        .confirm_import_attachment_staged(&lease, start.steps[0].entry_ordinal, &proof)
        .unwrap();
    let mut counts = AttachmentImportProgress::default();
    let wrong = [0x77; 32];
    assert_eq!(
        resume_import_operation(
            &fixture.service,
            &fixture.session(),
            &id,
            fixture.dir.path(),
            Some(&owned),
            Some(PACKAGE_PASSWORD),
            &wrong,
            None,
            &mut counts
        )
        .unwrap_err()
        .to_string(),
        "import_staged_key_mismatch"
    );
    assert_eq!(counts.written_file_count, 0);
    assert!(!final_file(fixture.dir.path(), &start.steps[0]).exists());
}

#[test]
fn rf022_parallel_field_edit_survives_and_concurrent_attachment_edit_blocks_activation() {
    for edit_attachments in [false, true] {
        let fixture = Fixture::new();
        let source = fixture.package(&fixture.payload(false), None);
        let owned = fixture.owned(&source);
        let id = op_id();
        let (view, batch, start) =
            fixture.prepare(&owned, &id, ImportSourceKind::Manual, None, HashMap::new());
        fixture.commit(&view, &batch, &start);
        let mut record = fixture.vault.load_object(OWNER).unwrap().unwrap();
        record.properties["body"] = json!("concurrent user edit");
        if edit_attachments {
            record.properties["__attachments"] =
                json!([{"id":"user_attachment","fileName":"user.txt"}]);
        }
        fixture.vault.save_object(&record).unwrap();
        let mut counts = AttachmentImportProgress::default();
        let result = fixture.run(&id, Some(&owned), Some(PACKAGE_PASSWORD), &mut counts);
        let current = fixture.vault.load_object(OWNER).unwrap().unwrap();
        assert_eq!(current.properties["body"], "concurrent user edit");
        if edit_attachments {
            assert!(result.is_err());
            assert_eq!(counts.committed_count, 0);
            assert_eq!(
                current.properties["__attachments"],
                record.properties["__attachments"]
            );
        } else {
            assert_eq!(result.unwrap().phase, ImportOperationPhase::Complete);
            assert_attachments(&fixture.service, OWNER, 2);
        }
    }
}

#[test]
fn rf022_zero_attachment_selection_preserves_local_available_collection() {
    let fixture = Fixture::new();
    let payload = fixture.payload(false);
    let source = fixture.package(&payload, None);
    let owned = fixture.owned(&source);
    let mut prepared = super::super::prepare_import_database(
        &fixture.vault,
        &fixture.account,
        &payload,
        ImportStrategy::Overwrite,
        NOW,
    )
    .unwrap();
    let local = json!([{"id":"existing_local","objectId":OWNER,"fileName":"local.txt","mimeType":"text/plain","sizeBytes":10,"createdAt":NOW,"vaultPath":"known local"}]);
    prepared.batch.objects[0].record.properties["__attachments"] = local.clone();
    fixture
        .vault
        .save_object(&prepared.batch.objects[0].record)
        .unwrap();
    let empty = HashSet::new();
    let id = op_id();
    let (view, batch, start) = fixture.prepare(
        &owned,
        &id,
        ImportSourceKind::Manual,
        Some(&empty),
        HashMap::new(),
    );
    assert!(start.steps.is_empty());
    fixture.commit(&view, &batch, &start);
    let mut counts = AttachmentImportProgress::default();
    fixture.run(&id, None, None, &mut counts).unwrap();
    assert_eq!(
        fixture
            .vault
            .load_object(OWNER)
            .unwrap()
            .unwrap()
            .properties["__attachments"],
        local
    );
    assert_eq!(counts.committed_count, 0);
}

#[test]
fn rf022_recovery_ready_handoff_reopens_without_original_random_package_credential() {
    let fixture = Fixture::new();
    let profile_bytes = b"{\"preferences\":{\"theme\":\"dark\"}}";
    let source = fixture.package(&fixture.payload(false), Some(profile_bytes));
    let owned = fixture.owned(&source);
    let id = op_id();
    let (view, batch, mut start) = fixture.prepare(
        &owned,
        &id,
        ImportSourceKind::Recovery,
        None,
        HashMap::new(),
    );
    let opened = owned.decrypt(PACKAGE_PASSWORD, fixture.dir.path()).unwrap();
    let handoff = prepare_recovery_handoff(
        &fixture.service,
        &fixture.session(),
        fixture.dir.path(),
        &owned,
        &opened,
        &mut start,
        &fixture.key(),
    )
    .unwrap();
    assert!(fixture.vault.load_object(OWNER).unwrap().is_none());
    assert!(fixture
        .vault
        .load_import_operation(&fixture.account, &id)
        .unwrap()
        .is_none());
    assert!(start.steps.iter().all(|step| step
        .initial_staged_proof
        .as_ref()
        .is_some_and(|proof| proof.stage_epoch == 0)));
    fixture.commit(&view, &batch, &start);
    handoff.accept();
    drop(opened);
    drop(owned);
    std::fs::remove_file(source).unwrap();
    let fresh = fixture.reopen();
    let mut counts = AttachmentImportProgress::default();
    let result = resumed(
        &fresh,
        &fixture.account,
        fixture.dir.path(),
        &id,
        None,
        &mut counts,
    )
    .unwrap();
    assert_eq!(result.phase, ImportOperationPhase::Complete);
    assert_eq!(counts.committed_count, 2);
    assert!(result.preferences_imported);
    assert_eq!(
        fresh
            .get_vault_store()
            .unwrap()
            .load_profile(&fixture.account)
            .unwrap()
            .unwrap()
            .data,
        profile_bytes
    );
    assert_attachments(&fresh, OWNER, 2);
}

#[test]
fn rf022_recovery_precommit_failure_keeps_business_and_journal_absent_and_cleans_owned_handoff() {
    let fixture = Fixture::new();
    let source = fixture.package(&fixture.payload(false), None);
    let owned = fixture.owned(&source);
    let id = op_id();
    fixture.db.execute_batch("CREATE TRIGGER rf022_abort_recovery BEFORE INSERT ON objects BEGIN SELECT RAISE(ABORT,'rf022 recovery failure'); END;").unwrap();
    let (view, batch, mut start) = fixture.prepare(
        &owned,
        &id,
        ImportSourceKind::Recovery,
        None,
        HashMap::new(),
    );
    let opened = owned.decrypt(PACKAGE_PASSWORD, fixture.dir.path()).unwrap();
    let handoff = prepare_recovery_handoff(
        &fixture.service,
        &fixture.session(),
        fixture.dir.path(),
        &owned,
        &opened,
        &mut start,
        &fixture.key(),
    )
    .unwrap();
    assert!(fixture
        .vault
        .commit_import_batch_with_operation(&fixture.account, &view.revision, &batch, &start)
        .is_err());
    drop(handoff);
    assert!(fixture.vault.load_object(OWNER).unwrap().is_none());
    assert!(fixture
        .vault
        .load_import_operation(&fixture.account, &id)
        .unwrap()
        .is_none());
    assert!(!fixture.dir.path().join(OPERATION_DIR).join(id).exists());
}

// Run by independent parent tests below, never claiming ordinary Err injection is a process crash.
#[test]
fn rf022_child_process_checkpoint() {
    let Ok(root) = std::env::var("SOLOSOUL_RF022_CHILD_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let account = std::env::var("SOLOSOUL_RF022_CHILD_ACCOUNT").unwrap();
    let id = std::env::var("SOLOSOUL_RF022_CHILD_OPERATION").unwrap();
    let mode = std::env::var("SOLOSOUL_RF022_CHILD_MODE").unwrap();
    let service = crate::VaultService::with_base_path(root.clone());
    service
        .unlock_secure(&account, &Zeroizing::new(PASSWORD.to_owned()))
        .unwrap();
    let session = service.capture_session(&account).unwrap();
    let key = service.attachment_key_for_session(&session).unwrap();
    let source = PathBuf::from(std::env::var("SOLOSOUL_RF022_CHILD_SOURCE").unwrap());
    let owned = OwnedImportPackage::capture(&source, &root).unwrap();
    let opened = owned.decrypt(PACKAGE_PASSWORD, &root).unwrap();
    let mut prepared = super::super::prepare_import_database(
        session.vault(),
        &account,
        &opened.payload,
        ImportStrategy::Overwrite,
        NOW,
    )
    .unwrap();
    let start = prepare_import_operation(
        &id,
        ImportSourceKind::Cli,
        json!({"strategy":"overwrite"}),
        &owned,
        &opened.payload,
        &prepared.imported_object_ids,
        &HashMap::new(),
        None,
        NOW,
        &root,
        session.vault(),
        &prepared.view,
        &mut prepared.batch,
        true,
        false,
    )
    .unwrap();
    service
        .with_session(&session, |vault| {
            vault.commit_import_batch_with_operation(
                &account,
                &prepared.view.revision,
                &prepared.batch,
                &start,
            )
        })
        .unwrap();
    if mode == "database" {
        std::process::exit(73);
    }
    let last_ordinal = start.steps[1].entry_ordinal;
    let progress = |percent: u8| {
        // The second entry callback follows actual first publication + journal COMMIT.
        let record = session
            .vault()
            .load_import_operation(&account, &id)
            .unwrap()
            .unwrap();
        if mode == "published"
            && record
                .steps
                .iter()
                .any(|step| step.phase == ImportAttachmentPhase::Published)
        {
            assert_eq!(record.attachment_count, 0);
            std::process::exit(73);
        }
        let _ = (percent, last_ordinal);
    };
    let mut counts = AttachmentImportProgress::default();
    resume_import_operation(
        &service,
        &session,
        &id,
        &root,
        Some(&owned),
        Some(PACKAGE_PASSWORD),
        &key,
        Some(&progress),
        &mut counts,
    )
    .unwrap();
    panic!("child checkpoint was not reached");
}

fn child_checkpoint(mode: &str) {
    let fixture = Fixture::new();
    let source = fixture.package(&fixture.payload(false), None);
    let id = op_id();
    fixture.service.lock();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .arg("--exact")
        .arg("export_import::operation::tests::rf022_child_process_checkpoint")
        .arg("--nocapture")
        .env("SOLOSOUL_RF022_CHILD_ROOT", fixture.dir.path())
        .env("SOLOSOUL_RF022_CHILD_ACCOUNT", &fixture.account)
        .env("SOLOSOUL_RF022_CHILD_OPERATION", &id)
        .env("SOLOSOUL_RF022_CHILD_SOURCE", &source)
        .env("SOLOSOUL_RF022_CHILD_MODE", mode);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let output = command.output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(73),
        "child must exit at real checkpoint: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let service = crate::VaultService::with_base_path(fixture.dir.path().into());
    service
        .unlock_secure(&fixture.account, &Zeroizing::new(PASSWORD.to_owned()))
        .unwrap();
    let vault = service.get_vault_store().unwrap();
    let record = vault
        .load_import_operation(&fixture.account, &id)
        .unwrap()
        .unwrap();
    assert_eq!(record.database_commit.object_write_count, 1);
    assert_eq!(record.attachment_count, 0);
    assert!(vault.load_object(OWNER).unwrap().is_some());
    if mode == "published" {
        assert_eq!(
            record
                .steps
                .iter()
                .filter(|step| step.phase == ImportAttachmentPhase::Published)
                .count(),
            1
        );
    } else {
        assert!(record
            .steps
            .iter()
            .all(|step| step.phase == ImportAttachmentPhase::Planned));
    }
    let owned = OwnedImportPackage::capture(&source, fixture.dir.path()).unwrap();
    let mut counts = AttachmentImportProgress::default();
    resumed(
        &service,
        &fixture.account,
        fixture.dir.path(),
        &id,
        Some(&owned),
        &mut counts,
    )
    .unwrap();
    assert_eq!(counts.written_file_count, 2);
    assert_eq!(counts.committed_count, 2);
    assert_attachments(&service, OWNER, 2);
    resumed(
        &service,
        &fixture.account,
        fixture.dir.path(),
        &id,
        None,
        &mut counts,
    )
    .unwrap();
    assert_eq!(counts.committed_count, 2);
}

#[test]
fn rf022_new_process_reopens_database_committed_operation_and_resumes_same_plan() {
    child_checkpoint("database");
}
#[test]
fn rf022_new_process_reopens_after_actual_first_publication_before_metadata() {
    child_checkpoint("published");
}

#[test]
fn rf022_recovery_corrupt_preferences_rejects_handoff_before_business_commit_and_removes_owned_stages(
) {
    let fixture = Fixture::new();
    let source = fixture.package(&fixture.payload(false), Some(b"profile preferences"));
    let broken = fixture.dir.path().join("broken-preferences.solosoul");
    {
        let mut archive = ZipArchive::new(File::open(&source).unwrap()).unwrap();
        let mut zip = ZipWriter::new(File::create(&broken).unwrap());
        for ordinal in 0..archive.len() {
            let mut entry = archive.by_index(ordinal).unwrap();
            let name = entry.name().to_string();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            if name == "preferences.enc" {
                *bytes.last_mut().unwrap() ^= 1;
            }
            zip.start_file(name, SimpleFileOptions::default()).unwrap();
            zip.write_all(&bytes).unwrap();
        }
        zip.finish().unwrap();
    }
    let owned = fixture.owned(&broken);
    let id = op_id();
    let (_view, _batch, mut start) = fixture.prepare(
        &owned,
        &id,
        ImportSourceKind::Recovery,
        None,
        HashMap::new(),
    );
    let opened = owned.decrypt(PACKAGE_PASSWORD, fixture.dir.path()).unwrap();
    let result = prepare_recovery_handoff(
        &fixture.service,
        &fixture.session(),
        fixture.dir.path(),
        &owned,
        &opened,
        &mut start,
        &fixture.key(),
    );
    assert_eq!(
        result
            .err()
            .expect("incomplete preferences cannot be accepted")
            .to_string(),
        "import_recovery_preferences_failed"
    );
    assert!(fixture.vault.load_object(OWNER).unwrap().is_none());
    assert!(fixture
        .vault
        .load_import_operation(&fixture.account, &id)
        .unwrap()
        .is_none());
    assert!(start.source_ready.is_none());
    assert!(!fixture.dir.path().join(OPERATION_DIR).join(&id).exists());
}
