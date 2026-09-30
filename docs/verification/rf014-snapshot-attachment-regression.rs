//! RF-014：只在合成 TempDir 验证真实云快照入口，不创建连接器或上传。
use super::*;
use crate::commands::export_import::{
    execute_export_for_session, import_execute_for_session, tests::rf020::Fixture, ExportRequest,
    ExportScope, ImportStrategy,
};
use serde_json::json;
use solosoul_vault::ObjectRecord;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use zeroize::Zeroizing;
use zip::ZipArchive;

const OBJECT_ID: &str = "rf014-synthetic-object";
const EXPORT_PASSWORD: &str = "export-password";

fn attachment_spec(encrypted: bool) -> (&'static str, &'static str, &'static [u8]) {
    if encrypted {
        (
            "rf014-solc",
            "encrypted.txt",
            b"RF014 synthetic SOLC attachment content",
        )
    } else {
        (
            "rf014-legacy",
            "legacy.txt",
            b"RF014 synthetic legacy plaintext content",
        )
    }
}

fn fixture(encrypted: bool) -> Fixture {
    let f = Fixture::new();
    let (id, file_name, contents) = attachment_spec(encrypted);
    let svc = f.service.read().unwrap();
    let dir = svc.base_path().join("attachments").join(OBJECT_ID).join(id);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(file_name);
    if encrypted {
        let key = svc.attachment_encryption_key().unwrap();
        let mut output = File::create(&path).unwrap();
        solosoul_crypto::cipher::encrypt_chunked_stream(
            &key,
            contents.len() as u64,
            &mut std::io::Cursor::new(contents),
            &mut output,
        )
        .unwrap();
    } else {
        std::fs::write(&path, contents).unwrap();
    }
    assert_eq!(
        solosoul_core::attachment_crypto::is_encrypted_file(&path),
        encrypted
    );
    f.vault
        .save_object(&ObjectRecord {
            id: OBJECT_ID.into(),
            account_id: f.account.clone(),
            type_id: "note".into(),
            section_type: "identity".into(),
            name: "RF014 synthetic object".into(),
            properties: json!({"__attachments": [{
                "id": id, "objectId": OBJECT_ID, "fileName": file_name,
                "mimeType": "text/plain", "sizeBytes": contents.len(),
                "createdAt": "2026-09-30T00:00:00Z", "vaultPath": path
            }]}),
            created_at: "2026-09-30T00:00:00Z".into(),
            updated_at: "2026-09-30T00:00:00Z".into(),
            version: 1,
            ..Default::default()
        })
        .unwrap();
    drop(svc);
    f
}

fn context(f: &Fixture) -> CloudPreContext {
    let svc = f.service.read().unwrap();
    CloudPreContext {
        service: f.service.clone(),
        session: svc.capture_session(&f.account).unwrap(),
        account_id: f.account.clone(),
        // 仅内存里的合成配置，不写入 Vault 或任何真实云端配置。
        config: solosoul_vault::CloudSyncConfig {
            snapshot_password: EXPORT_PASSWORD.into(),
            ..Default::default()
        },
        base_path: svc.base_path().to_path_buf(),
        device_id: "rf014-test-device".into(),
        emit_event: Arc::new(|_, _| panic!("local export must not emit cloud events")),
        barrier: None,
    }
}

fn selected_export(f: &Fixture, encrypted: bool, path: &Path) {
    let (id, _, _) = attachment_spec(encrypted);
    let req = ExportRequest {
        scope: ExportScope {
            selected_page_ids: vec![],
            selected_object_ids: vec![OBJECT_ID.into()],
            selected_tags: vec![],
            include_attachments: true,
            selected_attachment_ids: vec![id.into()],
            include_preferences: false,
            include_behavioral: false,
            include_all: false,
        },
        password: EXPORT_PASSWORD.into(),
        password_hint: None,
        save_path: path.to_string_lossy().into_owned(),
    };
    let svc = f.service.read().unwrap();
    let session = svc.capture_session(&f.account).unwrap();
    execute_export_for_session(&svc, &session, &req, path.to_str().unwrap()).unwrap();
}

