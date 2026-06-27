//! 搜索时的高敏感度字段过滤。
//!
//! 项目使用 4 级敏感度：public、internal、sensitive、critical。
//! 其中 sensitive 与 critical 的字段值默认会被 UI 掩码，因此也不应被搜索匹配到。

use std::collections::HashSet;

use crate::UserTemplate;

/// 需要排除在搜索匹配之外的字段敏感度等级。
pub const PROTECTED_SENSITIVITIES: &[&str] = &["sensitive", "critical"];

/// 判断给定敏感度是否属于受保护等级。
pub fn is_protected_sensitivity(level: &str) -> bool {
    PROTECTED_SENSITIVITIES.contains(&level.to_lowercase().as_str())
}

/// 根据对象的 `property_labels` 与模板定义，收集不应参与搜索的字段 key 集合。
///
/// 优先级：
/// 1. `property_labels` 中标记为 sensitive / critical 的字段 id。
/// 2. 若对象关联模板，回退到模板中 sensitivity_level 为 sensitive / critical 的 prop.id。
///
/// 返回的 key 与 `properties` 中的顶层字段 key 一致（例如 `"idNumber"`），
/// 因此可用于匹配 `search_properties_for_matches` 产生的 `field_path`。
pub fn collect_protected_field_keys(
    property_labels: Option<&serde_json::Value>,
    template_id: Option<&str>,
    templates: &std::collections::HashMap<String, UserTemplate>,
) -> HashSet<String> {
    let mut keys = HashSet::new();

    if let Some(labels) = property_labels.and_then(|v| v.as_object()) {
        for (key, val) in labels {
            if val.as_str().map_or(false, is_protected_sensitivity) {
                keys.insert(key.clone());
            }
        }
    }

    // 兜底：若对象缺少 property_labels（旧对象或模板创建后新增的敏感字段），
    // 从模板定义读取敏感度。
    if let Some(tid) = template_id {
        if let Some(tpl) = templates.get(tid) {
            for prop in &tpl.properties {
                if prop
                    .sensitivity_level
                    .as_deref()
                    .map_or(false, is_protected_sensitivity)
                {
                    keys.insert(prop.id.clone());
                }
            }
        }
    }

    keys
}

/// 判断某个字段值路径是否可以参与搜索匹配。
///
/// - 以 `__fields` 开头的路径是字段元数据（字段名、类型等），允许匹配，
///   这样用户仍可通过字段中文名找到对象。
/// - 路径中任意一段落在 `protected_keys` 中的字段值路径会被禁止匹配。
pub fn is_searchable_field_value(field_path: &str, protected_keys: &HashSet<String>) -> bool {
    if field_path.starts_with("__fields") {
        return true;
    }
    !field_path
        .split('.')
        .any(|seg| protected_keys.contains(seg))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn tpl(id: &str, level: &str) -> UserTemplate {
        UserTemplate {
            id: id.to_string(),
            account_id: "acc".to_string(),
            name: "T".to_string(),
            icon_id: None,
            properties: vec![crate::TemplateProperty {
                id: "idNumber".to_string(),
                name: "证件号码".to_string(),
                prop_type: crate::PropertyType::Text,
                sensitivity_level: Some(level.to_string()),
                sensitive: None,
                options: None,
                deprecated_at: None,
                contract_field: None,
            }],
            category: None,
            created_at: "now".to_string(),
            updated_at: None,
            contract_type_id: None,
        }
    }

    #[test]
    fn test_is_protected_sensitivity() {
        assert!(is_protected_sensitivity("critical"));
        assert!(is_protected_sensitivity("sensitive"));
        assert!(is_protected_sensitivity("CRITICAL"));
        assert!(!is_protected_sensitivity("internal"));
        assert!(!is_protected_sensitivity("public"));
    }

    #[test]
    fn test_collect_protected_keys_from_labels() {
        let labels = serde_json::json!({ "idNumber": "critical", "email": "internal" });
        let keys = collect_protected_field_keys(Some(&labels), None, &HashMap::new());
        assert!(keys.contains("idNumber"));
        assert!(!keys.contains("email"));
    }

    #[test]
    fn test_collect_protected_keys_fallback_to_template() {
        let mut templates = HashMap::new();
        templates.insert("identity".to_string(), tpl("identity", "critical"));
        let keys = collect_protected_field_keys(None, Some("identity"), &templates);
        assert!(keys.contains("idNumber"));
    }

    #[test]
    fn test_is_searchable_field_value() {
        let mut keys = HashSet::new();
        keys.insert("idNumber".to_string());

        assert!(!is_searchable_field_value("idNumber", &keys));
        assert!(!is_searchable_field_value("contact.idNumber", &keys));
        assert!(is_searchable_field_value("email", &keys));
        assert!(is_searchable_field_value("__fields.idNumber.name", &keys));
    }
}
