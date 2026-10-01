//! RF-015：真实 CLI 导出 handler 与掩码密码键盘回调，公开导入 API 实际解密包。
//! 不引入 zip/crypto/hex 依赖；不把旧 bool 入口升级为新会话导出接口。
use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use solosoul_core::attachment_crypto::{encrypt_file_stream, read_file_decrypted};
use solosoul_core::export_import::import_vault;
use solosoul_core::VaultService;
use solosoul_vault::ObjectRecord;
use std::sync::Arc;

const ACCOUNT: &str = "acc_rf015_cli_source";
const TARGET_ACCOUNT: &str = "acc_rf015_cli_target";
const OWNER: &str = "rf015-cli-selected";
const OTHER: &str = "rf015-cli-unselected";
const DELETED_OBJECT: &str = "rf015-cli-deleted";
const NOW: &str = "2026-10-01T00:00:00Z";
const PLAIN: &[u8] = b"CLI selected plaintext one";
const EXTRA: &[u8] = b"CLI selected plaintext two";

struct Fixture {
    app: App,
    directory: tempfile::TempDir,
    sources: Vec<(PathBuf, Vec<u8>)>,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let service = Arc::new(
            VaultService::try_with_base_path(directory.path().join("source-vault")).unwrap(),
        );
        service
            .create_account_with_id(ACCOUNT, "RF015 CLI", crate::TEST_PASSWORD, None)
            .unwrap();
        service.lock();
        service.unlock(ACCOUNT, crate::TEST_PASSWORD).unwrap();
        let mut app = App::new(service).unwrap();
        app.i18n.set_locale("zh-CN");
        app.enter_home(ACCOUNT);
        let mut f = Self {
            app,
            directory,
            sources: Vec::new(),
        };
        let plain = f.attachment(OWNER, "plain", PLAIN, false);
        let extra = f.attachment(OWNER, "extra-live", EXTRA, false);
        let deleted = f.attachment(
            OWNER,
            "deleted-att",
            b"deleted attachment not exported",
            true,
        );
        f.save_object(OWNER, vec![plain, extra, deleted], false);
        let other = f.attachment(
            OTHER,
            "other-live",
            b"unselected object not exported",
            false,
        );
        f.save_object(OTHER, vec![other], false);
        let deleted_object = f.attachment(
            DELETED_OBJECT,
            "deleted-object-live",
            b"deleted object not exported",
            false,
        );
        f.save_object(DELETED_OBJECT, vec![deleted_object], true);
        f
    }

    fn attachment(
        &mut self,
        object: &str,
        attachment: &str,
        content: &[u8],
        deleted: bool,
    ) -> serde_json::Value {
        let path = self
            .app
            .vault_service
            .base_path()
            .join("attachments")
            .join(object)
            .join(attachment)
            .join("payload.bin");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, content).unwrap();
        self.sources.push((path.clone(), content.to_vec()));
        serde_json::json!({
            "id": attachment,
            "objectId": object,
            "fileName": "payload.bin",
            "mimeType": "application/octet-stream",
            "sizeBytes": content.len(),
            "createdAt": NOW,
            "deletedAt": if deleted { Some(NOW) } else { None },
            "vaultPath": path.to_string_lossy(),
            "tags": []
        })
    }

    fn save_object(&self, id: &str, attachments: Vec<serde_json::Value>, deleted: bool) {
        let session = self.app.vault_service.capture_session(ACCOUNT).unwrap();
        self.app.vault_service.with_session(&session, |vault| {
            vault.save_object(&ObjectRecord {
                id: id.into(),
                account_id: ACCOUNT.into(),
                type_id: "note".into(),
                section_type: "identity".into(),
                name: format!("CLI 导出 {id}"),
                properties: serde_json::json!({"body":"rf015-cli-private-body", "__attachments":attachments}),
                sensitivity_level: "internal".into(),
                is_deleted: deleted,
                deleted_at: if deleted { Some(NOW.into()) } else { None },
                created_at: NOW.into(),
                updated_at: NOW.into(),
                version: 1,
                ..Default::default()
            })
        }).unwrap();
    }

    fn submit_export_password(&mut self, name: &str, include_attachments: bool) -> PathBuf {
        self.app.error_message = None;
        self.app.success_message = None;
        let mut args = vec!["/export", name, "--objects", OWNER];
        if include_attachments {
            args.push("--include-attachments");
        }
        // 必须通过实际 handler，不能直接调用 parser/Core 模拟 CLI 行为。
        handle(&mut self.app, &args).unwrap();
        let state = self.app.prompt.as_ref().expect("actual password prompt");
        assert!(matches!(
            &state.spec,
            PromptSpec::Text {
                mask: true,
                allow_toggle_mask: true,
                ..
            }
        ));
        assert!(state.mask);
        assert!(state.value.is_empty());
        assert!(self.app.auto_lock_paused);
        for character in crate::TEST_EXPORT_PASSWORD.chars() {
            assert!(prompt::handle_key(
                &mut self.app,
                KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE)
            ));
        }
        assert!(self.app.prompt.as_ref().unwrap().mask);
        assert!(prompt::handle_key(
            &mut self.app,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)
        ));
        assert!(self.app.prompt.is_none());
        assert!(!self.app.auto_lock_paused);
        self.app
            .vault_service
            .base_path()
            .join("exports")
            .join(name)
    }

    fn assert_success(&self) {
        assert!(
            self.app.error_message.is_none(),
            "{:?}",
            self.app.error_message
        );
        let message = &self
            .app
            .success_message
            .as_ref()
            .expect("actual export success")
            .0;
        assert!(!message.contains("cmd-export-success"));
        assert!(!message.contains("{$"));
    }

    fn assert_sources_unchanged(&self) {
        for (path, bytes) in &self.sources {
            assert_eq!(std::fs::read(path).unwrap(), *bytes, "{}", path.display());
        }
    }

    fn encrypt_selected_plain_source(&mut self) {
        let session = self.app.vault_service.capture_session(ACCOUNT).unwrap();
        let key = self
            .app
            .vault_service
            .attachment_key_for_session(&session)
            .unwrap();
        let source = self.directory.path().join("solc-plaintext.bin");
        std::fs::write(&source, PLAIN).unwrap();
        let destination = self
            .app
            .vault_service
            .base_path()
            .join("attachments")
            .join(OWNER)
            .join("plain")
            .join("payload.bin");
        encrypt_file_stream(&key, &source, &destination).unwrap();
        let cipher = std::fs::read(&destination).unwrap();
        assert!(cipher.starts_with(b"SOLC"));
        assert_eq!(
            read_file_decrypted(&key, &destination, 4096).unwrap(),
            PLAIN
        );
        let tracked = self
            .sources
            .iter_mut()
            .find(|(path, _)| *path == destination)
            .unwrap();
        tracked.1 = cipher;
        self.sources.push((source, PLAIN.to_vec()));
    }

    fn decrypt_package_into_independent_vault(&self, package: &Path, expected_contents: &[&[u8]]) {
        let preview = import_preview(package).unwrap();
        assert_eq!(preview.object_count, 1);
        assert_eq!(preview.has_attachments, !expected_contents.is_empty());
        // 使用公开 Core 导入实际执行 KDF、解密 payload/附件和目标 SOLC 加密。
        // 独立目标根与账户避免拿源路径/源元数据当作导出正确的证据。
        let service =
            VaultService::try_with_base_path(self.directory.path().join("target-vault")).unwrap();
        service
            .create_account_with_id(TARGET_ACCOUNT, "RF015 导入验证", crate::TEST_PASSWORD, None)
            .unwrap();
        service.lock();
        service
            .unlock(TARGET_ACCOUNT, crate::TEST_PASSWORD)
            .unwrap();
        let session = service.capture_session(TARGET_ACCOUNT).unwrap();
        let key = service.attachment_key_for_session(&session).unwrap();
        let count = import_vault(
            session.vault(),
            TARGET_ACCOUNT,
            package,
            crate::TEST_EXPORT_PASSWORD,
            ImportStrategy::Overwrite,
            service.base_path(),
            Some(&key),
        )
        .unwrap();
        assert_eq!(count, 1);
        let imported = session.vault().load_object(OWNER).unwrap().unwrap();
        assert_eq!(imported.account_id, TARGET_ACCOUNT);
        assert_eq!(imported.properties["body"], "rf015-cli-private-body");
        assert!(session.vault().load_object(OTHER).unwrap().is_none());
        assert!(session
            .vault()
            .load_object(DELETED_OBJECT)
            .unwrap()
            .is_none());
        let attachment_root = service.base_path().join("attachments");
        let files = regular_files(&attachment_root);
        assert_eq!(files.len(), expected_contents.len());
        let mut actual: Vec<Vec<u8>> = files
            .iter()
            .map(|path| {
                assert!(path.starts_with(attachment_root.join(OWNER)));
                assert!(std::fs::read(path).unwrap().starts_with(b"SOLC"));
                read_file_decrypted(&key, path, 4096).unwrap()
            })
            .collect();
        actual.sort();
        let mut expected: Vec<Vec<u8>> = expected_contents
            .iter()
            .map(|bytes| bytes.to_vec())
            .collect();
        expected.sort();
        assert_eq!(actual, expected);
        self.assert_sources_unchanged();
    }
}

