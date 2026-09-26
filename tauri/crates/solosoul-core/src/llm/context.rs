//! 普通聊天自动上下文的纯数据投影（RF-004）。
//!
//! Host 负责绑定会话、读取 Vault、执行保存的开关，以及包装系统提示和指南。
//! 本模块只从候选数据提取明确允许公开的叶值，不读取 UI 状态或执行 I/O。

use serde_json::{Map, Value};
use solosoul_vault::{ObjectRecord, TemplateProperty, UserTemplate};
use std::collections::HashMap;

const MAX_OBJECTS: usize = 3;
const MAX_OBJECT_LEAVES: usize = 8;
const MAX_VALUE_CHARS: usize = 100;
const MAX_GROUP_DEPTH: usize = 16;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct LlmContextProjection {
    pub object_lines: Vec<String>,
    pub preference_lines: Vec<String>,
}

/// 按调用方的候选顺序投影，候选 ID 本身不构成出站授权。
///
/// 每个对象再次校验账户、删除状态和对象级敏感度；模板只能来自同一账户。
/// 属性为空时仍保留公开对象身份，空数据和偏好的展示占位由 Host 添加。
/// 值最多保留 100 个 Unicode scalar，超长时附加省略标记；不改写输入数据。
pub fn project_context(
    account_id: &str,
    ordered_objects: &[ObjectRecord],
    templates: &HashMap<String, UserTemplate>,
    preferences: Option<&Map<String, Value>>,
) -> LlmContextProjection {
    let object_lines = ordered_objects
        .iter()
        .filter(|object| {
            object.account_id == account_id
                && !object.is_deleted
                && object.sensitivity_level == "public"
        })
        .take(MAX_OBJECTS)
        .map(|object| {
            let template = object
                .template_id
                .as_ref()
                .and_then(|id| templates.get(id).filter(|template| template.id == *id))
                .filter(|template| template.account_id == account_id);
            project_object(object, template)
        })
        .collect();

    let preference_lines = preferences
        .map(|preferences| {
            ["theme", "language", "accentColor", "autoLockTimeoutMinutes"]
                .into_iter()
                .filter_map(|key| {
                    let value = public_preference_value(key, preferences.get(key)?)?;
                    Some(format!("{key}: {value}"))
                })
                .collect()
        })
        .unwrap_or_default();

    LlmContextProjection {
        object_lines,
        preference_lines,
    }
}

fn project_object(object: &ObjectRecord, template: Option<&UserTemplate>) -> String {
    let mut entries = Vec::new();
    if let Some(properties) = object.properties.as_object() {
        let labels = object.property_labels.as_ref().and_then(Value::as_object);
        let definitions = properties.get("__fields").and_then(Value::as_object);
        for (key, value) in properties {
            if entries.len() == MAX_OBJECT_LEAVES {
                break;
            }
            let definition = definitions.and_then(|fields| fields.get(key)?.as_object());
            let property = template.and_then(|template| {
                template
                    .properties
                    .iter()
                    .find(|property| property.id == *key)
            });
            append_public_entries(
                key,
                value,
                is_dynamic_group(definition, property),
                is_public_field(key, labels, definition, property),
                0,
                &mut entries,
            );
        }
    }
    format!(
        "{}（{}）：{}",
        object.type_id,
        object.name,
        entries.join("、")
    )
}

/// get() 保留“键缺失”和“键存在但为 null/非法值”的区别，显式值不能回退。
fn is_public_field(
    key: &str,
    labels: Option<&Map<String, Value>>,
    definition: Option<&Map<String, Value>>,
    property: Option<&TemplateProperty>,
) -> bool {
    if let Some(value) = labels.and_then(|labels| labels.get(key)) {
        return value.as_str() == Some("public");
    }
    if let Some(value) = definition.and_then(|definition| definition.get("sensitivityLevel")) {
        return value.as_str() == Some("public");
    }
    property.and_then(|property| property.sensitivity_level.as_deref()) == Some("public")
}

