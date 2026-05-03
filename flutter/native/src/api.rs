//! FRB API surface — typed functions exposed to Dart via flutter_rust_bridge.
//!
//! This module replaces the JSON relay pattern with type-safe FRB bindings.
//! Functions here are annotated with `#[frb]` and auto-generated into Dart code.

use flutter_rust_bridge::frb;
use std::collections::HashMap;

// ============================================================================
// Initialization
// ============================================================================

/// Initialize the account manager with a base path.
/// Must be called before any other vault operations.
#[frb]
pub fn frb_init_account_manager(base_path: String) -> Result<(), String> {
    crate::init_account_manager(std::path::PathBuf::from(base_path))
}

// ============================================================================
// Prototype: Complex types for FRB validation
// ============================================================================

/// Sensitivity level for profile fields
#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SensitivityLevel {
    Public,
    Private,
    Restricted,
}

/// A single property value — tests enum-with-data FRB generation
#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PropertyValue {
    Text {
        text: String,
        sensitivity: SensitivityLevel,
    },
    Number {
        value: f64,
    },
    Boolean {
        value: bool,
    },
    RichText {
        html: String,
        sensitivity: SensitivityLevel,
    },
}

/// A field history entry
#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FieldHistoryEntry {
    pub value: PropertyValue,
    pub timestamp: String,
    pub source: Option<String>,
}

/// Nested HashMap structure — tests FRB's handling of complex nested types
#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FormHistories {
    pub histories: HashMap<String, HashMap<String, Vec<FieldHistoryEntry>>>,
}

/// Vault statistics returned from Rust
#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VaultStats {
    pub profile_count: usize,
    pub total_size_bytes: u64,
    pub last_modified: Option<String>,
    pub account_id: Option<String>,
}

/// Account info from Rust vault
#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountInfo {
    pub id: String,
    pub name: String,
    pub created_at: Option<String>,
    pub last_accessed: Option<String>,
    pub password_hint: Option<String>,
    pub last_login_at: Option<String>,
    pub last_operation_at: Option<String>,
    pub last_operation_desc: Option<String>,
}

/// Profile summary from Rust vault
#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProfileSummary {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: u32,
}

/// Result of account creation
#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateAccountResult {
    pub success: bool,
    pub error: Option<String>,
    pub account_id: Option<String>,
    pub name: Option<String>,
    pub salt: Option<String>,
    pub verify_hash: Option<String>,
}

/// Result of vault unlock
#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UnlockVaultResult {
    pub success: bool,
    pub error: Option<String>,
    pub crypto_version: Option<i32>,
}

/// Result of password change
#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChangePasswordResult {
    pub success: bool,
    pub error: Option<String>,
    pub salt: Option<String>,
    pub verify_hash: Option<String>,
}

// ============================================================================
// Prototype validation: simple function to test FRB pipeline
// ============================================================================

/// Test function — validates FRB can generate a simple function
#[frb]
pub fn frb_ping() -> String {
    "pong from Rust FRB".to_string()
}

/// Test function — validates FRB handles enum-with-data return
#[frb]
pub fn frb_test_property_value() -> PropertyValue {
    PropertyValue::Text {
        text: "hello".to_string(),
        sensitivity: SensitivityLevel::Private,
    }
}

/// Test function — validates FRB handles nested HashMap
#[frb]
pub fn frb_test_form_histories() -> FormHistories {
    let mut inner = HashMap::new();
    inner.insert(
        "field1".to_string(),
        vec![FieldHistoryEntry {
            value: PropertyValue::Number { value: 42.0 },
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            source: Some("test".to_string()),
        }],
    );
    let mut histories = HashMap::new();
    histories.insert("section1".to_string(), inner);
    FormHistories { histories }
}

// ============================================================================
// P0: FRB typed functions — replace JSON relay pattern
// ============================================================================

/// KDF parameter presets (re-exported for FRB codegen)
#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FrbKdfPreset {
    /// 8 MiB, 2 iterations — low-end devices
    Fast,
    /// 16 MiB, 3 iterations — default
    Balanced,
    /// 64 MiB, 3 iterations — high security
    Secure,
}

impl From<FrbKdfPreset> for crate::crypto::argon2::KdfPreset {
    fn from(preset: FrbKdfPreset) -> Self {
        match preset {
            FrbKdfPreset::Fast => crate::crypto::argon2::KdfPreset::Fast,
            FrbKdfPreset::Balanced => crate::crypto::argon2::KdfPreset::Balanced,
            FrbKdfPreset::Secure => crate::crypto::argon2::KdfPreset::Secure,
        }
    }
}

