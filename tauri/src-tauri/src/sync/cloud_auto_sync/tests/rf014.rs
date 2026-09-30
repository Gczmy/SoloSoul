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

/// 附加项都有真实小文件，避免“文件本来缺失”使删除过滤得到假绿灯。
fn additional_attachment(
    f: &Fixture,
    object_id: &str,
    attachment_id: &str,
    contents: &[u8],
) -> serde_json::Value {
    let svc = f.service.read().unwrap();
    let directory = svc
        .base_path()
        .join("attachments")
        .join(object_id)
        .join(attachment_id);
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("synthetic.txt");
    std::fs::write(&path, contents).unwrap();
    json!({
        "id": attachment_id, "objectId": object_id, "fileName": "synthetic.txt",
        "mimeType": "text/plain", "sizeBytes": contents.len(),
        "createdAt": "2026-09-30T00:00:00Z", "vaultPath": path
    })
}

fn package_entries(path: &Path) -> (serde_json::Value, Vec<String>) {
    let mut archive = ZipArchive::new(File::open(path).unwrap()).unwrap();
    let manifest: serde_json::Value =
        serde_json::from_reader(archive.by_name("manifest.json").unwrap()).unwrap();
    let mut entries = archive
        .file_names()
        .filter(|name| name.starts_with("attachments/"))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    entries.sort();
    (manifest, entries)
}

/// 独立空 Vault 的真实恢复，不用“ZIP 条目存在”代替可解密和关联成功。
fn restore_package(path: &Path, object_count: usize, attachment_count: usize) -> Fixture {
    let target = Fixture::new();
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
    assert_eq!(result.object_count, object_count);
    assert_eq!(result.attachment_count, attachment_count);
    result.require_complete().unwrap();
    drop(svc);
    target
}

