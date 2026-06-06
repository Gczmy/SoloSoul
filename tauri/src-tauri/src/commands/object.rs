//! Object CRUD commands (formerly unified_object)
//! Uses terminology approved per 21_矛盾冲突与待审批事项.md: UnifiedObject → Object

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectSummary {
    pub id: String,
    pub name: String,
    pub collection_type: String,
    pub sensitivity_level: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectData {
    pub id: String,
    pub account_id: String,
    pub name: String,
    pub collection_type: String,
    pub properties: serde_json::Value,
    pub sensitivity_level: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateObjectInput {
    pub account_id: String,
    pub name: String,
    pub collection_type: String,
    pub properties: serde_json::Value,
}

#[derive(Deserialize)]
pub struct UpdateObjectInput {
    pub name: String,
    pub properties: serde_json::Value,
    pub sensitivity_level: Option<String>,
}

#[derive(Deserialize)]
pub struct ObjectFilter {
    pub collection_type: Option<String>,
    pub sensitivity_level: Option<String>,
    pub keyword: Option<String>,
}

#[tauri::command]
pub async fn object_list(
    state: State<'_, AppState>,
    _account_id: String,
    _filter: Option<ObjectFilter>,
) -> Result<Vec<ObjectSummary>, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let profiles = vault.list_profiles().map_err(|e| e.to_string())?;
    let mut objects = Vec::new();
    for p in profiles {
        objects.push(ObjectSummary {
            id: p.id,
            name: p.name,
            collection_type: "profile".to_string(),
            sensitivity_level: "internal".to_string(),
            created_at: p.created_at.to_rfc3339(),
            updated_at: p.updated_at.to_rfc3339(),
        });
    }
    Ok(objects)
}

#[tauri::command]
pub async fn object_get(
    state: State<'_, AppState>,
    account_id: String,
    object_id: String,
) -> Result<Option<ObjectData>, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    match vault.load_profile(&object_id).map_err(|e| e.to_string())? {
        Some(profile) => Ok(Some(ObjectData {
            id: profile.id,
            account_id: account_id.clone(),
            name: profile.name,
            collection_type: "profile".to_string(),
            properties: serde_json::Value::Null,
            sensitivity_level: "internal".to_string(),
            created_at: profile.created_at.to_rfc3339(),
            updated_at: profile.updated_at.to_rfc3339(),
            deleted_at: None,
        })),
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn object_create(
    state: State<'_, AppState>,
    input: CreateObjectInput,
) -> Result<ObjectData, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let data = serde_json::to_vec(&input.properties).map_err(|e| e.to_string())?;
    let profile = solosoul_vault::Profile::new_with_id(&input.name, &input.name, data);
    vault.save_profile(&profile).map_err(|e| e.to_string())?;

    Ok(ObjectData {
        id: profile.id,
        account_id: input.account_id,
        name: profile.name,
        collection_type: input.collection_type,
        properties: input.properties,
        sensitivity_level: "internal".to_string(),
        created_at: profile.created_at.to_rfc3339(),
        updated_at: profile.updated_at.to_rfc3339(),
        deleted_at: None,
    })
}

#[tauri::command]
pub async fn object_update(
    state: State<'_, AppState>,
    object_id: String,
    input: UpdateObjectInput,
) -> Result<ObjectData, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let mut profile = vault
        .load_profile(&object_id)
        .map_err(|e| e.to_string())?
        .ok_or("Object not found".to_string())?;

    profile.name = input.name;
    profile.data = serde_json::to_vec(&input.properties).map_err(|e| e.to_string())?;
    profile.updated_at = chrono::Utc::now();
    profile.version += 1;
    vault.save_profile(&profile).map_err(|e| e.to_string())?;

    Ok(ObjectData {
        id: profile.id,
        account_id: object_id,
        name: profile.name,
        collection_type: "profile".to_string(),
        properties: input.properties,
        sensitivity_level: input.sensitivity_level.unwrap_or("internal".to_string()),
        created_at: profile.created_at.to_rfc3339(),
        updated_at: profile.updated_at.to_rfc3339(),
        deleted_at: None,
    })
}

#[tauri::command]
pub async fn object_delete(state: State<'_, AppState>, object_id: String) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
    vault.delete_profile(&object_id).map_err(|e| e.to_string())
}
