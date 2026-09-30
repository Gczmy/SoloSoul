//! RF021：真实 CLI 密码提示、加密包和 SQLite 回归候选；只使用合成 TempDir。
use super::*;
use crate::app::AppPhase;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rusqlite::types::ValueRef;
use rusqlite::Connection;
use solosoul_core::VaultService;
use solosoul_vault::{ObjectRecord, UserTemplate};
use std::collections::BTreeMap;
use std::sync::Arc;
use tempfile::TempDir;

const ACCOUNT: &str = "acc_rf021_cli_target";
const OTHER: &str = "acc_rf021_cli_other";
const OBJECT_IDS: [&str; 2] = ["rf021-cli-a", "rf021-cli-b"];
const STALE_SESSION: &str = "Vault session is no longer current";
type DatabaseState = BTreeMap<String, Vec<Vec<Vec<u8>>>>;

struct Fixture {
    app: App,
    package: PathBuf,
    _dir: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let source = VaultService::with_base_path(dir.path().join("source"));
        let source_account = "acc_rf021_cli_source";
        source
            .create_account_with_id(source_account, "RF021 source", crate::TEST_PASSWORD, None)
            .unwrap();
        source.unlock(source_account, crate::TEST_PASSWORD).unwrap();
        let source_vault = source.get_vault_store().unwrap();
        for (index, id) in OBJECT_IDS.iter().enumerate() {
            let template = UserTemplate {
                id: format!("rf021-cli-template-{index}"),
                account_id: source_account.into(),
                name: format!("RF021 CLI template {index}"),
                icon_id: Some("document".into()),
                properties: vec![],
                category: Some("identity".into()),
                created_at: "2026-09-30T00:00:00Z".into(),
                updated_at: None,
                contract_type_id: None,
            };
            source_vault.save_user_template(&template).unwrap();
            let mut record = record(id, source_account, "source");
            record.template_id = Some(template.id);
            record.template_type = Some("user".into());
            source_vault.save_object(&record).unwrap();
            // Core/CLI 导入契约不导入历史；源端确有历史，目标仍应保持原样。
            source_vault
                .save_snapshot(
                    id,
                    "user_edit",
                    b"synthetic source history",
                    "source history",
                )
                .unwrap();
        }
        let package = dir.path().join("rf021-real.solosoul");
        let exported = export_vault(
            &source_vault,
            source_account,
            crate::TEST_EXPORT_PASSWORD,
            &package,
            &ExportScope {
                full: true,
                ..ExportScope::default()
            },
            source.base_path(),
        )
        .unwrap();
        assert_eq!(exported, 2);
        source.lock();

        let service = Arc::new(VaultService::with_base_path(dir.path().join("target")));
        service
            .create_account_with_id(ACCOUNT, "RF021 target", crate::TEST_PASSWORD, None)
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

    fn db_path(&self, account: &str) -> PathBuf {
        self.app
            .vault_service
            .base_path()
            .join(account)
            .join("vault.db")
    }

    fn state(&self, account: &str) -> DatabaseState {
        database_state(&self.db_path(account))
    }

    fn start_import(&mut self, strategy: &str) {
        self.app.error_message = None;
        self.app.success_message = None;
        let path = self.package.to_str().unwrap().to_string();
        handle(&mut self.app, &["/import", &path, "--strategy", strategy]).unwrap();
        assert!(matches!(
            self.app.prompt.as_ref().map(|state| &state.spec),
            Some(PromptSpec::Text { mask: true, .. })
        ));
        assert!(self.app.auto_lock_paused);
    }

    fn key(&mut self, key: KeyCode) {
        assert!(prompt::handle_key(
            &mut self.app,
            KeyEvent::new(key, KeyModifiers::NONE)
        ));
    }

    fn confirm_password(&mut self, password: &str) {
        // 走实际 Text prompt 的按键及 finish 回调，不能只直调 Core 模拟 CLI。
        for character in password.chars() {
            self.key(KeyCode::Char(character));
        }
        self.key(KeyCode::Enter);
        assert!(self.app.prompt.is_none());
        assert!(!self.app.auto_lock_paused);
    }

