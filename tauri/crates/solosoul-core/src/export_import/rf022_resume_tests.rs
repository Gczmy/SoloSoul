//! RF022 Core parent：真实加密包 / SQLite / Session wrapper 回归；未执行候选。
use super::*;
use rusqlite::{types::ValueRef, Connection};
use serde_json::{json, Value};
use solosoul_vault::{ImportAttachmentPhase, ImportOperationPhase};
use std::collections::BTreeMap;

const PASSWORD: &str = "RF022-core-vault-password";
const PACKAGE_PASSWORD: &str = "RF022-Core-Export1";
const NOW: &str = "2026-10-01T00:00:00Z";
const OWNER: &str = "rf022_core_owner";

struct Fixture {
    dir: tempfile::TempDir,
    service: crate::VaultService,
    account: String,
    package: PathBuf,
}

impl Fixture {
    fn new(payload: Value) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let service = crate::VaultService::with_base_path(dir.path().into());
        let account = service
            .create_account("RF022 Core wrapper", PASSWORD, None)
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let package = write_package(dir.path(), &payload);
        Self {
            dir,
            service,
            account,
            package,
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
    fn db(&self) -> Connection {
        Connection::open(self.dir.path().join(&self.account).join("vault.db")).unwrap()
    }
    fn run(
        &self,
        id: &str,
        strategy: ImportStrategy,
        password: &str,
    ) -> Result<CoreImportOperationOutcome, ExportError> {
        import_vault_resumable(
            &self.service,
            &self.session(),
            id,
            &self.package,
            password,
            strategy,
            self.dir.path(),
            &self.key(),
        )
    }
    fn operation(&self, id: &str) -> solosoul_vault::ImportOperationRecord {
        self.service
            .get_vault_store()
            .unwrap()
            .load_import_operation(&self.account, id)
            .unwrap()
            .unwrap()
    }
    fn reopen(&self) -> crate::VaultService {
        self.service.lock();
        // 同应用重开连接显式复用 owner；独立进程竞争由专门子进程用例验证。
        let owner = self.service.root_owner();
        let fs = std::sync::Arc::new(crate::LocalVaultFileSystem::new(owner.root().to_path_buf()));
        let service = crate::VaultService::try_with_root_owner(owner, fs).unwrap();
        service
            .unlock_secure(&self.account, &Zeroizing::new(PASSWORD.into()))
            .unwrap();
        service
    }
}

fn object(id: &str, name: &str, attachment: bool) -> Value {
    let mut properties = json!({"body":format!("{name} synthetic secret")});
    if attachment {
        properties["__attachments"] = json!([{"id":"att_source", "objectId":id,
            "fileName":"synthetic.txt", "mimeType":"text/plain", "sizeBytes":17,
            "createdAt":NOW, "vaultPath":"/source/not/target", "tags":[]}]);
    }
    json!({"id":id,"type_id":"note","section_type":"identity","name":name,
        "properties":properties,"created_at":NOW,"version":1})
}

fn write_package(root: &Path, payload: &Value) -> PathBuf {
    let path = root.join(format!("{}.solosoul", uuid::Uuid::new_v4()));
    let salt = solosoul_crypto::kdf::generate_salt();
    let config = KdfConfig::development();
    let key = derive_export_key_cfg(PACKAGE_PASSWORD, &salt, &config).unwrap();
    let att_key =
        solosoul_crypto::hkdf_ext::derive_hkdf_key(&key, &salt, b"solosoul:attachments:v1")
            .unwrap();
    let has_attachments = payload["objects"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value["properties"]["__attachments"].is_array());
    let mut archive = ZipWriter::new(File::create(&path).unwrap());
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    archive.start_file("manifest.json", options).unwrap();
    archive
        .write_all(
            json!({"version":"2.0","salt_hex":hex::encode(salt),"has_attachments":has_attachments,
        "extra_files":[],"kdf":kdf_to_manifest_value(&config)})
            .to_string()
            .as_bytes(),
        )
        .unwrap();
    archive.start_file("payload.enc", options).unwrap();
    let bytes = serde_json::to_vec(payload).unwrap();
    solosoul_crypto::cipher::encrypt_chunked_stream(
        &key,
        bytes.len() as u64,
        &mut std::io::Cursor::new(bytes),
        &mut archive,
    )
    .unwrap();
    let mut emitted = HashSet::new();
    for value in payload["objects"].as_array().unwrap() {
        if let Some(attachments) = value["properties"]["__attachments"].as_array() {
            for attachment in attachments {
                let id = value["id"].as_str().unwrap();
                let att_id = attachment["id"].as_str().unwrap();
                if !emitted.insert((id, att_id)) {
                    continue;
                }
                archive
                    .start_file(format!("attachments/{id}/{att_id}.enc"), options)
                    .unwrap();
                let bytes = b"synthetic content";
                solosoul_crypto::cipher::encrypt_chunked_stream(
                    &att_key,
                    bytes.len() as u64,
                    &mut std::io::Cursor::new(bytes),
                    &mut archive,
                )
                .unwrap();
            }
        }
    }
    archive.finish().unwrap();
    path
}

type DatabaseState = BTreeMap<String, Vec<Vec<Vec<u8>>>>;
fn state(db: &Connection) -> DatabaseState {
    let mut result = BTreeMap::new();
    for table in [
        "objects",
        "user_templates",
        "object_snapshots",
        "sync_hlc",
        "import_operations",
        "import_attachment_steps",
    ] {
        let mut query = db.prepare(&format!("SELECT * FROM {table}")).unwrap();
        let columns = query.column_count();
        let mut cursor = query.query([]).unwrap();
        let mut rows = Vec::new();
        while let Some(row) = cursor.next().unwrap() {
            let cells = (0..columns)
                .map(|index| match row.get_ref(index).unwrap() {
                    ValueRef::Null => vec![b'N'],
                    ValueRef::Integer(value) => [vec![b'I'], value.to_le_bytes().to_vec()].concat(),
                    ValueRef::Real(value) => {
                        [vec![b'R'], value.to_bits().to_le_bytes().to_vec()].concat()
                    }
                    ValueRef::Text(value) => [vec![b'T'], value.to_vec()].concat(),
                    ValueRef::Blob(value) => [vec![b'B'], value.to_vec()].concat(),
                })
                .collect::<Vec<_>>();
            rows.push(cells);
        }
        rows.sort();
        result.insert(table.into(), rows);
    }
    result
}

#[test]
fn rf022_core_same_id_complete_fresh_reads_original_plan_without_reprepare() {
    let f = Fixture::new(json!({"objects":[object(OWNER,"first",false)]}));
    let id = uuid::Uuid::new_v4().to_string();
    assert!(
        f.run(&id, ImportStrategy::Overwrite, PACKAGE_PASSWORD)
            .unwrap()
            .complete
    );
    f.db().execute_batch("CREATE TRIGGER rf022_block_record BEFORE UPDATE ON objects BEGIN SELECT RAISE(ABORT,'no reprepare'); END;").unwrap();
    let before = state(&f.db());
    let replay = f
        .run(
            &id,
            ImportStrategy::Overwrite,
            "deliberately wrong new password",
        )
        .unwrap();
    assert!(replay.complete);
    assert_eq!(replay.object_write_count, 1);
    assert_eq!(state(&f.db()), before);
    assert!(f
        .service
        .get_vault_store()
        .unwrap()
        .list_import_operations(&f.account)
        .unwrap()
        .is_empty());
}

#[test]
fn rf022_core_same_id_changed_source_or_strategy_is_conflict_without_database_write() {
    let mut f = Fixture::new(json!({"objects":[object(OWNER,"first",false)]}));
    let id = uuid::Uuid::new_v4().to_string();
    f.run(&id, ImportStrategy::Overwrite, PACKAGE_PASSWORD)
        .unwrap();
    let before = state(&f.db());
    assert_eq!(
        f.run(&id, ImportStrategy::SkipExisting, PACKAGE_PASSWORD)
            .unwrap_err()
            .to_string(),
        "__IMPORT_ERR__:OPERATION_CONFLICT"
    );
    let original_name = f.package.file_name().unwrap().to_os_string();
    let other = write_package(
        f.dir.path(),
        &json!({"objects":[object(OWNER,"changed",false)]}),
    );
    let replacement_dir = f.dir.path().join("replacement");
    std::fs::create_dir(&replacement_dir).unwrap();
    let replacement = replacement_dir.join(original_name);
    std::fs::rename(other, &replacement).unwrap();
    f.package = replacement;
    assert_eq!(
        f.run(&id, ImportStrategy::Overwrite, PACKAGE_PASSWORD)
            .unwrap_err()
            .to_string(),
        "__IMPORT_ERR__:OPERATION_CONFLICT"
    );
    assert_eq!(state(&f.db()), before);
}

#[test]
fn rf022_core_duplicate_new_ids_preserve_per_write_count_across_reopen_resume() {
    let f =
        Fixture::new(json!({"objects":[object(OWNER,"first",false),object(OWNER,"last",false)]}));
    let id = uuid::Uuid::new_v4().to_string();
    let first = f
        .run(&id, ImportStrategy::SkipExisting, PACKAGE_PASSWORD)
        .unwrap();
    assert!(first.complete);
    assert_eq!(first.object_write_count, 2);
    let operation = f.operation(&id);
    assert_eq!(operation.start.plan["cliObjectWriteCount"], 2);
    assert_eq!(
        f.service
            .get_vault_store()
            .unwrap()
            .load_object(OWNER)
            .unwrap()
            .unwrap()
            .name,
        "last"
    );
    let reopened = f.reopen();
    let session = reopened.capture_session(&f.account).unwrap();
    let key = reopened.attachment_key_for_session(&session).unwrap();
    std::fs::remove_file(&f.package).unwrap();
    let before = state(&f.db());
    let result =
        resume_vault_import(&reopened, &session, &id, None, None, f.dir.path(), &key).unwrap();
    assert!(result.complete);
    assert_eq!(result.object_write_count, 2);
    assert_eq!(state(&f.db()), before);
}

#[test]
fn rf022_core_skip_membership_is_before_batch_and_merge_keeps_existing_history() {
    let f = Fixture::new(
        json!({"objects":[object("already","source",false),object(OWNER,"first",false),object(OWNER,"last",false)]}),
    );
    let vault = f.service.get_vault_store().unwrap();
    vault
        .save_object(&ObjectRecord {
            id: "already".into(),
            account_id: f.account.clone(),
            name: "local".into(),
            properties: json!({"body":"local"}),
            created_at: NOW.into(),
            updated_at: NOW.into(),
            ..Default::default()
        })
        .unwrap();
    vault
        .save_snapshot("already", "user_edit", b"local encrypted history", "local")
        .unwrap();
    let before = state(&f.db())["object_snapshots"].clone();
    let skip = f
        .run(
            &uuid::Uuid::new_v4().to_string(),
            ImportStrategy::SkipExisting,
            PACKAGE_PASSWORD,
        )
        .unwrap();
    assert_eq!(skip.object_write_count, 2);
    assert_eq!(vault.load_object("already").unwrap().unwrap().name, "local");
    let merge = f
        .run(
            &uuid::Uuid::new_v4().to_string(),
            ImportStrategy::Merge,
            PACKAGE_PASSWORD,
        )
        .unwrap();
    assert_eq!(merge.object_write_count, 3);
    assert_eq!(
        vault.load_object("already").unwrap().unwrap().name,
        "source"
    );
    assert_eq!(state(&f.db())["object_snapshots"], before);
}

#[test]
fn rf022_core_journal_insert_failure_rolls_back_records_and_journal_together() {
    let f = Fixture::new(json!({"objects":[object(OWNER,"first",false)]}));
    f.db().execute_batch("CREATE TRIGGER rf022_fail_journal BEFORE INSERT ON import_operations BEGIN SELECT RAISE(ABORT,'journal insert fault'); END;").unwrap();
    let before = state(&f.db());
    let id = uuid::Uuid::new_v4().to_string();
    assert_eq!(
        f.run(&id, ImportStrategy::Overwrite, PACKAGE_PASSWORD)
            .unwrap_err()
            .to_string(),
        "import_operation_write_failed"
    );
    assert_eq!(state(&f.db()), before);
    assert!(f
        .service
        .get_vault_store()
        .unwrap()
        .load_import_operation(&f.account, &id)
        .unwrap()
        .is_none());
}

#[test]
fn rf022_core_published_phase_commit_failure_preserves_actual_file_then_resume_without_source_password(
) {
    let f = Fixture::new(json!({"objects":[object(OWNER,"first",true)]}));
    f.db().execute_batch("CREATE TRIGGER rf022_fail_published BEFORE UPDATE OF phase ON import_attachment_steps WHEN NEW.phase='published' BEGIN SELECT RAISE(ABORT,'published commit fault'); END;").unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let partial = f
        .run(&id, ImportStrategy::Overwrite, PACKAGE_PASSWORD)
        .unwrap();
    assert!(!partial.complete);
    assert_eq!(partial.object_write_count, 1);
    assert_eq!(partial.attachment_count, 0);
    assert_eq!(partial.written_file_count, 1);
    let original = f.operation(&id);
    assert_eq!(original.steps[0].phase, ImportAttachmentPhase::Staged);
    let requirements = operation::import_credential_requirements(&original).unwrap();
    assert!(!requirements.source_required);
    assert!(!requirements.password_required);
    let attachment_id = original.steps[0].plan.attachment_id.clone();
    f.db()
        .execute_batch("DROP TRIGGER rf022_fail_published;")
        .unwrap();
    let reopened = f.reopen();
    let session = reopened.capture_session(&f.account).unwrap();
    let key = reopened.attachment_key_for_session(&session).unwrap();
    std::fs::remove_file(&f.package).unwrap();
    let result =
        resume_vault_import(&reopened, &session, &id, None, None, f.dir.path(), &key).unwrap();
    assert!(result.complete);
    assert_eq!(result.attachment_count, 1);
    assert_eq!(result.written_file_count, 1);
    let complete = session
        .vault()
        .load_import_operation(&f.account, &id)
        .unwrap()
        .unwrap();
    assert_eq!(complete.phase, ImportOperationPhase::Complete);
    assert_eq!(complete.steps[0].plan.attachment_id, attachment_id);
    let file = f
        .dir
        .path()
        .join("attachments")
        .join(OWNER)
        .join(&attachment_id)
        .join("synthetic.txt");
    assert!(crate::attachment_crypto::is_encrypted_file(&file));
    assert_eq!(
        crate::attachment_crypto::read_file_decrypted(&key, &file, 1024).unwrap(),
        b"synthetic content"
    );
}

#[test]
fn rf022_core_stale_original_session_and_missing_operation_have_zero_business_write() {
    let f = Fixture::new(json!({"objects":[object(OWNER,"first",false)]}));
    let session = f.session();
    let key = f.key();
    f.service.lock();
    f.service.unlock(&f.account, PASSWORD).unwrap();
    let before = state(&f.db());
    let error = import_vault_resumable(
        &f.service,
        &session,
        &uuid::Uuid::new_v4().to_string(),
        &f.package,
        PACKAGE_PASSWORD,
        ImportStrategy::Overwrite,
        f.dir.path(),
        &key,
    )
    .unwrap_err();
    assert_eq!(error.to_string(), "Vault session is no longer current");
    let error = resume_vault_import(
        &f.service,
        &f.session(),
        &uuid::Uuid::new_v4().to_string(),
        None,
        None,
        f.dir.path(),
        &f.key(),
    )
    .unwrap_err();
    assert_eq!(error.to_string(), "__IMPORT_ERR__:OPERATION_NOT_FOUND");
    assert_eq!(state(&f.db()), before);
}

#[test]
fn rf022_core_legacy_direct_none_key_remains_one_shot_usize_without_journal() {
    let f =
        Fixture::new(json!({"objects":[object(OWNER,"first",false),object(OWNER,"last",false)]}));
    let vault = f.service.get_vault_store().unwrap();
    let count = import_vault(
        &vault,
        &f.account,
        &f.package,
        PACKAGE_PASSWORD,
        ImportStrategy::Overwrite,
        f.dir.path(),
        None,
    )
    .unwrap();
    assert_eq!(count, 2);
    assert!(vault.list_import_operations(&f.account).unwrap().is_empty());
    assert_eq!(
        f.db()
            .query_row("SELECT COUNT(*) FROM import_operations", [], |r| r
                .get::<_, usize>(0))
            .unwrap(),
        0
    );
}

#[test]
fn rf022_core_complete_replay_wrong_native_root_is_not_success_and_writes_nothing() {
    let f = Fixture::new(json!({"objects":[object(OWNER,"first",false)]}));
    let id = uuid::Uuid::new_v4().to_string();
    assert!(
        f.run(&id, ImportStrategy::Overwrite, PACKAGE_PASSWORD)
            .unwrap()
            .complete
    );
    let before = state(&f.db());
    let wrong_root = tempfile::tempdir().unwrap();
    let session = f.session();
    let key = f.key();
    let rejected = resume_vault_import(
        &f.service,
        &session,
        &id,
        None,
        None,
        wrong_root.path(),
        &key,
    )
    .unwrap();
    assert!(!rejected.complete);
    assert_eq!(
        rejected.error_code.as_deref(),
        Some("import_operation_plan_mismatch")
    );
    assert_eq!(state(&f.db()), before);
    assert_eq!(std::fs::read_dir(wrong_root.path()).unwrap().count(), 0);
    let original =
        resume_vault_import(&f.service, &session, &id, None, None, f.dir.path(), &key).unwrap();
    assert!(original.complete);
    assert!(original.error_code.is_none());
    assert_eq!(original.object_write_count, 1);
    assert_eq!(state(&f.db()), before);
}

#[test]
fn rf022_core_manual_duplicate_ids_resume_preserves_host_unique_object_count() {
    // 真包中两条同 ID，经真实准备器和带 journal 的事务提交；不改 phase/source_kind。
    let f = Fixture::new(json!({
        "objects": [object(OWNER, "first", false), object(OWNER, "last", false)]
    }));
    let id = uuid::Uuid::new_v4().to_string();
    let session = f.session();
    let key = f.service.attachment_key_for_session(&session).unwrap();
    let initial_object_count: i64 = f
        .db()
        .query_row("SELECT COUNT(*) FROM objects", [], |row| row.get(0))
        .unwrap();

    let owned = operation::OwnedImportPackage::capture(&f.package, f.dir.path()).unwrap();
    let opened = owned.decrypt(PACKAGE_PASSWORD, f.dir.path()).unwrap();
    let mut prepared = prepare_import_database(
        session.vault(),
        session.account_id(),
        &opened.payload,
        ImportStrategy::Overwrite,
        NOW,
    )
    .unwrap();
    assert_eq!(prepared.batch.objects.len(), 2);
    let start = operation::prepare_import_operation(
        &id,
        solosoul_vault::ImportSourceKind::Manual,
        json!({"strategy":"overwrite", "sourceName":"manual-duplicates.solosoul",
            "includeAttachments":false, "includePreferences":false}),
        &owned,
        &opened.payload,
        &prepared.imported_object_ids,
        &HashMap::new(),
        None,
        NOW,
        f.dir.path(),
        session.vault(),
        &prepared.view,
        &mut prepared.batch,
        opened.has_attachments,
        opened.has_preferences,
    )
    .unwrap();
    assert_eq!(start.source_kind, solosoul_vault::ImportSourceKind::Manual);
    assert!(start.plan.get("cliObjectWriteCount").is_none());
    let accepted = f
        .service
        .with_session(&session, |vault| {
            vault.commit_import_batch_with_operation(
                session.account_id(),
                &prepared.view.revision,
                &prepared.batch,
                &start,
            )
        })
        .unwrap();
    assert!(!accepted.already_committed);
    assert_eq!(accepted.database_commit.object_write_count, 2);
    assert_eq!(accepted.database_commit.object_ids.len(), 1);
    assert!(accepted.database_commit.object_ids.contains(OWNER));
    let original_host_count = accepted.database_commit.object_ids.len();
    assert_eq!(
        accepted.operation.start.source_kind,
        solosoul_vault::ImportSourceKind::Manual
    );
    assert_eq!(
        accepted.operation.phase,
        ImportOperationPhase::RecordsCommitted
    );
    let actual_object_count: i64 = f
        .db()
        .query_row("SELECT COUNT(*) FROM objects", [], |row| row.get(0))
        .unwrap();
    assert_eq!(actual_object_count, initial_object_count + 1);
    assert_eq!(
        f.service
            .with_session(&session, |vault| vault.load_object(OWNER))
            .unwrap()
            .unwrap()
            .name,
        "last"
    );

    // 跨来源消费者使用原 journal 的唯一 ID 计数，不能回退为两次 SQL 写入。
    let before_resume = state(&f.db());
    let resumed =
        resume_vault_import(&f.service, &session, &id, None, None, f.dir.path(), &key).unwrap();
    assert!(resumed.complete);
    assert!(resumed.error_code.is_none());
    assert_eq!(resumed.operation_id, id);
    assert_eq!(resumed.object_write_count, original_host_count);
    assert_eq!(resumed.object_write_count, 1);
    assert_eq!(resumed.attachment_count, 0);
    assert!(!resumed.preferences_imported);
    let after_resume = state(&f.db());
    for table in ["objects", "user_templates", "object_snapshots", "sync_hlc"] {
        assert_eq!(after_resume.get(table), before_resume.get(table));
    }
    let completed = f.operation(&id);
    assert_eq!(completed.phase, ImportOperationPhase::Complete);
    assert_eq!(
        completed.start.source_kind,
        solosoul_vault::ImportSourceKind::Manual
    );
    assert!(completed.start.plan.get("cliObjectWriteCount").is_none());
    assert_eq!(completed.database_commit.object_write_count, 2);
    assert_eq!(
        completed.database_commit.object_ids.len(),
        original_host_count
    );

    // 已 Complete 的同 ID 在重开、源包删除后保持结果；原 journal/业务表都不再改动。
    drop(opened);
    drop(owned);
    std::fs::remove_file(&f.package).unwrap();
    let reopened = f.reopen();
    let reopened_session = reopened.capture_session(&f.account).unwrap();
    let reopened_key = reopened
        .attachment_key_for_session(&reopened_session)
        .unwrap();
    let before_replay = state(&f.db());
    let replay = resume_vault_import(
        &reopened,
        &reopened_session,
        &id,
        None,
        None,
        f.dir.path(),
        &reopened_key,
    )
    .unwrap();
    assert!(replay.complete);
    assert!(replay.error_code.is_none());
    assert_eq!(replay.operation_id, id);
    assert_eq!(replay.object_write_count, original_host_count);
    assert_eq!(replay.object_write_count, 1);
    assert_eq!(state(&f.db()), before_replay);
}
