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

/// Constant-time comparison of two byte slices.
/// Prevents timing attacks by always comparing all bytes regardless of early mismatch.
/// Use for comparing hashes, keys, or other cryptographic material.
#[frb]
pub fn frb_constant_time_compare(a: Vec<u8>, b: Vec<u8>) -> bool {
    crate::crypto::utils::constant_time_compare(&a, &b)
}

// ============================================================================
// Sync — Device-to-Device Synchronization
// ============================================================================

/// Direction of sync result
#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SyncDirection {
    /// Local changes pushed to remote
    Pushed,
    /// Remote changes pulled to local
    Pulled,
    /// Both sides had changes, merged via CRDT
    Merged,
    /// No changes on either side
    NoChange,
}

/// Result of a sync operation
#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncResult {
    pub success: bool,
    pub direction: SyncDirection,
    pub bytes_sent: usize,
    pub bytes_received: usize,
    pub error: Option<String>,
}

/// Discovered device on the local network
#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiscoveredDevice {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub addresses: Vec<String>,
}

/// Discover SoloSoul devices on the local network via mDNS.
/// Returns a list of discovered devices after waiting for [timeout_ms].
#[frb]
pub fn frb_mdns_discover(timeout_ms: u64) -> Result<Vec<DiscoveredDevice>, String> {
    let discovery = crate::discovery::mdns::MdnsDiscovery::new()?;
    let devices = discovery.browse(timeout_ms)?;
    Ok(devices
        .into_iter()
        .map(|d| DiscoveredDevice {
            name: d.name,
            host: d.host,
            port: d.port,
            addresses: d.addresses.iter().map(|a| a.to_string()).collect(),
        })
        .collect())
}

/// Advertise this device on the local network via mDNS.
/// [device_name] should be unique (e.g. account ID or device name).
/// [port] is the TCP port the sync server listens on.
#[frb]
pub fn frb_mdns_advertise(device_name: String, port: u16) -> Result<(), String> {
    let mut discovery = crate::discovery::mdns::MdnsDiscovery::new()?;
    discovery.advertise(&device_name, port)
}

/// Helper: get session key from account manager
fn get_session_key() -> Result<[u8; 32], String> {
    let manager_guard =
        crate::get_account_manager().map_err(|e| format!("Account manager error: {}", e))?;
    let manager = manager_guard
        .as_ref()
        .ok_or("Account manager not initialized")?;
    manager
        .get_session_key()
        .ok_or("Vault not unlocked".to_string())
        .map(|k| *k)
}

/// Helper: decrypt profile data bytes into ProfileData
fn decrypt_profile_data_bytes(encrypted: &[u8]) -> Result<crate::vault::ProfileData, String> {
    let key = get_session_key()?;
    let decrypted = crate::crypto::decrypt_profile_data(&key, encrypted)
        .map_err(|e| format!("Decrypt failed: {}", e))?;
    let json = String::from_utf8(decrypted.to_vec())
        .map_err(|e| format!("Invalid UTF-8: {}", e))?;
    serde_json::from_str(&json).map_err(|e| format!("JSON parse failed: {}", e))
}

/// Helper: encrypt ProfileData into bytes
fn encrypt_profile_data_bytes(profile_data: &crate::vault::ProfileData) -> Result<Vec<u8>, String> {
    let key = get_session_key()?;
    let json = serde_json::to_string(profile_data)
        .map_err(|e| format!("JSON serialize failed: {}", e))?;
    let encrypted = crate::crypto::encrypt_profile_data(&key, json.as_bytes())
        .map_err(|e| format!("Encrypt failed: {}", e))?;
    Ok(encrypted.to_vec())
}

