//! System template registry — loads built-in object templates from an embedded
//! JSON resource file (`resources/system_templates.json`).
//!
//! Templates are loaded once at application startup and cached in memory.
//! They are read-only; users cannot modify system templates, only hide them.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Data types (mirrors frontend expectations)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemTemplateProperty {
    pub id: String,
    pub name_i18n_key: String,
    pub name_fallback: String,
    #[serde(rename = "type")]
    pub prop_type: String,
    pub sensitive: Option<bool>,
    pub required: Option<bool>,
    pub options: Option<Vec<String>>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SystemTemplateResource {
    version: u32,
    templates: Vec<SystemTemplate>,
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

static REGISTRY: OnceLock<SystemTemplateRegistry> = OnceLock::new();

pub struct SystemTemplateRegistry {
    templates: HashMap<String, SystemTemplate>,
    version: u32,
}

impl SystemTemplateRegistry {
    /// Load templates from the embedded JSON resource.
    pub fn load() -> Result<Self, String> {
        let json_str = include_str!("../../resources/system_templates.json");
        let data: SystemTemplateResource =
            serde_json::from_str(json_str).map_err(|e| format!("Parse system_templates.json: {}", e))?;

        let mut templates = HashMap::new();
        for tpl in data.templates {
            templates.insert(tpl.key.clone(), tpl);
        }

        Ok(Self {
            templates,
            version: data.version,
        })
    }

    /// Initialise the global registry (idempotent).
    pub fn init() -> Result<(), String> {
        if REGISTRY.get().is_some() {
            return Ok(());
        }
        let registry = Self::load()?;
        REGISTRY.set(registry).map_err(|_| "Registry already initialised".to_string())
    }

    fn global() -> Option<&'static Self> {
        REGISTRY.get()
    }

    // -- Public query API ---------------------------------------------------

    pub fn get(key: &str) -> Option<SystemTemplate> {
        Self::global()?.templates.get(key).cloned()
    }

    pub fn list_all() -> Vec<SystemTemplate> {
        Self::global()
            .map(|r| r.templates.values().cloned().collect())
            .unwrap_or_default()
    }

    pub fn list_by_category(category: &str) -> Vec<SystemTemplate> {
        Self::global()
            .map(|r| {
                r.templates
                    .values()
                    .filter(|t| t.category == category)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn detect_for_object(properties: &serde_json::Map<String, serde_json::Value>) -> Option<String> {
        let registry = Self::global()?;
        let keys: std::collections::HashSet<&str> = properties.keys().map(|k| k.as_str()).collect();

        let mut best_match: Option<(String, usize)> = None;
        for (key, tpl) in &registry.templates {
            let tpl_keys: std::collections::HashSet<&str> =
                tpl.properties.iter().map(|p| p.id.as_str()).collect();
            let match_count = keys.intersection(&tpl_keys).count();
            let threshold = (tpl_keys.len() as f32 * 0.5).ceil() as usize;

            if match_count >= threshold {
                if best_match.as_ref().map_or(true, |(_, count)| match_count > *count) {
                    best_match = Some((key.clone(), match_count));
                }
            }
        }

        best_match.map(|(key, _)| key)
    }

    pub fn version() -> Option<u32> {
        Self::global().map(|r| r.version)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_system_templates() {
        let registry = SystemTemplateRegistry::load().unwrap();
        assert!(!registry.templates.is_empty());
        assert!(registry.templates.contains_key("passport"));
        assert!(registry.templates.contains_key("bank"));
    }

    #[test]
    fn test_get_template() {
        let _ = SystemTemplateRegistry::init();
        let tpl = SystemTemplateRegistry::get("passport");
        assert!(tpl.is_some());
        let tpl = tpl.unwrap();
        assert_eq!(tpl.key, "passport");
        assert_eq!(tpl.category, "travel");
        assert!(!tpl.properties.is_empty());
    }

    #[test]
    fn test_list_by_category() {
        let _ = SystemTemplateRegistry::init();
        let travel = SystemTemplateRegistry::list_by_category("travel");
        assert!(!travel.is_empty());
        assert!(travel.iter().any(|t| t.key == "passport"));
        assert!(travel.iter().any(|t| t.key == "visa"));
    }

    #[test]
    fn test_detect_for_object() {
        let _ = SystemTemplateRegistry::init();
        let mut props = serde_json::Map::new();
        props.insert("fullName".to_string(), serde_json::json!("张三"));
        props.insert("passportNumber".to_string(), serde_json::json!("E12345678"));
        props.insert("nationality".to_string(), serde_json::json!("CN"));
        props.insert("expiryDate".to_string(), serde_json::json!("2030-01-01"));

        let detected = SystemTemplateRegistry::detect_for_object(&props);
        assert_eq!(detected, Some("passport".to_string()));
    }

    #[test]
    fn test_detect_no_match() {
        let _ = SystemTemplateRegistry::init();
        let mut props = serde_json::Map::new();
        props.insert("foo".to_string(), serde_json::json!("bar"));
        props.insert("baz".to_string(), serde_json::json!("qux"));

        let detected = SystemTemplateRegistry::detect_for_object(&props);
        assert!(detected.is_none());
    }
}
