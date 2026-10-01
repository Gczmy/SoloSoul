//! RF-015：真实 SQLite、加密 ZIP 和附件范围回归。
use super::tests::{make_test_record, test_setup};
use super::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

const PASSWORD: &str = "RF015-Export-Password1";
const INSIDE: &str = "rf015_inside";
const OUTSIDE: &str = "rf015_outside";
const FIRST: &str = "rf015_first";
const SECOND: &str = "rf015_second";
const DELETED: &str = "rf015_deleted";
const OTHER: &str = "rf015_other";
const FIRST_BYTES: &[u8] = b"first selected attachment\nwith private text";
const SECOND_BYTES: &[u8] = &[0, 1, 2, 127, 128, 254, 255];
const OTHER_BYTES: &[u8] = b"attachment of an object outside the requested scope";

struct Fixture {
    vault: Arc<VaultStore>,
    account_id: String,
    dir: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        let (vault, account_id, dir) = test_setup();
        let fixture = Self {
            vault,
            account_id,
            dir,
        };
        let mut deleted = fixture.attachment(INSIDE, DELETED, b"deleted attachment");
        deleted.deleted_at = Some("2026-10-01T00:00:00Z".into());
        // All / Selected(deleted) 必须先排除删除项，不能误走大小错误。
        deleted.size_bytes = MAX_ATTACHMENT_BYTES + 1;
        fixture.save_record(
            INSIDE,
            "identity",
            vec![
                fixture.attachment(INSIDE, FIRST, FIRST_BYTES),
                fixture.attachment(INSIDE, SECOND, SECOND_BYTES),
                deleted,
            ],
        );
        fixture.save_record(
            OUTSIDE,
            "health",
            vec![fixture.attachment(OUTSIDE, OTHER, OTHER_BYTES)],
        );
        fixture
    }

    fn attachment(&self, owner: &str, id: &str, bytes: &[u8]) -> AttachmentMeta {
        let file_name = format!("{id}.bin");
        let directory = self.dir.path().join("attachments").join(owner).join(id);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join(&file_name), bytes).unwrap();
        AttachmentMeta {
            id: id.into(),
            object_id: owner.into(),
            file_name,
            mime_type: "application/octet-stream".into(),
            size_bytes: bytes.len() as u64,
            created_at: "2026-10-01T00:00:00Z".into(),
            deleted_at: None,
            src_path: None,
            vault_path: None,
            description: None,
            tags: vec![],
        }
    }

    fn save_record(&self, id: &str, page: &str, attachments: Vec<AttachmentMeta>) {
        let mut record = make_test_record(&self.account_id, id, id);
        record.section_type = page.into();
        record.properties = json!({"title": id, "__attachments": attachments});
        self.vault.save_object(&record).unwrap();
    }

    fn object_scope(&self) -> ExportScope {
        ExportScope {
            selected_object_ids: vec![INSIDE.into()],
            include_attachments: true,
            ..Default::default()
        }
    }

    fn path(&self, case: &str) -> PathBuf {
        self.dir.path().join(format!("rf015-{case}.solosoul"))
    }

    fn export(
        &self,
        case: &str,
        scope: &ExportScope,
        attachments: &AttachmentExportScope,
    ) -> DecodedPackage {
        let path = self.path(case);
        let count = export_vault_with_attachment_scope(
            &self.vault,
            &self.account_id,
            PASSWORD,
            &path,
            scope,
            self.dir.path(),
            attachments,
        )
        .unwrap();
        let package = self.decode(&path);
        assert_eq!(count, package.payload["objects"].as_array().unwrap().len());
        package
    }

    fn decode(&self, path: &Path) -> DecodedPackage {
        let manifest: Value =
            serde_json::from_slice(&read_file_from_zip(path, "manifest.json").unwrap()).unwrap();
        assert_eq!(manifest["version"], "2.0");
        let parsed = read_manifest(path).unwrap();
        let salt = hex::decode(&parsed.salt_hex).unwrap();
        let key = derive_export_key_cfg(PASSWORD, &salt, &parsed.kdf_config()).unwrap();
        let payload = decrypt_payload_stream(path, self.dir.path(), &key).unwrap();
        let att_key =
            solosoul_crypto::hkdf_ext::derive_hkdf_key(&key, &salt, b"solosoul:attachments:v1")
                .unwrap();
        let mut archive = ZipArchive::new(File::open(path).unwrap()).unwrap();
        let mut attachments = BTreeMap::new();
        let mut zip_names = HashSet::new();
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).unwrap();
            let name = entry.name().to_string();
            assert!(
                zip_names.insert(name.clone()),
                "actual ZIP must not contain duplicate entries"
            );
            if name.starts_with("attachments/") {
                let mut encrypted = Vec::new();
                entry.read_to_end(&mut encrypted).unwrap();
                assert!(encrypted.starts_with(b"SOLC\x02"));
                let plaintext =
                    solosoul_crypto::cipher::decrypt_chunked_from_bytes(&att_key, &encrypted)
                        .unwrap();
                assert!(attachments.insert(name, plaintext.to_vec()).is_none());
            }
        }
        DecodedPackage {
            manifest,
            payload,
            attachments,
            zip_names,
        }
    }

    fn export_audit_count(&self) -> usize {
        self.vault
            .list_audit_log(1000)
            .unwrap()
            .iter()
            .filter(|entry| entry.action_type == "export_execute")
            .count()
    }
}

