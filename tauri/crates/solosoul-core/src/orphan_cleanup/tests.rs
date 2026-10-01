//! RF-903：真实 Service/SQLite/SOLC/文件操作回归。仅由 Root 的 Cargo 队列运行。

#[cfg(windows)]
use super::cleanup_with_observers;
use super::{cleanup_in_window, cleanup_orphan_attachments_for_session, OrphanCleanupReport};
use crate::attachment_crypto::encrypt_file_stream;
use crate::import_activity::{begin_owned_root_activity, begin_owned_root_maintenance};
use crate::objects::{create_page, save_attachments, AttachmentMeta};
use crate::vault_service::{VaultService, VaultSession};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use zeroize::Zeroizing;

const PASSWORD: &str = "rf903-real-fixture-password";

struct Fixture {
    service: VaultService,
    session: VaultSession,
    key: Zeroizing<[u8; 32]>,
    account: String,
    root: PathBuf,
    // 最后释放临时目录；Service/Session/root owner 先 Drop，避免 Windows 锁句柄干扰收尾。
    temporary: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let temporary = TempDir::new().unwrap();
        let service =
            VaultService::try_with_base_path(temporary.path().join("native-root")).unwrap();
        let account = service.create_account("RF903", PASSWORD, None).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let session = service.capture_session(&account).unwrap();
        let key = service.attachment_key_for_session(&session).unwrap();
        let root = service.base_path().canonicalize().unwrap();
        Self {
            service,
            session,
            key,
            account,
            root,
            temporary,
        }
    }

    fn directory(&self, storage: &str, attachment: &str) -> PathBuf {
        let path = self.root.join("attachments").join(storage).join(attachment);
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn encrypted_with_key(&self, path: &Path, key: &[u8; 32], plaintext: &[u8]) -> Vec<u8> {
        let mut source = tempfile::NamedTempFile::new_in(self.temporary.path()).unwrap();
        source.write_all(plaintext).unwrap();
        source.flush().unwrap();
        encrypt_file_stream(key, source.path(), path).unwrap();
        fs::read(path).unwrap()
    }

    fn encrypted(&self, path: &Path, plaintext: &[u8]) -> Vec<u8> {
        self.encrypted_with_key(path, &self.key, plaintext)
    }

    fn foreign_account_key(&mut self) -> Zeroizing<[u8; 32]> {
        let foreign_account = self
            .service
            .create_account("Other account", PASSWORD, None)
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let foreign = self.service.capture_session(&foreign_account).unwrap();
        let key = self.service.attachment_key_for_session(&foreign).unwrap();
        self.service.unlock(&self.account, PASSWORD).unwrap();
        self.session = self.service.capture_session(&self.account).unwrap();
        assert_ne!(&*key, &*self.key);
        key
    }

    fn page(&self, name: &str) -> String {
        self.service
            .with_session(&self.session, |vault| {
                create_page(vault, &self.account, name).map(|record| record.id)
            })
            .unwrap()
    }

    fn reference(&self, owner: &str, storage: &str, attachment: &str, path: &Path) {
        self.service
            .with_session(&self.session, |vault| {
                let mut record = vault.load_object(owner)?.unwrap();
                save_attachments(
                    &mut record.properties,
                    &[AttachmentMeta {
                        id: attachment.to_owned(),
                        object_id: storage.to_owned(),
                        file_name: "sample.txt".to_owned(),
                        mime_type: "text/plain".to_owned(),
                        size_bytes: 23,
                        created_at: "2026-10-01T00:00:00Z".to_owned(),
                        deleted_at: None,
                        src_path: None,
                        vault_path: Some(path.to_string_lossy().into_owned()),
                        description: None,
                        tags: vec![],
                    }],
                );
                vault.save_object(&record)
            })
            .unwrap();
    }

    fn cleanup(&self) -> OrphanCleanupReport {
        cleanup_orphan_attachments_for_session(&self.service, &self.session).unwrap()
    }
}

fn no_deletions(report: &OrphanCleanupReport) {
    assert_eq!(report.removed, 0, "no directory may be reported as removed");
    assert_eq!(
        report.files_removed, 0,
        "no regular file was physically removed"
    );
    assert_eq!(
        report.freed_bytes, 0,
        "no undeleted file bytes may be counted as freed"
    );
}

fn complete_removal(report: &OrphanCleanupReport, directories: usize, files: usize, bytes: u64) {
    assert_eq!(report.removed, directories);
    assert_eq!(report.files_removed, files);
    assert_eq!(report.freed_bytes, bytes);
    assert_eq!(report.failed, 0);
    assert!(report.failure_code.is_none());
}

fn legacy_v1(key: &[u8; 32], plaintext: &[u8]) -> Vec<u8> {
    // 实际 legacy v1：base_nonce(12) + chunk_count(8) + AES-GCM ciphertext/tag，无 v2 AAD。
    let encrypted = solosoul_crypto::cipher::encrypt(key, plaintext, None).unwrap();
    let mut bytes = encrypted.nonce.to_vec();
    bytes.extend_from_slice(&1u64.to_be_bytes());
    bytes.extend_from_slice(&encrypted.ciphertext);
    bytes
}

