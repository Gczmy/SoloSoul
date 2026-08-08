use crate::commands::vault_handle;
use crate::services::profile_prefs::update_profile_prefs;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
#[cfg(target_os = "android")]
use tauri::Manager;
use tauri::State;

// ── Plaintext UI preferences (§4.1) ─────────────────────────

/// 解析 UI 偏好文件 `ui_preferences.json` 的存储路径。
///
/// - Android：使用应用私有数据目录（`app_data_dir`），避免 Android 10+
///   对外部存储的访问限制和 MediaProvider 开销。
/// - 桌面端：继续放在 Vault base 目录，保证 Vault 目录可移植。
///
/// 如果 Android 上旧文件还在 Vault base（可能是外部存储），且新路径
/// 尚未存在，则自动复制一份到新路径，实现无感迁移。
pub fn resolve_ui_prefs_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    svc: &solosoul_core::vault_service::VaultService,
) -> Result<PathBuf, String> {
    #[cfg(target_os = "android")]
    {
        let new_path = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("无法获取应用数据目录: {e}"))?
            .join("ui_preferences.json");
        let old_path = svc.base_path().join("ui_preferences.json");
        if !new_path.exists() && old_path.exists() {
            if let Err(e) = maybe_migrate_ui_prefs(&old_path, &new_path) {
                tracing::warn!("迁移 UI preferences 失败: {}", e);
                // 迁移失败时回退到旧路径，避免读取时丢失用户已有的 UI 偏好。
                return Ok(old_path);
            }
        }
        // 即使迁移时没删掉，启动时再次尝试清理残留旧文件。
        lazy_cleanup_old_ui_prefs(&old_path, &new_path);
        Ok(new_path)
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Ok(svc.base_path().join("ui_preferences.json"))
    }
}
/// 判断 IO 错误是否属于值得重试的短暂性错误。
#[cfg(any(target_os = "android", test))]
fn is_retryable_io_error(kind: std::io::ErrorKind) -> bool {
    matches!(
        kind,
        std::io::ErrorKind::WouldBlock
            | std::io::ErrorKind::Interrupted
            | std::io::ErrorKind::Other
    )
}

/// 尝试删除文件，文件不存在时视为成功；对短暂性 IO 错误重试指定次数。
///
/// 对明确的永久性错误（如 PermissionDenied）会立即短路返回，避免无意义重试。
#[cfg(any(target_os = "android", test))]
fn remove_with_retry(path: &std::path::Path, retries: u32) -> std::io::Result<()> {
    let mut last_err: Option<std::io::Error> = None;
    for attempt in 0..=retries {
        match std::fs::remove_file(path) {
            Ok(()) => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => {
                let kind = e.kind();
                last_err = Some(e);
                if !is_retryable_io_error(kind) || attempt == retries {
                    break;
                }
            }
        }
    }
    match last_err {
        Some(e) => Err(e),
        // 不可达：循环至少执行一次，remove_file 失败必然落入 Err 分支并记录错误。
        None => Err(std::io::Error::other("remove_with_retry: 无可用错误详情")),
    }
}

/// 在确认新路径已存在后，尝试清理可能残留的旧 UI preferences 文件。
#[cfg(any(target_os = "android", test))]
fn lazy_cleanup_old_ui_prefs(old_path: &std::path::Path, new_path: &std::path::Path) {
    if old_path == new_path {
        return;
    }
    if !new_path.exists() || !old_path.exists() {
        return;
    }
    if let Err(e) = remove_with_retry(old_path, 3) {
        tracing::debug!("清理旧 UI preferences 失败: {}", e);
    }
}

