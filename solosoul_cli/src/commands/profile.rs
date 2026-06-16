//! Profile 管理命令：/profile、/profile rename、/profile set。

use color_eyre::Result;
use serde_json::{Map, Value};
use solosoul_core::Profile;

use crate::app::{App, AppPhase};

fn map_err(e: String) -> color_eyre::Report {
    color_eyre::eyre::eyre!(e)
}

/// 命令入口。
pub fn handle(app: &mut App, args: &[&str]) -> Result<()> {
    let sub = args.get(1).copied().unwrap_or("");
    match sub {
        "" => show_profile(app),
        "rename" => rename_profile(app, args.get(2).copied()),
        "set" => set_profile_value(
            app,
            args.get(2).copied(),
            args.get(3..).map(|slice| slice.join(" ")),
        ),
        _ => {
            app.error_message = Some(
                "用法: /profile | /profile rename <名称> | /profile set <路径> <值>".to_string(),
            );
            Ok(())
        }
    }
}

/// 确保 Vault 已解锁，返回当前账户 ID 与 VaultStore 引用。
fn require_unlocked(app: &mut App) -> Result<(String, std::sync::Arc<solosoul_core::VaultStore>)> {
    if !app.vault_service.is_unlocked() {
        app.error_message = Some("请先使用 /unlock 登录".to_string());
        return Err(color_eyre::eyre::eyre!("Vault is locked"));
    }
    let account_id = app
        .vault_service
        .get_current_account()
        .ok_or_else(|| color_eyre::eyre::eyre!("No current account"))?;
    let vault = app
        .vault_service
        .get_vault_store()
        .ok_or_else(|| color_eyre::eyre::eyre!("Vault 未打开"))?;
    Ok((account_id, vault))
}

/// 加载当前账户的 Profile，不存在则创建空 Profile。
fn load_or_create_profile(app: &mut App) -> Result<Profile> {
    let (account_id, vault) = require_unlocked(app)?;
    match vault.load_profile(&account_id).map_err(map_err)? {
        Some(p) => Ok(p),
        None => {
            let p = Profile::new_with_id(&account_id, &account_id, Vec::new());
            vault.save_profile(&p).map_err(map_err)?;
            Ok(p)
        }
    }
}

/// 解析 Profile 数据为 JSON，空数据则返回空对象。
fn profile_data_value(profile: &Profile) -> Result<Value> {
    if profile.data.is_empty() {
        Ok(Value::Object(Map::new()))
    } else {
        serde_json::from_slice(&profile.data)
            .map_err(|e| color_eyre::eyre::eyre!("解析 profile 数据失败: {}", e))
    }
}

/// 保存 Profile 数据。
fn save_profile_data(app: &mut App, profile: &mut Profile, data: &Value) -> Result<()> {
    let (_, vault) = require_unlocked(app)?;
    profile.data = serde_json::to_vec(data).map_err(|e| {
        app.error_message = Some(format!("序列化 profile 数据失败: {}", e));
        color_eyre::eyre::eyre!(e)
    })?;
    profile.updated_at = chrono::Utc::now();
    profile.version += 1;
    vault.save_profile(profile).map_err(map_err)?;
    Ok(())
}

/// 执行 `/profile`：打开 Profile 展示屏幕。
fn show_profile(app: &mut App) -> Result<()> {
    let profile = load_or_create_profile(app)?;
    let data = profile_data_value(&profile)?;
    app.previous_phase = Some(app.phase.clone());
    app.phase = AppPhase::Profile {
        profile,
        data,
        selected: 0,
    };
    Ok(())
}

/// 执行 `/profile rename <名称>`：重命名 Profile。
fn rename_profile(app: &mut App, name: Option<&str>) -> Result<()> {
    let name = match name {
        Some(n) if !n.is_empty() => n.to_string(),
        _ => {
            app.error_message = Some("用法: /profile rename <名称>".to_string());
            return Ok(());
        }
    };

    let (account_id, vault) = require_unlocked(app)?;
    let mut profile = match vault.load_profile(&account_id).map_err(map_err)? {
        Some(p) => p,
        None => Profile::new_with_id(&account_id, &name, Vec::new()),
    };
    profile.name = name.clone();
    profile.updated_at = chrono::Utc::now();
    profile.version += 1;
    vault.save_profile(&profile).map_err(map_err)?;

    let data = profile_data_value(&profile)?;
    app.previous_phase = Some(app.phase.clone());
    app.phase = AppPhase::Profile {
        profile,
        data,
        selected: 0,
    };
    app.error_message = Some(format!("Profile 名称已更新为: {}", name));
    Ok(())
}