#[test]
fn rf903_public_real_nonempty_unmarked_orphan_removed_with_exact_ciphertext_bytes() {
    let fixture = Fixture::new();
    let directory = fixture.directory("orphan_object", "orphan_attachment");
    let bytes = fixture.encrypted(&directory.join("sample.txt"), &vec![0x37; 2 * 65536 + 31]);
    assert!(!directory
        .join(crate::export_import::operation::IMPORT_OWNER_MARKER)
        .exists());
    assert!(!directory.with_extension("import-owner").exists());
    let report = fixture.cleanup();
    complete_removal(&report, 1, 1, bytes.len() as u64);
    assert_eq!(report.preserved, 0);
    assert!(!directory.exists());
}

#[test]
fn rf903_public_multifile_fully_authenticated_directory_removed_and_counted_as_complete() {
    let fixture = Fixture::new();
    let directory = fixture.directory("orphan_object", "orphan_attachment");
    let first = fixture.encrypted(&directory.join("sample.txt"), b"one");
    let second = fixture.encrypted(&directory.join("second.bin"), &vec![0x53; 65536 + 41]);
    let report = fixture.cleanup();
    complete_removal(&report, 1, 2, (first.len() + second.len()) as u64);
    assert_eq!(report.preserved, 0);
    assert!(!directory.exists());
}

#[test]
fn rf903_public_mixed_plain_foreign_corrupt_zero_legacy_unsupported_directories_retained_whole() {
    let mut fixture = Fixture::new();
    let foreign_key = fixture.foreign_account_key();
    let mut expected = Vec::new();
    for (index, kind) in [
        "plain",
        "foreign",
        "corrupt",
        "zero",
        "legacy",
        "unsupported",
    ]
    .iter()
    .enumerate()
    {
        let directory = fixture.directory("mixed_object", &format!("att_{index}"));
        let valid_path = directory.join("sample.txt");
        let valid = fixture.encrypted(
            &valid_path,
            b"authenticated but directory contains another kind",
        );
        let other_path = directory.join("other.bin");
        let other = match *kind {
            "plain" => {
                fs::write(&other_path, b"legacy plaintext").unwrap();
                fs::read(&other_path).unwrap()
            }
            "foreign" => {
                fixture.encrypted_with_key(&other_path, &foreign_key, b"other actual account")
            }
            "corrupt" => {
                let mut bytes = fixture.encrypted(&other_path, &vec![0x41; 65536 + 7]);
                *bytes.last_mut().unwrap() ^= 1;
                fs::write(&other_path, &bytes).unwrap();
                bytes
            }
            "zero" => fixture.encrypted(&other_path, b""),
            "legacy" => {
                let bytes = legacy_v1(&fixture.key, b"real legacy v1");
                fs::write(&other_path, &bytes).unwrap();
                bytes
            }
            "unsupported" => {
                let mut bytes = fixture.encrypted(&other_path, b"unsupported v2 successor");
                bytes[4] = 3;
                fs::write(&other_path, &bytes).unwrap();
                bytes
            }
            _ => unreachable!(),
        };
        expected.push((directory, valid_path, valid, other_path, other));
    }
    let removable = fixture.directory("independent_owned", "att_owned");
    let removable_bytes =
        fixture.encrypted(&removable.join("sample.txt"), b"independent owned orphan");
    let report = fixture.cleanup();
    complete_removal(&report, 1, 1, removable_bytes.len() as u64);
    assert_eq!(report.preserved, expected.len());
    for (directory, valid_path, valid, other_path, other) in expected {
        assert!(directory.is_dir());
        assert_eq!(fs::read(valid_path).unwrap(), valid);
        assert_eq!(fs::read(other_path).unwrap(), other);
    }
    assert!(!removable.exists());
}

#[test]
fn rf903_public_empty_directory_not_claimed_by_current_key() {
    let fixture = Fixture::new();
    let directory = fixture.directory("empty_object", "empty_attachment");
    let report = fixture.cleanup();
    no_deletions(&report);
    assert_eq!(report.preserved, 1);
    assert_eq!(report.failed, 0);
    assert!(report.failure_code.is_none());
    assert!(directory.is_dir());
}

#[test]
fn rf903_public_fake_solc_header_does_not_authorize_deletion() {
    let fixture = Fixture::new();
    let directory = fixture.directory("fake_object", "fake_attachment");
    let mut fake = vec![0u8; 25 + 16 + 7];
    fake[..4].copy_from_slice(b"SOLC");
    fake[4] = 2;
    fake[17..25].copy_from_slice(&1u64.to_be_bytes());
    let path = directory.join("sample.txt");
    fs::write(&path, &fake).unwrap();
    let report = fixture.cleanup();
    no_deletions(&report);
    assert_eq!(report.preserved, 1);
    assert_eq!(fs::read(path).unwrap(), fake);
}

#[test]
fn rf903_public_appended_ciphertext_tail_is_retained() {
    let fixture = Fixture::new();
    let directory = fixture.directory("tail_object", "tail_attachment");
    let path = directory.join("sample.txt");
    let mut bytes = fixture.encrypted(&path, &vec![0x44; 65536 + 33]);
    bytes.extend_from_slice(b"unverified trailing bytes");
    fs::write(&path, &bytes).unwrap();
    let report = fixture.cleanup();
    no_deletions(&report);
    assert_eq!(report.preserved, 1);
    assert_eq!(fs::read(path).unwrap(), bytes);
}

#[test]
fn rf903_public_truncated_real_multichunk_ciphertext_retained() {
    let fixture = Fixture::new();
    let directory = fixture.directory("short_object", "short_attachment");
    let path = directory.join("sample.txt");
    let mut bytes = fixture.encrypted(&path, &vec![0x44; 65536 + 33]);
    bytes.truncate(bytes.len() - 9);
    fs::write(&path, &bytes).unwrap();
    let report = fixture.cleanup();
    no_deletions(&report);
    assert_eq!(report.preserved, 1);
    assert_eq!(fs::read(path).unwrap(), bytes);
}

