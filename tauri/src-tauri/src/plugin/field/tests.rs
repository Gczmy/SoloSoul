use super::*;
use crate::plugin::manifest::PluginContractBinding;
use solosoul_vault::{
    ObjectRecord, PropertyType, TemplateProperty, UserTemplate, VaultConfig, VaultStore,
};
use std::sync::Arc;
use tempfile::TempDir;

fn test_vault(account_id: &str) -> (TempDir, Arc<VaultStore>) {
    let tmp = TempDir::new().unwrap();
    let config = VaultConfig::new(account_id, tmp.path().to_path_buf()).with_data_key([0u8; 32]);
    let vault = VaultStore::open(config).unwrap();
    (tmp, Arc::new(vault))
}

#[test]
fn test_field_metadata() {
    let account_id = "acc_test_meta";
    let (_tmp, vault) = test_vault(account_id);

    let now = chrono::Utc::now().to_rfc3339();
    let template = UserTemplate {
        contract_type_id: None,
        id: "address".to_string(),
        account_id: account_id.to_string(),
        name: "地址".to_string(),
        icon_id: Some("map-pin".to_string()),
        properties: vec![
            TemplateProperty {
                contract_field: None,
                id: "street".to_string(),
                name: "街道".to_string(),
                prop_type: PropertyType::Text,
                sensitivity_level: Some("private".to_string()),
                sensitive: None,
                options: None,
                deprecated_at: None,
            },
            TemplateProperty {
                contract_field: None,
                id: "country".to_string(),
                name: "国家".to_string(),
                prop_type: PropertyType::Text,
                sensitivity_level: Some("internal".to_string()),
                sensitive: None,
                options: None,
                deprecated_at: None,
            },
        ],
        category: Some("identity".to_string()),
        created_at: now.clone(),
        updated_at: Some(now),
    };
    vault.save_user_template(&template).unwrap();

    let resolver = FieldResolver::with_vault(vault, account_id.to_string(), vec![]);

    let (label, sensitivity) = resolver.field_metadata("address.street").unwrap();
    assert_eq!(label, "街道");
    assert_eq!(sensitivity, "private");

    let (label2, sensitivity2) = resolver.field_metadata("address[0].country").unwrap();
    assert_eq!(label2, "国家");
    assert_eq!(sensitivity2, "internal");

    // 嵌套路径取第一级属性
    let (label3, sensitivity3) = resolver.field_metadata("address.street.extra").unwrap();
    assert_eq!(label3, "街道");
    assert_eq!(sensitivity3, "private");

    assert!(resolver.field_metadata("unknown.street").is_err());
}

#[test]
fn test_build_structure_tree() {
    let account_id = "acc_test_tree";
    let (_tmp, vault) = test_vault(account_id);

    let now = chrono::Utc::now().to_rfc3339();
    let template = UserTemplate {
        contract_type_id: None,
        id: "address".to_string(),
        account_id: account_id.to_string(),
        name: "地址".to_string(),
        icon_id: Some("map-pin".to_string()),
        properties: vec![
            TemplateProperty {
                contract_field: None,
                id: "street".to_string(),
                name: "街道".to_string(),
                prop_type: PropertyType::Text,
                sensitivity_level: Some("internal".to_string()),
                sensitive: None,
                options: None,
                deprecated_at: None,
            },
            TemplateProperty {
                contract_field: None,
                id: "country".to_string(),
                name: "国家".to_string(),
                prop_type: PropertyType::Text,
                sensitivity_level: Some("internal".to_string()),
                sensitive: None,
                options: None,
                deprecated_at: None,
            },
        ],
        category: Some("identity".to_string()),
        created_at: now.clone(),
        updated_at: Some(now),
    };
    vault.save_user_template(&template).unwrap();

    // 写入一条地址对象，验证 count 统计
    let record = ObjectRecord {
        contract_type_id: None,
        id: "addr_0".to_string(),
        account_id: account_id.to_string(),
        type_id: "address".to_string(),
        section_type: "identity".to_string(),
        name: "家".to_string(),
        icon_name: "map-pin".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({"street": "长安街1号", "country": "CN"}),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: None,
        template_type: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
    };
    vault.save_object(&record).unwrap();

    let resolver = FieldResolver::with_vault(vault, account_id.to_string(), vec![]);
    let json = resolver.build_structure_tree().unwrap();
    let tree: serde_json::Value = serde_json::from_str(&json).unwrap();

    let types = tree["types"].as_array().unwrap();
    assert_eq!(types.len(), 1);
    assert_eq!(types[0]["id"], "address");
    assert_eq!(types[0]["name"], "地址");
    assert_eq!(types[0]["category"], "identity");
    assert_eq!(types[0]["count"], 1);
    let props = types[0]["properties"].as_array().unwrap();
    assert_eq!(props.len(), 2);
    assert_eq!(props[0]["id"], "street");
    assert_eq!(props[0]["type"], "text");
}

