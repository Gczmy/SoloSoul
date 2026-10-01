//! RF-023：Host 与 Core 实际包兼容、实际 worker 的 owner/activity 生命周期。
use super::rf020::Fixture;
use super::*;
use crate::commands::export_import::export::{run_export_job, ExportJob};
use solosoul_core::export_import::export::{
    execute_encrypted_export, EncryptedExportRequest, EncryptedExportScope,
};
use std::path::Path;
use std::time::Duration;
const PASSWORD: &str = "RF023-Export1";
fn request(path: &Path, full: bool) -> ExportRequest {
    ExportRequest {
        scope: ExportScope {
            selected_page_ids: vec![],
            selected_object_ids: if full {
                vec![]
            } else {
                vec!["rf023-object".into()]
            },
            selected_tags: vec![],
            include_attachments: false,
            selected_attachment_ids: vec![],
            include_preferences: true,
            include_behavioral: false,
            include_all: full,
        },
        password: PASSWORD.into(),
        password_hint: Some("RF023 hint".into()),
        save_path: path.to_string_lossy().into_owned(),
    }
}
fn seed(f: &Fixture) {
    f.vault
        .save_object(&ObjectRecord {
            id: "rf023-object".into(),
            account_id: f.account.clone(),
            name: "RF023".into(),
            type_id: "note".into(),
            section_type: "identity".into(),
            properties: json!({"title":"actual fixture"}),
            contract_type_id: Some("rf023-contract".into()),
            created_at: "2026-10-01T00:00:00Z".into(),
            updated_at: "2026-10-01T00:00:00Z".into(),
            version: 1,
            ..Default::default()
        })
        .unwrap();
}
fn decode(path: &Path) -> (serde_json::Value, serde_json::Value, Option<Vec<u8>>) {
    let mut zip = ZipArchive::new(File::open(path).unwrap()).unwrap();
    let mut raw = vec![];
    zip.by_name("manifest.json")
        .unwrap()
        .read_to_end(&mut raw)
        .unwrap();
    let mut manifest: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    let salt = hex::decode(manifest["salt_hex"].as_str().unwrap()).unwrap();
    let cfg = solosoul_core::export_import::kdf_from_manifest_value(manifest.get("kdf"))
        .unwrap()
        .unwrap();
    let key = solosoul_crypto::kdf::derive_export_key(PASSWORD, &salt, &cfg).unwrap();
    raw.clear();
    zip.by_name("payload.enc")
        .unwrap()
        .read_to_end(&mut raw)
        .unwrap();
    let mut plain = vec![];
    solosoul_crypto::cipher::decrypt_chunked_stream(
        &key,
        &mut std::io::Cursor::new(raw),
        &mut plain,
    )
    .unwrap();
    let payload = serde_json::from_slice(&plain).unwrap();
    let prefs = if manifest["has_preferences"] == true {
        let mut data = vec![];
        zip.by_name("preferences.enc")
            .unwrap()
            .read_to_end(&mut data)
            .unwrap();
        let k = solosoul_crypto::hkdf_ext::derive_hkdf_key(&key, &salt, b"solosoul:preferences:v1")
            .unwrap();
        Some(
            solosoul_crypto::cipher::decrypt_from_bytes(&k, &data, None)
                .unwrap()
                .to_vec(),
        )
    } else {
        None
    };
    manifest.as_object_mut().unwrap().remove("salt_hex");
    manifest.as_object_mut().unwrap().remove("export_time");
    (manifest, payload, prefs)
}
#[test]
fn rf023_host_compatibility_facade_and_core_produce_semantically_equal_real_packages() {
    let f = Fixture::new();
    seed(&f);
    let svc = f.service.read().unwrap();
    let session = svc.capture_session(&f.account).unwrap();
    for full in [false, true] {
        let host = f.dir.path().join(format!("host-{full}.solosoul"));
        let core = f.dir.path().join(format!("core-{full}.solosoul"));
        let req = request(&host, full);
        execute_export_core(&svc, &f.account, &req, host.to_str().unwrap()).unwrap();
        // 直接构造业务请求，独立于 Host 的 wire 转换。
        let scope = EncryptedExportScope {
            selected_page_ids: vec![],
            selected_object_ids: if full {
                vec![]
            } else {
                vec!["rf023-object".into()]
            },
            selected_tags: vec![],
            include_all: full,
            attachments: AttachmentExportScope::None,
            include_preferences: true,
            include_behavioral: false,
        };
        let domain = EncryptedExportRequest {
            scope: &scope,
            password: PASSWORD,
            password_hint: &req.password_hint,
            app_version: env!("CARGO_PKG_VERSION"),
        };
        execute_encrypted_export(&svc, &session, &domain, &core).unwrap();
        let actual = decode(&host);
        assert_eq!(actual, decode(&core));
        assert_eq!(
            actual.1["objects"][0]["properties"]["title"],
            "actual fixture"
        );
        assert_eq!(actual.1["objects"][0]["contract_type_id"], "rf023-contract");
        assert_eq!(actual.0["password_hint"], "RF023 hint");
        assert!(actual.1["snapshots"].is_array());
    }
}
#[test]
fn rf023_gui_job_cannot_enter_pending_maintenance() {
    let f = Fixture::new();
    seed(&f);
    let owner = f.service.read().unwrap().root_owner();
    let maintenance = solosoul_core::import_activity::begin_owned_root_maintenance(owner).unwrap();
    let path = f.dir.path().join("blocked.solosoul");
    let req = request(&path, false);
    let result = ExportJob::prepare(f.service.clone(), &f.account, req, |path| {
        Ok(path.to_string())
    });
    let job = result.unwrap();
    assert_eq!(job.run().unwrap_err(), "IMPORT_DIRECTORY_BUSY");
    assert!(!path.exists());
    drop(maintenance);
    let job = ExportJob::prepare(
        f.service.clone(),
        &f.account,
        request(&path, false),
        |path| Ok(path.to_string()),
    )
    .unwrap();
    job.run().unwrap();
    decode(&path);
}
#[test]
fn rf023_cancelled_gui_awaiter_keeps_actual_worker_activity_until_job_finishes() {
    let f = Fixture::new();
    seed(&f);
    let path = f.dir.path().join("worker.solosoul");
    let owner = f.service.read().unwrap().root_owner();
    let job = ExportJob::prepare(
        f.service.clone(),
        &f.account,
        request(&path, false),
        |path| Ok(path.to_string()),
    )
    .unwrap();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let (finished_tx, finished_rx) = std::sync::mpsc::channel();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let waiter = tokio::spawn(run_export_job(move || {
            let result = job.run_with_activity(|| {
                entered_tx.send(()).unwrap();
                release_rx.recv().unwrap();
            });
            finished_tx.send(()).unwrap();
            result
        }));
        entered_rx.await.unwrap();
        assert!(
            solosoul_core::import_activity::begin_owned_root_maintenance(owner.clone()).is_err()
        );
        waiter.abort();
        assert!(waiter.await.unwrap_err().is_cancelled());
        assert!(
            solosoul_core::import_activity::begin_owned_root_maintenance(owner.clone()).is_err()
        );
        release_tx.send(()).unwrap();
        finished_rx.recv_timeout(Duration::from_secs(30)).unwrap();
    });
    let maintenance = solosoul_core::import_activity::begin_owned_root_maintenance(owner).unwrap();
    drop(maintenance);
    let (_, payload, _) = decode(&path);
    assert_eq!(payload["objects"][0]["id"], "rf023-object");
}
