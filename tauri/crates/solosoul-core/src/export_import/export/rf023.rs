//! RF-023：完整 Core 导出服务的真实包和原子发布回归。
use super::*;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

const MASTER: &str = "password123";
const PASSWORD: &str = "RF023-Export1";
const PREFS: &[u8] = b"{\"exportMarker\":\"RF023\"}";
struct Fixture {
    service: VaultService,
    session: VaultSession,
    vault: Arc<VaultStore>,
    account: String,
    dir: tempfile::TempDir,
}
impl Fixture {
    fn new(objects: bool) -> Self {
        let dir = tempfile::TempDir::new().unwrap();
        let service = VaultService::with_base_path(dir.path().join("vault"));
        let account = service.create_account("RF023", MASTER, None).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();
        let session = service.capture_session(&account).unwrap();
        let vault = service.get_vault_store().unwrap();
        for (id, name) in [
            ("rf023-referenced", "Referenced"),
            ("rf023-unused", "Unused"),
        ] {
            vault
                .save_user_template(&UserTemplate {
                    id: id.into(),
                    account_id: account.clone(),
                    name: name.into(),
                    icon_id: None,
                    properties: vec![],
                    category: None,
                    created_at: "2026-10-01T00:00:00Z".into(),
                    updated_at: None,
                    contract_type_id: None,
                })
                .unwrap();
        }
        vault
            .save_profile(&solosoul_vault::Profile::new_with_id(
                &account,
                &account,
                PREFS.to_vec(),
            ))
            .unwrap();
        vault
            .log_structured(
                "rf023-before",
                "object",
                Some("rf023-a"),
                None,
                "user",
                Some("synthetic evidence"),
            )
            .unwrap();
        if objects {
            for (id, page, tag) in [
                ("rf023-a", "identity", "keep"),
                ("rf023-b", "health", "drop"),
            ] {
                let record = ObjectRecord {
                    id: id.into(),
                    account_id: account.clone(),
                    type_id: "note".into(),
                    section_type: page.into(),
                    name: id.into(),
                    properties: json!({"title":id,"__attachments":[]}),
                    tags_json: vec![tag.into()],
                    template_id: Some("rf023-referenced".into()),
                    contract_type_id: Some("rf023-contract".into()),
                    created_at: "2026-10-01T00:00:00Z".into(),
                    updated_at: "2026-10-01T00:00:00Z".into(),
                    version: 1,
                    ..Default::default()
                };
                vault.save_object(&record).unwrap();
                for n in 0..53 {
                    vault
                        .save_snapshot_at(
                            id,
                            "edit",
                            format!("{id}:{n}").as_bytes(),
                            &format!("diff {n}"),
                            1_800_000_000_000 + n,
                        )
                        .unwrap();
                }
            }
        }
        std::fs::create_dir(dir.path().join("exports")).unwrap();
        Self {
            service,
            session,
            vault,
            account,
            dir,
        }
    }
    fn path(&self, name: &str) -> PathBuf {
        self.dir
            .path()
            .join("exports")
            .join(format!("{name}.solosoul"))
    }
    fn request<'a>(&self, scope: &'a EncryptedExportScope) -> EncryptedExportRequest<'a> {
        EncryptedExportRequest {
            scope,
            password: PASSWORD,
            password_hint: &None,
            app_version: "rf023-fixture-version",
        }
    }
    fn advanced(&self, scope: &EncryptedExportScope, path: &Path) {
        assert_eq!(
            execute_encrypted_export(&self.service, &self.session, &self.request(scope), path)
                .unwrap()
                .object_count,
            if scope.include_all { 2 } else { 1 }
        );
    }
    fn exports(&self) -> usize {
        self.service
            .get_vault_store()
            .unwrap()
            .list_audit_log(1000)
            .unwrap()
            .iter()
            .filter(|e| e.action_type == "export_execute")
            .count()
    }
    fn attachment(&self, encrypted: bool) -> PathBuf {
        let file = self
            .service
            .base_path()
            .join("attachments/rf023-a/rf023-att/sample.bin");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        if encrypted {
            let plain = self.dir.path().join("sample.bin");
            std::fs::write(&plain, b"real SOLC source").unwrap();
            crate::attachment_crypto::encrypt_file_stream(
                &self
                    .service
                    .attachment_key_for_session(&self.session)
                    .unwrap(),
                &plain,
                &file,
            )
            .unwrap();
        } else {
            std::fs::write(&file, b"plain attachment").unwrap();
        }
        let mut record = self.vault.load_object("rf023-a").unwrap().unwrap();
        record.properties["__attachments"] = json!([{"id":"rf023-att","objectId":"rf023-a","fileName":"sample.bin","mimeType":"application/octet-stream","sizeBytes":16,"createdAt":"2026-10-01T00:00:00Z","vaultPath":file}]);
        self.vault.save_object(&record).unwrap();
        file
    }
}
fn entry(zip: &mut ZipArchive<File>, name: &str) -> Vec<u8> {
    let mut b = vec![];
    zip.by_name(name).unwrap().read_to_end(&mut b).unwrap();
    b
}
fn decode(path: &Path) -> (Value, Value, BTreeMap<String, Vec<u8>>) {
    let mut zip = ZipArchive::new(File::open(path).unwrap()).unwrap();
    let manifest: Value = serde_json::from_slice(&entry(&mut zip, "manifest.json")).unwrap();
    let salt = hex::decode(manifest["salt_hex"].as_str().unwrap()).unwrap();
    let cfg = kdf_from_manifest_value(manifest.get("kdf"))
        .unwrap()
        .unwrap();
    let key = solosoul_crypto::kdf::derive_export_key(PASSWORD, &salt, &cfg).unwrap();
    let mut plain = vec![];
    solosoul_crypto::cipher::decrypt_chunked_stream(
        &key,
        &mut std::io::Cursor::new(entry(&mut zip, "payload.enc")),
        &mut plain,
    )
    .unwrap();
    let payload = serde_json::from_slice(&plain).unwrap();
    let mut extras = BTreeMap::new();
    for (name, label) in [
        ("preferences.enc", b"solosoul:preferences:v1".as_slice()),
        ("behavioral.enc", b"solosoul:behavioral:v1".as_slice()),
    ] {
        if zip.file_names().any(|n| n == name) {
            let k = solosoul_crypto::hkdf_ext::derive_hkdf_key(&key, &salt, label).unwrap();
            extras.insert(
                name.to_string(),
                solosoul_crypto::cipher::decrypt_from_bytes(&k, &entry(&mut zip, name), None)
                    .unwrap()
                    .to_vec(),
            );
        }
    }
    (manifest, payload, extras)
}
fn directory(path: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    std::fs::read_dir(path)
        .unwrap()
        .map(|e| {
            let p = e.unwrap().path();
            let b = std::fs::read(&p).unwrap();
            (p, b)
        })
        .collect()
}
fn normalize(mut manifest: Value, mut payload: Value) -> (Value, Value) {
    for key in ["salt_hex", "export_time"] {
        manifest.as_object_mut().unwrap().remove(key);
    }
    for key in ["objects", "templates", "snapshots"] {
        if let Some(a) = payload[key].as_array_mut() {
            a.sort_by_key(|v| serde_json::to_string(v).unwrap());
        }
    }
    (manifest, payload)
}

