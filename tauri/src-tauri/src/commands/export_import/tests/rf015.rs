//! RF-015：真实 SQLite、导出 ZIP 与包内密文解密，不替换导出器或附件读写。
use super::rf020::Fixture;
use crate::commands::export_import::export::estimate_attachments;
use crate::commands::export_import::helpers::{decrypt_zip_entry_streaming, read_manifest};
use crate::commands::export_import::{
    collect_scope_objects, derive_export_key_cfg, execute_export_core_with_attachment_scope,
    execute_export_for_session, execute_export_for_session_with_attachment_scope,
    AttachmentExportScope, ExportRequest, ExportScope,
};
use serde_json::{json, Value};
use solosoul_vault::ObjectRecord;
use std::fs::{self, File};
use std::path::Path;
use zip::ZipArchive;

pub(crate) const OBJECT_ID: &str = "rf015-main-object";
pub(crate) const PLAIN_ID: &str = "rf015-plain";
pub(crate) const SOLC_ID: &str = "rf015-solc";
pub(crate) const EXPORT_PASSWORD: &str = "rf015-export-password";
pub(crate) const PLAIN_BYTES: &[u8] = b"RF015 actual legacy plaintext attachment";
pub(crate) const SOLC_BYTES: &[u8] =
    b"RF015 actual SOLC attachment encrypted with original vault key";