#[test]
fn rf903_public_zero_chunk_writer_output_never_proves_key_ownership() {
    let fixture = Fixture::new();
    let directory = fixture.directory("zero_object", "zero_attachment");
    let path = directory.join("sample.txt");
    let bytes = fixture.encrypted(&path, b"");
    assert_eq!(bytes.len(), 25);
    assert_eq!(&bytes[17..25], &0u64.to_be_bytes());
    let report = fixture.cleanup();
    no_deletions(&report);
    assert_eq!(report.preserved, 1);
    assert_eq!(fs::read(path).unwrap(), bytes);
}

#[test]
fn rf903_public_real_legacy_v1_remains_readable_and_is_retained() {
    let fixture = Fixture::new();
    let directory = fixture.directory("legacy_object", "legacy_attachment");
    let path = directory.join("sample.txt");
    let plaintext = b"old AEAD stream without header authentication";
    let bytes = legacy_v1(&fixture.key, plaintext);
    assert_eq!(
        solosoul_crypto::cipher::decrypt_chunked_from_bytes(&fixture.key, &bytes)
            .unwrap()
            .as_slice(),
        plaintext
    );
    fs::write(&path, &bytes).unwrap();
    let report = fixture.cleanup();
    no_deletions(&report);
    assert_eq!(report.preserved, 1);
    assert_eq!(fs::read(path).unwrap(), bytes);
}

#[test]
fn rf903_public_foreign_account_same_storage_owner_retained_while_current_orphan_removed() {
    let mut fixture = Fixture::new();
    let foreign_key = fixture.foreign_account_key();
    let foreign_dir = fixture.directory("shared_storage_object", "foreign_attachment");
    let foreign_path = foreign_dir.join("sample.txt");
    let foreign =
        fixture.encrypted_with_key(&foreign_path, &foreign_key, b"actual foreign account key");
    let own_dir = fixture.directory("shared_storage_object", "current_attachment");
    let own = fixture.encrypted(
        &own_dir.join("sample.txt"),
        b"current account actual orphan",
    );
    let report = fixture.cleanup();
    complete_removal(&report, 1, 1, own.len() as u64);
    assert_eq!(report.preserved, 1);
    assert_eq!(fs::read(foreign_path).unwrap(), foreign);
    assert!(foreign_dir.is_dir());
    assert!(!own_dir.exists());
}

#[test]
fn rf903_public_active_object_reference_protects_only_referenced_attachment() {
    let fixture = Fixture::new();
    let owner = fixture.page("Reference owner");
    let referenced = fixture.directory(&owner, "referenced_att");
    let path = referenced.join("sample.txt");
    let bytes = fixture.encrypted(&path, b"active object attachment");
    fixture.reference(&owner, &owner, "referenced_att", &path);
    let orphan = fixture.directory(&owner, "unreferenced_att");
    let orphan_bytes = fixture.encrypted(
        &orphan.join("sample.txt"),
        b"same object unreferenced attachment",
    );
    let report = fixture.cleanup();
    complete_removal(&report, 1, 1, orphan_bytes.len() as u64);
    assert_eq!(report.preserved, 1);
    assert_eq!(fs::read(path).unwrap(), bytes);
    assert!(!orphan.exists());
}

#[test]
fn rf903_public_softdeleted_object_reference_is_still_recoverable_and_preserved() {
    let fixture = Fixture::new();
    let owner = fixture.page("Softdeleted reference owner");
    let directory = fixture.directory(&owner, "retained_att");
    let path = directory.join("sample.txt");
    let bytes = fixture.encrypted(&path, b"softdeleted attachment");
    fixture.reference(&owner, &owner, "retained_att", &path);
    fixture
        .service
        .with_session(&fixture.session, |vault| vault.delete_object(&owner, true))
        .unwrap();
    assert!(
        fixture
            .session
            .vault()
            .load_object(&owner)
            .unwrap()
            .unwrap()
            .is_deleted
    );
    let report = fixture.cleanup();
    no_deletions(&report);
    assert_eq!(report.preserved, 1);
    assert_eq!(fs::read(path).unwrap(), bytes);
}

#[test]
fn rf903_public_absolute_vault_path_alias_protects_physical_directory() {
    let fixture = Fixture::new();
    let owner = fixture.page("Alias owner");
    let directory = fixture.directory("physical_storage", "physical_att");
    let path = directory.join("sample.txt");
    let bytes = fixture.encrypted(&path, b"reference through actual vaultPath alias");
    fixture.reference(&owner, "old_logical_storage", "logical_attachment", &path);
    let report = fixture.cleanup();
    no_deletions(&report);
    assert_eq!(report.preserved, 1);
    assert_eq!(fs::read(path).unwrap(), bytes);
}

