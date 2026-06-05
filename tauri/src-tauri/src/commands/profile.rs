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

#[derive(Serialize, Deserialize)]
pub struct SectionData {
    pub section_type: String,
    pub fields: Vec<FieldValue>,
}

#[derive(Serialize, Deserialize)]
pub struct FieldValue {
    pub key: String,
    pub label: String,
    pub value: serde_json::Value,
    pub sensitivity_level: Option<String>,
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

#[tauri::command]
pub async fn profile_get_section(
    state: State<'_, AppState>,
    account_id: String,
    section_type: String,
) -> Result<Option<SectionData>, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let profile = vault.load_profile(&account_id)?;
    match profile {
        Some(p) => {
            let data: serde_json::Value = serde_json::from_slice(&p.data)
                .map_err(|e| format!("Parse error: {}", e))?;
            let sections = data.get("sections").and_then(|s| s.as_array());
            if let Some(sections) = sections {
                for sec in sections {
                    if sec.get("type").and_then(|t| t.as_str()) == Some(&section_type) {
                        let st = sec.get("type").and_then(|t| t.as_str()).unwrap_or("").to_string();
                        let fields: Vec<FieldValue> = sec.get("fields")
                            .and_then(|f| f.as_array())
                            .map(|arr| {
                                arr.iter().map(|f| FieldValue {
                                    key: f.get("key").and_then(|k| k.as_str()).unwrap_or("").to_string(),
                                    label: f.get("label").and_then(|l| l.as_str()).unwrap_or("").to_string(),
                                    value: f.get("value").cloned().unwrap_or(serde_json::Value::Null),
                                    sensitivity_level: f.get("sensitivityLevel").and_then(|s| s.as_str()).map(|s| s.to_string()),
                                }).collect()
                            })
                            .unwrap_or_default();
                        return Ok(Some(SectionData { section_type: st, fields }));
                    }
                }
            }
            Ok(None)
        }
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn profile_update_field(
    state: State<'_, AppState>,
    account_id: String,
    section_type: String,
    field_key: String,
    field_value: serde_json::Value,
) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let mut profile = vault.load_profile(&account_id)?
        .ok_or("Profile not found")?;

    let mut data: serde_json::Value = serde_json::from_slice(&profile.data)
        .map_err(|e| format!("Parse error: {}", e))?;

    if let Some(sections) = data.get_mut("sections").and_then(|s| s.as_array_mut()) {
        for sec in sections.iter_mut() {
            if sec.get("type").and_then(|t| t.as_str()) == Some(&section_type) {
                if let Some(fields) = sec.get_mut("fields").and_then(|f| f.as_array_mut()) {
                    for field in fields.iter_mut() {
                        if field.get("key").and_then(|k| k.as_str()) == Some(&field_key) {
                            field["value"] = field_value.clone();
                            break;
                        }
                    }
                }
                break;
            }
        }
    }

    profile.data = serde_json::to_vec(&data).map_err(|e| e.to_string())?;
    profile.updated_at = chrono::Utc::now();
    profile.version += 1;
    vault.save_profile(&profile)
}
