//! 备份命令：/backup list、create、restore、delete。

use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::Result;
use serde::{Deserialize, Serialize};
use solosoul_core::backup::{
    decode_profile_backup, encode_profile_backup, ProfileBackupError, ProfilePayloadEncoding,
};
use std::time::Instant;

use crate::app::{App, AppPhase};
use crate::commands::require_unlocked;
use crate::commands::require_unlocked_with_vault;
use crate::t;
use crate::widgets::prompt::{self, PromptResult, PromptSpec};

/// 备份信息，与 GUI 的 `BackupInfo` 字段保持一致。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub size_bytes: u64,
    pub object_count: usize,
}

/// 备份清单摘要，仅用于列表展示。
#[derive(Deserialize)]
struct BackupManifestHeader {
    created_at: String,
    profile_count: usize,
}

fn backups_dir(app: &App) -> PathBuf {
    app.vault_service.base_path().join("backups")
}

/// 清理备份名称：仅保留字母、数字、连字符和下划线，其余替换为下划线。
fn sanitize_backup_name(name: &str) -> Result<String> {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        Err(color_eyre::eyre::eyre!("Backup name cannot be empty"))
    } else {
        Ok(sanitized)
    }
}

/// 命令路由入口。
pub fn handle(app: &mut App, args: &[&str]) -> Result<()> {
    let sub = args.first().copied().unwrap_or("");
    match sub {
        "list" => backup_list(app),
        "create" => match args.get(1).copied() {
            Some(name) => backup_create(app, name),
            None => {
                app.error_message = Some(t!(app.i18n, "cmd-provide-backup-name"));
                Ok(())
            }
        },
        "restore" => match args.get(1).copied() {
            Some(id) => backup_restore(app, id),
            None => {
                app.error_message = Some(t!(app.i18n, "cmd-provide-backup-id", cmd = "restore"));
                Ok(())
            }
        },
        "delete" => match args.get(1).copied() {
            Some(id) => backup_delete(app, id),
            None => {
                app.error_message = Some(t!(app.i18n, "cmd-provide-backup-id", cmd = "delete"));
                Ok(())
            }
        },
        _ => {
            app.error_message = Some(t!(app.i18n, "cmd-backup-usage"));
            Ok(())
        }
    }
}

/// `/backup list`：列出 `{base}/backups/` 下的备份文件。
fn backup_list(app: &mut App) -> Result<()> {
    let items = list_backup_infos(app)?;
    app.previous_phase = Some(app.phase.clone());
    app.phase = AppPhase::BackupList { items, selected: 0 };
    Ok(())
}

fn list_backup_infos(app: &App) -> Result<Vec<BackupInfo>> {
    let dir = backups_dir(app);
    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut backups = Vec::new();
    for entry in
        fs::read_dir(&dir).map_err(|e| color_eyre::eyre::eyre!("读取备份目录失败: {}", e))?
    {
        let entry = entry.map_err(|e| color_eyre::eyre::eyre!("读取目录项失败: {}", e))?;
        let path = entry.path();
        if let Some(info) = read_backup_info(&path) {
            backups.push(info);
        }
    }

    backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(backups)
}

fn read_backup_info(path: &Path) -> Option<BackupInfo> {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    if ext != "solosoul_backup" && ext != "zip" {
        return None;
    }

    let metadata = fs::metadata(path).ok()?;
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let created_at = metadata
        .created()
        .ok()
        .and_then(|t| {
            let secs = t.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64;
            chrono::DateTime::from_timestamp(secs, 0).map(|dt| dt.to_rfc3339())
        })
        .unwrap_or_default();

    let (manifest_created, profile_count) = read_manifest_summary(path).unwrap_or_default();
    let created_at = if manifest_created.is_empty() {
        created_at
    } else {
        manifest_created
    };

    Some(BackupInfo {
        id: name.clone(),
        name,
        created_at,
        size_bytes: metadata.len(),
        object_count: profile_count,
    })
}

fn read_manifest_summary(path: &Path) -> Option<(String, usize)> {
    let content = fs::read_to_string(path).ok()?;
    let header: BackupManifestHeader = serde_json::from_str(&content).ok()?;
    Some((header.created_at, header.profile_count))
}

