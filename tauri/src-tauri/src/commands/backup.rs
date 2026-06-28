//! Backup commands — create, list, restore, delete vault backups

use serde::{Deserialize, Serialize};
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
}

#[tauri::command]
pub async fn backup_create(state: State<'_, AppState>, name: String) -> Result<BackupInfo, String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();

    let backup_dir = backups_dir(svc.base_path());
    fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let safe_name = sanitize_backup_name(&name)?;
    let backup_path = backup_dir.join(format!("{}_{}.solosoul_backup", safe_name, timestamp));

    // Collect all profiles
    let profiles = vault.list_profiles().map_err(|e| e.to_string())?;
    let object_count = profiles.len();

    #[derive(Serialize)]
    struct BackupManifest {
        version: String,
        created_at: String,
        profile_count: usize,
        profiles: Vec<ProfileBackupEntry>,
    }
    #[derive(Serialize)]
    struct ProfileBackupEntry {
        id: String,
        name: String,
        /// Base64-encoded profile data (避免 JSON 序列化为大型数字数组)
        data_b64: String,
        created_at: String,
        updated_at: String,
        version: u32,
    }

    let mut backup_profiles = Vec::new();
    for p in &profiles {
        if let Ok(Some(profile)) = vault.load_profile(&p.id) {
            backup_profiles.push(ProfileBackupEntry {
                id: profile.id,
                name: profile.name,
                data_b64: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &profile.data),
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

    let json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    fs::write(&backup_path, json).map_err(|e| e.to_string())?;

    let metadata = fs::metadata(&backup_path).map_err(|e| e.to_string())?;

    Ok(BackupInfo {
        id: format!("{}_{}", safe_name, timestamp),
        name,
        created_at: chrono::Utc::now().to_rfc3339(),
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
    let content = fs::read_to_string(&backup_path).map_err(|e| e.to_string())?;

    #[derive(Deserialize)]
    #[allow(dead_code)]
    struct RestoreManifest {
        version: String,
        created_at: String,
        profile_count: usize,
        profiles: Vec<RestoreProfileEntry>,
    }
    #[derive(Deserialize)]
    struct RestoreProfileEntry {
        id: String,
        name: String,
        #[serde(default)]
        data_b64: String,
        #[serde(default)]
        data: Vec<u8>,
        created_at: String,
        updated_at: String,
        version: u32,
    }

    let manifest: RestoreManifest = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    let mut restored = 0usize;

    for entry in &manifest.profiles {
        // 兼容新旧两种格式：优先 data_b64，回退旧版 data (Vec<u8>)
        let data = if !entry.data_b64.is_empty() {
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &entry.data_b64)
                .map_err(|e| format!("Base64 decode profile data: {}", e))?
        } else {
            entry.data.clone()
        };
        let profile = solosoul_vault::Profile {
            id: entry.id.clone(),
            name: entry.name.clone(),
            data,
            created_at: chrono::DateTime::parse_from_rfc3339(&entry.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: chrono::DateTime::parse_from_rfc3339(&entry.updated_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            version: entry.version,
        };
        vault.save_profile(&profile).map_err(|e| e.to_string())?;
        restored += 1;
    }

    Ok(restored)
}

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
