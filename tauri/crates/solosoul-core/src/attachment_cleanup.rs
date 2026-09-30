//! RF-016：先提交附件删除意图，再清理其明确指定的标准存储目录。
//!
//! 数据库持锁重核当前引用；文件系统检查拒绝受管目录下的链接/reparse point。
//! 这些检查不承诺阻止同用户外部进程在检查后替换路径，也不代表安全擦除介质。

use crate::{VaultService, VaultSession};
use solosoul_vault::{AttachmentCleanupIntent, VaultStore};
use std::fs::{self, Metadata};
use std::io::ErrorKind;
use std::path::Path;

/// 已接受的删除中，完成与仍需维护重试的意图数。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct CleanupReport {
    pub completed: usize,
    pub pending: usize,
}

/// 兼容无会话宿主；新宿主使用原会话包装。
pub fn purge_attachments(
    vault: &VaultStore,
    account_id: &str,
    object_id: &str,
    ids: &[String],
    base: &Path,
) -> Result<CleanupReport, String> {
    let has_metadata = has_selected_metadata(vault, account_id, object_id, ids)?;
    let intents = vault.queue_attachment_deletions(account_id, object_id, ids)?;
    if has_metadata {
        audit_accepted(vault, object_id, intents.len());
    }
    Ok(execute(&intents, |intent| {
        Ok(vault.run_attachment_cleanup_intent(intent, |stored| remove_directory(base, stored)))
    }))
}

/// 维护只执行已提交意图，不扫描或删除未登记的孤立文件。
pub fn retry_attachment_cleanup(
    vault: &VaultStore,
    account_id: &str,
    base: &Path,
) -> Result<CleanupReport, String> {
    let intents = vault.list_attachment_cleanup_intents(account_id)?;
    Ok(execute(&intents, |intent| {
        Ok(vault.run_attachment_cleanup_intent(intent, |stored| remove_directory(base, stored)))
    }))
}

/// queue 前失效返回错误；queue 后任何未完成项返回 pending，宿主仍须同步元数据变化。
pub fn purge_attachments_for_session(
    service: &VaultService,
    session: &VaultSession,
    object_id: &str,
    ids: &[String],
) -> Result<CleanupReport, String> {
    let intents = service.with_session(session, |vault| {
        let has_metadata = has_selected_metadata(vault, session.account_id(), object_id, ids)?;
        let intents = vault.queue_attachment_deletions(session.account_id(), object_id, ids)?;
        if has_metadata {
            audit_accepted(vault, object_id, intents.len());
        }
        Ok(intents)
    })?;
    Ok(execute_for_session(service, session, &intents))
}

/// 首次会话校验/list 失败返回错误；后续失效保留剩余意图并返回 pending。
pub fn retry_attachment_cleanup_for_session(
    service: &VaultService,
    session: &VaultSession,
) -> Result<CleanupReport, String> {
    let intents = service.with_session(session, |vault| {
        vault.list_attachment_cleanup_intents(session.account_id())
    })?;
    Ok(execute_for_session(service, session, &intents))
}

fn has_selected_metadata(
    vault: &VaultStore,
    account_id: &str,
    object_id: &str,
    ids: &[String],
) -> Result<bool, String> {
    match vault.load_object(object_id)? {
        Some(record) if record.account_id == account_id => {
            // 仅决定审计是否记录首次逻辑删除；授权与严格元数据解析由 queue 的事务完成。
            Ok(record
                .properties
                .get("__attachments")
                .and_then(|v| v.as_array())
                .is_some_and(|items| {
                    items.iter().any(|item| {
                        item.get("id")
                            .and_then(|v| v.as_str())
                            .is_some_and(|id| ids.iter().any(|selected| selected.as_str() == id))
                    })
                }))
        }
        _ => Err("对象不存在或账户不匹配".to_string()),
    }
}

fn audit_accepted(vault: &VaultStore, object_id: &str, count: usize) {
    if count != 0 {
        let _ = vault.log_structured(
            "attachment_purge",
            "attachment",
            Some(object_id),
            None,
            "user",
            Some(&format!("cleanup_intents={count}")),
        );
    }
}