/// 将旧的 UI preferences 文件迁移到新的私有目录。
///
/// 先复制到目标目录内的临时文件，再原子重命名为目标文件，避免留下
/// 半成品的 `ui_preferences.json`。迁移成功后删除旧文件；删除失败仅
/// 记录日志，不影响迁移结果。
#[cfg(any(target_os = "android", test))]
fn maybe_migrate_ui_prefs(
    old_path: &std::path::Path,
    new_path: &std::path::Path,
) -> Result<(), String> {
    if old_path == new_path {
        return Ok(());
    }
    if let Some(parent) = new_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建 UI preferences 父目录失败: {e}"))?;
    }

    // 与原文件同目录的临时文件，保证 `rename` 在同一块文件系统内原子完成。
    let tmp_path = new_path.with_extension("tmp");
    let copy_res =
        std::fs::copy(old_path, &tmp_path).map_err(|e| format!("复制 UI preferences 失败: {e}"));
    if let Err(e) = copy_res {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e);
    }

    if let Err(e) = std::fs::rename(&tmp_path, new_path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(format!("重命名 UI preferences 失败: {e}"));
    }

    if let Err(e) = remove_with_retry(old_path, 3) {
        tracing::warn!("删除旧 UI preferences 失败（已重试）: {}", e);
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiPreferences {
    pub theme: String,
    pub accent_color: String,
    pub language: String,
    #[serde(default)]
    pub has_seen_onboarding: bool,
    /// 应用级"已请求过通知权限"标记：系统权限对话框每次安装最多弹一次，
    /// 避免每个新账户首次触发备份提醒时重复弹窗。
    #[serde(default)]
    pub notification_permission_requested: bool,
}

impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            theme: "system".to_string(),
            accent_color: "ocean".to_string(),
            language: String::new(),
            has_seen_onboarding: false,
            notification_permission_requested: false,
        }
    }
}

#[tauri::command]
pub async fn ui_get_preferences(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<UiPreferences, String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let path = resolve_ui_prefs_path(&app, &svc)?;
    if !path.exists() {
        return Ok(UiPreferences::default());
    }
    let content = std::fs::read_to_string(&path).map_err(|e| format!("Read UI prefs: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Parse UI prefs: {}", e))
}

#[tauri::command]
pub async fn ui_update_preference(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let path = resolve_ui_prefs_path(&app, &svc)?;
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut prefs: serde_json::Value = if path.exists() {
        let content = std::fs::read_to_string(&path).map_err(|e| format!("Read: {}", e))?;
        serde_json::from_str::<serde_json::Value>(&content).unwrap_or_default()
    } else {
        serde_json::Value::Object(serde_json::Map::new())
    };
    // Handle corrupted file (e.g. literal null)
    if !prefs.is_object() {
        prefs = serde_json::Value::Object(serde_json::Map::new());
    }
    if let Some(obj) = prefs.as_object_mut() {
        // Try to parse the value as JSON so objects/numbers can be stored; fall back to string.
        let parsed = match serde_json::from_str(&value) {
            Ok(v) => v,
            Err(_) => serde_json::Value::String(value),
        };
        obj.insert(key, parsed);
    }
    let json = serde_json::to_string(&prefs).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("Write UI prefs: {}", e))?;
    Ok(())
}

/// 读取自动同步开关持久化状态（ui_preferences.json 中的 `auto_sync_enabled` 键）。
///
/// 设备自动同步开关原为内存 AtomicBool（默认 false），重启即丢；
/// 持久化到明文 UI 偏好文件（非敏感），AppState 启动时据此恢复。
pub fn read_auto_sync_pref<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    svc: &solosoul_core::vault_service::VaultService,
) -> Option<bool> {
    let path = resolve_ui_prefs_path(app, svc).ok()?;
    let content = std::fs::read_to_string(&path).ok()?;
    let prefs: serde_json::Value = serde_json::from_str(&content).ok()?;
    prefs.get("auto_sync_enabled").and_then(|v| v.as_bool())
}

