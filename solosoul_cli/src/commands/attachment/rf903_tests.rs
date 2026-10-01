//! RF903：真实 CLI handler、Vault/SQLite、密文与实际 worker，不 mock cleanup。
use super::*;
use solosoul_core::attachment_crypto::{encrypt_file_stream, read_file_decrypted};
use solosoul_core::import_activity::begin_owned_root_activity;
use solosoul_core::{VaultService, VaultSession};
use solosoul_vault::ObjectRecord;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};
use std::time::Duration;
use tempfile::TempDir;
use uuid::Uuid;

const ACCOUNT_A: &str = "acc_rf903_cli_a";
const ACCOUNT_B: &str = "acc_rf903_cli_b";
const STORAGE_OBJECT: &str = "rf903-shared-object";
const CONTENT: &[u8] = b"real private orphan fixture";
const WAIT: Duration = Duration::from_secs(20);

struct Fixture {
    app: App,
    directory: TempDir,
}
impl Fixture {
    fn new(locale: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let service =
            Arc::new(VaultService::try_with_base_path(directory.path().join("vault")).unwrap());
        service
            .create_account_with_id(ACCOUNT_A, "RF903 A", crate::TEST_PASSWORD, None)
            .unwrap();
        service.lock();
        service.unlock(ACCOUNT_A, crate::TEST_PASSWORD).unwrap();
        let mut app = App::new(service).unwrap();
        app.i18n.set_locale(locale);
        app.enter_home(ACCOUNT_A);
        Self { app, directory }
    }
    fn session(&self) -> VaultSession {
        let account = self.app.vault_service.get_current_account().unwrap();
        self.app.vault_service.capture_session(&account).unwrap()
    }
    fn encrypt(&self, object: &str, attachment: &str) -> PathBuf {
        let session = self.session();
        let key = self
            .app
            .vault_service
            .attachment_key_for_session(&session)
            .unwrap();
        let source = self
            .directory
            .path()
            .join(format!("source-{attachment}.bin"));
        std::fs::write(&source, CONTENT).unwrap();
        let destination = self
            .app
            .vault_service
            .base_path()
            .join("attachments")
            .join(object)
            .join(attachment)
            .join("payload.bin");
        std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
        encrypt_file_stream(&key, &source, &destination).unwrap();
        assert_eq!(
            read_file_decrypted(&key, &destination, 1024).unwrap(),
            CONTENT
        );
        destination
    }
    fn save_reference(&self, object: &str, attachment: &str, path: &Path) -> ObjectRecord {
        let session = self.session();
        let record = record(session.account_id(), object, attachment, path);
        self.app
            .vault_service
            .with_session(&session, |vault| vault.save_object(&record))
            .unwrap();
        record
    }
    fn unlock(&mut self, account: &str) {
        self.app.vault_service.lock();
        self.app
            .vault_service
            .unlock(account, crate::TEST_PASSWORD)
            .unwrap();
        self.app.enter_home(account);
    }
    fn cleanup(&mut self) {
        handle(&mut self.app, &["cleanup"]).unwrap();
    }
    fn assert_success_counts(&self, count: usize, bytes: u64, preserved: usize) {
        assert!(
            self.app.error_message.is_none(),
            "{:?}",
            self.app.error_message
        );
        let expected = t!(
            self.app.i18n,
            "cmd-cleanup-result",
            count = count.to_string(),
            bytes = bytes.to_string(),
            preserved = preserved.to_string(),
            failed = "0"
        );
        assert_eq!(self.app.success_message.as_ref().unwrap().0, expected);
        assert!(!expected.contains("cmd-cleanup-result"));
        assert!(!expected.contains("{$"));
    }
}

fn record(account: &str, object: &str, attachment: &str, path: &Path) -> ObjectRecord {
    ObjectRecord {
        id: object.into(),
        account_id: account.into(),
        name: "RF903 actual reference".into(),
        type_id: "note".into(),
        section_type: "identity".into(),
        properties: serde_json::json!({"__attachments": [{
            "id":attachment, "objectId":object, "fileName":"payload.bin",
            "mimeType":"application/octet-stream", "sizeBytes":CONTENT.len(),
            "createdAt":"2026-10-01T00:00:00Z", "vaultPath":path.to_string_lossy()
        }]}),
        created_at: "2026-10-01T00:00:00Z".into(),
        updated_at: "2026-10-01T00:00:00Z".into(),
        ..Default::default()
    }
}