fn execute_for_session(
    service: &VaultService,
    session: &VaultSession,
    intents: &[AttachmentCleanupIntent],
) -> CleanupReport {
    execute(intents, |intent| {
        service.with_session(session, |vault| {
            Ok(vault.run_attachment_cleanup_intent(intent, |stored| {
                // action 仅处理事务重载的权威路径；不调用 Vault 或会话 API。
                remove_directory(service.base_path(), stored)
            }))
        })
    })
}

fn execute(
    intents: &[AttachmentCleanupIntent],
    mut run: impl FnMut(&AttachmentCleanupIntent) -> Result<Result<bool, String>, String>,
) -> CleanupReport {
    let mut report = CleanupReport::default();
    for (index, intent) in intents.iter().enumerate() {
        match run(intent) {
            Ok(Ok(true)) => report.completed += 1,
            Ok(Ok(false)) => report.pending += 1,
            Ok(Err(_)) => {
                report.pending += 1;
                tracing::warn!("attachment_cleanup_pending");
            }
            Err(_) => {
                // 原会话不再允许提交；余下意图全部保留，不切换到新账户执行。
                report.pending += intents.len() - index;
                tracing::warn!("attachment_cleanup_session_stale");
                break;
            }
        }
    }
    report
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        && !matches!(
            value.to_ascii_uppercase().as_str(),
            "CON" | "PRN" | "AUX" | "NUL"
        )
        && !(value.len() == 4 && {
            let upper = value.to_ascii_uppercase();
            (upper.starts_with("COM") || upper.starts_with("LPT"))
                && matches!(upper.as_bytes()[3], b'1'..=b'9')
        })
}

fn io_code(error: &std::io::Error) -> String {
    match error.kind() {
        ErrorKind::PermissionDenied => "attachment_cleanup_permission_denied",
        _ => "attachment_cleanup_io_error",
    }
    .to_string()
}

fn link_or_reparse(meta: &Metadata) -> bool {
    if meta.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    false
}

fn regular_node(path: &Path, directory: bool) -> Result<Option<Metadata>, String> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(io_code(&error)),
    };
    if link_or_reparse(&meta)
        || (directory && !meta.is_dir())
        || (!directory && !meta.is_dir() && !meta.is_file())
    {
        return Err("attachment_cleanup_unsafe_path".to_string());
    }
    let canonical = fs::canonicalize(path).map_err(|e| io_code(&e))?;
    if canonical != path {
        return Err("attachment_cleanup_unsafe_path".to_string());
    }
    Ok(Some(meta))
}

fn check_tree(path: &Path) -> Result<(), String> {
    let mut pending = vec![path.to_path_buf()];
    while let Some(path) = pending.pop() {
        let Some(meta) = regular_node(&path, false)? else {
            continue;
        };
        if meta.is_dir() {
            for entry in fs::read_dir(&path).map_err(|e| io_code(&e))? {
                pending.push(entry.map_err(|e| io_code(&e))?.path());
            }
        }
    }
    Ok(())
}

