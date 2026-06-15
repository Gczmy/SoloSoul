use crate::commands::auth::verify_password_core;
use crate::state::AppState;
use tauri::{Emitter, State};

#[tauri::command]
pub async fn unlock(
    state: State<'_, AppState>,
    account_id: String,
    password: String,
) -> Result<(), String> {
    let svc = state.vault_service.read().unwrap();
    svc.unlock(&account_id, &password)?;
    Ok(())
}

#[tauri::command]
pub async fn lock(state: State<'_, AppState>) -> Result<(), String> {
    let app_handle = state.app_handle().clone();
    let svc = state.vault_service.read().unwrap();
    svc.lock();
    // Emit event so frontend can clear sensitive stores and redirect to login
    let _ = app_handle.emit("vault-locked", ());
    Ok(())
}

#[tauri::command]
pub async fn get_state(state: State<'_, AppState>) -> Result<String, String> {
    let svc = state.vault_service.read().unwrap();
    Ok(svc.get_vault_state())
}

#[tauri::command]
pub async fn change_password(
    state: State<'_, AppState>,
    account_id: String,
    old_password: String,
    new_password: String,
) -> Result<(), String> {
    let svc = state.vault_service.read().unwrap();
    svc.change_password(&account_id, &old_password, &new_password)
}

#[tauri::command]
pub async fn delete_account(
    state: State<'_, AppState>,
    account_id: String,
    password: String,
) -> Result<(), String> {
    let svc = state.vault_service.read().unwrap();
    // Verify password before allowing destructive account deletion
    let config_path = svc.base_path().join(&account_id).join("config.json");
    let content =
        std::fs::read_to_string(&config_path).map_err(|_| "Account not found".to_string())?;
    let config: crate::services::vault_service::AccountConfig =
        serde_json::from_str(&content).map_err(|_| "Parse error".to_string())?;
    if !verify_password_core(&password, &config)? {
        return Err("Invalid password".to_string());
    }
    drop(config);
    svc.delete_account(&account_id)
}

#[tauri::command]
pub async fn vault_list_accounts(
    state: State<'_, AppState>,
) -> Result<Vec<crate::services::vault_service::AccountSummary>, String> {
    let vault_service = state.vault_service.clone();
    tokio::task::spawn_blocking(move || {
        let svc = vault_service
            .read()
            .map_err(|_| "Vault service lock is poisoned".to_string())?;
        let accounts = svc.list_accounts();
        if accounts.is_empty() {
            return Err("Vault account cache is empty".to_string());
        }
        Ok(accounts)
    })
    .await
    .map_err(|e| format!("vault_list_accounts task failed: {}", e))?
}

#[tauri::command]
pub async fn vault_update_hint(
    state: State<'_, AppState>,
    account_id: String,
    hint: Option<String>,
) -> Result<(), String> {
    let svc = state.vault_service.read().unwrap();
    svc.update_password_hint(&account_id, hint.as_deref().unwrap_or(""))
}
