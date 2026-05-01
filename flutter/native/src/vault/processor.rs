//! Vault processor - JSON-based request/response handler for FFI
//!
//! This module provides a type-safe JSON RPC interface for vault operations
//! that works around flutter_rust_bridge's complex type handling limitations.

use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use crate::vault::{Profile, ProfileSummary};
use crate::account::AccountManager;

/// Write debug log to file (works in sandboxed environment)
fn log_to_file(msg: &str) {
    // Use home directory for macOS sandbox compatibility
    let log_path = if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join("Library/Logs/solosoul_debug.log")
    } else {
        PathBuf::from("/tmp/solosoul_debug.log")
    };
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
        let timestamp = chrono::Utc::now().to_rfc3339();
        let _ = writeln!(file, "[{}] {}", timestamp, msg);
    }
}

/// Request envelope for vault operations
#[derive(Debug, Deserialize)]
pub struct VaultRequest {
    pub action: String,
    #[serde(default)]
    pub payload: Option<serde_json::Value>,
}

/// Response envelope for vault operations
#[derive(Debug, Serialize)]
pub struct VaultResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl VaultResponse {
    pub fn success(data: serde_json::Value) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(msg: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg),
        }
    }
}

/// Payload for save_profile action
#[derive(Debug, Serialize, Deserialize)]
pub struct SaveProfilePayload {
    pub name: String,
    pub data: String, // Base64 encoded encrypted data
}

/// Payload for create_profile action
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateProfilePayload {
    pub name: String,
    pub data: String, // Base64 encoded encrypted data
}

/// Payload for unlock_vault action
#[derive(Debug, Serialize, Deserialize)]
pub struct UnlockVaultPayload {
    pub account_id: String,
    pub password: String,
}

/// Payload for unlock_vault_with_key action
#[derive(Debug, Serialize, Deserialize)]
pub struct UnlockVaultWithKeyPayload {
    pub account_id: String,
    pub session_key: String, // base64-encoded [u8; 32]
}

/// Payload for create_account action
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAccountPayload {
    pub account_id: String,
    pub name: String,
    pub password: String,
}

/// Payload for change_password action
#[derive(Debug, Serialize, Deserialize)]
pub struct ChangePasswordPayload {
    pub account_id: String,
    pub old_password: String,
    pub new_password: String,
}

/// Payload for encrypt_data / decrypt_data actions
#[derive(Debug, Serialize, Deserialize)]
pub struct CryptoPayload {
    pub data: String,  // Base64 encoded plaintext (encrypt) or ciphertext (decrypt)
}

/// Profile summary for JSON serialization
#[derive(Debug, Serialize)]
pub struct JsonProfileSummary {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: u32,
}

impl From<ProfileSummary> for JsonProfileSummary {
    fn from(p: ProfileSummary) -> Self {
        JsonProfileSummary {
            id: p.id,
            name: p.name,
            created_at: p.created_at.to_rfc3339(),
            updated_at: p.updated_at.to_rfc3339(),
            version: p.version,
        }
    }
}

/// Handle a vault request and return JSON response
pub fn handle_vault_request(
    request: VaultRequest,
    account_manager: &AccountManager,
) -> VaultResponse {
    log_to_file(&format!("[PROCESSOR] Received action: {}", request.action));
    match request.action.as_str() {
        "ping" => {
            log_to_file("[PROCESSOR] Ping received - FFI is working!");
            VaultResponse::success(serde_json::json!({"pong": true}))
        }
        "list_profiles" => handle_list_profiles(account_manager),
        "save_profile" => handle_save_profile(request.payload, account_manager),
        "create_profile" => handle_create_profile(request.payload, account_manager),
        "load_profile" => handle_load_profile(request.payload, account_manager),
        "delete_profile" => handle_delete_profile(request.payload, account_manager),
        "get_vault_stats" => handle_vault_stats(account_manager),
        "is_unlocked" => handle_is_unlocked(account_manager),
        "unlock_vault" => handle_unlock_vault(request.payload, account_manager),
        "unlock_vault_with_key" => handle_unlock_vault_with_key(request.payload, account_manager),
        "lock_vault" => handle_lock_vault(account_manager),
        "create_account" => handle_create_account(request.payload, account_manager),
        "change_password" => handle_change_password(request.payload, account_manager),
        "get_account_config" => handle_get_account_config(request.payload, account_manager),
        "delete_account" => handle_delete_account(request.payload, account_manager),
        "search_profiles" => handle_search_profiles(request.payload, account_manager),
        "save_field_histories" => handle_save_field_histories(request.payload, account_manager),
        "load_field_histories" => handle_load_field_histories(request.payload, account_manager),
        "delete_field_histories" => handle_delete_field_histories(request.payload, account_manager),
        "save_setting" => handle_save_setting(request.payload, account_manager),
        "load_setting" => handle_load_setting(request.payload, account_manager),
        "delete_setting" => handle_delete_setting(request.payload, account_manager),
        "list_accounts" => handle_list_accounts(account_manager),
        "encrypt_data" => handle_encrypt_data(request.payload, account_manager),
        "decrypt_data" => handle_decrypt_data(request.payload, account_manager),
        "verify_password" => handle_verify_password(request.payload, account_manager),
        "migrate_encryption" => handle_migrate_encryption(request.payload, account_manager),
        "update_account_metadata" => handle_update_account_metadata(request.payload, account_manager),
        _ => VaultResponse::error(format!("Unknown action: {}", request.action)),
    }
}

