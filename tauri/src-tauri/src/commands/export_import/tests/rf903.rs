use super::rf020::{objects as package_objects, package, Fixture};
use super::*;
use solosoul_core::attachment_crypto::read_file_decrypted;
use solosoul_core::objects as core_objects;
use std::path::{Path, PathBuf};

struct ImportedAttachment {
    object_id: String,
    attachment_id: String,
    metadata: serde_json::Value,
    path: PathBuf,
    ciphertext: Vec<u8>,
}

fn import_for_account(f: &Fixture, account: &str, path: &Path) {
    let svc = f.service.read().unwrap();
    let session = svc.capture_session(account).unwrap();
    let result = import_execute_for_session(
        &svc,
        &session,
        path.to_string_lossy().into_owned(),
        Zeroizing::new("export-password".into()),
        ImportStrategy::Overwrite,
        None,
        None,
        HashMap::new(),
        "en-US",
        None,
    )
    .unwrap();
    assert_eq!(result.status, ImportStatus::Complete, "{result:?}");
    assert_eq!(result.object_count, 2);
    assert_eq!(result.attachment_count, 2);
    assert_eq!(result.attachment_files_written, 2);
}

fn unlock_account(f: &Fixture, account: &str) {
    let svc = f.service.read().unwrap();
    svc.lock();
    svc.unlock_secure(account, &Zeroizing::new("password123".into()))
        .unwrap();
    assert_eq!(svc.get_current_account().as_deref(), Some(account));
}

fn imported_attachments(f: &Fixture, account: &str) -> Vec<ImportedAttachment> {
    let svc = f.service.read().unwrap();
    let session = svc.capture_session(account).unwrap();
    let key = svc.attachment_key_for_session(&session).unwrap();
    (0..2)
        .map(|n| {
            let object_id = format!("rf020-{n}");
            let object = session.vault().load_object(&object_id).unwrap().unwrap();
            assert_eq!(object.account_id, account);
            assert!(!object.is_deleted);
            let attachments = core_objects::load_attachments(&object.properties);
            assert_eq!(attachments.len(), 1);
            let attachment = &attachments[0];
            assert_eq!(attachment.object_id, object_id);
            assert!(attachment.deleted_at.is_none());
            assert!(Uuid::parse_str(&attachment.id).is_ok());
            let path = PathBuf::from(attachment.vault_path.as_ref().unwrap());
            assert_eq!(
                path,
                svc.base_path()
                    .join("attachments")
                    .join(&object_id)
                    .join(&attachment.id)
                    .join("sample.txt")
            );
            let ciphertext = std::fs::read(&path).unwrap();
            assert!(ciphertext.starts_with(b"SOLC"));
            assert_eq!(read_file_decrypted(&key, &path, 1024).unwrap(), b"fixture");
            ImportedAttachment {
                object_id,
                attachment_id: attachment.id.clone(),
                metadata: object.properties["__attachments"].clone(),
                path,
                ciphertext,
            }
        })
        .collect()
}

fn assert_files_preserved<'a>(
    attachments: impl IntoIterator<Item = &'a ImportedAttachment>,
    cleanup_result: &Result<(usize, u64), String>,
) {
    for attachment in attachments {
        assert!(
            attachment.path.is_file(),
            "cleanup removed a valid imported attachment: object={}, attachment={}, path={:?}, cleanup={cleanup_result:?}",
            attachment.object_id,
            attachment.attachment_id,
            attachment.path
        );
        assert_eq!(
            std::fs::read(&attachment.path).unwrap(),
            attachment.ciphertext,
            "cleanup changed valid attachment bytes: {:?}",
            attachment.path
        );
    }
}

fn assert_current_account_can_read(f: &Fixture, account: &str, attachments: &[ImportedAttachment]) {
    // 每次从当前会话重新取得 Vault/密钥，不能使用切换后已关闭的 Fixture.vault。
    let svc = f.service.read().unwrap();
    let session = svc.capture_session(account).unwrap();
    let key = svc.attachment_key_for_session(&session).unwrap();
    for attachment in attachments {
        let object = session
            .vault()
            .load_object(&attachment.object_id)
            .unwrap()
            .unwrap();
        assert_eq!(object.account_id, account);
        assert!(!object.is_deleted);
        assert_eq!(object.properties["__attachments"], attachment.metadata);
        assert_eq!(
            read_file_decrypted(&key, &attachment.path, 1024).unwrap(),
            b"fixture"
        );
    }
}