fn remove_directory(base: &Path, intent: &AttachmentCleanupIntent) -> Result<(), String> {
    if [
        &intent.object_id,
        &intent.storage_object_id,
        &intent.attachment_id,
    ]
    .into_iter()
    .any(|id| !valid_id(id))
    {
        return Err("attachment_cleanup_invalid_id".to_string());
    }
    // 配置根允许 /tmp、/var 等合法系统别名；受管子目录不允许链接。
    let root = fs::canonicalize(base).map_err(|e| io_code(&e))?;
    if regular_node(&root, true)?.is_none() {
        return Err("attachment_cleanup_invalid_base".to_string());
    }
    let attachments = root.join("attachments");
    let object = attachments.join(&intent.storage_object_id);
    let target = object.join(&intent.attachment_id);
    for ancestor in [&attachments, &object, &target] {
        if !ancestor.starts_with(&root) {
            return Err("attachment_cleanup_unsafe_path".into());
        }
        if regular_node(ancestor, true)?.is_none() {
            return Ok(());
        }
    }
    check_tree(&target)?;
    // 删除前再核对祖先，缩小检查窗口；外部进程路径替换仍不在 DB/session 锁内。
    for ancestor in [&attachments, &object, &target] {
        if regular_node(ancestor, true)?.is_none() {
            return Ok(());
        }
    }
    match fs::remove_dir_all(&target) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_code(&error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::{create_page, load_attachments, save_attachments, AttachmentMeta};
    use tempfile::TempDir;

    const ACCOUNT: &str = "acc_rf016_account";

    fn setup() -> (TempDir, VaultStore) {
        let dir = TempDir::new().unwrap();
        let config = solosoul_vault::VaultConfig::new(ACCOUNT, dir.path().to_path_buf())
            .with_data_key([0x42; 32]);
        (dir, VaultStore::open(config).unwrap())
    }

    fn attachment(vault: &VaultStore, owner: &str, storage: &str, id: &str) {
        let mut record = vault.load_object(owner).unwrap().unwrap();
        let mut attachments = load_attachments(&record.properties);
        attachments.push(AttachmentMeta {
            id: id.to_string(),
            object_id: storage.to_string(),
            file_name: "payload.bin".to_string(),
            mime_type: "application/octet-stream".to_string(),
            size_bytes: 7,
            created_at: "2026-09-30T00:00:00Z".to_string(),
            deleted_at: None,
            src_path: None,
            vault_path: Some("/never/delete/by/vaultPath".to_string()),
            description: None,
            tags: vec![],
        });
        save_attachments(&mut record.properties, &attachments);
        vault.save_object(&record).unwrap();
    }

    fn directory(base: &Path, storage: &str, id: &str) -> std::path::PathBuf {
        let dir = base.join("attachments").join(storage).join(id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("payload.bin"), b"content").unwrap();
        dir
    }

    fn intent(storage: &str, id: &str) -> AttachmentCleanupIntent {
        AttachmentCleanupIntent {
            account_id: ACCOUNT.to_string(),
            object_id: "owner".to_string(),
            storage_object_id: storage.to_string(),
            attachment_id: id.to_string(),
            created_at: 0,
            attempts: 0,
            last_error_code: None,
        }
    }

    #[test]
    fn rf016_storage_owner_difference_and_unregistered_orphans_are_preserved() {
        let (base, vault) = setup();
        let owner = create_page(&vault, ACCOUNT, "owner").unwrap().id;
        attachment(&vault, &owner, "old_storage", "att_a");
        let actual = directory(base.path(), "old_storage", "att_a");
        let unrelated = directory(base.path(), &owner, "att_a");
        let report =
            purge_attachments(&vault, ACCOUNT, &owner, &["att_a".into()], base.path()).unwrap();
        assert_eq!(
            report,
            CleanupReport {
                completed: 1,
                pending: 0
            }
        );
        assert!(!actual.exists());
        assert!(unrelated.join("payload.bin").is_file());
        assert!(
            load_attachments(&vault.load_object(&owner).unwrap().unwrap().properties).is_empty()
        );
        assert_eq!(
            retry_attachment_cleanup(&vault, ACCOUNT, base.path()).unwrap(),
            CleanupReport::default()
        );
        assert!(unrelated.is_dir());
    }

    #[test]
    fn rf016_account_identifier_is_not_a_cleanup_path_component() {
        let base = TempDir::new().unwrap();
        let account = format!("acc_{}", "a".repeat(124));
        let vault = VaultStore::open(
            solosoul_vault::VaultConfig::new(&account, base.path().to_path_buf())
                .with_data_key([0x42; 32]),
        )
        .unwrap();
        let owner = create_page(&vault, &account, "owner").unwrap().id;
        attachment(&vault, &owner, &owner, "att_a");
        let target = directory(base.path(), &owner, "att_a");
        assert_eq!(
            purge_attachments(&vault, &account, &owner, &["att_a".into()], base.path()).unwrap(),
            CleanupReport {
                completed: 1,
                pending: 0
            }
        );
        assert!(!target.exists());
    }

    #[test]
    fn rf016_missing_directory_completes_and_batch_preserves_unselected_attachment() {
        let (base, vault) = setup();
        let owner = create_page(&vault, ACCOUNT, "owner").unwrap().id;
        for id in ["att_missing", "att_remove", "att_keep"] {
            attachment(&vault, &owner, &owner, id);
        }
        let remove = directory(base.path(), &owner, "att_remove");
        let keep = directory(base.path(), &owner, "att_keep");
        let report = purge_attachments(
            &vault,
            ACCOUNT,
            &owner,
            &["att_missing".into(), "att_remove".into()],
            base.path(),
        )
        .unwrap();
        assert_eq!(
            report,
            CleanupReport {
                completed: 2,
                pending: 0
            }
        );
        assert!(!remove.exists());
        assert!(keep.is_dir());
        assert_eq!(
            load_attachments(&vault.load_object(&owner).unwrap().unwrap().properties)[0].id,
            "att_keep"
        );
    }

    #[test]
    fn rf016_failed_path_remains_pending_and_fresh_retry_finishes() {
        let (base, vault) = setup();
        let owner = create_page(&vault, ACCOUNT, "owner").unwrap().id;
        attachment(&vault, &owner, &owner, "att_a");
        let parent = base.path().join("attachments").join(&owner);
        fs::create_dir_all(&parent).unwrap();
        let target = parent.join("att_a");
        fs::write(&target, b"not a directory").unwrap();
        let report =
            purge_attachments(&vault, ACCOUNT, &owner, &["att_a".into()], base.path()).unwrap();
        assert_eq!(
            report,
            CleanupReport {
                completed: 0,
                pending: 1
            }
        );
        let pending = vault.list_attachment_cleanup_intents(ACCOUNT).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(
            pending[0].last_error_code.as_deref(),
            Some("file_action_failed")
        );
        assert!(pending[0].attempts > 0);
        fs::remove_file(&target).unwrap();
        directory(base.path(), &owner, "att_a");
        assert_eq!(
            retry_attachment_cleanup(&vault, ACCOUNT, base.path()).unwrap(),
            CleanupReport {
                completed: 1,
                pending: 0
            }
        );
        assert!(!target.exists());
    }

    #[test]
    fn rf016_soft_deleted_current_reference_blocks_then_releases_cleanup() {
        let (base, vault) = setup();
        let owner = create_page(&vault, ACCOUNT, "owner").unwrap().id;
        let shared = create_page(&vault, ACCOUNT, "shared").unwrap().id;
        attachment(&vault, &owner, "storage", "att_a");
        attachment(&vault, &shared, "storage", "att_a");
        let mut other = vault.load_object(&shared).unwrap().unwrap();
        other.is_deleted = true;
        vault.save_object(&other).unwrap();
        let target = directory(base.path(), "storage", "att_a");
        assert_eq!(
            purge_attachments(&vault, ACCOUNT, &owner, &["att_a".into()], base.path()).unwrap(),
            CleanupReport {
                completed: 0,
                pending: 1
            }
        );
        assert!(target.is_dir());
        save_attachments(&mut other.properties, &[]);
        vault.save_object(&other).unwrap();
        assert_eq!(
            retry_attachment_cleanup(&vault, ACCOUNT, base.path()).unwrap(),
            CleanupReport {
                completed: 1,
                pending: 0
            }
        );
        assert!(!target.exists());
    }

    #[test]
    fn rf016_legacy_deleted_owner_rejection_and_report_api_soft_deleted_compatibility() {
        let (base, vault) = setup();
        let owner = create_page(&vault, ACCOUNT, "owner").unwrap().id;
        attachment(&vault, &owner, &owner, "att_a");
        let target = directory(base.path(), &owner, "att_a");
        assert!(
            purge_attachments(&vault, "foreign", &owner, &["att_a".into()], base.path()).is_err()
        );
        let mut record = vault.load_object(&owner).unwrap().unwrap();
        record.is_deleted = true;
        vault.save_object(&record).unwrap();
        assert!(
            crate::objects::purge_attachment(&vault, ACCOUNT, &owner, "att_a", base.path())
                .is_err()
        );
        assert!(target.is_dir());
        assert!(vault
            .list_attachment_cleanup_intents(ACCOUNT)
            .unwrap()
            .is_empty());
        assert_eq!(
            purge_attachments(&vault, ACCOUNT, &owner, &["att_a".into()], base.path()).unwrap(),
            CleanupReport {
                completed: 1,
                pending: 0
            }
        );
        assert!(!target.exists());
    }

    #[test]
    fn rf016_invalid_path_components_never_escape_the_attachment_root() {
        let base = TempDir::new().unwrap();
        let sentinel = base.path().join("sentinel");
        fs::write(&sentinel, b"keep").unwrap();
        for invalid in [
            "",
            "..",
            "../sentinel",
            "..\\sentinel",
            "/sentinel",
            "C:sentinel",
            "att. ",
            "CON",
        ] {
            let mut value = intent("storage", "att_a");
            value.storage_object_id = invalid.into();
            assert_eq!(
                remove_directory(base.path(), &value).unwrap_err(),
                "attachment_cleanup_invalid_id"
            );
        }
        assert_eq!(fs::read(sentinel).unwrap(), b"keep");
    }

    #[test]
    fn rf016_lock_before_queue_preserves_metadata_and_files() {
        let base = TempDir::new().unwrap();
        let service = VaultService::with_base_path(base.path().to_path_buf());
        service
            .create_account_with_id(ACCOUNT, "RF016", "password123", None)
            .unwrap();
        let session = service.capture_session(ACCOUNT).unwrap();
        let owner = create_page(session.vault(), ACCOUNT, "owner").unwrap().id;
        attachment(session.vault(), &owner, &owner, "att_a");
        let target = directory(base.path(), &owner, "att_a");
        service.lock();
        assert!(
            purge_attachments_for_session(&service, &session, &owner, &["att_a".into()]).is_err()
        );
        service.unlock(ACCOUNT, "password123").unwrap();
        let fresh = service.capture_session(ACCOUNT).unwrap();
        assert_eq!(
            load_attachments(
                &fresh
                    .vault()
                    .load_object(&owner)
                    .unwrap()
                    .unwrap()
                    .properties
            )
            .len(),
            1
        );
        assert!(target.is_dir());
    }

    #[test]
    fn rf016_lock_after_queue_keeps_intent_for_fresh_session_retry() {
        let base = TempDir::new().unwrap();
        let service = VaultService::with_base_path(base.path().to_path_buf());
        service
            .create_account_with_id(ACCOUNT, "RF016", "password123", None)
            .unwrap();
        let session = service.capture_session(ACCOUNT).unwrap();
        let owner = create_page(session.vault(), ACCOUNT, "owner").unwrap().id;
        attachment(session.vault(), &owner, &owner, "att_a");
        let target = directory(base.path(), &owner, "att_a");
        let intents = service
            .with_session(&session, |vault| {
                vault.queue_attachment_deletions(ACCOUNT, &owner, &["att_a".into()])
            })
            .unwrap();
        service.lock();
        assert_eq!(
            execute_for_session(&service, &session, &intents),
            CleanupReport {
                completed: 0,
                pending: 1
            }
        );
        assert!(target.is_dir());
        service.unlock(ACCOUNT, "password123").unwrap();
        let fresh = service.capture_session(ACCOUNT).unwrap();
        assert_eq!(
            retry_attachment_cleanup_for_session(&service, &fresh).unwrap(),
            CleanupReport {
                completed: 1,
                pending: 0
            }
        );
        assert!(!target.exists());
    }

    #[test]
    fn rf016_session_loss_between_items_reports_completed_and_all_remaining_pending() {
        let base = TempDir::new().unwrap();
        let service = VaultService::with_base_path(base.path().to_path_buf());
        service
            .create_account_with_id(ACCOUNT, "RF016", "password123", None)
            .unwrap();
        let session = service.capture_session(ACCOUNT).unwrap();
        let owner = create_page(session.vault(), ACCOUNT, "owner").unwrap().id;
        for id in ["att_a", "att_b"] {
            attachment(session.vault(), &owner, &owner, id);
        }
        let first = directory(base.path(), &owner, "att_a");
        let second = directory(base.path(), &owner, "att_b");
        let intents = service
            .with_session(&session, |vault| {
                vault.queue_attachment_deletions(ACCOUNT, &owner, &["att_a".into(), "att_b".into()])
            })
            .unwrap();
        let mut calls = 0;
        let report = execute(&intents, |intent| {
            let result = service.with_session(&session, |vault| {
                Ok(vault.run_attachment_cleanup_intent(intent, |stored| {
                    remove_directory(base.path(), stored)
                }))
            });
            calls += 1;
            if calls == 1 {
                service.lock();
            }
            result
        });
        assert_eq!(
            report,
            CleanupReport {
                completed: 1,
                pending: 1
            }
        );
        assert_ne!(first.exists(), second.exists());
        service.unlock(ACCOUNT, "password123").unwrap();
        let fresh = service.capture_session(ACCOUNT).unwrap();
        assert_eq!(
            retry_attachment_cleanup_for_session(&service, &fresh).unwrap(),
            CleanupReport {
                completed: 1,
                pending: 0
            }
        );
        assert!(!first.exists() && !second.exists());
    }

    #[cfg(unix)]
    #[test]
    fn rf016_configured_base_alias_is_allowed() {
        let container = TempDir::new().unwrap();
        let actual = container.path().join("actual");
        fs::create_dir(&actual).unwrap();
        let target = directory(&actual, "storage", "att_a");
        let alias = container.path().join("alias");
        std::os::unix::fs::symlink(&actual, &alias).unwrap();
        remove_directory(&alias, &intent("storage", "att_a")).unwrap();
        assert!(!target.exists());
    }

    #[cfg(unix)]
    #[test]
    fn rf016_ancestor_target_and_descendant_links_are_refused() {
        for level in 0..4 {
            let base = TempDir::new().unwrap();
            let outside = TempDir::new().unwrap();
            let sentinel = outside.path().join("sentinel");
            fs::write(&sentinel, b"keep").unwrap();
            let root = base.path().join("attachments");
            let object = root.join("storage");
            let target = object.join("att_a");
            let link = match level {
                0 => root,
                1 => {
                    fs::create_dir_all(&root).unwrap();
                    object
                }
                2 => {
                    fs::create_dir_all(&object).unwrap();
                    target.clone()
                }
                _ => {
                    directory(base.path(), "storage", "att_a");
                    target.join("linked")
                }
            };
            std::os::unix::fs::symlink(outside.path(), link).unwrap();
            assert_eq!(
                remove_directory(base.path(), &intent("storage", "att_a")).unwrap_err(),
                "attachment_cleanup_unsafe_path"
            );
            assert_eq!(fs::read(sentinel).unwrap(), b"keep");
            if level == 3 {
                assert!(target.join("payload.bin").is_file());
            }
        }
    }

    #[cfg(windows)]
    #[test]
    fn rf016_windows_junction_ancestors_targets_and_descendants_are_refused() {
        use std::os::windows::process::CommandExt;
        for level in 0..4 {
            let base = TempDir::new().unwrap();
            let outside = TempDir::new().unwrap();
            let sentinel = outside.path().join("sentinel");
            fs::write(&sentinel, b"keep").unwrap();
            let root = base.path().join("attachments");
            let object = root.join("storage");
            let target = object.join("att_a");
            let link = match level {
                0 => root,
                1 => {
                    fs::create_dir_all(&root).unwrap();
                    object
                }
                2 => {
                    fs::create_dir_all(&object).unwrap();
                    target.clone()
                }
                _ => {
                    directory(base.path(), "storage", "att_a");
                    target.join("linked")
                }
            };
            let result = std::process::Command::new("cmd.exe")
                .args(["/d", "/c", "mklink", "/J"])
                .arg(&link)
                .arg(outside.path())
                .creation_flags(0x08000000)
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "temporary junction creation failed"
            );
            assert_eq!(
                remove_directory(base.path(), &intent("storage", "att_a")).unwrap_err(),
                "attachment_cleanup_unsafe_path"
            );
            assert_eq!(fs::read(sentinel).unwrap(), b"keep");
            if level == 3 {
                assert!(target.join("payload.bin").is_file());
            }
            fs::remove_dir(&link).unwrap();
        }
    }
}
