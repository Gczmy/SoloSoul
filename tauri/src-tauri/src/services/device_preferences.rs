//! 旧版全局同步开关迁入首个解锁账户，之后只读写账户加密 Profile。

use solosoul_core::VaultService;
use solosoul_vault::{storage::DeviceSyncPreferences, VaultStore};

pub fn load_sync_preferences(
    app: &tauri::AppHandle,
    svc: &VaultService,
) -> Result<DeviceSyncPreferences, String> {
    let vault = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let _guard = crate::commands::settings::UI_PREFS_LOCK
        .lock()
        .map_err(|_| "UI preferences lock poisoned")?;
    let path = crate::commands::settings::resolve_ui_prefs_path(app, svc)?;
    let prefs = migrate_sync_preferences(&vault, &path)?;
    svc.set_ui_prefs_sync_enabled(prefs.ui_prefs_sync_enabled);
    Ok(prefs)
}

fn migrate_sync_preferences(
    vault: &VaultStore,
    path: &std::path::Path,
) -> Result<DeviceSyncPreferences, String> {
    let current = vault.device_sync_preferences()?;
    let mut legacy: serde_json::Value = match std::fs::read(path) {
        Ok(bytes) => {
            serde_json::from_slice(&bytes).map_err(|e| format!("Read UI preferences: {e}"))?
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => serde_json::json!({}),
        Err(e) => return Err(format!("Read UI preferences: {e}")),
    };
    let prefs = match current {
        Some(prefs) => prefs,
        None => vault.update_device_sync_preferences(|prefs| {
            if let Some(value) = legacy.get("auto_sync_enabled").and_then(|v| v.as_bool()) {
                prefs.auto_sync_enabled = value;
            }
            if let Some(value) = legacy
                .get("ui_prefs_sync_enabled")
                .and_then(|v| v.as_bool())
            {
                prefs.ui_prefs_sync_enabled = value;
            }
        })?,
    };
    // 先确保持久化成功，再清理明文键；失败可重试，不覆盖已存在的账户选择。
    if let Some(obj) = legacy.as_object_mut() {
        let old_auto = obj.remove("auto_sync_enabled");
        let old_ui = obj.remove("ui_prefs_sync_enabled");
        if old_auto.is_some() || old_ui.is_some() {
            let temp_path = path.with_extension("sync-migration.tmp");
            std::fs::write(
                &temp_path,
                serde_json::to_vec(&legacy).map_err(|e| e.to_string())?,
            )
            .map_err(|e| format!("Migrate UI preferences: {e}"))?;
            std::fs::rename(&temp_path, path)
                .map_err(|e| format!("Migrate UI preferences: {e}"))?;
        }
    }
    Ok(prefs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solosoul_vault::VaultConfig;

    #[test]
    fn migrates_legacy_once_without_overwriting_account_or_ui_preferences() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ui_preferences.json");
        std::fs::create_dir_all(dir.path().join("first")).unwrap();
        std::fs::create_dir_all(dir.path().join("second")).unwrap();
        std::fs::write(
            &path,
            r#"{"theme":"dark","auto_sync_enabled":true,"ui_prefs_sync_enabled":false}"#,
        )
        .unwrap();
        let vault = VaultStore::open(
            VaultConfig::new("first", dir.path().join("first")).with_data_key([3; 32]),
        )
        .unwrap();
        let prefs = migrate_sync_preferences(&vault, &path).unwrap();
        assert!(prefs.auto_sync_enabled);
        assert!(!prefs.ui_prefs_sync_enabled);
        let ui: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(ui, serde_json::json!({"theme":"dark"}));
        vault
            .update_device_sync_preferences(|p| p.auto_sync_enabled = false)
            .unwrap();
        // 模拟中断后留下旧文件，不可覆盖用户已保存的账户选择。
        std::fs::write(&path, r#"{"theme":"dark","auto_sync_enabled":true}"#).unwrap();
        assert!(
            !migrate_sync_preferences(&vault, &path)
                .unwrap()
                .auto_sync_enabled
        );
        let other = VaultStore::open(
            VaultConfig::new("second", dir.path().join("second")).with_data_key([4; 32]),
        )
        .unwrap();
        assert_eq!(
            migrate_sync_preferences(&other, &path).unwrap(),
            DeviceSyncPreferences::default()
        );
    }
}
