//! RF022 Recovery 的真实 SQLite/加密 ZIP/Noise 回归候选；尚未执行。
use super::*;
use crate::commands::export_import::{derive_export_key_cfg, ImportResult, ImportStatus};
use rusqlite::types::Value as SqlValue;
use serde_json::json;
use std::io::Write;
use std::path::PathBuf;
use zeroize::Zeroizing;

const MASTER: &str = "rf022-recovery-local-password";
const PACKAGE: &str = "rf022-recovery-random-transport-password";

struct Fixture {
    svc: VaultService,
    db: rusqlite::Connection,
    account: String,
    dir: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let svc = VaultService::with_base_path(dir.path().to_path_buf());
        let account = format!("acc_{}", uuid::Uuid::new_v4().simple());
        // 生产新账户入口的同一 helper，后续失败不得删除它。
        create_recovery_account(
            &svc,
            &account,
            "Recovery fixture",
            MASTER,
            None,
            None,
            &|_, _| {},
        )
        .unwrap();
        let db =
            rusqlite::Connection::open(svc.get_vault_store().unwrap().base_path().join("vault.db"))
                .unwrap();
        Self {
            svc,
            db,
            account,
            dir,
        }
    }
    fn session(&self) -> VaultSession {
        self.svc.capture_session(&self.account).unwrap()
    }
    fn package(&self, corrupt_attachment: bool) -> PathBuf {
        let path = self
            .dir
            .path()
            .join(format!("{}.solosoul", uuid::Uuid::new_v4()));
        let salt = solosoul_crypto::kdf::generate_salt();
        let key =
            derive_export_key_cfg(PACKAGE, &salt, &solosoul_crypto::kdf::KdfConfig::balanced())
                .unwrap();
        let attachment_key =
            solosoul_crypto::hkdf_ext::derive_hkdf_key(&key, &salt, b"solosoul:attachments:v1")
                .unwrap();
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("manifest.json", options).unwrap();
        zip.write_all(
            json!({"version":"2.0","salt_hex":hex::encode(salt),"has_attachments":true})
                .to_string()
                .as_bytes(),
        )
        .unwrap();
        zip.start_file("payload.enc", options).unwrap();
        let bytes = serde_json::to_vec(&json!({"objects":(0..2).map(|n| json!({
            "id":format!("rf022-recovery-{n}"),"name":"Transport note","type_id":"note","section_type":"identity",
            "properties":{"text":"incoming", "__attachments":[{"id":format!("a{n}"),"objectId":format!("rf022-recovery-{n}"),
                "fileName":"sample.txt","mimeType":"text/plain","sizeBytes":999,"createdAt":"2026-09-30T00:00:00Z"}]}
        })).collect::<Vec<_>>()})).unwrap();
        solosoul_crypto::cipher::encrypt_chunked_stream(
            &key,
            bytes.len() as u64,
            &mut std::io::Cursor::new(bytes),
            &mut zip,
        )
        .unwrap();
        for n in 0..2 {
            zip.start_file(format!("attachments/rf022-recovery-{n}/a{n}.enc"), options)
                .unwrap();
            if corrupt_attachment && n == 1 {
                zip.write_all(b"invalid ciphertext").unwrap();
            } else {
                solosoul_crypto::cipher::encrypt_chunked_stream(
                    &attachment_key,
                    7,
                    &mut std::io::Cursor::new(b"fixture"),
                    &mut zip,
                )
                .unwrap();
            }
        }
        zip.finish().unwrap();
        path
    }
    fn run(&self, path: PathBuf, operation_id: &str) -> ImportResult {
        import_downloaded_recovery_for_session(
            &self.svc,
            &self.session(),
            &self.account,
            path,
            Zeroizing::new(PACKAGE.to_owned()),
            operation_id,
            None,
        )
        .unwrap()
    }
    fn raw(&self) -> Vec<Vec<Vec<SqlValue>>> {
        [
            "objects",
            "user_templates",
            "object_snapshots",
            "sync_hlc",
            "import_operations",
            "import_attachment_steps",
        ]
        .into_iter()
        .map(|table| {
            let mut stmt = self
                .db
                .prepare(&format!("SELECT * FROM {table} ORDER BY 1,2"))
                .unwrap();
            let count = stmt.column_count();
            let rows = stmt
                .query_map([], |row| {
                    (0..count)
                        .map(|i| row.get(i))
                        .collect::<rusqlite::Result<Vec<SqlValue>>>()
                })
                .unwrap();
            rows.map(Result::unwrap).collect()
        })
        .collect()
    }
    fn assert_not_committed(&self, result: &ImportResult, id: &str, before: &[Vec<Vec<SqlValue>>]) {
        assert_eq!(result.status, ImportStatus::NotCommitted);
        assert_eq!(result.operation_id.as_deref(), Some(id));
        assert_eq!(
            (
                result.object_count,
                result.template_count,
                result.snapshot_count,
                result.attachment_count,
                result.attachment_files_written
            ),
            (0, 0, 0, 0, 0)
        );
        assert_eq!(self.raw(), before);
        assert!(self.svc.has_account(&self.account));
        assert_eq!(
            self.svc.get_current_account().as_deref(),
            Some(self.account.as_str())
        );
        assert!(self
            .session()
            .vault()
            .load_import_operation(&self.account, id)
            .unwrap()
            .is_none());
    }
}