fn assert_restored_bytes(f: &Fixture, file_name: &str, expected: &[u8]) {
    let svc = f.service.read().unwrap();
    let object = f.vault.load_object(OBJECT_ID).unwrap().unwrap();
    let meta = object.properties["__attachments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|value| value["fileName"] == file_name)
        .expect("live attachment metadata must be restored");
    let path = Path::new(meta["vaultPath"].as_str().unwrap());
    assert!(path.starts_with(svc.base_path().join("attachments")));
    assert!(solosoul_core::attachment_crypto::is_encrypted_file(path));
    assert_eq!(
        solosoul_core::attachment_crypto::read_file_decrypted(
            &svc.attachment_encryption_key().unwrap(),
            path,
            1024,
        )
        .unwrap(),
        expected
    );
}

#[tokio::test]
async fn rf014_cloud_snapshot_excludes_deleted_objects_and_attachments() {
    let f = fixture(false);
    let mut object = f.vault.load_object(OBJECT_ID).unwrap().unwrap();
    let extra = additional_attachment(
        &f,
        OBJECT_ID,
        "rf014-second-live",
        b"Second live attachment",
    );
    let mut deleted = additional_attachment(
        &f,
        OBJECT_ID,
        "rf014-deleted-attachment",
        b"Deleted attachment still has a physical file",
    );
    deleted["deletedAt"] = "2026-09-30T00:00:03Z".into();
    let deleted_object_id = "rf014-deleted-object";
    let orphan = additional_attachment(
        &f,
        deleted_object_id,
        "rf014-attachment-of-deleted-object",
        b"Deleted object attachment still has a physical file",
    );
    let mut source_files = Vec::new();
    for meta in [
        object.properties["__attachments"][0].clone(),
        extra.clone(),
        deleted.clone(),
        orphan.clone(),
    ] {
        let path = std::path::PathBuf::from(meta["vaultPath"].as_str().unwrap());
        source_files.push((path.clone(), std::fs::read(path).unwrap()));
    }
    object.properties["__attachments"]
        .as_array_mut()
        .unwrap()
        .extend([extra, deleted]);
    f.vault.save_object(&object).unwrap();
    f.vault
        .save_object(&ObjectRecord {
            id: deleted_object_id.into(),
            account_id: f.account.clone(),
            type_id: "note".into(),
            section_type: "identity".into(),
            name: "Deleted synthetic object".into(),
            properties: json!({"__attachments": [orphan]}),
            is_deleted: true,
            deleted_at: Some("2026-09-30T00:00:03Z".into()),
            created_at: "2026-09-30T00:00:00Z".into(),
            updated_at: "2026-09-30T00:00:03Z".into(),
            version: 1,
            ..Default::default()
        })
        .unwrap();

    let pre = context(&f);
    let mut collected = collect_all_attachment_ids(pre.session.vault(), &f.account).unwrap();
    collected.sort();
    assert_eq!(
        collected,
        vec!["rf014-legacy".to_string(), "rf014-second-live".to_string()]
    );
    let path = f.dir.path().join("live-only.solosoul");
    export_full_snapshot(&pre, &path).await.unwrap();
    let (manifest, entries) = package_entries(&path);
    assert_eq!(manifest["object_count"], 1);
    assert_eq!(manifest["has_attachments"], true);
    assert_eq!(
        entries,
        [
            format!("attachments/{OBJECT_ID}/rf014-legacy.enc"),
            format!("attachments/{OBJECT_ID}/rf014-second-live.enc"),
        ]
    );
    let target = restore_package(&path, 1, 2);
    assert!(target
        .vault
        .load_object(deleted_object_id)
        .unwrap()
        .is_none());
    assert_restored_bytes(&target, attachment_spec(false).1, attachment_spec(false).2);
    assert_restored_bytes(&target, "synthetic.txt", b"Second live attachment");
    let target_base = target.service.read().unwrap().base_path().to_path_buf();
    // 恢复会重新生成随机附件 ID，检查实际目录总数而不是旧 ID 的目录缺失。
    assert_eq!(
        std::fs::read_dir(target_base.join("attachments").join(OBJECT_ID))
            .unwrap()
            .count(),
        2
    );
    assert!(!target_base
        .join("attachments")
        .join(deleted_object_id)
        .exists());
    for (path, bytes) in source_files {
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
}

#[test]
fn rf014_manual_empty_attachment_selection_still_exports_no_attachment_bytes() {
    let f = fixture(false);
    let path = f.dir.path().join("manual-empty.solosoul");
    let req = ExportRequest {
        scope: ExportScope {
            selected_page_ids: vec![],
            selected_object_ids: vec![OBJECT_ID.into()],
            selected_tags: vec![],
            include_attachments: true,
            selected_attachment_ids: vec![],
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
    drop(svc);
    let (manifest, entries) = package_entries(&path);
    assert_eq!(manifest["object_count"], 1);
    assert_eq!(manifest["has_attachments"], false);
    assert!(entries.is_empty());
    let target = restore_package(&path, 1, 0);
    assert!(!target
        .service
        .read()
        .unwrap()
        .base_path()
        .join("attachments")
        .join(OBJECT_ID)
        .exists());
}

#[tokio::test]
async fn rf014_empty_vault_full_snapshot_is_restorable_without_attachments() {
    let f = Fixture::new();
    let path = f.dir.path().join("empty.solosoul");
    export_full_snapshot(&context(&f), &path).await.unwrap();
    let (manifest, entries) = package_entries(&path);
    assert_eq!(manifest["object_count"], 0);
    assert_eq!(manifest["has_attachments"], false);
    assert!(entries.is_empty());
    let target = restore_package(&path, 0, 0);
    assert!(target
        .vault
        .list_object_records(&target.account)
        .unwrap()
        .is_empty());
}

/// 覆盖单纯锁定、其他账户和同账户重新解锁；旧任务不能替换已有有效包。
#[tokio::test]
async fn rf014_expired_snapshot_preserves_existing_target_after_session_changes() {
    for transition in ["lock", "account-switch", "same-account-reunlock"] {
        let f = fixture(false);
        let output_directory = f.dir.path().join("exports");
        std::fs::create_dir(&output_directory).unwrap();
        let target = output_directory.join("existing.solosoul");
        selected_export(&f, false, &target);
        let previous = std::fs::read(&target).unwrap();
        let pre = context(&f);
        {
            let svc = f.service.read().unwrap();
            match transition {
                "lock" => svc.lock(),
                "account-switch" => {
                    svc.create_account_with_id(
                        "acc_rf014_other",
                        "Other synthetic account",
                        "password456",
                        None,
                    )
                    .unwrap();
                }
                "same-account-reunlock" => {
                    svc.lock();
                    svc.unlock(&f.account, "password123").unwrap();
                }
                _ => unreachable!(),
            }
        }
        assert!(
            export_full_snapshot(&pre, &target).await.is_err(),
            "expired snapshot unexpectedly succeeded after {transition}"
        );
        assert_eq!(std::fs::read(&target).unwrap(), previous, "{transition}");
        let files = std::fs::read_dir(&output_directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        assert_eq!(
            files.as_slice(),
            std::slice::from_ref(&target),
            "temporary output remains after {transition}"
        );
        if transition == "account-switch" {
            let svc = f.service.read().unwrap();
            assert!(svc
                .get_vault_store()
                .unwrap()
                .load_object(OBJECT_ID)
                .unwrap()
                .is_none());
        }
    }
}
