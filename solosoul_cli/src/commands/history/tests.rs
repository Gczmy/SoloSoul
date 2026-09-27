//! RF008：真实 CLI 确认流程与共享回滚用例的持久化结果。
use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{json, Value};
use solosoul_core::{VaultService, VaultStore};
use std::sync::Arc;

const OBJECT_ID: &str = "rf008-object";

struct Fixture {
    app: App,
    vault: Arc<VaultStore>,
    db: rusqlite::Connection,
    source: String,
    // Windows 上先释放全部数据库句柄，再清理临时目录。
    _dir: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::TempDir::new().unwrap();
        let service = VaultService::with_base_path(dir.path().to_path_buf());
        service
            .create_account_with_id("acc_rf008_cli", "Test", crate::TEST_PASSWORD, None)
            .unwrap();
        let vault = service.get_vault_store().unwrap();
        let db = rusqlite::Connection::open(vault.base_path().join("vault.db")).unwrap();
        let mut app = App::new(Arc::new(service)).unwrap();
        app.i18n.set_locale("zh-CN");
        let mut object = super::tests::make_obj("acc_rf008_cli".into(), OBJECT_ID, "current");
        object.created_at = "2026-01-01T00:00:00Z".into();
        object.updated_at = object.created_at.clone();
        object.version = 7;
        object.tags_json = vec!["current".into()];
        object.property_labels = Some(json!({"title": "public"}));
        object.template_hash = Some("current-template-hash".into());
        vault.save_object(&object).unwrap();
        let snapshot = json!({
            "name": "历史名称",
            "tags": ["restored", 42, "shared"],
            "properties": {
                "title": "旧值",
                "__fields": {"title": {"name": "历史字段", "type": "text"}}
            },
            "propertyLabels": {"title": "critical"}
        });
        vault
            .save_snapshot_at(
                OBJECT_ID,
                "user_edit",
                &serde_json::to_vec(&snapshot).unwrap(),
                "source",
                1000,
            )
            .unwrap();
        let source = vault.list_snapshots(OBJECT_ID).unwrap()[0]["id"]
            .as_str()
            .unwrap()
            .to_owned();
        Self {
            app,
            vault,
            db,
            source,
            _dir: dir,
        }
    }

    fn confirm(&mut self) {
        rollback(&mut self.app, Some(OBJECT_ID), Some(&self.source)).unwrap();
        assert!(self.app.prompt.is_some());
        assert!(self.app.auto_lock_paused);
        assert!(prompt::handle_key(
            &mut self.app,
            KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE)
        ));
        assert!(self.app.prompt.is_none());
        assert!(!self.app.auto_lock_paused);
    }

    fn rollback_snapshots(&self) -> Vec<Value> {
        self.vault
            .list_snapshots(OBJECT_ID)
            .unwrap()
            .into_iter()
            .filter(|s| s["triggeredBy"] == "rollback")
            .collect()
    }

    fn rollback_audits(&self) -> Vec<solosoul_vault::AuditLogEntry> {
        self.vault
            .list_audit_log(100)
            .unwrap()
            .into_iter()
            .filter(|entry| {
                entry.action_type == "object_rollback"
                    && entry.entity_id.as_deref() == Some(OBJECT_ID)
            })
            .collect()
    }

    fn assert_restored(&self) {
        let object = self.vault.load_object(OBJECT_ID).unwrap().unwrap();
        assert_eq!(object.name, "历史名称");
        assert_eq!(object.properties["title"], "旧值");
        assert_eq!(object.property_labels, Some(json!({"title": "critical"})));
        assert_eq!(object.tags_json, ["restored", "shared"]);
        assert_eq!(object.version, 8);
        assert_eq!(
            object.template_hash.as_deref(),
            Some("current-template-hash")
        );
    }
}