#[test]
fn rf022_recovery_pre_handoff_failure_keeps_account_with_zero_business_and_no_journal() {
    let _guard = crate::VAULT_TEST_LOCK.lock().unwrap();
    let f = Fixture::new();
    let before = f.raw();
    let id = uuid::Uuid::new_v4().to_string();
    let result = f.run(f.package(true), &id);
    f.assert_not_committed(&result, &id, &before);
    f.svc.lock();
    f.svc.unlock(&f.account, MASTER).unwrap();
    assert!(f.svc.capture_session(&f.account).is_ok());
}

#[test]
fn rf022_recovery_db_failure_keeps_account_and_new_authenticated_fresh_retry_succeeds() {
    let _guard = crate::VAULT_TEST_LOCK.lock().unwrap();
    let f = Fixture::new();
    let before = f.raw();
    f.db.execute_batch("CREATE TRIGGER rf022_recovery_fail BEFORE INSERT ON objects BEGIN SELECT RAISE(ABORT,'recovery db fault'); END;").unwrap();
    let failed_id = uuid::Uuid::new_v4().to_string();
    let result = f.run(f.package(false), &failed_id);
    f.assert_not_committed(&result, &failed_id, &before);
    f.db.execute_batch("DROP TRIGGER rf022_recovery_fail;")
        .unwrap();
    f.svc.lock();
    f.svc.unlock(&f.account, MASTER).unwrap();
    // 新认证包是新的 sourceProof/Fresh；从不把它塞给旧 ID 的 resume。
    let fresh_id = uuid::Uuid::new_v4().to_string();
    let path = f.package(false);
    let host = RecoveryHost::start(
        "127.0.0.1:0",
        path,
        Zeroizing::new(PACKAGE.to_owned()),
        f.account.clone(),
        "Recovery fixture".into(),
    )
    .unwrap();
    let info = host.connection_info();
    let cancel = Arc::new(AtomicBool::new(false));
    let run_cancel = cancel.clone();
    let host_thread = std::thread::spawn(move || host.run(run_cancel));
    let received = recover_from_host(
        &info.bind_addr,
        &info.pin,
        f.dir.path(),
        Some(&info.fingerprint),
        Some(&info.nonce),
        None,
    )
    .unwrap();
    host_thread.join().unwrap().unwrap();
    let fresh = import_downloaded_recovery_for_session(
        &f.svc,
        &f.session(),
        &received.account_id,
        received.downloaded_path,
        Zeroizing::new(received.recovery_password),
        &fresh_id,
        None,
    )
    .unwrap();
    assert_eq!(fresh.status, ImportStatus::Complete);
    assert_eq!(fresh.object_count, 2);
    assert_eq!(fresh.attachment_count, 2);
    assert!(f.svc.has_account(&f.account));
    assert!(f
        .session()
        .vault()
        .load_import_operation(&f.account, &failed_id)
        .unwrap()
        .is_none());
    assert_eq!(fresh.operation_id.as_deref(), Some(fresh_id.as_str()));
}

