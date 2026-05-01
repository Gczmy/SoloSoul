//! FRB API surface — typed functions exposed to Dart via flutter_rust_bridge.
//!
//! This module replaces the JSON relay pattern with type-safe FRB bindings.
//! Functions here are annotated with `#[frb]` and auto-generated into Dart code.

use flutter_rust_bridge::frb;
use std::collections::HashMap;

use crate::account::AccountManager;
use crate::vault::processor::{VaultRequest, VaultResponse};

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
