use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize)]
pub struct ProfileSummary {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: u32,
}

#[derive(Deserialize)]
pub struct SaveProfilePayload {
    pub account_id: String,
    pub name: String,
    pub data: Vec<u8>,
}

#[derive(Deserialize)]
pub struct LoadProfilePayload {
    pub account_id: String,
}

#[tauri::command]
pub async fn profile_save(
    state: State<'_, AppState>,
    payload: SaveProfilePayload,
) -> Result<ProfileSummary, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let existing = vault
        .list_profiles()
        .ok()
        .and_then(|profiles| profiles.into_iter().find(|p| p.name == payload.name));

    let profile = if let Some(ref existing_p) = existing {
        solosoul_vault::Profile {
            id: payload.name.clone(),
            name: payload.name.clone(),
            data: payload.data,
            created_at: existing_p.created_at,
            updated_at: chrono::Utc::now(),
            version: existing_p.version + 1,
        }
    } else {
        solosoul_vault::Profile::new_with_id(&payload.name, &payload.name, payload.data)
    };

    let summary = solosoul_vault::ProfileSummary::from_profile(&profile);
    vault.save_profile(&profile)?;

    Ok(ProfileSummary {
        id: summary.id,
        name: summary.name,
        created_at: summary.created_at.to_rfc3339(),
        updated_at: summary.updated_at.to_rfc3339(),
        version: summary.version,
    })
}

#[tauri::command]
pub async fn profile_load(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<Option<solosoul_vault::Profile>, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
    vault.load_profile(&account_id)
}

#[tauri::command]
pub async fn profile_list(state: State<'_, AppState>) -> Result<Vec<ProfileSummary>, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let summaries = vault.list_profiles()?;
    Ok(summaries
        .into_iter()
        .map(|s| ProfileSummary {
            id: s.id,
            name: s.name,
            created_at: s.created_at.to_rfc3339(),
            updated_at: s.updated_at.to_rfc3339(),
            version: s.version,
        })
        .collect())
}

#[tauri::command]
pub async fn profile_delete(state: State<'_, AppState>, profile_id: String) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
    vault.delete_profile(&profile_id)
}
