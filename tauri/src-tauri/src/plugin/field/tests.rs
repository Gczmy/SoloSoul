use super::*;
use crate::plugin::manifest::PluginContractBinding;
use solosoul_vault::{
    ContractRoleBinding, ObjectRecord, PropertyType, TemplateProperty, UserTemplate, VaultConfig,
    VaultStore,
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
        icon_id: Some("map-pin".to_string()),        properties: vec![TemplateProperty {
                contract_field: Some(true),
                contract_bindings: None,
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
        icon_id: None,        properties: vec![TemplateProperty {
                contract_field: None, // 未标记为 contract_field
                contract_bindings: None,
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

    // list_objects 替代 .count
    let json = resolver.list_objects("address").unwrap();
    let items: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["name"].as_str().unwrap(), "家");
    assert_eq!(
        items[0]["properties"]["street"].as_str().unwrap(),
        "长安街1号"
    );
}

/// 精确复现用户场景：带 contracts (typed-lookup) + legacy 对象 (contract_type_id=None, template_id=None)
/// 验证 collection_type 回退能正确匹配
#[test]
fn test_list_objects_typed_lookup_with_legacy_objects() {
    let account_id = "acc_list_legacy_in_typed";
    let (_tmp, vault) = test_vault(account_id);

    let now = chrono::Utc::now().to_rfc3339();
    // 模板无 contract_type_id（legacy 模板）
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
        updated_at: Some(now.clone()),
    };
    vault.save_user_template(&template).unwrap();

    // legacy 对象：contract_type_id=None, template_id=None
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
        created_at: now.clone(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
    };
    vault.save_object(&record).unwrap();

    // 带 contracts（模拟地址格式化器的 manifest 声明）
    let contracts = vec![PluginContractBinding {
        type_id: "com.solosoul.address/v1".to_string(),
        version: 1,
        type_id_aliases: vec!["address".to_string()],
        ..Default::default()
    }];
    let resolver = FieldResolver::with_vault_and_contracts(
        vault,
        account_id.to_string(),
        vec!["address.*".to_string()],
        contracts,
    );

    // list_objects 走 typed-lookup 路径，应通过 collection_type 回退找到 legacy 对象
    let json = resolver.list_objects("address").unwrap();
    assert_ne!(
        json, "[]",
        "Should NOT be empty - should find legacy objects via collection_type fallback"
    );
    let items: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
    assert_eq!(
        items.len(),
        1,
        "Should find 1 legacy address object via collection_type fallback"
    );
    assert_eq!(items[0]["name"].as_str().unwrap(), "家");
    assert_eq!(
        items[0]["properties"]["street"].as_str().unwrap(),
        "长安街1号"
    );
}

/// 新版 contract_bindings 解析：用户模板字段通过 contract_bindings 绑定到 plugin role
#[test]
fn test_resolve_typed_with_contract_bindings() {
    let account_id = "acc_bindings";
    let (_tmp, vault) = test_vault(account_id);

    let now = chrono::Utc::now().to_rfc3339();
    let now2 = now.clone();
    // 用户自定义模板：字段 ID 为 "specificAddress"（不是 street），
    // 通过 contract_bindings 声明绑定到 address-fmt/v1 的 street role
    let template = UserTemplate {
        contract_type_id: Some("com.solosoul.address/v1".to_string()),
        id: "addr_tpl".to_string(),
        account_id: account_id.to_string(),
        name: "临时地址".to_string(),
        icon_id: Some("map-pin".to_string()),
        properties: vec![TemplateProperty {
            contract_field: None, // 旧版标记为空
            contract_bindings: Some(vec![ContractRoleBinding {
                contract_type_id: "com.solosoul.address/v1".to_string(),
                role_id: "street".to_string(),
            }]),
            id: "specificAddress".to_string(),
            name: "具体地址".to_string(),
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
        type_id: "addr_tpl".to_string(),
        section_type: "identity".to_string(),
        name: "家".to_string(),
        icon_name: "map-pin".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({"specificAddress": "123 Main St"}),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: None,
        template_type: None,
        created_at: now.clone(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
    };
    vault.save_object(&record).unwrap();

    let contracts = vec![PluginContractBinding {
        type_id: "com.solosoul.address/v1".to_string(),
        type_id_aliases: vec!["addr_tpl".to_string()],
        ..Default::default()
    }];
    let resolver = FieldResolver::with_vault_and_contracts(
        vault,
        account_id.to_string(),
        vec!["addr_tpl.*".to_string()],
        contracts,
    );

    let result = resolver.resolve("addr_tpl.specificAddress").unwrap();
    assert_eq!(result, "123 Main St");
}

/// 旧版 contract_field = true 仍然可解析
#[test]
fn test_resolve_typed_legacy_contract_field_still_works() {
    let account_id = "acc_legacy_field";
    let (_tmp, vault) = test_vault(account_id);

    let now = chrono::Utc::now().to_rfc3339();
    let template = UserTemplate {
        contract_type_id: Some("com.solosoul.address/v1".to_string()),
        id: "addr".to_string(),
        account_id: account_id.to_string(),
        name: "地址".to_string(),
        icon_id: Some("map-pin".to_string()),
        properties: vec![TemplateProperty {
            contract_field: Some(true), // 旧版标记
            contract_bindings: None,
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
        created_at: now.clone(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
    };
    vault.save_object(&record).unwrap();

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

    // 即使字段 ID 等于 role_id（legacy 模式），也应通过旧版路径找到
    let result = resolver.resolve("addr.street").unwrap();
    assert_eq!(result, "长安街1号");
}

/// 新版 binding 优先级高于旧版 contract_field + 字段 ID 匹配
#[test]
fn test_resolve_typed_role_binding_overrides_legacy_id() {
    let account_id = "acc_override";
    let (_tmp, vault) = test_vault(account_id);

    let now = chrono::Utc::now().to_rfc3339();
    // 模板同时包含旧版 street 字段和新版 specificAddress（绑定到 street role）
    let template = UserTemplate {
        contract_type_id: Some("com.solosoul.address/v1".to_string()),
        id: "addr".to_string(),
        account_id: account_id.to_string(),
        name: "地址".to_string(),
        icon_id: None,
        properties: vec![
            TemplateProperty {
                contract_field: Some(true),
                contract_bindings: None,
                id: "street".to_string(),
                name: "旧街道".to_string(),
                prop_type: PropertyType::Text,
                sensitivity_level: Some("internal".to_string()),
                sensitive: None,
                options: None,
                deprecated_at: None,
            },
            TemplateProperty {
                contract_field: None,
                contract_bindings: Some(vec![ContractRoleBinding {
                    contract_type_id: "com.solosoul.address/v1".to_string(),
                    role_id: "street".to_string(),
                }]),
                id: "specificAddress".to_string(),
                name: "具体地址".to_string(),
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

    // 新版 binding 对应的对象
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
        properties: serde_json::json!({"specificAddress": "456 Oak Ave", "street": "789 Pine St"}),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: None,
        template_type: None,
        created_at: now.clone(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
    };
    vault.save_object(&record).unwrap();

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

    // 解析 street role → 应返回 specificAddress（新版 binding 优先）而不是旧版 street 字段
    let result = resolver.resolve("addr.street").unwrap();
    assert_eq!(result, "456 Oak Ave", "新版 role binding 应优先于旧版 contract_field");
}

/// field_metadata_typed 通过 role binding 正确返回字段标签和敏感度
#[test]
fn test_field_metadata_typed_with_role_binding() {
    let account_id = "acc_meta_binding";
    let (_tmp, vault) = test_vault(account_id);

    let now = chrono::Utc::now().to_rfc3339();
    let template = UserTemplate {
        contract_type_id: Some("com.solosoul.address/v1".to_string()),
        id: "addr".to_string(),
        account_id: account_id.to_string(),
        name: "地址".to_string(),
        icon_id: None,
        properties: vec![TemplateProperty {
            contract_field: None,
            contract_bindings: Some(vec![ContractRoleBinding {
                contract_type_id: "com.solosoul.address/v1".to_string(),
                role_id: "street".to_string(),
            }]),
            id: "specificAddress".to_string(),
            name: "具体地址".to_string(),
            prop_type: PropertyType::Text,
            sensitivity_level: Some("sensitive".to_string()),
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
        properties: serde_json::json!({"specificAddress": "123 Main St"}),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: None,
        template_type: None,
        created_at: now.clone(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
    };
    vault.save_object(&record).unwrap();

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

    let (label, sensitivity) = resolver.field_metadata("addr.specificAddress").unwrap();
    assert_eq!(label, "具体地址");
    assert_eq!(sensitivity, "sensitive");
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
