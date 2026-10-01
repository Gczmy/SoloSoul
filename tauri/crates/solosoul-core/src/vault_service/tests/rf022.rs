//! RF022 真实 VaultService 换钥/删除与附件清理边界。
use super::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use solosoul_vault::{
    ImportAttachmentOwnerPlan, ImportAttachmentStepPlan, ImportCiphertextProof,
    ImportDatabaseBatch, ImportOperationStart, ImportOwnedAttachmentMarker, ImportSourceKind,
};

const PASSWORD: &str = "rf022-service-password";
fn service() -> (VaultService, TempDir, String) {
    let (svc, dir) = setup_service();
    let id = svc.create_account("RF022 service", PASSWORD, None).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    (svc, dir, id)
}
fn pending(svc: &VaultService, account: &str) -> String {
    let session = svc.capture_session(account).unwrap();
    let vault = session.vault();
    let view = vault.read_import_view(account).unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let start = ImportOperationStart {
        operation_id: id.clone(),
        source_kind: ImportSourceKind::Manual,
        source: solosoul_vault::ImportSourceProof {
            sha256: "a".repeat(64),
            length: 1,
        },
        root_binding: vault.import_root_binding().unwrap(),
        request_fingerprint: "b".repeat(64),
        plan: json!({"nativeRoot":svc.base_path().canonicalize().unwrap().to_str().unwrap()}),
        owners: vec![],
        steps: vec![],
        preferences_required: false,
        source_ready: None,
    };
    svc.with_session(&session, |vault| {
        vault.commit_import_batch_with_operation(
            account,
            &view.revision,
            &ImportDatabaseBatch::default(),
            &start,
        )
    })
    .unwrap();
    id
}
fn config(svc: &VaultService, account: &str) -> Vec<u8> {
    std::fs::read(svc.base_path().join(account).join("config.json")).unwrap()
}
#[test]
fn rf022_pending_real_journal_blocks_key_rotation_and_delete_without_revoking_original_session() {
    let (svc, _dir, account) = service();
    let id = pending(&svc, &account);
    let original = svc.capture_session(&account).unwrap();
    let before = config(&svc, &account);
    assert_eq!(
        svc.change_password(&account, PASSWORD, "newpassword123")
            .unwrap_err(),
        "IMPORT_OPERATIONS_PENDING"
    );
    assert_eq!(
        svc.delete_account(&account).unwrap_err(),
        "IMPORT_OPERATIONS_PENDING"
    );
    let old_key = Zeroizing::new(svc.get_session_key().unwrap().to_vec());
    assert_eq!(
        svc.unlock_with_kdf_upgrade(&account, PASSWORD, &old_key, original.generation())
            .unwrap_err(),
        "IMPORT_OPERATIONS_PENDING"
    );
    assert_eq!(config(&svc, &account), before);
    svc.with_session(&original, |vault| {
        assert!(vault.load_import_operation(&account, &id)?.is_some());
        Ok(())
    })
    .unwrap();
    svc.lock();
    svc.unlock(&account, PASSWORD).unwrap();
    assert!(svc
        .get_vault_store()
        .unwrap()
        .load_import_operation(&account, &id)
        .unwrap()
        .is_some());
}
#[test]
fn rf022_actual_precommit_worker_blocks_password_and_account_maintenance_until_worker_exits() {
    let (svc, _dir, account) = service();
    let original = svc.capture_session(&account).unwrap();
    let before = config(&svc, &account);
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let root = svc.base_path().clone();
    let worker = std::thread::spawn(move || {
        let _activity = crate::import_activity::begin_import_activity(&root).unwrap();
        ready_tx.send(()).unwrap();
        release_rx.recv().unwrap();
    });
    ready_rx.recv().unwrap();
    assert_eq!(
        svc.change_password(&account, PASSWORD, "newpassword123")
            .unwrap_err(),
        "IMPORT_OPERATIONS_ACTIVE"
    );
    assert_eq!(
        svc.delete_account(&account).unwrap_err(),
        "IMPORT_OPERATIONS_ACTIVE"
    );
    assert_eq!(config(&svc, &account), before);
    svc.with_session(&original, |_| Ok(())).unwrap();
    release_tx.send(()).unwrap();
    worker.join().unwrap();
    svc.change_password(&account, PASSWORD, "newpassword123")
        .unwrap();
    svc.lock();
    svc.unlock(&account, "newpassword123").unwrap();
}