#[test]
fn rf008_cli_confirm_matches_core_and_writes_history_and_audit_once() {
    let _guard = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut cli = Fixture::new();
    let core = Fixture::new();
    let cli_audit_before = cli.vault.list_audit_log(100).unwrap().len();
    let core_audit_before = core.vault.list_audit_log(100).unwrap().len();

    // 默认确认仍为否；取消不得有任何写入，随后真正确认才执行一次。
    rollback(&mut cli.app, Some(OBJECT_ID), Some(&cli.source)).unwrap();
    assert!(prompt::handle_key(
        &mut cli.app,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)
    ));
    assert_eq!(
        cli.vault.load_object(OBJECT_ID).unwrap().unwrap().version,
        7
    );
    assert_eq!(cli.vault.list_snapshots(OBJECT_ID).unwrap().len(), 1);
    assert_eq!(
        cli.vault.list_audit_log(100).unwrap().len(),
        cli_audit_before
    );

    cli.confirm();
    let outcome =
        solosoul_core::objects::rollback_object(&core.vault, OBJECT_ID, &core.source).unwrap();
    assert!(outcome.snapshot_error.is_none());
    assert!(outcome.audit_error.is_none());
    assert!(cli.app.error_message.is_none());
    assert!(cli.app.success_message.is_some());
    cli.assert_restored();
    core.assert_restored();
    assert_eq!(cli.vault.list_snapshots(OBJECT_ID).unwrap().len(), 2);
    assert_eq!(core.vault.list_snapshots(OBJECT_ID).unwrap().len(), 2);
    assert_eq!(
        cli.vault.list_audit_log(100).unwrap().len(),
        cli_audit_before + 1
    );
    assert_eq!(
        core.vault.list_audit_log(100).unwrap().len(),
        core_audit_before + 1
    );

    // 两个独立 Vault 输入相同；只规范化时钟与数据库生成的随机快照 ID。
    let mut cli_record =
        serde_json::to_value(cli.vault.load_object(OBJECT_ID).unwrap().unwrap()).unwrap();
    let mut core_record = serde_json::to_value(&outcome.record).unwrap();
    assert!(cli_record["updated_at"]
        .as_str()
        .is_some_and(|t| chrono::DateTime::parse_from_rfc3339(t).is_ok()));
    assert!(core_record["updated_at"]
        .as_str()
        .is_some_and(|t| chrono::DateTime::parse_from_rfc3339(t).is_ok()));
    cli_record.as_object_mut().unwrap().remove("updated_at");
    core_record.as_object_mut().unwrap().remove("updated_at");
    assert_eq!(cli_record, core_record);
    let cli_snapshots = cli.rollback_snapshots();
    let core_snapshots = core.rollback_snapshots();
    assert_eq!(cli_snapshots.len(), 1);
    assert_eq!(core_snapshots.len(), 1);
    assert_eq!(cli_snapshots[0]["diffSummary"], "diff_rollback");
    assert_eq!(core_snapshots[0]["diffSummary"], "diff_rollback");
    let cli_data = cli
        .vault
        .get_snapshot(cli_snapshots[0]["id"].as_str().unwrap())
        .unwrap()
        .unwrap();
    let core_data = core
        .vault
        .get_snapshot(core_snapshots[0]["id"].as_str().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&cli_data).unwrap(),
        serde_json::from_slice::<Value>(&core_data).unwrap()
    );
    let cli_audits = cli.rollback_audits();
    let core_audits = core.rollback_audits();
    assert_eq!(cli_audits.len(), 1);
    assert_eq!(core_audits.len(), 1);
    let cli_details: Value =
        serde_json::from_str(cli_audits[0].details.as_deref().unwrap()).unwrap();
    let core_details: Value =
        serde_json::from_str(core_audits[0].details.as_deref().unwrap()).unwrap();
    assert_eq!(cli_details["snapshot"], cli.source);
    assert_eq!(core_details["snapshot"], core.source);
    let normalize_audit = |entry: &solosoul_vault::AuditLogEntry| {
        let mut value = serde_json::to_value(entry).unwrap();
        let fields = value.as_object_mut().unwrap();
        fields.remove("id");
        fields.remove("timestamp");
        let mut details: Value = serde_json::from_str(entry.details.as_deref().unwrap()).unwrap();
        details.as_object_mut().unwrap().remove("snapshot");
        fields.insert("details".into(), details);
        value
    };
    assert_eq!(
        normalize_audit(&cli_audits[0]),
        normalize_audit(&core_audits[0])
    );
}

#[test]
fn rf008_cli_snapshot_failure_reports_committed_object_and_still_audits() {
    let _guard = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut fixture = Fixture::new();
    fixture.db.execute_batch("CREATE TRIGGER rf008_reject_snapshot BEFORE INSERT ON object_snapshots WHEN NEW.triggered_by='rollback' BEGIN SELECT RAISE(ABORT, 'rf008 snapshot failure'); END;").unwrap();
    fixture.app.success_message = Some(("旧成功提示".into(), Instant::now()));
    fixture.confirm();
    fixture.assert_restored();
    assert_eq!(fixture.vault.list_snapshots(OBJECT_ID).unwrap().len(), 1);
    assert!(fixture.rollback_snapshots().is_empty());
    assert_eq!(fixture.rollback_audits().len(), 1);
    assert!(fixture.app.success_message.is_none());
    let error = fixture.app.error_message.as_deref().unwrap();
    assert!(error.contains("对象已恢复"), "{error}");
    assert!(error.contains("保存回滚历史失败"), "{error}");
    assert!(!error.contains("记录操作审计失败"), "{error}");
}

#[test]
fn rf008_cli_audit_failure_reports_committed_object_without_success() {
    let _guard = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    for (locale, restored, audit_failed, snapshot_failed) in [
        (
            "zh-CN",
            "对象已恢复",
            "记录操作审计失败",
            "保存回滚历史失败",
        ),
        (
            "en-US",
            "Object restored",
            "Failed to record audit log",
            "Failed to save rollback history",
        ),
    ] {
        let mut fixture = Fixture::new();
        fixture.app.i18n.set_locale(locale);
        fixture.db.execute_batch("CREATE TRIGGER rf008_reject_audit BEFORE INSERT ON audit_log WHEN NEW.action='object_rollback' BEGIN SELECT RAISE(ABORT, 'rf008 audit failure'); END;").unwrap();
        fixture.app.success_message = Some(("旧成功提示".into(), Instant::now()));
        fixture.confirm();
        fixture.assert_restored();
        assert_eq!(fixture.vault.list_snapshots(OBJECT_ID).unwrap().len(), 2);
        assert_eq!(fixture.rollback_snapshots().len(), 1);
        assert_eq!(
            fixture.rollback_snapshots()[0]["diffSummary"],
            "diff_rollback"
        );
        assert!(fixture.rollback_audits().is_empty());
        assert!(fixture.app.success_message.is_none());
        let error = fixture.app.error_message.as_deref().unwrap();
        assert!(error.contains(restored), "{error}");
        assert!(error.contains(audit_failed), "{error}");
        assert!(!error.contains(snapshot_failed), "{error}");
        if locale == "en-US" {
            assert!(!error.contains("对象已恢复"), "{error}");
        }
    }
}
