//! object 命令测试 —— misc（P047 拆分）

use super::super::*;
use super::setup_vault;
use solosoul_vault::{ObjectRecord, UserTemplate};

#[test]
fn test_validate_dynamic_groups_ok() {
    let properties = serde_json::json!({
        "__fields": {
            "contactMethods": {
                "name": "联系方式",
                "type": "dynamic_group",
                "allowedTypes": ["phone", "email"]
            }
        },
        "contactMethods": [
            { "id": "1", "name": "手机", "type": "phone", "value": "123" },
            { "id": "2", "name": "邮箱", "type": "email", "value": "a@b.com" }
        ]
    });
    assert!(validate_dynamic_groups(&properties).is_ok());
}

#[test]
fn test_validate_dynamic_groups_invalid_type() {
    let properties = serde_json::json!({
        "__fields": {
            "contactMethods": {
                "name": "联系方式",
                "type": "dynamic_group",
                "allowedTypes": ["phone"]
            }
        },
        "contactMethods": [
            { "id": "1", "name": "邮箱", "type": "email", "value": "a@b.com" }
        ]
    });
    assert!(validate_dynamic_groups(&properties).is_err());
}

#[test]
fn test_validate_dynamic_groups_exceeds_max_items() {
    let properties = serde_json::json!({
        "__fields": {
            "contactMethods": {
                "name": "联系方式",
                "type": "dynamic_group",
                "maxItems": 1
            }
        },
        "contactMethods": [
            { "id": "1", "name": "手机", "type": "phone", "value": "123" },
            { "id": "2", "name": "邮箱", "type": "email", "value": "a@b.com" }
        ]
    });
    assert!(validate_dynamic_groups(&properties).is_err());
}

#[test]
fn test_backfill_missing_property_labels_from_template() {
    let (vault, _dir) = setup_vault();

    // 创建一个带字段敏感度的模板
    let tpl = UserTemplate {
        id: "tpl-sensitivity".to_string(),
        account_id: "test_account".to_string(),
        name: "ID Card".to_string(),
        icon_id: None,
        properties: vec![
            solosoul_vault::TemplateProperty {
                id: "name".to_string(),
                name: "姓名".to_string(),
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
                id: "id_number".to_string(),
                name: "身份证号".to_string(),
                prop_type: solosoul_vault::PropertyType::Text,
                sensitive: None,
                sensitivity_level: Some("critical".to_string()),
                options: None,
                deprecated_at: None,
                contract_field: None,
                contract_bindings: None,
                allowed_types: None,
                max_items: None,
            },
            solosoul_vault::TemplateProperty {
                id: "__dynamic_group__".to_string(),
                name: "联系方式".to_string(),
                prop_type: solosoul_vault::PropertyType::DynamicGroup,
                sensitive: None,
                sensitivity_level: Some("sensitive".to_string()),
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
    vault.save_user_template(&tpl).unwrap();

    // 模拟旧 bug 恢复出的对象：有模板，但 property_labels 为空
    let record = ObjectRecord {
        id: "obj-no-labels".to_string(),
        account_id: "test_account".to_string(),
        type_id: "identity".to_string(),
        section_type: "identity".to_string(),
        name: "Old ID".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({
            "name": "Alice",
            "id_number": "123456",
            "__dynamic_group__": []
        }),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: Some("tpl-sensitivity".to_string()),
        template_type: Some("user".to_string()),
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
        ..Default::default()
    };
    vault.save_object(&record).unwrap();

    // 重置标记并手动触发补齐
    vault
        .set_sys_config("property_labels_backfill_v1", "0")
        .unwrap();
    let filled = vault.backfill_missing_property_labels().unwrap();
    assert_eq!(filled, 1, "should backfill one object");

    let updated = vault.load_object("obj-no-labels").unwrap().unwrap();
    let labels = updated
        .property_labels
        .expect("property_labels should be populated")
        .as_object()
        .unwrap()
        .clone();
    assert_eq!(labels.get("name").and_then(|v| v.as_str()), Some("public"));
    assert_eq!(
        labels.get("id_number").and_then(|v| v.as_str()),
        Some("critical")
    );
    assert_eq!(
        labels.get("__dynamic_group__").and_then(|v| v.as_str()),
        Some("sensitive")
    );
}
