//! RF022：真实 CLI handler / masked prompt / journal / reopen / SQLite 回归；未执行候选。
use super::*;
use crate::app::AppPhase;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rusqlite::{types::ValueRef, Connection};
use serde_json::json;
use solosoul_core::VaultService;
use solosoul_vault::{ImportOperationPhase, ImportOperationRecord, ObjectRecord};
use std::collections::BTreeMap;
use std::sync::Arc;
use tempfile::TempDir;

const ACCOUNT: &str = "acc_rf022_cli_target";
const OTHER: &str = "acc_rf022_cli_other";
const OWNER: &str = "rf022_cli_owner";
const NOW: &str = "2026-10-01T00:00:00Z";

type State = BTreeMap<String, Vec<Vec<Vec<u8>>>>;
struct Fixture {
    app: App,
    package: PathBuf,
    _dir: TempDir,
}
impl Fixture {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let source = VaultService::with_base_path(dir.path().join("source"));
        let source_account = "acc_rf022_cli_source";
        source
            .create_account_with_id(source_account, "RF022 source", crate::TEST_PASSWORD, None)
            .unwrap();
        source.unlock(source_account, crate::TEST_PASSWORD).unwrap();
        let vault = source.get_vault_store().unwrap();
        let mut attachments = Vec::new();
        for attachment in ["att_a", "att_b"] {
            let directory = source
                .base_path()
                .join("attachments")
                .join(OWNER)
                .join(attachment);
            std::fs::create_dir_all(&directory).unwrap();
            let file = directory.join(format!("{attachment}.txt"));
            // 合成旧明文源，公开 export_vault 按现有兼容规则生成真实加密 ZIP。
            // 目标生产导入必须使用当前账户的 at-rest key；后续断言解密真实文件。
            let bytes = format!("synthetic content of {attachment}");
            std::fs::write(&file, &bytes).unwrap();
            attachments.push(
                json!({"id":attachment,"objectId":OWNER,"fileName":format!("{attachment}.txt"),
                "mimeType":"text/plain","sizeBytes":bytes.len(),"createdAt":NOW,
                "vaultPath":file.to_string_lossy(),"srcPath":file.to_string_lossy(),"tags":[]}),
            );
        }
        vault
            .save_object(&ObjectRecord {
                id: OWNER.into(),
                account_id: source_account.into(),
                type_id: "note".into(),
                section_type: "identity".into(),
                name: "CLI synthetic source".into(),
                icon_name: "document".into(),
                properties: json!({"body":"CLI source body","__attachments":attachments}),
                sensitivity_level: "internal".into(),
                created_at: NOW.into(),
                updated_at: NOW.into(),
                ..Default::default()
            })
            .unwrap();
        let package = dir.path().join("真实导入包.solosoul");
        assert_eq!(
            export_vault(
                &vault,
                source_account,
                crate::TEST_EXPORT_PASSWORD,
                &package,
                &ExportScope {
                    full: true,
                    include_attachments: true,
                    ..Default::default()
                },
                source.base_path()
            )
            .unwrap(),
            1
        );
        source.lock();
        let service = Arc::new(VaultService::with_base_path(dir.path().join("target")));
        service
            .create_account_with_id(ACCOUNT, "RF022 target", crate::TEST_PASSWORD, None)
            .unwrap();
        service.unlock(ACCOUNT, crate::TEST_PASSWORD).unwrap();
        let mut app = App::new(service).unwrap();
        app.i18n.set_locale("zh-CN");
        app.phase = AppPhase::Home {
            account_id: ACCOUNT.into(),
        };
        Self {
            app,
            package,
            _dir: dir,
        }
    }
    fn db(&self) -> Connection {
        Connection::open(
            self.app
                .vault_service
                .base_path()
                .join(ACCOUNT)
                .join("vault.db"),
        )
        .unwrap()
    }
    fn state(&self) -> State {
        db_state(&self.db(), true)
    }
    fn business(&self) -> State {
        db_state(&self.db(), false)
    }
    fn command(&mut self, args: &[&str]) {
        handle(&mut self.app, args).unwrap();
    }
    fn start(&mut self) {
        self.app.error_message = None;
        self.app.success_message = None;
        let path = self.package.to_string_lossy().into_owned();
        self.command(&["/import", &path]);
        assert!(matches!(
            self.app.prompt.as_ref().map(|p| &p.spec),
            Some(PromptSpec::Text { mask: true, .. })
        ));
        assert!(self.app.auto_lock_paused);
    }
    fn key(&mut self, code: KeyCode) {
        assert!(prompt::handle_key(
            &mut self.app,
            KeyEvent::new(code, KeyModifiers::NONE)
        ));
    }
    fn text(&mut self, text: &str) {
        for character in text.chars() {
            self.key(KeyCode::Char(character));
        }
        self.key(KeyCode::Enter);
    }
    fn confirm(&mut self) {
        self.text(crate::TEST_EXPORT_PASSWORD);
        assert!(self.app.prompt.is_none());
        assert!(!self.app.auto_lock_paused);
    }
    fn pending(&self) -> ImportOperationRecord {
        let operations = self
            .app
            .vault_service
            .get_vault_store()
            .unwrap()
            .list_import_operations(ACCOUNT)
            .unwrap();
        assert_eq!(operations.len(), 1);
        operations.into_iter().next().unwrap()
    }
    fn install_failure(&self, phase: &str) {
        let sql=match phase {
            "metadata"=>"CREATE TRIGGER rf022_cli_failure BEFORE UPDATE OF properties ON objects WHEN NEW.id='rf022_cli_owner' BEGIN SELECT RAISE(ABORT,'synthetic metadata failure'); END;",
            "stage"=>"CREATE TRIGGER rf022_cli_failure BEFORE UPDATE OF phase ON import_attachment_steps WHEN NEW.phase='staged' BEGIN SELECT RAISE(ABORT,'synthetic stage confirmation failure'); END;",
            "published"=>"CREATE TRIGGER rf022_cli_failure BEFORE UPDATE OF phase ON import_attachment_steps WHEN NEW.phase='published' BEGIN SELECT RAISE(ABORT,'synthetic publication confirmation failure'); END;",
            _=>panic!("unknown fault"),
        };
        self.db().execute_batch(sql).unwrap();
    }
    fn remove_failure(&self) {
        self.db()
            .execute_batch("DROP TRIGGER rf022_cli_failure;")
            .unwrap();
    }
    fn partial(&mut self, phase: &str) -> ImportOperationRecord {
        self.install_failure(phase);
        self.start();
        self.confirm();
        assert!(self.app.success_message.is_none());
        let operation = self.pending();
        let text = self.app.error_message.as_ref().unwrap();
        assert!(text.contains(&operation.start.operation_id));
        assert!(text.contains("--resume"));
        assert_eq!(operation.start.plan["cliObjectWriteCount"], 1);
        operation
    }
    fn reopen(&mut self) {
        let base = self.app.vault_service.base_path().to_path_buf();
        self.app.vault_service.lock();
        // 保留旧 App/句柄期间的重开必须显式复用 owner，不隐式再次获取同根。
        let owner = self.app.vault_service.root_owner();
        let file_system = Arc::new(solosoul_core::LocalVaultFileSystem::new(base));
        let service = Arc::new(VaultService::try_with_root_owner(owner, file_system).unwrap());
        service
            .unlock_secure(ACCOUNT, &Zeroizing::new(crate::TEST_PASSWORD.into()))
            .unwrap();
        let mut app = App::new(service).unwrap();
        app.i18n.set_locale("zh-CN");
        app.phase = AppPhase::Home {
            account_id: ACCOUNT.into(),
        };
        self.app = app;
    }
    fn assert_complete(&self, id: &str) {
        assert!(
            self.app.error_message.is_none(),
            "{:?}",
            self.app.error_message
        );
        assert!(self.app.prompt.is_none());
        let message = &self.app.success_message.as_ref().unwrap().0;
        assert!(message.contains("成功导入 1 个对象"));
        assert!(message.contains(id));
        let vault = self.app.vault_service.get_vault_store().unwrap();
        let complete = vault.load_import_operation(ACCOUNT, id).unwrap().unwrap();
        assert_eq!(complete.phase, ImportOperationPhase::Complete);
        assert_eq!(complete.attachment_count, 2);
        assert!(vault.list_import_operations(ACCOUNT).unwrap().is_empty());
        let session = self.app.vault_service.capture_session(ACCOUNT).unwrap();
        let key = self
            .app
            .vault_service
            .attachment_key_for_session(&session)
            .unwrap();
        for step in complete.steps {
            let file = self
                .app
                .vault_service
                .base_path()
                .join("attachments")
                .join(OWNER)
                .join(&step.plan.attachment_id)
                .join(&step.plan.safe_file_name);
            assert!(solosoul_core::attachment_crypto::is_encrypted_file(&file));
            assert_eq!(
                solosoul_core::attachment_crypto::read_file_decrypted(&key, &file, 1024).unwrap(),
                format!("synthetic content of {}", step.plan.source_attachment_id).as_bytes()
            );
        }
    }
}

