//! object 命令测试 —— template_sync（P047 拆分）

use super::super::*;
use super::setup_vault;
use solosoul_vault::{ObjectRecord, PropertyType, TemplateProperty, UserTemplate};

#[test]
fn test_template_fingerprint_stable_and_sensitive() {
    let tpl = UserTemplate {
        id: "tpl-1".to_string(),
        account_id: "acc-1".to_string(),
        name: "Contact".to_string(),
        icon_id: None,
        properties: vec![solosoul_vault::TemplateProperty {
            id: "name".to_string(),
            name: "Name".to_string(),
            prop_type: solosoul_vault::PropertyType::Text,
            sensitive: None,
            sensitivity_level: Some("internal".to_string()),
            options: None,
            deprecated_at: None,
            contract_field: None,
            contract_bindings: None,
            allowed_types: None,
            max_items: None,
        }],
        category: Some("identity".to_string()),
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: Some("2024-01-01T00:00:00Z".to_string()),
        contract_type_id: None,
    };

    let hash1 = template_fingerprint(&tpl);
    let hash2 = template_fingerprint(&tpl);
    assert_eq!(hash1, hash2);
    assert_eq!(hash1.len(), 16);

    let mut tpl_modified = tpl;
    // 指纹排除模板名称/图标/分类等元数据，只反映字段定义
    tpl_modified.name = "Contact Updated".to_string();
    let hash3 = template_fingerprint(&tpl_modified);
    assert_eq!(hash1, hash3);

    // 修改字段敏感度应改变指纹
    tpl_modified.properties[0].sensitivity_level = Some("sensitive".to_string());
    let hash4 = template_fingerprint(&tpl_modified);
    assert_ne!(hash1, hash4);
}