/// 尝试将字符串解析为 JSON，失败则回退为字符串。
fn parse_value(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_string()))
}

/// 执行 `/profile set <路径> <值>`：使用点号路径设置 Profile 数据。
/// 示例: `/profile set identity.fullName 张三`
fn set_profile_value(app: &mut App, path: Option<&str>, value: Option<String>) -> Result<()> {
    let path = match path {
        Some(p) if !p.is_empty() => p,
        _ => {
            app.error_message = Some("用法: /profile set <路径> <值>".to_string());
            return Ok(());
        }
    };
    let value = match value {
        Some(v) => parse_value(&v),
        None => {
            app.error_message = Some("用法: /profile set <路径> <值>".to_string());
            return Ok(());
        }
    };

    let mut profile = load_or_create_profile(app)?;
    let mut data = profile_data_value(&profile)?;

    if let Err(e) = set_value_at_path(&mut data, path, value) {
        app.error_message = Some(format!("设置失败: {}", e));
        return Ok(());
    }

    save_profile_data(app, &mut profile, &data)?;
    app.error_message = Some(format!("已更新: {}", path));

    // 刷新展示屏幕
    let data = profile_data_value(&profile)?;
    app.previous_phase = Some(app.phase.clone());
    app.phase = AppPhase::Profile {
        profile,
        data,
        selected: 0,
    };
    Ok(())
}

/// 在 JSON 对象的点号路径上设置值，仅支持对象层级。
fn set_value_at_path(data: &mut Value, path: &str, value: Value) -> Result<(), String> {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return Err("路径为空".to_string());
    }

    let obj = data.as_object_mut().ok_or("Profile 数据不是对象")?;
    let (last, parents) = parts.split_last().unwrap();

    let mut current = obj;
    for part in parents {
        current = current
            .entry(*part)
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .ok_or_else(|| format!("路径 {} 不是对象", part))?;
    }
    current.insert(last.to_string(), value);
    Ok(())
}

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
        std::env::set_var("SOLOSOUL_DATA_DIR", dir.path());
        let vault = VaultService::new();
        let account = vault.create_account("Test", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap().to_string();
        let app = App::new(Arc::new(vault)).unwrap();
        app.vault_service
            .unlock(&account_id, "password123")
            .unwrap();
        (app, account_id, dir)
    }

    #[test]
    fn test_show_profile_creates_empty() {
        let (mut app, _account_id, _dir) = unlocked_app();
        handle(&mut app, &["/profile"]).unwrap();
        assert!(matches!(app.phase, AppPhase::Profile { .. }));
    }

    #[test]
    fn test_rename_profile() {
        let (mut app, _account_id, _dir) = unlocked_app();
        handle(&mut app, &["/profile", "rename", "My Profile"]).unwrap();
        assert!(app
            .error_message
            .as_deref()
            .unwrap_or("")
            .contains("My Profile"));
        if let AppPhase::Profile { profile, .. } = &app.phase {
            assert_eq!(profile.name, "My Profile");
        } else {
            panic!("expected Profile phase");
        }
    }

    #[test]
    fn test_set_profile_value() {
        let (mut app, account_id, _dir) = unlocked_app();
        handle(&mut app, &["/profile", "set", "identity.fullName", "张三"]).unwrap();
        assert!(app
            .error_message
            .as_deref()
            .unwrap_or("")
            .contains("identity.fullName"));

        let vault = app.vault_service.get_vault_store().unwrap();
        let profile = vault.load_profile(&account_id).unwrap().unwrap();
        let data: Value = serde_json::from_slice(&profile.data).unwrap();
        assert_eq!(data["identity"]["fullName"], "张三");
    }

    #[test]
    fn test_set_profile_value_nested_object() {
        let (mut app, account_id, _dir) = unlocked_app();
        handle(
            &mut app,
            &["/profile", "set", "preferences.mealPreference", "素食"],
        )
        .unwrap();

        let vault = app.vault_service.get_vault_store().unwrap();
        let profile = vault.load_profile(&account_id).unwrap().unwrap();
        let data: Value = serde_json::from_slice(&profile.data).unwrap();
        assert_eq!(data["preferences"]["mealPreference"], "素食");
    }

    #[test]
    fn test_unknown_subcommand() {
        let (mut app, _account_id, _dir) = unlocked_app();
        handle(&mut app, &["/profile", "foo"]).unwrap();
        assert!(app.error_message.as_deref().unwrap_or("").contains("用法"));
    }
}
