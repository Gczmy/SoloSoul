//! 设置与诊断命令：/language、/theme、/setting、/debug_log。

use std::path::Path;

use color_eyre::Result;
use serde_json::{Map, Value};

use crate::app::App;

fn map_err(e: String) -> color_eyre::Report {
    color_eyre::eyre::eyre!(e)
}

/// 命令入口。
pub fn handle(app: &mut App, args: &[&str]) -> Result<()> {
    let cmd = args.first().copied().unwrap_or("");
    match cmd {
        "/language" => language(app, args.get(1).copied()),
        "/theme" => theme(app, args.get(1).copied()),
        "/setting" => setting(app, args.get(1).copied(), args.get(2).copied()),
        "/debug_log" => debug_log(app, args.get(1).copied()),
        _ => {
            app.error_message = Some(format!("未知设置命令: {}", cmd));
            Ok(())
        }
    }
}

/// UI 偏好设置文件路径。
fn ui_prefs_path(app: &App) -> std::path::PathBuf {
    app.vault_service.base_path().join("ui_preferences.json")
}

/// 加载 UI 偏好，缺失字段使用默认值填充。
fn load_ui_prefs(app: &App) -> Map<String, Value> {
    let path = ui_prefs_path(app);
    let mut map = if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|c| serde_json::from_str::<Value>(&c).ok())
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default()
    } else {
        Map::new()
    };

    map.entry("theme")
        .or_insert_with(|| Value::String("system".to_string()));
    map.entry("accentColor")
        .or_insert_with(|| Value::String("ocean".to_string()));
    map.entry("language")
        .or_insert_with(|| Value::String(String::new()));
    map.entry("hasSeenOnboarding")
        .or_insert_with(|| Value::Bool(false));

    map
}

/// 保存 UI 偏好到 `{base}/ui_preferences.json`。
fn save_ui_prefs(app: &mut App, prefs: &Map<String, Value>) -> Result<()> {
    let path = ui_prefs_path(app);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            app.error_message = Some(format!("创建偏好目录失败: {}", e));
            color_eyre::eyre::eyre!(e)
        })?;
    }
    let json = serde_json::to_string(&Value::Object(prefs.clone())).map_err(|e| {
        app.error_message = Some(format!("序列化 UI 偏好失败: {}", e));
        color_eyre::eyre::eyre!(e)
    })?;
    std::fs::write(&path, json).map_err(|e| {
        app.error_message = Some(format!("写入 UI 偏好失败: {}", e));
        color_eyre::eyre::eyre!(e)
    })
}

/// 执行 `/language [lang]`：获取或设置界面语言。
fn language(app: &mut App, lang: Option<&str>) -> Result<()> {
    let mut prefs = load_ui_prefs(app);
    match lang {
        Some(lang) => {
            prefs.insert("language".to_string(), Value::String(lang.to_string()));
            save_ui_prefs(app, &prefs)?;
            app.error_message = Some(format!("语言已设置为: {}", lang));
        }
        None => {
            let current = prefs["language"].as_str().unwrap_or("");
            app.error_message = Some(format!("当前语言: {}", current));
        }
    }
    Ok(())
}

/// 执行 `/theme [theme]`：获取或设置界面主题。
fn theme(app: &mut App, theme: Option<&str>) -> Result<()> {
    let mut prefs = load_ui_prefs(app);
    match theme {
        Some(theme) => {
            prefs.insert("theme".to_string(), Value::String(theme.to_string()));
            save_ui_prefs(app, &prefs)?;
            app.error_message = Some(format!("主题已设置为: {}", theme));
        }
        None => {
            let current = prefs["theme"].as_str().unwrap_or("system");
            app.error_message = Some(format!("当前主题: {}", current));
        }
    }
    Ok(())
}

/// 确保 Vault 已解锁，返回当前账户 ID。
fn require_unlocked(app: &mut App) -> Result<String> {
    if !app.vault_service.is_unlocked() {
        app.error_message = Some("请先使用 /unlock 登录".to_string());
        return Err(color_eyre::eyre::eyre!("Vault is locked"));
    }
    app.vault_service
        .get_current_account()
        .ok_or_else(|| color_eyre::eyre::eyre!("No current account"))
}

/// 尝试将字符串解析为 JSON，失败则回退为字符串。
fn parse_value(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_string()))
}

