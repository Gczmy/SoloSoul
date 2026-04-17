mod frb_generated; /* AUTO INJECTED BY flutter_rust_bridge. This line may not be accurate, and you can change it according to your needs. */
// SoloSoul Core - Rust Core Library for Flutter
//
// This library provides:
// - High-performance Argon2id key derivation
// - AES-256-GCM encryption/decryption
// - Vault storage with rusqlite + SQLCipher (双重加密)
// - E2EE cloud sync engine
// - Wasm plugin sandbox

pub mod crypto;
pub mod vault;
pub mod sync;
pub mod plugin;
pub mod account;

use thiserror::Error;
use flutter_rust_bridge::frb;
use serde::{Deserialize, Serialize};
use std::ffi::CString;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("Vault error: {0}")]
    Vault(String),

    #[error("Sync error: {0}")]
    Sync(String),

    #[error("Plugin error: {0}")]
    Plugin(String),

    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
}

/// Result type for Core operations
pub type CoreResult<T> = Result<T, CoreError>;

// ============================================================================
// Global account manager singleton
// ============================================================================

lazy_static::lazy_static! {
    static ref ACCOUNT_MANAGER: Mutex<Option<account::AccountManager>> = Mutex::new(None);
}

fn get_account_manager() -> Result<std::sync::MutexGuard<'static, Option<account::AccountManager>>, String> {
    ACCOUNT_MANAGER.lock().map_err(|e| format!("Lock poisoned: {}", e))
}

fn init_account_manager(base_path: PathBuf) -> Result<(), String> {
    let mut guard = get_account_manager()?;
    if guard.is_none() {
        *guard = Some(account::AccountManager::new(base_path));
    }
    Ok(())
}

// ============================================================================
// Flutter Rust Bridge API
// ============================================================================

/// Vault statistics for FFI bridge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FfiVaultStats {
    pub profile_count: usize,
    pub total_size_bytes: u64,
    pub last_modified: Option<String>,
}

/// Bridge-specific account info with String dates for Dart
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeAccountInfo {
    pub id: String,
    pub name: String,
    pub last_accessed: Option<String>,
}

/// Bridge-specific profile summary with String dates for Dart
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeProfileSummary {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: u32,
}

/// Unlock result for Flutter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockResult {
    pub success: bool,
    pub error: Option<String>,
}

/// Initialize the account manager with base path
// // #[frb] // disabled // Temporarily disabled - complex async types cause codegen issues
pub async fn init_account_manager_async(base_path: String) -> bool {
    let path = PathBuf::from(base_path);
    init_account_manager(path).is_ok()
}

/// Initialize vault with master password
// #[frb] // disabled
pub async fn init_vault(account_name: String, password: String, base_path: String) -> UnlockResult {
    let path = PathBuf::from(&base_path);
    if let Err(e) = init_account_manager(path) {
        return UnlockResult {
            success: false,
            error: Some(e),
        };
    }

    let manager = get_account_manager().map_err(|e| UnlockResult {
        success: false,
        error: Some(e),
    }).unwrap();

    let manager = manager.as_ref().unwrap();

    match manager.create_account(&account_name, &password) {
        Ok(_) => UnlockResult {
            success: true,
            error: None,
        },
        Err(e) => UnlockResult {
            success: false,
            error: Some(e),
        },
    }
}

/// Unlock vault with master password
// #[frb] // disabled
pub async fn unlock_vault(account_id: String, password: String, base_path: String) -> UnlockResult {
    let path = PathBuf::from(&base_path);
    if let Err(e) = init_account_manager(path) {
        return UnlockResult {
            success: false,
            error: Some(e),
        };
    }

    let manager = get_account_manager().map_err(|e| UnlockResult {
        success: false,
        error: Some(e),
    }).unwrap();

    let manager = manager.as_ref().unwrap();
    let result = manager.unlock(&account_id, &password);
    UnlockResult {
        success: result.success,
        error: result.error,
    }
}

/// Lock the vault
// #[frb] // disabled
pub async fn lock_vault() -> bool {
    if let Ok(guard) = get_account_manager() {
        if let Some(manager) = guard.as_ref() {
            manager.lock();
            return true;
        }
    }
    false
}

/// Check if vault is unlocked
// #[frb] // disabled
pub async fn is_vault_unlocked() -> bool {
    if let Ok(guard) = get_account_manager() {
        if let Some(manager) = guard.as_ref() {
            return manager.is_unlocked();
        }
    }
    false
}

/// List all accounts
// #[frb] // disabled
pub async fn list_accounts(base_path: String) -> Vec<BridgeAccountInfo> {
    let path = PathBuf::from(&base_path);
    if let Err(_) = init_account_manager(path) {
        return vec![];
    }

    if let Ok(guard) = get_account_manager() {
        if let Some(manager) = guard.as_ref() {
            return manager
                .list_accounts_sorted()
                .into_iter()
                .map(|a| BridgeAccountInfo {
                    id: a.id,
                    name: a.name,
                    last_accessed: a.last_accessed.map(|dt| dt.to_rfc3339()),
                })
                .collect();
        }
    }
    vec![]
}

