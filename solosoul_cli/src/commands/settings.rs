//! 设置与诊断命令：/language、/theme、/setting、/debug_log。
//!
//! Phase 方案 C：本模块除了 4 个原始命令（保持脚本兼容），还导出
//! `open_menu` / `open_language_select` / `open_theme_select` /
//! `apply_language` / `apply_theme` / `start_settings_preference_edit` /
//! `trigger_debug_log_export` / `dispatch_item`，驱动 TUI 设置菜单 phase。

use std::path::Path;
use std::time::Instant;

use color_eyre::Result;
use serde_json::{Map, Value};

use crate::app::{App, AppPhase};
use crate::commands::require_unlocked_with_vault;
use crate::t;
use crate::widgets::prompt::{PromptResult, PromptSpec};

/// 命令入口。
pub fn handle(app: &mut App, args: &[&str]) -> Result<()> {
    let cmd = args.first().copied().unwrap_or("");
    match cmd {
        "/language" => language(app, args.get(1).copied()),
        "/theme" => theme(app, args.get(1).copied()),
        "/setting" => setting(app, args.get(1).copied(), args.get(2).copied()),
        "/debug_log" => debug_log(app, args.get(1).copied()),
        _ => {
            app.error_message = Some(t!(app.i18n, "cmd-unknown-subcommand", cmd = cmd));
            Ok(())
        }
    }
}

// === 通用 helper ===

/// 当前生效语言（从 ui_preferences.json 读取，缺失返回空串）。
pub fn current_language(app: &App) -> String {
    load_ui_prefs(app)
        .get("language")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// 当前生效主题（从 ui_preferences.json 读取）。
pub fn current_theme(app: &App) -> String {
    load_ui_prefs(app)
        .get("theme")
        .and_then(|v| v.as_str())
        .unwrap_or("system")
        .to_string()
}

// === Phase 方案 C：设置菜单入口 ===

/// 打开设置菜单顶层（无论 Home 点击还是键入无参 `/setting` 都会调用）。
///
/// 单槽 `previous_phase`：把当前 phase 压栈，新 phase 为 SettingsMenu。
pub fn open_menu(app: &mut App) {
    let prefs = load_ui_prefs(app);
    let language = prefs
        .get("language")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let theme = prefs
        .get("theme")
        .and_then(|v| v.as_str())
        .unwrap_or("system")
        .to_string();
    app.previous_phase = Some(app.phase.clone());
    app.phase = AppPhase::SettingsMenu {
        selected: 0,
        current_language: language,
        current_theme: theme,
    };
}

/// 通过菜单项 index 进入对应子 phase（与 mouse::execute_click 共享）。
///
/// `idx`：0=语言 / 1=主题 / 2=自定义键值 / 3=导出调试包。
pub fn dispatch_item(app: &mut App, idx: usize) {
    match idx {
        0 => open_language_select(app),
        1 => open_theme_select(app),
        2 => start_settings_preference_edit(app),
        3 => {
            let _ = debug_log(app, None);
        }
        _ => {}
    }
}

/// 进入语言选择列表。
pub fn open_language_select(app: &mut App) {
    let selected = crate::screens::settings_language_select::OPTIONS
        .iter()
        .position(|(c, _)| *c == current_language(app))
        .unwrap_or(0);
    app.previous_phase = Some(app.phase.clone());
    app.phase = AppPhase::SettingsLanguageSelect { selected };
}

/// 进入主题选择列表。
pub fn open_theme_select(app: &mut App) {
    let selected = crate::screens::settings_theme_select::OPTIONS
        .iter()
        .position(|(c, _)| *c == current_theme(app))
        .unwrap_or(0);
    app.previous_phase = Some(app.phase.clone());
    app.phase = AppPhase::SettingsThemeSelect { selected };
}

/// 应用新语言：写入 ui_preferred，更新运行时 i18n locale，显示 success toast。
pub fn apply_language(app: &mut App, code: &str) {
    let mut prefs = load_ui_prefs(app);
    prefs.insert("language".to_string(), Value::String(code.to_string()));
    if let Err(e) = save_ui_prefs(app, &prefs) {
        app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e));
        crate::commands::core::back(app);
        return;
    }
    // 同步更新运行时 i18n locale
    app.i18n.set_locale(code);
    app.success_message = Some((
        t!(app.i18n, "cmd-language-set", code = code),
        Instant::now(),
    ));
    // 显式抹为 SettingsMenu + 刷新 current_language；不调 core::back 让 phase 能覆盖为当前 phase (Home/Locked)。
    let current_theme = current_theme(app);
    app.phase = AppPhase::SettingsMenu {
        selected: 0,
        current_language: code.to_string(),
        current_theme,
    };
    if let Some(AppPhase::SettingsMenu { .. }) = app.previous_phase.as_ref() {
        app.previous_phase = None;
    }
}

