use crate::state::AppState;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::State;

// ── Plaintext UI preferences (§4.1) ─────────────────────────

fn ui_prefs_path(svc: &crate::services::vault_service::VaultService) -> PathBuf {
    svc.base_path().join("ui_preferences.json")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiPreferences {
    pub theme: String,
    pub accent_color: String,
    pub language: String,
    #[serde(default)]
    pub window_size: Option<WindowSize>,
    #[serde(default)]
    pub has_seen_onboarding: bool,
}

impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            theme: "system".to_string(),
            accent_color: "ocean".to_string(),
            language: String::new(),
            window_size: None,
            has_seen_onboarding: false,
        }
    }
}

#[tauri::command]
pub async fn ui_get_preferences(state: State<'_, AppState>) -> Result<UiPreferences, String> {
    let svc = state.vault_service.read().await;
    let path = ui_prefs_path(&svc);
    if !path.exists() {
        return Ok(UiPreferences::default());
    }
    let content = std::fs::read_to_string(&path).map_err(|e| format!("Read UI prefs: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Parse UI prefs: {}", e))
}

#[tauri::command]
pub async fn ui_update_preference(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let path = ui_prefs_path(&svc);
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
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

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
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    // Load or create profile so preferences can always be saved.
    // This mirrors user_data_get_preferences which returns an empty map
    // when the profile doesn't exist — the two must be symmetric.
    let mut profile = match vault.load_profile(&payload.account_id) {
        Ok(Some(profile)) => profile,
        Ok(None) => solosoul_vault::Profile::new_with_id(
            &payload.account_id,
            &payload.account_id,
            Vec::new(),
        ),
        Err(e) => return Err(format!("Failed to load profile: {}", e)),
    };

    let mut data: Value = if profile.data.is_empty() {
        Value::Object(serde_json::Map::new())
    } else {
        serde_json::from_slice(&profile.data).map_err(|e| format!("Parse error: {}", e))?
    };

    let prefs = data.get_mut("preferences").and_then(|p| p.as_object_mut());

    if let Some(prefs) = prefs {
        for (k, v) in &payload.preferences {
            prefs.insert(k.clone(), v.clone());
        }
    } else {
        if let Some(obj) = data.as_object_mut() {
            let map: serde_json::Map<String, Value> = payload.preferences.into_iter().collect();
            obj.insert("preferences".to_string(), Value::Object(map));
        }
    }

    profile.data = serde_json::to_vec(&data).map_err(|e| e.to_string())?;
    profile.updated_at = chrono::Utc::now();
    profile.version += 1;
    vault.save_profile(&profile)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solosoul_vault::{Profile, VaultConfig, VaultStore};
    use tempfile::TempDir;

    fn setup_vault() -> (VaultStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let config = VaultConfig::new("test_account", dir.path().to_path_buf());
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
            window_size: None,
            has_seen_onboarding: true,
        };
        let json = serde_json::to_string(&original).unwrap();
        assert!(json.contains("\"theme\":\"dark\""));
        assert!(json.contains("\"accentColor\":\"rose\""));
        assert!(json.contains("\"language\":\"zh-CN\""));
        assert!(json.contains("\"hasSeenOnboarding\":true"));
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
}