struct Published {
    operation: String,
    lease: solosoul_vault::ImportOperationLease,
    directory: std::path::PathBuf,
    file: std::path::PathBuf,
    marker: ImportOwnedAttachmentMarker,
}
fn publish(svc: &VaultService, account: &str) -> Published {
    use solosoul_vault::{ImportHistoryChange, ImportObjectWrite};
    let session = svc.capture_session(account).unwrap();
    let vault = session.vault();
    let view = vault.read_import_view(account).unwrap();
    let object_id = "rf022_service_owner";
    let record = solosoul_vault::ObjectRecord {
        id: object_id.into(),
        account_id: account.into(),
        name: "Imported".into(),
        type_id: "note".into(),
        section_type: "identity".into(),
        icon_name: "document".into(),
        properties: json!({"body":"local value"}),
        sensitivity_level: "internal".into(),
        created_at: "2026-10-01T00:00:00Z".into(),
        updated_at: "2026-10-01T00:00:00Z".into(),
        version: 1,
        ..Default::default()
    };
    let batch = ImportDatabaseBatch {
        templates: vec![],
        objects: vec![ImportObjectWrite {
            record,
            history: ImportHistoryChange::Keep,
        }],
    };
    let id = uuid::Uuid::new_v4().to_string();
    let attachment_id = uuid::Uuid::new_v4().to_string();
    let directory = svc
        .base_path()
        .join("attachments")
        .join(object_id)
        .join(&attachment_id);
    let file = directory.join("sample.txt");
    let metadata = json!({"id":attachment_id,"objectId":object_id,"fileName":"sample.txt","mimeType":"text/plain",
        "sizeBytes":12,"createdAt":"2026-10-01T00:00:00Z","deletedAt":null,"srcPath":file.to_string_lossy(),"vaultPath":file.to_string_lossy(),"description":null,"tags":[]});
    let root_binding = vault.import_root_binding().unwrap();
    let start = ImportOperationStart {
        operation_id: id.clone(),
        source_kind: ImportSourceKind::Manual,
        source: solosoul_vault::ImportSourceProof {
            sha256: "a".repeat(64),
            length: 1,
        },
        root_binding: root_binding.clone(),
        request_fingerprint: "b".repeat(64),
        plan: json!({"nativeRoot":svc.base_path().canonicalize().unwrap().to_str().unwrap()}),
        owners: vec![ImportAttachmentOwnerPlan {
            owner_id: object_id.into(),
            expected_attachments: None,
        }],
        steps: vec![ImportAttachmentStepPlan {
            entry_ordinal: 2,
            source_object_id: object_id.into(),
            source_attachment_id: "source_att".into(),
            owner_id: object_id.into(),
            attachment_id: attachment_id.clone(),
            safe_file_name: "sample.txt".into(),
            metadata,
            initial_staged_proof: None,
        }],
        preferences_required: false,
        source_ready: None,
    };
    svc.with_session(&session, |vault| {
        vault.commit_import_batch_with_operation(account, &view.revision, &batch, &start)
    })
    .unwrap();
    let lease = vault
        .claim_import_operation(account, &id, &root_binding)
        .unwrap();
    let plain = tempfile::NamedTempFile::new_in(svc.base_path()).unwrap();
    std::fs::write(plain.path(), b"actual bytes").unwrap();
    let encrypted = tempfile::NamedTempFile::new_in(svc.base_path()).unwrap();
    crate::attachment_crypto::encrypt_file_stream(
        &svc.attachment_key_for_session(&session).unwrap(),
        plain.path(),
        encrypted.path(),
    )
    .unwrap();
    let ciphertext = std::fs::read(encrypted.path()).unwrap();
    let proof = ImportCiphertextProof {
        sha256: hex::encode(Sha256::digest(&ciphertext)),
        length: ciphertext.len() as u64,
        plaintext_length: 12,
        stage_epoch: lease.epoch(),
    };
    vault
        .confirm_import_attachment_staged(&lease, 2, &proof)
        .unwrap();
    let marker = ImportOwnedAttachmentMarker {
        account_id: account.into(),
        operation_id: id.clone(),
        entry_ordinal: 2,
        owner_id: object_id.into(),
        attachment_id,
        root_binding,
    };
    vault
        .publish_import_attachment(&lease, 2, |_| {
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(&file, &ciphertext).unwrap();
            std::fs::write(
                directory.join(crate::export_import::operation::IMPORT_OWNER_MARKER),
                serde_json::to_vec(&marker).unwrap(),
            )
            .unwrap();
            std::fs::write(
                directory
                    .parent()
                    .unwrap()
                    .join(format!("{}.import-owner", marker.attachment_id)),
                serde_json::to_vec(&marker).unwrap(),
            )
            .unwrap();
            Ok(())
        })
        .unwrap();
    Published {
        operation: id,
        lease,
        directory,
        file,
        marker,
    }
}
#[test]
fn rf022_completed_global_import_rekeys_real_ciphertext_and_encrypted_journal_preserving_markers() {
    let (svc, _dir, account) = service();
    let published = publish(&svc, &account);
    let vault = svc.get_vault_store().unwrap();
    vault
        .commit_import_attachment_metadata(&published.lease, &published.marker.owner_id)
        .unwrap();
    vault.complete_import_operation(&published.lease).unwrap();
    let marker_path = published
        .directory
        .join(crate::export_import::operation::IMPORT_OWNER_MARKER);
    let marker_before = std::fs::read(&marker_path).unwrap();
    let encrypted_before = std::fs::read(&published.file).unwrap();
    svc.change_password(&account, PASSWORD, "newpassword123")
        .unwrap();
    assert_eq!(std::fs::read(&marker_path).unwrap(), marker_before);
    assert_ne!(std::fs::read(&published.file).unwrap(), encrypted_before);
    let session = svc.capture_session(&account).unwrap();
    let output = svc.base_path().join("rf022-rekey-output.txt");
    crate::attachment_crypto::copy_decrypt_file(
        &svc.attachment_key_for_session(&session).unwrap(),
        &published.file,
        &output,
    )
    .unwrap();
    assert_eq!(std::fs::read(&output).unwrap(), b"actual bytes");
    svc.lock();
    let fresh = VaultService::with_base_path(svc.base_path().clone());
    fresh.unlock(&account, "newpassword123").unwrap();
    let record = fresh
        .get_vault_store()
        .unwrap()
        .load_import_operation(&account, &published.operation)
        .unwrap()
        .unwrap();
    assert_eq!(record.phase, solosoul_vault::ImportOperationPhase::Complete);
    assert_eq!(record.attachment_count, 1);
    assert_eq!(
        crate::objects::cleanup_orphan_attachments(
            &fresh.get_vault_store().unwrap(),
            &account,
            fresh.base_path()
        )
        .unwrap()
        .0,
        0
    );
    assert!(published.file.exists());
}
#[test]
fn rf022_orphan_scanner_preserves_pending_and_bad_markers_and_deletes_only_abandoned_unreferenced_owned_directory(
) {
    let (svc, _dir, account) = service();
    let published = publish(&svc, &account);
    let vault = svc.get_vault_store().unwrap();
    let bad = svc
        .base_path()
        .join("attachments/unknown_owner/unknown_attachment");
    std::fs::create_dir_all(&bad).unwrap();
    std::fs::write(
        bad.join(crate::export_import::operation::IMPORT_OWNER_MARKER),
        b"invalid json",
    )
    .unwrap();
    assert_eq!(
        crate::objects::cleanup_orphan_attachments(&vault, &account, svc.base_path())
            .unwrap()
            .0,
        0
    );
    assert!(published.file.exists());
    assert!(bad.exists());
    vault.abandon_import_operation(&published.lease).unwrap();
    assert_eq!(
        crate::objects::cleanup_orphan_attachments(&vault, &account, svc.base_path())
            .unwrap()
            .0,
        1
    );
    assert!(!published.directory.exists());
    assert!(bad.exists());
}