fn db_state(db: &Connection, journal: bool) -> State {
    let mut tables = vec!["objects", "user_templates", "object_snapshots", "sync_hlc"];
    if journal {
        tables.extend(["import_operations", "import_attachment_steps"]);
    }
    let mut result = BTreeMap::new();
    for table in tables {
        let mut query = db.prepare(&format!("SELECT * FROM {table}")).unwrap();
        let columns = query.column_count();
        let mut cursor = query.query([]).unwrap();
        let mut rows = Vec::new();
        while let Some(row) = cursor.next().unwrap() {
            rows.push(
                (0..columns)
                    .map(|index| match row.get_ref(index).unwrap() {
                        ValueRef::Null => vec![b'N'],
                        ValueRef::Integer(value) => {
                            [vec![b'I'], value.to_le_bytes().to_vec()].concat()
                        }
                        ValueRef::Real(value) => {
                            [vec![b'R'], value.to_bits().to_le_bytes().to_vec()].concat()
                        }
                        ValueRef::Text(value) => [vec![b'T'], value.to_vec()].concat(),
                        ValueRef::Blob(value) => [vec![b'B'], value.to_vec()].concat(),
                    })
                    .collect::<Vec<_>>(),
            );
        }
        rows.sort();
        result.insert(table.into(), rows);
    }
    result
}
fn lock() -> std::sync::MutexGuard<'static, ()> {
    crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}