#[test]
fn rf022_recovery_existing_fresh_preserves_existing_record_and_account_credentials() {
    let _guard = crate::VAULT_TEST_LOCK.lock().unwrap();
    let f = Fixture::new();
    let vault = f.svc.get_vault_store().unwrap();
    let now = chrono::Utc::now().to_rfc3339();
    vault
        .save_object(&solosoul_vault::ObjectRecord {
            id: "rf022-recovery-0".into(),
            account_id: f.account.clone(),
            type_id: "note".into(),
            section_type: "identity".into(),
            name: "Keep local".into(),
            properties: json!({"local":true}),
            created_at: now.clone(),
            updated_at: now,
            version: 1,
            ..Default::default()
        })
        .unwrap();
    let config = f.dir.path().join(&f.account).join("config.json");
    let config_before = std::fs::read(&config).unwrap();
    let before = vault.load_object("rf022-recovery-0").unwrap().unwrap();
    let result = f.run(f.package(false), &uuid::Uuid::new_v4().to_string());
    assert_eq!(result.status, ImportStatus::Complete);
    assert_eq!(result.object_count, 1);
    assert_eq!(result.attachment_count, 1);
    assert_eq!(
        serde_json::to_value(vault.load_object("rf022-recovery-0").unwrap().unwrap()).unwrap(),
        serde_json::to_value(before).unwrap()
    );
    assert_eq!(std::fs::read(config).unwrap(), config_before);
    f.svc.lock();
    assert!(f.svc.unlock(&f.account, MASTER).is_ok());
}

#[test]
fn rf022_recovery_accepted_ready_reopens_and_resumes_without_source_or_transport_password() {
    let _guard = crate::VAULT_TEST_LOCK.lock().unwrap();
    let f = Fixture::new();
    f.db.execute_batch("CREATE TRIGGER rf022_recovery_metadata_fail BEFORE UPDATE ON objects BEGIN SELECT RAISE(ABORT,'metadata fault'); END;").unwrap();
    let path = f.package(false);
    let id = uuid::Uuid::new_v4().to_string();
    let result = f.run(path.clone(), &id);
    assert_eq!(result.status, ImportStatus::Partial);
    assert_eq!(result.object_count, 2);
    assert_eq!(result.attachment_count, 0);
    let stored = f
        .session()
        .vault()
        .load_import_operation(&f.account, &id)
        .unwrap()
        .unwrap();
    let requirements =
        solosoul_core::export_import::operation::import_credential_requirements(&stored).unwrap();
    assert!(!requirements.source_required);
    assert!(!requirements.password_required);
    assert!(stored.start.source_ready.is_some());
    std::fs::remove_file(path).unwrap();
    f.db.execute_batch("DROP TRIGGER rf022_recovery_metadata_fail;")
        .unwrap();
    f.svc.lock();
    let reopened = VaultService::with_base_path(f.dir.path().to_path_buf());
    // 真实 AppState 启动也显式加载账户目录；with_base_path 本身只创建空缓存。
    reopened.load_accounts();
    reopened.unlock(&f.account, MASTER).unwrap();
    let session = reopened.capture_session(&f.account).unwrap();
    let resumed = crate::commands::export_import::resume_import_for_session(
        &reopened, &session, &id, None, None, None,
    )
    .unwrap();
    assert_eq!(resumed.status, ImportStatus::Complete);
    assert_eq!(resumed.attachment_count, 2);
    let again = crate::commands::export_import::resume_import_for_session(
        &reopened, &session, &id, None, None, None,
    )
    .unwrap();
    assert_eq!(again.operation_id, resumed.operation_id);
    assert_eq!(again.attachment_count, resumed.attachment_count);
    assert_eq!(reopened.list_accounts().len(), 1);
}