fn is_dynamic_group(
    definition: Option<&Map<String, Value>>,
    property: Option<&TemplateProperty>,
) -> bool {
    // 与对象定义的 `type ?? template.type` 一致：null 可以回退，其他显式类型不能。
    match definition.and_then(|definition| definition.get("type")) {
        Some(value) if !value.is_null() => value.as_str() == Some("dynamic_group"),
        _ => property.is_some_and(|property| property.prop_type.as_str() == "dynamic_group"),
    }
}

fn append_public_entries(
    key: &str,
    value: &Value,
    dynamic_group: bool,
    public: bool,
    depth: usize,
    entries: &mut Vec<String>,
) {
    if !public
        || key.starts_with("__")
        || depth > MAX_GROUP_DEPTH
        || entries.len() == MAX_OBJECT_LEAVES
    {
        return;
    }
    if dynamic_group {
        let parsed_group;
        let children = match value {
            Value::Array(children) => children,
            Value::String(raw) => {
                parsed_group = match serde_json::from_str::<Value>(raw) {
                    Ok(parsed) => parsed,
                    Err(_) => return,
                };
                let Some(children) = parsed_group.as_array() else {
                    return;
                };
                children
            }
            _ => return,
        };
        for child in children {
            if entries.len() == MAX_OBJECT_LEAVES {
                break;
            }
            let Some(child) = child.as_object() else {
                continue;
            };
            if child
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| id.starts_with("__"))
            {
                continue;
            }
            let Some(name) = child
                .get("name")
                .and_then(Value::as_str)
                .or_else(|| child.get("id").and_then(Value::as_str))
                .filter(|name| !name.is_empty() && !name.starts_with("__"))
            else {
                continue;
            };
            let Some(value) = child.get("value") else {
                continue;
            };
            // 只有 public 父级会进入本分支，子级必须独立声明 public，不能降低保护。
            append_public_entries(
                &format!("{key}.{name}"),
                value,
                is_dynamic_group(Some(child), None),
                is_public_field(name, None, Some(child), None),
                depth + 1,
                entries,
            );
        }
        return;
    }

    if let Some(text) = public_leaf_text(value) {
        entries.push(format!("{key}: {}", truncate_value(&text)));
    }
}

