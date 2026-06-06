//! Collection commands — user-defined collections with smart filter support
//! Per §17 术语规范: Collection = 集合, SmartCollection = 智能集合

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

// ── Types ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionDef {
    pub id: String,
    pub name: String,
    pub icon_name: String,
    pub description: Option<String>,
    /// Filter rules for smart collections
    pub filter: Option<SmartFilter>,
    pub sort_order: usize,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartFilter {
    /// Filter type: "all" | "any"
    pub operator: String,
    pub conditions: Vec<FilterCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterCondition {
    /// Property key to match against
    pub field: String,
    /// Operator: "equals" | "contains" | "startsWith" | "greaterThan" | "lessThan" | "before" | "after"
    pub op: String,
    /// Value to compare
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCollectionInput {
    pub account_id: String,
    pub name: String,
    pub icon_name: Option<String>,
    pub description: Option<String>,
    pub filter: Option<SmartFilter>,
}

// ── Command implementations ────────────────────────────────

#[tauri::command]
pub async fn collection_list(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<Vec<CollectionDef>, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    // Load collections from the account profile's preferences
    match vault.load_profile(&account_id) {
        Ok(Some(profile)) => {
            let data: serde_json::Value =
                serde_json::from_slice(&profile.data).map_err(|e| format!("Parse: {}", e))?;
            let prefs = data.get("preferences").and_then(|p| p.as_object());
            let cols = prefs
                .and_then(|p| p.get("collections"))
                .and_then(|v| serde_json::from_value::<Vec<CollectionDef>>(v.clone()).ok())
                .unwrap_or_default();
            Ok(cols)
        }
        _ => Ok(vec![]),
    }
}

#[tauri::command]
pub async fn collection_create(
    state: State<'_, AppState>,
    input: CreateCollectionInput,
) -> Result<CollectionDef, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let col = CollectionDef {
        id: Uuid::new_v4().to_string(),
        name: input.name.clone(),
        icon_name: input.icon_name.unwrap_or_else(|| "folder".to_string()),
        description: input.description,
        filter: input.filter,
        sort_order: 0,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    // Save to account profile preferences
    let mut profile = match vault.load_profile(&input.account_id) {
        Ok(Some(p)) => p,
        Ok(None) => solosoul_vault::Profile::new_with_id(
            &input.account_id, &input.account_id, Vec::new(),
        ),
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

    let mut cols: Vec<CollectionDef> = prefs
        .get("collections")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    cols.push(col.clone());
    prefs["collections"] = serde_json::to_value(&cols).map_err(|e| e.to_string())?;

    profile.data = serde_json::to_vec(&data).map_err(|e| e.to_string())?;
    profile.updated_at = chrono::Utc::now();
    profile.version += 1;
    vault.save_profile(&profile)?;

    Ok(col)
}

#[tauri::command]
pub async fn collection_delete(
    state: State<'_, AppState>,
    account_id: String,
    collection_id: String,
) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let mut profile = vault
        .load_profile(&account_id)
        .map_err(|e| e.to_string())?
        .ok_or("Profile not found")?;

    let mut data: serde_json::Value =
        serde_json::from_slice(&profile.data).map_err(|e| format!("Parse: {}", e))?;

    if let Some(prefs) = data.get_mut("preferences").and_then(|p| p.as_object_mut()) {
        let cols: Vec<CollectionDef> = prefs
            .get("collections")
            .and_then(|v| serde_json::from_value::<Vec<CollectionDef>>(v.clone()).ok())
            .unwrap_or_default()
            .into_iter()
            .filter(|c: &CollectionDef| c.id != collection_id)
            .collect();
        prefs["collections"] = serde_json::to_value(&cols).map_err(|e| e.to_string())?;
    }

    profile.data = serde_json::to_vec(&data).map_err(|e| e.to_string())?;
    profile.updated_at = chrono::Utc::now();
    profile.version += 1;
    vault.save_profile(&profile)
}
