//! 命令路由与执行器。

use crate::app::App;

// ---- 共享帮助函数 ----

/// 确保 Vault 已解锁，返回当前账户 ID。
pub fn require_unlocked(app: &mut App) -> color_eyre::Result<String> {
    if !app.vault_service.is_unlocked() {
        app.error_message = Some("请先使用 /unlock 登录".to_string());
        return Err(color_eyre::eyre::eyre!("Vault is locked"));
    }
    app.vault_service
        .get_current_account()
        .ok_or_else(|| color_eyre::eyre::eyre!("No current account"))
}

/// 确保 Vault 已解锁，返回 (账户 ID, VaultStore)。
pub fn require_unlocked_with_vault(app: &mut App) -> color_eyre::Result<(String, std::sync::Arc<solosoul_core::VaultStore>)> {
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
pub fn update_profile_preference(app: &mut App, key: &str, value: serde_json::Value) -> color_eyre::Result<()> {
    use serde_json::{Map, Value};
    let account_id = require_unlocked(app)?;

    let vault = app
        .vault_service
        .get_vault_store()
        .ok_or_else(|| color_eyre::eyre::eyre!("Vault 未打开"))?;

    let mut profile = match vault.load_profile(&account_id).map_err(|e| color_eyre::eyre::eyre!(e))? {
        Some(p) => p,
        None => solosoul_core::Profile::new_with_id(&account_id, &account_id, Vec::new()),
    };

    let mut data: Value = if profile.data.is_empty() {
        Value::Object(Map::new())
    } else {
        serde_json::from_slice(&profile.data)
            .map_err(|e| color_eyre::eyre::eyre!("解析 profile 数据失败: {}", e))?
    };

    if let Some(obj) = data.as_object_mut() {
        let prefs = obj
            .entry("preferences")
            .or_insert_with(|| Value::Object(Map::new()));
        if let Some(p) = prefs.as_object_mut() {
            p.insert(key.to_string(), value);
        }
    }

    profile.data = serde_json::to_vec(&data).map_err(|e| {
        app.error_message = Some(format!("序列化 profile 数据失败: {}", e));
        color_eyre::eyre::eyre!(e)
    })?;
    profile.updated_at = chrono::Utc::now();
    profile.version += 1;

    vault.save_profile(&profile).map_err(|e| color_eyre::eyre::eyre!(e))?;
    Ok(())
}

// ---- CLI 命令错误类型 ----

/// CLI 命令错误类型，简化为 String。
pub type CliError = String;

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