fn attachment(
    f: &Fixture,
    owner: &str,
    id: &str,
    file_name: &str,
    bytes: &[u8],
    encrypted: bool,
) -> Value {
    let svc = f.service.read().unwrap();
    let path = svc
        .base_path()
        .join("attachments")
        .join(owner)
        .join(id)
        .join(file_name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    if encrypted {
        let session = svc.capture_session(&f.account).unwrap();
        let key = svc.attachment_key_for_session(&session).unwrap();
        let mut file = File::create(&path).unwrap();
        solosoul_crypto::cipher::encrypt_chunked_stream(
            &key,
            bytes.len() as u64,
            &mut std::io::Cursor::new(bytes),
            &mut file,
        )
        .unwrap();
    } else {
        fs::write(&path, bytes).unwrap();
    }
    assert_eq!(
        solosoul_core::attachment_crypto::is_encrypted_file(&path),
        encrypted
    );
    json!({
        "id": id, "objectId": owner, "fileName": file_name, "mimeType": "text/plain",
        "sizeBytes": bytes.len(), "createdAt": "2026-10-01T00:00:00Z", "vaultPath": path
    })
}

fn save_object(f: &Fixture, id: &str, attachments: Vec<Value>) {
    f.vault
        .save_object(&ObjectRecord {
            id: id.into(),
            account_id: f.account.clone(),
            type_id: "note".into(),
            section_type: "identity".into(),
            name: format!("RF015 {id}"),
            properties: json!({"__attachments": attachments, "value": "真实导出对象"}),
            created_at: "2026-10-01T00:00:00Z".into(),
            updated_at: "2026-10-01T00:00:00Z".into(),
            version: 1,
            ..Default::default()
        })
        .unwrap();
}

pub(crate) fn paired_fixture() -> Fixture {
    let f = Fixture::new();
    let plain = attachment(&f, OBJECT_ID, PLAIN_ID, "plain.txt", PLAIN_BYTES, false);
    let encrypted = attachment(&f, OBJECT_ID, SOLC_ID, "cipher.txt", SOLC_BYTES, true);
    save_object(&f, OBJECT_ID, vec![plain, encrypted]);
    f
}

fn request(path: &Path) -> ExportRequest {
    ExportRequest {
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
    }
}

fn manual_export(f: &Fixture, req: &ExportRequest, path: &Path) -> Result<(), String> {
    let svc = f.service.read().unwrap();
    let session = svc.capture_session(&f.account).unwrap();
    execute_export_for_session(&svc, &session, req, path.to_str().unwrap())
}

fn typed_export(
    f: &Fixture,
    req: &ExportRequest,
    path: &Path,
    scope: &AttachmentExportScope,
) -> Result<(), String> {
    let svc = f.service.read().unwrap();
    let session = svc.capture_session(&f.account).unwrap();
    execute_export_for_session_with_attachment_scope(
        &svc,
        &session,
        req,
        path.to_str().unwrap(),
        scope,
    )
}

/// 验证实际包结构、payload 内容及每个附件的 HKDF + SOLC 解密；不能只断言元数据或条目名。
pub(crate) fn assert_package(path: &Path, object_ids: &[&str], expected: &[(&str, &str, &[u8])]) {
    let mut archive = ZipArchive::new(File::open(path).unwrap()).unwrap();
    let manifest_json: Value =
        serde_json::from_reader(archive.by_name("manifest.json").unwrap()).unwrap();
    assert_eq!(manifest_json["object_count"], object_ids.len());
    assert_eq!(manifest_json["has_attachments"], !expected.is_empty());
    let mut entries = archive
        .file_names()
        .filter(|name| name.starts_with("attachments/"))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    entries.sort();
    let mut expected_entries = expected
        .iter()
        .map(|(owner, id, _)| format!("attachments/{owner}/{id}.enc"))
        .collect::<Vec<_>>();
    expected_entries.sort();
    assert_eq!(entries, expected_entries);
    drop(archive);
    let manifest = read_manifest(path.to_str().unwrap()).unwrap();
    let salt = hex::decode(&manifest.salt_hex).unwrap();
    let key = derive_export_key_cfg(EXPORT_PASSWORD, &salt, &manifest.kdf_config()).unwrap();
    let mut payload_bytes = Vec::new();
    decrypt_zip_entry_streaming(
        path.to_str().unwrap(),
        "payload.enc",
        &key,
        &mut payload_bytes,
    )
    .unwrap();
    let payload: Value = serde_json::from_slice(&payload_bytes).unwrap();
    let objects = payload["objects"].as_array().unwrap();
    let mut actual_ids = objects
        .iter()
        .map(|object| object["id"].as_str().unwrap())
        .collect::<Vec<_>>();
    actual_ids.sort();
    let mut expected_ids = object_ids.to_vec();
    expected_ids.sort();
    assert_eq!(actual_ids, expected_ids);
    assert!(objects
        .iter()
        .all(|object| object["properties"]["value"] == "真实导出对象"));
    let attachment_key =
        solosoul_crypto::hkdf_ext::derive_hkdf_key(&key, &salt, b"solosoul:attachments:v1")
            .unwrap();
    for (owner, id, plaintext) in expected {
        let mut decrypted = Vec::new();
        decrypt_zip_entry_streaming(
            path.to_str().unwrap(),
            &format!("attachments/{owner}/{id}.enc"),
            &attachment_key,
            &mut decrypted,
        )
        .unwrap();
        assert_eq!(decrypted.as_slice(), *plaintext);
    }
}

#[test]
fn rf015_legacy_wire_payload_stays_readable_with_manual_empty_selection() {
    let scope_json = json!({
        "selectedPageIds": [], "selectedObjectIds": [OBJECT_ID], "selectedTags": [],
        "includeAttachments": true, "selectedAttachmentIds": [],
        "includePreferences": false, "includeBehavioral": false
    });
    let legacy = json!({"scope": scope_json, "password": EXPORT_PASSWORD, "passwordHint": null, "savePath": "legacy.solosoul"});
    let req: ExportRequest = serde_json::from_value(legacy.clone()).unwrap();
    assert!(!req.scope.include_all);
    assert!(
        matches!(req.scope.attachment_export_scope(), AttachmentExportScope::Selected(ids) if ids.is_empty())
    );
    let mut expected = legacy;
    expected["scope"]["includeAll"] = false.into();
    assert_eq!(serde_json::to_value(&req).unwrap(), expected);
    let f = paired_fixture();
    let path = f.dir.path().join("legacy-wire.solosoul");
    manual_export(&f, &req, &path).unwrap();
    assert_package(&path, &[OBJECT_ID], &[]);
}

#[test]
fn rf015_manual_disabled_with_ids_and_enabled_empty_both_export_zero_attachments() {
    let f = paired_fixture();
    for (name, enabled, ids) in [
        (
            "disabled-with-ids",
            false,
            vec![PLAIN_ID.into(), SOLC_ID.into()],
        ),
        ("enabled-empty", true, vec![]),
    ] {
        let path = f.dir.path().join(format!("{name}.solosoul"));
        let mut req = request(&path);
        req.scope.include_attachments = enabled;
        req.scope.selected_attachment_ids = ids;
        manual_export(&f, &req, &path).unwrap();
        assert_package(&path, &[OBJECT_ID], &[]);
    }
}

#[test]
fn rf015_manual_partial_filters_unknown_deleted_and_duplicate_selection_ids() {
    let f = paired_fixture();
    let mut deleted = attachment(
        &f,
        OBJECT_ID,
        "rf015-deleted",
        "deleted.txt",
        b"physical deleted attachment",
        false,
    );
    deleted["deletedAt"] = "2026-10-01T00:00:01Z".into();
    let deleted_path = std::path::PathBuf::from(deleted["vaultPath"].as_str().unwrap());
    let original_deleted = fs::read(&deleted_path).unwrap();
    let mut record = f.vault.load_object(OBJECT_ID).unwrap().unwrap();
    record.properties["__attachments"]
        .as_array_mut()
        .unwrap()
        .push(deleted);
    f.vault.save_object(&record).unwrap();
    let path = f.dir.path().join("manual-partial.solosoul");
    let mut req = request(&path);
    req.scope.selected_attachment_ids = vec![
        SOLC_ID.into(),
        "unknown-id".into(),
        "rf015-deleted".into(),
        SOLC_ID.into(),
    ];
    let scope = req.scope.attachment_export_scope();
    assert!(matches!(scope, AttachmentExportScope::Selected(ref ids) if ids.len() == 3));
    manual_export(&f, &req, &path).unwrap();
    assert_package(&path, &[OBJECT_ID], &[(OBJECT_ID, SOLC_ID, SOLC_BYTES)]);
    assert_eq!(fs::read(&deleted_path).unwrap(), original_deleted);
}

#[test]
fn rf015_explicit_all_exports_plain_and_solc_only_inside_selected_objects() {
    let f = paired_fixture();
    let extra = attachment(
        &f,
        "rf015-unselected-object",
        "rf015-outside",
        "outside.txt",
        b"unselected object payload",
        false,
    );
    let extra_path = std::path::PathBuf::from(extra["vaultPath"].as_str().unwrap());
    let extra_bytes = fs::read(&extra_path).unwrap();
    save_object(&f, "rf015-unselected-object", vec![extra]);
    let path = f.dir.path().join("explicit-all-selected-object.solosoul");
    let req = request(&path);
    assert!(req.scope.selected_attachment_ids.is_empty());
    typed_export(&f, &req, &path, &AttachmentExportScope::All).unwrap();
    assert_package(
        &path,
        &[OBJECT_ID],
        &[
            (OBJECT_ID, PLAIN_ID, PLAIN_BYTES),
            (OBJECT_ID, SOLC_ID, SOLC_BYTES),
        ],
    );
    assert_eq!(fs::read(extra_path).unwrap(), extra_bytes);
}

#[test]
fn rf015_explicit_none_ignores_wire_ids_and_invalid_attachment_metadata() {
    let f = paired_fixture();
    let outside = f.dir.path().join("none-outside.txt");
    fs::write(&outside, b"never selected for export").unwrap();
    let mut record = f.vault.load_object(OBJECT_ID).unwrap().unwrap();
    record.properties["__attachments"][0]["sizeBytes"] = (101u64 * 1024 * 1024).into();
    record.properties["__attachments"][0]["vaultPath"] =
        outside.to_string_lossy().into_owned().into();
    f.vault.save_object(&record).unwrap();
    let path = f.dir.path().join("explicit-none.solosoul");
    let mut req = request(&path);
    req.scope.selected_attachment_ids = vec![PLAIN_ID.into(), SOLC_ID.into()];
    typed_export(&f, &req, &path, &AttachmentExportScope::None).unwrap();
    assert_package(&path, &[OBJECT_ID], &[]);
    assert_eq!(fs::read(outside).unwrap(), b"never selected for export");
}

#[test]
fn rf015_unselected_oversized_and_outside_paths_do_not_block_valid_selected_attachment() {
    let f = paired_fixture();
    let mut oversized = attachment(
        &f,
        OBJECT_ID,
        "rf015-oversized",
        "large.txt",
        b"small real source with oversized declaration",
        false,
    );
    oversized["sizeBytes"] = (101u64 * 1024 * 1024).into();
    let outside = f.dir.path().join("outside-vault-attachments.txt");
    fs::write(&outside, b"real file outside vault attachments").unwrap();
    let invalid = json!({
        "id": "rf015-invalid-path", "objectId": OBJECT_ID, "fileName": "outside.txt", "mimeType": "text/plain",
        "sizeBytes": 3, "createdAt": "2026-10-01T00:00:00Z", "vaultPath": outside
    });
    let mut record = f.vault.load_object(OBJECT_ID).unwrap().unwrap();
    let atts = record.properties["__attachments"].as_array_mut().unwrap();
    atts.push(oversized);
    atts.push(invalid);
    f.vault.save_object(&record).unwrap();
    let path = f.dir.path().join("valid-selected.solosoul");
    let mut req = request(&path);
    req.scope.selected_attachment_ids = vec![PLAIN_ID.into()];
    manual_export(&f, &req, &path).unwrap();
    assert_package(&path, &[OBJECT_ID], &[(OBJECT_ID, PLAIN_ID, PLAIN_BYTES)]);
    for (id, name, prefix) in [
        (
            "rf015-oversized",
            "oversized-selected",
            "__EXPORT_ERR__:ATTACHMENT_TOO_LARGE",
        ),
        (
            "rf015-invalid-path",
            "outside-selected",
            "Attachment path escapes vault attachments directory:",
        ),
    ] {
        let rejected_path = f.dir.path().join(format!("{name}.solosoul"));
        let mut selected = request(&rejected_path);
        selected.scope.selected_attachment_ids = vec![id.into()];
        assert!(manual_export(&f, &selected, &rejected_path)
            .unwrap_err()
            .starts_with(prefix));
        assert!(!rejected_path.exists());
    }
    assert_eq!(
        fs::read(outside).unwrap(),
        b"real file outside vault attachments"
    );
}

#[test]
fn rf015_estimator_preserves_available_selected_and_declared_byte_boundaries() {
    let f = paired_fixture();
    let mut deleted = attachment(
        &f,
        OBJECT_ID,
        "rf015-estimate-deleted",
        "deleted.txt",
        b"deleted bytes do not count",
        false,
    );
    deleted["deletedAt"] = "2026-10-01T00:00:01Z".into();
    let mut record = f.vault.load_object(OBJECT_ID).unwrap().unwrap();
    record.properties["__attachments"]
        .as_array_mut()
        .unwrap()
        .push(deleted);
    f.vault.save_object(&record).unwrap();
    let outside = attachment(
        &f,
        "rf015-not-selected",
        "rf015-not-counted",
        "outside.txt",
        b"outside record scope",
        false,
    );
    save_object(&f, "rf015-not-selected", vec![outside]);
    let req = request(&f.dir.path().join("estimate.solosoul"));
    let records = collect_scope_objects(&f.vault, &f.account, &req.scope).unwrap();
    assert_eq!(records.len(), 1);
    let cases = [
        (AttachmentExportScope::None, (0, 0, 0)),
        (
            AttachmentExportScope::from_manual_selection(true, &[]),
            (2, 0, 0),
        ),
        (
            AttachmentExportScope::from_manual_selection(
                true,
                &[SOLC_ID.into(), SOLC_ID.into(), "unknown".into()],
            ),
            (2, 1, SOLC_BYTES.len() as u64),
        ),
        (
            AttachmentExportScope::All,
            (2, 2, (PLAIN_BYTES.len() + SOLC_BYTES.len()) as u64),
        ),
    ];
    for (scope, expected) in cases {
        assert_eq!(estimate_attachments(&records, &scope), expected);
    }
}

#[test]
fn rf015_typed_core_all_captures_original_session_and_exports_actual_package() {
    let f = paired_fixture();
    let path = f.dir.path().join("typed-core-all.solosoul");
    let mut req = request(&path);
    req.scope.include_all = true;
    req.scope.selected_object_ids.clear();
    let svc = f.service.read().unwrap();
    execute_export_core_with_attachment_scope(
        &svc,
        &f.account,
        &req,
        path.to_str().unwrap(),
        &AttachmentExportScope::All,
    )
    .unwrap();
    drop(svc);
    assert_package(
        &path,
        &[OBJECT_ID],
        &[
            (OBJECT_ID, PLAIN_ID, PLAIN_BYTES),
            (OBJECT_ID, SOLC_ID, SOLC_BYTES),
        ],
    );
}
