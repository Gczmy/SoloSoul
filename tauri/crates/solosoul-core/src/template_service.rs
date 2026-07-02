//! Default template seed registry — loads built-in object templates from embedded
//! JSON resource files (`resources/system_templates_*.json`).
//!
//! These templates are **not** runtime system templates. They are seed data that
//! gets imported once into the user's vault as regular `UserTemplate`s during
//! account creation. After import, users can freely edit or delete them.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Data types (seed format — mirrors the JSON structure)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemTemplateProperty {
    pub id: String,
    pub name_i18n_key: String,
    pub name_fallback: String,
    #[serde(rename = "type")]
    pub prop_type: String,
    /// 4-tier sensitivity level: "public" | "internal" | "sensitive" | "critical".
    /// Replaces the legacy `sensitive` boolean.
    pub sensitivity_level: Option<String>,
    /// Legacy boolean — kept for backward-compat during deserialization only.
    #[serde(default, skip_serializing)]
    pub sensitive: Option<bool>,
    pub required: Option<bool>,
    pub options: Option<Vec<String>>,
    /// 插件合约字段映射 — 当此属性映射到插件合约中的字段时为 true（旧版）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_field: Option<bool>,
    /// 新版插件契约角色绑定。
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "contractBindings"
    )]
    pub contract_bindings: Option<Vec<solosoul_vault::ContractRoleBinding>>,
}

