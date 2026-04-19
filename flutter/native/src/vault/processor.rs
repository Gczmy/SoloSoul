//! Vault processor - JSON-based request/response handler for FFI
//!
//! This module provides a type-safe JSON RPC interface for vault operations
//! that works around flutter_rust_bridge's complex type handling limitations.

use serde::{Deserialize, Serialize};
use crate::vault::{Profile, ProfileSummary};
use crate::account::AccountManager;

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
    match request.action.as_str() {
        "list_profiles" => handle_list_profiles(account_manager),
        "save_profile" => handle_save_profile(request.payload, account_manager),
        "create_profile" => handle_create_profile(request.payload, account_manager),
        "load_profile" => handle_load_profile(request.payload, account_manager),
        "delete_profile" => handle_delete_profile(request.payload, account_manager),
        "get_vault_stats" => handle_vault_stats(account_manager),
        "is_unlocked" => handle_is_unlocked(account_manager),
        "unlock_vault" => handle_unlock_vault(request.payload, account_manager),
        "lock_vault" => handle_lock_vault(account_manager),
        "create_account" => handle_create_account(request.payload, account_manager),
        "change_password" => handle_change_password(request.payload, account_manager),
        "get_account_config" => handle_get_account_config(request.payload, account_manager),
        "delete_account" => handle_delete_account(request.payload, account_manager),
        "search_profiles" => handle_search_profiles(request.payload, account_manager),
        "save_field_histories" => handle_save_field_histories(request.payload, account_manager),
        "load_field_histories" => handle_load_field_histories(request.payload, account_manager),
        "delete_field_histories" => handle_delete_field_histories(request.payload, account_manager),
        _ => VaultResponse::error(format!("Unknown action: {}", request.action)),
    }
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
            VaultResponse::success(serde_json::to_value(summaries).unwrap())
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

    VaultResponse::success(serde_json::to_value(JsonProfileSummary::from(summary)).unwrap())
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

    VaultResponse::success(serde_json::to_value(JsonProfileSummary::from(summary)).unwrap())
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
    let payload: UnlockVaultPayload = match payload {
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

fn handle_lock_vault(manager: &AccountManager) -> VaultResponse {
    manager.lock();
    VaultResponse::success(serde_json::json!({
        "success": true,
    }))
}

fn handle_create_account(payload: Option<serde_json::Value>, manager: &AccountManager) -> VaultResponse {
    let payload: CreateAccountPayload = match payload {
        Some(p) => match serde_json::from_value(p) {
            Ok(p) => p,
            Err(e) => return VaultResponse::error(format!("Invalid payload: {}", e)),
        },
        None => return VaultResponse::error("Missing payload".to_string()),
    };

    match manager.create_account(&payload.name, &payload.password) {
        Ok(info) => VaultResponse::success(serde_json::json!({
            "id": info.id,
            "name": info.name,
            "salt": info.salt,
            "verify_hash": info.verify_hash,
            "created": true
        })),
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
            VaultResponse::success(serde_json::to_value(summaries).unwrap())
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
}