/// 应用新主题：写入 ui_prefs，并显示 success toast，回退到 SettingsMenu。
pub fn apply_theme(app: &mut App, name: &str) {
    let mut prefs = load_ui_prefs(app);
    prefs.insert("theme".to_string(), Value::String(name.to_string()));
    if let Err(e) = save_ui_prefs(app, &prefs) {
        app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e));
        crate::commands::core::back(app);
        return;
    }
    app.success_message = Some((t!(app.i18n, "cmd-theme-set", name = name), Instant::now()));
    let current_language = current_language(app);
    app.phase = AppPhase::SettingsMenu {
        selected: 0,
        current_language,
        current_theme: name.to_string(),
    };
    if let Some(AppPhase::SettingsMenu { .. }) = app.previous_phase.as_ref() {
        app.previous_phase = None;
    }
}

/// 进入「自定义偏好键值」向导：phase 设为 SettingsPreferenceEdit 并弹出键名 prompt。
pub fn start_settings_preference_edit(app: &mut App) {
    app.previous_phase = Some(app.phase.clone());
    app.phase = AppPhase::SettingsPreferenceEdit;
    open_preference_key_prompt(app);
}

/// 弹出键名输入 prompt；用户在 prompt 中确认 → 链入 open_preference_value_prompt。
fn open_preference_key_prompt(app: &mut App) {
    crate::widgets::prompt::open(
        app,
        PromptSpec::Text {
            label: "偏好键名（例：notifications）".to_string(),
            initial: String::new(),
            mask: false,
            allow_toggle_mask: false,
        },
        Box::new(|app, result| match result {
            PromptResult::Text(raw) => {
                let key = raw.trim().to_string();
                if key.is_empty() {
                    app.error_message = Some(t!(app.i18n, "cmd-key-empty"));
                    crate::commands::core::back(app);
                } else {
                    open_preference_value_prompt(app, key);
                }
            }
            PromptResult::Cancel => {
                crate::commands::core::back(app);
            }
            _ => {}
        }),
    );
}

/// 弹出偏好值输入 prompt；用户在 prompt 中确认 → 复用 `handle({/setting, key, value})` 写入。
fn open_preference_value_prompt(app: &mut App, key: String) {
    use crate::widgets::prompt::PromptCallback;
    let label = t!(app.i18n, "cmd-preference-value-label", key = &key);
    let code = key;
    let on_done: PromptCallback = Box::new(move |app, result| match result {
        PromptResult::Text(value) => {
            let _ = handle(app, &["/setting", &code, &value]);
            crate::commands::core::back(app);
        }
        PromptResult::Cancel => {
            crate::commands::core::back(app);
        }
        _ => {}
    });
    crate::widgets::prompt::open(
        app,
        PromptSpec::Text {
            label,
            initial: String::new(),
            mask: false,
            allow_toggle_mask: false,
        },
        on_done,
    );
}

/// UI 偏好设置文件路径。
fn ui_prefs_path(app: &App) -> std::path::PathBuf {
    app.vault_service.base_path().join("ui_preferences.json")
}

/// 加载 UI 偏好，缺失字段使用默认值填充。
pub(crate) fn load_ui_prefs(app: &App) -> Map<String, Value> {
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
    map.entry("language")
        .or_insert_with(|| Value::String(String::new()));

    map
}