#[test]
fn rf903_cleanup_preserves_same_package_imported_into_two_accounts() {
    let f = Fixture::new();
    let path = package(f.dir.path(), package_objects(), true, false, false);
    import_for_account(&f, &f.account, &path);
    let account_a_attachments = imported_attachments(&f, &f.account);

    let account_b = format!("acc_{}", Uuid::new_v4().simple());
    {
        let svc = f.service.read().unwrap();
        svc.lock();
        svc.create_account_with_id(&account_b, "RF903 B", "password123", None)
            .unwrap();
    }
    // 同一真实包、同一全局附件根：对象 ID 相同，附件 UUID 由生产导入分别生成。
    import_for_account(&f, &account_b, &path);
    let account_b_attachments = imported_attachments(&f, &account_b);
    for (a, b) in account_a_attachments.iter().zip(&account_b_attachments) {
        assert_eq!(a.object_id, b.object_id);
        assert_ne!(a.attachment_id, b.attachment_id);
        assert_eq!(
            a.path.parent().unwrap().parent(),
            b.path.parent().unwrap().parent()
        );
    }

    unlock_account(&f, &f.account);
    assert_current_account_can_read(&f, &f.account, &account_a_attachments);
    let cleanup_result = {
        let svc = f.service.read().unwrap();
        let session = svc.capture_session(&f.account).unwrap();
        let maintenance =
            solosoul_core::import_activity::begin_owned_root_maintenance(session.root_owner())
                .unwrap();
        solosoul_core::orphan_cleanup::cleanup_orphan_attachments_with_maintenance(
            &svc,
            &session,
            &maintenance,
        )
        .map(|report| (report.removed, report.freed_bytes))
    };
    // 只约束有效数据不能丢失，不预设将来的清理策略或返回值。
    assert_files_preserved(
        account_a_attachments.iter().chain(&account_b_attachments),
        &cleanup_result,
    );
    assert_current_account_can_read(&f, &f.account, &account_a_attachments);
    unlock_account(&f, &account_b);
    assert_current_account_can_read(&f, &account_b, &account_b_attachments);
}

#[test]
fn rf903_cleanup_preserves_imported_attachment_through_trash_restore() {
    let f = Fixture::new();
    let path = package(f.dir.path(), package_objects(), true, false, false);
    import_for_account(&f, &f.account, &path);
    let mut attachments = imported_attachments(&f, &f.account);
    let object_id = &attachments[0].object_id;
    let svc = f.service.read().unwrap();
    let session = svc.capture_session(&f.account).unwrap();
    let vault = session.vault();
    let object = vault.load_object(object_id).unwrap().unwrap();
    core_objects::move_to_trash(vault, &object, "object", None, 3_600_000).unwrap();
    assert!(vault.load_object(object_id).unwrap().unwrap().is_deleted);
    let trash_id = vault
        .list_trash_items(None, None)
        .unwrap()
        .into_iter()
        .find(|item| item.original_id == *object_id)
        .unwrap()
        .id;
    let trash = vault.get_trash_item(&trash_id).unwrap().unwrap();
    let trash_data: serde_json::Value = serde_json::from_slice(&trash.data).unwrap();
    assert_eq!(
        trash_data["properties"]["__attachments"],
        attachments[0].metadata
    );

    let cleanup_result = {
        let maintenance =
            solosoul_core::import_activity::begin_owned_root_maintenance(session.root_owner())
                .unwrap();
        solosoul_core::orphan_cleanup::cleanup_orphan_attachments_with_maintenance(
            &svc,
            &session,
            &maintenance,
        )
        .map(|report| (report.removed, report.freed_bytes))
    };
    assert_eq!(
        vault.get_trash_item(&trash.id).unwrap().unwrap().data,
        trash.data
    );
    // 先经过真实恢复入口，再核验已恢复对象的附件，直接暴露元数据恢复但文件丢失。
    let restored = core_objects::restore_from_trash(vault, &trash.id).unwrap();
    let restored_object = vault.load_object(&restored.restored_id).unwrap().unwrap();
    assert_eq!(
        restored_object.properties["__attachments"],
        attachments[0].metadata
    );
    assert!(vault.get_trash_item(&trash.id).unwrap().is_none());
    // 同名冲突可以生成新对象 ID；附件元数据与物理路径仍必须保持原引用。
    attachments[0].object_id = restored.restored_id;
    drop(session);
    drop(svc);

    assert_files_preserved(&attachments, &cleanup_result);
    assert_current_account_can_read(&f, &f.account, &attachments);
}
