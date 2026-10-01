//! RF-015：恢复入口显式 All，实际解密加密包，不启动网络恢复主机。
use super::*;
use crate::commands::export_import::{
    decrypt_zip_entry_streaming, derive_export_key_cfg, read_manifest,
};
use solosoul_core::attachment_crypto::{encrypt_file_stream, read_file_decrypted};
use solosoul_vault::ObjectRecord;
use std::path::{Path, PathBuf};

const ACCOUNT: &str = "acc_rf015_recovery";
const OWNER: &str = "rf015-recovery-owner";
const OTHER: &str = "rf015-recovery-other";
const DELETED_OBJECT: &str = "rf015-recovery-deleted";
const MASTER: &str = "rf015-recovery-master-password";
const EXPORT_PASSWORD: &str = "Rf015RecoveryExport1";
const NOW: &str = "2026-10-01T00:00:00Z";

struct Fixture {
    service: VaultService,
    directory: tempfile::TempDir,
    source_bytes: Vec<(PathBuf, Vec<u8>)>,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let service = VaultService::try_with_base_path(directory.path().join("vault")).unwrap();
        service
            .create_account_with_id(ACCOUNT, "RF015 恢复包", MASTER, None)
            .unwrap();
        service.lock();
        service.unlock(ACCOUNT, MASTER).unwrap();
        Self {
            service,
            directory,
            source_bytes: Vec::new(),
        }
    }

    fn attachment(
        &mut self,
        object: &str,
        attachment: &str,
        content: &[u8],
        encrypted: bool,
        deleted: bool,
    ) -> serde_json::Value {
        let path = self
            .service
            .base_path()
            .join("attachments")
            .join(object)
            .join(attachment)
            .join("payload.bin");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        if encrypted {
            let source = self
                .directory
                .path()
                .join(format!("source-{attachment}.bin"));
            std::fs::write(&source, content).unwrap();
            let session = self.service.capture_session(ACCOUNT).unwrap();
            let key = self.service.attachment_key_for_session(&session).unwrap();
            encrypt_file_stream(&key, &source, &path).unwrap();
            assert!(std::fs::read(&path).unwrap().starts_with(b"SOLC"));
            assert_eq!(read_file_decrypted(&key, &path, 4096).unwrap(), content);
            self.source_bytes.push((source, content.to_vec()));
        } else {
            std::fs::write(&path, content).unwrap();
        }
        self.source_bytes
            .push((path.clone(), std::fs::read(&path).unwrap()));
        serde_json::json!({
            "id": attachment,
            "objectId": object,
            "fileName": "payload.bin",
            "mimeType": "application/octet-stream",
            "sizeBytes": content.len(),
            "createdAt": NOW,
            "deletedAt": if deleted { Some(NOW) } else { None },
            "vaultPath": path.to_string_lossy(),
            "tags": []
        })
    }

    fn save_object(&self, id: &str, attachments: Vec<serde_json::Value>, deleted: bool) {
        let session = self.service.capture_session(ACCOUNT).unwrap();
        self.service
            .with_session(&session, |vault| {
                vault.save_object(&ObjectRecord {
                    id: id.into(),
                    account_id: ACCOUNT.into(),
                    type_id: "note".into(),
                    section_type: "identity".into(),
                    name: format!("恢复附件 {id}"),
                    properties: serde_json::json!({
                        "body": "rf015-private-recovery-body",
                        "__attachments": attachments
                    }),
                    sensitivity_level: "internal".into(),
                    is_deleted: deleted,
                    deleted_at: if deleted { Some(NOW.into()) } else { None },
                    created_at: NOW.into(),
                    updated_at: NOW.into(),
                    version: 1,
                    ..Default::default()
                })
            })
            .unwrap();
    }

    fn assert_sources_unchanged(&self) {
        for (path, bytes) in &self.source_bytes {
            assert_eq!(std::fs::read(path).unwrap(), *bytes, "{}", path.display());
        }
    }
}

fn attachment_entry_names(path: &Path) -> Vec<String> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
    let mut names = Vec::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index).unwrap();
        if entry.name().starts_with("attachments/") {
            names.push(entry.name().to_string());
        }
    }
    names.sort();
    names
}

