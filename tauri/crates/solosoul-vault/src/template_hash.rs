//! Template content hashing for deduplication.
//!
//! Computes a SHA-256 hash of a `UserTemplate`'s canonical JSON representation,
//! ignoring fields that vary by account or time (id, account_id, created_at, updated_at).
//!
//! This is the single source of truth for template equality comparison during import.
//! Every `UserTemplate` with the same content (name, icon_id, category, contract_type_id,
//! and all property fields) produces the same hash, regardless of its database ID.

use sha2::Digest;

use crate::{TemplateProperty, UserTemplate};

/// Collect template properties sorted by id, returning a JSON array of canonical property objects.
fn template_properties_sorted(tpl: &UserTemplate) -> Vec<serde_json::Value> {
    let mut sorted: Vec<&TemplateProperty> = tpl.properties.iter().collect();
    sorted.sort_by(|a, b| a.id.cmp(&b.id));
    sorted
        .iter()
        .map(|p| {
            serde_json::json!({
                "id": p.id,
                "name": p.name,
                "prop_type": p.prop_type,
                "sensitivity_level": p.sensitivity_level,
                "options": p.options,
                "contract_field": p.contract_field,
                "contract_bindings": p.contract_bindings,
            })
        })
        .collect()
}

/// Compute the content hash of a `UserTemplate`.
///
/// The hash covers: `name`, `icon_id`, `category`, `contract_type_id`,
/// and all properties (sorted by id). It intentionally **excludes**:
/// `id`, `account_id`, `created_at`, `updated_at`.
///
/// Two templates with identical content produce the same hash regardless of
/// their database IDs, making this suitable for deduplication during import.
pub fn user_template_content_hash(tpl: &UserTemplate) -> String {
    let sorted_props = template_properties_sorted(tpl);
    let canonical = serde_json::json!({
        "name": tpl.name,
        "icon_id": tpl.icon_id,
        "category": tpl.category,
        "contract_type_id": tpl.contract_type_id,
        "properties": sorted_props,
    });
    let bytes = serde_json::to_vec(&canonical).unwrap_or_default();
    hex::encode(sha2::Sha256::digest(&bytes))
}

/// Generate an imported template's local ID.
///
/// Format: `imported:<content_hash_prefix>:<original_id>`
/// This preserves provenance while preventing ID collisions.
pub fn imported_template_id(original_id: &str, content_hash: &str) -> String {
    let prefix = &content_hash[..content_hash.len().min(12)];
    format!("imported:{}:{}", prefix, original_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PropertyType;

    fn make_template(name: &str) -> UserTemplate {
        UserTemplate {
            contract_type_id: None,
            id: "ignored_id".to_string(),
            account_id: "ignored_account".to_string(),
            name: name.to_string(),
            icon_id: None,
            properties: vec![TemplateProperty {
                contract_field: None,
                contract_bindings: None,
                id: "f1".to_string(),
                name: "Field 1".to_string(),
                prop_type: PropertyType::Text,
                sensitivity_level: Some("internal".to_string()),
                options: None,
                sensitive: None,
                deprecated_at: None,
            }],
            category: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: None,
        }
    }

    #[test]
    fn test_same_content_same_hash() {
        let a = make_template("Test");
        let b = make_template("Test");
        assert_eq!(
            user_template_content_hash(&a),
            user_template_content_hash(&b)
        );
    }

    #[test]
    fn test_different_content_different_hash() {
        let a = make_template("Alpha");
        let b = make_template("Beta");
        assert_ne!(
            user_template_content_hash(&a),
            user_template_content_hash(&b)
        );
    }

    #[test]
    fn test_hash_ignores_id_and_account() {
        let mut a = make_template("Same");
        let mut b = make_template("Same");
        a.id = "id_A".to_string();
        b.id = "id_B".to_string();
        a.account_id = "acc_A".to_string();
        b.account_id = "acc_B".to_string();
        assert_eq!(
            user_template_content_hash(&a),
            user_template_content_hash(&b)
        );
    }

    #[test]
    fn test_imported_template_id_format() {
        let hash = "abcdef1234567890deadbeef";
        let id = imported_template_id("orig_tpl", hash);
        assert!(id.starts_with("imported:"));
        assert!(id.ends_with(":orig_tpl"));
        assert!(id.contains("abcdef123456"));
    }
}
