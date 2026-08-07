//! 命令路由与执行器。

use crate::app::App;

// ---- 共享帮助函数 ----

use crate::t;

/// 确保 Vault 已解锁，返回当前账户 ID。
pub fn require_unlocked(app: &mut App) -> color_eyre::Result<String> {
    if !app.vault_service.is_unlocked() {
        app.error_message = Some(t!(app.i18n, "cmd-need-unlock"));
        return Err(color_eyre::eyre::eyre!("Vault is locked"));
    }
    app.vault_service
        .get_current_account()
        .ok_or_else(|| color_eyre::eyre::eyre!("No current account"))
}

/// 确保 Vault 已解锁，返回 (账户 ID, VaultStore)。
pub fn require_unlocked_with_vault(
    app: &mut App,
) -> color_eyre::Result<(String, std::sync::Arc<solosoul_core::VaultStore>)> {
    let account_id = require_unlocked(app)?;
    let vault = app
        .vault_service
        .get_vault_store()
        .ok_or_else(|| color_eyre::eyre::eyre!("Vault 未打开"))?;
    Ok((account_id, vault))
}

/// 尝试将字符串解析为 JSON，失败则回退为字符串。
pub fn parse_value(raw: &str) -> serde_json::Value {
    serde_json::from_str(raw).unwrap_or_else(|_| serde_json::Value::String(raw.to_string()))
}

/// 更新当前账户加密偏好中的单个键值。
pub fn update_profile_preference(
    app: &mut App,
    key: &str,
    value: serde_json::Value,
) -> color_eyre::Result<()> {
    use serde_json::{Map, Value};
    let (account_id, vault) = require_unlocked_with_vault(app)?;

    let mut profile = match vault
        .load_profile(&account_id)
        .map_err(|e| color_eyre::eyre::eyre!(e))?
    {
        Some(p) => p,
        None => solosoul_core::Profile::new_with_id(&account_id, &account_id, Vec::new()),
    };

    let mut data: Value = if profile.data.is_empty() {
        Value::Object(Map::new())
    } else {
        serde_json::from_slice(&profile.data)
            .map_err(|e| color_eyre::eyre::eyre!("解析 profile 数据失败: {}", e))?
    };

    // P032：根非对象时不再静默跳过——偏好写入丢失且调用方误报成功，改为显式报错
    let obj = match data.as_object_mut() {
        Some(o) => o,
        None => {
            return Err(color_eyre::eyre::eyre!(
                "profile 数据根不是对象，无法写入偏好"
            ));
        }
    };
    let prefs = obj
        .entry("preferences")
        .or_insert_with(|| Value::Object(Map::new()));
    if let Some(p) = prefs.as_object_mut() {
        p.insert(key.to_string(), value);
    }

    profile.data = serde_json::to_vec(&data).map_err(|e| {
        app.error_message = Some(t!(
            app.i18n,
            "cmd-profile-serialize-failed",
            err = e.to_string()
        ));
        color_eyre::eyre::eyre!(e)
    })?;
    profile.updated_at = chrono::Utc::now();
    profile.version += 1;

    vault
        .save_profile(&profile)
        .map_err(|e| color_eyre::eyre::eyre!(e))?;
    Ok(())
}

pub mod attachment;
pub mod auth;
pub mod backup;
pub mod core;
pub mod doctor;
pub mod embed_model;
pub mod export_import;
pub mod history;
pub mod llm;
pub mod log;
pub mod ocr;
pub mod plugin;
pub mod profile;
pub mod search;
pub mod security;
pub mod settings;
pub mod sync;
pub mod system;
pub mod template;
pub mod vault_read;
pub mod vault_write;

#[cfg(test)]
mod tests {
    use super::*;
    use solosoul_core::VaultService;
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

    /// P032：profile 数据根非对象时应报错，不得静默跳过偏好写入。
    #[test]
    fn test_update_profile_preference_rejects_non_object_root() {
        let (mut app, account_id, _dir) = unlocked_app();
        let vault = app.vault_service.get_vault_store().unwrap();

        // 先走一次成功路径确保 profile 存在，再将 data 写成非对象 JSON（数组）
        update_profile_preference(&mut app, "seed", serde_json::Value::Bool(true)).unwrap();
        let mut profile = vault.load_profile(&account_id).unwrap().unwrap();
        profile.data = b"[1,2,3]".to_vec();
        vault.save_profile(&profile).unwrap();

        let err =
            update_profile_preference(&mut app, "theme", serde_json::Value::String("dark".into()))
                .unwrap_err();
        assert!(
            err.to_string().contains("根不是对象"),
            "应报告根非对象: {}",
            err
        );

        // 数据保持原样（未写入、未覆盖）
        let p2 = vault.load_profile(&account_id).unwrap().unwrap();
        assert_eq!(p2.data, b"[1,2,3]");
    }

    /// P032 正向：正常对象根写入偏好成功。
    #[test]
    fn test_update_profile_preference_success() {
        let (mut app, account_id, _dir) = unlocked_app();
        let vault = app.vault_service.get_vault_store().unwrap();

        update_profile_preference(&mut app, "theme", serde_json::Value::String("dark".into()))
            .unwrap();

        let profile = vault.load_profile(&account_id).unwrap().unwrap();
        let data: serde_json::Value = serde_json::from_slice(&profile.data).unwrap();
        assert_eq!(data["preferences"]["theme"], "dark");
    }
}