/// `/backup create <name>`：创建包含全部 Profile 的备份。
fn backup_create(app: &mut App, name: &str) -> Result<()> {
    let (_account_id, vault) = require_unlocked_with_vault(app)?;

    let safe_name = sanitize_backup_name(name)?;
    let backup_dir = backups_dir(app);
    fs::create_dir_all(&backup_dir).map_err(|e| {
        app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e));
        color_eyre::eyre::eyre!(e)
    })?;

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let backup_path = backup_dir.join(format!("{}_{}.solosoul_backup", safe_name, timestamp));

    let profiles = vault
        .list_profiles()
        .map_err(|e| color_eyre::eyre::eyre!(e))?;
    let mut backup_profiles = Vec::new();
    for summary in &profiles {
        // P033：单个 profile 加载/解密失败不再静默跳过——备份不完整必须中止并报告
        let profile = vault
            .load_profile(&summary.id)
            .map_err(|e| {
                app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e));
                color_eyre::eyre::eyre!("备份中止：Profile {} 加载失败: {}", summary.id, e)
            })?
            .ok_or_else(|| {
                app.error_message =
                    Some(t!(app.i18n, "cmd-operation-failed", err = "Profile 不存在"));
                color_eyre::eyre::eyre!("备份中止：Profile {} 不存在", summary.id)
            })?;
        backup_profiles.push(profile);
    }

    let json = encode_profile_backup(
        &backup_profiles,
        chrono::Utc::now(),
        ProfilePayloadEncoding::ByteArray,
    )
    .map_err(|e| {
        app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e));
        color_eyre::eyre::eyre!(e)
    })?;

    // P007 复核：备份内容为解密后明文（profile.data 未加密），统一走共享
    // write_private_file——创建时即定 0600 权限，无先写后 chmod 的明文窗口期。
    crate::util::write_private_file(&backup_path, &json).map_err(|e| {
        app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e));
        color_eyre::eyre::eyre!(e)
    })?;

    let metadata = fs::metadata(&backup_path).map_err(|e| {
        app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e));
        color_eyre::eyre::eyre!(e)
    })?;

    let id = format!("{}_{}", safe_name, timestamp);
    app.success_message = Some((
        t!(
            app.i18n,
            "cmd-backup-created",
            id = id,
            size = profiles.len().to_string(),
            bytes = metadata.len().to_string()
        ),
        Instant::now(),
    ));
    Ok(())
}

/// `/backup restore <id>`：精确匹配文件名 stem，确认后恢复 Profile。
fn backup_restore(app: &mut App, backup_id: &str) -> Result<()> {
    let _account_id = require_unlocked(app)?;
    let dir = backups_dir(app);
    let path = find_backup_path(&dir, backup_id)?;
    let (created_at, profile_count) = read_manifest_summary(&path).unwrap_or_default();

    let message = t!(
        app.i18n,
        "cmd-prompt-restore-backup",
        id = &backup_id,
        date = &created_at,
        count = &profile_count.to_string()
    );
    let backup_id = backup_id.to_string();
    prompt::open(
        app,
        PromptSpec::Confirm {
            message,
            default_yes: false,
        },
        Box::new(move |app, result| {
            if let PromptResult::Confirm(true) = result {
                if let Err(e) = do_restore(app, &backup_id) {
                    app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e));
                } else {
                    app.success_message = Some((
                        t!(app.i18n, "cmd-backup-restored", id = backup_id),
                        Instant::now(),
                    ));
                }
            }
        }),
    );

    Ok(())
}

fn do_restore(app: &mut App, backup_id: &str) -> Result<()> {
    let vault = app
        .vault_service
        .get_vault_store()
        .ok_or_else(|| color_eyre::eyre::eyre!("Vault 未打开"))?;
    let dir = backups_dir(app);
    let path = find_backup_path(&dir, backup_id)?;
    let content = fs::read_to_string(&path).map_err(|e| {
        app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e));
        color_eyre::eyre::eyre!(e)
    })?;

    // RF-013：共用 RF-011 的完整解码规则，任何坏条目都在首次保存前拒绝。
    let decoded =
        decode_profile_backup(content.as_bytes(), chrono::Utc::now()).map_err(|error| {
            let message = match error {
                ProfileBackupError::InvalidBase64 { reason, .. } => {
                    format!("Profile 数据 Base64 无效: {}", reason)
                }
                ProfileBackupError::MissingData { .. } => "Profile 缺少备份数据".to_string(),
                other => other.to_string(),
            };
            app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = &message));
            color_eyre::eyre::eyre!(message)
        })?;
    for profile in decoded.profiles {
        vault
            .save_profile(&profile)
            .map_err(|e| color_eyre::eyre::eyre!(e))?;
    }

    Ok(())
}

