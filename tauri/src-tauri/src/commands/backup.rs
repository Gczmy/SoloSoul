//! Backup commands — create, list, restore, delete vault backups

use serde::{Deserialize, Serialize};
use solosoul_core::backup::{
    decode_profile_backup, encode_profile_backup, ProfileBackupError, ProfilePayloadEncoding,
};
use std::fs;
use std::path::PathBuf;
use tauri::State;

use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub size_bytes: u64,
    pub object_count: usize,
}

fn backups_dir(base_path: &std::path::Path) -> PathBuf {
    base_path.join("backups")
}

/// R009: restrict backup names to alphanumeric, hyphen and underscore to avoid
/// path traversal and produce predictable file names.
fn sanitize_backup_name(name: &str) -> Result<String, String> {
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
        return Err("Backup name cannot be empty".to_string());
    }
    Ok(sanitized)
}

#[tauri::command]
pub async fn backup_list(state: State<'_, AppState>) -> Result<Vec<BackupInfo>, String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let backup_dir = backups_dir(svc.base_path());
    if !backup_dir.exists() {
        return Ok(vec![]);
    }

    let mut backups = Vec::new();
    let dir = fs::read_dir(&backup_dir).map_err(|e| e.to_string())?;

    for entry in dir {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if ext != "solosoul_backup" && ext != "zip" {
            continue;
        }

        let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let created_at = metadata
            .created()
            .ok()
            .and_then(|t| {
                let dur = t.duration_since(std::time::UNIX_EPOCH).ok()?;
                let secs = dur.as_secs() as i64;
                chrono::DateTime::from_timestamp(secs, 0).map(|dt| dt.to_rfc3339())
            })
            .unwrap_or_default();

        backups.push(BackupInfo {
            id: name.clone(),
            name,
            created_at,
            size_bytes: metadata.len(),
            object_count: 0,
        });
    }
    backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(backups)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_info_serde_roundtrip() {
        let info = BackupInfo {
            id: "backup-20240601_120000".to_string(),
            name: "My Backup".to_string(),
            created_at: "2024-06-01T12:00:00Z".to_string(),
            size_bytes: 4096,
            object_count: 5,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"id\":\"backup-20240601_120000\""));
        assert!(json.contains("\"name\":\"My Backup\""));
        // Struct uses default serde (snake_case, no rename_all)
        assert!(json.contains("\"size_bytes\":4096"));

        let restored: BackupInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, info.id);
        assert_eq!(restored.name, info.name);
        assert_eq!(restored.size_bytes, info.size_bytes);
        assert_eq!(restored.object_count, info.object_count);
    }

    #[test]
    fn test_sanitize_backup_name_replaces_special_chars() {
        assert_eq!(sanitize_backup_name("hello world").unwrap(), "hello_world");
        assert_eq!(sanitize_backup_name("a/b/c").unwrap(), "a_b_c");
        assert_eq!(sanitize_backup_name("my.backup@2").unwrap(), "my_backup_2");
        assert_eq!(
            sanitize_backup_name("../../etc/passwd").unwrap(),
            "______etc_passwd"
        );
    }

    #[test]
    fn test_sanitize_backup_name_preserves_allowed_chars() {
        let result = sanitize_backup_name("My-Backup_2024").unwrap();
        assert_eq!(result, "My-Backup_2024");
    }

    #[test]
    fn test_sanitize_backup_name_preserves_alphanumeric() {
        let result = sanitize_backup_name("Backup42").unwrap();
        assert_eq!(result, "Backup42");
    }

    #[test]
    fn test_sanitize_backup_name_empty_fails() {
        assert!(sanitize_backup_name("").is_err());
        assert_eq!(
            sanitize_backup_name("").unwrap_err(),
            "Backup name cannot be empty"
        );
    }

    #[test]
    fn test_sanitize_backup_name_all_spaces_becomes_underscores() {
        // All chars get replaced by '_', name is non-empty so it passes
        let result = sanitize_backup_name("   ").unwrap();
        assert_eq!(result, "___");
    }

    #[test]
    fn test_backups_dir_joins_path() {
        let base = std::path::Path::new("/tmp/solosoul_test");
        let dir = backups_dir(base);
        assert_eq!(dir, std::path::PathBuf::from("/tmp/solosoul_test/backups"));
    }
    struct Rf012Fixture {
        // Vault 先于临时目录释放，避免 Windows 上数据库句柄阻止清理。
        vault: solosoul_vault::VaultStore,
        directory: tempfile::TempDir,
    }

    impl Rf012Fixture {
        fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            // 仅合成测试账户与固定测试密钥，不访问用户 Vault。
            let config = solosoul_vault::VaultConfig::new(
                "rf012-synthetic-account",
                directory.path().to_path_buf(),
            )
            .with_data_key([0x42; 32]);
            let vault = solosoul_vault::VaultStore::open(config).unwrap();
            Self { vault, directory }
        }

        fn seed_profiles(&self) -> Vec<solosoul_vault::Profile> {
            let entries = [
                (
                    "synthetic-unicode",
                    "合成档案🌟",
                    "多语言内容：你好 / café / 🌍".as_bytes().to_vec(),
                ),
                (
                    "synthetic-binary",
                    "合成二进制",
                    vec![0, 255, 128, 1, 10, 13, 0],
                ),
                ("synthetic-empty", "合成空内容", Vec::new()),
            ];
            entries
                .into_iter()
                .enumerate()
                .map(|(index, (id, name, data))| {
                    let mut profile = solosoul_vault::Profile::new_with_id(id, name, data);
                    profile.created_at = rf012_now() - chrono::Duration::days(index as i64 + 1);
                    profile.version = index as u32 + 3;
                    self.vault.save_profile(&profile).unwrap();
                    // save_profile 会更新 updated_at，断言以真实落库元数据为准。
                    self.vault.load_profile(id).unwrap().unwrap()
                })
                .collect()
        }

        fn corrupt_profile(&self, id: &str) {
            let conn = rusqlite::Connection::open(self.vault.base_path().join("vault.db")).unwrap();
            // 保留 SOLO magic，确保走真实解密失败，而不是旧明文兼容分支。
            assert_eq!(
                conn.execute(
                    "UPDATE profiles SET data = X'534F4C4FDEADBEEF' WHERE id = ?1",
                    rusqlite::params![id],
                )
                .unwrap(),
                1
            );
        }

        fn create(
            &self,
            name: &str,
            profiles: &[solosoul_vault::ProfileSummary],
        ) -> Result<BackupInfo, String> {
            create_profile_backup(
                &self.vault,
                self.directory.path(),
                name,
                profiles,
                rf012_now(),
            )
        }

        fn backup_files(&self) -> std::collections::BTreeMap<String, Vec<u8>> {
            let directory = backups_dir(self.directory.path());
            if !directory.exists() {
                return std::collections::BTreeMap::new();
            }
            fs::read_dir(directory)
                .unwrap()
                .map(|entry| {
                    let entry = entry.unwrap();
                    assert!(
                        entry.file_type().unwrap().is_file(),
                        "备份目录不应残留临时目录"
                    );
                    (
                        entry.file_name().into_string().unwrap(),
                        fs::read(entry.path()).unwrap(),
                    )
                })
                .collect()
        }
    }

    fn rf012_now() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339("2026-09-26T12:34:56Z")
            .unwrap()
            .with_timezone(&chrono::Utc)
    }

    #[test]
    fn rf012_unreadable_last_profile_aborts_without_publishing() {
        let fixture = Rf012Fixture::new();
        fixture.seed_profiles();
        let profiles = fixture.vault.list_profiles().unwrap();
        assert_eq!(profiles.len(), 3);
        let damaged_id = &profiles.last().unwrap().id;
        fixture.corrupt_profile(damaged_id);
        assert!(fixture.vault.load_profile(damaged_id).is_err());
        for summary in &profiles[..profiles.len() - 1] {
            assert!(fixture.vault.load_profile(&summary.id).unwrap().is_some());
        }
        // 元数据枚举仍成功，生产收集路径必须在最后一条读取失败时中止。
        let profiles = fixture.vault.list_profiles().unwrap();
        assert_eq!(profiles.len(), 3);
        let error = fixture
            .create("synthetic-unreadable", &profiles)
            .unwrap_err();
        assert!(error.contains("failed to load profile"), "{error}");
        assert!(error.contains(damaged_id), "{error}");
        assert!(!error.contains("disappeared"), "{error}");
        assert!(fixture.backup_files().is_empty());
        assert!(!backups_dir(fixture.directory.path()).exists());
    }

    #[test]
    fn rf012_missing_enumerated_profile_aborts_without_publishing() {
        let fixture = Rf012Fixture::new();
        fixture.seed_profiles();
        let profiles = fixture.vault.list_profiles().unwrap();
        let missing_id = &profiles.last().unwrap().id;
        fixture.vault.delete_profile(missing_id).unwrap();
        assert!(fixture.vault.load_profile(missing_id).unwrap().is_none());
        assert_eq!(fixture.vault.list_profiles().unwrap().len(), 2);

        let error = fixture.create("synthetic-missing", &profiles).unwrap_err();
        assert!(error.contains("disappeared after enumeration"), "{error}");
        assert!(error.contains(missing_id), "{error}");
        assert!(!error.contains("failed to load"), "{error}");
        assert!(fixture.backup_files().is_empty());
        assert!(!backups_dir(fixture.directory.path()).exists());
    }

    #[test]
    fn rf012_failed_collection_preserves_existing_same_name_backup() {
        for disappears in [false, true] {
            let fixture = Rf012Fixture::new();
            fixture.seed_profiles();
            let profiles = fixture.vault.list_profiles().unwrap();
            let info = fixture.create("synthetic-same-second", &profiles).unwrap();
            fixture.create("synthetic-unrelated", &profiles).unwrap();
            let before = fixture.backup_files();
            assert_eq!(before.len(), 2);
            assert!(before.contains_key(&format!("{}.solosoul_backup", info.id)));

            let last_id = &profiles.last().unwrap().id;
            if disappears {
                fixture.vault.delete_profile(last_id).unwrap();
            } else {
                fixture.corrupt_profile(last_id);
            }
            // 同一生产 worker、名称及秒级时间，失败不能截断已有有效文件。
            assert!(fixture.create("synthetic-same-second", &profiles).is_err());
            assert_eq!(fixture.backup_files(), before);
        }
    }

    #[test]
    fn rf012_complete_manifest_matches_profile_bytes_metadata_and_counts() {
        for empty_vault in [false, true] {
            let fixture = Rf012Fixture::new();
            let expected = if empty_vault {
                Vec::new()
            } else {
                fixture.seed_profiles()
            };
            let profiles = fixture.vault.list_profiles().unwrap();
            let name = "合成 backup/fixture";
            let info = fixture.create(name, &profiles).unwrap();
            assert_eq!(info.id, "合成_backup_fixture_20260926_123456");
            assert_eq!(info.name, name);
            assert_eq!(info.created_at, rf012_now().to_rfc3339());
            assert_eq!(info.object_count, expected.len());

            let files = fixture.backup_files();
            assert_eq!(files.len(), 1);
            let bytes = files.get(&format!("{}.solosoul_backup", info.id)).unwrap();
            assert_eq!(info.size_bytes, bytes.len() as u64);
            let manifest: serde_json::Value = serde_json::from_slice(bytes).unwrap();
            assert_eq!(manifest["version"], "2.0");
            assert_eq!(manifest["created_at"], info.created_at);
            assert_eq!(
                manifest["profile_count"].as_u64().unwrap() as usize,
                info.object_count
            );
            let entries = manifest["profiles"].as_array().unwrap();
            assert_eq!(entries.len(), expected.len());
            let mut seen_ids = std::collections::BTreeSet::new();
            for entry in entries {
                let id = entry["id"].as_str().unwrap();
                assert!(seen_ids.insert(id));
                let profile = expected.iter().find(|profile| profile.id == id).unwrap();
                assert_eq!(entry["name"], profile.name);
                assert_eq!(entry["created_at"], profile.created_at.to_rfc3339());
                assert_eq!(entry["updated_at"], profile.updated_at.to_rfc3339());
                assert_eq!(entry["version"], profile.version);
                assert!(entry.get("data").is_none(), "2.0 清单保留 data_b64 格式");
                let decoded = base64::Engine::decode(
                    &base64::engine::general_purpose::STANDARD,
                    entry["data_b64"].as_str().unwrap(),
                )
                .unwrap();
                assert_eq!(decoded, profile.data);
            }
        }
    }
}