/// Handle list_accounts action - returns all accounts from accounts.json
fn handle_list_accounts(manager: &AccountManager) -> VaultResponse {
    let accounts = manager.list_accounts();
    let account_summaries: Vec<serde_json::Value> = accounts
        .into_iter()
        .map(|a| serde_json::json!({
            "id": a.id,
            "name": a.name,
            "last_accessed": a.last_accessed.map(|dt: chrono::DateTime<chrono::Utc>| dt.to_rfc3339()),
        }))
        .collect();
    VaultResponse::success(serde_json::json!({
        "accounts": account_summaries,
    }))
}

fn handle_list_profiles(manager: &AccountManager) -> VaultResponse {
    let vault_guard = match manager.get_vault_store() {
        Some(g) => g,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    let vault = match vault_guard.as_ref() {
        Some(v) => v,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    match vault.list_profiles() {
        Ok(profiles) => {
            let summaries: Vec<JsonProfileSummary> = profiles
                .into_iter()
                .map(JsonProfileSummary::from)
                .collect();
            match serde_json::to_value(summaries) {
                Ok(json) => VaultResponse::success(json),
                Err(e) => VaultResponse::error(format!("Failed to serialize profiles: {}", e)),
            }
        }
        Err(e) => VaultResponse::error(format!("Failed to list profiles: {}", e)),
    }
}

fn handle_save_profile(payload: Option<serde_json::Value>, manager: &AccountManager) -> VaultResponse {
    let vault_guard = match manager.get_vault_store() {
        Some(g) => g,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    let vault = match vault_guard.as_ref() {
        Some(v) => v,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    let payload: SaveProfilePayload = match payload {
        Some(p) => match serde_json::from_value(p) {
            Ok(p) => p,
            Err(e) => return VaultResponse::error(format!("Invalid payload: {}", e)),
        },
        None => return VaultResponse::error("Missing payload".to_string()),
    };

    // Decode base64 data
    let data = match base64_decode(&payload.data) {
        Ok(d) => d,
        Err(e) => return VaultResponse::error(format!("Failed to decode data: {}", e)),
    };

    // Check if profile exists by name
    let existing = vault.list_profiles()
        .ok()
        .and_then(|profiles| profiles.into_iter().find(|p| p.name == payload.name));

    // Delete any existing profile with a different ID (legacy data with random UUID)
    if let Some(ref existing_profile) = existing {
        if existing_profile.id != payload.name {
            // Delete the old profile with wrong ID
            let _ = vault.delete_profile(&existing_profile.id);
        }
    }

    let profile = if existing.is_some() {
        // Use the existing profile's created_at since we're updating
        Profile {
            id: payload.name.clone(),  // Use name as ID for consistent lookups
            name: payload.name.clone(),
            data,
            created_at: existing.as_ref().unwrap().created_at,
            updated_at: chrono::Utc::now(),
            version: existing.as_ref().unwrap().version + 1,
        }
    } else {
        // Use profile name as ID so Flutter can look it up by accountId
        Profile::new_with_id(&payload.name, &payload.name, data)
    };

    let summary = ProfileSummary::from_profile(&profile);
    if let Err(e) = vault.save_profile(&profile) {
        return VaultResponse::error(format!("Failed to save profile: {}", e));
    }

    match serde_json::to_value(JsonProfileSummary::from(summary)) {
        Ok(json) => VaultResponse::success(json),
        Err(e) => VaultResponse::error(format!("Failed to serialize profile: {}", e)),
    }
}

fn handle_create_profile(payload: Option<serde_json::Value>, manager: &AccountManager) -> VaultResponse {
    let vault_guard = match manager.get_vault_store() {
        Some(g) => g,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    let vault = match vault_guard.as_ref() {
        Some(v) => v,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    let payload: CreateProfilePayload = match payload {
        Some(p) => match serde_json::from_value(p) {
            Ok(p) => p,
            Err(e) => return VaultResponse::error(format!("Invalid payload: {}", e)),
        },
        None => return VaultResponse::error("Missing payload".to_string()),
    };

    // Decode base64 data
    let data = match base64_decode(&payload.data) {
        Ok(d) => d,
        Err(e) => return VaultResponse::error(format!("Failed to decode data: {}", e)),
    };

    let profile = Profile::new_with_id(&payload.name, &payload.name, data);
    let summary = ProfileSummary::from_profile(&profile);

    if let Err(e) = vault.save_profile(&profile) {
        return VaultResponse::error(format!("Failed to create profile: {}", e));
    }

    match serde_json::to_value(JsonProfileSummary::from(summary)) {
        Ok(json) => VaultResponse::success(json),
        Err(e) => VaultResponse::error(format!("Failed to serialize profile: {}", e)),
    }
}

fn handle_load_profile(payload: Option<serde_json::Value>, manager: &AccountManager) -> VaultResponse {
    let vault_guard = match manager.get_vault_store() {
        Some(g) => g,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    let vault = match vault_guard.as_ref() {
        Some(v) => v,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    let id = match payload {
        Some(p) => {
            match p.get("id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return VaultResponse::error("Missing profile id".to_string()),
            }
        }
        None => return VaultResponse::error("Missing payload".to_string()),
    };

    match vault.load_profile(&id) {
        Ok(Some(profile)) => {
            let data_b64 = base64_encode(&profile.data);
            VaultResponse::success(serde_json::json!({
                "id": profile.id,
                "name": profile.name,
                "data": data_b64,
                "version": profile.version,
            }))
        }
        Ok(None) => VaultResponse::error("Profile not found".to_string()),
        Err(e) => VaultResponse::error(format!("Failed to load profile: {}", e)),
    }
}

fn handle_delete_profile(payload: Option<serde_json::Value>, manager: &AccountManager) -> VaultResponse {
    let vault_guard = match manager.get_vault_store() {
        Some(g) => g,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    let vault = match vault_guard.as_ref() {
        Some(v) => v,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    let id = match payload {
        Some(p) => {
            match p.get("id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return VaultResponse::error("Missing profile id".to_string()),
            }
        }
        None => return VaultResponse::error("Missing payload".to_string()),
    };

    match vault.delete_profile(&id) {
        Ok(()) => VaultResponse::success(serde_json::json!({"deleted": true})),
        Err(e) => VaultResponse::error(format!("Failed to delete profile: {}", e)),
    }
}

fn handle_vault_stats(manager: &AccountManager) -> VaultResponse {
    let vault_guard = match manager.get_vault_store() {
        Some(g) => g,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    let vault = match vault_guard.as_ref() {
        Some(v) => v,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    match vault.stats() {
        Ok(stats) => VaultResponse::success(serde_json::json!({
            "profile_count": stats.profile_count,
            "total_size_bytes": stats.total_size_bytes,
            "last_modified": stats.last_modified,
        })),
        Err(e) => VaultResponse::error(format!("Failed to get vault stats: {}", e)),
    }
}

fn handle_is_unlocked(manager: &AccountManager) -> VaultResponse {
    VaultResponse::success(serde_json::json!({
        "is_unlocked": manager.is_unlocked(),
    }))
}

fn handle_unlock_vault(payload: Option<serde_json::Value>, manager: &AccountManager) -> VaultResponse {
    log_to_file("[PROCESSOR] handle_unlock_vault called");
    let payload: UnlockVaultPayload = match payload {
        Some(p) => match serde_json::from_value(p) {
            Ok(p) => p,
            Err(e) => {
                log_to_file(&format!("[PROCESSOR] Payload parse error: {}", e));
                return VaultResponse::error(format!("Invalid payload: {}", e));
            }
        },
        None => {
            log_to_file("[PROCESSOR] Missing payload");
            return VaultResponse::error("Missing payload".to_string());
        }
    };

    log_to_file(&format!("[PROCESSOR] Calling manager.unlock for account_id: {}", payload.account_id));
    let result = manager.unlock(&payload.account_id, &payload.password);
    log_to_file(&format!("[PROCESSOR] manager.unlock returned, success={}", result.success));

    VaultResponse::success(serde_json::json!({
        "success": result.success,
        "error": result.error,
        "crypto_version": result.crypto_version,
    }))
}

fn handle_unlock_vault_with_key(payload: Option<serde_json::Value>, manager: &AccountManager) -> VaultResponse {
    log_to_file("[PROCESSOR] handle_unlock_vault_with_key called");
    let payload: UnlockVaultWithKeyPayload = match payload {
        Some(p) => match serde_json::from_value(p) {
            Ok(p) => p,
            Err(e) => {
                log_to_file(&format!("[PROCESSOR] Payload parse error: {}", e));
                return VaultResponse::error(format!("Invalid payload: {}", e));
            }
        },
        None => {
            log_to_file("[PROCESSOR] Missing payload");
            return VaultResponse::error("Missing payload".to_string());
        }
    };

    log_to_file(&format!("[PROCESSOR] Calling manager.unlock_with_key for account_id: {}", payload.account_id));
    let result = manager.unlock_with_key(&payload.account_id, &payload.session_key);
    log_to_file(&format!("[PROCESSOR] manager.unlock_with_key returned, success={}", result.success));

    VaultResponse::success(serde_json::json!({
        "success": result.success,
        "error": result.error,
        "crypto_version": result.crypto_version,
    }))
}

fn handle_lock_vault(manager: &AccountManager) -> VaultResponse {
    manager.lock();
    VaultResponse::success(serde_json::json!({
        "success": true,
    }))
}

fn handle_create_account(payload: Option<serde_json::Value>, manager: &AccountManager) -> VaultResponse {
    log_to_file("[PROCESSOR] handle_create_account called");
    let payload: CreateAccountPayload = match payload {
        Some(p) => match serde_json::from_value(p) {
            Ok(p) => p,
            Err(e) => return VaultResponse::error(format!("Invalid payload: {}", e)),
        },
        None => return VaultResponse::error("Missing payload".to_string()),
    };
    log_to_file(&format!("[PROCESSOR] create_account payload: name={}", payload.name));

    log_to_file("[PROCESSOR] Calling manager.create_account...");
    match manager.create_account(&payload.name, &payload.password) {
        Ok(info) => {
            log_to_file(&format!("[PROCESSOR] manager.create_account success: id={}", info.id));
            VaultResponse::success(serde_json::json!({
                "id": info.id,
                "name": info.name,
                "salt": info.salt,
                "verify_hash": info.verify_hash,
                "created": true
            }))
        },
        Err(e) => VaultResponse::error(format!("Failed to create account: {}", e)),
    }
}

fn handle_change_password(payload: Option<serde_json::Value>, manager: &AccountManager) -> VaultResponse {
    let payload: ChangePasswordPayload = match payload {
        Some(p) => match serde_json::from_value(p) {
            Ok(p) => p,
            Err(e) => return VaultResponse::error(format!("Invalid payload: {}", e)),
        },
        None => return VaultResponse::error("Missing payload".to_string()),
    };

    match manager.change_password(&payload.account_id, &payload.old_password, &payload.new_password) {
        Ok(info) => VaultResponse::success(serde_json::json!({
            "id": info.id,
            "name": info.name,
            "salt": info.salt,
            "verify_hash": info.verify_hash,
            "crypto_version": info.crypto_version,
        })),
        Err(e) => VaultResponse::error(format!("Failed to change password: {}", e)),
    }
}

fn handle_get_account_config(payload: Option<serde_json::Value>, manager: &AccountManager) -> VaultResponse {
    let account_id = match payload {
        Some(p) => {
            match p.get("account_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return VaultResponse::error("Missing account_id".to_string()),
            }
        }
        None => return VaultResponse::error("Missing payload".to_string()),
    };

    match manager.get_account_config(&account_id) {
        Some(info) => VaultResponse::success(serde_json::json!({
            "id": info.id,
            "name": info.name,
            "salt": info.salt,
            "verify_hash": info.verify_hash,
            "crypto_version": info.crypto_version,
            "password_hint": info.password_hint,
            "last_login_at": info.last_login_at.map(|d| d.to_rfc3339()),
            "last_operation_at": info.last_operation_at.map(|d| d.to_rfc3339()),
            "last_operation_desc": info.last_operation_desc,
            "recent_devices": info.recent_devices.iter().map(|d| serde_json::json!({
                "device_name": d.device_name,
                "last_used": d.last_used.to_rfc3339(),
            })).collect::<Vec<_>>(),
            "biometric_enabled": info.biometric_enabled,
        })),
        None => VaultResponse::error("Account not found".to_string()),
    }
}

fn handle_delete_account(payload: Option<serde_json::Value>, manager: &AccountManager) -> VaultResponse {
    let account_id = match payload {
        Some(p) => {
            match p.get("account_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return VaultResponse::error("Missing account_id".to_string()),
            }
        }
        None => return VaultResponse::error("Missing payload".to_string()),
    };

    match manager.delete_account(&account_id) {
        Ok(()) => VaultResponse::success(serde_json::json!({"deleted": true})),
        Err(e) => VaultResponse::error(format!("Failed to delete account: {}", e)),
    }
}

fn handle_search_profiles(payload: Option<serde_json::Value>, manager: &AccountManager) -> VaultResponse {
    let vault_guard = match manager.get_vault_store() {
        Some(g) => g,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    let vault = match vault_guard.as_ref() {
        Some(v) => v,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    let query = match payload {
        Some(p) => {
            match p.get("query").and_then(|v| v.as_str()) {
                Some(q) => q.to_lowercase(),
                None => return VaultResponse::error("Missing query".to_string()),
            }
        }
        None => return VaultResponse::error("Missing payload".to_string()),
    };

    if query.len() < 2 {
        return VaultResponse::success(serde_json::json!([]));
    }

    // Search by name (case-insensitive LIKE)
    match vault.search_profiles(&query) {
        Ok(results) => {
            let summaries: Vec<JsonProfileSummary> = results
                .into_iter()
                .map(JsonProfileSummary::from)
                .collect();
            match serde_json::to_value(summaries) {
                Ok(json) => VaultResponse::success(json),
                Err(e) => VaultResponse::error(format!("Failed to serialize search results: {}", e)),
            }
        }
        Err(e) => VaultResponse::error(format!("Search failed: {}", e)),
    }
}

fn handle_save_field_histories(payload: Option<serde_json::Value>, manager: &AccountManager) -> VaultResponse {
    let (account_id, data_b64) = match payload {
        Some(p) => {
            let account_id = match p.get("account_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return VaultResponse::error("Missing account_id".to_string()),
            };
            let data_b64 = match p.get("data").and_then(|v| v.as_str()) {
                Some(d) => d.to_string(),
                None => return VaultResponse::error("Missing data".to_string()),
            };
            (account_id, data_b64)
        }
        None => return VaultResponse::error("Missing payload".to_string()),
    };

    let vault_guard = match manager.get_vault_store() {
        Some(g) => g,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    let vault = match vault_guard.as_ref() {
        Some(v) => v,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    let data = match base64_decode(&data_b64) {
        Ok(d) => d,
        Err(e) => return VaultResponse::error(format!("Failed to decode data: {}", e)),
    };

    match vault.save_field_histories(&account_id, &data) {
        Ok(()) => VaultResponse::success(serde_json::json!({"saved": true})),
        Err(e) => VaultResponse::error(format!("Failed to save field histories: {}", e)),
    }
}

fn handle_load_field_histories(payload: Option<serde_json::Value>, manager: &AccountManager) -> VaultResponse {
    let account_id = match payload {
        Some(p) => {
            match p.get("account_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return VaultResponse::error("Missing account_id".to_string()),
            }
        }
        None => return VaultResponse::error("Missing payload".to_string()),
    };

    let vault_guard = match manager.get_vault_store() {
        Some(g) => g,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    let vault = match vault_guard.as_ref() {
        Some(v) => v,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    match vault.load_field_histories(&account_id) {
        Ok(Some(data)) => {
            let encoded = base64_encode(&data);
            VaultResponse::success(serde_json::json!({"data": encoded}))
        }
        Ok(None) => VaultResponse::success(serde_json::json!({"data": serde_json::Value::Null})),
        Err(e) => VaultResponse::error(format!("Failed to load field histories: {}", e)),
    }
}

fn handle_delete_field_histories(payload: Option<serde_json::Value>, manager: &AccountManager) -> VaultResponse {
    let account_id = match payload {
        Some(p) => {
            match p.get("account_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return VaultResponse::error("Missing account_id".to_string()),
            }
        }
        None => return VaultResponse::error("Missing payload".to_string()),
    };

    let vault_guard = match manager.get_vault_store() {
        Some(g) => g,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    let vault = match vault_guard.as_ref() {
        Some(v) => v,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    match vault.delete_field_histories(&account_id) {
        Ok(()) => VaultResponse::success(serde_json::json!({"deleted": true})),
        Err(e) => VaultResponse::error(format!("Failed to delete field histories: {}", e)),
    }
}

fn handle_save_setting(payload: Option<serde_json::Value>, manager: &AccountManager) -> VaultResponse {
    let (account_id, data_b64) = match payload {
        Some(p) => {
            let account_id = match p.get("account_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return VaultResponse::error("Missing account_id".to_string()),
            };
            let data_b64 = match p.get("data").and_then(|v| v.as_str()) {
                Some(d) => d.to_string(),
                None => return VaultResponse::error("Missing data".to_string()),
            };
            (account_id, data_b64)
        }
        None => return VaultResponse::error("Missing payload".to_string()),
    };

    let vault_guard = match manager.get_vault_store() {
        Some(g) => g,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    let vault = match vault_guard.as_ref() {
        Some(v) => v,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    let data = match base64_decode(&data_b64) {
        Ok(d) => d,
        Err(e) => return VaultResponse::error(format!("Failed to decode data: {}", e)),
    };

    match vault.save_setting(&account_id, &data) {
        Ok(()) => VaultResponse::success(serde_json::json!({"saved": true})),
        Err(e) => VaultResponse::error(format!("Failed to save setting: {}", e)),
    }
}

fn handle_load_setting(payload: Option<serde_json::Value>, manager: &AccountManager) -> VaultResponse {
    let account_id = match payload {
        Some(p) => {
            match p.get("account_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return VaultResponse::error("Missing account_id".to_string()),
            }
        }
        None => return VaultResponse::error("Missing payload".to_string()),
    };

    let vault_guard = match manager.get_vault_store() {
        Some(g) => g,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    let vault = match vault_guard.as_ref() {
        Some(v) => v,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    match vault.load_setting(&account_id) {
        Ok(Some(data)) => {
            let encoded = base64_encode(&data);
            VaultResponse::success(serde_json::json!({"data": encoded}))
        }
        Ok(None) => VaultResponse::success(serde_json::json!({"data": serde_json::Value::Null})),
        Err(e) => VaultResponse::error(format!("Failed to load setting: {}", e)),
    }
}

fn handle_delete_setting(payload: Option<serde_json::Value>, manager: &AccountManager) -> VaultResponse {
    let account_id = match payload {
        Some(p) => {
            match p.get("account_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return VaultResponse::error("Missing account_id".to_string()),
            }
        }
        None => return VaultResponse::error("Missing payload".to_string()),
    };

    let vault_guard = match manager.get_vault_store() {
        Some(g) => g,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    let vault = match vault_guard.as_ref() {
        Some(v) => v,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    match vault.delete_setting(&account_id) {
        Ok(()) => VaultResponse::success(serde_json::json!({"deleted": true})),
        Err(e) => VaultResponse::error(format!("Failed to delete setting: {}", e)),
    }
}

// =============================================================================
// Phase 1: Unified encryption layer — encrypt/decrypt via Rust
// =============================================================================

fn handle_encrypt_data(payload: Option<serde_json::Value>, manager: &AccountManager) -> VaultResponse {
    let session_key = match manager.get_session_key() {
        Some(k) => k,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    let payload: CryptoPayload = match payload {
        Some(p) => match serde_json::from_value(p) {
            Ok(p) => p,
            Err(e) => return VaultResponse::error(format!("Invalid payload: {}", e)),
        },
        None => return VaultResponse::error("Missing payload".to_string()),
    };

    let plaintext = match base64_decode(&payload.data) {
        Ok(d) => d,
        Err(e) => return VaultResponse::error(format!("Failed to decode data: {}", e)),
    };

    let key: [u8; 32] = match session_key.as_slice().try_into() {
        Ok(k) => k,
        Err(_) => return VaultResponse::error("Invalid session key length".to_string()),
    };

    match crate::crypto::encrypt_profile_data(&key, &plaintext) {
        Ok(blob) => VaultResponse::success(serde_json::json!({
            "data": base64_encode(&blob),
        })),
        Err(e) => VaultResponse::error(format!("Encryption failed: {}", e)),
    }
}

fn handle_decrypt_data(payload: Option<serde_json::Value>, manager: &AccountManager) -> VaultResponse {
    let session_key = match manager.get_session_key() {
        Some(k) => k,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    let payload: CryptoPayload = match payload {
        Some(p) => match serde_json::from_value(p) {
            Ok(p) => p,
            Err(e) => return VaultResponse::error(format!("Invalid payload: {}", e)),
        },
        None => return VaultResponse::error("Missing payload".to_string()),
    };

    let ciphertext = match base64_decode(&payload.data) {
        Ok(d) => d,
        Err(e) => return VaultResponse::error(format!("Failed to decode data: {}", e)),
    };

    let key: [u8; 32] = match session_key.as_slice().try_into() {
        Ok(k) => k,
        Err(_) => return VaultResponse::error("Invalid session key length".to_string()),
    };

    match crate::crypto::decrypt_profile_data(&key, &ciphertext) {
        Ok(plaintext) => VaultResponse::success(serde_json::json!({
            "data": base64_encode(&plaintext),
        })),
        Err(e) => VaultResponse::error(format!("Decryption failed: {}", e)),
    }
}

fn handle_verify_password(payload: Option<serde_json::Value>, manager: &AccountManager) -> VaultResponse {
    #[derive(Deserialize)]
    struct VerifyPayload {
        account_id: String,
        password: String,
    }

    let payload: VerifyPayload = match payload {
        Some(p) => match serde_json::from_value(p) {
            Ok(p) => p,
            Err(e) => return VaultResponse::error(format!("Invalid payload: {}", e)),
        },
        None => return VaultResponse::error("Missing payload".to_string()),
    };

    let result = manager.unlock(&payload.account_id, &payload.password);
    VaultResponse::success(serde_json::json!({
        "success": result.success,
        "error": result.error,
        "crypto_version": result.crypto_version,
    }))
}

fn handle_migrate_encryption(payload: Option<serde_json::Value>, manager: &AccountManager) -> VaultResponse {
    let session_key = match manager.get_session_key() {
        Some(k) => k,
        None => return VaultResponse::error("Vault not unlocked".to_string()),
    };

    let payload: CryptoPayload = match payload {
        Some(p) => match serde_json::from_value(p) {
            Ok(p) => p,
            Err(e) => return VaultResponse::error(format!("Invalid payload: {}", e)),
        },
        None => return VaultResponse::error("Missing payload".to_string()),
    };

    let legacy_data = match base64_decode(&payload.data) {
        Ok(d) => d,
        Err(e) => return VaultResponse::error(format!("Failed to decode data: {}", e)),
    };

    let key: [u8; 32] = match session_key.as_slice().try_into() {
        Ok(k) => k,
        Err(_) => return VaultResponse::error("Invalid session key length".to_string()),
    };

    // Check if already SOLO format — return as-is
    if legacy_data.len() >= 33 && &legacy_data[0..4] == b"SOLO" {
        return VaultResponse::success(serde_json::json!({
            "data": base64_encode(&legacy_data),
            "migrated": false,
        }));
    }

    match crate::crypto::migrate_to_solo_format(&key, &legacy_data) {
        Ok(solo_blob) => VaultResponse::success(serde_json::json!({
            "data": base64_encode(&solo_blob),
            "migrated": true,
        })),
        Err(e) => VaultResponse::error(format!("Migration failed: {}", e)),
    }
}

fn handle_update_account_metadata(payload: Option<serde_json::Value>, manager: &AccountManager) -> VaultResponse {
    #[derive(Deserialize)]
    struct MetadataPayload {
        account_id: String,
        #[serde(default)]
        password_hint: Option<String>,
        #[serde(default)]
        last_login_at: Option<String>,
        #[serde(default)]
        last_operation_at: Option<String>,
        #[serde(default)]
        last_operation_desc: Option<String>,
        #[serde(default)]
        recent_devices: Option<Vec<serde_json::Value>>,
        /// Append a single device to the list (upsert by device_name)
        #[serde(default)]
        add_device: Option<serde_json::Value>,
        #[serde(default)]
        biometric_enabled: Option<bool>,
    }

    let payload: MetadataPayload = match payload {
        Some(p) => match serde_json::from_value(p) {
            Ok(p) => p,
            Err(e) => return VaultResponse::error(format!("Invalid payload: {}", e)),
        },
        None => return VaultResponse::error("Missing payload".to_string()),
    };

    let last_login_at = payload.last_login_at.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&chrono::Utc)));
    let last_operation_at = payload.last_operation_at.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&chrono::Utc)));

    // Handle add_device (append/upsert) vs recent_devices (overwrite)
    let recent_devices = if let Some(device) = payload.add_device {
        // Read current config to get existing devices
        let config_path = manager.base_path().join(&payload.account_id).join("config.json");
        let existing_devices = if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(config) = serde_json::from_str::<crate::account::AccountConfig>(&content) {
                config.recent_devices
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let device_name = device.get("device_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let last_used = device.get("last_used").and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&chrono::Utc))
            .unwrap_or_else(chrono::Utc::now);

        let mut devices = existing_devices;
        if let Some(idx) = devices.iter().position(|d| d.device_name == device_name) {
            devices[idx] = crate::account::DeviceEntry { device_name, last_used };
        } else {
            if devices.len() >= 5 {
                devices.remove(0);
            }
            devices.push(crate::account::DeviceEntry { device_name, last_used });
        }
        Some(devices)
    } else {
        payload.recent_devices.map(|devices| {
            devices.into_iter().filter_map(|d| {
                Some(crate::account::DeviceEntry {
                    device_name: d.get("device_name")?.as_str()?.to_string(),
                    last_used: chrono::DateTime::parse_from_rfc3339(d.get("last_used")?.as_str()?)
                        .ok()?.with_timezone(&chrono::Utc),
                })
            }).collect::<Vec<_>>()
        })
    };

    match manager.update_account_metadata(
        &payload.account_id,
        payload.password_hint,
        last_login_at,
        last_operation_at,
        payload.last_operation_desc,
        recent_devices,
        payload.biometric_enabled,
    ) {
        Ok(()) => VaultResponse::success(serde_json::json!({"updated": true})),
        Err(e) => VaultResponse::error(format!("Failed to update metadata: {}", e)),
    }
}

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

// Base64 encoding using standard alphabet
fn base64_encode(data: &[u8]) -> String {
    BASE64.encode(data)
}

/// Base64 decoding
fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    BASE64.decode(input).map_err(|e| format!("Base64 decode error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_encode_decode() {
        let original = b"Hello, World!";
        let encoded = base64_encode(original);
        assert_eq!(encoded, "SGVsbG8sIFdvcmxkIQ==");
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_vault_response_success() {
        let response = VaultResponse::success(serde_json::json!({"test": true}));
        assert!(response.success);
        assert!(response.data.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_vault_response_error() {
        let response = VaultResponse::error("Test error".to_string());
        assert!(!response.success);
        assert!(response.data.is_none());
        assert_eq!(response.error, Some("Test error".to_string()));
    }

    #[test]
    fn test_vault_request_deserialize() {
        let json = r#"{"action": "list_profiles"}"#;
        let request: VaultRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.action, "list_profiles");
        assert!(request.payload.is_none());
    }

    #[test]
    fn test_vault_request_with_payload() {
        let json = r#"{"action": "save_profile", "payload": {"name": "test", "data": "SGVsbG8="}}"#;
        let request: VaultRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.action, "save_profile");
        assert!(request.payload.is_some());
    }

    #[test]
    fn test_vault_request_with_complex_payload() {
        let json = r#"{
            "action": "create_account",
            "payload": {
                "account_id": "acc_123",
                "name": "Test Account",
                "password": "securepassword123"
            }
        }"#;
        let request: VaultRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.action, "create_account");
        assert!(request.payload.is_some());

        let payload = request.payload.unwrap();
        assert_eq!(payload.get("name").unwrap().as_str().unwrap(), "Test Account");
    }

    #[test]
    fn test_vault_response_serialization() {
        let response = VaultResponse::success(serde_json::json!({
            "id": "acc_123",
            "name": "Test"
        }));
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("acc_123"));
    }

    #[test]
    fn test_vault_response_error_serialization() {
        let response = VaultResponse::error("Something went wrong".to_string());
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("Something went wrong"));
    }

    #[test]
    fn test_vault_request_unknown_action() {
        use tempfile::TempDir;
        use crate::account::AccountManager;

        let temp_dir = TempDir::new().unwrap();
        let manager = AccountManager::new(temp_dir.path().to_path_buf());

        let request = handle_vault_request(
            VaultRequest {
                action: "unknown_action".to_string(),
                payload: None,
            },
            &manager,
        );
        // Unknown action returns error
        assert!(!request.success);
        assert!(request.error.is_some());
    }

    #[test]
    fn test_save_profile_payload_deserialize() {
        let json = r#"{"name": "profile1", "data": "SGVsbG8sIFdvcmxkIQ=="}"#;
        let payload: SaveProfilePayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.name, "profile1");
        assert_eq!(payload.data, "SGVsbG8sIFdvcmxkIQ==");
    }

    #[test]
    fn test_create_profile_payload_deserialize() {
        let json = r#"{"name": "new_profile", "data": "dGVzdCBkYXRh"}"#;
        let payload: CreateProfilePayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.name, "new_profile");
        assert_eq!(payload.data, "dGVzdCBkYXRh");
    }

    #[test]
    fn test_unlock_vault_payload_deserialize() {
        let json = r#"{"account_id": "acc_abc123", "password": "mypassword"}"#;
        let payload: UnlockVaultPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.account_id, "acc_abc123");
        assert_eq!(payload.password, "mypassword");
    }

    #[test]
    fn test_change_password_payload_deserialize() {
        let json = r#"{
            "account_id": "acc_123",
            "old_password": "oldpass",
            "new_password": "newpass123"
        }"#;
        let payload: ChangePasswordPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.account_id, "acc_123");
        assert_eq!(payload.old_password, "oldpass");
        assert_eq!(payload.new_password, "newpass123");
    }

    #[test]
    fn test_json_profile_summary_from_profile_summary() {
        use crate::vault::ProfileSummary;
        use chrono::Utc;

        let summary = ProfileSummary {
            id: "test_id".to_string(),
            name: "Test Profile".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: 1,
        };

        let json_summary: JsonProfileSummary = summary.into();
        assert_eq!(json_summary.id, "test_id");
        assert_eq!(json_summary.name, "Test Profile");
        assert_eq!(json_summary.version, 1);
    }

    #[test]
    fn test_json_profile_summary_serialization() {
        let summary = JsonProfileSummary {
            id: "acc_123".to_string(),
            name: "Profile Name".to_string(),
            created_at: "2024-01-01T00:00:00+00:00".to_string(),
            updated_at: "2024-01-02T00:00:00+00:00".to_string(),
            version: 2,
        };

        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("acc_123"));
        assert!(json.contains("Profile Name"));
        assert!(json.contains("\"version\":2"));
    }

    #[test]
    fn test_base64_decode_invalid_input() {
        let result = base64_decode("not-valid-base64!!!");
        assert!(result.is_err());
    }

    #[test]
    fn test_base64_encode_decodes_to_original() {
        let original = vec![0x00, 0xFF, 0x42, 0x13, 0x37];
        let encoded = base64_encode(&original);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_handle_vault_request_invalid_json() {
        let json = r#"{"action": "save_profile", "payload": {"name":}}"#;
        let request: Result<VaultRequest, _> = serde_json::from_str(json);
        assert!(request.is_err());
    }

    #[test]
    fn test_vault_request_empty_action() {
        let json = r#"{"action": "", "payload": null}"#;
        let request: VaultRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.action, "");
        assert!(request.payload.is_none());
    }
}