    fn confirm(&mut self) {
        self.confirm_password(crate::TEST_EXPORT_PASSWORD);
    }

    fn seed_local_objects(&self) {
        let vault = self.app.vault_service.get_vault_store().unwrap();
        for id in OBJECT_IDS {
            vault.save_object(&record(id, ACCOUNT, "local")).unwrap();
            vault
                .save_snapshot(id, "user_edit", b"synthetic local history", "local history")
                .unwrap();
        }
    }

    fn assert_imported(&self) {
        assert!(
            self.app.error_message.is_none(),
            "{:?}",
            self.app.error_message
        );
        let message = &self.app.success_message.as_ref().unwrap().0;
        assert!(message.contains("成功导入"));
        assert!(message.contains('2'));
        let vault = self.app.vault_service.get_vault_store().unwrap();
        for (index, id) in OBJECT_IDS.iter().enumerate() {
            let loaded = vault.load_object(id).unwrap().unwrap();
            assert_eq!(loaded.account_id, ACCOUNT);
            assert_eq!(loaded.name, format!("source {id}"));
            assert_eq!(loaded.properties["value"], format!("source {id} secret"));
            let template = vault
                .load_user_template(loaded.template_id.as_ref().unwrap())
                .unwrap()
                .unwrap();
            assert_eq!(template.account_id, ACCOUNT);
            assert_eq!(template.name, format!("RF021 CLI template {index}"));
            assert!(template.id.starts_with("imported:"));
        }
        assert_eq!(
            vault
                .list_user_templates(ACCOUNT)
                .unwrap()
                .iter()
                .filter(|template| template.name.starts_with("RF021 CLI template "))
                .count(),
            2
        );
        let conn = Connection::open(self.db_path(ACCOUNT)).unwrap();
        for id in OBJECT_IDS {
            let properties: String = conn
                .query_row(
                    "SELECT properties FROM objects WHERE id = ?1",
                    [id],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(!properties.contains("source"), "属性应加密存储");
            let hlc: i64 = conn
                .query_row(
                    "SELECT wall_time_ms FROM sync_hlc WHERE table_name = 'objects' AND record_id = ?1",
                    [id],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(hlc > 0);
        }
    }

    fn install_failure(&self, stage: &str) {
        let (operation, table, predicate) = match stage {
            "templates" => ("INSERT", "user_templates", "NEW.id LIKE 'imported:%'"),
            "object_insert" => (
                "INSERT",
                "objects",
                "NEW.id IN ('rf021-cli-a', 'rf021-cli-b')",
            ),
            "object_update" => (
                "UPDATE",
                "objects",
                "NEW.id IN ('rf021-cli-a', 'rf021-cli-b')",
            ),
            "hlc" => (
                "INSERT",
                "sync_hlc",
                "NEW.table_name = 'objects' AND NEW.record_id IN ('rf021-cli-a', 'rf021-cli-b')",
            ),
            _ => panic!("unknown synthetic failure stage"),
        };
        // 计数本身处于同一事务；第二条实际 DML 失败后，计数也必须回滚到 0。
        Connection::open(self.db_path(ACCOUNT))
            .unwrap()
            .execute_batch(&format!(
                "CREATE TABLE rf021_attempts (attempts INTEGER NOT NULL);
                 INSERT INTO rf021_attempts VALUES (0);
                 CREATE TRIGGER rf021_fail BEFORE {operation} ON {table}
                 WHEN {predicate} BEGIN
                    UPDATE rf021_attempts SET attempts = attempts + 1;
                    SELECT RAISE(ABORT, 'rf021 synthetic second write failure')
                    WHERE (SELECT attempts FROM rf021_attempts) = 2;
                 END;"
            ))
            .unwrap();
    }

    fn remove_failure(&self) {
        Connection::open(self.db_path(ACCOUNT))
            .unwrap()
            .execute_batch("DROP TRIGGER rf021_fail; DROP TABLE rf021_attempts;")
            .unwrap();
    }
}

fn record(id: &str, account: &str, origin: &str) -> ObjectRecord {
    ObjectRecord {
        id: id.into(),
        account_id: account.into(),
        type_id: "note".into(),
        section_type: "identity".into(),
        name: format!("{origin} {id}"),
        icon_name: "document".into(),
        properties: serde_json::json!({"value": format!("{origin} {id} secret")}),
        sensitivity_level: "internal".into(),
        created_at: "2026-09-30T00:00:00Z".into(),
        updated_at: "2026-09-30T00:00:00Z".into(),
        version: 1,
        ..ObjectRecord::default()
    }
}

fn cell(value: ValueRef<'_>) -> Vec<u8> {
    match value {
        ValueRef::Null => vec![b'N'],
        ValueRef::Integer(value) => {
            let mut bytes = vec![b'I'];
            bytes.extend_from_slice(&value.to_le_bytes());
            bytes
        }
        ValueRef::Real(value) => {
            let mut bytes = vec![b'R'];
            bytes.extend_from_slice(&value.to_bits().to_le_bytes());
            bytes
        }
        ValueRef::Text(value) => {
            let mut bytes = vec![b'T'];
            bytes.extend_from_slice(value);
            bytes
        }
        ValueRef::Blob(value) => {
            let mut bytes = vec![b'B'];
            bytes.extend_from_slice(value);
            bytes
        }
    }
}

/// 比较真实 SQLite 所有业务表的原始字节，不以解密后的对象数量代替零写断言。
fn database_state(path: &Path) -> DatabaseState {
    assert!(path.is_file());
    let conn = Connection::open(path).unwrap();
    let tables = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    let mut state = BTreeMap::new();
    for table in tables {
        let sql = format!("SELECT * FROM \"{}\"", table.replace('"', "\"\""));
        let mut statement = conn.prepare(&sql).unwrap();
        let count = statement.column_count();
        let mut cursor = statement.query([]).unwrap();
        let mut rows = Vec::new();
        while let Some(row) = cursor.next().unwrap() {
            rows.push(
                (0..count)
                    .map(|index| cell(row.get_ref(index).unwrap()))
                    .collect::<Vec<_>>(),
            );
        }
        rows.sort();
        state.insert(table, rows);
    }
    state
}

fn assert_stale_prompt(transition: &str) {
    let _guard = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut f = Fixture::new();
    f.start_import("overwrite");
    f.app.vault_service.lock();
    match transition {
        "switch" => {
            f.app
                .vault_service
                .create_account_with_id(OTHER, "RF021 other", crate::TEST_PASSWORD, None)
                .unwrap();
            f.app
                .vault_service
                .unlock(OTHER, crate::TEST_PASSWORD)
                .unwrap();
        }
        "reunlock" => {
            f.app
                .vault_service
                .unlock(ACCOUNT, crate::TEST_PASSWORD)
                .unwrap();
        }
        "lock" => {}
        _ => panic!("unknown session transition"),
    }
    // 转换自身允许合法写入；只比较旧提示提交前后，避免把正常解锁副作用当导入写入。
    let before = f.state(ACCOUNT);
    let other_before = (transition == "switch").then(|| f.state(OTHER));
    f.confirm();
    assert!(f
        .app
        .error_message
        .as_ref()
        .unwrap()
        .contains(STALE_SESSION));
    assert!(f.app.success_message.is_none());
    assert_eq!(f.state(ACCOUNT), before, "{transition}: 原账户必须零写");
    if let Some(other_before) = other_before {
        assert_eq!(f.state(OTHER), other_before, "新账户必须零写");
    }
}

fn assert_database_failure(stage: &str) {
    let _guard = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut f = Fixture::new();
    if stage == "object_update" {
        f.seed_local_objects();
    }
    // DDL 装好后再开始导入，不能故意用 data_version 过期替代实际第二条写入失败。
    f.install_failure(stage);
    let before = f.state(ACCOUNT);
    f.start_import("overwrite");
    f.confirm();
    let expected = match stage {
        "templates" => "import_batch_templates_failed",
        "object_insert" | "object_update" => "import_batch_objects_failed",
        "hlc" => "import_batch_hlc_failed",
        _ => unreachable!(),
    };
    assert!(
        f.app.error_message.as_ref().unwrap().contains(expected),
        "{stage}: 应到达实际失败阶段，不能把提前拒绝误当回滚：{:?}",
        f.app.error_message
    );
    assert!(f.app.success_message.is_none(), "{stage}");
    assert_eq!(
        f.state(ACCOUNT),
        before,
        "{stage}: 模板/对象/历史/HLC 应整批回滚"
    );
    f.remove_failure();
    f.start_import("overwrite");
    f.confirm();
    f.assert_imported();
}

#[test]
fn rf021_cli_real_password_prompt_imports_objects_templates_and_hlc() {
    let _guard = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut f = Fixture::new();
    f.start_import("overwrite");
    f.confirm();
    f.assert_imported();
    let vault = f.app.vault_service.get_vault_store().unwrap();
    for id in OBJECT_IDS {
        assert!(
            vault.list_snapshots(id).unwrap().is_empty(),
            "Core/CLI 不导入源历史"
        );
    }
}

#[test]
fn rf021_cli_cancel_password_prompt_does_not_write_database() {
    let _guard = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut f = Fixture::new();
    let before = f.state(ACCOUNT);
    f.start_import("overwrite");
    f.key(KeyCode::Esc);
    assert!(f.app.prompt.is_none());
    assert!(!f.app.auto_lock_paused);
    assert!(f.app.error_message.is_none());
    assert!(f.app.success_message.is_none());
    assert_eq!(f.state(ACCOUNT), before);
}

#[test]
fn rf021_cli_old_password_prompt_is_rejected_after_lock() {
    assert_stale_prompt("lock");
}

#[test]
fn rf021_cli_old_password_prompt_is_rejected_after_account_switch() {
    assert_stale_prompt("switch");
}

#[test]
fn rf021_cli_old_password_prompt_is_rejected_after_same_account_reunlock() {
    assert_stale_prompt("reunlock");
}

#[test]
fn rf021_cli_second_template_insert_failure_rolls_back_and_recovers() {
    assert_database_failure("templates");
}

#[test]
fn rf021_cli_second_object_insert_failure_rolls_back_and_recovers() {
    assert_database_failure("object_insert");
}

#[test]
fn rf021_cli_second_object_update_failure_rolls_back_and_recovers() {
    assert_database_failure("object_update");
}

#[test]
fn rf021_cli_second_object_hlc_insert_failure_rolls_back_and_recovers() {
    assert_database_failure("hlc");
}

#[test]
fn rf021_cli_overwrite_keeps_existing_encrypted_history() {
    let _guard = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut f = Fixture::new();
    f.seed_local_objects();
    let history_before = f.state(ACCOUNT).remove("object_snapshots").unwrap();
    f.start_import("overwrite");
    f.confirm();
    f.assert_imported();
    assert_eq!(
        f.state(ACCOUNT).remove("object_snapshots").unwrap(),
        history_before
    );
}

#[test]
fn rf021_cli_preview_remains_available_when_locked() {
    let _guard = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut f = Fixture::new();
    f.app.vault_service.lock();
    let before = f.state(ACCOUNT);
    let path = f.package.to_str().unwrap().to_string();
    handle(&mut f.app, &["/import", &path, "--preview"]).unwrap();
    assert!(f.app.prompt.is_none());
    assert!(f.app.error_message.is_none());
    assert!(f.app.info_message.as_ref().unwrap().contains("导出包预览"));
    assert_eq!(f.state(ACCOUNT), before);
}

#[test]
fn rf021_cli_wrong_import_password_does_not_write_database() {
    let _guard = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut f = Fixture::new();
    let before = f.state(ACCOUNT);
    f.start_import("overwrite");
    f.confirm_password("WrongExportPass1");
    assert!(f.app.error_message.is_some());
    assert!(f.app.success_message.is_none());
    assert_eq!(f.state(ACCOUNT), before);
}
