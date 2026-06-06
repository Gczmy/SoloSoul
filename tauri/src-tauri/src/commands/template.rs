//! User template commands (§25.3.4)

use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn template_save_from_object(
    state: State<'_, AppState>,
    object_id: String,
    template_name: String,
    icon_id: Option<String>,
) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
    let record = vault.load_object(&object_id)?.ok_or("Object not found")?;
    let template_props: Vec<serde_json::Value> = if let serde_json::Value::Object(ref props) = record.properties {
        props.iter().map(|(k, _)| serde_json::json!({"id": k, "name": k, "type": "text"})).collect()
    } else { vec![] };
    let template = serde_json::json!({
        "name": template_name, "iconId": icon_id.unwrap_or_else(|| "document".to_string()),
        "typeId": record.type_id, "properties": template_props,
        "createdAt": chrono::Utc::now().to_rfc3339(),
    });
    let account_id = &record.account_id;
    let mut profile = match vault.load_profile(account_id) {
        Ok(Some(p)) => p,
        Ok(None) => solosoul_vault::Profile::new_with_id(account_id, account_id, Vec::new()),
        Err(e) => return Err(format!("Load: {}", e)),
    };
    let mut data: serde_json::Value = if profile.data.is_empty() { serde_json::Value::Object(serde_json::Map::new()) }
        else { serde_json::from_slice(&profile.data).map_err(|e| format!("Parse: {}", e))? };
    let prefs = data.as_object_mut().ok_or("Invalid")?
        .entry("preferences".to_string()).or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    let mut templates: Vec<serde_json::Value> = prefs.get("userTemplates")
        .and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
    templates.push(template);
    prefs["userTemplates"] = serde_json::to_value(&templates).map_err(|e| e.to_string())?;
    profile.data = serde_json::to_vec(&data).map_err(|e| e.to_string())?;
    profile.updated_at = chrono::Utc::now(); profile.version += 1;
    vault.save_profile(&profile)?;
    let _ = vault.log_action("template_save", &format!("object={}", object_id));
    Ok(())
}

#[tauri::command]
pub async fn template_list(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
    match vault.load_profile(&account_id) {
        Ok(Some(profile)) => {
            let data: serde_json::Value = serde_json::from_slice(&profile.data).map_err(|e| format!("Parse: {}", e))?;
            Ok(data.get("preferences").and_then(|p| p.get("userTemplates"))
                .and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default())
        }
        _ => Ok(vec![]),
    }
}
