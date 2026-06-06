use crate::state::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct AccountInfo {
    pub id: String,
    pub name: String,
    pub salt: String,
    pub verify_hash: String,
    pub password_hint: Option<String>,
    pub created_at: Option<String>,
}

#[tauri::command]
pub async fn check_has_account(state: State<'_, AppState>) -> Result<bool, String> {
    let svc = state.vault_service.read().await;
    Ok(svc.has_any_account())
}

#[tauri::command]
pub async fn bootstrap(
    state: State<'_, AppState>,
    account_name: String,
    password: String,
) -> Result<AccountInfo, String> {
    let svc = state.vault_service.read().await;
    let result = svc.create_account(&account_name, &password)?;
    Ok(AccountInfo {
        id: result["id"].as_str().unwrap_or("").to_string(),
        name: result["name"].as_str().unwrap_or("").to_string(),
        salt: result["salt"].as_str().unwrap_or("").to_string(),
        verify_hash: result["verifyHash"].as_str().unwrap_or("").to_string(),
        password_hint: None,
        created_at: Some(chrono::Utc::now().to_rfc3339()),
    })
}

#[tauri::command]
pub async fn login(
    state: State<'_, AppState>,
    account_id: String,
    password: String,
) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    svc.unlock(&account_id, &password)
}

#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    svc.lock();
    Ok(())
}

#[tauri::command]
pub async fn get_current_account(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let svc = state.vault_service.read().await;
    Ok(svc.get_current_account())
}