#[test]
fn rf903_cli_cleanup_removes_real_authenticated_orphan_with_exact_bytes_in_both_locales() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    for locale in ["zh-CN", "en-US"] {
        let mut f = Fixture::new(locale);
        let id = Uuid::new_v4().to_string();
        let file = f.encrypt(STORAGE_OBJECT, &id);
        let bytes = std::fs::metadata(&file).unwrap().len();
        assert!(bytes > CONTENT.len() as u64);
        f.cleanup();
        assert!(!file.exists());
        assert!(!file.parent().unwrap().exists());
        f.assert_success_counts(1, bytes, 0);
        let message = &f.app.success_message.as_ref().unwrap().0;
        if locale == "zh-CN" {
            assert!(message.contains("保留") && message.contains("失败"));
        } else {
            assert!(message.contains("preserved") && message.contains("failed"));
        }
    }
}

#[test]
fn rf903_cli_cleanup_preserves_two_accounts_same_object_and_removes_only_current_key_orphan() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut f = Fixture::new("en-US");
    let a_id = Uuid::new_v4().to_string();
    let a_file = f.encrypt(STORAGE_OBJECT, &a_id);
    let a_record = f.save_reference(STORAGE_OBJECT, &a_id, &a_file);
    let a_bytes = std::fs::read(&a_file).unwrap();
    f.app.vault_service.lock();
    f.app
        .vault_service
        .create_account_with_id(ACCOUNT_B, "RF903 B", crate::TEST_PASSWORD, None)
        .unwrap();
    let b_id = Uuid::new_v4().to_string();
    let b_file = f.encrypt(STORAGE_OBJECT, &b_id);
    let b_record = f.save_reference(STORAGE_OBJECT, &b_id, &b_file);
    let b_bytes = std::fs::read(&b_file).unwrap();
    assert_eq!(
        a_file.parent().unwrap().parent(),
        b_file.parent().unwrap().parent()
    );
    f.unlock(ACCOUNT_A);
    let orphan = f.encrypt(STORAGE_OBJECT, &Uuid::new_v4().to_string());
    f.cleanup();
    assert!(
        !orphan.exists(),
        "cleanup must retain historical true-orphan deletion"
    );
    assert!(f.app.error_message.is_none(), "{:?}", f.app.error_message);
    assert!(f.app.success_message.is_some());
    assert_eq!(std::fs::read(&a_file).unwrap(), a_bytes);
    assert_eq!(std::fs::read(&b_file).unwrap(), b_bytes);
    for (account, expected, file) in [(ACCOUNT_A, a_record, a_file), (ACCOUNT_B, b_record, b_file)]
    {
        f.unlock(account);
        let session = f.session();
        assert_eq!(
            serde_json::to_value(
                session
                    .vault()
                    .load_object(STORAGE_OBJECT)
                    .unwrap()
                    .unwrap()
            )
            .unwrap(),
            serde_json::to_value(expected).unwrap()
        );
        let key = f
            .app
            .vault_service
            .attachment_key_for_session(&session)
            .unwrap();
        assert_eq!(read_file_decrypted(&key, &file, 1024).unwrap(), CONTENT);
    }
}

struct ReleaseOnDrop(Option<mpsc::Sender<()>>);
impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        if let Some(sender) = self.0.take() {
            let _ = sender.send(());
        }
    }
}