fn scalar_text(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn public_leaf_text(value: &Value) -> Option<String> {
    if let Value::Array(values) = value {
        // 必须验证整个数组；不能在截断点后藏入未审核的对象或 null。
        values
            .iter()
            .map(scalar_text)
            .collect::<Option<Vec<_>>>()
            .map(|values| values.join(", "))
    } else {
        scalar_text(value)
    }
}

fn truncate_value(value: &str) -> String {
    let mut chars = value.chars();
    let mut result: String = chars.by_ref().take(MAX_VALUE_CHARS).collect();
    if chars.next().is_some() {
        result.push('…');
    }
    result
}

fn public_preference_value(key: &str, value: &Value) -> Option<String> {
    match key {
        "theme" => value
            .as_str()
            .filter(|value| matches!(*value, "light" | "dark" | "system"))
            .map(str::to_owned),
        "accentColor" => value
            .as_str()
            .filter(|value| {
                matches!(
                    *value,
                    "ocean" | "amber" | "forest" | "rose" | "purple" | "custom"
                )
            })
            .map(str::to_owned),
        "language" => value
            .as_str()
            .filter(|value| {
                !value.is_empty()
                    && value.len() <= MAX_VALUE_CHARS
                    && value.starts_with(|character: char| character.is_ascii_alphabetic())
                    && value.chars().all(|character| {
                        character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
                    })
            })
            .map(str::to_owned),
        "autoLockTimeoutMinutes" => value
            .as_f64()
            .filter(|minutes| minutes.is_finite() && *minutes > 0.0)
            .and_then(|_| scalar_text(value))
            .map(|value| truncate_value(&value)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use solosoul_vault::PropertyType;

    fn object(properties: Value) -> ObjectRecord {
        ObjectRecord {
            id: "object".into(),
            account_id: "account".into(),
            type_id: "person".into(),
            name: "公开对象".into(),
            sensitivity_level: "public".into(),
            properties,
            ..Default::default()
        }
    }

    fn property(id: &str, sensitivity: Option<&str>) -> TemplateProperty {
        TemplateProperty {
            id: id.into(),
            name: id.into(),
            prop_type: PropertyType::Text,
            sensitivity_level: sensitivity.map(str::to_owned),
            sensitive: None,
            options: None,
            deprecated_at: None,
            contract_field: None,
            contract_bindings: None,
            allowed_types: None,
            max_items: None,
        }
    }

    fn template(properties: Vec<TemplateProperty>) -> UserTemplate {
        UserTemplate {
            id: "template".into(),
            account_id: "account".into(),
            name: "模板".into(),
            icon_id: None,
            properties,
            category: None,
            created_at: String::new(),
            updated_at: None,
            contract_type_id: None,
        }
    }

    fn projected_text(
        objects: &[ObjectRecord],
        templates: &HashMap<String, UserTemplate>,
    ) -> String {
        project_context("account", objects, templates, None)
            .object_lines
            .join("\n")
    }

    #[test]
    fn rf004_only_explicitly_public_fields_without_internal_metadata() {
        let mut object = object(json!({
            "a": "ALLOW", "b": "SECRET_INTERNAL", "c": "SECRET_SENSITIVE",
            "d": "SECRET_CRITICAL", "e": "SECRET_MISSING", "f": "SECRET_INVALID",
            "__meta": "SECRET_META"
        }));
        object.property_labels = Some(json!({
            "a": "public", "b": "internal", "c": "sensitive", "d": "critical",
            "f": "invalid", "__meta": "public"
        }));
        let before = serde_json::to_value(&object).unwrap();
        let result = projected_text(std::slice::from_ref(&object), &HashMap::new());
        assert!(result.contains("ALLOW"));
        assert!(!result.contains("SECRET_"));
        assert!(!result.contains("__meta"));
        assert_eq!(serde_json::to_value(&object).unwrap(), before);
    }

    #[test]
    fn rf004_labels_precede_definitions_precede_templates() {
        let template = template(vec![
            property("a", Some("public")),
            property("b", Some("public")),
            property("c", Some("public")),
            property("d", Some("internal")),
        ]);
        let mut object = object(json!({
            "a": "SECRET_LABEL", "b": "SECRET_DEFINITION",
            "c": "ALLOW_TEMPLATE", "d": "ALLOW_LABEL",
            "__fields": {
                "a": {"sensitivityLevel": "public"},
                "b": {"sensitivityLevel": "invalid"},
                "d": {"sensitivityLevel": "critical"}
            }
        }));
        object.template_id = Some("template".into());
        object.property_labels = Some(json!({"a": "", "d": "public"}));
        let result = projected_text(&[object], &HashMap::from([("template".into(), template)]));
        assert!(!result.contains("SECRET_"));
        assert!(result.contains("ALLOW_TEMPLATE"));
        assert!(result.contains("ALLOW_LABEL"));
    }

    #[test]
    fn rf004_deleted_template_keeps_persisted_definitions() {
        let mut object = object(json!({
            "a": "ALLOW_SNAPSHOT", "b": "SECRET_UNKNOWN",
            "__fields": {"a": {"sensitivityLevel": "public"}}
        }));
        object.template_id = Some("deleted".into());
        let result = projected_text(&[object], &HashMap::new());
        assert!(result.contains("ALLOW_SNAPSHOT"));
        assert!(!result.contains("SECRET_"));
    }

    #[test]
    fn rf004_dynamic_groups_recursively_preserve_parent_protection() {
        let children = json!([
            {"name": "ok", "value": "ALLOW_CHILD", "sensitivityLevel": "public"},
            {"name": "private", "value": "SECRET_CHILD", "sensitivityLevel": "critical"},
            {"name": "unknown", "value": "SECRET_MISSING"},
            {"name": "invalid", "value": "SECRET_INVALID", "sensitivityLevel": "invalid"},
            {"name": "__metadata", "value": "SECRET_METADATA", "sensitivityLevel": "public"},
            {"name": "nested", "type": "dynamic_group", "sensitivityLevel": "public", "value": [
                {"name": "ok", "value": "ALLOW_NESTED", "sensitivityLevel": "public"},
                {"name": "hidden", "value": "SECRET_NESTED", "sensitivityLevel": "internal"}
            ]},
            {"name": "locked", "type": "dynamic_group", "sensitivityLevel": "sensitive", "value": [
                {"name": "ok", "value": "SECRET_PARENT", "sensitivityLevel": "public"}
            ]}
        ]);
        for value in [children.clone(), Value::String(children.to_string())] {
            let object = object(json!({
                "group": value, "hidden": children,
                "__fields": {
                    "group": {"type": "dynamic_group", "sensitivityLevel": "public"},
                    "hidden": {"type": "dynamic_group", "sensitivityLevel": "critical"}
                }
            }));
            let before = object.properties.clone();
            let result = projected_text(std::slice::from_ref(&object), &HashMap::new());
            assert!(result.contains("group.ok: ALLOW_CHILD"));
            assert!(result.contains("group.nested.ok: ALLOW_NESTED"));
            assert!(!result.contains("SECRET_"));
            assert_eq!(object.properties, before);
        }
    }

    #[test]
    fn rf004_unknown_structures_and_malformed_groups_are_not_serialized() {
        let mut object = object(json!({
            "a": {"nested": "SECRET_OBJECT"}, "b": [{"value": "SECRET_ARRAY"}],
            "c": "SECRET_MALFORMED", "d": ["ALLOW_ARRAY", 0, false],
            "e": null, "f": ["SECRET_MIXED", null], "g": [],
            "__fields": {"c": {"type": "dynamic_group"}}
        }));
        object.property_labels = Some(json!({
            "a": "public", "b": "public", "c": "public", "d": "public",
            "e": "public", "f": "public", "g": "public"
        }));
        let result = projected_text(&[object], &HashMap::new());
        assert!(!result.contains("SECRET_"));
        assert!(result.contains("d: ALLOW_ARRAY, 0, false"));
        assert!(result.ends_with("g: "));
    }

    #[test]
    fn rf004_filtering_precedes_field_limits_and_respects_object_restrictions() {
        let mut properties: Map<String, Value> = (0..9)
            .map(|index| (format!("hidden{index}"), json!(format!("SECRET_{index}"))))
            .collect();
        properties.insert("visible".into(), json!("ALLOW_AFTER_FILTER"));
        let mut allowed = object(Value::Object(properties));
        allowed.property_labels = Some(json!({"visible": "public"}));
        let mut deleted = object(json!({}));
        deleted.name = "SECRET_DELETED".into();
        deleted.is_deleted = true;
        let mut private = object(json!({}));
        private.name = "SECRET_OBJECT".into();
        private.sensitivity_level = "internal".into();
        let result = projected_text(&[allowed, deleted, private], &HashMap::new());
        assert!(result.contains("ALLOW_AFTER_FILTER"));
        assert!(!result.contains("SECRET_"));
    }

    #[test]
    fn rf004_account_and_object_limit_keep_candidate_order() {
        let mut foreign = object(json!({}));
        foreign.account_id = "other".into();
        foreign.name = "SECRET_FOREIGN".into();
        let mut invalid = object(json!({}));
        invalid.sensitivity_level = "unknown".into();
        invalid.name = "SECRET_UNKNOWN".into();
        let mut objects = vec![foreign, invalid];
        for name in ["third", "first", "second", "SECRET_LIMIT"] {
            let mut candidate = object(json!({}));
            candidate.name = name.into();
            objects.push(candidate);
        }
        let result = project_context("account", &objects, &HashMap::new(), None);
        assert_eq!(
            result.object_lines,
            [
                "person（third）：",
                "person（first）：",
                "person（second）："
            ]
        );
    }

    #[test]
    fn rf004_foreign_or_mismatched_template_cannot_authorize_fields() {
        for (template_account, template_id) in [("other", "template"), ("account", "wrong-id")] {
            let mut template = template(vec![property("a", Some("public"))]);
            template.account_id = template_account.into();
            template.id = template_id.into();
            let mut object = object(json!({
                "a": "SECRET_TEMPLATE", "b": "ALLOW_DEFINITION",
                "__fields": {"b": {"sensitivityLevel": "public"}}
            }));
            object.template_id = Some("template".into());
            let result = projected_text(&[object], &HashMap::from([("template".into(), template)]));
            assert!(!result.contains("SECRET_"));
            assert!(result.contains("ALLOW_DEFINITION"));
        }
    }

    #[test]
    fn rf004_explicit_null_and_invalid_sensitivity_never_fall_back() {
        for invalid in [
            Value::Null,
            json!(""),
            json!("PUBLIC"),
            json!(true),
            json!([]),
            json!({}),
        ] {
            let template = template(vec![
                property("label", Some("public")),
                property("definition", Some("public")),
            ]);
            let mut object = object(json!({
                "label": "SECRET_LABEL", "definition": "SECRET_DEFINITION",
                "__fields": {"label": {"sensitivityLevel": "public"}, "definition": {"sensitivityLevel": invalid}}
            }));
            object.template_id = Some("template".into());
            object.property_labels = Some(json!({"label": invalid}));
            assert!(
                !projected_text(&[object], &HashMap::from([("template".into(), template)]))
                    .contains("SECRET_")
            );
        }
    }

    #[test]
    fn rf004_dynamic_internal_ids_names_and_missing_names_are_rejected() {
        let object = object(json!({
            "group": [
                {"id": "__hidden", "name": "visible", "value": "SECRET_ID", "sensitivityLevel": "public"},
                {"id": "visible", "name": "__hidden", "value": "SECRET_NAME", "sensitivityLevel": "public"},
                {"id": "__fallback", "value": "SECRET_FALLBACK", "sensitivityLevel": "public"},
                {"id": "visible", "name": "", "value": "SECRET_EMPTY", "sensitivityLevel": "public"},
                {"value": "SECRET_MISSING", "sensitivityLevel": "public"},
                {"id": "fallback", "value": "ALLOW_ID", "sensitivityLevel": "public"},
                {"name": "zero", "value": 0, "sensitivityLevel": "public"},
                {"name": "false", "value": false, "sensitivityLevel": "public"},
                {"name": "empty", "value": "", "sensitivityLevel": "public"}
            ],
            "__fields": {"group": {"type": "dynamic_group", "sensitivityLevel": "public"}}
        }));
        let result = projected_text(&[object], &HashMap::new());
        assert!(!result.contains("SECRET_"));
        assert!(result.contains("group.fallback: ALLOW_ID"));
        assert!(result.contains("group.zero: 0"));
        assert!(result.contains("group.false: false"));
        assert!(result.ends_with("group.empty: "));
    }

    #[test]
    fn rf004_dynamic_depth_is_bounded_at_sixteen() {
        fn children(nesting: usize, marker: &str) -> Value {
            if nesting == 0 {
                json!([{"name": "leaf", "value": marker, "sensitivityLevel": "public"}])
            } else {
                json!([{"name": "nested", "type": "dynamic_group", "value": children(nesting - 1, marker), "sensitivityLevel": "public"}])
            }
        }
        let object = object(json!({
            "allowed": children(15, "ALLOW_DEPTH_16"),
            "hidden": children(16, "SECRET_DEPTH_17"),
            "__fields": {
                "allowed": {"type": "dynamic_group", "sensitivityLevel": "public"},
                "hidden": {"type": "dynamic_group", "sensitivityLevel": "public"}
            }
        }));
        let result = projected_text(&[object], &HashMap::new());
        assert!(result.contains("ALLOW_DEPTH_16"));
        assert!(!result.contains("SECRET_"));
    }

    #[test]
    fn rf004_dynamic_type_uses_nullish_definition_fallback() {
        let mut dynamic = property("group", Some("public"));
        dynamic.prop_type = PropertyType::DynamicGroup;
        let template = template(vec![dynamic]);
        let templates = HashMap::from([("template".into(), template)]);
        let mut object = object(json!({
            "group": [{"name": "leaf", "value": "ALLOW_TEMPLATE_TYPE", "sensitivityLevel": "public"}],
            "__fields": {"group": {"type": null}}
        }));
        object.template_id = Some("template".into());
        assert!(projected_text(std::slice::from_ref(&object), &templates)
            .contains("ALLOW_TEMPLATE_TYPE"));
        object.properties["__fields"]["group"]["type"] = json!("unknown");
        assert!(!projected_text(&[object], &templates).contains("ALLOW_TEMPLATE_TYPE"));
    }

    #[test]
    fn rf004_field_values_limit_unicode_scalars_after_filtering() {
        let unicode = "界😀".repeat(50);
        let unicode_object = object(json!({
            "exact": unicode,
            "long": format!("{unicode}SECRET_TAIL"),
            "__fields": {
                "exact": {"sensitivityLevel": "public"},
                "long": {"sensitivityLevel": "public"}
            }
        }));
        let result = projected_text(&[unicode_object], &HashMap::new());
        assert_eq!(
            result,
            format!("person（公开对象）：exact: {unicode}、long: {unicode}…")
        );
        // 独立夹具使此断言不依赖 serde_json preserve_order 的 feature 合并结果。
        let mut limited_object = object(json!({"__fields": {}}));
        for index in 0..9 {
            let key = format!("a{index}");
            limited_object.properties[&key] = json!(format!("ALLOW_{index}"));
            limited_object.properties["__fields"][&key] = json!({"sensitivityLevel": "public"});
        }
        let result = projected_text(&[limited_object], &HashMap::new());
        assert_eq!(result.matches("ALLOW_").count(), 8);
        assert!(!result.contains("ALLOW_8"));
    }

    #[test]
    fn rf004_preferences_project_only_four_well_typed_keys() {
        let preferences = json!({
            "theme": "dark", "language": "zh-CN", "accentColor": "custom", "autoLockTimeoutMinutes": 5,
            "customAccentHex": "SECRET_HEX", "llmConfig": {"apiKey": "SECRET_KEY"},
            "__metadata": "SECRET_METADATA", "other": "SECRET_OTHER"
        });
        let before = preferences.clone();
        let result = project_context("account", &[], &HashMap::new(), preferences.as_object());
        assert_eq!(
            result.preference_lines,
            [
                "theme: dark",
                "language: zh-CN",
                "accentColor: custom",
                "autoLockTimeoutMinutes: 5"
            ]
        );
        assert_eq!(preferences, before);
        assert!(result.object_lines.is_empty());
    }

    #[test]
    fn rf004_preferences_reject_structures_unknown_values_and_unbounded_locale() {
        for preferences in [
            json!({"theme": {"value": "SECRET"}, "language": ["SECRET"], "accentColor": false, "autoLockTimeoutMinutes": "5"}),
            json!({"theme": "SECRET", "language": "zh-CN\nSECRET", "accentColor": "SECRET", "autoLockTimeoutMinutes": -1}),
            json!({"theme": null, "language": "a".repeat(101), "accentColor": null, "autoLockTimeoutMinutes": 0}),
        ] {
            assert!(
                project_context("account", &[], &HashMap::new(), preferences.as_object())
                    .preference_lines
                    .is_empty()
            );
        }
        assert_eq!(
            project_context("account", &[], &HashMap::new(), None),
            LlmContextProjection::default()
        );
    }
}