/// 写入自动同步开关持久化状态（ui_preferences.json）。
/// 失败返回错误，由调用方记录日志（非关键路径，不阻断开关操作）。
pub fn write_auto_sync_pref<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    svc: &solosoul_core::vault_service::VaultService,
    enabled: bool,
) -> Result<(), String> {
    let path = resolve_ui_prefs_path(app, svc)?;
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut prefs: serde_json::Value = if path.exists() {
        let content = std::fs::read_to_string(&path).map_err(|e| format!("Read UI prefs: {e}"))?;
        serde_json::from_str::<serde_json::Value>(&content).unwrap_or_default()
    } else {
        serde_json::Value::Object(serde_json::Map::new())
    };
    if !prefs.is_object() {
        prefs = serde_json::Value::Object(serde_json::Map::new());
    }
    if let Some(obj) = prefs.as_object_mut() {
        obj.insert(
            "auto_sync_enabled".to_string(),
            serde_json::Value::Bool(enabled),
        );
    }
    let json = serde_json::to_string(&prefs).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("Write UI prefs: {e}"))?;
    Ok(())
}

/// 读取「账户设置偏好是否随设备同步」开关持久化状态（ui_preferences.json 中的
/// `ui_prefs_sync_enabled` 键）。
///
/// 该开关决定设备同步时是否携带本机的 UI 外观偏好（主题、主题色等）；
/// 持久化到明文 UI 偏好文件（非敏感），AppState 启动时据此恢复。
/// 未设置时返回 None（调用方按默认值 true 处理）。
pub fn read_ui_prefs_sync_pref<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    svc: &solosoul_core::vault_service::VaultService,
) -> Option<bool> {
    let path = resolve_ui_prefs_path(app, svc).ok()?;
    let content = std::fs::read_to_string(&path).ok()?;
    let prefs: serde_json::Value = serde_json::from_str(&content).ok()?;
    prefs.get("ui_prefs_sync_enabled").and_then(|v| v.as_bool())
}

/// 写入「账户设置偏好是否随设备同步」开关持久化状态（ui_preferences.json）。
/// 失败返回错误，由调用方记录日志（非关键路径，不阻断开关操作）。
pub fn write_ui_prefs_sync_pref<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    svc: &solosoul_core::vault_service::VaultService,
    enabled: bool,
) -> Result<(), String> {
    let path = resolve_ui_prefs_path(app, svc)?;
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut prefs: serde_json::Value = if path.exists() {
        let content = std::fs::read_to_string(&path).map_err(|e| format!("Read UI prefs: {e}"))?;
        serde_json::from_str::<serde_json::Value>(&content).unwrap_or_default()
    } else {
        serde_json::Value::Object(serde_json::Map::new())
    };
    if !prefs.is_object() {
        prefs = serde_json::Value::Object(serde_json::Map::new());
    }
    if let Some(obj) = prefs.as_object_mut() {
        obj.insert(
            "ui_prefs_sync_enabled".to_string(),
            serde_json::Value::Bool(enabled),
        );
    }
    let json = serde_json::to_string(&prefs).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("Write UI prefs: {e}"))?;
    Ok(())
}

// ── Vault-encrypted preferences ─────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePreferencesPayload {
    pub account_id: String,
    pub preferences: HashMap<String, Value>,
}

