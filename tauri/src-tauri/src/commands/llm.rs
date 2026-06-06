//! LLM configuration commands — provider settings stored in vault preferences.
//! Actual chat API calls are made from the frontend (avoiding Rust reqwest dependency).

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmConfig {
    pub provider: String,        // "openai" | "anthropic" | "ollama"
    pub api_key: String,         // encrypted at rest in vault
    pub model: String,           // e.g. "gpt-4o", "claude-opus-4-8", "llama3"
    pub base_url: Option<String>, // for Ollama / custom endpoints
    pub enabled: bool,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: "ollama".to_string(),
            api_key: String::new(),
            model: "llama3".to_string(),
            base_url: Some("http://localhost:11434".to_string()),
            enabled: false,
        }
    }
}

#[tauri::command]
pub async fn llm_get_config(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<LlmConfig, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    match vault.load_profile(&account_id) {
        Ok(Some(profile)) => {
            let data: serde_json::Value =
                serde_json::from_slice(&profile.data).map_err(|e| format!("Parse: {}", e))?;
            let prefs = data.get("preferences").and_then(|p| p.as_object());
            if let Some(llm) = prefs.and_then(|p| p.get("llmConfig")) {
                serde_json::from_value(llm.clone()).map_err(|e| format!("Parse: {}", e))
            } else {
                Ok(LlmConfig::default())
            }
        }
        _ => Ok(LlmConfig::default()),
    }
}

#[tauri::command]
pub async fn llm_update_config(
    state: State<'_, AppState>,
    account_id: String,
    config: LlmConfig,
) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let mut profile = match vault.load_profile(&account_id) {
        Ok(Some(p)) => p,
        Ok(None) => solosoul_vault::Profile::new_with_id(&account_id, &account_id, Vec::new()),
        Err(e) => return Err(format!("Load profile: {}", e)),
    };

    let mut data: serde_json::Value = if profile.data.is_empty() {
        serde_json::Value::Object(serde_json::Map::new())
    } else {
        serde_json::from_slice(&profile.data).map_err(|e| format!("Parse: {}", e))?
    };

    let prefs = data
        .as_object_mut()
        .ok_or("Invalid profile data")?
        .entry("preferences".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));

    prefs["llmConfig"] = serde_json::to_value(&config).map_err(|e| e.to_string())?;

    profile.data = serde_json::to_vec(&data).map_err(|e| e.to_string())?;
    profile.updated_at = chrono::Utc::now();
    profile.version += 1;
    vault.save_profile(&profile)
}