struct DecodedPackage {
    manifest: Value,
    payload: Value,
    attachments: BTreeMap<String, Vec<u8>>,
    zip_names: HashSet<String>,
}
impl DecodedPackage {
    fn assert_contents(&self, object_ids: &[&str], ids: &[(&str, &str, &[u8])]) {
        let actual: HashSet<&str> = self.payload["objects"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value["id"].as_str().unwrap())
            .collect();
        assert_eq!(actual, object_ids.iter().copied().collect());
        assert_eq!(
            self.manifest["object_count"].as_u64(),
            Some(object_ids.len() as u64)
        );
        assert_eq!(
            self.manifest["has_attachments"].as_bool(),
            Some(!ids.is_empty())
        );
        let expected: BTreeMap<String, Vec<u8>> = ids
            .iter()
            .map(|(object, id, bytes)| (format!("attachments/{object}/{id}.enc"), bytes.to_vec()))
            .collect();
        assert_eq!(self.attachments, expected);
        let mut names: HashSet<String> = expected.into_keys().collect();
        names.extend(["manifest.json".into(), "payload.enc".into()]);
        assert_eq!(self.zip_names, names);
    }
}

fn selected(ids: &[&str]) -> AttachmentExportScope {
    AttachmentExportScope::from_manual_selection(
        true,
        &ids.iter().map(|id| (*id).to_string()).collect::<Vec<_>>(),
    )
}

#[test]
fn rf015_explicit_scope_parameterization_decrypts_real_packages() {
    let fixture = Fixture::new();
    let cases = [
        ("none", AttachmentExportScope::None, vec![]),
        (
            "manual_disabled",
            AttachmentExportScope::from_manual_selection(false, &[FIRST.into()]),
            vec![],
        ),
        ("selected_empty", selected(&[]), vec![]),
        (
            "selected_one",
            selected(&[FIRST]),
            vec![(INSIDE, FIRST, FIRST_BYTES)],
        ),
        (
            "all",
            AttachmentExportScope::All,
            vec![(INSIDE, FIRST, FIRST_BYTES), (INSIDE, SECOND, SECOND_BYTES)],
        ),
        ("unknown", selected(&["unknown_id"]), vec![]),
        ("deleted", selected(&[DELETED]), vec![]),
        (
            "duplicate_request_ids",
            selected(&[FIRST, FIRST, "unknown_id"]),
            vec![(INSIDE, FIRST, FIRST_BYTES)],
        ),
        ("outside_object", selected(&[OTHER]), vec![]),
        (
            "inside_and_outside",
            selected(&[FIRST, OTHER]),
            vec![(INSIDE, FIRST, FIRST_BYTES)],
        ),
    ];
    assert_eq!(
        selected(&[]),
        AttachmentExportScope::Selected(HashSet::new())
    );
    let audits_before = fixture.export_audit_count();
    for (name, attachments, expected) in &cases {
        let package = fixture.export(name, &fixture.object_scope(), attachments);
        package.assert_contents(&[INSIDE], expected);
        // 只改变打包范围，保留历史 metadata-only payload 语义。
        let exported = &package.payload["objects"][0];
        assert_eq!(load_attachments(&exported["properties"]).len(), 3);
        assert_eq!(package.manifest["export_scope"], "partial");
    }
    assert_eq!(fixture.export_audit_count() - audits_before, cases.len());
}