/// Loaded profile data returned from Rust
#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoadedProfile {
    pub id: String,
    pub name: String,
    pub data: Vec<u8>,
    pub version: u32,
}

/// Result of key derivation
#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeriveKeyResult {
    pub success: bool,
    pub error: Option<String>,
    pub key: Option<Vec<u8>>,
}

// ---------------------------------------------------------------------------
// Encrypt / Decrypt (highest priority — core crypto path)
// ---------------------------------------------------------------------------

/// Encrypt arbitrary bytes using the vault's session key.
/// Returns the encrypted SOLO blob bytes.
/// Vault must be unlocked.
#[frb]
pub fn frb_encrypt_bytes(data: Vec<u8>) -> Result<Vec<u8>, String> {
    let manager_guard =
        crate::get_account_manager().map_err(|e| format!("Account manager error: {}", e))?;
    let manager = manager_guard
        .as_ref()
        .ok_or("Account manager not initialized")?;

    let session_key = manager.get_session_key().ok_or("Vault not unlocked")?;

    let key: [u8; 32] = session_key
        .as_slice()
        .try_into()
        .map_err(|_| "Invalid session key length")?;

    let blob = crate::crypto::encrypt_profile_data(&key, &data)
        .map_err(|e| format!("Encryption failed: {}", e))?;

    Ok(blob.to_vec())
}

/// Decrypt SOLO blob (or legacy Dart format) bytes using the vault's session key.
/// Returns the plaintext bytes.
/// Vault must be unlocked.
#[frb]
pub fn frb_decrypt_bytes(data: Vec<u8>) -> Result<Vec<u8>, String> {
    let manager_guard =
        crate::get_account_manager().map_err(|e| format!("Account manager error: {}", e))?;
    let manager = manager_guard
        .as_ref()
        .ok_or("Account manager not initialized")?;

    let session_key = manager.get_session_key().ok_or("Vault not unlocked")?;

    let key: [u8; 32] = session_key
        .as_slice()
        .try_into()
        .map_err(|_| "Invalid session key length")?;

    let plaintext = crate::crypto::decrypt_profile_data(&key, &data)
        .map_err(|e| format!("Decryption failed: {}", e))?;

    Ok(plaintext.to_vec())
}

// ---------------------------------------------------------------------------
// Profile CRUD (high priority — data path)
// ---------------------------------------------------------------------------

/// Save a profile (create or update) with raw encrypted bytes.
/// Returns the profile summary on success.
/// Vault must be unlocked.
#[frb]
pub fn frb_save_profile(name: String, data: Vec<u8>) -> Result<ProfileSummary, String> {
    let manager_guard =
        crate::get_account_manager().map_err(|e| format!("Account manager error: {}", e))?;
    let manager = manager_guard
        .as_ref()
        .ok_or("Account manager not initialized")?;

    let vault_guard = manager.get_vault_store();
    let vault_lock = vault_guard.ok_or("Vault not unlocked")?;
    let vault = vault_lock.as_ref().ok_or("Vault not unlocked")?;

    // Check if profile exists by name
    let existing = vault
        .list_profiles()
        .ok()
        .and_then(|profiles| profiles.into_iter().find(|p| p.name == name));

    // Delete any existing profile with a different ID (legacy data with random UUID)
    if let Some(ref existing_profile) = existing {
        if existing_profile.id != name {
            let _ = vault.delete_profile(&existing_profile.id);
        }
    }

    let profile = if existing.is_some() {
        crate::vault::Profile {
            id: name.clone(),
            name: name.clone(),
            data,
            created_at: existing.as_ref().unwrap().created_at,
            updated_at: chrono::Utc::now(),
            version: existing.as_ref().unwrap().version + 1,
        }
    } else {
        crate::vault::Profile::new_with_id(&name, &name, data)
    };

    let summary = crate::vault::ProfileSummary::from_profile(&profile);
    vault
        .save_profile(&profile)
        .map_err(|e| format!("Failed to save profile: {}", e))?;

    Ok(ProfileSummary {
        id: summary.id,
        name: summary.name,
        created_at: summary.created_at.to_rfc3339(),
        updated_at: summary.updated_at.to_rfc3339(),
        version: summary.version,
    })
}

