//! RF307：真实 Vault 失败、安全拒绝载荷、日志脱敏与原写入边界。
use super::super::snapshot::rollback_snapshot_in_vault;
use super::super::{create_object_in_vault, validate_object_input, CreateObjectInput};
use super::setup_vault;
use crate::commands::error::{BackendError, BackendErrorCode as Code, BackendErrorStage as Stage};
use serde_json::{json, Value};
use solosoul_vault::ObjectRecord;
const SECRET: &str = "RF307-SYNTHETIC-SECRET-FIELD-key-path";
fn fixtures() -> Value {
    serde_json::from_str(include_str!("rf307-fixtures.json")).unwrap()
}
fn assert_fixture(name: &str, error: &BackendError) {
    assert_eq!(serde_json::to_value(error).unwrap(), fixtures()[name]);
    let displays = format!(
        "{} {:?} {}",
        error,
        error,
        serde_json::to_string(error).unwrap()
    );
    assert!(!displays.contains(SECRET));
}
fn input(id: &str, properties: Value) -> CreateObjectInput {
    CreateObjectInput {
        account_id: "test_account".into(),
        name: SECRET.into(),
        collection_type: "note".into(),
        properties,
        parent_id: None,
        icon_name: None,
        template_id: None,
        template_type: None,
        id: Some(id.into()),
    }
}
#[test]
fn rf307_input_failure_wire_keeps_fixed_numeric_limits_without_user_values() {
    assert_fixture(
        "emptyName",
        &validate_object_input(" ", &json!({SECRET:SECRET})).unwrap_err(),
    );
    assert_fixture(
        "longName",
        &validate_object_input(&"雪".repeat(201), &json!({})).unwrap_err(),
    );
    assert_fixture(
        "largePayload",
        &validate_object_input(SECRET, &json!({"value":"x".repeat(10*1024*1024)})).unwrap_err(),
    );
    assert!(validate_object_input(&"雪".repeat(200), &json!({SECRET:SECRET})).is_ok());
}
#[test]
fn rf307_duplicate_and_dynamic_validation_reject_before_any_write() {
    let (vault, _dir) = setup_vault();
    let created = create_object_in_vault(
        &vault,
        &input(SECRET, json!({"value":SECRET})),
        "test_account",
        "2001-01-01T00:00:00Z",
    )
    .unwrap();
    let before = serde_json::to_value(vault.list_object_records("test_account").unwrap()).unwrap();
    let audit = serde_json::to_value(vault.list_audit_log(100).unwrap()).unwrap();
    assert_fixture(
        "duplicateId",
        &create_object_in_vault(
            &vault,
            &input(&created.id, json!({})),
            "test_account",
            "2002-01-01T00:00:00Z",
        )
        .unwrap_err(),
    );
    assert_fixture(
        "invalidGroup",
        &create_object_in_vault(
            &vault,
            &input(
                "bad",
                json!({"__fields": {SECRET:{"type":"dynamic_group"}}, SECRET:SECRET}),
            ),
            "test_account",
            "2002-01-01T00:00:00Z",
        )
        .unwrap_err(),
    );
    assert_eq!(
        serde_json::to_value(vault.list_object_records("test_account").unwrap()).unwrap(),
        before
    );
    assert_eq!(
        serde_json::to_value(vault.list_audit_log(100).unwrap()).unwrap(),
        audit
    );
    assert_eq!(vault.list_snapshots(&created.id).unwrap().len(), 1);
}
#[test]
fn rf307_real_rollback_failure_stages_keep_object_history_and_audit_unchanged() {
    let (vault, _dir) = setup_vault();
    vault
        .save_object(&ObjectRecord {
            id: "target".into(),
            account_id: "test_account".into(),
            name: SECRET.into(),
            type_id: "note".into(),
            section_type: "identity".into(),
            sensitivity_level: "internal".into(),
            properties: json!({"value":SECRET}),
            version: 7,
            ..Default::default()
        })
        .unwrap();
    let before = serde_json::to_value(vault.load_object("target").unwrap()).unwrap();
    let foreign = save_snapshot(&vault, "foreign", br#"{}"#);
    let malformed = save_snapshot(&vault, "target", SECRET.as_bytes());
    let labels = save_snapshot(
        &vault,
        "target",
        &serde_json::to_vec(&json!({"name":SECRET,"propertyLabels":SECRET})).unwrap(),
    );
    let histories = vault.list_snapshots("target").unwrap();
    for (name, id) in [
        ("rollbackMismatch", foreign.as_str()),
        ("rollbackMissing", "missing"),
        ("rollbackInvalid", malformed.as_str()),
        ("rollbackLabels", labels.as_str()),
    ] {
        assert_fixture(
            name,
            &rollback_snapshot_in_vault(&vault, id, "target").unwrap_err(),
        );
        assert_eq!(
            serde_json::to_value(vault.load_object("target").unwrap()).unwrap(),
            before
        );
        assert_eq!(vault.list_snapshots("target").unwrap(), histories);
        assert!(vault.list_audit_log(100).unwrap().is_empty());
    }
}
#[derive(Clone)]
struct LogBuffer(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
impl std::io::Write for LogBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
#[test]
fn rf307_all_rollback_stage_codes_and_redacted_diagnostics_ignore_cause_text() {
    use solosoul_core::objects::RollbackErrorStage as R;
    let bytes = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let writer = LogBuffer(bytes.clone());
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_writer(move || writer.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    for (stage, code) in [
        (R::SnapshotOwner, Code::SnapshotReadFailed),
        (R::SnapshotNotFound, Code::SnapshotNotFound),
        (R::Ownership, Code::SnapshotOwnershipMismatch),
        (R::SnapshotRead, Code::SnapshotReadFailed),
        (R::SnapshotParse, Code::SnapshotInvalid),
        (R::ObjectRead, Code::ObjectReadFailed),
        (R::ObjectNotFound, Code::ObjectNotFound),
        (R::Labels, Code::SnapshotInvalid),
        (R::Version, Code::SnapshotRollbackFailed),
        (R::Serialize, Code::SnapshotRollbackFailed),
        (R::ObjectSave, Code::ObjectWriteFailed),
    ] {
        let error = super::super::errors::rollback(solosoul_core::objects::RollbackError {
            stage,
            message: SECRET.into(),
        });
        assert_eq!(error.code, code);
        assert!(!serde_json::to_string(&error).unwrap().contains(SECRET));
    }
    assert_fixture("writeFailed", &super::super::errors::write(SECRET.into()));
    super::super::errors::warn_followup(Stage::SnapshotSave, SECRET.into());
    super::super::errors::warn_followup(Stage::Audit, SECRET.into());
    let logs = String::from_utf8(bytes.lock().unwrap().clone()).unwrap();
    assert!(logs.contains("Object operation failed"));
    assert!(logs.contains("cause_type"));
    assert!(logs.contains("Snapshot save failed"));
    assert!(logs.contains("Audit log write failed"));
    assert!(!logs.contains(SECRET));
}
#[test]
fn rf307_actual_template_read_failure_keeps_old_core_string_api_compatible() {
    let (vault, dir) = setup_vault();
    let db = rusqlite::Connection::open(dir.path().join("vault.db")).unwrap();
    db.execute_batch("DROP TABLE user_templates;").unwrap();
    let mut values = input("read-error", json!({SECRET:SECRET}));
    values.template_id = Some(SECRET.into());
    let error = create_object_in_vault(&vault, &values, "test_account", "2001-01-01T00:00:00Z")
        .unwrap_err();
    assert_fixture("templateReadFailed", &error);
    assert!(vault
        .list_object_records("test_account")
        .unwrap()
        .is_empty());
    let core_input = solosoul_core::objects::CreateRecordInput {
        id: "read-error".into(),
        type_id: "note".into(),
        section_type: "note".into(),
        name: SECRET.into(),
        icon_name: "document".into(),
        parent_id: None,
        properties: json!({}),
        template_id: Some(SECRET.into()),
        template_type: None,
    };
    let typed = solosoul_core::objects::build_create_record_typed(
        &vault,
        "test_account",
        core_input.clone(),
        "now",
    )
    .unwrap_err();
    assert_eq!(
        typed.stage,
        solosoul_core::objects::CreateRecordErrorStage::TemplateRead
    );
    assert_eq!(
        solosoul_core::objects::build_create_record(&vault, "test_account", core_input, "now")
            .unwrap_err(),
        typed.message
    );
}
#[test]
fn rf307_error_json_nullable_and_retryable_are_explicit() {
    assert_fixture("locked", &BackendError::new(Code::VaultLocked));
    assert!(!BackendError::new(Code::ObjectWriteFailed).retryable);
    assert!(!BackendError::new(Code::SnapshotRollbackFailed).retryable);
}

fn save_snapshot(vault: &solosoul_vault::VaultStore, owner: &str, data: &[u8]) -> String {
    let previous = vault.list_snapshots(owner).unwrap();
    vault
        .save_snapshot(owner, "user_edit", data, "seed")
        .unwrap();
    vault
        .list_snapshots(owner)
        .unwrap()
        .into_iter()
        .find(|v| !previous.iter().any(|old| old["id"] == v["id"]))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string()
}
#[test]
fn rf307_actual_storage_write_failure_rejects_without_retry_or_content() {
    let (vault, dir) = setup_vault();
    let db = rusqlite::Connection::open(dir.path().join("vault.db")).unwrap();
    db.execute_batch("CREATE TRIGGER rf307_write_failure BEFORE INSERT ON objects BEGIN SELECT RAISE(ABORT,'RF307-SYNTHETIC-SECRET-FIELD-key-path'); END;").unwrap();
    assert_fixture(
        "writeFailed",
        &create_object_in_vault(
            &vault,
            &input("write-failure", json!({"value":SECRET})),
            "test_account",
            "now",
        )
        .unwrap_err(),
    );
    assert!(vault
        .list_object_records("test_account")
        .unwrap()
        .is_empty());
    assert!(vault.list_audit_log(100).unwrap().is_empty());
}
#[test]
fn rf307_locked_and_maintenance_admission_are_typed_from_actual_state() {
    let dir = tempfile::tempdir().unwrap();
    let service = solosoul_core::VaultService::with_base_path(dir.path().to_path_buf());
    let owner = service.root_owner();
    let service = std::sync::RwLock::new(service);
    let error = super::super::errors::vault_handle_for_service(&service)
        .err()
        .unwrap();
    assert_fixture("locked", &error);
    let _maintenance = solosoul_core::import_activity::begin_owned_root_maintenance(owner).unwrap();
    let error = super::super::errors::vault_handle_for_service(&service)
        .err()
        .unwrap();
    assert_eq!(error.code, Code::VaultBusy);
    assert!(error.retryable);
}
#[test]
fn rf307_real_cancelled_join_error_has_safe_task_stage() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let cause = runtime.block_on(async {
        let task = tokio::spawn(std::future::pending::<()>());
        task.abort();
        task.await.unwrap_err()
    });
    let error = super::super::errors::task(cause);
    assert_eq!(error.code, Code::InternalError);
    assert_eq!(error.safe_details.unwrap().stage, Stage::Task);
    assert!(!error.retryable);
}

#[test]
fn rf307_real_corrupt_object_logs_only_error_category() {
    use solosoul_vault::encryption::{encrypt_text_field, DataEncryptionKey};
    let (vault, dir) = setup_vault();
    vault
        .save_object(&ObjectRecord {
            id: SECRET.into(),
            account_id: "test_account".into(),
            name: SECRET.into(),
            type_id: "note".into(),
            properties: json!({"value": SECRET}),
            ..Default::default()
        })
        .unwrap();
    let encrypted = encrypt_text_field(&DataEncryptionKey::new([0x42; 32]), SECRET).unwrap();
    let db = rusqlite::Connection::open(dir.path().join("vault.db")).unwrap();
    db.execute(
        "UPDATE objects SET properties = ?1 WHERE id = ?2",
        rusqlite::params![encrypted, SECRET],
    )
    .unwrap();
    let bytes = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let writer = LogBuffer(bytes.clone());
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_writer(move || writer.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    let cause = vault.load_object(SECRET).unwrap_err();
    // 内部旧 API 仍可包含诊断，IPC 和日志不能带出对象名称、ID 或原始原因。
    assert!(cause.contains(SECRET));
    let error = super::super::errors::read(cause);
    assert_eq!(error.code, Code::ObjectReadFailed);
    assert!(!serde_json::to_string(&error).unwrap().contains(SECRET));
    let logs = String::from_utf8(bytes.lock().unwrap().clone()).unwrap();
    assert!(logs.contains("error_category"));
    assert!(logs.contains("Object operation failed"));
    assert!(!logs.contains(SECRET));
}
