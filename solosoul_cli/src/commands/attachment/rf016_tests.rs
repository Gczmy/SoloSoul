//! RF016：实际CLI确认框与临时Vault，不访问用户数据。
use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use solosoul_core::VaultService;
use solosoul_vault::ObjectRecord;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

pub(crate) struct Fixture {
    pub app: App,
    pub account: String,
    pub file: PathBuf,
    _dir: TempDir,
}
impl Fixture {
    pub(crate) fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let service = Arc::new(VaultService::with_base_path(dir.path().join("vault")));
        let account = "acc_rf016_cli_account".to_string();
        service
            .create_account_with_id(&account, "RF016 synthetic", crate::TEST_PASSWORD, None)
            .unwrap();
        service.unlock(&account, crate::TEST_PASSWORD).unwrap();
        let file = service
            .base_path()
            .join("attachments/rf016-cli-object/rf016-cli-att/file.txt");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, b"synthetic CLI attachment").unwrap();
        let record = ObjectRecord {
            contract_type_id: None,
            id: "rf016-cli-object".into(),
            account_id: account.clone(),
            type_id: "note".into(),
            section_type: "identity".into(),
            name: "RF016 synthetic".into(),
            icon_name: "document".into(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({"__attachments": [{"id":"rf016-cli-att","objectId":"rf016-cli-object","fileName":"file.txt","mimeType":"text/plain","sizeBytes":24,"createdAt":"2026-09-30T00:00:00Z","vaultPath":file.to_string_lossy()}]}),
            property_labels: None,
            sensitivity_level: "internal".into(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            template_hash: None,
            ignored_template_hash: None,
            created_at: "2026-09-30T00:00:00Z".into(),
            updated_at: "2026-09-30T00:00:00Z".into(),
            version: 1,
        };
        service
            .get_vault_store()
            .unwrap()
            .save_object(&record)
            .unwrap();
        let mut app = App::new(service).unwrap();
        app.i18n.set_locale("zh-CN");
        app.phase = AppPhase::ObjectDetail { object: record };
        Self {
            app,
            account,
            file,
            _dir: dir,
        }
    }
    fn key(&mut self, key: KeyCode) {
        assert!(crate::widgets::prompt::handle_key(
            &mut self.app,
            KeyEvent::new(key, KeyModifiers::NONE)
        ));
    }
    fn attachment_count(&self) -> usize {
        objects::load_attachments(
            &self
                .app
                .vault_service
                .get_vault_store()
                .unwrap()
                .load_object("rf016-cli-object")
                .unwrap()
                .unwrap()
                .properties,
        )
        .len()
    }
}

#[test]
fn rf016_cli_cancel_does_not_remove_metadata_or_files() {
    let _guard = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut f = Fixture::new();
    handle(&mut f.app, &["purge", "rf016-cli-att"]).unwrap();
    f.key(KeyCode::Esc);
    assert_eq!(f.attachment_count(), 1);
    assert!(f.file.exists());
    assert!(f.app.success_message.is_none());
    assert!(f
        .app
        .vault_service
        .get_vault_store()
        .unwrap()
        .list_attachment_cleanup_intents(&f.account)
        .unwrap()
        .is_empty());
}

#[test]
fn rf016_cli_confirm_uses_the_recoverable_executor() {
    let _guard = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut f = Fixture::new();
    handle(&mut f.app, &["purge", "rf016-cli-att"]).unwrap();
    f.key(KeyCode::Char('y'));
    assert_eq!(f.attachment_count(), 0);
    assert!(
        matches!(&f.app.phase, AppPhase::ObjectDetail{object} if objects::load_attachments(&object.properties).is_empty())
    );
    assert!(!f.file.exists());
    assert!(f
        .app
        .success_message
        .as_ref()
        .unwrap()
        .0
        .contains("已彻底删除"));
    assert!(f
        .app
        .vault_service
        .get_vault_store()
        .unwrap()
        .list_attachment_cleanup_intents(&f.account)
        .unwrap()
        .is_empty());
}

