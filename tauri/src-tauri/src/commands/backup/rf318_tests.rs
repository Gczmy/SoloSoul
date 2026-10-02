//! 实际坏包、路径 IO 和 SQLite 拒写；仅临时合成 Vault。
use super::*;
use crate::commands::error::BackendError;
use serde_json::{json, Value};
use solosoul_vault::{Profile, VaultConfig, VaultStore};

struct Fixture {
    vault: VaultStore,
    dir: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let vault = VaultStore::open(
            VaultConfig::new("rf318-account", dir.path().to_path_buf()).with_data_key([0x31; 32]),
        )
        .unwrap();
        Self { vault, dir }
    }
    fn seed(&self) -> Vec<Profile> {
        (0..2)
            .map(|n| {
                let p = Profile::new_with_id(&format!("rf318-{n}"), "old", b"old".to_vec());
                self.vault.save_profile(&p).unwrap();
                self.vault.load_profile(&p.id).unwrap().unwrap()
            })
            .collect()
    }
}
fn wire(error: TransferFailure) -> Value {
    let debug = format!("{error:?}");
    assert!(!debug.contains("RF318_PRIVATE"));
    let packet = serde_json::to_value(BackendError::from(error)).unwrap();
    let text = packet.to_string();
    assert!(!text.contains("RF318_PRIVATE") && !text.contains("vault.db"));
    packet
}
#[test]
fn rf318_backup_lookup_and_directory_io_have_distinct_safe_codes() {
    let f = Fixture::new();
    assert!(list_backups(&backups_dir(f.dir.path())).unwrap().is_empty());
    let missing = wire(find_backup(f.dir.path(), "RF318_PRIVATE_ID").unwrap_err());
    assert_eq!(missing["code"], "BACKUP_NOT_FOUND");
    fs::write(backups_dir(f.dir.path()), b"RF318_PRIVATE_BLOCKER").unwrap();
    let io = wire(list_backups(&backups_dir(f.dir.path())).unwrap_err());
    assert_eq!(io["code"], "BACKUP_READ_FAILED");
    assert_eq!(io["safeDetails"]["stage"], "read");
    assert_eq!(io["retryable"], true);
    assert_eq!(
        wire(find_backup(f.dir.path(), "RF318_PRIVATE_ID").unwrap_err())["code"],
        "BACKUP_READ_FAILED"
    );
}
#[test]
fn rf318_backup_invalid_package_and_version_never_write_profiles() {
    let f = Fixture::new();
    let before = f.seed();
    for (bytes, code) in [
        (b"RF318_PRIVATE_BAD_JSON".to_vec(), "BACKUP_INVALID_PACKAGE"),
        (
            json!({"version":"99.0","created_at":"2026-10-02T00:00:00Z","profile_count":0,"profiles":[]})
                .to_string()
                .into_bytes(),
            "BACKUP_UNSUPPORTED_VERSION",
        ),
    ] {
        let packet =
            wire(restore_profile_backup_safe(&f.vault, &bytes, chrono::Utc::now()).unwrap_err());
        assert_eq!(packet["code"], code);
        assert_eq!(packet["safeDetails"]["stage"], "validate");
        assert!(packet["safeDetails"].get("completedCount").is_none());
        assert_eq!(f.vault.list_profiles().unwrap().len(), before.len());
        for p in &before {
            assert_eq!(f.vault.load_profile(&p.id).unwrap().unwrap().data, p.data);
        }
    }
}
#[test]
fn rf318_backup_restore_database_failure_reports_exact_written_prefix() {
    for rejected in [0, 1] {
        let f = Fixture::new();
        let before = f.seed();
        let mut incoming = before.clone();
        for p in &mut incoming {
            p.data = b"new".to_vec();
            p.name = "new".into();
        }
        let bytes = encode_profile_backup(
            &incoming,
            chrono::Utc::now(),
            ProfilePayloadEncoding::Base64,
        )
        .unwrap();
        let db = rusqlite::Connection::open(f.vault.base_path().join("vault.db")).unwrap();
        db.execute_batch(&format!(
            "CREATE TRIGGER rf318_reject BEFORE INSERT ON profiles
            WHEN NEW.id='rf318-{rejected}' BEGIN SELECT RAISE(ABORT,'RF318_PRIVATE_SQL'); END;"
        ))
        .unwrap();
        let packet =
            wire(restore_profile_backup_safe(&f.vault, &bytes, chrono::Utc::now()).unwrap_err());
        assert_eq!(
            packet["code"],
            if rejected == 0 {
                "BACKUP_RESTORE_FAILED"
            } else {
                "BACKUP_RESTORE_PARTIAL"
            }
        );
        assert_eq!(
            packet["safeDetails"],
            json!({"stage":"write","completedCount":rejected})
        );
        assert_eq!(packet["retryable"], false);
        for (n, p) in before.iter().enumerate() {
            let actual = f.vault.load_profile(&p.id).unwrap().unwrap();
            assert_eq!(
                actual.data,
                if n < rejected {
                    b"new".to_vec()
                } else {
                    p.data.clone()
                }
            );
        }
    }
}
#[test]
fn rf318_backup_collection_or_write_failure_preserves_existing_output() {
    let f = Fixture::new();
    let before = f.seed();
    let summaries = f.vault.list_profiles().unwrap();
    let now = chrono::Utc::now();
    create_profile_backup_safe(&f.vault, f.dir.path(), "same", &summaries, now).unwrap();
    let output = fs::read_dir(backups_dir(f.dir.path()))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let bytes = fs::read(&output).unwrap();
    f.vault.delete_profile(&before[1].id).unwrap();
    let packet = wire(
        create_profile_backup_safe(&f.vault, f.dir.path(), "same", &summaries, now).unwrap_err(),
    );
    assert_eq!(packet["code"], "BACKUP_READ_FAILED");
    assert_eq!(fs::read(&output).unwrap(), bytes);
    let blocked = f.dir.path().join("RF318_PRIVATE_BLOCKED");
    fs::write(&blocked, b"blocker").unwrap();
    let packet = wire(create_profile_backup_safe(&f.vault, &blocked, "new", &[], now).unwrap_err());
    assert_eq!(packet["code"], "BACKUP_WRITE_FAILED");
    assert_eq!(packet["safeDetails"]["stage"], "write");
    assert_eq!(fs::read(blocked).unwrap(), b"blocker");
}