// ── Stage 4-B typed-lookup 单元测试 ─────────────────────────────────

/// typed-lookup happy path：UserTemplate + ObjectRecord 都标 contract
#[test]
fn test_resolve_typed_happy_path() {
    let account_id = "acc_typed_happy";
    let (_tmp, vault) = test_vault(account_id);

    let now = chrono::Utc::now().to_rfc3339();
    let template = UserTemplate {
        contract_type_id: Some("com.solosoul.address/v1".to_string()),
        id: "addr".to_string(),
        account_id: account_id.to_string(),
        name: "地址".to_string(),
        icon_id: Some("map-pin".to_string()),
        properties: vec![TemplateProperty {
            contract_field: Some(true),
            id: "street".to_string(),
            name: "街道".to_string(),
            prop_type: PropertyType::Text,
            sensitivity_level: Some("internal".to_string()),
            sensitive: None,
            options: None,
            deprecated_at: None,
        }],
        category: Some("identity".to_string()),
        created_at: now.clone(),
        updated_at: Some(now),
    };
    vault.save_user_template(&template).unwrap();

    let record = ObjectRecord {
        contract_type_id: Some("com.solosoul.address/v1".to_string()),
        id: "addr_1".to_string(),
        account_id: account_id.to_string(),
        type_id: "addr".to_string(),
        section_type: "identity".to_string(),
        name: "家".to_string(),
        icon_name: "map-pin".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({"street": "长安街1号"}),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: None,
        template_type: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
    };
    vault.save_object(&record).unwrap();

    // 使用 SECONDARY alias 路径：alias "addr" → contract_type_id "com.solosoul.address/v1"
    let contracts = vec![PluginContractBinding {
        type_id: "com.solosoul.address/v1".to_string(),
        type_id_aliases: vec!["addr".to_string()],
        ..Default::default()
    }];
    let resolver = FieldResolver::with_vault_and_contracts(
        vault,
        account_id.to_string(),
        vec!["addr.*".to_string()],
        contracts,
    );

    let result = resolver.resolve("addr.street").unwrap();
    assert_eq!(result, "长安街1号");
}

/// typed-lookup：用户未建契约模板 → InvalidField
#[test]
fn test_resolve_typed_missing_template() {
    let account_id = "acc_typed_missing_tpl";
    let (_tmp, vault) = test_vault(account_id);

    let contracts = vec![PluginContractBinding {
        type_id: "com.solosoul.address/v1".to_string(),
        type_id_aliases: vec!["address".to_string()],
        ..Default::default()
    }];
    let resolver =
        FieldResolver::with_vault_and_contracts(vault, account_id.to_string(), vec![], contracts);

    let result = resolver.resolve("address.street");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("contract_type_id"));
}

/// typed-lookup：property 上 contract_field != Some(true) → InvalidField（gate）
#[test]
fn test_resolve_typed_contract_field_false() {
    let account_id = "acc_typed_gate_false";
    let (_tmp, vault) = test_vault(account_id);

    let now = chrono::Utc::now().to_rfc3339();
    let template = UserTemplate {
        contract_type_id: Some("com.solosoul.address/v1".to_string()),
        id: "addr".to_string(),
        account_id: account_id.to_string(),
        name: "地址".to_string(),
        icon_id: None,
        properties: vec![TemplateProperty {
            contract_field: None, // 未标记为 contract_field
            id: "street".to_string(),
            name: "街道".to_string(),
            prop_type: PropertyType::Text,
            sensitivity_level: Some("internal".to_string()),
            sensitive: None,
            options: None,
            deprecated_at: None,
        }],
        category: Some("identity".to_string()),
        created_at: now.clone(),
        updated_at: Some(now),
    };
    vault.save_user_template(&template).unwrap();

    let record = ObjectRecord {
        contract_type_id: Some("com.solosoul.address/v1".to_string()),
        id: "addr_1".to_string(),
        account_id: account_id.to_string(),
        type_id: "addr".to_string(),
        section_type: "identity".to_string(),
        name: "家".to_string(),
        icon_name: "map-pin".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({"street": "长安街1号"}),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: None,
        template_type: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
    };
    vault.save_object(&record).unwrap();

    let contracts = vec![PluginContractBinding {
        type_id: "com.solosoul.address/v1".to_string(),
        type_id_aliases: vec!["addr".to_string()],
        ..Default::default()
    }];
    let resolver =
        FieldResolver::with_vault_and_contracts(vault, account_id.to_string(), vec![], contracts);

    let result = resolver.resolve("addr.street");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("gate") || err.contains("contract_field"),
        "Expected gate rejection error, got: {}",
        err
    );
}