#[test]
fn rf022_cli_fresh_actual_masked_prompt_completes_and_displays_operation_id() {
    let _guard = lock();
    let mut f = Fixture::new();
    f.start();
    f.confirm();
    let id = f
        .db()
        .query_row("SELECT operation_id FROM import_operations", [], |row| {
            row.get::<_, String>(0)
        })
        .unwrap();
    f.assert_complete(&id);
    assert!(uuid::Uuid::parse_str(&id).is_ok());
}

#[test]
fn rf022_cli_pending_reopens_ready_task_and_resumes_deleted_source_without_password() {
    let _guard = lock();
    let mut f = Fixture::new();
    let operation = f.partial("metadata");
    assert_eq!(operation.attachment_count, 0);
    assert_eq!(operation.steps.len(), 2);
    assert!(operation
        .steps
        .iter()
        .all(|step| step.phase == solosoul_vault::ImportAttachmentPhase::Published));
    assert!(
        !import_credential_requirements(&operation)
            .unwrap()
            .password_required
    );
    f.command(&["/import", "--pending"]);
    assert!(f
        .app
        .info_message
        .as_ref()
        .unwrap()
        .contains(&operation.start.operation_id));
    let ids = operation
        .steps
        .iter()
        .map(|step| step.plan.attachment_id.clone())
        .collect::<Vec<_>>();
    f.remove_failure();
    f.reopen();
    std::fs::remove_file(&f.package).unwrap();
    let vault = f.app.vault_service.get_vault_store().unwrap();
    let mut record = vault.load_object(OWNER).unwrap().unwrap();
    record.name = "unrelated local edit must survive resume".into();
    vault.save_object(&record).unwrap();
    f.command(&["/import", "--resume", &operation.start.operation_id]);
    f.assert_complete(&operation.start.operation_id);
    let complete = vault
        .load_import_operation(ACCOUNT, &operation.start.operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        complete
            .steps
            .iter()
            .map(|step| step.plan.attachment_id.clone())
            .collect::<Vec<_>>(),
        ids
    );
    assert_eq!(
        vault.load_object(OWNER).unwrap().unwrap().name,
        "unrelated local edit must survive resume"
    );
}

#[test]
fn rf022_cli_published_file_confirmation_failure_reports_actual_file_and_resumes_once() {
    let _guard = lock();
    let mut f = Fixture::new();
    let operation = f.partial("published");
    let text = f.app.error_message.as_ref().unwrap();
    assert!(text.contains("可用附件 0"));
    assert!(text.contains("已写文件 1"));
    assert_eq!(
        operation.steps[0].phase,
        solosoul_vault::ImportAttachmentPhase::Staged
    );
    f.remove_failure();
    f.reopen();
    std::fs::remove_file(&f.package).unwrap();
    // 第二 ZIP 条目尚 Planned，仍须原包；本例不假称已经全 ready。
    let requirements = import_credential_requirements(&operation).unwrap();
    assert!(requirements.source_required);
    f.command(&["/import", "--resume", &operation.start.operation_id]);
    assert!(matches!(
        f.app.prompt.as_ref().map(|p| &p.spec),
        Some(PromptSpec::Text { mask: false, .. })
    ));
    f.key(KeyCode::Esc);
    assert!(f.app.prompt.is_none());
    assert_eq!(f.pending().start.operation_id, operation.start.operation_id);
}