impl SystemTemplateProperty {
    /// Return the effective sensitivity level, migrating legacy `sensitive` boolean.
    pub fn effective_sensitivity_level(&self) -> String {
        self.sensitivity_level.clone().unwrap_or_else(|| {
            if self.sensitive.unwrap_or(false) {
                "sensitive".to_string()
            } else {
                "internal".to_string()
            }
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemTemplate {
    pub key: String,
    pub category: String,
    pub icon: String,
    pub name_i18n_key: String,
    pub name_fallback: String,
    pub properties: Vec<SystemTemplateProperty>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_type_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SystemTemplateResource {
    version: u32,
    templates: Vec<SystemTemplate>,
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

pub struct SystemTemplateRegistry {
    templates: HashMap<String, SystemTemplate>,
    version: u32,
}

impl SystemTemplateRegistry {
    /// Load templates from an embedded JSON resource for the given locale.
    /// `locale` should be a language code like "zh-CN" or "en-US".
    pub fn load_for_locale(locale: &str) -> Result<Self, String> {
        let json_str = if locale.starts_with("zh") {
            include_str!("../resources/system_templates_zh.json")
        } else {
            include_str!("../resources/system_templates_en.json")
        };
        let data: SystemTemplateResource =
            serde_json::from_str(json_str).map_err(|e| format!("Parse system_templates: {}", e))?;

        let mut templates = HashMap::new();
        for tpl in data.templates {
            templates.insert(tpl.key.clone(), tpl);
        }

        Ok(Self {
            templates,
            version: data.version,
        })
    }

    // -- Public query API ---------------------------------------------------

    pub fn get(&self, key: &str) -> Option<SystemTemplate> {
        self.templates.get(key).cloned()
    }

    pub fn list_all(&self) -> Vec<SystemTemplate> {
        self.templates.values().cloned().collect()
    }

    pub fn list_by_category(&self, category: &str) -> Vec<SystemTemplate> {
        self.templates
            .values()
            .filter(|t| t.category == category)
            .cloned()
            .collect()
    }

    pub fn version(&self) -> u32 {
        self.version
    }
}

// ---------------------------------------------------------------------------
// Seed import — convert SystemTemplates into UserTemplates and persist
// ---------------------------------------------------------------------------

/// Import default templates from the seed registry into the vault as regular
/// user templates. Called once during account creation.
pub fn seed_default_templates(
    vault: &solosoul_vault::VaultStore,
    account_id: &str,
    locale: &str,
) -> Result<(), String> {
    let registry = SystemTemplateRegistry::load_for_locale(locale)?;

    for st in registry.list_all() {
        let properties: Vec<solosoul_vault::TemplateProperty> = st
            .properties
            .iter()
            .map(|p| solosoul_vault::TemplateProperty {
                contract_field: p.contract_field,
                contract_bindings: p.contract_bindings.clone(),
                id: p.id.clone(),
                name: p.name_fallback.clone(),
                prop_type: solosoul_vault::PropertyType::parse(&p.prop_type)
                    .unwrap_or(solosoul_vault::PropertyType::Text),
                sensitivity_level: Some(p.effective_sensitivity_level()),
                options: p.options.clone(),
                sensitive: None,
                deprecated_at: None,
            })
            .collect();

        let now = chrono::Utc::now().to_rfc3339();
        let user_template = solosoul_vault::UserTemplate {
            contract_type_id: st.contract_type_id.clone(),
            id: st.key.clone(),
            account_id: account_id.to_string(),
            name: st.name_fallback.clone(),
            icon_id: Some(st.icon.clone()),
            properties,
            category: Some(st.category.clone()),
            created_at: now.clone(),
            updated_at: Some(now),
        };

        vault.save_user_template(&user_template)?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// detect_for_object — now a pure function over a slice of UserTemplates
// ---------------------------------------------------------------------------

pub fn detect_for_object(
    user_templates: &[solosoul_vault::UserTemplate],
    properties: &serde_json::Map<String, serde_json::Value>,
) -> Option<String> {
    let keys: std::collections::HashSet<&str> = properties.keys().map(|k| k.as_str()).collect();

    let mut best_match: Option<(String, usize)> = None;
    for tpl in user_templates {
        let tpl_keys: std::collections::HashSet<&str> =
            tpl.properties.iter().map(|p| p.id.as_str()).collect();
        let match_count = keys.intersection(&tpl_keys).count();
        let threshold = (tpl_keys.len() as f32 * 0.5).ceil() as usize;

        if match_count >= threshold
            && best_match
                .as_ref()
                .is_none_or(|(_, count)| match_count > *count)
        {
            best_match = Some((tpl.id.clone(), match_count));
        }
    }

    best_match.map(|(id, _)| id)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_zh_templates() {
        let registry = SystemTemplateRegistry::load_for_locale("zh-CN").unwrap();
        assert!(!registry.templates.is_empty());
        assert!(registry.templates.contains_key("passport"));
        let passport = registry.get("passport").unwrap();
        assert_eq!(passport.name_fallback, "护照");
    }

    #[test]
    fn test_load_en_templates() {
        let registry = SystemTemplateRegistry::load_for_locale("en-US").unwrap();
        assert!(!registry.templates.is_empty());
        assert!(registry.templates.contains_key("passport"));
        let passport = registry.get("passport").unwrap();
        assert_eq!(passport.name_fallback, "Passport");
    }

    #[test]
    fn test_list_by_category() {
        let registry = SystemTemplateRegistry::load_for_locale("en").unwrap();
        let travel = registry.list_by_category("travel");
        assert!(!travel.is_empty());
        assert!(travel.iter().any(|t| t.key == "passport"));
        assert!(travel.iter().any(|t| t.key == "visa"));
    }

    #[test]
    fn test_detect_for_object() {
        // Build a fake user template list to test detection
        let user_tpls = vec![solosoul_vault::UserTemplate {
            contract_type_id: None,
            id: "passport".to_string(),
            account_id: "acc_1".to_string(),
            name: "Passport".to_string(),
            icon_id: None,
            properties: vec![
                solosoul_vault::TemplateProperty {
                    contract_field: None,
                    contract_bindings: None,
                    id: "fullName".to_string(),
                    name: "Full Name".to_string(),
                    prop_type: solosoul_vault::PropertyType::Text,
                    sensitivity_level: None,
                    options: None,
                    sensitive: None,
                    deprecated_at: None,
                },
                solosoul_vault::TemplateProperty {
                    contract_field: None,
                    contract_bindings: None,
                    id: "passportNumber".to_string(),
                    name: "Passport Number".to_string(),
                    prop_type: solosoul_vault::PropertyType::Text,
                    sensitivity_level: None,
                    options: None,
                    sensitive: None,
                    deprecated_at: None,
                },
                solosoul_vault::TemplateProperty {
                    contract_field: None,
                    contract_bindings: None,
                    id: "nationality".to_string(),
                    name: "Nationality".to_string(),
                    prop_type: solosoul_vault::PropertyType::Text,
                    sensitivity_level: None,
                    options: None,
                    sensitive: None,
                    deprecated_at: None,
                },
                solosoul_vault::TemplateProperty {
                    contract_field: None,
                    contract_bindings: None,
                    id: "dateOfBirth".to_string(),
                    name: "Date of Birth".to_string(),
                    prop_type: solosoul_vault::PropertyType::Text,
                    sensitivity_level: None,
                    options: None,
                    sensitive: None,
                    deprecated_at: None,
                },
            ],
            category: Some("travel".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: None,
        }];

        let mut props = serde_json::Map::new();
        props.insert("fullName".to_string(), serde_json::json!("张三"));
        props.insert("passportNumber".to_string(), serde_json::json!("E12345678"));
        props.insert("nationality".to_string(), serde_json::json!("CN"));
        props.insert("expiryDate".to_string(), serde_json::json!("2030-01-01"));

        let detected = detect_for_object(&user_tpls, &props);
        assert_eq!(detected, Some("passport".to_string()));
    }

    #[test]
    fn test_detect_no_match() {
        let user_tpls: Vec<solosoul_vault::UserTemplate> = vec![];
        let mut props = serde_json::Map::new();
        props.insert("foo".to_string(), serde_json::json!("bar"));
        props.insert("baz".to_string(), serde_json::json!("qux"));

        let detected = detect_for_object(&user_tpls, &props);
        assert!(detected.is_none());
    }
}