#[test]
fn rf016_cli_old_confirmation_cannot_delete_after_session_changes() {
    let _guard = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    for transition in ["lock", "switch", "reunlock"] {
        let mut f = Fixture::new();
        handle(&mut f.app, &["purge", "rf016-cli-att"]).unwrap();
        f.app.vault_service.lock();
        if transition == "switch" {
            f.app
                .vault_service
                .create_account_with_id("acc_rf016_other", "Other", crate::TEST_PASSWORD, None)
                .unwrap();
            f.app
                .vault_service
                .unlock("acc_rf016_other", crate::TEST_PASSWORD)
                .unwrap();
        } else if transition == "reunlock" {
            f.app
                .vault_service
                .unlock(&f.account, crate::TEST_PASSWORD)
                .unwrap();
        }
        f.key(KeyCode::Char('y'));
        assert!(f.file.exists(), "{transition}");
        assert!(f.app.error_message.is_some());
        assert!(f.app.success_message.is_none());
        f.app.vault_service.lock();
        f.app
            .vault_service
            .unlock(&f.account, crate::TEST_PASSWORD)
            .unwrap();
        assert_eq!(f.attachment_count(), 1);
    }
}

#[cfg(windows)]
#[test]
fn rf016_cli_pending_is_not_reported_as_physical_completion() {
    use std::os::windows::fs::OpenOptionsExt;
    let _guard = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut f = Fixture::new();
    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(3)
        .open(&f.file)
        .unwrap();
    handle(&mut f.app, &["purge", "rf016-cli-att"]).unwrap();
    f.key(KeyCode::Char('y'));
    assert_eq!(f.attachment_count(), 0);
    assert!(f.file.exists());
    assert!(f.app.success_message.is_none());
    assert!(f.app.error_message.as_ref().unwrap().contains("待重试"));
    assert_eq!(
        f.app
            .vault_service
            .get_vault_store()
            .unwrap()
            .list_attachment_cleanup_intents(&f.account)
            .unwrap()
            .len(),
        1
    );
    drop(held);
    retry_pending_cleanup(&mut f.app, &f.account);
    assert!(!f.file.exists());
    assert!(f
        .app
        .vault_service
        .get_vault_store()
        .unwrap()
        .list_attachment_cleanup_intents(&f.account)
        .unwrap()
        .is_empty());
}

#[test]
fn rf016_cli_confirmation_refreshes_list_cache_and_clamps_selection() {
    let _guard = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut f = Fixture::new();
    let object = f
        .app
        .vault_service
        .get_vault_store()
        .unwrap()
        .load_object("rf016-cli-object")
        .unwrap()
        .unwrap();
    f.app.phase = AppPhase::AttachmentList {
        object_id: object.id,
        items: objects::load_attachments(&object.properties),
        show_deleted: false,
        selected: 5,
    };
    handle(&mut f.app, &["purge", "rf016-cli-att"]).unwrap();
    f.key(KeyCode::Char('y'));
    assert!(
        matches!(&f.app.phase,AppPhase::AttachmentList{items,selected:0,..} if items.is_empty())
    );
}

#[test]
fn rf016_cli_confirmation_rejects_a_soft_deleted_owner() {
    let _guard = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut f = Fixture::new();
    handle(&mut f.app, &["purge", "rf016-cli-att"]).unwrap();
    let vault = f.app.vault_service.get_vault_store().unwrap();
    let mut record = vault.load_object("rf016-cli-object").unwrap().unwrap();
    record.is_deleted = true;
    vault.save_object(&record).unwrap();
    f.key(KeyCode::Char('y'));
    assert!(f.file.exists());
    assert_eq!(f.attachment_count(), 1);
    assert!(f.app.error_message.is_some());
    assert!(vault
        .list_attachment_cleanup_intents(&f.account)
        .unwrap()
        .is_empty());
}

#[test]
fn rf016_cli_refresh_preserves_include_deleted_list_semantics() {
    let _guard = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut f = Fixture::new();
    let vault = f.app.vault_service.get_vault_store().unwrap();
    let mut record = vault.load_object("rf016-cli-object").unwrap().unwrap();
    let mut items = objects::load_attachments(&record.properties);
    let mut active = items[0].clone();
    active.id = "retained-active".into();
    items.push(active.clone());
    let mut deleted = active;
    deleted.id = "retained-deleted".into();
    deleted.deleted_at = Some("2026-09-30T00:00:00Z".into());
    items.push(deleted);
    objects::save_attachments(&mut record.properties, &items);
    vault.save_object(&record).unwrap();
    f.app.phase = AppPhase::AttachmentList {
        object_id: record.id,
        items,
        show_deleted: true,
        selected: 5,
    };
    handle(&mut f.app, &["purge", "rf016-cli-att"]).unwrap();
    f.key(KeyCode::Char('y'));
    assert!(
        matches!(&f.app.phase,AppPhase::AttachmentList{items,selected:1,show_deleted:true,..} if items.len()==2)
    );
}