/// Get vault statistics
// #[frb] // disabled
pub async fn get_vault_stats() -> FfiVaultStats {
    FfiVaultStats {
        profile_count: 0,
        total_size_bytes: 0,
        last_modified: None,
    }
}

/// List all profiles
// #[frb] // disabled
pub async fn list_profiles() -> Vec<BridgeProfileSummary> {
    if let Ok(guard) = get_account_manager() {
        if let Some(manager) = guard.as_ref() {
            if let Some(vault_guard) = manager.get_vault_store() {
                if let Some(ref vault) = *vault_guard {
                    if let Ok(profiles) = vault.list_profiles() {
                        return profiles
                            .into_iter()
                            .map(|p| BridgeProfileSummary {
                                id: p.id,
                                name: p.name,
                                created_at: p.created_at.to_rfc3339(),
                                updated_at: p.updated_at.to_rfc3339(),
                                version: p.version,
                            })
                            .collect();
                    }
                }
            }
        }
    }
    vec![]
}

/// Create a new profile
// #[frb] // disabled
pub async fn create_profile(name: String, data: Vec<u8>) -> Result<BridgeProfileSummary, String> {
    if let Ok(guard) = get_account_manager() {
        if let Some(manager) = guard.as_ref() {
            if let Some(vault_guard) = manager.get_vault_store() {
                if let Some(ref vault) = *vault_guard {
                    let profile = vault::Profile::new(&name, data);
                    let summary = vault::ProfileSummary::from_profile(&profile);
                    vault.save_profile(&profile).map_err(|e| e.to_string())?;
                    return Ok(BridgeProfileSummary {
                        id: summary.id,
                        name: summary.name,
                        created_at: summary.created_at.to_rfc3339(),
                        updated_at: summary.updated_at.to_rfc3339(),
                        version: summary.version,
                    });
                }
            }
        }
    }
    Err("Vault not unlocked".to_string())
}

/// Save a profile (update if exists)
// #[frb] // disabled
pub async fn real_save_profile(name: String, data: Vec<u8>) -> Result<BridgeProfileSummary, String> {
    if let Ok(guard) = get_account_manager() {
        if let Some(manager) = guard.as_ref() {
            if let Some(vault_guard) = manager.get_vault_store() {
                if let Some(ref vault) = *vault_guard {
                    // Check if profile exists to determine if this is insert or update
                    let existing = vault.list_profiles()
                        .ok()
                        .and_then(|profiles| profiles.into_iter().find(|p| p.name == name));

                    let profile = if let Some(existing) = existing {
                        let mut p = vault::Profile {
                            id: existing.id,
                            name: name.clone(),
                            data,
                            created_at: existing.created_at,
                            updated_at: chrono::Utc::now(),
                            version: existing.version + 1,
                        };
                        p
                    } else {
                        vault::Profile::new(&name, data)
                    };

                    let summary = vault::ProfileSummary::from_profile(&profile);
                    vault.save_profile(&profile).map_err(|e| e.to_string())?;
                    return Ok(BridgeProfileSummary {
                        id: summary.id,
                        name: summary.name,
                        created_at: summary.created_at.to_rfc3339(),
                        updated_at: summary.updated_at.to_rfc3339(),
                        version: summary.version,
                    });
                }
            }
        }
    }
    Err("Vault not unlocked".to_string())
}

/// Load a profile by ID
// #[frb] // disabled
pub async fn real_load_profile(id: String) -> Result<Option<Vec<u8>>, String> {
    if let Ok(guard) = get_account_manager() {
        if let Some(manager) = guard.as_ref() {
            if let Some(vault_guard) = manager.get_vault_store() {
                if let Some(ref vault) = *vault_guard {
                    if let Ok(Some(profile)) = vault.load_profile(&id) {
                        return Ok(Some(profile.data));
                    }
                    return Ok(None);
                }
            }
        }
    }
    Err("Vault not unlocked".to_string())
}

/// Delete a profile by ID
// #[frb] // disabled
pub async fn real_delete_profile(id: String) -> Result<bool, String> {
    if let Ok(guard) = get_account_manager() {
        if let Some(manager) = guard.as_ref() {
            if let Some(vault_guard) = manager.get_vault_store() {
                if let Some(ref vault) = *vault_guard {
                    return vault.delete_profile(&id).map(|_| true).map_err(|e| e.to_string());
                }
            }
        }
    }
    Err("Vault not unlocked".to_string())
}

/// List all profile summaries (FFI version)
// #[frb] // disabled
pub async fn real_list_profiles() -> Vec<BridgeProfileSummary> {
    list_profiles().await
}

