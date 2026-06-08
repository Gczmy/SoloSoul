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
pub async fn verify_password(
    state: State<'_, AppState>,
    account_id: String,
    password: String,
) -> Result<bool, String> {
    use solosoul_crypto::kdf::{derive_key, KdfConfig};
    let svc = state.vault_service.read().await;
    let config_path = svc.base_path().join(&account_id).join("config.json");
    let content = std::fs::read_to_string(&config_path)
        .map_err(|_| "Account not found".to_string())?;
    let config: crate::services::vault_service::AccountConfig =
        serde_json::from_str(&content).map_err(|_| "Parse error".to_string())?;

    let salt_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &config.salt,
    )
    .map_err(|_| "Invalid salt".to_string())?;
    let salt_arr: [u8; 16] = salt_bytes.as_slice().try_into().map_err(|_| "Bad salt".to_string())?;

    let kdf_config = KdfConfig::balanced();
    let master_key = derive_key(&password, &salt_arr, &kdf_config)
        .map_err(|_| "KDF failed".to_string())?;

    let verify_data = b"SOLOSOUL_VAULT_VERIFY_v1";
    let verify_key = derive_key(
        &hex::encode(master_key.as_slice()),
        verify_data,
        &KdfConfig { memory_kb: 8192, iterations: 1, parallelism: 1 },
    )
    .map_err(|_| "Verify failed".to_string())?;

    Ok(hex::encode(verify_key.as_slice()) == config.verify_hash)
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