/// 保存 UI 偏好到 `{base}/ui_preferences.json`。
fn save_ui_prefs(app: &mut App, prefs: &Map<String, Value>) -> Result<()> {
    let path = ui_prefs_path(app);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e));
            color_eyre::eyre::eyre!(e)
        })?;
    }
    let json = serde_json::to_string(&Value::Object(prefs.clone())).map_err(|e| {
        app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e));
        color_eyre::eyre::eyre!(e)
    })?;
    std::fs::write(&path, json).map_err(|e| {
        app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e));
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
            // 同步更新运行时 i18n locale
            app.i18n.set_locale(lang);
            app.error_message = Some(t!(app.i18n, "current-language", code = lang));
        }
        None => {
            let current = prefs["language"].as_str().unwrap_or("");
            app.error_message = Some(t!(app.i18n, "current-language", code = current));
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
            app.error_message = Some(t!(app.i18n, "current-theme", code = theme));
        }
        None => {
            let current = prefs["theme"].as_str().unwrap_or("system");
            app.error_message = Some(t!(app.i18n, "current-theme", code = current));
        }
    }
    Ok(())
}

/// 执行 `/setting <key> <value>`：更新加密用户偏好。
fn setting(app: &mut App, key: Option<&str>, value: Option<&str>) -> Result<()> {
    let key = match key {
        Some(k) => k,
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-setting-usage"));
            return Ok(());
        }
    };
    let value = match value {
        Some(v) => crate::commands::parse_value(v),
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-setting-usage"));
            return Ok(());
        }
    };

    crate::commands::update_profile_preference(app, key, value)?;
    app.success_message = Some((
        t!(app.i18n, "cmd-preference-updated", key = key),
        Instant::now(),
    ));
    Ok(())
}