/// Sync profile with a remote device as the initiator (sends state vector first).
///
/// [account_id] identifies the account to sync.
/// [remote_addr] is the remote device address (e.g. "192.168.1.5:9900").
/// [pairing_key] is the shared pairing key for Noise handshake.
/// [device_salt] is this device's unique identifier for key derivation.
#[frb]
pub fn frb_sync_initiator(
    account_id: String,
    remote_addr: String,
    pairing_key: Vec<u8>,
    device_salt: Vec<u8>,
) -> Result<SyncResult, String> {
    use crate::sync::engine::SyncEngine;
    use crate::sync::transport::TcpTransport;

    // 1. Load and decrypt current profile
    let manager_guard =
        crate::get_account_manager().map_err(|e| format!("Account manager error: {}", e))?;
    let manager = manager_guard
        .as_ref()
        .ok_or("Account manager not initialized")?;
    let vault_guard = manager.get_vault_store();
    let vault_lock = vault_guard.ok_or("Vault not unlocked")?;
    let vault = vault_lock.as_ref().ok_or("Vault not unlocked")?;

    let profile = vault
        .load_profile(&account_id)
        .map_err(|e| format!("Load profile failed: {}", e))?
        .ok_or_else(|| format!("Profile not found: {}", account_id))?;

    let profile_data = decrypt_profile_data_bytes(&profile.data)?;

    // 2. Create CRDT doc from profile
    let meta = crate::sync::crdt::DocMeta {
        profile_id: account_id.clone(),
        version: 1,
        last_modified: chrono::Utc::now().to_rfc3339(),
    };
    let crdt_doc = crate::sync::crdt::SoloDoc::from_profile(&profile_data, &meta);

    // 3. Establish Noise channel
    let local_keypair =
        crate::sync::protocol::SecureChannel::derive_keypair(&pairing_key, &device_salt);
    let responder_keypair =
        crate::sync::protocol::SecureChannel::derive_keypair(&pairing_key, b"remote");
    let (channel, _) =
        crate::sync::protocol::SecureChannel::handshake_ik(&local_keypair, &responder_keypair)
            .map_err(|e| format!("Noise handshake failed: {}", e))?;

    // 4. Connect and sync
    let transport =
        TcpTransport::connect(&remote_addr).map_err(|e| format!("TCP connect failed: {}", e))?;

    let mut engine = SyncEngine::new(crdt_doc, Some(channel), Box::new(transport));
    let result = engine.sync_initiator()?;

    // 5. If we pulled changes, encrypt and save updated profile
    if matches!(
        result.direction,
        crate::sync::engine::SyncDirection::Pulled
            | crate::sync::engine::SyncDirection::Merged
    ) {
        let updated_data = engine.crdt.to_profile()?;
        let encrypted = encrypt_profile_data_bytes(&updated_data)?;
        let mut new_profile = profile.clone();
        new_profile.data = encrypted;
        new_profile.updated_at = chrono::Utc::now();
        vault
            .save_profile(&new_profile)
            .map_err(|e| format!("Save profile failed: {}", e))?;
    }

    Ok(SyncResult {
        success: result.success,
        direction: match result.direction {
            crate::sync::engine::SyncDirection::Pushed => SyncDirection::Pushed,
            crate::sync::engine::SyncDirection::Pulled => SyncDirection::Pulled,
            crate::sync::engine::SyncDirection::Merged => SyncDirection::Merged,
            crate::sync::engine::SyncDirection::NoChange => SyncDirection::NoChange,
        },
        bytes_sent: result.bytes_sent,
        bytes_received: result.bytes_received,
        error: result.error,
    })
}

/// Sync profile with a remote device as the responder (receives state vector first).
///
/// [account_id] identifies the account to sync.
/// [remote_addr] is the remote device address (e.g. "192.168.1.5:9900").
/// [pairing_key] is the shared pairing key for Noise handshake.
/// [device_salt] is this device's unique identifier for key derivation.
#[frb]
pub fn frb_sync_responder(
    account_id: String,
    remote_addr: String,
    pairing_key: Vec<u8>,
    device_salt: Vec<u8>,
) -> Result<SyncResult, String> {
    use crate::sync::engine::SyncEngine;
    use crate::sync::transport::TcpTransport;

    // 1. Load and decrypt current profile
    let manager_guard =
        crate::get_account_manager().map_err(|e| format!("Account manager error: {}", e))?;
    let manager = manager_guard
        .as_ref()
        .ok_or("Account manager not initialized")?;
    let vault_guard = manager.get_vault_store();
    let vault_lock = vault_guard.ok_or("Vault not unlocked")?;
    let vault = vault_lock.as_ref().ok_or("Vault not unlocked")?;

    let profile = vault
        .load_profile(&account_id)
        .map_err(|e| format!("Load profile failed: {}", e))?
        .ok_or_else(|| format!("Profile not found: {}", account_id))?;

    let profile_data = decrypt_profile_data_bytes(&profile.data)?;

    // 2. Create CRDT doc from profile
    let meta = crate::sync::crdt::DocMeta {
        profile_id: account_id.clone(),
        version: 1,
        last_modified: chrono::Utc::now().to_rfc3339(),
    };
    let crdt_doc = crate::sync::crdt::SoloDoc::from_profile(&profile_data, &meta);

    // 3. Establish Noise channel
    let local_keypair =
        crate::sync::protocol::SecureChannel::derive_keypair(&pairing_key, &device_salt);
    let initiator_keypair =
        crate::sync::protocol::SecureChannel::derive_keypair(&pairing_key, b"remote");
    let (_, channel) =
        crate::sync::protocol::SecureChannel::handshake_ik(&initiator_keypair, &local_keypair)
            .map_err(|e| format!("Noise handshake failed: {}", e))?;

    // 4. Connect and sync
    let transport =
        TcpTransport::connect(&remote_addr).map_err(|e| format!("TCP connect failed: {}", e))?;

    let mut engine = SyncEngine::new(crdt_doc, Some(channel), Box::new(transport));
    let result = engine.sync_responder()?;

    // 5. If we pulled changes, encrypt and save updated profile
    if matches!(
        result.direction,
        crate::sync::engine::SyncDirection::Pulled
            | crate::sync::engine::SyncDirection::Merged
    ) {
        let updated_data = engine.crdt.to_profile()?;
        let encrypted = encrypt_profile_data_bytes(&updated_data)?;
        let mut new_profile = profile.clone();
        new_profile.data = encrypted;
        new_profile.updated_at = chrono::Utc::now();
        vault
            .save_profile(&new_profile)
            .map_err(|e| format!("Save profile failed: {}", e))?;
    }

    Ok(SyncResult {
        success: result.success,
        direction: match result.direction {
            crate::sync::engine::SyncDirection::Pushed => SyncDirection::Pushed,
            crate::sync::engine::SyncDirection::Pulled => SyncDirection::Pulled,
            crate::sync::engine::SyncDirection::Merged => SyncDirection::Merged,
            crate::sync::engine::SyncDirection::NoChange => SyncDirection::NoChange,
        },
        bytes_sent: result.bytes_sent,
        bytes_received: result.bytes_received,
        error: result.error,
    })
}

