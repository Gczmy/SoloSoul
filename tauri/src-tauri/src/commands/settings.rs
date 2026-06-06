use crate::state::AppState;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tauri::State;

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