/// 更新当前账户加密偏好中的单个键值。
fn update_profile_preference(app: &mut App, key: &str, value: Value) -> Result<()> {
    let account_id = require_unlocked(app)?;

    let vault = app
        .vault_service
        .get_vault_store()
        .ok_or_else(|| color_eyre::eyre::eyre!("Vault 未打开"))?;

    let mut profile = match vault.load_profile(&account_id).map_err(map_err)? {
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

    vault.save_profile(&profile).map_err(map_err)?;
    Ok(())
}

/// 执行 `/setting <key> <value>`：更新加密用户偏好。
fn setting(app: &mut App, key: Option<&str>, value: Option<&str>) -> Result<()> {
    let key = match key {
        Some(k) => k,
        None => {
            app.error_message = Some("用法: /setting <key> <value>".to_string());
            return Ok(());
        }
    };
    let value = match value {
        Some(v) => parse_value(v),
        None => {
            app.error_message = Some("用法: /setting <key> <value>".to_string());
            return Ok(());
        }
    };

    update_profile_preference(app, key, value)?;
    app.error_message = Some(format!("偏好已更新: {}", key));
    Ok(())
}

/// 执行 `/debug_log [file_name]`：导出诊断包。
fn debug_log(app: &mut App, file_name: Option<&str>) -> Result<()> {
    let account_id = require_unlocked(app)?;

    let vault = app
        .vault_service
        .get_vault_store()
        .ok_or_else(|| color_eyre::eyre::eyre!("Vault 未打开"))?;

    let entries = vault.list_audit_log(10000).map_err(map_err)?;

    // 脱敏系统信息：不包含密码、会话密钥等敏感字段。
    let system_info = serde_json::json!({
        "appName": "SoloSoul CLI",
        "version": env!("CARGO_PKG_VERSION"),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "dataDir": app.vault_service.base_path().display().to_string(),
        "lockAcquired": app.process_lock.is_some(),
        "accountId": account_id,
    });

    let bundle = serde_json::json!({
        "exportedAt": chrono::Utc::now().to_rfc3339(),
        "systemInfo": system_info,
        "auditLog": entries,
    });

    let json = serde_json::to_string_pretty(&bundle).map_err(|e| {
        app.error_message = Some(format!("序列化诊断包失败: {}", e));
        color_eyre::eyre::eyre!(e)
    })?;

    let logs_dir = app.vault_service.base_path().join("logs");
    std::fs::create_dir_all(&logs_dir).map_err(|e| {
        app.error_message = Some(format!("创建日志目录失败: {}", e));
        color_eyre::eyre::eyre!(e)
    })?;

    let file_name = file_name.unwrap_or("debug_log.json");
    let file_name = Path::new(file_name)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "debug_log.json".to_string());
    let path = logs_dir.join(&file_name);

    std::fs::write(&path, &json).map_err(|e| {
        app.error_message = Some(format!("写入诊断包失败: {}", e));
        color_eyre::eyre::eyre!(e)
    })?;

    app.error_message = Some(format!("诊断包已导出至: {}", path.display()));
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
        (app, account_id, dir)
    }

    #[test]
    fn test_language_get_and_set() {
        let (mut app, _id, _dir) = unlocked_app();

        // 获取默认值
        handle(&mut app, &["/language"]).unwrap();
        assert!(app
            .error_message
            .as_deref()
            .unwrap_or("")
            .contains("当前语言"));

        // 设置
        handle(&mut app, &["/language", "zh-CN"]).unwrap();
        let path = app.vault_service.base_path().join("ui_preferences.json");
        let content = std::fs::read_to_string(&path).unwrap();
        let prefs: Value = serde_json::from_str(&content).unwrap();
        assert_eq!(prefs["language"], "zh-CN");
    }

    #[test]
    fn test_theme_get_and_set() {
        let (mut app, _id, _dir) = unlocked_app();

        handle(&mut app, &["/theme"]).unwrap();
        assert!(app
            .error_message
            .as_deref()
            .unwrap_or("")
            .contains("当前主题"));

        handle(&mut app, &["/theme", "dark"]).unwrap();
        let path = app.vault_service.base_path().join("ui_preferences.json");
        let content = std::fs::read_to_string(&path).unwrap();
        let prefs: Value = serde_json::from_str(&content).unwrap();
        assert_eq!(prefs["theme"], "dark");
    }

    #[test]
    fn test_setting_updates_encrypted_preference() {
        let (mut app, account_id, _dir) = unlocked_app();
        handle(&mut app, &["/setting", "notifications", "true"]).unwrap();

        let vault = app.vault_service.get_vault_store().unwrap();
        let profile = vault.load_profile(&account_id).unwrap().unwrap();
        let data: Value = serde_json::from_slice(&profile.data).unwrap();
        assert_eq!(data["preferences"]["notifications"], true);
    }

    #[test]
    fn test_debug_log_creates_file() {
        let (mut app, _id, _dir) = unlocked_app();
        handle(&mut app, &["/debug_log", "test_debug.json"]).unwrap();

        let path = app
            .vault_service
            .base_path()
            .join("logs")
            .join("test_debug.json");
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        let parsed: Value = serde_json::from_str(&content).unwrap();
        assert!(parsed["systemInfo"].is_object());
        assert!(parsed["auditLog"].is_array());
        assert!(!content.to_lowercase().contains("password"));
        assert!(!content.to_lowercase().contains("sessionkey"));
    }
}
