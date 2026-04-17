//! Host functions - Secure interface between Wasm and Rust Core
//!
//! Plugins can only access data through these strictly defined functions

use wasmtime::Trap;

/// Host functions for plugin access
pub struct SoloHostFunctions {
    /// Plugin ID
    pub plugin_id: String,
    /// Session ID for audit
    pub session_id: String,
    /// Requested fields (from manifest)
    pub requested_fields: Vec<String>,
}

impl SoloHostFunctions {
    /// Create new host functions
    pub fn new(plugin_id: &str, session_id: &str, requested_fields: Vec<String>) -> Self {
        Self {
            plugin_id: plugin_id.to_string(),
            session_id: session_id.to_string(),
            requested_fields,
        }
    }

    /// Get user name (requires confirmation)
    pub fn get_user_name(&mut self) -> Result<String, Trap> {
        // TODO: Trigger Flutter confirmation dialog via channel
        // For now, return empty
        Ok(String::new())
    }

    /// Get ID card number (requires strict confirmation + TTL)
    pub fn get_id_card_number(&mut self) -> Result<String, Trap> {
        // TODO: Trigger Flutter confirmation with "阅后即焚" warning
        Ok(String::new())
    }

    /// Get email (requires confirmation)
    pub fn get_email(&mut self) -> Result<String, Trap> {
        // TODO: Trigger Flutter confirmation dialog
        Ok(String::new())
    }

    /// Get phone number (requires confirmation)
    pub fn get_phone(&mut self) -> Result<String, Trap> {
        // TODO: Trigger Flutter confirmation dialog
        Ok(String::new())
    }

    /// Check if field is allowed
    pub fn is_field_allowed(&self, field: &str) -> bool {
        self.requested_fields.iter().any(|f| {
            f == field || f == "*" || field.starts_with(f)
        })
    }
}