#[tauri::command]
pub async fn backup_create(state: State<'_, AppState>, name: String) -> Result<BackupInfo, String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();
    let profiles = vault.list_profiles()?;
    let info = create_profile_backup(vault, svc.base_path(), &name, &profiles, chrono::Utc::now())?;

    state.auto_sync.trigger_debounce();
    state.device_auto_sync.trigger_data_change();
    Ok(info)
}

/// RF-012：完整读取枚举结果后才写出文件，读取错误或条目消失均中止备份。
fn create_profile_backup(
    vault: &solosoul_vault::VaultStore,
    base_path: &std::path::Path,
    name: &str,
    profiles: &[solosoul_vault::ProfileSummary],
    now: chrono::DateTime<chrono::Utc>,
) -> Result<BackupInfo, String> {
    let timestamp = now.format("%Y%m%d_%H%M%S");
    let safe_name = sanitize_backup_name(name)?;
    let backup_dir = backups_dir(base_path);
    let backup_path = backup_dir.join(format!("{}_{}.solosoul_backup", safe_name, timestamp));

    let mut backup_profiles = Vec::with_capacity(profiles.len());
    for summary in profiles {
        let profile = vault
            .load_profile(&summary.id)
            .map_err(|e| {
                format!(
                    "Backup aborted: failed to load profile '{}': {}",
                    summary.id, e
                )
            })?
            .ok_or_else(|| {
                format!(
                    "Backup aborted: profile '{}' disappeared after enumeration",
                    summary.id
                )
            })?;
        backup_profiles.push(profile);
    }

    let object_count = backup_profiles.len();
    let bytes = encode_profile_backup(&backup_profiles, now, ProfilePayloadEncoding::Base64)
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;
    fs::write(&backup_path, bytes).map_err(|e| e.to_string())?;
    let metadata = fs::metadata(&backup_path).map_err(|e| e.to_string())?;

    Ok(BackupInfo {
        id: format!("{}_{}", safe_name, timestamp),
        name: name.to_string(),
        created_at: now.to_rfc3339(),
        size_bytes: metadata.len(),
        object_count,
    })
}