#[test]
fn test_compute_sync_changes_categorizes_fields() {
    let record = ObjectRecord {
        id: "obj-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "identity".to_string(),
        section_type: "identity".to_string(),
        name: "Test".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({
            "__fields": {
                "oldField": { "name": "Old Field", "type": "text" },
                "textField": { "name": "Text Field", "type": "text" },
                "numberField": { "name": "Number Field", "type": "number" }
            },
            "oldField": "old value",
            "textField": "hello",
            "numberField": 42
        }),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: Some("tpl-1".to_string()),
        template_type: Some("user".to_string()),
        contract_type_id: None,
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
        ..Default::default()
    };

    let tpl = UserTemplate {
        id: "tpl-1".to_string(),
        account_id: "acc-1".to_string(),
        name: "Contact".to_string(),
        icon_id: None,
        properties: vec![
            solosoul_vault::TemplateProperty {
                id: "textField".to_string(),
                name: "Text Field".to_string(),
                prop_type: solosoul_vault::PropertyType::Text,
                sensitive: None,
                sensitivity_level: Some("sensitive".to_string()),
                options: None,
                deprecated_at: None,
                contract_field: None,
                contract_bindings: None,
                allowed_types: None,
                max_items: None,
            },
            solosoul_vault::TemplateProperty {
                id: "numberField".to_string(),
                name: "Number Field".to_string(),
                prop_type: solosoul_vault::PropertyType::Text,
                sensitive: None,
                sensitivity_level: Some("internal".to_string()),
                options: None,
                deprecated_at: None,
                contract_field: None,
                contract_bindings: None,
                allowed_types: None,
                max_items: None,
            },
            solosoul_vault::TemplateProperty {
                id: "newField".to_string(),
                name: "New Field".to_string(),
                prop_type: solosoul_vault::PropertyType::Text,
                sensitive: None,
                sensitivity_level: Some("internal".to_string()),
                options: None,
                deprecated_at: None,
                contract_field: None,
                contract_bindings: None,
                allowed_types: None,
                max_items: None,
            },
        ],
        category: Some("identity".to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: Some(chrono::Utc::now().to_rfc3339()),
        contract_type_id: None,
    };

    let result = compute_sync_changes(&record, &tpl);
    assert!(result.has_changes);

    // oldField removed
    assert_eq!(result.fields_deprecated.len(), 1);
    assert_eq!(result.fields_deprecated[0].id, "oldField");

    // newField added
    assert_eq!(result.fields_added.len(), 1);
    assert_eq!(result.fields_added[0].id, "newField");

    // textField sensitivity updated; numberField number->text safe conversion
    assert_eq!(result.fields_updated.len(), 2);
    let updated_ids: std::collections::HashSet<_> = result
        .fields_updated
        .iter()
        .map(|f| f.id.as_str())
        .collect();
    assert!(updated_ids.contains("textField"));
    assert!(updated_ids.contains("numberField"));

    // no incompatible fields in this scenario
    assert!(result.fields_incompatible.is_empty());
}

#[test]
fn test_apply_sync_changes_archives_incompatible_field() {
    let mut record = ObjectRecord {
        id: "obj-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "identity".to_string(),
        section_type: "identity".to_string(),
        name: "Test".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({
            "__fields": {
                "numberField": { "name": "Number Field", "type": "number" }
            },
            "numberField": 42
        }),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: Some("tpl-1".to_string()),
        template_type: Some("user".to_string()),
        contract_type_id: None,
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
        ..Default::default()
    };

    let tpl = UserTemplate {
        id: "tpl-1".to_string(),
        account_id: "acc-1".to_string(),
        name: "Contact".to_string(),
        icon_id: None,
        properties: vec![solosoul_vault::TemplateProperty {
            id: "numberField".to_string(),
            name: "Number Field".to_string(),
            prop_type: solosoul_vault::PropertyType::Date,
            sensitive: None,
            sensitivity_level: Some("internal".to_string()),
            options: None,
            deprecated_at: None,
            contract_field: None,
            contract_bindings: None,
            allowed_types: None,
            max_items: None,
        }],
        category: Some("identity".to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: Some(chrono::Utc::now().to_rfc3339()),
        contract_type_id: None,
    };

    let result = compute_sync_changes(&record, &tpl);
    apply_sync_changes(&mut record, &tpl, &result, false);

    // 原字段应被重置为空字符串（date 默认值）
    assert_eq!(record.properties.get("numberField").unwrap(), "");

    // 旧字段应被归档到 __deprecatedFields
    let deprecated = record
        .properties
        .get("__deprecatedFields")
        .and_then(|v| v.as_object())
        .unwrap();
    assert!(deprecated.contains_key("numberField"));
    let archived = deprecated.get("numberField").unwrap().as_object().unwrap();
    assert_eq!(archived.get("value").unwrap(), 42);
    assert_eq!(
        archived.get("reason").and_then(|v| v.as_str()).unwrap(),
        "type_incompatible"
    );

    // template_hash 应已更新
    assert_eq!(record.template_hash, Some(result.template_hash));
}

#[test]
fn test_apply_sync_changes_preserves_safe_type_conversion() {
    let mut record = ObjectRecord {
        id: "obj-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "identity".to_string(),
        section_type: "identity".to_string(),
        name: "Test".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({
            "__fields": {
                "textField": { "name": "Text Field", "type": "text" }
            },
            "textField": "hello"
        }),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: Some("tpl-1".to_string()),
        template_type: Some("user".to_string()),
        contract_type_id: None,
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
        ..Default::default()
    };

    let tpl = UserTemplate {
        id: "tpl-1".to_string(),
        account_id: "acc-1".to_string(),
        name: "Contact".to_string(),
        icon_id: None,
        properties: vec![solosoul_vault::TemplateProperty {
            id: "textField".to_string(),
            name: "Text Field".to_string(),
            prop_type: solosoul_vault::PropertyType::MultilineText,
            sensitive: None,
            sensitivity_level: Some("internal".to_string()),
            options: None,
            deprecated_at: None,
            contract_field: None,
            contract_bindings: None,
            allowed_types: None,
            max_items: None,
        }],
        category: Some("identity".to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: Some(chrono::Utc::now().to_rfc3339()),
        contract_type_id: None,
    };

    let result = compute_sync_changes(&record, &tpl);
    assert!(result.fields_incompatible.is_empty());
    apply_sync_changes(&mut record, &tpl, &result, false);

    // 安全转换：text -> multiline 应保留原值
    assert_eq!(record.properties.get("textField").unwrap(), "hello");
    assert!(
        record.properties.get("__deprecatedFields").is_none()
            || record
                .properties
                .get("__deprecatedFields")
                .and_then(|v| v.as_object())
                .map(|m| m.is_empty())
                .unwrap_or(true)
    );
}

#[test]
fn test_compute_sync_changes_uses_property_labels_as_sensitivity_baseline() {
    let record = ObjectRecord {
        id: "obj-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "identity".to_string(),
        section_type: "identity".to_string(),
        name: "Test".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        // __fields 中动态字段组敏感度为旧值 internal，但 property_labels 已为 public
        properties: serde_json::json!({
            "__fields": {
                "textField": { "name": "Text Field", "type": "text", "sensitivityLevel": "internal" },
                "dynamicGroup": { "name": "Dynamic Group", "type": "dynamic_group", "sensitivityLevel": "internal" }
            },
            "textField": "hello",
            "dynamicGroup": []
        }),
        property_labels: Some(serde_json::json!({
            "textField": "internal",
            "dynamicGroup": "public"
        })),
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: Some("tpl-1".to_string()),
        template_type: Some("user".to_string()),
        contract_type_id: None,
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
        ..Default::default()
    };

    let tpl = UserTemplate {
        id: "tpl-1".to_string(),
        account_id: "acc-1".to_string(),
        name: "Contact".to_string(),
        icon_id: None,
        properties: vec![
            solosoul_vault::TemplateProperty {
                id: "textField".to_string(),
                name: "Text Field".to_string(),
                prop_type: solosoul_vault::PropertyType::Text,
                sensitive: None,
                sensitivity_level: Some("public".to_string()),
                options: None,
                deprecated_at: None,
                contract_field: None,
                contract_bindings: None,
                allowed_types: None,
                max_items: None,
            },
            solosoul_vault::TemplateProperty {
                id: "dynamicGroup".to_string(),
                name: "Dynamic Group".to_string(),
                prop_type: solosoul_vault::PropertyType::DynamicGroup,
                sensitive: None,
                sensitivity_level: Some("public".to_string()),
                options: None,
                deprecated_at: None,
                contract_field: None,
                contract_bindings: None,
                allowed_types: None,
                max_items: None,
            },
        ],
        category: Some("identity".to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: Some(chrono::Utc::now().to_rfc3339()),
        contract_type_id: None,
    };

    let result = compute_sync_changes(&record, &tpl);

    // 只有 textField 的敏感度真正从 internal 变到 public
    assert_eq!(result.fields_updated.len(), 1);
    assert_eq!(result.fields_updated[0].id, "textField");
    assert!(
        result.fields_updated[0]
            .changes
            .iter()
            .any(|c| matches!(c, SyncFieldChangeItem::Sensitivity { old_level, new_level } if old_level == "internal" && new_level == "public"))
    );

    // dynamicGroup 在 property_labels 中已经是 public，不应被误报
    assert!(!result.fields_updated.iter().any(|f| f.id == "dynamicGroup"));
}

#[test]
fn test_apply_sync_changes_preserves_multiline_value_on_rename() {
    let (vault, _dir) = setup_vault();

    // 创建模板：字段 ID 为 f1，名称 "1"，类型 multiline
    let tpl = UserTemplate {
        contract_type_id: None,
        id: "tpl-rename".to_string(),
        account_id: "acc-1".to_string(),
        name: "Rename Test".to_string(),
        icon_id: None,
        properties: vec![TemplateProperty {
            contract_field: None,
            contract_bindings: None,
            id: "f1".to_string(),
            name: "1".to_string(),
            prop_type: PropertyType::MultilineText,
            sensitivity_level: None,
            sensitive: None,
            options: None,
            deprecated_at: None,
            allowed_types: None,
            max_items: None,
        }],
        category: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: None,
    };
    vault.save_user_template(&tpl).unwrap();

    // 创建对象：字段 f1 的值为 "a"，__fields 中记录旧名称 "1"
    let mut record = ObjectRecord {
        id: "obj-rename".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "note".to_string(),
        section_type: "note".to_string(),
        name: "Test Object".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({
            "__fields": {
                "f1": { "name": "1", "type": "multiline" }
            },
            "f1": "a"
        }),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: Some("tpl-rename".to_string()),
        template_type: Some("user".to_string()),
        template_hash: Some(template_fingerprint(&tpl)),
        ignored_template_hash: None,
        contract_type_id: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
    };
    vault.save_object(&record).unwrap();

    // 修改模板字段名 "1" -> "2"，类型不变
    let mut modified_tpl = tpl;
    modified_tpl.properties[0].name = "2".to_string();
    modified_tpl.updated_at = Some(chrono::Utc::now().to_rfc3339());
    vault.save_user_template(&modified_tpl).unwrap();

    // 计算并应用同步
    let result = compute_sync_changes(&record, &modified_tpl);
    assert!(result.has_changes, "should detect name change");
    assert!(
        result.fields_updated.iter().any(|f| f.id == "f1"),
        "f1 should be in updated fields"
    );
    assert!(
        result.fields_added.is_empty(),
        "rename should not be treated as added field"
    );
    assert!(
        result.fields_deprecated.is_empty(),
        "rename should not be treated as deprecated field"
    );

    apply_sync_changes(&mut record, &modified_tpl, &result, false);

    // 关键断言：字段值必须保留，__fields 中的字段名应更新为 "2"
    assert_eq!(
        record.properties["f1"], "a",
        "multiline value must be preserved"
    );
    let fields = record.properties["__fields"].as_object().unwrap();
    assert_eq!(
        fields["f1"]["name"], "2",
        "__fields name should be updated to new template name"
    );
    assert_eq!(
        fields["f1"]["type"], "multiline",
        "__fields type should remain multiline"
    );
}

#[test]
fn test_apply_sync_changes_preserves_existing_values_when_fields_missing() {
    // 旧对象可能缺少 __fields（功能上线前创建），同步时不应覆盖已有字段值。
    let mut record = ObjectRecord {
        id: "obj-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "identity".to_string(),
        section_type: "identity".to_string(),
        name: "Test".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        // 没有 __fields，但实际有字段值
        properties: serde_json::json!({
            "f1": "a"
        }),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: Some("tpl-1".to_string()),
        template_type: Some("user".to_string()),
        contract_type_id: None,
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
        ..Default::default()
    };

    let tpl = UserTemplate {
        id: "tpl-1".to_string(),
        account_id: "acc-1".to_string(),
        name: "Contact".to_string(),
        icon_id: None,
        properties: vec![solosoul_vault::TemplateProperty {
            id: "f1".to_string(),
            name: "2".to_string(),
            prop_type: solosoul_vault::PropertyType::Text,
            sensitive: None,
            sensitivity_level: Some("internal".to_string()),
            options: None,
            deprecated_at: None,
            contract_field: None,
            contract_bindings: None,
            allowed_types: None,
            max_items: None,
        }],
        category: Some("identity".to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: Some(chrono::Utc::now().to_rfc3339()),
        contract_type_id: None,
    };

    let result = compute_sync_changes(&record, &tpl);
    apply_sync_changes(&mut record, &tpl, &result, false);

    // 关键断言：即使缺少 __fields，已有字段值 "a" 也必须保留
    assert_eq!(
        record.properties["f1"], "a",
        "existing value must not be overwritten"
    );
    let fields = record.properties["__fields"].as_object().unwrap();
    assert_eq!(fields["f1"]["name"], "2");
}

#[test]
fn test_compute_sync_changes_detects_dynamic_group_metadata() {
    let record = ObjectRecord {
        id: "obj-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "identity".to_string(),
        section_type: "identity".to_string(),
        name: "Test".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({
            "__fields": {
                "contacts": {
                    "name": "联系方式",
                    "type": "dynamic_group",
                    "allowedTypes": ["text"],
                    "maxItems": 5
                }
            },
            "contacts": []
        }),
        property_labels: Some(serde_json::json!({ "contacts": "internal" })),
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: Some("tpl-1".to_string()),
        template_type: Some("user".to_string()),
        contract_type_id: None,
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
        ..Default::default()
    };

    let tpl = UserTemplate {
        id: "tpl-1".to_string(),
        account_id: "acc-1".to_string(),
        name: "Contact".to_string(),
        icon_id: None,
        properties: vec![solosoul_vault::TemplateProperty {
            id: "contacts".to_string(),
            name: "联系方式".to_string(),
            prop_type: solosoul_vault::PropertyType::DynamicGroup,
            sensitive: None,
            sensitivity_level: Some("internal".to_string()),
            options: None,
            deprecated_at: None,
            contract_field: None,
            contract_bindings: None,
            allowed_types: Some(vec![
                solosoul_vault::PropertyType::Text,
                solosoul_vault::PropertyType::Phone,
            ]),
            max_items: Some(10),
        }],
        category: Some("identity".to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: Some(chrono::Utc::now().to_rfc3339()),
        contract_type_id: None,
    };

    let result = compute_sync_changes(&record, &tpl);
    assert!(
        result.has_changes,
        "should detect dynamic group metadata changes"
    );
    assert_eq!(result.fields_updated.len(), 1);
    assert_eq!(result.fields_updated[0].id, "contacts");
    assert!(
        result.fields_updated[0]
            .changes
            .iter()
            .any(|c| matches!(c, SyncFieldChangeItem::Metadata { metadata_keys } if metadata_keys.contains(&"allowedTypes".to_string()) && metadata_keys.contains(&"maxItems".to_string()))),
        "expected Metadata change for allowedTypes and maxItems, got {:?}",
        result.fields_updated[0].changes
    );
}

#[test]
fn test_compute_sync_changes_detects_field_metadata() {
    let record = ObjectRecord {
        id: "obj-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "identity".to_string(),
        section_type: "identity".to_string(),
        name: "Test".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({
            "__fields": {
                "nameField": {
                    "name": "姓名",
                    "type": "text",
                    "deprecatedAt": "",
                    "contractField": false
                }
            },
            "nameField": "Alice"
        }),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: Some("tpl-1".to_string()),
        template_type: Some("user".to_string()),
        contract_type_id: None,
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
        ..Default::default()
    };

    let tpl = UserTemplate {
        id: "tpl-1".to_string(),
        account_id: "acc-1".to_string(),
        name: "Contact".to_string(),
        icon_id: None,
        properties: vec![solosoul_vault::TemplateProperty {
            id: "nameField".to_string(),
            name: "姓名".to_string(),
            prop_type: solosoul_vault::PropertyType::Text,
            sensitive: None,
            sensitivity_level: Some("internal".to_string()),
            options: None,
            deprecated_at: Some("2024-01-01T00:00:00Z".to_string()),
            contract_field: Some(true),
            contract_bindings: None,
            allowed_types: None,
            max_items: None,
        }],
        category: Some("identity".to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: Some(chrono::Utc::now().to_rfc3339()),
        contract_type_id: None,
    };

    let result = compute_sync_changes(&record, &tpl);
    assert!(result.has_changes, "should detect field metadata changes");
    assert_eq!(result.fields_updated.len(), 1);
    assert_eq!(result.fields_updated[0].id, "nameField");
    assert!(
        result.fields_updated[0]
            .changes
            .iter()
            .any(|c| matches!(c, SyncFieldChangeItem::Metadata { metadata_keys } if metadata_keys.contains(&"deprecatedAt".to_string()) && metadata_keys.contains(&"contractField".to_string()))),
        "expected Metadata change for deprecatedAt and contractField, got {:?}",
        result.fields_updated[0].changes
    );
}
