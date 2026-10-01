//! RF-024：Host wire / 调度入口和独立 Core 请求的真实包行为一致。
use super::rf020::{objects, package, Fixture};
use super::*;
use crate::commands::export_import::import::PreviewJob;
use solosoul_core::export_import::import as core;
fn core_options(strategy: core::AdvancedImportStrategy) -> core::ImportOptions {
    core::ImportOptions {
        strategy,
        locale: "en-US".into(),
        ..Default::default()
    }
}
fn normalized(outcome: &ImportResult) -> serde_json::Value {
    let mut value = serde_json::to_value(outcome).unwrap();
    let map = value.as_object_mut().unwrap();
    map.remove("operationId");
    map.remove("sessionGeneration");
    value
}
fn semantic(f: &Fixture) -> Vec<serde_json::Value> {
    let mut values:Vec<_>=f.vault.list_object_records(&f.account).unwrap().into_iter().map(|r|json!({"name":r.name,"type":r.type_id,"section":r.section_type,"properties":r.properties,"contract":r.contract_type_id,"snapshots":f.vault.list_snapshots(&r.id).unwrap().len()})).collect();
    values.sort_by_key(|v| v["name"].as_str().unwrap().to_owned());
    values
}
#[test]
fn rf024_host_three_strategy_wire_adapter_matches_independent_core_request_and_database() {
    for (wire, domain) in [
        (
            ImportStrategy::SkipExisting,
            core::AdvancedImportStrategy::SkipExisting,
        ),
        (
            ImportStrategy::Overwrite,
            core::AdvancedImportStrategy::Overwrite,
        ),
        (
            ImportStrategy::KeepBoth,
            core::AdvancedImportStrategy::KeepBoth,
        ),
    ] {
        let host = Fixture::new();
        let direct = Fixture::new();
        let path = package(host.dir.path(), objects(), false, false, false);
        let core_path = direct.dir.path().join("direct.solosoul");
        std::fs::copy(&path, &core_path).unwrap();
        let h = {
            let svc = host.service.read().unwrap();
            let session = svc.capture_session(&host.account).unwrap();
            import_execute_resumable_for_session(
                &svc,
                &session,
                path.to_string_lossy().into_owned(),
                Zeroizing::new("export-password".into()),
                wire,
                None,
                None,
                HashMap::new(),
                "en-US",
                None,
                &Uuid::new_v4().to_string(),
                solosoul_vault::ImportSourceKind::Manual,
            )
            .unwrap()
        };
        let d: ImportResult = {
            let svc = direct.service.read().unwrap();
            let session = svc.capture_session(&direct.account).unwrap();
            core::execute_encrypted_import(
                &svc,
                &session,
                core::EncryptedImportRequest {
                    source_path: core_path.to_string_lossy().into_owned(),
                    password: Zeroizing::new("export-password".into()),
                    options: core_options(domain),
                    operation: Some((
                        Uuid::new_v4().to_string(),
                        solosoul_vault::ImportSourceKind::Manual,
                    )),
                },
                None,
            )
            .unwrap()
            .into()
        };
        assert_eq!(normalized(&h), normalized(&d));
        assert_eq!(h.status, ImportStatus::Complete);
        assert_eq!(h.object_count, 2);
        assert_eq!(h.snapshot_count, 2);
        assert_eq!(semantic(&host), semantic(&direct));
        assert_eq!(semantic(&host).len(), 2);
    }
}
#[test]
fn rf024_actual_host_preview_matches_core_dto_errors_and_has_no_database_writes() {
    let f = Fixture::new();
    f.vault
        .save_object(&ObjectRecord {
            id: "rf020-0".into(),
            account_id: f.account.clone(),
            name: "Synthetic".into(),
            type_id: "note".into(),
            section_type: "identity".into(),
            properties: json!({}),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
            version: 1,
            ..Default::default()
        })
        .unwrap();
    let path = package(f.dir.path(), objects(), true, false, false);
    let job = PreviewJob::prepare(
        f.service.clone(),
        path.to_string_lossy().into_owned(),
        Zeroizing::new("export-password".into()),
        |p| Ok(std::path::PathBuf::from(p)),
    )
    .unwrap();
    let h = job.run().unwrap();
    let (d, error) = {
        let svc = f.service.read().unwrap();
        let session = svc.capture_session(&f.account).unwrap();
        let d: DecryptedImportPreview =
            core::decrypt_import_preview(&svc, &session, path.to_str().unwrap(), "export-password")
                .unwrap()
                .into();
        let error =
            core::decrypt_import_preview(&svc, &session, path.to_str().unwrap(), "wrong password")
                .unwrap_err();
        (
            d,
            crate::services::encrypted_import::map_import_failure(error),
        )
    };
    assert_eq!(
        serde_json::to_value(&h).unwrap(),
        serde_json::to_value(&d).unwrap()
    );
    assert_eq!(h.objects.len(), 2);
    assert_eq!(h.attachments.len(), 2);
    assert_eq!(h.conflicts.len(), 1);
    assert_eq!(h.conflicts[0].kind, ConflictKind::Identical);
    let failed = PreviewJob::prepare(
        f.service.clone(),
        path.to_string_lossy().into_owned(),
        Zeroizing::new("wrong password".into()),
        |p| Ok(std::path::PathBuf::from(p)),
    )
    .unwrap()
    .run()
    .unwrap_err();
    assert_eq!(failed, error);
    assert_eq!(failed, import_err("DECRYPT_FAILED"));
    assert_eq!(f.vault.list_object_records(&f.account).unwrap().len(), 1);
}