/// `/backup delete <id>`：精确匹配文件名 stem，确认后删除备份文件。
fn backup_delete(app: &mut App, backup_id: &str) -> Result<()> {
    let dir = backups_dir(app);
    let _path = find_backup_path(&dir, backup_id)?;

    let message = t!(app.i18n, "cmd-prompt-delete-backup", id = &backup_id);
    let backup_id = backup_id.to_string();
    prompt::open(
        app,
        PromptSpec::Confirm {
            message,
            default_yes: false,
        },
        Box::new(move |app, result| {
            if let PromptResult::Confirm(true) = result {
                if let Err(e) = do_delete(app, &backup_id) {
                    app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e));
                } else {
                    app.success_message = Some((
                        t!(app.i18n, "cmd-backup-deleted", id = backup_id),
                        Instant::now(),
                    ));
                }
            }
        }),
    );

    Ok(())
}

fn do_delete(app: &mut App, backup_id: &str) -> Result<()> {
    let dir = backups_dir(app);
    let path = find_backup_path(&dir, backup_id)?;
    fs::remove_file(&path).map_err(|e| {
        app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e));
        color_eyre::eyre::eyre!(e)
    })
}

/// 精确匹配备份文件 stem。
fn find_backup_path(dir: &Path, backup_id: &str) -> Result<PathBuf> {
    if !dir.exists() {
        return Err(color_eyre::eyre::eyre!("备份 '{}' 不存在", backup_id));
    }

    for entry in
        fs::read_dir(dir).map_err(|e| color_eyre::eyre::eyre!("读取备份目录失败: {}", e))?
    {
        let entry = entry.map_err(|e| color_eyre::eyre::eyre!("读取目录项失败: {}", e))?;
        let path = entry.path();
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            if stem == backup_id {
                return Ok(path);
            }
        }
    }

    Err(color_eyre::eyre::eyre!("备份 '{}' 不存在", backup_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent};
    use solosoul_core::{Profile, VaultService};
    use std::sync::Arc;

    pub(super) fn unlocked_app() -> (App, String, tempfile::TempDir) {
        let _guard = crate::VAULT_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::TempDir::new().unwrap();
        let vault = VaultService::with_base_path(dir.path().to_path_buf());
        let account = vault
            .create_account("Test", crate::TEST_PASSWORD, None)
            .unwrap();
        let account_id = account["id"].as_str().unwrap().to_string();
        let mut app = App::new(Arc::new(vault)).unwrap();
        // 这些用例断言中文文案，不依赖运行测试的系统语言。
        app.i18n.set_locale("zh-CN");
        (app, account_id, dir)
    }

    fn create_test_profile(app: &mut App, name: &str) {
        let vault = app.vault_service.get_vault_store().unwrap();
        vault
            .save_profile(&Profile::new(name, b"test data".to_vec()))
            .unwrap();
    }

    fn first_backup_id(app: &mut App) -> String {
        handle(app, &["list"]).unwrap();
        match &app.phase {
            AppPhase::BackupList { items, .. } => items
                .first()
                .map(|i| i.id.clone())
                .expect("测试应至少有一个备份"),
            _ => panic!("expected BackupList"),
        }
    }

    pub(super) fn confirm_prompt(app: &mut App) {
        // 默认选中“否”，需要先切换到“是”再确认。
        crate::widgets::prompt::handle_key(app, KeyEvent::from(KeyCode::Left));
        crate::widgets::prompt::handle_key(app, KeyEvent::from(KeyCode::Enter));
    }

    #[test]
    fn test_backup_create_and_list() {
        let (mut app, _id, _dir) = unlocked_app();
        create_test_profile(&mut app, "TestProfile");

        handle(&mut app, &["create", "weekly"]).unwrap();
        let backups_dir = app.vault_service.base_path().join("backups");
        let entries: Vec<_> = std::fs::read_dir(&backups_dir).unwrap().collect();
        assert_eq!(entries.len(), 1);

        handle(&mut app, &["list"]).unwrap();
        match &app.phase {
            AppPhase::BackupList { items, selected } => {
                assert_eq!(items.len(), 1);
                assert_eq!(*selected, 0);
                assert!(items[0].id.starts_with("weekly_"));
                assert_eq!(items[0].object_count, 1);
                assert!(items[0].size_bytes > 0);
            }
            _ => panic!("expected BackupList"),
        }
    }

    /// P007: 备份为解密后明文，落盘权限须收紧为 0600。
    #[cfg(unix)]
    #[test]
    fn test_backup_create_sets_0600_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let (mut app, _id, _dir) = unlocked_app();
        create_test_profile(&mut app, "TestProfile");

        handle(&mut app, &["create", "weekly"]).unwrap();
        let backups_dir = app.vault_service.base_path().join("backups");
        let entry = std::fs::read_dir(&backups_dir)
            .unwrap()
            .next()
            .unwrap()
            .unwrap();
        let mode = entry.path().metadata().unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "备份文件权限应为 0600，实际 {:#o}", mode);
    }

    /// P007 复核: 旧版本遗留的 0644 备份文件被覆写时也应收紧为 0600
    ///（write_private_file 对已存在文件显式 set_permissions，不再只靠 mode 新建语义）。
    #[cfg(unix)]
    #[test]
    fn test_backup_overwrite_tightens_existing_0644() {
        use std::os::unix::fs::PermissionsExt;
        let (mut app, _id, _dir) = unlocked_app();
        create_test_profile(&mut app, "TestProfile");

        handle(&mut app, &["create", "weekly"]).unwrap();
        let backups_dir = app.vault_service.base_path().join("backups");
        let entry = std::fs::read_dir(&backups_dir)
            .unwrap()
            .next()
            .unwrap()
            .unwrap();
        let path = entry.path();

        // 模拟旧版本遗留的 0644
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

        // 再次创建同名备份（覆盖写入）
        handle(&mut app, &["create", "weekly"]).unwrap();

        let mode = path.metadata().unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "覆写后权限应为 0600，实际 {:#o}", mode);
    }

    /// P033：单个 profile 解密失败时备份必须中止，不得静默跳过产生不完整备份。
    #[test]
    fn test_backup_create_aborts_on_unreadable_profile() {
        use zeroize::Zeroizing;
        let (mut app, account_id, _dir) = unlocked_app();
        create_test_profile(&mut app, "CorruptMe");

        let profile_id = {
            let vault = app.vault_service.get_vault_store().unwrap();
            vault.list_profiles().unwrap()[0].id.clone()
        };

        // 锁定关闭连接后，直接破坏该 profile 的密文，再重新解锁
        app.vault_service.lock();
        let db_path = app
            .vault_service
            .base_path()
            .join(&account_id)
            .join("vault.db");
        // 保留 SOLO blob magic（SOLO = 534F4C4F）的垃圾密文才能触发真实解密失败
        // （decrypt_field 对无 magic 的短数据按旧版明文直接放行）。
        let out = std::process::Command::new("sqlite3")
            .args([
                db_path.to_str().unwrap(),
                &format!(
                    "UPDATE profiles SET data = X'534F4C4FDEADBEEF' WHERE id = '{}';",
                    profile_id
                ),
            ])
            .output()
            .expect("sqlite3 CLI 应可用");
        assert!(
            out.status.success(),
            "sqlite3 更新失败: {}",
            String::from_utf8_lossy(&out.stderr)
        );

        app.vault_service
            .unlock_secure(
                &account_id,
                &Zeroizing::new(crate::TEST_PASSWORD.to_string()),
            )
            .unwrap();

        // 备份应中止并报告，而非静默跳过
        let err = handle(&mut app, &["create", "broken"]).unwrap_err();
        assert!(err.to_string().contains("备份中止"), "{}", err);

        // 未生成任何备份文件
        let backups_dir = app.vault_service.base_path().join("backups");
        let entries: Vec<_> = std::fs::read_dir(&backups_dir).unwrap().collect();
        assert!(entries.is_empty(), "不应生成不完整备份");
    }

    #[test]
    fn test_backup_restore() {
        let (mut app, _id, _dir) = unlocked_app();
        create_test_profile(&mut app, "OriginalProfile");

        handle(&mut app, &["create", "snapshot"]).unwrap();
        let id = first_backup_id(&mut app);

        handle(&mut app, &["restore", &id]).unwrap();
        confirm_prompt(&mut app);

        assert!(
            app.success_message
                .as_ref()
                .map(|(s, _)| s.as_str())
                .unwrap_or("")
                .contains("已恢复"),
            "expected restore success message, got {:?}",
            app.error_message
        );

        let vault = app.vault_service.get_vault_store().unwrap();
        let profiles = vault.list_profiles().unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].name, "OriginalProfile");
    }

    #[test]
    fn test_backup_delete() {
        let (mut app, _id, _dir) = unlocked_app();
        create_test_profile(&mut app, "ToDelete");

        handle(&mut app, &["create", "temp"]).unwrap();
        let id = first_backup_id(&mut app);
        let path = app
            .vault_service
            .base_path()
            .join("backups")
            .join(format!("{}.solosoul_backup", id));
        assert!(path.exists());

        handle(&mut app, &["delete", &id]).unwrap();
        confirm_prompt(&mut app);

        assert!(
            app.success_message
                .as_ref()
                .map(|(s, _)| s.as_str())
                .unwrap_or("")
                .contains("已删除"),
            "expected delete success message, got {:?}",
            app.error_message
        );
        assert!(!path.exists());
    }

    #[test]
    fn test_backup_restore_not_found() {
        let (mut app, _id, _dir) = unlocked_app();
        assert!(handle(&mut app, &["restore", "missing_id"]).is_err());
    }

    #[test]
    fn test_backup_delete_not_found() {
        let (mut app, _id, _dir) = unlocked_app();
        assert!(handle(&mut app, &["delete", "missing_id"]).is_err());
    }

    fn rf011_entry(id: &str, payload: serde_json::Value) -> serde_json::Value {
        let mut entry = serde_json::json!({
            "id": id, "name": "历史 Profile", "version": 3,
            "created_at": "2025-01-02T03:04:05Z",
            "updated_at": "2026-02-03T04:05:06Z"
        });
        entry
            .as_object_mut()
            .unwrap()
            .extend(payload.as_object().unwrap().clone());
        entry
    }

    fn rf011_write_backup(app: &App, entries: Vec<serde_json::Value>) {
        let dir = backups_dir(app);
        fs::create_dir_all(&dir).unwrap();
        let manifest = serde_json::json!({
            "version": "2.0", "created_at": "2026-09-25T00:00:00Z",
            "profile_count": entries.len(), "profiles": entries
        });
        crate::util::write_private_file(
            &dir.join("rf011.solosoul_backup"),
            &serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn rf011_restores_gui_and_legacy_profile_bytes_and_metadata() {
        let (mut app, _id, _dir) = unlocked_app();
        // 固定 GUI 2.0 线格式，含非 UTF-8 字节，不能只验证文本或空清单。
        rf011_write_backup(
            &app,
            vec![
                rf011_entry("gui", serde_json::json!({"data_b64": "AAH+/2Zvbw=="})),
                rf011_entry(
                    "legacy",
                    serde_json::json!({"data": [0, 1, 254, 255, 102, 111, 111]}),
                ),
            ],
        );
        handle(&mut app, &["restore", "rf011"]).unwrap();
        let vault = app.vault_service.get_vault_store().unwrap();
        assert!(vault.list_profiles().unwrap().is_empty(), "确认前不能写入");
        let restore_started_at = chrono::Utc::now();
        confirm_prompt(&mut app);
        let restore_finished_at = chrono::Utc::now();
        assert!(app.error_message.is_none(), "{:?}", app.error_message);
        assert!(app.success_message.is_some());
        for id in ["gui", "legacy"] {
            let restored = vault.load_profile(id).unwrap().unwrap();
            assert_eq!(restored.data, vec![0, 1, 254, 255, 102, 111, 111]);
            assert_eq!(restored.name, "历史 Profile");
            assert_eq!(restored.version, 3);
            assert_eq!(
                restored.created_at.to_rfc3339(),
                "2025-01-02T03:04:05+00:00"
            );
            // Vault 保存时统一更新修改时间，恢复沿用这条现有存储规则。
            assert!(restored.updated_at >= restore_started_at);
            assert!(restored.updated_at <= restore_finished_at);
        }
    }

    #[test]
    fn rf011_dual_fields_and_explicit_empty_payloads_are_compatible() {
        let (mut app, _id, _dir) = unlocked_app();
        let cases = [
            (
                "preferred",
                serde_json::json!({"data_b64": "Zm9v", "data": [42]}),
                b"foo".to_vec(),
            ),
            (
                "fallback",
                serde_json::json!({"data_b64": "", "data": [42]}),
                vec![42],
            ),
            ("empty_b64", serde_json::json!({"data_b64": ""}), vec![]),
            ("empty_array", serde_json::json!({"data": []}), vec![]),
        ];
        rf011_write_backup(
            &app,
            cases
                .iter()
                .map(|(id, payload, _)| rf011_entry(id, payload.clone()))
                .collect(),
        );
        do_restore(&mut app, "rf011").unwrap();
        let vault = app.vault_service.get_vault_store().unwrap();
        for (id, _, expected) in &cases {
            assert_eq!(&vault.load_profile(id).unwrap().unwrap().data, expected);
        }
        // 空 profiles 清单仍是有效备份，且不会删除现有 Profile。
        rf011_write_backup(&app, vec![]);
        do_restore(&mut app, "rf011").unwrap();
        assert_eq!(vault.list_profiles().unwrap().len(), cases.len());
        assert_eq!(
            vault.load_profile("preferred").unwrap().unwrap().data,
            b"foo"
        );
    }

    #[test]
    fn rf011_invalid_later_entry_does_not_write_any_profile() {
        let (mut app, _id, _dir) = unlocked_app();
        let vault = app.vault_service.get_vault_store().unwrap();
        let mut original = Profile::new("original", b"keep secret".to_vec());
        original.id = "existing".to_owned();
        vault.save_profile(&original).unwrap();
        let original = vault.load_profile("existing").unwrap().unwrap();
        for invalid in [
            serde_json::json!({"data_b64": "!invalid", "data": [42]}),
            serde_json::json!({"data_b64": "Zg="}),
            serde_json::json!({}),
            serde_json::json!({"data_b64": null, "data": null}),
            serde_json::json!({"data_b64": 42}),
            serde_json::json!({"data": [256]}),
        ] {
            rf011_write_backup(
                &app,
                vec![
                    rf011_entry("existing", serde_json::json!({"data_b64": "Zm9v"})),
                    rf011_entry("invalid", invalid),
                ],
            );
            assert!(do_restore(&mut app, "rf011").is_err());
            let actual = vault.load_profile("existing").unwrap().unwrap();
            assert_eq!(actual.data, original.data);
            assert_eq!(actual.name, original.name);
            assert_eq!(actual.version, original.version);
            assert_eq!(actual.created_at, original.created_at);
            assert_eq!(actual.updated_at, original.updated_at);
            assert!(vault.load_profile("invalid").unwrap().is_none());
            assert_eq!(vault.list_profiles().unwrap().len(), 1);
        }
    }

    #[test]
    fn rf011_cancel_and_decode_error_do_not_report_success() {
        let (mut app, _id, _dir) = unlocked_app();
        rf011_write_backup(
            &app,
            vec![rf011_entry("gui", serde_json::json!({"data_b64": "Zm9v"}))],
        );
        handle(&mut app, &["restore", "rf011"]).unwrap();
        crate::widgets::prompt::handle_key(&mut app, KeyEvent::from(KeyCode::Enter));
        let vault = app.vault_service.get_vault_store().unwrap();
        assert!(vault.list_profiles().unwrap().is_empty());
        assert!(app.success_message.is_none());

        rf011_write_backup(
            &app,
            vec![rf011_entry(
                "gui",
                serde_json::json!({"data_b64": "invalid!"}),
            )],
        );
        handle(&mut app, &["restore", "rf011"]).unwrap();
        confirm_prompt(&mut app);
        assert!(app.error_message.as_deref().unwrap().contains("Base64"));
        assert!(app.success_message.is_none());
        assert!(vault.list_profiles().unwrap().is_empty());
    }
}

#[cfg(test)]
mod rf013_tests;