#[test]
fn rf015_all_and_selected_preserve_full_page_and_object_ranges() {
    let fixture = Fixture::new();
    let cases = [
        (
            "full",
            ExportScope {
                full: true,
                include_attachments: false,
                ..Default::default()
            },
            AttachmentExportScope::All,
            vec![INSIDE, OUTSIDE],
            vec![
                (INSIDE, FIRST, FIRST_BYTES),
                (INSIDE, SECOND, SECOND_BYTES),
                (OUTSIDE, OTHER, OTHER_BYTES),
            ],
        ),
        (
            "page",
            ExportScope {
                selected_page_ids: vec!["identity".into()],
                ..Default::default()
            },
            AttachmentExportScope::All,
            vec![INSIDE],
            vec![(INSIDE, FIRST, FIRST_BYTES), (INSIDE, SECOND, SECOND_BYTES)],
        ),
        (
            "object",
            ExportScope {
                selected_object_ids: vec![OUTSIDE.into()],
                ..Default::default()
            },
            AttachmentExportScope::All,
            vec![OUTSIDE],
            vec![(OUTSIDE, OTHER, OTHER_BYTES)],
        ),
        (
            "page_filters_selected",
            ExportScope {
                selected_page_ids: vec!["identity".into()],
                ..Default::default()
            },
            selected(&[OTHER]),
            vec![INSIDE],
            vec![],
        ),
        (
            "object_filters_selected",
            ExportScope {
                selected_object_ids: vec![OUTSIDE.into()],
                ..Default::default()
            },
            selected(&[FIRST]),
            vec![OUTSIDE],
            vec![],
        ),
    ];
    for (name, scope, attachments, objects, expected) in &cases {
        fixture
            .export(name, scope, attachments)
            .assert_contents(objects, expected);
    }
}

#[test]
fn rf015_legacy_bool_api_preserves_none_and_all_real_packages() {
    let fixture = Fixture::new();
    for include_attachments in [false, true] {
        let mut scope = fixture.object_scope();
        scope.include_attachments = include_attachments;
        let path = fixture.path(if include_attachments {
            "legacy_all"
        } else {
            "legacy_none"
        });
        assert_eq!(
            export_vault(
                &fixture.vault,
                &fixture.account_id,
                PASSWORD,
                &path,
                &scope,
                fixture.dir.path()
            )
            .unwrap(),
            1
        );
        let expected = if include_attachments {
            vec![(INSIDE, FIRST, FIRST_BYTES), (INSIDE, SECOND, SECOND_BYTES)]
        } else {
            vec![]
        };
        fixture.decode(&path).assert_contents(&[INSIDE], &expected);
    }
}