/// Encrypt data with AES-256-GCM
// #[frb] // disabled
pub async fn encrypt_data(data: Vec<u8>, key: Vec<u8>) -> Vec<u8> {
    if key.len() != 32 {
        return data;
    }
    let key_array: [u8; 32] = match key.try_into() {
        Ok(k) => k,
        Err(_) => return data,
    };
    match crate::crypto::encrypt_blob(&key_array, &data) {
        Ok(blob) => blob.to_vec(),
        Err(_) => data,
    }
}

/// Decrypt data with AES-256-GCM
// #[frb] // disabled
pub async fn decrypt_data(encrypted: Vec<u8>, key: Vec<u8>) -> Vec<u8> {
    if key.len() != 32 {
        return encrypted;
    }
    let key_array: [u8; 32] = match key.try_into() {
        Ok(k) => k,
        Err(_) => return encrypted,
    };
    match crate::crypto::decrypt_blob(&key_array, &encrypted) {
        Ok(blob) => blob.to_vec(),
        Err(_) => encrypted,
    }
}

/// Simple ping test function
// #[frb] // disabled
pub fn ping(s: String) -> String {
    format!("pong: {}", s)
}

// ============================================================================
// JSON Relay Pattern FFI - Single function for all vault operations
// ============================================================================

/// Unified vault request handler using JSON relay pattern.
///
/// This function sidesteps flutter_rust_bridge's complex type limitations by
/// accepting a JSON request envelope and returning a JSON response envelope.
///
/// Request format:
///   {"action": "list_profiles"|"save_profile"|"create_profile"|"load_profile"|"delete_profile"|"get_vault_stats"|"is_unlocked", "payload": {...}}
///
/// Response format:
///   {"success": bool, "data": {...}, "error": "string"|null}
#[frb]
pub fn vault_request(request_json: String) -> String {
    let request: vault::processor::VaultRequest = match serde_json::from_str(&request_json) {
        Ok(r) => r,
        Err(e) => {
            let response = vault::processor::VaultResponse::error(format!("Invalid request JSON: {}", e));
            return serde_json::to_string(&response).unwrap_or_else(|_| r#"{"success":false,"error":"serialization error"}"#.to_string());
        }
    };

    // Get account manager
    let manager_guard = match get_account_manager() {
        Ok(g) => g,
        Err(e) => {
            let response = vault::processor::VaultResponse::error(format!("Account manager error: {}", e));
            return serde_json::to_string(&response).unwrap_or_else(|_| r#"{"success":false,"error":"serialization error"}"#.to_string());
        }
    };

    let manager = match manager_guard.as_ref() {
        Some(m) => m,
        None => {
            let response = vault::processor::VaultResponse::error("No account manager".to_string());
            return serde_json::to_string(&response).unwrap_or_else(|_| r#"{"success":false,"error":"serialization error"}"#.to_string());
        }
    };

    let response = vault::processor::handle_vault_request(request, manager);
    serde_json::to_string(&response).unwrap_or_else(|_| r#"{"success":false,"error":"response serialization error"}"#.to_string())
}

// C-compatible FFI entry point for direct Dart FFI calls (bypasses flutter_rust_bridge)
#[no_mangle]
pub extern "C" fn vault_request_ffi(
    request_ptr: *const libc::c_char,
    request_len: libc::size_t,
) -> *mut libc::c_char {
    use std::ffi::{CStr, CString};
    use std::slice;

    if request_ptr.is_null() {
        let response = vault::processor::VaultResponse::error("Null request pointer".to_string());
        let json = serde_json::to_string(&response).unwrap_or_default();
        return CString::new(json).unwrap().into_raw();
    }

    // Read the request string from C
    let request_bytes = unsafe { slice::from_raw_parts(request_ptr as *const u8, request_len) };
    let request_str = String::from_utf8_lossy(request_bytes);

    // Process the request
    let response_str = vault_request(request_str.to_string());

    // Return ownership to Dart (caller must free)
    CString::new(response_str).unwrap().into_raw()
}

/// Initialize account manager from base path (C-compatible)
#[no_mangle]
pub extern "C" fn init_account_manager_ffi(base_path_ptr: *const libc::c_char) -> libc::c_int {
    use std::ffi::CStr;

    if base_path_ptr.is_null() {
        return -1;
    }

    let c_str = unsafe { CStr::from_ptr(base_path_ptr) };
    let base_path = c_str.to_string_lossy().to_string();
    let path = PathBuf::from(base_path);

    match init_account_manager(path) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

/// Check if vault is unlocked (C-compatible)
#[no_mangle]
pub extern "C" fn is_vault_unlocked_ffi() -> libc::c_int {
    if let Ok(guard) = get_account_manager() {
        if let Some(manager) = guard.as_ref() {
            if manager.is_unlocked() {
                return 1;
            }
        }
    }
    0
}

/// Free a string allocated by Rust (must be called by Dart to prevent memory leaks)
#[no_mangle]
pub extern "C" fn free_rust_string_ffi(ptr: *mut libc::c_char) {
    if !ptr.is_null() {
        drop(unsafe { CString::from_raw(ptr) });
    }
}
