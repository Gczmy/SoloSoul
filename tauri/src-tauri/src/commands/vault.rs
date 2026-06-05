use tauri::{State, Emitter};
use crate::state::AppState;

#[tauri::command]
pub async fn unlock(
    state: State<'_, AppState>,
    account_id: String,
    password: String,
) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    svc.unlock(&account_id, &password)
}

#[tauri::command]
pub async fn lock(state: State<'_, AppState>) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    svc.lock();
    Ok(())
}

#[tauri::command]
pub async fn get_state(state: State<'_, AppState>) -> Result<String, String> {
    let svc = state.vault_service.read().await;
    Ok(svc.get_vault_state())
}

#[tauri::command]
pub async fn change_password(
    state: State<'_, AppState>,
    account_id: String,
    old_password: String,
    new_password: String,
) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    svc.change_password(&account_id, &old_password, &new_password)
}

#[tauri::command]
pub async fn delete_account(
    state: State<'_, AppState>,
    account_id: String,
    password: String,
) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    svc.unlock(&account_id, &password)?;
    svc.delete_account(&account_id)
}

#[tauri::command]
pub async fn list_accounts(state: State<'_, AppState>) -> Result<Vec<serde_json::Value>, String> {
    let svc = state.vault_service.read().await;
    Ok(svc.list_accounts())
}