// ============================================================================
// OCR — Phase 1 (MRZ) + Phase 2 (General OCR)
// ============================================================================

/// OCR 引擎状态
#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone)]
pub struct OcrEngineStatus {
    pub is_loaded: bool,
    pub det_loaded: bool,
    pub cls_loaded: bool,
    pub rec_loaded: bool,
    pub uptime_secs: u64,
}

/// 边界框（相对坐标 0.0~1.0）
#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone)]
pub struct FrbBoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// OCR 文本块
#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone)]
pub struct FrbOcrBlock {
    pub text: String,
    pub confidence: f32,
    pub bbox: FrbBoundingBox,
}

/// 通用 OCR 识别结果
#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone)]
pub struct FrbOcrResult {
    pub raw_text: String,
    pub blocks: Vec<FrbOcrBlock>,
    pub confidence: f32,
}

// ----------------------------------------------------------------------------
// 初始化
// ----------------------------------------------------------------------------

/// Phase 1 兼容：仅加载 rec 模型
#[frb]
pub fn frb_ocr_init(model_bytes: Vec<u8>) -> Result<(), String> {
    crate::ocr::load_models_from_memory(&model_bytes)
        .map_err(|e| e.to_string())
}

/// Phase 2：加载 det + cls + rec 三个模型
///
/// `det_model_bytes`: det 模型字节（~4MB），空切片表示跳过
/// `cls_model_bytes`: cls 模型字节（~1MB），空切片表示跳过
/// `rec_model_bytes`: rec 模型字节（~8MB）
#[frb]
pub fn frb_ocr_init_v2(
    det_model_bytes: Vec<u8>,
    cls_model_bytes: Vec<u8>,
    rec_model_bytes: Vec<u8>,
) -> Result<(), String> {
    crate::ocr::load_models_from_memory_v2(&det_model_bytes, &cls_model_bytes, &rec_model_bytes)
        .map_err(|e| e.to_string())
}

// ----------------------------------------------------------------------------
// MRZ 识别（Phase 1）
// ----------------------------------------------------------------------------

/// Extract raw MRZ lines from an image.
#[frb]
pub fn frb_ocr_extract_mrz_raw(image_data: Vec<u8>) -> Result<Vec<String>, String> {
    let img = image::load_from_memory(&image_data)
        .map_err(|e| format!("Invalid image: {}", e))?;

    crate::ocr::extract_mrz_lines(&img)
        .map_err(|e| e.to_string())
}

// ----------------------------------------------------------------------------
// 通用 OCR（Phase 2）
// ----------------------------------------------------------------------------

/// 对任意图像执行通用 OCR 识别
///
/// 返回结构化结果，包含每个文本块的坐标、文本和置信度。
#[frb]
pub fn frb_ocr_recognize(image_data: Vec<u8>) -> Result<FrbOcrResult, String> {
    let img = image::load_from_memory(&image_data)
        .map_err(|e| format!("Invalid image: {}", e))?;

    let result = crate::ocr::recognize_image(&img)
        .map_err(|e| e.to_string())?;

    Ok(FrbOcrResult {
        raw_text: result.raw_text,
        blocks: result.blocks.into_iter().map(|b| FrbOcrBlock {
            text: b.text,
            confidence: b.confidence,
            bbox: FrbBoundingBox {
                x: b.bbox.x,
                y: b.bbox.y,
                width: b.bbox.width,
                height: b.bbox.height,
            },
        }).collect(),
        confidence: result.confidence,
    })
}

