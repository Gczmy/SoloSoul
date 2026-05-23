//! Plugin manifest - Plugin configuration and permissions

use serde::{Deserialize, Serialize};

/// Plugin manifest (manifest.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub plugin_id: String,
    pub name: String,
    pub version: String,
    /// 插件 ABI 版本，与主软件严格匹配
    pub plugin_api_version: String,
    /// 兼容的最低 SoloSoul App 版本
    pub min_app_version: String,
    /// 兼容的最高 SoloSoul App 版本
    pub max_app_version: String,
    pub description: String,
    pub publisher: String,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub signature: Option<String>,
    #[serde(default)]
    pub required_fields: Vec<String>,
    #[serde(default)]
    pub optional_fields: Vec<String>,
    #[serde(default)]
    pub network_policy: Option<NetworkPolicy>,
    #[serde(default = "default_data_ttl_seconds")]
    pub data_ttl_seconds: u64,
    #[serde(default = "default_require_user_confirmation")]
    pub require_user_confirmation: bool,
    #[serde(default = "default_consent_validity_hours")]
    pub consent_validity_hours: u64,
}

fn default_data_ttl_seconds() -> u64 {
    300
}

fn default_require_user_confirmation() -> bool {
    true
}

fn default_consent_validity_hours() -> u64 {
    24
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

impl NetworkPolicy {
    /// 检查域名是否匹配白名单
    /// 支持精确匹配和通配符前缀（如 `*.solosoul.dev`）
    pub fn allows_domain(&self, host: &str) -> bool {
        if self.block_all_outbound && self.allowed_domains.is_empty() {
            return false;
        }
        self.allowed_domains.iter().any(|pattern| {
            if pattern.starts_with("*.") {
                let suffix = &pattern[2..];
                host == suffix || host.ends_with(&format!(".{}", suffix))
            } else {
                host == pattern
            }
        })
    }
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
        if self.plugin_api_version.is_empty() {
            return Err("plugin_api_version is required".to_string());
        }
        if self.min_app_version.is_empty() {
            return Err("min_app_version is required".to_string());
        }
        if self.max_app_version.is_empty() {
            return Err("max_app_version is required".to_string());
        }
        if self.description.is_empty() {
            return Err("description is required".to_string());
        }
        if self.publisher.is_empty() {
            return Err("publisher is required".to_string());
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
            return field.starts_with(prefix);
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_manifest() -> PluginManifest {
        PluginManifest {
            plugin_id: "com.example.test".to_string(),
            name: "Test Plugin".to_string(),
            version: "1.0.0".to_string(),
            plugin_api_version: "1.0".to_string(),
            min_app_version: "1.0.0".to_string(),
            max_app_version: "2.0.0".to_string(),
            description: "A test plugin".to_string(),
            publisher: "Test Publisher".to_string(),
            homepage: None,
            signature: None,
            required_fields: vec!["identity.full_name".to_string()],
            optional_fields: vec![],
            network_policy: None,
            data_ttl_seconds: 300,
            require_user_confirmation: true,
            consent_validity_hours: 24,
        }
    }

    #[test]
    fn test_validate_manifest() {
        assert!(test_manifest().validate().is_ok());
    }

    #[test]
    fn test_validate_missing_plugin_api_version() {
        let mut m = test_manifest();
        m.plugin_api_version = "".to_string();
        assert!(m.validate().is_err());
    }

    #[test]
    fn test_field_matching() {
        let manifest = PluginManifest {
            required_fields: vec!["identity.full_name".to_string(), "travel.*".to_string()],
            ..test_manifest()
        };

        assert!(manifest.is_field_requested("identity.full_name"));
        assert!(manifest.is_field_requested("travel.passports"));
        assert!(!manifest.is_field_requested("financial.bank_accounts"));
    }

    #[test]
    fn test_network_policy_allows_domain() {
        let policy = NetworkPolicy {
            block_all_outbound: true,
            allowed_domains: vec![
                "api.solosoul.dev".to_string(),
                "*.solosoul.dev".to_string(),
            ],
        };

        assert!(policy.allows_domain("api.solosoul.dev"));
        assert!(policy.allows_domain("cdn.solosoul.dev"));
        assert!(!policy.allows_domain("evil.com"));
    }

    #[test]
    fn test_network_policy_block_all() {
        let policy = NetworkPolicy {
            block_all_outbound: true,
            allowed_domains: vec![],
        };
        assert!(!policy.allows_domain("any.com"));
    }
}
