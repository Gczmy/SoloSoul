//! 同一合成 JSON 由实际 Host serde、生成 TS 和前端流程共同检查。
use super::super::super::backup::BackupInfo;
use super::super::super::export_import::{
    AttachmentExportScope, AttachmentInfo, DocumentSensitivity, ExportDocumentResult,
    ImportOperationSummary,
};
use super::*;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures.json")).unwrap()
}
fn round_trip<T: DeserializeOwned + Serialize>(key: &str) -> T {
    let expected = fixture()[key].clone();
    let value: T = serde_json::from_value(expected.clone()).unwrap();
    assert_eq!(serde_json::to_value(&value).unwrap(), expected, "{key}");
    value
}
#[test]
fn rf304_backup_keeps_snake_case_and_document_results_keep_camel_case() {
    let backup: BackupInfo = round_trip("backup");
    assert_eq!(backup.size_bytes, 12);
    let mut invalid = fixture()["backup"].clone();
    let size = invalid
        .as_object_mut()
        .unwrap()
        .remove("size_bytes")
        .unwrap();
    invalid["sizeBytes"] = size;
    assert!(serde_json::from_value::<BackupInfo>(invalid).is_err());
    let _: ExportDocumentResult = round_trip("document");
    for (level, encoded) in [
        (DocumentSensitivity::None, "none"),
        (DocumentSensitivity::Sensitive, "sensitive"),
        (DocumentSensitivity::Critical, "critical"),
    ] {
        assert_eq!(serde_json::to_value(level).unwrap(), json!(encoded));
    }
}
#[test]
fn rf304_import_missing_null_and_empty_selections_remain_distinct() {
    let all: AdvancedImportRequest = round_trip("requestAll");
    let none: AdvancedImportRequest = round_trip("requestNone");
    assert!(all.selections.is_none() && all.selected_attachment_ids.is_none());
    assert!(none.selections.as_ref().unwrap().is_empty());
    assert!(none.selected_attachment_ids.as_ref().unwrap().is_empty());
    let missing: AdvancedImportRequest =
        serde_json::from_value(fixture()["requestMissing"].clone()).unwrap();
    assert_eq!(missing.locale, "en-US");
    assert!(missing.operation_id.is_none() && missing.object_strategies.is_empty());
    assert!(missing.selections.is_none() && missing.selected_attachment_ids.is_none());
    for (key, bad) in [
        ("locale", json!(null)),
        ("strategy", json!("unknown")),
        ("objectStrategies", json!(null)),
    ] {
        let mut invalid = fixture()["requestAll"].clone();
        invalid[key] = bad;
        assert!(
            serde_json::from_value::<AdvancedImportRequest>(invalid).is_err(),
            "accepted {key}"
        );
    }
}
#[test]
fn rf304_export_default_and_empty_attachment_scope_preserve_existing_protocol() {
    let scope: ExportScope = round_trip("scope");
    assert!(
        matches!(scope.attachment_export_scope(), AttachmentExportScope::Selected(ids) if ids.is_empty())
    );
    let mut old = fixture()["scope"].clone();
    old.as_object_mut().unwrap().remove("includeAll");
    assert!(
        !serde_json::from_value::<ExportScope>(old)
            .unwrap()
            .include_all
    );
    let _: ExportRequest = round_trip("exportRequest");
    let _: ExportEstimate = round_trip("estimate");
    let _: AttachmentInfo = round_trip("attachment");
}
#[test]
fn rf304_import_outcome_preserves_partial_files_nullable_errors_and_recovery_summary() {
    for key in ["complete", "partial", "notCommitted"] {
        let result: ImportResult = round_trip(key);
        assert_eq!(result.is_complete(), key == "complete");
    }
    let result: ImportResult = round_trip("partial");
    assert_eq!(result.attachment_count, 0);
    assert_eq!(result.attachment_files_written, 1);
    assert_eq!(result.failure_stage, Some(ImportStage::Attachments));
    let operation: ImportOperationSummary = round_trip("operation");
    assert!(!operation.outcome.is_complete());
    let mut invalid = fixture()["complete"].clone();
    invalid["status"] = json!("success");
    assert!(serde_json::from_value::<ImportResult>(invalid).is_err());
}
#[test]
fn rf304_preview_and_scope_tree_use_full_vault_summary_without_changing_nulls() {
    let _: ImportPreview = round_trip("preview");
    let _: PageGroup = round_trip("pageGroup");
    let preview: DecryptedImportPreview = round_trip("decrypted");
    assert!(preview.objects[0].template_id.is_none());
    assert!(preview.objects[0].properties.is_null());
    assert_eq!(preview.conflicts[0].kind, ConflictKind::RenamedLocal);
}

#[test]
fn rf318_actual_backend_error_serde_preserves_safe_count_and_outcome_is_separate() {
    use crate::commands::error::{BackendError, BackendErrorCode as C, BackendErrorStage as S};
    let error_fixtures: serde_json::Value =
        serde_json::from_str(include_str!("rf318-fixtures.json")).unwrap();
    for (key, error) in [
        (
            "restoreNone",
            BackendError::new(C::BackupRestoreFailed)
                .at(S::Write)
                .completed_count(0),
        ),
        (
            "restorePartial",
            BackendError::new(C::BackupRestorePartial)
                .at(S::Write)
                .completed_count(1),
        ),
        (
            "alreadyWritten",
            BackendError::new(C::BackupMetadataFailed)
                .at(S::Read)
                .completed_count(2),
        ),
        (
            "badPackage",
            BackendError::new(C::ImportInvalidPackage).at(S::Validate),
        ),
        (
            "password",
            BackendError::new(C::ImportDecryptFailed).at(S::Read),
        ),
        (
            "unconfirmed",
            BackendError::new(C::TransferTaskUnconfirmed).at(S::Task),
        ),
    ] {
        assert_eq!(serde_json::to_value(error).unwrap(), error_fixtures[key]);
    }
    for key in ["complete", "partial", "notCommitted"] {
        let outcome: ImportResult = round_trip(key);
        assert_eq!(
            serde_json::to_value(outcome).unwrap(),
            fixture()[key].clone()
        );
    }
}