#[test]
fn rf022_cli_resume_source_then_masked_password_uses_original_operation_without_reprepare() {
    let _guard = lock();
    let mut f = Fixture::new();
    let operation = f.partial("stage");
    f.remove_failure();
    f.reopen();
    f.command(&["/import", "--resume", &operation.start.operation_id]);
    assert!(matches!(
        f.app.prompt.as_ref().map(|p| &p.spec),
        Some(PromptSpec::Text { mask: false, .. })
    ));
    let path = f.package.to_string_lossy().into_owned();
    f.text(&path);
    assert!(matches!(
        f.app.prompt.as_ref().map(|p| &p.spec),
        Some(PromptSpec::Text { mask: true, .. })
    ));
    f.confirm();
    f.assert_complete(&operation.start.operation_id);
    let complete = f
        .app
        .vault_service
        .get_vault_store()
        .unwrap()
        .load_import_operation(ACCOUNT, &operation.start.operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(complete.start.steps, operation.start.steps);
}

#[test]
fn rf022_cli_source_prompt_cancel_keeps_durable_pending_unchanged() {
    let _guard = lock();
    let mut f = Fixture::new();
    let operation = f.partial("stage");
    f.remove_failure();
    let before = f.state();
    f.command(&["/import", "--resume", &operation.start.operation_id]);
    f.key(KeyCode::Esc);
    assert_eq!(f.state(), before);
    assert_eq!(f.pending().phase, operation.phase);
    assert!(!f.app.auto_lock_paused);
}

#[test]
fn rf022_cli_password_prompt_cancel_keeps_durable_pending_unchanged() {
    let _guard = lock();
    let mut f = Fixture::new();
    let operation = f.partial("stage");
    f.remove_failure();
    let before = f.state();
    let path = f.package.to_string_lossy().into_owned();
    f.command(&["/import", "--resume", &operation.start.operation_id, &path]);
    f.key(KeyCode::Esc);
    assert_eq!(f.state(), before);
    assert_eq!(f.pending().start.operation_id, operation.start.operation_id);
    assert!(!f.app.auto_lock_paused);
}

#[test]
fn rf022_cli_wrong_package_proof_cannot_change_original_plan_or_business_tables() {
    let _guard = lock();
    let mut f = Fixture::new();
    let operation = f.partial("stage");
    f.remove_failure();
    let wrong = f._dir.path().join("other.solosoul");
    std::fs::write(&wrong, b"different package bytes").unwrap();
    let before = f.state();
    let path = wrong.to_string_lossy().into_owned();
    f.command(&["/import", "--resume", &operation.start.operation_id, &path]);
    f.confirm();
    assert!(f
        .app
        .error_message
        .as_ref()
        .unwrap()
        .contains("import_source_changed"));
    assert_eq!(f.state(), before);
}

#[test]
fn rf022_cli_wrong_password_keeps_id_mapping_and_record_counts_for_next_resume() {
    let _guard = lock();
    let mut f = Fixture::new();
    let operation = f.partial("stage");
    f.remove_failure();
    let before = f.business();
    let path = f.package.to_string_lossy().into_owned();
    f.command(&["/import", "--resume", &operation.start.operation_id, &path]);
    f.text("wrong-package-password");
    assert!(f
        .app
        .error_message
        .as_ref()
        .unwrap()
        .contains(&operation.start.operation_id));
    assert_eq!(f.business(), before);
    assert_eq!(f.pending().start.steps, operation.start.steps);
    f.command(&["/import", "--resume", &operation.start.operation_id, &path]);
    f.confirm();
    f.assert_complete(&operation.start.operation_id);
}

fn assert_stale_resume(transition: &str, source_prompt: bool) {
    let _guard = lock();
    let mut f = Fixture::new();
    let operation = f.partial("stage");
    f.remove_failure();
    let path = f.package.to_string_lossy().into_owned();
    if source_prompt {
        f.command(&["/import", "--resume", &operation.start.operation_id]);
    } else {
        f.command(&["/import", "--resume", &operation.start.operation_id, &path]);
    }
    f.app.vault_service.lock();
    match transition {
        "reunlock" => {
            f.app
                .vault_service
                .unlock(ACCOUNT, crate::TEST_PASSWORD)
                .unwrap();
        }
        "switch" => {
            f.app
                .vault_service
                .create_account_with_id(OTHER, "RF022 other", crate::TEST_PASSWORD, None)
                .unwrap();
            f.app
                .vault_service
                .unlock(OTHER, crate::TEST_PASSWORD)
                .unwrap();
        }
        "lock" => {}
        _ => panic!("unknown transition"),
    }
    let before = f.state();
    if source_prompt {
        f.text(&path);
    } else {
        f.confirm();
    }
    assert!(f.app.prompt.is_none());
    assert!(f
        .app
        .error_message
        .as_ref()
        .unwrap()
        .contains("Vault session is no longer current"));
    assert!(f.app.success_message.is_none());
    assert_eq!(f.state(), before);
    if transition == "switch" {
        assert!(f
            .app
            .vault_service
            .get_vault_store()
            .unwrap()
            .list_import_operations(OTHER)
            .unwrap()
            .is_empty());
    }
}

#[test]
fn rf022_cli_old_source_prompt_cannot_continue_after_same_account_reunlock() {
    assert_stale_resume("reunlock", true);
}
#[test]
fn rf022_cli_old_password_prompt_cannot_resume_after_same_account_reunlock() {
    assert_stale_resume("reunlock", false);
}
#[test]
fn rf022_cli_old_password_prompt_cannot_resume_in_new_account() {
    assert_stale_resume("switch", false);
}
#[test]
fn rf022_cli_old_password_prompt_cannot_resume_after_lock() {
    assert_stale_resume("lock", false);
}

#[test]
fn rf022_cli_pending_locked_rejected_and_locked_preview_still_works() {
    let _guard = lock();
    let mut f = Fixture::new();
    f.app.vault_service.lock();
    assert!(handle(&mut f.app, &["/import", "--pending"]).is_err());
    assert!(f.app.prompt.is_none());
    let path = f.package.to_string_lossy().into_owned();
    f.command(&["/import", &path, "--preview"]);
    assert!(f.app.info_message.as_ref().unwrap().contains("导出包预览"));
    assert!(f.app.prompt.is_none());
}

#[test]
fn rf022_cli_pending_list_only_current_account_and_missing_id_fixed_error() {
    let _guard = lock();
    let mut f = Fixture::new();
    let operation = f.partial("stage");
    f.remove_failure();
    f.app.vault_service.lock();
    f.app
        .vault_service
        .create_account_with_id(OTHER, "RF022 other", crate::TEST_PASSWORD, None)
        .unwrap();
    f.app
        .vault_service
        .unlock(OTHER, crate::TEST_PASSWORD)
        .unwrap();
    f.app.error_message = None;
    f.command(&["/import", "--pending"]);
    assert!(!f
        .app
        .info_message
        .as_ref()
        .unwrap()
        .contains(&operation.start.operation_id));
    f.command(&["/import", "--resume", &uuid::Uuid::new_v4().to_string()]);
    assert!(f
        .app
        .error_message
        .as_ref()
        .unwrap()
        .contains("__IMPORT_ERR__:OPERATION_NOT_FOUND"));
    assert!(f.app.prompt.is_none());
}

#[test]
fn rf022_cli_resume_rejects_new_strategy_selection_or_preview_without_database_write() {
    let _guard = lock();
    let mut f = Fixture::new();
    let before = f.state();
    let id = uuid::Uuid::new_v4().to_string();
    for args in [
        vec!["/import", "--resume", &id, "--strategy", "merge"],
        vec!["/import", "--resume", &id, "--objects", "another"],
        vec!["/import", "--resume", &id, "--preview"],
        vec!["/import", "--pending", "--resume", &id],
    ] {
        f.app.error_message = None;
        f.command(&args);
        assert!(f.app.error_message.is_some());
        assert!(f.app.prompt.is_none());
        assert_eq!(f.state(), before);
    }
    assert!(crate::commands::system::command_usage("/import")
        .unwrap()
        .contains("--resume"));
}

#[test]
fn rf022_cli_complete_resume_needs_no_source_or_password_and_does_not_repeat_counts() {
    let _guard = lock();
    let mut f = Fixture::new();
    f.start();
    f.confirm();
    let id = f
        .db()
        .query_row("SELECT operation_id FROM import_operations", [], |row| {
            row.get::<_, String>(0)
        })
        .unwrap();
    f.reopen();
    std::fs::remove_file(&f.package).unwrap();
    let before = f.state();
    f.command(&["/import", "--resume", &id]);
    f.assert_complete(&id);
    assert_eq!(f.state(), before);
    f.command(&["/import", "--resume", &id]);
    f.assert_complete(&id);
    assert_eq!(f.state(), before);
}