#[test]
fn rf023_advanced_real_package_preserves_templates_latest_fifty_history_hkdf_extras_and_metadata() {
    let f = Fixture::new(true);
    let mut scope = EncryptedExportScope::full_snapshot();
    scope.attachments = AttachmentExportScope::None;
    scope.include_behavioral = true;
    let before = f.vault.list_audit_log(100_000).unwrap();
    let path = f.path("advanced");
    f.advanced(&scope, &path);
    let (m, p, e) = decode(&path);
    assert_eq!(m["export_app_version"], "rf023-fixture-version");
    assert_eq!(m["has_preferences"], true);
    assert_eq!(m["has_behavioral"], true);
    assert_eq!(m["has_attachments"], false);
    assert_eq!(
        p["templates"].as_array().unwrap().len(),
        f.vault.list_user_templates(&f.account).unwrap().len()
    );
    assert!(p["templates"]
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["id"] == "rf023-unused"));
    assert_eq!(p["objects"].as_array().unwrap().len(), 2);
    assert_eq!(p["objects"][0]["contract_type_id"], "rf023-contract");
    assert_eq!(p["objects"][0]["properties"]["__attachments"], json!([]));
    assert_eq!(p["snapshots"].as_array().unwrap().len(), 100);
    for id in ["rf023-a", "rf023-b"] {
        let decoded = p["snapshots"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| s["object_id"] == id)
            .map(|s| {
                assert_eq!(s["triggered_by"], "edit");
                base64::Engine::decode(
                    &base64::engine::general_purpose::STANDARD,
                    s["data"].as_str().unwrap(),
                )
                .unwrap()
            })
            .collect::<BTreeSet<_>>();
        let expected = (3..53)
            .map(|n| format!("{id}:{n}").into_bytes())
            .collect::<BTreeSet<_>>();
        assert_eq!(decoded, expected);
    }
    assert_eq!(e["preferences.enc"], PREFS);
    assert_eq!(
        serde_json::from_slice::<Value>(&e["behavioral.enc"]).unwrap(),
        serde_json::to_value(before).unwrap()
    );
    assert_eq!(f.exports(), 1);
    let estimate = estimate_encrypted_export(&f.vault, &f.account, &scope).unwrap();
    assert_eq!(estimate.object_count, 2);
    assert_eq!(
        estimate.template_count,
        p["templates"].as_array().unwrap().len()
    );
}
#[test]
fn rf023_legacy_public_bool_and_explicit_api_make_equivalent_packages_with_original_profile() {
    let f = Fixture::new(true);
    let scope = ExportScope {
        full: true,
        ..Default::default()
    };
    let a = f.path("legacy-a");
    let b = f.path("legacy-b");
    assert_eq!(
        export_vault(
            &f.vault,
            &f.account,
            PASSWORD,
            &a,
            &scope,
            f.service.base_path()
        )
        .unwrap(),
        2
    );
    assert_eq!(
        export_vault_with_attachment_scope(
            &f.vault,
            &f.account,
            PASSWORD,
            &b,
            &scope,
            f.service.base_path(),
            &AttachmentExportScope::None
        )
        .unwrap(),
        2
    );
    let (m, p, e) = decode(&a);
    let (m2, p2, e2) = decode(&b);
    assert_eq!(normalize(m.clone(), p.clone()), normalize(m2, p2));
    assert_eq!(e, e2);
    assert!(e.is_empty());
    assert_eq!(m["export_scope"], "full");
    assert!(m.get("selected_tags").is_none());
    assert!(m.get("export_app_version").is_none());
    assert!(p.get("snapshots").is_none());
    assert!(p["objects"][0].get("contract_type_id").is_none());
    assert_eq!(p["templates"].as_array().unwrap().len(), 1);
    assert_eq!(p["templates"][0]["id"], "rf023-referenced");
    let destination = Fixture::new(false);
    assert_eq!(
        import_vault(
            &destination.vault,
            &destination.account,
            &a,
            PASSWORD,
            ImportStrategy::Overwrite,
            destination.service.base_path(),
            None
        )
        .unwrap(),
        2
    );
    assert_eq!(
        destination
            .vault
            .load_object("rf023-a")
            .unwrap()
            .unwrap()
            .properties["title"],
        "rf023-a"
    );
}
#[test]
fn rf023_advanced_union_any_tags_and_unselected_corruption_keep_existing_read_boundaries() {
    let f = Fixture::new(true);
    let mut scope = EncryptedExportScope::full_snapshot();
    scope.include_all = false;
    scope.selected_page_ids = vec!["identity".into()];
    scope.selected_object_ids = vec!["rf023-b".into(), "rf023-b".into(), "unknown".into()];
    scope.selected_tags = vec!["keep".into()];
    scope.attachments = AttachmentExportScope::None;
    let path = f.path("partial");
    f.advanced(&scope, &path);
    let (_, p, _) = decode(&path);
    assert_eq!(p["objects"].as_array().unwrap().len(), 1);
    assert_eq!(p["objects"][0]["id"], "rf023-a");
    assert_eq!(p["templates"].as_array().unwrap().len(), 1);
    let db = rusqlite::Connection::open(f.vault.base_path().join("vault.db")).unwrap();
    db.execute("UPDATE objects SET properties=X'00' WHERE id='rf023-a'", [])
        .unwrap();
    scope.selected_page_ids.clear();
    scope.selected_tags.clear();
    scope.selected_object_ids = vec!["rf023-b".into()];
    f.advanced(&scope, &f.path("unselected-bad"));
    scope.include_all = true;
    assert!(execute_encrypted_export(
        &f.service,
        &f.session,
        &f.request(&scope),
        &f.path("full-bad")
    )
    .is_err());
}
#[test]
fn rf023_empty_advanced_full_exports_templates_but_legacy_and_partial_reject_empty() {
    let f = Fixture::new(false);
    let scope = EncryptedExportScope::full_snapshot();
    let path = f.path("empty");
    assert_eq!(
        execute_encrypted_export(&f.service, &f.session, &f.request(&scope), &path)
            .unwrap()
            .object_count,
        0
    );
    let (_, p, _) = decode(&path);
    assert_eq!(p["objects"], json!([]));
    assert!(p["templates"].as_array().unwrap().len() >= 2);
    let mut partial = scope.clone();
    partial.include_all = false;
    assert!(matches!(
        execute_encrypted_export(
            &f.service,
            &f.session,
            &f.request(&partial),
            &f.path("partial")
        ),
        Err(ExportFailure::NoObjectsSelected)
    ));
    assert_eq!(
        export_vault(
            &f.vault,
            &f.account,
            PASSWORD,
            &f.path("legacy"),
            &ExportScope {
                full: true,
                ..Default::default()
            },
            f.service.base_path()
        )
        .unwrap_err()
        .to_string(),
        "没有选中任何对象"
    );
}
#[test]
fn rf023_legacy_solc_writer_failure_preserves_old_backup_audit_sources_and_output_directory() {
    let f = Fixture::new(true);
    let source = f.attachment(true);
    let original = std::fs::read(&source).unwrap();
    let path = f.path("keep-old");
    std::fs::write(&path, b"previous backup sentinel").unwrap();
    let files = directory(path.parent().unwrap());
    let audits = f.exports();
    let error = export_vault(
        &f.vault,
        &f.account,
        PASSWORD,
        &path,
        &ExportScope {
            full: true,
            include_attachments: true,
            ..Default::default()
        },
        f.service.base_path(),
    )
    .unwrap_err();
    assert!(error.to_string().contains("CLI 缺少附件解密密钥"));
    assert_eq!(directory(path.parent().unwrap()), files);
    assert_eq!(f.exports(), audits);
    assert_eq!(std::fs::read(source).unwrap(), original);
    assert_eq!(
        export_vault(
            &f.vault,
            &f.account,
            PASSWORD,
            &path,
            &ExportScope {
                full: true,
                ..Default::default()
            },
            f.service.base_path()
        )
        .unwrap(),
        2
    );
    decode(&path);
    assert_eq!(directory(path.parent().unwrap()).len(), 1);
}
#[test]
fn rf023_prepared_original_session_cannot_publish_after_same_account_reunlock() {
    let f = Fixture::new(true);
    let source = f.attachment(true);
    let bytes = std::fs::read(source).unwrap();
    let scope = EncryptedExportScope::full_snapshot();
    let plan = prepare_session_export(&f.service, &f.session, &f.request(&scope)).unwrap();
    let path = f.path("stale");
    std::fs::write(&path, b"old backup").unwrap();
    let before = directory(path.parent().unwrap());
    f.service.lock();
    f.service.unlock(&f.account, MASTER).unwrap();
    let count = plan.object_count;
    let result = execute_plan(plan, &path, |output| {
        publish_for_session(&f.service, &f.session, output, &path, count)
    });
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("no longer current"));
    assert_eq!(directory(path.parent().unwrap()), before);
    assert_eq!(f.exports(), 0);
    assert_eq!(
        std::fs::read(
            f.service
                .base_path()
                .join("attachments/rf023-a/rf023-att/sample.bin")
        )
        .unwrap(),
        bytes
    );
}
#[test]
fn rf023_optional_read_and_success_audit_failures_remain_best_effort() {
    let f = Fixture::new(true);
    let db = rusqlite::Connection::open(f.vault.base_path().join("vault.db")).unwrap();
    db.execute_batch("DROP TABLE profiles; DROP TABLE audit_log;")
        .unwrap();
    let mut scope = EncryptedExportScope::full_snapshot();
    scope.include_behavioral = true;
    scope.attachments = AttachmentExportScope::None;
    let path = f.path("missing-optional");
    f.advanced(&scope, &path);
    let (m, _, e) = decode(&path);
    assert!(e.is_empty());
    assert_eq!(m["has_preferences"], false);
    assert_eq!(m["has_behavioral"], false);
    assert_eq!(m["extra_files"], json!([]));
}
#[test]
fn rf023_password_domain_errors_and_real_publish_failure_do_not_replace_target() {
    let f = Fixture::new(true);
    let scope = EncryptedExportScope::full_snapshot();
    let path = f.path("target");
    std::fs::write(&path, b"existing backup").unwrap();
    let files = directory(path.parent().unwrap());
    for (password, code) in [("", "PASSWORD_EMPTY"), (MASTER, "SAME_AS_MASTER_PASSWORD")] {
        let req = EncryptedExportRequest {
            password,
            ..f.request(&scope)
        };
        assert_eq!(
            execute_encrypted_export(&f.service, &f.session, &req, &path)
                .unwrap_err()
                .to_string(),
            code
        );
        assert_eq!(directory(path.parent().unwrap()), files);
    }
    let blocked = f.path("directory-target");
    std::fs::create_dir(&blocked).unwrap();
    std::fs::write(blocked.join("keep.txt"), b"keep").unwrap();
    let error =
        execute_encrypted_export(&f.service, &f.session, &f.request(&scope), &blocked).unwrap_err();
    assert!(error.to_string().starts_with("Publish ZIP:"));
    assert_eq!(std::fs::read(blocked.join("keep.txt")).unwrap(), b"keep");
    assert_eq!(
        std::fs::read_dir(path.parent().unwrap()).unwrap().count(),
        2
    );
    assert_eq!(f.exports(), 0);
}