#[test]
fn rf903_cli_cleanup_is_rejected_at_actual_publication_before_metadata_commit() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut f = Fixture::new("en-US");
    let session = f.session();
    let service = Arc::clone(&f.app.vault_service);
    let activity = begin_owned_root_activity(session.root_owner()).unwrap();
    let key = service.attachment_key_for_session(&session).unwrap();
    let attachment = Uuid::new_v4().to_string();
    let source = f.directory.path().join("actual-worker-source.bin");
    std::fs::write(&source, CONTENT).unwrap();
    let file = service
        .base_path()
        .join("attachments")
        .join(STORAGE_OBJECT)
        .join(&attachment)
        .join("payload.bin");
    let worker_file = file.clone();
    let worker_record = record(ACCOUNT_A, STORAGE_OBJECT, &attachment, &file);
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let mut release = ReleaseOnDrop(Some(release_tx));
    let worker = std::thread::spawn(move || {
        let _activity = activity;
        std::fs::create_dir_all(worker_file.parent().unwrap()).unwrap();
        encrypt_file_stream(&key, &source, &worker_file).unwrap();
        let ciphertext = std::fs::read(&worker_file).unwrap();
        entered_tx.send(ciphertext).unwrap();
        release_rx.recv_timeout(WAIT).unwrap();
        service
            .with_session(&session, |vault| vault.save_object(&worker_record))
            .unwrap();
    });
    let ciphertext = entered_rx.recv_timeout(WAIT).unwrap();
    assert!(f
        .session()
        .vault()
        .load_object(STORAGE_OBJECT)
        .unwrap()
        .is_none());
    f.cleanup();
    assert!(f.app.success_message.is_none());
    assert!(f
        .app
        .error_message
        .as_deref()
        .unwrap()
        .contains("IMPORT_OPERATIONS_ACTIVE"));
    assert_eq!(std::fs::read(&file).unwrap(), ciphertext);
    assert!(f
        .session()
        .vault()
        .load_object(STORAGE_OBJECT)
        .unwrap()
        .is_none());
    release.0.take().unwrap().send(()).unwrap();
    worker.join().unwrap();
    assert!(f
        .session()
        .vault()
        .load_object(STORAGE_OBJECT)
        .unwrap()
        .is_some());
    f.cleanup();
    assert!(f.app.error_message.is_none(), "{:?}", f.app.error_message);
    assert!(f.app.success_message.is_some());
    assert_eq!(std::fs::read(&file).unwrap(), ciphertext);
}

#[cfg(windows)]
#[test]
fn rf903_cli_pending_explicit_delete_blocks_scan_then_retries_without_maintenance_self_busy() {
    use std::os::windows::fs::OpenOptionsExt;
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut f = Fixture::new("zh-CN");
    let attachment = Uuid::new_v4().to_string();
    let file = f.encrypt(STORAGE_OBJECT, &attachment);
    f.save_reference(STORAGE_OBJECT, &attachment, &file);
    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(3)
        .open(&file)
        .unwrap();
    let session = f.session();
    f.app
        .vault_service
        .with_session(&session, |vault| {
            vault.queue_attachment_deletions(ACCOUNT_A, STORAGE_OBJECT, &[attachment])
        })
        .unwrap();
    let orphan = f.encrypt(STORAGE_OBJECT, &Uuid::new_v4().to_string());
    let orphan_bytes = std::fs::read(&orphan).unwrap();
    let freed = orphan_bytes.len() as u64;
    f.cleanup();
    assert!(file.exists());
    assert_eq!(std::fs::read(&orphan).unwrap(), orphan_bytes);
    assert!(f.app.success_message.is_none());
    assert!(f.app.error_message.as_deref().unwrap().contains("待重试"));
    assert_eq!(
        f.session()
            .vault()
            .list_attachment_cleanup_intents(ACCOUNT_A)
            .unwrap()
            .len(),
        1
    );
    drop(held);
    f.cleanup();
    assert!(!file.exists());
    assert!(!orphan.exists());
    assert!(f
        .session()
        .vault()
        .list_attachment_cleanup_intents(ACCOUNT_A)
        .unwrap()
        .is_empty());
    f.assert_success_counts(1, freed, 0);
}

#[cfg(windows)]
#[test]
fn rf903_cli_busy_file_is_not_freed_or_reported_complete_and_real_retry_succeeds() {
    use std::os::windows::fs::OpenOptionsExt;
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    for locale in ["zh-CN", "en-US"] {
        let mut f = Fixture::new(locale);
        let file = f.encrypt(STORAGE_OBJECT, &Uuid::new_v4().to_string());
        let before = std::fs::read(&file).unwrap();
        let held = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(3)
            .open(&file)
            .unwrap();
        f.cleanup();
        assert_eq!(std::fs::read(&file).unwrap(), before);
        assert!(f.app.success_message.is_none());
        let expected_result = t!(
            f.app.i18n,
            "cmd-cleanup-result",
            count = "0",
            bytes = "0",
            preserved = "0",
            failed = "1"
        );
        let expected = t!(
            f.app.i18n,
            "cmd-cleanup-incomplete",
            result = expected_result
        );
        assert_eq!(f.app.error_message.as_deref(), Some(expected.as_str()));
        assert!(!expected.contains("cmd-cleanup-incomplete"));
        drop(held);
        f.cleanup();
        assert!(!file.exists());
        f.assert_success_counts(1, before.len() as u64, 0);
    }
}