#[test]
fn rf022_abandoned_activated_global_attachment_rekeys_and_keeps_its_owner_marker() {
    let (svc, _dir, account) = service();
    let published = publish(&svc, &account);
    let vault = svc.get_vault_store().unwrap();
    vault
        .commit_import_attachment_metadata(&published.lease, &published.marker.owner_id)
        .unwrap();
    vault.abandon_import_operation(&published.lease).unwrap();
    let old = svc
        .attachment_key_for_session(&svc.capture_session(&account).unwrap())
        .unwrap();
    let marker_path = published
        .directory
        .join(crate::export_import::operation::IMPORT_OWNER_MARKER);
    let marker_before = std::fs::read(&marker_path).unwrap();
    svc.change_password(&account, PASSWORD, "newpassword123")
        .unwrap();
    let new = svc
        .attachment_key_for_session(&svc.capture_session(&account).unwrap())
        .unwrap();
    let output = svc.base_path().join("rf022-abandoned-rekey-output.txt");
    assert!(crate::attachment_crypto::copy_decrypt_file(&old, &published.file, &output).is_err());
    crate::attachment_crypto::copy_decrypt_file(&new, &published.file, &output).unwrap();
    assert_eq!(std::fs::read(output).unwrap(), b"actual bytes");
    assert_eq!(std::fs::read(marker_path).unwrap(), marker_before);
    assert_eq!(
        svc.get_vault_store()
            .unwrap()
            .load_import_operation(&account, &published.operation)
            .unwrap()
            .unwrap()
            .phase,
        solosoul_vault::ImportOperationPhase::Abandoned
    );
}