/// Load a profile by ID, returning raw encrypted bytes.
/// Vault must be unlocked.
#[frb]
pub fn frb_load_profile(id: String) -> Result<Option<LoadedProfile>, String> {
    let manager_guard =
        crate::get_account_manager().map_err(|e| format!("Account manager error: {}", e))?;
    let manager = manager_guard
        .as_ref()
        .ok_or("Account manager not initialized")?;

    let vault_guard = manager.get_vault_store();
    let vault_lock = vault_guard.ok_or("Vault not unlocked")?;
    let vault = vault_lock.as_ref().ok_or("Vault not unlocked")?;

    match vault.load_profile(&id) {
        Ok(Some(profile)) => Ok(Some(LoadedProfile {
            id: profile.id,
            name: profile.name,
            data: profile.data,
            version: profile.version,
        })),
        Ok(None) => Ok(None),
        Err(e) => Err(format!("Failed to load profile: {}", e)),
    }
}

// ---------------------------------------------------------------------------
// Account management (high priority — auth path)
// ---------------------------------------------------------------------------

/// Create a new account. Returns account info including salt and verify_hash.
#[frb]
pub fn frb_create_account(name: String, password: String) -> Result<CreateAccountResult, String> {
    let manager_guard =
        crate::get_account_manager().map_err(|e| format!("Account manager error: {}", e))?;
    let manager = manager_guard
        .as_ref()
        .ok_or("Account manager not initialized")?;

    match manager.create_account(&name, &password) {
        Ok(info) => Ok(CreateAccountResult {
            success: true,
            error: None,
            account_id: Some(info.id),
            name: Some(info.name),
            salt: Some(info.salt),
            verify_hash: Some(info.verify_hash),
        }),
        Err(e) => Ok(CreateAccountResult {
            success: false,
            error: Some(e),
            account_id: None,
            name: None,
            salt: None,
            verify_hash: None,
        }),
    }
}

/// Unlock the vault with account_id and password.
/// Returns success status and crypto_version.
#[frb]
pub fn frb_unlock_vault(account_id: String, password: String) -> Result<UnlockVaultResult, String> {
    let manager_guard =
        crate::get_account_manager().map_err(|e| format!("Account manager error: {}", e))?;
    let manager = manager_guard
        .as_ref()
        .ok_or("Account manager not initialized")?;

    let result = manager.unlock(&account_id, &password);
    Ok(UnlockVaultResult {
        success: result.success,
        error: result.error,
        crypto_version: Some(result.crypto_version as i32),
    })
}

/// Lock the vault — clears session key and closes database connection.
#[frb]
pub fn frb_lock_vault() -> bool {
    if let Ok(guard) = crate::get_account_manager() {
        if let Some(manager) = guard.as_ref() {
            manager.lock();
            return true;
        }
    }
    false
}

/// List all accounts.
#[frb]
pub fn frb_list_accounts() -> Result<Vec<AccountInfo>, String> {
    let manager_guard =
        crate::get_account_manager().map_err(|e| format!("Account manager error: {}", e))?;
    let manager = manager_guard
        .as_ref()
        .ok_or("Account manager not initialized")?;

    let accounts = manager.list_accounts();
    Ok(accounts
        .into_iter()
        .map(|a| AccountInfo {
            id: a.id,
            name: a.name,
            created_at: a.created_at,
            last_accessed: a.last_accessed,
            password_hint: a.password_hint,
            last_login_at: a.last_login_at,
            last_operation_at: a.last_operation_at,
            last_operation_desc: a.last_operation_desc,
        })
        .collect())
}

/// Delete an account and all its data.
#[frb]
pub fn frb_delete_account(account_id: String) -> Result<bool, String> {
    let manager_guard =
        crate::get_account_manager().map_err(|e| format!("Account manager error: {}", e))?;
    let manager = manager_guard
        .as_ref()
        .ok_or("Account manager not initialized")?;

    match manager.delete_account(&account_id) {
        Ok(()) => Ok(true),
        Err(e) => Err(format!("Failed to delete account: {}", e)),
    }
}

