mod frb_generated; /* AUTO INJECTED BY flutter_rust_bridge. This line may not be accurate, and you can change it according to your needs. */
// SoloSoul Core - Rust Core Library for Flutter
//
// This library provides:
// - High-performance Argon2id key derivation
// - AES-256-GCM encryption/decryption
// - Vault storage with rusqlite + SQLCipher (双重加密)
// - E2EE cloud sync engine
// - Wasm plugin sandbox

pub mod account;
pub mod api;
pub mod crypto;
pub mod discovery;
pub mod ocr;
#[cfg(feature = "sandbox")]
pub mod plugin;
pub mod safe_storage;
pub mod sync;
pub mod vault;

use flutter_rust_bridge::frb;
use serde::{Deserialize, Serialize};
use std::ffi::CString;
use std::path::PathBuf;
use std::sync::Mutex;
use thiserror::Error;

/// 全局日志文件 Mutex，防止多线程竞争
static LOG_FILE_MUTEX: Mutex<()> = Mutex::new(());

/// 写日志到 /tmp/solosoul_rust.log
pub(crate) fn log_to_file(msg: &str) {
    use std::fs::OpenOptions;
    use std::io::Write;
    let _guard = LOG_FILE_MUTEX.lock();
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("/tmp/solosoul_rust.log") {
        let _ = writeln!(file, "[{}] {}", now, msg);
    }
}

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

pub(crate) fn get_account_manager(
) -> Result<std::sync::MutexGuard<'static, Option<account::AccountManager>>, String> {
    ACCOUNT_MANAGER
        .lock()
        .map_err(|e| format!("Lock poisoned: {}", e))
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
            let response =
                vault::processor::VaultResponse::error(format!("Invalid request JSON: {}", e));
            return serde_json::to_string(&response).unwrap_or_else(|_| {
                r#"{"success":false,"error":"serialization error"}"#.to_string()
            });
        }
    };

    // Get account manager
    let manager_guard = match get_account_manager() {
        Ok(g) => g,
        Err(e) => {
            let response =
                vault::processor::VaultResponse::error(format!("Account manager error: {}", e));
            return serde_json::to_string(&response).unwrap_or_else(|_| {
                r#"{"success":false,"error":"serialization error"}"#.to_string()
            });
        }
    };

    let manager = match manager_guard.as_ref() {
        Some(m) => m,
        None => {
            let response = vault::processor::VaultResponse::error("No account manager".to_string());
            return serde_json::to_string(&response).unwrap_or_else(|_| {
                r#"{"success":false,"error":"serialization error"}"#.to_string()
            });
        }
    };

    let response = vault::processor::handle_vault_request(request, manager);
    serde_json::to_string(&response).unwrap_or_else(|_| {
        r#"{"success":false,"error":"response serialization error"}"#.to_string()
    })
}

// C-compatible FFI entry point for direct Dart FFI calls (bypasses flutter_rust_bridge)
#[no_mangle]
pub extern "C" fn vault_request_ffi(
    request_ptr: *const libc::c_char,
    request_len: libc::size_t,
) -> *mut libc::c_char {
    use std::ffi::CString;
    use std::panic;
    use std::slice;

    // Catch panics to prevent crashing the entire process
    let result = panic::catch_unwind(|| {
        if request_ptr.is_null() {
            let response =
                vault::processor::VaultResponse::error("Null request pointer".to_string());
            let json = serde_json::to_string(&response).unwrap_or_default();
            return CString::new(json).unwrap().into_raw();
        }

        // Read the request string from C
        let request_bytes = unsafe { slice::from_raw_parts(request_ptr as *const u8, request_len) };
        let request_str = String::from_utf8_lossy(request_bytes);

        // Process the request
        let response_str = vault_request(request_str.to_string());

        // Return ownership to Dart (caller must free)
        CString::new(response_str)
            .unwrap_or_else(|_| {
                CString::new(r#"{"success":false,"error":"response encoding error"}"#).unwrap()
            })
            .into_raw()
    });

    match result {
        Ok(ptr) => ptr,
        Err(_) => {
            let response = vault::processor::VaultResponse::error("Internal error".to_string());
            let json = serde_json::to_string(&response).unwrap_or_default();
            CString::new(json)
                .unwrap_or_else(|_| CString::new("{}").unwrap())
                .into_raw()
        }
    }
}

/// Initialize account manager from base path (C-compatible)
///
/// # Safety
/// `base_path_ptr` must be a valid, null-terminated C string pointer.
#[no_mangle]
pub unsafe extern "C" fn init_account_manager_ffi(
    base_path_ptr: *const libc::c_char,
) -> libc::c_int {
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
///
/// # Safety
/// `ptr` must have been previously allocated by `CString::into_raw` in this crate.
#[no_mangle]
pub unsafe extern "C" fn free_rust_string_ffi(ptr: *mut libc::c_char) {
    if !ptr.is_null() {
        drop(unsafe { CString::from_raw(ptr) });
    }
}