#[cfg(windows)]
struct Junction(PathBuf);
#[cfg(windows)]
impl Drop for Junction {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir(&self.0);
    }
}

#[cfg(windows)]
#[test]
fn rf903_cli_junction_ancestors_and_descendants_do_not_delete_external_authenticated_bytes() {
    use std::os::windows::process::CommandExt;
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    for level in 0..4 {
        let mut f = Fixture::new("en-US");
        let outside = tempfile::tempdir().unwrap();
        let source = f.directory.path().join("junction-source.bin");
        std::fs::write(&source, CONTENT).unwrap();
        let external = outside.path().join("external.bin");
        let key = f
            .app
            .vault_service
            .attachment_key_for_session(&f.session())
            .unwrap();
        encrypt_file_stream(&key, &source, &external).unwrap();
        let outside_bytes = std::fs::read(&external).unwrap();
        let root = f.app.vault_service.base_path().join("attachments");
        let object = root.join(STORAGE_OBJECT);
        let attachment = object.join(Uuid::new_v4().to_string());
        let link = match level {
            0 => root,
            1 => {
                std::fs::create_dir_all(&root).unwrap();
                object
            }
            2 => {
                std::fs::create_dir_all(&object).unwrap();
                attachment
            }
            _ => {
                std::fs::create_dir_all(&attachment).unwrap();
                attachment.join("descendant")
            }
        };
        let junction = Junction(link.clone());
        let result = std::process::Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(&link)
            .arg(outside.path())
            .creation_flags(0x08000000)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "real temporary junction must be created"
        );
        f.cleanup();
        assert!(link.exists());
        assert_eq!(std::fs::read(&external).unwrap(), outside_bytes);
        assert_eq!(read_file_decrypted(&key, &external, 1024).unwrap(), CONTENT);
        drop(junction);
    }
}

#[test]
fn rf903_cli_locked_cleanup_keeps_bytes_and_does_not_publish_success() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut f = Fixture::new("en-US");
    let file = f.encrypt(STORAGE_OBJECT, &Uuid::new_v4().to_string());
    let before = std::fs::read(&file).unwrap();
    f.app.vault_service.lock();
    assert!(handle(&mut f.app, &["cleanup"]).is_err());
    assert!(f.app.success_message.is_none());
    assert!(f.app.error_message.is_some());
    assert_eq!(std::fs::read(&file).unwrap(), before);
}

#[test]
fn rf903_cli_locked_cleanup_clears_previous_real_cleanup_success_and_keeps_bytes() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    for locale in ["zh-CN", "en-US"] {
        let mut f = Fixture::new(locale);
        let first = f.encrypt(STORAGE_OBJECT, &Uuid::new_v4().to_string());
        let first_bytes = std::fs::metadata(&first).unwrap().len();
        f.cleanup();
        f.assert_success_counts(1, first_bytes, 0);
        assert!(!first.exists());

        let next = f.encrypt(STORAGE_OBJECT, &Uuid::new_v4().to_string());
        let before = std::fs::read(&next).unwrap();
        assert!(
            f.app.success_message.is_some(),
            "the previous real cleanup success must still be present"
        );
        f.app.vault_service.lock();
        assert!(!f.app.vault_service.is_unlocked());

        assert!(handle(&mut f.app, &["cleanup"]).is_err());
        assert!(
            f.app.success_message.is_none(),
            "locked cleanup must clear its previous completion message before returning"
        );
        let expected = t!(f.app.i18n, "cmd-need-unlock");
        assert_eq!(f.app.error_message.as_deref(), Some(expected.as_str()));
        assert_eq!(std::fs::read(&next).unwrap(), before);
        assert!(next.parent().unwrap().is_dir());
    }
}