/// Get vault statistics.
#[frb]
pub fn frb_get_vault_stats() -> Result<VaultStats, String> {
    let manager_guard =
        crate::get_account_manager().map_err(|e| format!("Account manager error: {}", e))?;
    let manager = manager_guard
        .as_ref()
        .ok_or("Account manager not initialized")?;

    let vault_guard = manager.get_vault_store();
    let vault_lock = vault_guard.ok_or("Vault not unlocked")?;
    let vault = vault_lock.as_ref().ok_or("Vault not unlocked")?;

    match vault.stats() {
        Ok(stats) => Ok(VaultStats {
            profile_count: stats.profile_count,
            total_size_bytes: stats.total_size_bytes,
            last_modified: stats.last_modified,
            account_id: Some(manager.get_unlocked_account().unwrap_or_default()),
        }),
        Err(e) => Err(format!("Failed to get vault stats: {}", e)),
    }
}

/// Change account password.
#[frb]
pub fn frb_change_password(
    account_id: String,
    old_password: String,
    new_password: String,
) -> Result<ChangePasswordResult, String> {
    let manager_guard =
        crate::get_account_manager().map_err(|e| format!("Account manager error: {}", e))?;
    let manager = manager_guard
        .as_ref()
        .ok_or("Account manager not initialized")?;

    match manager.change_password(&account_id, &old_password, &new_password) {
        Ok(info) => Ok(ChangePasswordResult {
            success: true,
            error: None,
            salt: Some(info.salt),
            verify_hash: Some(info.verify_hash),
        }),
        Err(e) => Ok(ChangePasswordResult {
            success: false,
            error: Some(e),
            salt: None,
            verify_hash: None,
        }),
    }
}

// ---------------------------------------------------------------------------
// Key derivation (high priority — standalone FFI endpoint)
// ---------------------------------------------------------------------------

/// Derive a key from password and salt using Argon2id.
/// This is a standalone function that doesn't require vault to be unlocked.
/// Used by Dart auth flow for biometric credential verification, etc.
///
/// - `salt`: raw salt bytes (typically 32 bytes)
/// - `memory_kib`: memory in KiB (e.g. 16384 for 16 MiB)
/// - `iterations`: number of iterations (e.g. 1)
/// - `parallelism`: degree of parallelism (e.g. 4)
#[frb]
pub fn frb_derive_key(
    password: String,
    salt: Vec<u8>,
    memory_kib: u32,
    iterations: u32,
    parallelism: u32,
) -> Result<Vec<u8>, String> {
    let key =
        crate::crypto::argon2::derive_key(&password, &salt, memory_kib, iterations, parallelism)
            .map_err(|e| format!("Key derivation failed: {}", e))?;

    Ok(key.to_vec())
}

// ---------------------------------------------------------------------------
// Random salt generation (standalone — no vault required)
// ---------------------------------------------------------------------------

/// Generate cryptographically secure random bytes.
/// Used for salt generation, nonces, and biometric tokens.
#[frb]
pub fn frb_generate_salt(length: u32) -> Vec<u8> {
    crate::crypto::utils::random_bytes(length as usize)
}

// ---------------------------------------------------------------------------
// Generic AES-256-GCM encrypt/decrypt with explicit key (standalone)
// ---------------------------------------------------------------------------

/// Encrypt data with an explicit 32-byte key using AES-256-GCM (SOLO blob format).
/// Does NOT require the vault to be unlocked — the key is provided by the caller.
/// Used by biometric credential service to encrypt session keys and bio tokens.
#[frb]
pub fn frb_encrypt_with_key(key: Vec<u8>, plaintext: Vec<u8>) -> Result<Vec<u8>, String> {
    let key_arr: [u8; 32] = key
        .as_slice()
        .try_into()
        .map_err(|_| format!("Key must be 32 bytes, got {}", key.len()))?;

    let blob = crate::crypto::encrypt_profile_data(&key_arr, &plaintext)
        .map_err(|e| format!("Encryption failed: {}", e))?;

    Ok(blob.to_vec())
}

/// Decrypt SOLO blob (or legacy format) data with an explicit 32-byte key.
/// Does NOT require the vault to be unlocked — the key is provided by the caller.
/// Used by biometric credential service to decrypt session keys and bio tokens.
#[frb]
pub fn frb_decrypt_with_key(key: Vec<u8>, ciphertext: Vec<u8>) -> Result<Vec<u8>, String> {
    let key_arr: [u8; 32] = key
        .as_slice()
        .try_into()
        .map_err(|_| format!("Key must be 32 bytes, got {}", key.len()))?;

    let plaintext = crate::crypto::decrypt_profile_data(&key_arr, &ciphertext)
        .map_err(|e| format!("Decryption failed: {}", e))?;

    Ok(plaintext.to_vec())
}