#[test]
fn rf015_recovery_explicit_all_exports_active_plaintext_and_solc_and_preserves_sources() {
    // 串行许可涵盖实际服务、会话及临时文件的完整生命周期。
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut f = Fixture::new();
    let plain = f.attachment(OWNER, "plain", b"old plaintext attachment", false, false);
    let solc = f.attachment(OWNER, "solc", b"SOLC private attachment", true, false);
    let extra = f.attachment(
        OWNER,
        "extra-live",
        b"extra active attachment",
        false,
        false,
    );
    let deleted = f.attachment(
        OWNER,
        "deleted-att",
        b"deleted attachment stays local",
        false,
        true,
    );
    f.save_object(OWNER, vec![plain, solc, extra, deleted], false);
    let other = f.attachment(OTHER, "other-live", b"another active object", false, false);
    f.save_object(OTHER, vec![other], false);
    let deleted_object = f.attachment(
        DELETED_OBJECT,
        "deleted-object-live",
        b"deleted object stays local",
        false,
        false,
    );
    f.save_object(DELETED_OBJECT, vec![deleted_object], true);

    let package = export_recovery_package(&f.service, ACCOUNT, EXPORT_PASSWORD).unwrap();
    let path = package.to_path_buf();
    assert!(path.starts_with(std::env::temp_dir()));
    assert!(path.exists());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let manifest = read_manifest(path.to_str().unwrap()).unwrap();
    assert!(manifest.has_attachments);
    let salt = hex::decode(&manifest.salt_hex).unwrap();
    let key = derive_export_key_cfg(EXPORT_PASSWORD, &salt, &manifest.kdf_config()).unwrap();
    let attachment_key =
        solosoul_crypto::hkdf_ext::derive_hkdf_key(&key, &salt, b"solosoul:attachments:v1")
            .unwrap();
    let mut payload = zeroize::Zeroizing::new(Vec::new());
    decrypt_zip_entry_streaming(path.to_str().unwrap(), "payload.enc", &key, &mut *payload)
        .unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&payload).unwrap();
    let objects = payload["objects"].as_array().unwrap();
    let mut ids: Vec<_> = objects.iter().map(|o| o["id"].as_str().unwrap()).collect();
    ids.sort();
    assert_eq!(ids, vec![OTHER, OWNER]);
    assert!(objects
        .iter()
        .all(|o| o["properties"]["body"] == "rf015-private-recovery-body"));

    let expected = [
        (
            format!("attachments/{OTHER}/other-live.enc"),
            b"another active object".as_slice(),
        ),
        (
            format!("attachments/{OWNER}/extra-live.enc"),
            b"extra active attachment".as_slice(),
        ),
        (
            format!("attachments/{OWNER}/plain.enc"),
            b"old plaintext attachment".as_slice(),
        ),
        (
            format!("attachments/{OWNER}/solc.enc"),
            b"SOLC private attachment".as_slice(),
        ),
    ];
    let mut expected_names: Vec<_> = expected.iter().map(|(name, _)| name.clone()).collect();
    expected_names.sort();
    // 全部实际 ZIP 条目严格相等：既不遗漏新 All，也不打包软删除附件/对象。
    assert_eq!(attachment_entry_names(&path), expected_names);
    for (name, content) in expected {
        let mut decoded = zeroize::Zeroizing::new(Vec::new());
        decrypt_zip_entry_streaming(
            path.to_str().unwrap(),
            &name,
            &attachment_key,
            &mut *decoded,
        )
        .unwrap();
        assert_eq!(decoded.as_slice(), content, "{name}");
    }
    f.assert_sources_unchanged();
    drop(package);
    assert!(!path.exists());
    f.assert_sources_unchanged();
}

#[test]
fn rf015_recovery_empty_all_exports_empty_payload_without_attachment_entries() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let f = Fixture::new();
    let package = export_recovery_package(&f.service, ACCOUNT, EXPORT_PASSWORD).unwrap();
    let path = package.to_path_buf();
    let manifest = read_manifest(path.to_str().unwrap()).unwrap();
    assert!(!manifest.has_attachments);
    assert!(attachment_entry_names(&path).is_empty());
    let salt = hex::decode(&manifest.salt_hex).unwrap();
    let key = derive_export_key_cfg(EXPORT_PASSWORD, &salt, &manifest.kdf_config()).unwrap();
    let mut payload = zeroize::Zeroizing::new(Vec::new());
    decrypt_zip_entry_streaming(path.to_str().unwrap(), "payload.enc", &key, &mut *payload)
        .unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&payload).unwrap();
    assert!(payload["objects"].as_array().unwrap().is_empty());
    drop(package);
    assert!(!path.exists());
}