#[test]
fn rf015_unselected_oversize_and_invalid_sources_do_not_affect_selected_bytes() {
    for oversize in [false, true] {
        let fixture = Fixture::new();
        let mut record = fixture.vault.load_object(INSIDE).unwrap().unwrap();
        let mut atts = load_attachments(&record.properties);
        let mut bad = fixture.attachment(INSIDE, "unselected_bad", b"unselected bad source");
        if oversize {
            bad.file_name = "oversize.bin".into();
            bad.size_bytes = MAX_ATTACHMENT_BYTES + 1;
        } else {
            // 真正存在的目录不可作为附件源读取；若错误解析未选 src 会导致导出失败。
            bad.src_path = Some(fixture.dir.path().to_string_lossy().into_owned());
        }
        atts.push(bad);
        record.properties["__attachments"] = serde_json::to_value(&atts).unwrap();
        fixture.vault.save_object(&record).unwrap();
        fixture
            .export(
                "selected_good",
                &fixture.object_scope(),
                &selected(&[FIRST]),
            )
            .assert_contents(&[INSIDE], &[(INSIDE, FIRST, FIRST_BYTES)]);
        fixture
            .export(
                "selected_empty_ignores_bad",
                &fixture.object_scope(),
                &selected(&[]),
            )
            .assert_contents(&[INSIDE], &[]);
        fixture
            .export(
                "none_ignores_bad",
                &fixture.object_scope(),
                &AttachmentExportScope::None,
            )
            .assert_contents(&[INSIDE], &[]);
        let audits_before = fixture.export_audit_count();
        let error = export_vault_with_attachment_scope(
            &fixture.vault,
            &fixture.account_id,
            PASSWORD,
            &fixture.path("bad_selected"),
            &fixture.object_scope(),
            fixture.dir.path(),
            &selected(&["unselected_bad"]),
        )
        .unwrap_err();
        if oversize {
            assert_eq!(error.to_string(), "附件过大: oversize.bin");
        } else {
            assert!(
                matches!(error, ExportError::Io(_) | ExportError::Msg(_)),
                "selected invalid directory must fail through the real reader"
            );
        }
        assert_eq!(fixture.export_audit_count(), audits_before);
    }
}

#[test]
fn rf015_encrypted_source_still_requires_key_only_when_selected() {
    let fixture = Fixture::new();
    let mut record = fixture.vault.load_object(INSIDE).unwrap().unwrap();
    let mut attachments = load_attachments(&record.properties);
    let plaintext = fixture
        .dir
        .path()
        .join("attachments")
        .join(INSIDE)
        .join(FIRST)
        .join(format!("{FIRST}.bin"));
    let encrypted = plaintext.with_extension("solc");
    crate::attachment_crypto::encrypt_file_stream(&[0x15; 32], &plaintext, &encrypted).unwrap();
    attachments
        .iter_mut()
        .find(|att| att.id == FIRST)
        .unwrap()
        .vault_path = Some(encrypted.to_string_lossy().into_owned());
    record.properties["__attachments"] = serde_json::to_value(&attachments).unwrap();
    fixture.vault.save_object(&record).unwrap();
    fixture
        .export("skip_solc", &fixture.object_scope(), &selected(&[SECOND]))
        .assert_contents(&[INSIDE], &[(INSIDE, SECOND, SECOND_BYTES)]);
    let error = export_vault_with_attachment_scope(
        &fixture.vault,
        &fixture.account_id,
        PASSWORD,
        &fixture.path("selected_solc"),
        &fixture.object_scope(),
        fixture.dir.path(),
        &selected(&[FIRST]),
    )
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        format!(
            "附件 {INSIDE}/{FIRST} 已加密落盘，请使用最新版 GUI 客户端导出（CLI 缺少附件解密密钥）"
        )
    );
    let legacy_error = export_vault(
        &fixture.vault,
        &fixture.account_id,
        PASSWORD,
        &fixture.path("legacy_solc"),
        &fixture.object_scope(),
        fixture.dir.path(),
    )
    .unwrap_err();
    assert_eq!(legacy_error.to_string(), error.to_string());
}

#[test]
fn rf015_explicit_scope_preserves_empty_object_selection_error() {
    let fixture = Fixture::new();
    for attachments in [
        AttachmentExportScope::None,
        selected(&[]),
        selected(&[FIRST]),
        AttachmentExportScope::All,
    ] {
        let path = fixture.path("no_objects");
        let error = export_vault_with_attachment_scope(
            &fixture.vault,
            &fixture.account_id,
            PASSWORD,
            &path,
            &ExportScope::default(),
            fixture.dir.path(),
            &attachments,
        )
        .unwrap_err();
        assert_eq!(error.to_string(), "没有选中任何对象");
        assert!(!path.exists());
    }
}