/// Legacy 路径不受 typed-lookup 影响
#[test]
fn test_resolve_legacy_unchanged() {
    let account_id = "acc_legacy_unchanged";
    let (_tmp, vault) = test_vault(account_id);

    let now = chrono::Utc::now().to_rfc3339();
    let template = UserTemplate {
        contract_type_id: None,
        id: "address".to_string(),
        account_id: account_id.to_string(),
        name: "地址".to_string(),
        icon_id: Some("map-pin".to_string()),
        properties: vec![TemplateProperty {
            contract_field: None,
            id: "street".to_string(),
            name: "街道".to_string(),
            prop_type: PropertyType::Text,
            sensitivity_level: Some("internal".to_string()),
            sensitive: None,
            options: None,
            deprecated_at: None,
        }],
        category: Some("identity".to_string()),
        created_at: now.clone(),
        updated_at: Some(now),
    };
    vault.save_user_template(&template).unwrap();

    let record = ObjectRecord {
        contract_type_id: None,
        id: "addr_1".to_string(),
        account_id: account_id.to_string(),
        type_id: "address".to_string(),
        section_type: "identity".to_string(),
        name: "家".to_string(),
        icon_name: "map-pin".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({"street": "长安街1号"}),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: None,
        template_type: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
    };
    vault.save_object(&record).unwrap();

    // 不传 contracts → 走 legacy 路径
    let resolver = FieldResolver::with_vault(vault, account_id.to_string(), vec![]);

    let result = resolver.resolve("address.street").unwrap();
    assert_eq!(result, "长安街1号");

    let count = resolver.resolve("address.count").unwrap();
    assert_eq!(count, "1");
}

/// parse_typed_field SECONDARY alias 路径（无 vault 模式）
#[test]
fn test_parse_typed_field_secondary_alias() {
    let contracts = vec![PluginContractBinding {
        type_id: "com.solosoul.address/v1".to_string(),
        type_id_aliases: vec!["address".to_string()],
        ..Default::default()
    }];
    let resolver = FieldResolver::with_contracts(contracts);

    let res = resolver.parse_typed_field("address.street").unwrap();
    assert_eq!(
        res,
        Some(("com.solosoul.address/v1".to_string(), "street".to_string()))
    );

    // miss
    let res = resolver.parse_typed_field("unknown.field").unwrap();
    assert_eq!(res, None);
}

#[test]
fn test_normalize_for_permission() {
    assert_eq!(
        normalize_for_permission("address[0].street"),
        Some("address.street".to_string())
    );
    assert_eq!(
        normalize_for_permission("travel.primary_passport.number"),
        Some("travel.primary_passport.number".to_string())
    );
    assert_eq!(
        normalize_for_permission("address.count"),
        Some("address.count".to_string())
    );
    assert!(normalize_for_permission("address[a].street").is_none());
    assert!(normalize_for_permission("").is_some());
}

#[test]
fn test_pattern_matches() {
    assert!(pattern_matches("address.street", "address.street"));
    assert!(pattern_matches("address.*", "address.street"));
    assert!(pattern_matches("*.street", "address.street"));
    assert!(pattern_matches("*", "address.street"));
    assert!(!pattern_matches("address.city", "address.street"));
    assert!(!pattern_matches("identity.*", "address.street"));
}

#[test]
fn test_parse_indexed_field() {
    assert_eq!(
        parse_indexed_field("address[0].street"),
        Some(("address".to_string(), 0, "street".to_string()))
    );
    assert_eq!(
        parse_indexed_field("travel[3].primary_passport.number"),
        Some((
            "travel".to_string(),
            3,
            "primary_passport.number".to_string()
        ))
    );
    assert!(parse_indexed_field("address.count").is_none());
}

#[test]
fn test_extract_property() {
    let props = serde_json::json!({
        "street": "长安街1号",
        "postalCode": "100000",
        "primary_passport": { "number": "E12345678" }
    });
    assert_eq!(extract_property(&props, "street"), "长安街1号");
    assert_eq!(extract_property(&props, "postalCode"), "100000");
    assert_eq!(
        extract_property(&props, "primary_passport.number"),
        "E12345678"
    );
    assert_eq!(extract_property(&props, "missing"), "");
}