/// 同时验证 ZIP 内容、包内密文解密与独立 Vault 恢复，避免夹具错误造成假红灯。
fn assert_package_and_restore(path: &Path, encrypted: bool) {
    let (id, file_name, contents) = attachment_spec(encrypted);
    let expected_entry = format!("attachments/{OBJECT_ID}/{id}.enc");
    let mut archive = ZipArchive::new(File::open(path).unwrap()).unwrap();
    let manifest_json: serde_json::Value =
        serde_json::from_reader(archive.by_name("manifest.json").unwrap()).unwrap();
    assert_eq!(manifest_json["object_count"], 1);
    let entries: Vec<_> = archive
        .file_names()
        .filter(|name| name.starts_with("attachments/"))
        .map(str::to_owned)
        .collect();
    eprintln!(
        "RF014 mode={} object_count={} has_attachments={} attachment_entries={entries:?}",
        if encrypted {
            "SOLC"
        } else {
            "legacy-plaintext"
        },
        manifest_json["object_count"],
        manifest_json["has_attachments"]
    );
    assert_eq!(
        entries,
        vec![expected_entry.clone()],
        "full cloud snapshot must contain the valid attachment bytes"
    );
    drop(archive);
    let manifest =
        crate::commands::export_import::helpers::read_manifest(path.to_str().unwrap()).unwrap();
    assert!(manifest.has_attachments);
    let salt = hex::decode(&manifest.salt_hex).unwrap();
    let key =
        solosoul_crypto::kdf::derive_export_key(EXPORT_PASSWORD, &salt, &manifest.kdf_config())
            .unwrap();
    let attachment_key =
        solosoul_crypto::hkdf_ext::derive_hkdf_key(&key, &salt, b"solosoul:attachments:v1")
            .unwrap();
    let mut plaintext = Vec::new();
    crate::commands::export_import::helpers::decrypt_zip_entry_streaming(
        path.to_str().unwrap(),
        &expected_entry,
        &attachment_key,
        &mut plaintext,
    )
    .unwrap();
    assert_eq!(plaintext, contents);

    let target = Fixture::new();
    assert!(target.vault.load_object(OBJECT_ID).unwrap().is_none());
    let svc = target.service.read().unwrap();
    let session = svc.capture_session(&target.account).unwrap();
    let result = import_execute_for_session(
        &svc,
        &session,
        path.to_string_lossy().into_owned(),
        Zeroizing::new(EXPORT_PASSWORD.into()),
        ImportStrategy::Overwrite,
        None,
        None,
        HashMap::new(),
        "en-US",
        None,
    )
    .unwrap();
    assert_eq!((result.object_count, result.attachment_count), (1, 1));
    result.require_complete().unwrap();
    let object = target.vault.load_object(OBJECT_ID).unwrap().unwrap();
    assert_eq!(object.account_id, target.account);
    let restored = object.properties["__attachments"].as_array().unwrap();
    assert_eq!(restored.len(), 1);
    assert_eq!(restored[0]["fileName"], file_name);
    let restored_path = Path::new(restored[0]["vaultPath"].as_str().unwrap());
    assert!(restored_path.starts_with(svc.base_path().join("attachments")));
    assert!(solosoul_core::attachment_crypto::is_encrypted_file(
        restored_path
    ));
    let target_key = svc.attachment_encryption_key().unwrap();
    assert_eq!(
        solosoul_core::attachment_crypto::read_file_decrypted(&target_key, restored_path, 1024)
            .unwrap(),
        contents
    );
}

#[test]
fn rf014_selected_plaintext_attachment_roundtrips() {
    let f = fixture(false);
    let path = f.dir.path().join("selected-legacy.solosoul");
    selected_export(&f, false, &path);
    assert_package_and_restore(&path, false);
}

#[test]
fn rf014_selected_solc_attachment_roundtrips() {
    let f = fixture(true);
    let path = f.dir.path().join("selected-solc.solosoul");
    selected_export(&f, true, &path);
    assert_package_and_restore(&path, true);
}

#[tokio::test]
async fn rf014_cloud_snapshot_contains_plaintext_attachment() {
    let f = fixture(false);
    let path = f.dir.path().join("cloud-legacy.solosoul");
    export_full_snapshot(&context(&f), &path).await.unwrap();
    assert_package_and_restore(&path, false);
}

#[tokio::test]
async fn rf014_cloud_snapshot_contains_solc_attachment() {
    let f = fixture(true);
    let path = f.dir.path().join("cloud-solc.solosoul");
    export_full_snapshot(&context(&f), &path).await.unwrap();
    assert_package_and_restore(&path, true);
}