#[test]
fn rf022_recovery_existing_rejects_foreign_source_account_and_old_session_before_any_write() {
    let _guard = crate::VAULT_TEST_LOCK.lock().unwrap();
    let f = Fixture::new();
    let path = f.package(false);
    let before = f.raw();
    let old = f.session();
    let err = import_downloaded_recovery_for_session(
        &f.svc,
        &old,
        "different-account",
        path.clone(),
        Zeroizing::new(PACKAGE.into()),
        &uuid::Uuid::new_v4().to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err, "RECOVERY_ACCOUNT_MISMATCH");
    assert_eq!(f.raw(), before);
    f.svc.lock();
    f.svc.unlock(&f.account, MASTER).unwrap();
    let err = import_downloaded_recovery_for_session(
        &f.svc,
        &old,
        &f.account,
        path,
        Zeroizing::new(PACKAGE.into()),
        &uuid::Uuid::new_v4().to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err, "Vault session is no longer current");
    assert_eq!(f.raw(), before);
    assert!(f.svc.has_account(&f.account));
}

#[test]
fn rf022_recovery_completion_notification_requires_complete_original_session() {
    let _guard = crate::VAULT_TEST_LOCK.lock().unwrap();
    let f = Fixture::new();
    let session = f.session();
    let notified = std::sync::atomic::AtomicUsize::new(0);
    let partial = ImportResult {
        status: ImportStatus::Partial,
        ..Default::default()
    };
    notify_recovery_complete(&f.svc, &session, &partial, || {
        notified.fetch_add(1, Ordering::SeqCst);
    });
    let complete = ImportResult {
        status: ImportStatus::Complete,
        ..Default::default()
    };
    notify_recovery_complete(&f.svc, &session, &complete, || {
        notified.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(notified.load(Ordering::SeqCst), 1);
    f.svc.lock();
    f.svc.unlock(&f.account, MASTER).unwrap();
    notify_recovery_complete(&f.svc, &session, &complete, || {
        notified.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(notified.load(Ordering::SeqCst), 1);
}

#[test]
fn rf022_recovery_transport_rejects_wrong_pin_nonce_or_fingerprint_without_business() {
    let _guard = crate::VAULT_TEST_LOCK.lock().unwrap();
    let f = Fixture::new();
    let before = f.raw();
    for mode in ["pin", "nonce", "fingerprint"] {
        let host = RecoveryHost::start(
            "127.0.0.1:0",
            f.package(false),
            Zeroizing::new(PACKAGE.into()),
            f.account.clone(),
            "Recovery fixture".into(),
        )
        .unwrap();
        let info = host.connection_info();
        let cancel = Arc::new(AtomicBool::new(false));
        let child_cancel = cancel.clone();
        let thread = std::thread::spawn(move || host.run(child_cancel));
        let pin = if mode == "pin" {
            if info.pin == "000000" {
                "999999"
            } else {
                "000000"
            }
        } else {
            &info.pin
        };
        let nonce = if mode == "nonce" {
            "invalid-nonce"
        } else {
            &info.nonce
        };
        let fingerprint = if mode == "fingerprint" {
            "invalid-fingerprint"
        } else {
            &info.fingerprint
        };
        let result = recover_from_host(
            &info.bind_addr,
            pin,
            f.dir.path(),
            Some(fingerprint),
            Some(nonce),
            None,
        );
        cancel.store(true, Ordering::SeqCst);
        let _ = thread.join().unwrap();
        assert!(result.is_err());
        assert_eq!(f.raw(), before);
    }
}
