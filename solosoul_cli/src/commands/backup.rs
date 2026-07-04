//! 备份命令：/backup list、create、restore、delete。

use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use crate::app::{App, AppPhase};
use crate::commands::{require_unlocked};
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

/// 备份清单结构。
#[derive(Serialize)]
struct BackupManifest {
    version: String,
    created_at: String,
    profile_count: usize,
    profiles: Vec<BackupProfileEntry>,
}

#[derive(Serialize)]
struct BackupProfileEntry {
    id: String,
    name: String,
    data: Vec<u8>,
    created_at: String,
    updated_at: String,
    version: u32,
}

/// 恢复时读取的清单结构。
/// `_version` / `_created_at` / `_profile_count` 保留以维持旧版备份 JSON 反序列化兼容性。
#[derive(Deserialize)]
struct RestoreManifest {
    #[serde(rename = "version")]
    _version: String,
    #[serde(rename = "created_at")]
    _created_at: String,
    #[serde(rename = "profile_count")]
    _profile_count: usize,
    profiles: Vec<RestoreProfileEntry>,
}

#[derive(Deserialize)]
struct RestoreProfileEntry {
    id: String,
    name: String,
    data: Vec<u8>,
    created_at: String,
    updated_at: String,
    version: u32,
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
        Err(color_eyre::eyre::eyre!("备份名称不能为空"))
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
                app.error_message = Some("请提供备份名称，例如 /backup create weekly".to_string());
                Ok(())
            }
        },
        "restore" => match args.get(1).copied() {
            Some(id) => backup_restore(app, id),
            None => {
                app.error_message =
                    Some("请提供备份 ID，例如 /backup restore weekly_20260101_120000".to_string());
                Ok(())
            }
        },
        "delete" => match args.get(1).copied() {
            Some(id) => backup_delete(app, id),
            None => {
                app.error_message =
                    Some("请提供备份 ID，例如 /backup delete weekly_20260101_120000".to_string());
                Ok(())
            }
        },
        _ => {
            app.error_message =
                Some("用法：/backup list | create <name> | restore <id> | delete <id>".to_string());
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
    let _account_id = require_unlocked(app)?;
    let vault = app
        .vault_service
        .get_vault_store()
        .ok_or_else(|| color_eyre::eyre::eyre!("Vault 未打开"))?;

    let safe_name = sanitize_backup_name(name)?;
    let backup_dir = backups_dir(app);
    fs::create_dir_all(&backup_dir).map_err(|e| {
        app.error_message = Some(format!("创建备份目录失败: {}", e));
        color_eyre::eyre::eyre!(e)
    })?;

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let backup_path = backup_dir.join(format!("{}_{}.solosoul_backup", safe_name, timestamp));

    let profiles = vault.list_profiles().map_err(|e| color_eyre::eyre::eyre!(e))?;
    let mut backup_profiles = Vec::new();
    for summary in &profiles {
        if let Ok(Some(profile)) = vault.load_profile(&summary.id) {
            backup_profiles.push(BackupProfileEntry {
                id: profile.id,
                name: profile.name,
                data: profile.data,
                created_at: profile.created_at.to_rfc3339(),
                updated_at: profile.updated_at.to_rfc3339(),
                version: profile.version,
            });
        }
    }

    let manifest = BackupManifest {
        version: "2.0".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        profile_count: backup_profiles.len(),
        profiles: backup_profiles,
    };

    let json = serde_json::to_string_pretty(&manifest).map_err(|e| {
        app.error_message = Some(format!("序列化备份清单失败: {}", e));
        color_eyre::eyre::eyre!(e)
    })?;

    fs::write(&backup_path, json).map_err(|e| {
        app.error_message = Some(format!("写入备份文件失败: {}", e));
        color_eyre::eyre::eyre!(e)
    })?;

    let metadata = fs::metadata(&backup_path).map_err(|e| {
        app.error_message = Some(format!("读取备份文件元信息失败: {}", e));
        color_eyre::eyre::eyre!(e)
    })?;

    let id = format!("{}_{}", safe_name, timestamp);
    app.error_message = Some(format!(
        "备份已创建: {} ({} profiles, {} bytes)",
        id,
        profiles.len(),
        metadata.len()
    ));
    Ok(())
}

/// `/backup restore <id>`：精确匹配文件名 stem，确认后恢复 Profile。
fn backup_restore(app: &mut App, backup_id: &str) -> Result<()> {
    let _account_id = require_unlocked(app)?;
    let dir = backups_dir(app);
    let path = find_backup_path(&dir, backup_id)?;
    let (created_at, profile_count) = read_manifest_summary(&path).unwrap_or_default();

    let message = format!(
        "确认恢复备份 '{}'？\n创建时间: {}\n包含 {} 个 Profile。\n当前 Vault 中的同名 Profile 将被覆盖。",
        backup_id, created_at, profile_count
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
                    app.error_message = Some(format!("恢复备份失败: {}", e));
                } else {
                    app.error_message = Some(format!("备份 '{}' 已恢复", backup_id));
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
        app.error_message = Some(format!("读取备份文件失败: {}", e));
        color_eyre::eyre::eyre!(e)
    })?;

    let manifest: RestoreManifest = serde_json::from_str(&content).map_err(|e| {
        app.error_message = Some(format!("解析备份清单失败: {}", e));
        color_eyre::eyre::eyre!(e)
    })?;

    use solosoul_core::Profile;
    for entry in &manifest.profiles {
        let profile = Profile {
            id: entry.id.clone(),
            name: entry.name.clone(),
            data: entry.data.clone(),
            created_at: chrono::DateTime::parse_from_rfc3339(&entry.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: chrono::DateTime::parse_from_rfc3339(&entry.updated_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            version: entry.version,
        };
        vault.save_profile(&profile).map_err(|e| color_eyre::eyre::eyre!(e))?;
    }

    Ok(())
}

/// `/backup delete <id>`：精确匹配文件名 stem，确认后删除备份文件。
fn backup_delete(app: &mut App, backup_id: &str) -> Result<()> {
    let dir = backups_dir(app);
    let _path = find_backup_path(&dir, backup_id)?;

    let message = format!("确认删除备份 '{}'？\n此操作不可恢复。", backup_id);
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
                    app.error_message = Some(format!("删除备份失败: {}", e));
                } else {
                    app.error_message = Some(format!("备份 '{}' 已删除", backup_id));
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
        app.error_message = Some(format!("删除备份文件失败: {}", e));
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

    fn unlocked_app() -> (App, String, tempfile::TempDir) {
        let _guard = crate::VAULT_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::TempDir::new().unwrap();
        let vault = VaultService::with_base_path(dir.path().to_path_buf());
        let account = vault
            .create_account("Test", crate::TEST_PASSWORD, None)
            .unwrap();
        let account_id = account["id"].as_str().unwrap().to_string();
        let app = App::new(Arc::new(vault)).unwrap();
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

    fn confirm_prompt(app: &mut App) {
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

    #[test]
    fn test_backup_restore() {
        let (mut app, _id, _dir) = unlocked_app();
        create_test_profile(&mut app, "OriginalProfile");

        handle(&mut app, &["create", "snapshot"]).unwrap();
        let id = first_backup_id(&mut app);

        handle(&mut app, &["restore", &id]).unwrap();
        confirm_prompt(&mut app);

        assert!(
            app.error_message
                .as_deref()
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
            app.error_message
                .as_deref()
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
}