#[tauri::command]
pub async fn user_data_get_preferences(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<HashMap<String, Value>, String> {
    let vault = vault_handle(&state)?;

    // Load profile for preferences
    match vault.load_profile(&account_id) {
        Ok(Some(profile)) => {
            let data: Value =
                serde_json::from_slice(&profile.data).map_err(|e| format!("Parse error: {}", e))?;
            let prefs = data
                .get("preferences")
                .and_then(|p| p.as_object())
                .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                .unwrap_or_default();
            Ok(prefs)
        }
        _ => Ok(HashMap::new()),
    }
}

#[tauri::command]
pub async fn user_data_update_preference(
    state: State<'_, AppState>,
    payload: UpdatePreferencesPayload,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;

    // Load or create profile so preferences can always be saved.
    // This mirrors user_data_get_preferences which returns an empty map
    // when the profile doesn't exist — the two must be symmetric.
    update_profile_prefs(&vault, &payload.account_id, |prefs| {
        for (k, v) in &payload.preferences {
            prefs.insert(k.clone(), v.clone());
        }
        Ok(())
    })?;
    state.auto_sync.trigger_debounce();
    state.device_auto_sync.trigger_data_change();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use solosoul_vault::{Profile, VaultConfig, VaultStore};
    use tempfile::TempDir;

    fn setup_vault() -> (VaultStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let config =
            VaultConfig::new("test_account", dir.path().to_path_buf()).with_data_key([0x42u8; 32]);
        let vault = VaultStore::open(config).unwrap();
        (vault, dir)
    }

    #[test]
    fn test_ui_preferences_default() {
        let prefs = UiPreferences::default();
        assert_eq!(prefs.theme, "system");
        assert_eq!(prefs.accent_color, "ocean");
        assert_eq!(prefs.language, "");
        assert!(!prefs.has_seen_onboarding);
    }

    #[test]
    fn test_ui_preferences_serde_roundtrip() {
        let original = UiPreferences {
            theme: "dark".to_string(),
            accent_color: "rose".to_string(),
            language: "zh-CN".to_string(),
            has_seen_onboarding: true,
            notification_permission_requested: false,
        };
        let json = serde_json::to_string(&original).unwrap();
        assert!(json.contains("\"theme\":\"dark\""));
        assert!(json.contains("\"accentColor\":\"rose\""));
        assert!(json.contains("\"language\":\"zh-CN\""));
        assert!(json.contains("\"hasSeenOnboarding\":true"));
        assert!(json.contains("\"notificationPermissionRequested\":false"));
        let restored: UiPreferences = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.theme, original.theme);
        assert_eq!(restored.accent_color, original.accent_color);
        assert_eq!(restored.language, original.language);
        assert!(restored.has_seen_onboarding);
    }

    #[test]
    fn test_ui_preferences_missing_onboarding_defaults_to_false() {
        let json = r#"{"theme":"light","accentColor":"ocean","language":"en-US"}"#;
        let restored: UiPreferences = serde_json::from_str(json).unwrap();
        assert!(!restored.has_seen_onboarding);
    }

    #[test]
    fn test_update_preferences_payload_deserialization() {
        let json = r#"{"accountId":"acc-1","preferences":{"key1":"value1","key2":42}}"#;
        let payload: UpdatePreferencesPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.account_id, "acc-1");
        assert_eq!(payload.preferences.get("key1").unwrap(), "value1");
        assert_eq!(payload.preferences.get("key2").unwrap(), 42);
    }

    #[test]
    fn test_vault_preferences_save_and_load() {
        let (vault, _dir) = setup_vault();
        let account_id = "test_acc";

        // Simulate saving preferences via user_data_update_preference logic
        let mut profile = Profile::new_with_id(account_id, account_id, Vec::new());
        let mut data = serde_json::Map::new();
        let mut prefs = serde_json::Map::new();
        prefs.insert("theme".to_string(), serde_json::json!("dark"));
        prefs.insert("notifications".to_string(), serde_json::json!(true));
        data.insert("preferences".to_string(), serde_json::Value::Object(prefs));
        profile.data = serde_json::to_vec(&serde_json::Value::Object(data)).unwrap();

        vault.save_profile(&profile).unwrap();

        // Simulate loading preferences via user_data_get_preferences logic
        let loaded = vault.load_profile(account_id).unwrap().unwrap();
        let loaded_data: serde_json::Value = serde_json::from_slice(&loaded.data).unwrap();
        let loaded_prefs: HashMap<String, Value> = loaded_data
            .get("preferences")
            .and_then(|p| p.as_object())
            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();

        assert_eq!(loaded_prefs.get("theme").unwrap(), "dark");
        assert_eq!(loaded_prefs.get("notifications").unwrap(), true);
    }

    #[test]
    fn test_vault_preferences_update_existing() {
        let (vault, _dir) = setup_vault();
        let account_id = "test_acc";

        // Create initial profile with preferences
        let mut profile = Profile::new_with_id(account_id, account_id, Vec::new());
        let mut data = serde_json::Map::new();
        let mut prefs = serde_json::Map::new();
        prefs.insert("theme".to_string(), serde_json::json!("light"));
        data.insert("preferences".to_string(), serde_json::Value::Object(prefs));
        profile.data = serde_json::to_vec(&serde_json::Value::Object(data)).unwrap();
        vault.save_profile(&profile).unwrap();

        // Simulate update: load, modify, save
        let mut profile = vault.load_profile(account_id).unwrap().unwrap();
        let mut data: serde_json::Value = serde_json::from_slice(&profile.data).unwrap();
        let prefs = data
            .get_mut("preferences")
            .and_then(|p| p.as_object_mut())
            .unwrap();
        prefs.insert("theme".to_string(), serde_json::json!("dark"));
        prefs.insert("language".to_string(), serde_json::json!("en"));
        profile.data = serde_json::to_vec(&data).unwrap();
        profile.version += 1;
        vault.save_profile(&profile).unwrap();

        let loaded = vault.load_profile(account_id).unwrap().unwrap();
        let loaded_data: serde_json::Value = serde_json::from_slice(&loaded.data).unwrap();
        let prefs = loaded_data.get("preferences").unwrap();
        assert_eq!(prefs.get("theme").unwrap(), "dark");
        assert_eq!(prefs.get("language").unwrap(), "en");
        assert_eq!(loaded.version, 2);
    }

    #[test]
    fn test_vault_preferences_empty_profile() {
        let (vault, _dir) = setup_vault();
        let account_id = "test_acc";

        // Empty profile returns empty preferences
        let profile = Profile::new_with_id(account_id, account_id, Vec::new());
        vault.save_profile(&profile).unwrap();

        let loaded = vault.load_profile(account_id).unwrap().unwrap();
        // Mimic the command logic: treat empty data as an empty object
        let loaded_data: serde_json::Value = if loaded.data.is_empty() {
            serde_json::Value::Object(serde_json::Map::new())
        } else {
            serde_json::from_slice(&loaded.data).unwrap()
        };
        let prefs: HashMap<String, Value> = loaded_data
            .get("preferences")
            .and_then(|p| p.as_object())
            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        assert!(prefs.is_empty());
    }

    #[test]
    fn test_vault_preferences_create_on_first_save() {
        let (vault, _dir) = setup_vault();
        let account_id = "new_acc";

        // Simulate the command logic when profile does not exist:
        // create a new profile and insert preferences at root level
        let mut profile = Profile::new_with_id(account_id, account_id, Vec::new());
        let mut data = serde_json::Map::new();
        let map: serde_json::Map<String, Value> =
            [("theme".to_string(), serde_json::json!("dark"))]
                .into_iter()
                .collect();
        data.insert("preferences".to_string(), Value::Object(map));
        profile.data = serde_json::to_vec(&Value::Object(data)).unwrap();
        profile.version += 1;
        vault.save_profile(&profile).unwrap();

        let loaded = vault.load_profile(account_id).unwrap().unwrap();
        let loaded_data: serde_json::Value = serde_json::from_slice(&loaded.data).unwrap();
        let prefs = loaded_data.get("preferences").unwrap();
        assert_eq!(prefs.get("theme").unwrap(), "dark");
        assert_eq!(loaded.version, 2);
    }

    #[test]
    fn test_resolve_ui_prefs_path_desktop_uses_vault_base() {
        let app = tauri::test::mock_app();
        let dir = TempDir::new().unwrap();
        let base = dir.path().join(".solosoul");
        std::fs::create_dir_all(&base).unwrap();
        let svc = solosoul_core::vault_service::VaultService::with_base_path(base.clone());

        let path = resolve_ui_prefs_path(app.handle(), &svc).unwrap();

        // 桌面端应继续使用 Vault base 目录，保证 Vault 目录可移植
        assert_eq!(path, base.join("ui_preferences.json"));
    }

    #[test]
    fn test_resolve_ui_prefs_path_roundtrip() {
        let app = tauri::test::mock_app();
        let dir = TempDir::new().unwrap();
        let base = dir.path().join(".solosoul");
        std::fs::create_dir_all(&base).unwrap();
        let svc = solosoul_core::vault_service::VaultService::with_base_path(base);

        let path = resolve_ui_prefs_path(app.handle(), &svc).unwrap();
        let original = UiPreferences {
            theme: "dark".to_string(),
            accent_color: "ocean".to_string(),
            language: "zh-CN".to_string(),
            has_seen_onboarding: true,
            notification_permission_requested: false,
        };
        std::fs::write(&path, serde_json::to_string(&original).unwrap()).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let prefs: UiPreferences = serde_json::from_str(&content).unwrap();
        assert_eq!(prefs.language, "zh-CN");
        assert_eq!(prefs.theme, "dark");
        assert_eq!(prefs.accent_color, "ocean");
        assert!(prefs.has_seen_onboarding);
    }

    #[test]
    fn test_maybe_migrate_ui_prefs_success() {
        let dir = TempDir::new().unwrap();
        let old = dir.path().join("old_ui_preferences.json");
        let new_dir = dir.path().join("new");
        let new = new_dir.join("ui_preferences.json");
        let original = UiPreferences {
            theme: "dark".to_string(),
            accent_color: "rose".to_string(),
            language: "zh-CN".to_string(),
            has_seen_onboarding: false,
            notification_permission_requested: false,
        };
        std::fs::write(&old, serde_json::to_string(&original).unwrap()).unwrap();

        maybe_migrate_ui_prefs(&old, &new).unwrap();

        assert!(new.exists());
        assert!(!old.exists());
    }

    #[test]
    fn test_maybe_migrate_ui_prefs_same_path_is_noop() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("ui_preferences.json");
        std::fs::write(&p, "x").unwrap();

        // 同一路径时不不应删除或覆盖文件
        maybe_migrate_ui_prefs(&p, &p).unwrap();

        assert!(p.exists());
        let content = std::fs::read_to_string(&p).unwrap();
        assert_eq!(content, "x");
    }

    #[test]
    fn test_remove_with_retry_success() {
        let dir = TempDir::new().unwrap();
        let f = dir.path().join("a.txt");
        std::fs::write(&f, "x").unwrap();
        assert!(f.exists());
        remove_with_retry(&f, 3).unwrap();
        assert!(!f.exists());
    }

    #[test]
    fn test_remove_with_retry_not_found_is_ok() {
        let dir = TempDir::new().unwrap();
        let f = dir.path().join("missing.txt");
        remove_with_retry(&f, 3).unwrap();
    }

    #[test]
    fn test_remove_with_retry_failure() {
        let dir = TempDir::new().unwrap();
        let d = dir.path().join("dir");
        std::fs::create_dir(&d).unwrap();
        // 删除目录（当作文件）应失败
        assert!(remove_with_retry(&d, 2).is_err());
    }

    #[test]
    fn test_lazy_cleanup_old_ui_prefs_success() {
        let dir = TempDir::new().unwrap();
        let old = dir.path().join("old_ui_preferences.json");
        let new = dir.path().join("new_ui_preferences.json");
        std::fs::write(&old, "x").unwrap();
        std::fs::write(&new, "y").unwrap();

        lazy_cleanup_old_ui_prefs(&old, &new);

        assert!(!old.exists());
        assert!(new.exists());
    }

    #[test]
    fn test_lazy_cleanup_old_ui_prefs_same_path_is_noop() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("ui_preferences.json");
        std::fs::write(&p, "x").unwrap();

        // 同一路径时不能误删唯一副本
        lazy_cleanup_old_ui_prefs(&p, &p);

        assert!(p.exists());
    }

    #[test]
    fn test_lazy_cleanup_old_ui_prefs_new_missing_is_noop() {
        let dir = TempDir::new().unwrap();
        let old = dir.path().join("old_ui_preferences.json");
        let new = dir.path().join("new_ui_preferences.json");
        std::fs::write(&old, "x").unwrap();

        // 新文件不存在时不应删除旧文件，避免丢失数据
        lazy_cleanup_old_ui_prefs(&old, &new);

        assert!(old.exists());
        assert!(!new.exists());
    }
}
