use crate::core::sensitivity::{
    SensitivityLevel, SensitivityLogEntry, SensitivityManager, SensitivityMap,
};
use crate::state::AppState;
use solosoul_crypto::kdf::{derive_key, KdfConfig};
use tauri::State;

#[tauri::command]
pub async fn sensitivity_get_field(
    manager: State<'_, SensitivityManager>,
    field_id: String,
) -> Result<String, String> {
    let map = manager.map.read().await;
    Ok(map.get(&field_id).as_str().to_string())
}

#[tauri::command]
pub async fn sensitivity_get_map(
    manager: State<'_, SensitivityManager>,
) -> Result<SensitivityMap, String> {
    let map = manager.map.read().await;
    Ok(map.clone())
}

#[tauri::command]
pub async fn sensitivity_update_field(
    app: State<'_, AppState>,
    manager: State<'_, SensitivityManager>,
    field_id: String,
    new_level: String,
    password: String,
    reason: Option<String>,
) -> Result<(), String> {
    if password.is_empty() {
        return Err("Password required to change sensitivity".to_string());
    }

    let level = SensitivityLevel::parse_level(&new_level)
        .ok_or_else(|| format!("Invalid level: {}", new_level))?;

    let mut map = manager.map.write().await;
    let old = map.get(&field_id);

    // Downgrade protection: downgrading requires vault password verification
    if (level as i32) < (old as i32) {
        verify_vault_password(&app, &password)?;

        // Cooldown check (§5): same field cannot be downgraded again within 5 min
        const DOWNGRADE_COOLDOWN_SECS: i64 = 300;
        let log = manager.log.read().await;
        if let Some(last_downgrade) = log
            .entries
            .iter()
            .rev()
            .find(|e| e.field_id == field_id && (e.old_level as i32) > (e.new_level as i32))
        {
            if let Ok(last_ts) = chrono::DateTime::parse_from_rfc3339(&last_downgrade.timestamp) {
                let elapsed = chrono::Utc::now()
                    .signed_duration_since(last_ts.with_timezone(&chrono::Utc))
                    .num_seconds();
                if elapsed < DOWNGRADE_COOLDOWN_SECS {
                    let remaining = DOWNGRADE_COOLDOWN_SECS - elapsed;
                    return Err(format!(
                        "Downgrade cooldown active. Please wait {} more seconds before downgrading this field again.",
                        remaining
                    ));
                }
            }
        }
    }

    map.set(&field_id, level);
    let mut log = manager.log.write().await;
    log.push(&field_id, old, level, reason.unwrap_or_default());
    Ok(())
}

/// Verify the given password against stored account credentials.
/// Uses the same Argon2id derivation as vault unlock.
fn verify_vault_password(app: &AppState, password: &str) -> Result<(), String> {
    let svc = app.vault_service.blocking_read();
    let accounts = svc.list_accounts();

    if accounts.is_empty() {
        return Err("No accounts configured".to_string());
    }

    let main_config = KdfConfig::balanced();

    for account in &accounts {
        let stored_salt = account["salt"].as_str().unwrap_or("");
        let stored_hash = account["verifyHash"].as_str().unwrap_or("");

        // Decode salt from base64
        let salt_bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, stored_salt)
                .map_err(|_| "Invalid salt encoding".to_string())?;

        let salt_arr: [u8; 16] = salt_bytes
            .as_slice()
            .try_into()
            .map_err(|_| "Invalid salt length".to_string())?;

        // Derive master key from password + salt (same as unlock)
        let master_key = derive_key(password, &salt_arr, &main_config)
            .map_err(|_| "Key derivation failed".to_string())?;

        // Derive verification sub-key (same as unlock)
        let verify_data = b"SOLOSOUL_VAULT_VERIFY_v1";
        let verify_key = derive_key(
            &hex::encode(master_key.as_slice()),
            verify_data,
            &KdfConfig {
                memory_kb: 8192,
                iterations: 1,
                parallelism: 1,
            },
        )
        .map_err(|_| "Verify key derivation failed".to_string())?;

        let computed_hash = hex::encode(verify_key.as_slice());

        if computed_hash == stored_hash {
            return Ok(());
        }
    }

    Err("Invalid password".to_string())
}

#[tauri::command]
pub async fn sensitivity_get_log(
    manager: State<'_, SensitivityManager>,
    limit: Option<usize>,
) -> Result<Vec<SensitivityLogEntry>, String> {
    let log = manager.log.read().await;
    let entries: Vec<_> = log
        .entries
        .iter()
        .rev()
        .take(limit.unwrap_or(100))
        .cloned()
        .collect();
    Ok(entries)
}