#[test]
fn rf903_public_normal_activity_blocks_maintenance_cleanup_without_touching_files() {
    let fixture = Fixture::new();
    let directory = fixture.directory("busy_object", "busy_attachment");
    let path = directory.join("sample.txt");
    let bytes = fixture.encrypted(&path, b"must not delete during activity");
    let activity = begin_owned_root_activity(fixture.service.root_owner()).unwrap();
    assert_eq!(
        cleanup_orphan_attachments_for_session(&fixture.service, &fixture.session)
            .err()
            .unwrap(),
        "IMPORT_OPERATIONS_ACTIVE"
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
    drop(activity);
    complete_removal(&fixture.cleanup(), 1, 1, bytes.len() as u64);
}

#[test]
fn rf903_public_existing_maintenance_blocks_cleanup_and_new_activity() {
    let fixture = Fixture::new();
    let directory = fixture.directory("maintenance_object", "maintenance_attachment");
    let path = directory.join("sample.txt");
    let bytes = fixture.encrypted(&path, b"must not enter existing maintenance");
    let guard = begin_owned_root_maintenance(fixture.service.root_owner()).unwrap();
    assert_eq!(
        begin_owned_root_activity(fixture.service.root_owner())
            .err()
            .unwrap(),
        "IMPORT_DIRECTORY_BUSY"
    );
    assert_eq!(
        cleanup_orphan_attachments_for_session(&fixture.service, &fixture.session)
            .err()
            .unwrap(),
        "IMPORT_OPERATIONS_ACTIVE"
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
    drop(guard);
    complete_removal(&fixture.cleanup(), 1, 1, bytes.len() as u64);
}

#[test]
fn rf903_public_original_session_cannot_revive_after_lock_and_same_account_reunlock() {
    let fixture = Fixture::new();
    let directory = fixture.directory("session_object", "session_attachment");
    let path = directory.join("sample.txt");
    let bytes = fixture.encrypted(&path, b"stale session cannot delete");
    fixture.service.lock();
    fixture.service.unlock(&fixture.account, PASSWORD).unwrap();
    assert_eq!(
        cleanup_orphan_attachments_for_session(&fixture.service, &fixture.session)
            .err()
            .unwrap(),
        "Vault session is no longer current"
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
    let fresh = fixture.service.capture_session(&fixture.account).unwrap();
    let report = cleanup_orphan_attachments_for_session(&fixture.service, &fresh).unwrap();
    complete_removal(&report, 1, 1, bytes.len() as u64);
}

#[test]
fn rf903_cleanup_window_requires_original_root_owner_guard() {
    let fixture = Fixture::new();
    let other = Fixture::new();
    let directory = fixture.directory("wrong_owner_object", "wrong_owner_attachment");
    let path = directory.join("sample.txt");
    let bytes = fixture.encrypted(&path, b"foreign guard cannot delete");
    let guard = begin_owned_root_maintenance(other.service.root_owner()).unwrap();
    assert_eq!(
        cleanup_in_window(&fixture.service, &fixture.session, &guard, |_| {})
            .err()
            .unwrap(),
        "attachment_cleanup_owner_mismatch"
    );
    assert_eq!(fs::read(path).unwrap(), bytes);
}

#[test]
fn rf903_real_cleanup_window_rejects_both_new_activity_and_second_maintenance() {
    let fixture = Fixture::new();
    let directory = fixture.directory("window_object", "window_attachment");
    let bytes = fixture.encrypted(&directory.join("sample.txt"), b"actual maintenance window");
    let guard = begin_owned_root_maintenance(fixture.service.root_owner()).unwrap();
    let mut observations = 0;
    let report = cleanup_in_window(&fixture.service, &fixture.session, &guard, |_| {
        observations += 1;
        assert_eq!(
            begin_owned_root_activity(fixture.service.root_owner())
                .err()
                .unwrap(),
            "IMPORT_DIRECTORY_BUSY"
        );
        assert_eq!(
            begin_owned_root_maintenance(fixture.service.root_owner())
                .err()
                .unwrap(),
            "IMPORT_OPERATIONS_ACTIVE"
        );
    })
    .unwrap();
    assert_eq!(observations, 1);
    complete_removal(&report, 1, 1, bytes.len() as u64);
    assert!(!directory.exists());
}

#[test]
fn rf903_real_lock_after_authentication_stops_round_before_deletion() {
    let fixture = Fixture::new();
    let mut expected = Vec::new();
    for id in ["att_a", "att_b"] {
        let directory = fixture.directory("stale_window_object", id);
        let path = directory.join("sample.txt");
        let bytes = fixture.encrypted(&path, b"original session bytes");
        expected.push((path, bytes));
    }
    let guard = begin_owned_root_maintenance(fixture.service.root_owner()).unwrap();
    let mut observations = 0;
    let report = cleanup_in_window(&fixture.service, &fixture.session, &guard, |_| {
        observations += 1;
        fixture.service.lock();
    })
    .unwrap();
    assert_eq!(observations, 1);
    no_deletions(&report);
    assert_eq!(report.failed, 1);
    assert_eq!(
        report.failure_code.as_deref(),
        Some("attachment_cleanup_session_stale")
    );
    for (path, bytes) in expected {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}

#[test]
fn rf903_real_database_change_after_authentication_stops_round_before_deletion() {
    let fixture = Fixture::new();
    let owner = fixture.page("Changing reference view");
    let directory = fixture.directory("view_change_object", "view_change_attachment");
    let path = directory.join("sample.txt");
    let bytes = fixture.encrypted(&path, b"view changes after complete authentication");
    let guard = begin_owned_root_maintenance(fixture.service.root_owner()).unwrap();
    let mut observations = 0;
    let report = cleanup_in_window(&fixture.service, &fixture.session, &guard, |_| {
        observations += 1;
        fixture
            .service
            .with_session(&fixture.session, |vault| {
                let mut object = vault.load_object(&owner)?.unwrap();
                object.name = "Actual changed object".to_owned();
                vault.save_object(&object)
            })
            .unwrap();
    })
    .unwrap();
    assert_eq!(observations, 1);
    no_deletions(&report);
    assert_eq!(report.failed, 1);
    assert_eq!(
        report.failure_code.as_deref(),
        Some("attachment_cleanup_view_changed")
    );
    assert_eq!(fs::read(path).unwrap(), bytes);
    assert_eq!(
        fixture
            .session
            .vault()
            .load_object(&owner)
            .unwrap()
            .unwrap()
            .name,
        "Actual changed object"
    );
}

#[test]
fn rf903_real_append_after_authentication_fails_candidate_and_preserves_changed_file() {
    let fixture = Fixture::new();
    let directory = fixture.directory("append_window_object", "append_window_attachment");
    let path = directory.join("sample.txt");
    let mut expected = fixture.encrypted(&path, b"original authenticated bytes");
    expected.extend_from_slice(b"after-authentication tail");
    let guard = begin_owned_root_maintenance(fixture.service.root_owner()).unwrap();
    let mut observations = 0;
    let report = cleanup_in_window(&fixture.service, &fixture.session, &guard, |_| {
        observations += 1;
        fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"after-authentication tail")
            .unwrap();
    })
    .unwrap();
    assert_eq!(observations, 1);
    no_deletions(&report);
    assert_eq!(report.failed, 1);
    assert!(report.failure_code.is_none());
    assert!(directory.is_dir());
    assert_eq!(fs::read(path).unwrap(), expected);
}

#[test]
fn rf903_real_path_replacement_after_authentication_fails_and_retains_both_files() {
    let fixture = Fixture::new();
    let directory = fixture.directory("replace_window_object", "replace_window_attachment");
    let path = directory.join("sample.txt");
    let bytes = fixture.encrypted(&path, b"original authenticated file identity");
    let backup = fixture
        .temporary
        .path()
        .join("original-ciphertext-preserved.bin");
    let replacement = b"new unrelated file at identical path";
    let guard = begin_owned_root_maintenance(fixture.service.root_owner()).unwrap();
    let mut observations = 0;
    let report = cleanup_in_window(&fixture.service, &fixture.session, &guard, |_| {
        observations += 1;
        fs::rename(&path, &backup).unwrap();
        fs::write(&path, replacement).unwrap();
    })
    .unwrap();
    assert_eq!(observations, 1);
    no_deletions(&report);
    assert_eq!(report.failed, 1);
    assert!(report.failure_code.is_none());
    assert!(directory.is_dir());
    assert_eq!(fs::read(path).unwrap(), replacement);
    assert_eq!(fs::read(backup).unwrap(), bytes);
}

#[cfg(windows)]
#[test]
fn rf903_windows_occupied_handle_authenticates_but_reports_zero_freed_bytes_until_retry() {
    use std::os::windows::fs::OpenOptionsExt;
    let fixture = Fixture::new();
    let directory = fixture.directory("occupied_object", "occupied_attachment");
    let path = directory.join("sample.txt");
    let bytes = fixture.encrypted(&path, &vec![0x73; 65536 + 43]);
    // 允许真实 scanner 打开/读取，唯独不共享 DELETE，迫使实际 remove_file 失败。
    let occupied = fs::OpenOptions::new()
        .read(true)
        .share_mode(0x0000_0001 | 0x0000_0002)
        .open(&path)
        .unwrap();
    let report = fixture.cleanup();
    no_deletions(&report);
    assert_eq!(report.failed, 1);
    assert!(report.failure_code.is_none());
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert!(directory.is_dir());
    drop(occupied);
    let retry = fixture.cleanup();
    complete_removal(&retry, 1, 1, bytes.len() as u64);
    assert!(!directory.exists());
}

#[cfg(unix)]
#[test]
fn rf903_unix_symlink_object_attachment_and_child_never_follow_external_files() {
    for level in 0..3 {
        let fixture = Fixture::new();
        let outside = TempDir::new().unwrap();
        let external_directory = outside.path().join("external_att");
        fs::create_dir(&external_directory).unwrap();
        let external_file = external_directory.join("sample.txt");
        let external_cipher = fixture.encrypted(
            &external_file,
            b"external same-key bytes are not owned by path",
        );
        let sentinel = external_directory.join("sentinel.txt");
        fs::write(&sentinel, b"never touch external plaintext").unwrap();
        let storage = fixture.root.join("attachments").join("linked_object");
        let directory = storage.join("linked_att");
        let (link, link_target) = match level {
            0 => {
                fs::create_dir_all(storage.parent().unwrap()).unwrap();
                (storage.clone(), outside.path().to_path_buf())
            }
            1 => {
                fs::create_dir_all(&storage).unwrap();
                (directory.clone(), external_directory.clone())
            }
            _ => {
                fs::create_dir_all(&directory).unwrap();
                fixture.encrypted(
                    &directory.join("sample.txt"),
                    b"owned file mixed with symlink",
                );
                (directory.join("linked-child"), external_directory.clone())
            }
        };
        std::os::unix::fs::symlink(link_target, &link).unwrap();
        let report = fixture.cleanup();
        no_deletions(&report);
        assert!(report.preserved + report.failed >= 1);
        assert_eq!(fs::read(&external_file).unwrap(), external_cipher);
        assert_eq!(
            fs::read(&sentinel).unwrap(),
            b"never touch external plaintext"
        );
        assert!(fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
        if level == 2 {
            assert!(directory.join("sample.txt").is_file());
        }
        fs::remove_file(link).unwrap();
    }
}

#[cfg(windows)]
#[test]
fn rf903_windows_junction_object_attachment_and_child_never_follow_external_files() {
    use std::os::windows::fs::MetadataExt;
    use std::os::windows::process::CommandExt;
    struct OwnedJunction(PathBuf);
    impl Drop for OwnedJunction {
        fn drop(&mut self) {
            // 只解除本次精确 junction，不递归删除其目标；先于两个 TempDir 的 Drop。
            let _ = fs::remove_dir(&self.0);
        }
    }
    for level in 0..3 {
        let fixture = Fixture::new();
        let outside = TempDir::new().unwrap();
        let external_directory = outside.path().join("external_att");
        fs::create_dir(&external_directory).unwrap();
        let external_file = external_directory.join("sample.txt");
        let external_cipher = fixture.encrypted(&external_file, b"external same-key bytes");
        let sentinel = external_directory.join("sentinel.txt");
        fs::write(&sentinel, b"external plaintext must remain").unwrap();
        let storage = fixture.root.join("attachments").join("linked_object");
        let directory = storage.join("linked_att");
        let (link, target) = match level {
            0 => {
                fs::create_dir_all(storage.parent().unwrap()).unwrap();
                (storage.clone(), outside.path().to_path_buf())
            }
            1 => {
                fs::create_dir_all(&storage).unwrap();
                (directory.clone(), external_directory.clone())
            }
            _ => {
                fs::create_dir_all(&directory).unwrap();
                fixture.encrypted(
                    &directory.join("sample.txt"),
                    b"owned file mixed with junction",
                );
                (directory.join("linked-child"), external_directory.clone())
            }
        };
        let result = std::process::Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(&link)
            .arg(&target)
            .creation_flags(0x08000000)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "real temporary junction creation must succeed: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let junction = OwnedJunction(link.clone());
        assert_ne!(
            fs::symlink_metadata(&link).unwrap().file_attributes() & 0x0000_0400,
            0
        );
        let report = fixture.cleanup();
        no_deletions(&report);
        assert!(report.preserved + report.failed >= 1);
        assert_eq!(fs::read(&external_file).unwrap(), external_cipher);
        assert_eq!(
            fs::read(&sentinel).unwrap(),
            b"external plaintext must remain"
        );
        assert!(fs::symlink_metadata(&link).is_ok());
        if level == 2 {
            assert!(directory.join("sample.txt").is_file());
        }
        drop(junction);
        assert!(external_directory.is_dir());
        assert_eq!(fs::read(&external_file).unwrap(), external_cipher);
    }
}

// RF903 r2：追加回归；上方 r1 的全部原字节与原断言保持不变。

#[test]
fn rf903_unknown_or_foreign_outer_sidecar_created_after_authentication_fails_without_deletion() {
    for foreign in [false, true] {
        let fixture = Fixture::new();
        let directory = fixture.directory("new_sidecar_object", "new_sidecar_attachment");
        let path = directory.join("sample.txt");
        let bytes = fixture.encrypted(
            &path,
            b"full ciphertext authenticated before sidecar appears",
        );
        let sidecar = directory.with_file_name("new_sidecar_attachment.import-owner");
        assert!(!sidecar.exists());
        let marker = if foreign {
            serde_json::to_vec(&solosoul_vault::ImportOwnedAttachmentMarker {
                account_id: "acc_rf903_foreign_marker".to_owned(),
                operation_id: uuid::Uuid::new_v4().to_string(),
                entry_ordinal: 0,
                owner_id: "new_sidecar_object".to_owned(),
                attachment_id: "new_sidecar_attachment".to_owned(),
                root_binding: fixture.session.vault().import_root_binding().unwrap(),
            })
            .unwrap()
        } else {
            b"unknown outer sidecar format".to_vec()
        };
        let guard = begin_owned_root_maintenance(fixture.service.root_owner()).unwrap();
        let mut observations = 0;
        let report = cleanup_in_window(&fixture.service, &fixture.session, &guard, |_| {
            observations += 1;
            assert!(!sidecar.exists());
            fs::write(&sidecar, &marker).unwrap();
        })
        .unwrap();
        assert_eq!(observations, 1);
        no_deletions(&report);
        assert_eq!(report.failed, 1);
        assert!(report.failure_code.is_none());
        assert!(directory.is_dir());
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(fs::read(&sidecar).unwrap(), marker);
    }
}

#[test]
fn rf903_external_hard_link_with_actual_current_key_is_preserved_without_freeing_bytes() {
    let fixture = Fixture::new();
    let external = fixture
        .temporary
        .path()
        .join("external-current-key-ciphertext.solc");
    assert!(!external.starts_with(&fixture.root));
    let bytes = fixture.encrypted(&external, &vec![0x5d; 65536 + 39]);
    let directory = fixture.directory("hard_link_object", "hard_link_attachment");
    let linked = directory.join("sample.txt");
    fs::hard_link(&external, &linked).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_eq!(
            fs::metadata(&external).unwrap().ino(),
            fs::metadata(&linked).unwrap().ino()
        );
        assert_eq!(
            fs::metadata(&external).unwrap().dev(),
            fs::metadata(&linked).unwrap().dev()
        );
        assert_eq!(fs::metadata(&linked).unwrap().nlink(), 2);
    }
    let report = fixture.cleanup();
    no_deletions(&report);
    assert_eq!(report.preserved, 1);
    assert_eq!(report.failed, 0);
    assert!(report.failure_code.is_none());
    assert!(directory.is_dir());
    assert_eq!(fs::read(&linked).unwrap(), bytes);
    assert_eq!(fs::read(&external).unwrap(), bytes);
}

#[cfg(windows)]
#[test]
fn rf903_windows_partial_physical_removal_counts_only_deleted_first_file_and_retry_remainder() {
    use std::os::windows::fs::OpenOptionsExt;
    let fixture = Fixture::new();
    let directory = fixture.directory("partial_object", "partial_attachment");
    let first_path = directory.join("a.txt");
    let second_path = directory.join("b.txt");
    let first = fixture.encrypted(&first_path, b"first sorted payload physically removed");
    let second = fixture.encrypted(&second_path, &vec![0x6e; 65536 + 47]);
    // 全体 payload 仍可打开并真实 AEAD 认证，只有第二个文件的 DELETE 被 OS 拒绝。
    let occupied = fs::OpenOptions::new()
        .read(true)
        .share_mode(0x0000_0001 | 0x0000_0002)
        .open(&second_path)
        .unwrap();
    let report = fixture.cleanup();
    assert_eq!(
        report.removed, 0,
        "attachment directory was not fully removed"
    );
    assert_eq!(report.failed, 1);
    assert_eq!(
        report.files_removed, 1,
        "a.txt alone was physically removed"
    );
    assert_eq!(report.freed_bytes, first.len() as u64);
    assert!(report.failure_code.is_none());
    assert!(!first_path.exists());
    assert!(directory.is_dir());
    assert_eq!(fs::read(&second_path).unwrap(), second);
    drop(occupied);
    let retry = fixture.cleanup();
    complete_removal(&retry, 1, 1, second.len() as u64);
    assert!(!directory.exists());
    assert_eq!(
        report.freed_bytes + retry.freed_bytes,
        (first.len() + second.len()) as u64
    );
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[test]
fn rf903_actual_scanner_window_denies_independent_process_and_owner_release_allows_recovery() {
    let fixture = Fixture::new();
    let directory = fixture.directory("process_window_object", "process_window_attachment");
    let path = directory.join("sample.txt");
    let bytes = fixture.encrypted(&path, b"real scanner with root owner held");
    let account_config = fixture.root.join(&fixture.account).join("config.json");
    let config_before = fs::read(&account_config).unwrap();
    let business_marker = fixture.root.join("owned.written");
    let child_account = fixture.root.join("acc_child");
    assert!(!business_marker.exists());
    assert!(!child_account.exists());
    let guard = begin_owned_root_maintenance(fixture.service.root_owner()).unwrap();
    let mut observations = 0;
    let report = cleanup_in_window(&fixture.service, &fixture.session, &guard, |_| {
        observations += 1;
        // 复用原 RF905 的真实同 test binary 子进程，不 mock owner，也不新增 ignored helper。
        // 子进程明确尝试 VaultService::try_with_base_path 并只在固定 BUSY 后 exit(23)。
        assert_eq!(
            crate::root_ownership_tests::probe(&fixture.root, "owned_try"),
            23
        );
        assert!(!business_marker.exists());
        assert!(!child_account.exists());
        assert_eq!(fs::read(&account_config).unwrap(), config_before);
        assert_eq!(fs::read(&path).unwrap(), bytes);
    })
    .unwrap();
    assert_eq!(observations, 1);
    complete_removal(&report, 1, 1, bytes.len() as u64);
    assert!(!directory.exists());
    drop(guard);
    let Fixture {
        service,
        session,
        key,
        account: _,
        root,
        temporary,
    } = fixture;
    drop(session);
    drop(service);
    drop(key);
    // 保留同一个真实 TEMP root；只释放原句柄，不能换空目录伪造“恢复成功”。
    assert_eq!(crate::root_ownership_tests::probe(&root, "owned_try"), 0);
    assert_eq!(
        fs::read(&business_marker).unwrap(),
        b"owned business committed"
    );
    assert!(child_account.join("config.json").is_file());
    // child 已真实退出；本进程再成功 acquire 证明其 OS owner handle 已释放。
    let owner = solosoul_vault::root_owner::VaultRootOwner::acquire(&root).unwrap();
    assert!(owner.is_process_locked());
    assert_eq!(owner.root(), root.as_path());
    drop(owner);
    drop(temporary);
}

#[test]
fn rf903_inner_file_named_like_outer_import_sidecar_is_unverified_payload_and_retained() {
    let fixture = Fixture::new();
    let directory = fixture.directory("inner_name_object", "inner_name_attachment");
    let path = directory.join("sample.txt");
    let bytes = fixture.encrypted(
        &path,
        b"owned payload cannot authorize deletion of inner JSON",
    );
    let inner_same_name = directory.join("inner_name_attachment.import-owner");
    let legal_outer_sidecar = directory.with_file_name("inner_name_attachment.import-owner");
    let marker = serde_json::to_vec(&solosoul_vault::ImportOwnedAttachmentMarker {
        account_id: fixture.account.clone(),
        operation_id: uuid::Uuid::new_v4().to_string(),
        entry_ordinal: 0,
        owner_id: "inner_name_object".to_owned(),
        attachment_id: "inner_name_attachment".to_owned(),
        root_binding: fixture.session.vault().import_root_binding().unwrap(),
    })
    .unwrap();
    // 完整且字段正确的 JSON 也不是 SOLC payload；此路径不是合法 outer marker 位置。
    assert!(!legal_outer_sidecar.exists());
    assert!(!directory
        .join(crate::export_import::operation::IMPORT_OWNER_MARKER)
        .exists());
    fs::write(&inner_same_name, &marker).unwrap();
    let report = fixture.cleanup();
    no_deletions(&report);
    assert_eq!(report.preserved, 1);
    assert_eq!(report.failed, 0);
    assert!(report.failure_code.is_none());
    assert!(directory.is_dir());
    assert_eq!(fs::read(path).unwrap(), bytes);
    assert_eq!(fs::read(inner_same_name).unwrap(), marker);
}

#[test]
fn rf903_actual_attachments_root_replacement_at_platform_checkpoint_stops_round_and_restores_backup(
) {
    struct RestoreAttachments {
        original: PathBuf,
        backup: PathBuf,
    }
    impl RestoreAttachments {
        fn restore(&self) -> std::io::Result<()> {
            if self.backup.exists() {
                if self.original.exists() {
                    // 仅移除本次新建的 empty 目录；不能递归清除任何意外新文件。
                    fs::remove_dir(&self.original)?;
                }
                fs::rename(&self.backup, &self.original)?;
            }
            Ok(())
        }
    }
    impl Drop for RestoreAttachments {
        fn drop(&mut self) {
            let _ = self.restore();
        }
    }
    let fixture = Fixture::new();
    let directory = fixture.directory("root_change_object", "root_change_attachment");
    let path = directory.join("sample.txt");
    let bytes = fixture.encrypted(&path, b"original directory identity preserved in backup");
    let attachments = fixture.root.join("attachments");
    let backup = fixture.root.join("attachments-renamed-for-rf903-test");
    assert!(!backup.exists());
    let restore = RestoreAttachments {
        original: attachments.clone(),
        backup: backup.clone(),
    };
    let guard = begin_owned_root_maintenance(fixture.service.root_owner()).unwrap();
    let mut observations = 0;
    let replace_root = |_: &Path| {
        observations += 1;
        fs::rename(&attachments, &backup).unwrap();
        fs::create_dir(&attachments).unwrap();
    };
    // Windows 的祖先目录 rename 被后代打开句柄阻止；真实替换在后代打开前执行。
    // Unix 仍保留原来的完整 payload 认证后替换，不能把 Windows 证据描述为 after_auth。
    #[cfg(windows)]
    let report = cleanup_with_observers(
        &fixture.service,
        &fixture.session,
        &guard,
        replace_root,
        |_| panic!("root identity change must stop before any candidate authentication"),
    )
    .unwrap();
    #[cfg(not(windows))]
    let report =
        cleanup_in_window(&fixture.service, &fixture.session, &guard, replace_root).unwrap();
    assert_eq!(observations, 1);
    no_deletions(&report);
    assert_eq!(
        report.failure_code.as_deref(),
        Some("attachment_cleanup_root_changed")
    );
    assert_eq!(fs::read_dir(&attachments).unwrap().count(), 0);
    let backup_payload = backup
        .join("root_change_object")
        .join("root_change_attachment")
        .join("sample.txt");
    assert_eq!(fs::read(&backup_payload).unwrap(), bytes);
    assert!(
        fixture.root.join(".lock").is_file(),
        "the Native root itself was never renamed"
    );
    restore.restore().unwrap();
    assert!(!backup.exists());
    assert_eq!(fs::read(path).unwrap(), bytes);
}

#[cfg(windows)]
#[test]
fn rf903_windows_after_auth_descendant_pins_prevent_real_ancestor_rename() {
    let fixture = Fixture::new();
    let directory = fixture.directory("pinned_root_object", "pinned_root_attachment");
    let path = directory.join("sample.txt");
    let bytes = fixture.encrypted(&path, b"fully authenticated original file stays pinned");
    let attachments = fixture.root.join("attachments");
    let backup = fixture.root.join("attachments-cannot-rename-after-auth");
    assert!(!backup.exists());
    let guard = begin_owned_root_maintenance(fixture.service.root_owner()).unwrap();
    let mut observations = 0;
    let report = cleanup_in_window(&fixture.service, &fixture.session, &guard, |observed| {
        observations += 1;
        assert_eq!(observed, directory.as_path());
        // 实际尝试 rename，验证 Windows 后代 pins 阻止祖先更名；没有发生根替换。
        let error = fs::rename(&attachments, &backup).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(attachments.is_dir());
        assert!(!backup.exists());
        assert_eq!(fs::read(&path).unwrap(), bytes);
    })
    .unwrap();
    assert_eq!(observations, 1);
    assert_eq!(report.removed, 1);
    assert_eq!(report.files_removed, 1);
    assert_eq!(report.freed_bytes, bytes.len() as u64);
    assert_eq!(report.preserved, 0);
    assert_eq!(report.failed, 0);
    assert!(report.failure_code.is_none());
    assert!(!directory.exists());
    assert!(!path.exists());
    assert!(!backup.exists());
    assert!(attachments.is_dir());
    assert!(fixture.root.join(".lock").is_file());
}

#[test]
fn rf903_empty_scan_original_session_lock_does_not_report_success() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.root.join("attachments")).unwrap();
    let guard = begin_owned_root_maintenance(fixture.service.root_owner()).unwrap();
    let report = super::cleanup_with_observers(
        &fixture.service,
        &fixture.session,
        &guard,
        |_| fixture.service.lock(),
        |_| panic!("empty root must not authenticate a candidate"),
    )
    .unwrap();
    no_deletions(&report);
    assert_eq!(
        report.failure_code.as_deref(),
        Some("attachment_cleanup_session_stale")
    );
    assert_eq!(report.failed, 1);
    assert_eq!(
        fs::read_dir(fixture.root.join("attachments"))
            .unwrap()
            .count(),
        0
    );
}