// ----------------------------------------------------------------------------
// 状态查询与资源管理
// ----------------------------------------------------------------------------

/// Get the current OCR engine status.
#[frb]
pub fn frb_ocr_status() -> OcrEngineStatus {
    let status = crate::ocr::engine_status();
    OcrEngineStatus {
        is_loaded: status.is_loaded,
        det_loaded: status.det_loaded,
        cls_loaded: status.cls_loaded,
        rec_loaded: status.rec_loaded,
        uptime_secs: status.uptime_secs,
    }
}

/// Release OCR engine resources.
#[frb]
pub fn frb_ocr_release() {
    crate::ocr::unload_models();
}

// ============================================================================
// Plugin System
// ============================================================================

// StreamSink 需要通过 crate::frb_generated 导入，待 FRB 代码生成时处理
// #[cfg(feature = "sandbox")]
// use flutter_rust_bridge::StreamSink;

/// Plugin manifest exposed to Dart (re-exported from plugin module)
#[cfg(feature = "sandbox")]
pub use crate::plugin::PluginManifest;

/// Plugin event for StreamSink
// PluginEvent 用于 StreamSink，待 FRB Stream 支持启用后取消注释
// #[cfg(feature = "sandbox")]
// pub use crate::plugin::PluginEvent;

/// Plugin session info (bridge-friendly, non-opaque)
#[cfg(feature = "sandbox")]
#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone)]
pub struct PluginSessionInfo {
    pub session_id: String,
    pub plugin_id: String,
    pub plugin_name: String,
    pub started_at_secs: i64,
    pub expires_at_secs: i64,
}

/// 获取插件安装基础目录（Dart/Rust 路径一致性）
#[cfg(feature = "sandbox")]
#[frb]
pub fn frb_get_plugin_base_dir() -> Result<String, String> {
    crate::plugin::with_manager(|m| Ok(m.get_base_dir()))
}

/// 安装插件（Rust 侧直接读取文件）
#[cfg(feature = "sandbox")]
#[frb]
pub fn frb_plugin_install(wasm_path: String, manifest_path: String) -> Result<String, String> {
    crate::plugin::with_manager(|m| m.install_plugin(wasm_path, manifest_path))
}

/// 加载插件清单
#[cfg(feature = "sandbox")]
#[frb]
pub fn frb_plugin_load_manifest(plugin_id: String) -> Result<PluginManifest, String> {
    crate::plugin::with_manager(|m| m.load_manifest(&plugin_id))
}

/// 列出已安装插件
#[cfg(feature = "sandbox")]
#[frb]
pub fn frb_plugin_list_installed() -> Result<Vec<String>, String> {
    crate::plugin::with_manager(|m| m.list_installed())
}

/// 列出所有活跃 Session
#[cfg(feature = "sandbox")]
#[frb]
pub fn frb_plugin_list_active_sessions() -> Result<Vec<PluginSessionInfo>, String> {
    use crate::plugin::SessionInfo;
    crate::plugin::with_manager(|m| {
        Ok(m.list_active_sessions()
            .into_iter()
            .map(|s: SessionInfo| PluginSessionInfo {
                session_id: s.session_id,
                plugin_id: s.plugin_id,
                plugin_name: s.plugin_name,
                started_at_secs: s.started_at_secs,
                expires_at_secs: s.expires_at_secs,
            })
            .collect())
    })
}

/// 执行插件（返回事件流）
/// TODO: StreamSink 类型需要 FRB 代码生成支持，当前为 stub
#[cfg(feature = "sandbox")]
#[frb]
pub fn frb_plugin_execute(
    plugin_id: String,
    session_ttl_seconds: u64,
) -> Result<i32, String> {
    // stub: 返回 0 表示成功
    Ok(0)
}

/// 响应用户授权（Dart -> Rust）
#[cfg(feature = "sandbox")]
#[frb]
pub fn frb_plugin_consent_response(
    request_id: String,
    approved: bool,
    value: Option<String>,
) -> Result<(), String> {
    crate::plugin::with_manager(|m| m.consent_response(request_id, approved, value))
}

/// 强制卸载插件
#[cfg(feature = "sandbox")]
#[frb]
pub fn frb_plugin_force_unload(plugin_id: String) -> Result<(), String> {
    crate::plugin::with_manager(|m| m.force_unload(&plugin_id))
}