#[tauri::command]
pub async fn backup_restore(
    state: State<'_, AppState>,
    backup_id: String,
) -> Result<usize, String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();

    let backup_dir = backups_dir(svc.base_path());
    let mut found_path: Option<PathBuf> = None;

    if let Ok(dir) = fs::read_dir(&backup_dir) {
        for entry in dir {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                // R009: use exact match instead of prefix matching to avoid deleting/restoring
                // the wrong backup when IDs share a prefix.
                if stem == backup_id.as_str() {
                    found_path = Some(path);
                    break;
                }
            }
        }
    }

    let backup_path = found_path.ok_or_else(|| format!("Backup '{}' not found", backup_id))?;
    let content = fs::read(&backup_path).map_err(|e| e.to_string())?;
    let restored = restore_profile_backup(vault, &content, chrono::Utc::now())?;
    state.auto_sync.trigger_debounce();
    state.device_auto_sync.trigger_data_change();

    Ok(restored)
}

/// RF-013：共享解码完成后才按原顺序保存，格式错误不会造成前缀覆盖。
/// 每条保存仍沿用 VaultStore 的既有事务；不宣称数据库故障时整批原子恢复。
fn restore_profile_backup(
    vault: &solosoul_vault::VaultStore,
    content: &[u8],
    now: chrono::DateTime<chrono::Utc>,
) -> Result<usize, String> {
    let decoded = decode_profile_backup(content, now).map_err(|error| match error {
        ProfileBackupError::InvalidBase64 { index, reason } => {
            format!(
                "Base64 decode profile data: entry {}: {}",
                index + 1,
                reason
            )
        }
        other => other.to_string(),
    })?;
    let restored = decoded.profiles.len();
    for profile in decoded.profiles {
        vault.save_profile(&profile)?;
    }
    Ok(restored)
}

#[cfg(test)]
#[path = "backup/rf013_tests.rs"]
mod rf013_tests;

#[tauri::command]
pub async fn backup_delete(state: State<'_, AppState>, backup_id: String) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let backup_dir = backups_dir(svc.base_path());

    if let Ok(dir) = fs::read_dir(&backup_dir) {
        for entry in dir {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                // R009: exact match only.
                if stem == backup_id.as_str() {
                    fs::remove_file(&path).map_err(|e| e.to_string())?;
                    return Ok(());
                }
            }
        }
    }
    Err(format!("Backup '{}' not found", backup_id))
}