/// 执行 `/debug_log [file_name]`：导出诊断包。\n///
/// 成功时写入 `success_message`（绿色 toast，5 秒过期），失败保留 `error_message`（红色 overlay）。\n/// 设计目标：调用方不再依赖字符串前缀判定成败，避免诊包路径文案变更引起静默兼容性问题。
fn debug_log(app: &mut App, file_name: Option<&str>) -> Result<()> {
    let (account_id, vault) = require_unlocked_with_vault(app)?;

    let entries = vault
        .list_audit_log(10000)
        .map_err(|e| color_eyre::eyre::eyre!(e))?;

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
        app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e));
        color_eyre::eyre::eyre!(e)
    })?;

    let logs_dir = app.vault_service.base_path().join("logs");
    std::fs::create_dir_all(&logs_dir).map_err(|e| {
        app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e));
        color_eyre::eyre::eyre!(e)
    })?;

    let file_name = file_name.unwrap_or("debug_log.json");
    let file_name = Path::new(file_name)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "debug_log.json".to_string());
    let path = logs_dir.join(&file_name);

    std::fs::write(&path, &json).map_err(|e| {
        app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e));
        color_eyre::eyre::eyre!(e)
    })?;

    app.success_message = Some((
        t!(
            app.i18n,
            "cmd-debug-log-exported",
            path = &path.display().to_string()
        ),
        Instant::now(),
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use solosoul_core::VaultService;
    use std::sync::Arc;
    use zeroize::Zeroizing;

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

    #[test]
    fn test_language_get_and_set() {
        let (mut app, _id, _dir) = unlocked_app();

        // 获取默认值
        handle(&mut app, &["/language"]).unwrap();
        assert!(
            app.error_message.is_some(),
            "language get should set an error_message"
        );

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
        assert!(
            app.error_message.is_some(),
            "theme get should set an error_message"
        );

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

    // === Phase 方案 C：菜单/语言/主题 dispatch 测试 ===

    #[test]
    fn test_open_menu_sets_settings_menu_phase() {
        let (mut app, _id, _dir) = unlocked_app();
        // 模拟 home → 进入 menu：设置 app.phase=Home，让 open_menu snapshot 为 previous_phase。
        app.phase = crate::app::AppPhase::Home {
            account_id: "acc".to_string(),
        };
        open_menu(&mut app);

        match &app.phase {
            AppPhase::SettingsMenu {
                selected,
                current_theme,
                ..
            } => {
                assert_eq!(*selected, 0);
                assert!(current_theme == "system"); // 默认主题
            }
            _ => panic!("expected SettingsMenu, got {:?}", app.phase),
        }
        assert!(matches!(app.previous_phase, Some(AppPhase::Home { .. })));
    }

    #[test]
    fn test_apply_language_writes_preferences_and_refreshes_menu() {
        let (mut app, _id, _dir) = unlocked_app();
        // 直接走 open_menu → 模拟 phase 跳转
        open_menu(&mut app);

        // 切语言
        apply_language(&mut app, "en-US");

        // 验证写入
        let path = app.vault_service.base_path().join("ui_preferences.json");
        let content = std::fs::read_to_string(&path).unwrap();
        let prefs: Value = serde_json::from_str(&content).unwrap();
        assert_eq!(prefs["language"], "en-US");

        // success_message 应设置
        assert!(
            app.success_message.is_some(),
            "apply_language 应设置 success_message"
        );

        // phase 应回 SettingsMenu 且 current_language 已刷新
        match &app.phase {
            AppPhase::SettingsMenu {
                current_language, ..
            } => assert_eq!(current_language, "en-US"),
            _ => panic!(
                "expected SettingsMenu after apply_language, got {:?}",
                app.phase
            ),
        }
    }

    #[test]
    fn test_apply_theme_writes_preferences_and_refreshes_menu() {
        let (mut app, _id, _dir) = unlocked_app();
        open_menu(&mut app);

        apply_theme(&mut app, "dark");

        let path = app.vault_service.base_path().join("ui_preferences.json");
        let content = std::fs::read_to_string(&path).unwrap();
        let prefs: Value = serde_json::from_str(&content).unwrap();
        assert_eq!(prefs["theme"], "dark");

        assert!(app.success_message.is_some());
        if let AppPhase::SettingsMenu { current_theme, .. } = &app.phase {
            assert_eq!(current_theme, "dark");
        } else {
            panic!("expected SettingsMenu after apply_theme");
        }
    }

    #[test]
    fn test_handle_setting_with_two_args_still_writes_profile() {
        // 兼容性：直接键入 `/setting <key> <value>` 仍走 handle 路径（走加密 profile preferences）。
        let (mut app, account_id, _dir) = unlocked_app();
        // 脚本式调用要求 Vault 已解锁；unlocked_app() 仅创建不解锁。
        app.vault_service
            .unlock_secure(
                &account_id,
                &Zeroizing::new(crate::TEST_PASSWORD.to_string()),
            )
            .unwrap();
        handle(&mut app, &["/setting", "ui.theme", "\"dark\""]).unwrap();
        let vault = app.vault_service.get_vault_store().unwrap();
        let profile = vault.load_profile(&account_id).unwrap().unwrap();
        let data: Value = serde_json::from_slice(&profile.data).unwrap();
        assert_eq!(data["preferences"]["ui.theme"], "dark");
    }

    #[test]
    fn test_open_language_select_populates_selected_from_current() {
        let (mut app, _id, _dir) = unlocked_app();
        open_menu(&mut app);
        apply_language(&mut app, "en-US");
        // 现在 phase 是 SettingsMenu，selected=0, current_language=en-US。
        // 重入 language select，selected 应定位到 en-US 的索引 (1)。
        open_language_select(&mut app);
        match &app.phase {
            AppPhase::SettingsLanguageSelect { selected } => {
                assert_eq!(*selected, 1);
            }
            _ => panic!("expected SettingsLanguageSelect, got {:?}", app.phase),
        }
    }

    #[test]
    fn test_open_theme_select_populates_selected_from_current() {
        let (mut app, _id, _dir) = unlocked_app();
        open_menu(&mut app);
        apply_theme(&mut app, "dark");
        open_theme_select(&mut app);
        match &app.phase {
            AppPhase::SettingsThemeSelect { selected } => {
                assert_eq!(*selected, 2); // ("dark" 是位置的索引 2)
            }
            _ => panic!("expected SettingsThemeSelect, got {:?}", app.phase),
        }
    }
}
