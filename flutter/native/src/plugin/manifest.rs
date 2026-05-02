//! Plugin manifest - Plugin configuration and permissions

use serde::{Deserialize, Serialize};

/// Plugin manifest (manifest.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub plugin_id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub required_fields: Vec<String>,
    #[serde(default)]
    pub optional_fields: Vec<String>,
    #[serde(default)]
    pub network_policy: Option<NetworkPolicy>,
    #[serde(default)]
    pub data_ttl_seconds: u64,
    #[serde(default = "default_require_user_confirmation")]
    pub require_user_confirmation: bool,
}

fn default_require_user_confirmation() -> bool {
    true
}

/// Network policy for plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicy {
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    #[serde(default = "default_block_all_outbound")]
    pub block_all_outbound: bool,
}

fn default_block_all_outbound() -> bool {
    true
}

impl PluginManifest {
    /// Validate manifest
    pub fn validate(&self) -> Result<(), String> {
        if self.plugin_id.is_empty() {
            return Err("plugin_id is required".to_string());
        }
        if self.name.is_empty() {
            return Err("name is required".to_string());
        }
        if self.version.is_empty() {
            return Err("version is required".to_string());
        }
        if self.required_fields.is_empty() && self.optional_fields.is_empty() {
            return Err(
                "at least one of required_fields or optional_fields must be specified".to_string(),
            );
        }
        Ok(())
    }

    /// Check if a field is requested
    pub fn is_field_requested(&self, field: &str) -> bool {
        self.required_fields
            .iter()
            .any(|f| Self::field_matches(f, field))
            || self
                .optional_fields
                .iter()
                .any(|f| Self::field_matches(f, field))
    }

    /// Check if field pattern matches
    fn field_matches(pattern: &str, field: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        if pattern == field {
            return true;
        }
        // Handle wildcards like "identity.*"
        if pattern.ends_with(".*") {
            let prefix = &pattern[..pattern.len() - 2];
            return field.starts_with(prefix) || field.starts_with(&prefix.replace(".*", ""));
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_manifest() {
        let manifest = PluginManifest {
            plugin_id: "com.example.test".to_string(),
            name: "Test Plugin".to_string(),
            version: "1.0.0".to_string(),
            required_fields: vec!["identity.full_name".to_string()],
            optional_fields: vec![],
            network_policy: None,
            data_ttl_seconds: 300,
            require_user_confirmation: true,
        };

        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn test_field_matching() {
        let manifest = PluginManifest {
            plugin_id: "com.example.test".to_string(),
            name: "Test".to_string(),
            version: "1.0.0".to_string(),
            required_fields: vec!["identity.full_name".to_string(), "travel.*".to_string()],
            optional_fields: vec![],
            network_policy: None,
            data_ttl_seconds: 300,
            require_user_confirmation: true,
        };

        assert!(manifest.is_field_requested("identity.full_name"));
        assert!(manifest.is_field_requested("travel.passports"));
        assert!(!manifest.is_field_requested("financial.bank_accounts"));
    }
}