fn regular_files(root: &Path) -> Vec<PathBuf> {
    if !root.exists() {
        return Vec::new();
    }
    let mut files = Vec::new();
    for entry in std::fs::read_dir(root).unwrap() {
        let entry = entry.unwrap();
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            files.extend(regular_files(&entry.path()));
        } else {
            assert!(kind.is_file(), "unexpected non-regular fixture file");
            files.push(entry.path());
        }
    }
    files.sort();
    files
}

#[test]
fn rf015_cli_default_attachment_flag_exports_no_files_and_decrypts_records() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut f = Fixture::new();
    let package = f.submit_export_password("rf015-none.solosoul", false);
    f.assert_success();
    assert!(package.exists());
    f.assert_sources_unchanged();
    f.decrypt_package_into_independent_vault(&package, &[]);
}

#[test]
fn rf015_cli_include_attachments_exports_all_live_files_of_selected_objects_only() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut f = Fixture::new();
    let package = f.submit_export_password("rf015-all.solosoul", true);
    f.assert_success();
    assert!(package.exists());
    f.assert_sources_unchanged();
    f.decrypt_package_into_independent_vault(&package, &[PLAIN, EXTRA]);
}

#[test]
fn rf015_cli_legacy_include_flag_retains_solc_without_key_error() {
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut f = Fixture::new();
    f.encrypt_selected_plain_source();
    let _package = f.submit_export_password("rf015-solc-error.solosoul", true);
    assert!(f.app.success_message.is_none());
    assert!(f
        .app
        .error_message
        .as_deref()
        .unwrap()
        .contains("CLI 缺少附件解密密钥"));
    f.assert_sources_unchanged();
    // None 不读取附件体；仍可导出同一个含 SOLC 元数据的选中对象。
    let package = f.submit_export_password("rf015-solc-none.solosoul", false);
    f.assert_success();
    f.decrypt_package_into_independent_vault(&package, &[]);
}
