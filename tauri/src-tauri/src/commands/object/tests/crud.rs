//! object 命令测试 —— crud（P047 拆分）

use super::super::*;
use super::setup_vault;
use solosoul_vault::{ObjectRecord, TrashItem, UserTemplate};

#[test]
fn test_inherit_contract_type_id() {
    let (vault, _dir) = setup_vault();

    // Missing template_id → None
    assert_eq!(inherit_contract_type_id(&vault, None), None);

    // Non-existent template → None (graceful fallback)
    assert_eq!(inherit_contract_type_id(&vault, Some("nonexistent")), None);

    // Template with contract_type_id → Some
    let tpl = UserTemplate {
        contract_type_id: Some("com.solosoul.address/v1".to_string()),
        id: "addr-template".to_string(),
        account_id: "acc-1".to_string(),
        name: "Address".to_string(),
        icon_id: None,
        properties: vec![],
        category: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: None,
    };
    vault.save_user_template(&tpl).unwrap();
    assert_eq!(
        inherit_contract_type_id(&vault, Some("addr-template")),
        Some("com.solosoul.address/v1".to_string())
    );

    // Template without contract_type_id → None
    let tpl2 = UserTemplate {
        contract_type_id: None,
        id: "plain-template".to_string(),
        account_id: "acc-1".to_string(),
        name: "Plain".to_string(),
        icon_id: None,
        properties: vec![],
        category: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: None,
    };
    vault.save_user_template(&tpl2).unwrap();
    assert_eq!(
        inherit_contract_type_id(&vault, Some("plain-template")),
        None
    );
}

#[test]
fn test_record_to_data_conversion() {
    let record = ObjectRecord {
        contract_type_id: None,
        id: "obj-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "note".to_string(),
        section_type: "identity".to_string(),
        name: "Test Object".to_string(),
        icon_name: "document".to_string(),
        parent_id: Some("parent-1".to_string()),
        children_ids: vec!["child-1".to_string()],
        properties: serde_json::json!({"key": "value"}),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec!["tag1".to_string()],
        template_id: None,
        template_type: None,
        template_hash: None,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-02T00:00:00Z".to_string(),
        version: 1,
        ..Default::default()
    };
    let data = record_to_data(&record);
    assert_eq!(data.id, "obj-1");
    assert_eq!(data.account_id, "acc-1");
    assert_eq!(data.collection_type, "note");
    assert_eq!(data.name, "Test Object");
    assert_eq!(data.sensitivity_level, "internal");
    assert_eq!(data.deleted_at, None);
    assert_eq!(data.tags, vec!["tag1"]);
}

#[test]
fn test_object_data_serde_roundtrip() {
    let original = ObjectData {
        id: "obj-1".to_string(),
        account_id: "acc-1".to_string(),
        name: "Test".to_string(),
        collection_type: "note".to_string(),
        properties: serde_json::json!({"foo": "bar"}),
        sensitivity_level: "public".to_string(),
        property_labels: None,
        tags: vec!["tag-a".to_string()],
        contract_type_id: None,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
        deleted_at: Some("2024-02-01T00:00:00Z".to_string()),
        template_id: None,
        template_type: None,
        template_hash: None,
        ignored_template_hash: None,
    };
    let json = serde_json::to_string(&original).unwrap();
    assert!(json.contains("accountId"));
    assert!(json.contains("typeId"));
    let restored: ObjectData = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.id, original.id);
    assert_eq!(restored.name, original.name);
    assert_eq!(restored.tags, original.tags);
}

#[test]
fn test_retention_ms_parsing() {
    assert_eq!(retention_ms("30d"), 30 * 24 * 3600 * 1000i64);
    assert_eq!(retention_ms("60d"), 60 * 24 * 3600 * 1000i64);
    assert_eq!(retention_ms("half_year"), 180 * 24 * 3600 * 1000i64);
    assert_eq!(retention_ms("one_year"), 365 * 24 * 3600 * 1000i64);
    assert_eq!(retention_ms("never"), i64::MAX);
    assert_eq!(retention_ms("unknown"), 30 * 24 * 3600 * 1000i64);
}

#[test]
fn test_object_filter_deserialization() {
    let json = r#"{"typeId":"note","keyword":"test"}"#;
    let filter: ObjectFilter = serde_json::from_str(json).unwrap();
    assert_eq!(filter.collection_type, Some("note".to_string()));
    assert_eq!(filter.keyword, Some("test".to_string()));
    assert_eq!(filter.sensitivity_level, None);
    assert_eq!(filter.parent_id, None);
}

#[test]
fn test_create_object_input_deserialization() {
    let json = r#"{"accountId":"acc-1","name":"My Note","typeId":"note","properties":{},"parentId":"parent-1","iconName":"star"}"#;
    let input: CreateObjectInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.account_id, "acc-1");
    assert_eq!(input.icon_name, Some("star".to_string()));
    assert_eq!(input.parent_id, Some("parent-1".to_string()));
}

#[test]
fn test_vault_object_save_and_load() {
    let (vault, _dir) = setup_vault();
    let record = ObjectRecord {
        contract_type_id: None,
        id: "obj-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "note".to_string(),
        section_type: "identity".to_string(),
        name: "Test Note".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({"content": "hello"}),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: None,
        template_type: None,
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
        ..Default::default()
    };
    vault.save_object(&record).unwrap();
    let loaded = vault.load_object("obj-1").unwrap().unwrap();
    assert_eq!(loaded.name, "Test Note");
    assert_eq!(loaded.properties, serde_json::json!({"content": "hello"}));
}

#[test]
fn test_vault_object_list_and_soft_delete() {
    let (vault, _dir) = setup_vault();
    for i in 0..3 {
        let record = ObjectRecord {
            contract_type_id: None,
            id: format!("obj-{}", i),
            account_id: "acc-1".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: format!("Note {}", i),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::Value::Object(serde_json::Map::new()),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            template_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            ..Default::default()
        };
        vault.save_object(&record).unwrap();
    }
    let all = vault
        .list_objects("acc-1", None, None, None, false, false)
        .unwrap();
    assert_eq!(all.len(), 3);

    vault.delete_object("obj-1", true).unwrap();
    let remaining = vault
        .list_objects("acc-1", None, None, None, false, false)
        .unwrap();
    assert_eq!(remaining.len(), 2);

    let deleted = vault
        .list_objects("acc-1", None, None, None, false, true)
        .unwrap();
    assert_eq!(deleted.len(), 1);
}

#[test]
fn test_update_object_input_deserialization() {
    let json = r#"{"name":"Updated Name","properties":{"key":"val"},"sensitivityLevel":"private"}"#;
    let input: UpdateObjectInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.name, "Updated Name");
    assert_eq!(input.sensitivity_level, Some("private".to_string()));
}

#[test]
fn test_object_create_with_parent() {
    let (vault, _dir) = setup_vault();
    let parent = ObjectRecord {
        contract_type_id: None,
        id: "parent-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "note".to_string(),
        section_type: "identity".to_string(),
        name: "Parent".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({}),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: None,
        template_type: None,
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
        ..Default::default()
    };
    vault.save_object(&parent).unwrap();

    let child = ObjectRecord {
        contract_type_id: None,
        id: "child-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "note".to_string(),
        section_type: "identity".to_string(),
        name: "Child".to_string(),
        icon_name: "document".to_string(),
        parent_id: Some("parent-1".to_string()),
        children_ids: vec![],
        properties: serde_json::json!({}),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: None,
        template_type: None,
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
        ..Default::default()
    };
    vault.save_object(&child).unwrap();

    // Simulate object_create parent update logic
    if let Ok(Some(mut p)) = vault.load_object("parent-1") {
        if !p.children_ids.contains(&"child-1".to_string()) {
            p.children_ids.push("child-1".to_string());
            p.updated_at = chrono::Utc::now().to_rfc3339();
            p.version += 1;
            vault.save_object(&p).unwrap();
        }
    }

    let updated_parent = vault.load_object("parent-1").unwrap().unwrap();
    assert!(updated_parent.children_ids.contains(&"child-1".to_string()));
}

#[test]
fn test_hard_delete_purges_object() {
    let (vault, _dir) = setup_vault();
    let record = ObjectRecord {
        contract_type_id: None,
        id: "obj-purge-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "note".to_string(),
        section_type: "identity".to_string(),
        name: "Purge Me".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({"content": "bye"}),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: None,
        template_type: None,
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
        ..Default::default()
    };
    vault.save_object(&record).unwrap();

    let trash = TrashItem {
        id: "trash_purge_1".to_string(),
        item_type: "object".to_string(),
        original_id: record.id.clone(),
        original_parent_id: None,
        original_section_type: Some(record.section_type.clone()),
        original_sort_order: None,
        data: serde_json::to_vec(&serde_json::json!({"name": "Purge Me"})).unwrap_or_default(),
        deleted_at: chrono::Utc::now().timestamp_millis(),
        expires_at: None,
        deleted_by: "user".to_string(),
        name_snapshot: record.name.clone(),
        icon_snapshot: None,
    };
    vault.save_trash_item(&trash).unwrap();

    // Hard delete object and trash item (object_purge equivalent)
    vault.delete_object(&record.id, false).unwrap();
    vault.delete_trash_item(&trash.id).unwrap();

    assert!(vault.load_object(&record.id).unwrap().is_none());
    assert!(vault.get_trash_item(&trash.id).unwrap().is_none());
}

#[test]
fn test_truncate_preview_properties() {
    // P020：非 `__` 字段截断到前 N 个、`__*` 元数据完整保留、字符串值限长。
    let long = "x".repeat(500);
    let props = serde_json::json!({
        "field1": "v1",
        "field2": "v2",
        "field3": "v3",
        "field4": "v4",
        "field5": "v5",
        "field6": "v6",
        "field7": "v7",
        "field8": "v8",
        "field9": "v9", // 超出 PREVIEW_FIELD_LIMIT，应被截掉
        "__fields": { "field1": { "name": "字段1", "type": "text" } },
        "__templateName": "身份信息",
        "__deprecatedFields": [],
        "huge": long.clone(),
    });
    let out = truncate_preview_properties(&props, None);
    let obj = out.as_object().unwrap();
    // 9 个非 `__` 字段 → 恰好保留 8 个（Map 无序，按计数断言）
    let non_meta: Vec<&String> = obj.keys().filter(|k| !k.starts_with("__")).collect();
    assert_eq!(
        non_meta.len(),
        8,
        "非 __ 字段应截断到 8 个，实际: {non_meta:?}"
    );
    // `__*` 元数据完整保留
    for meta in ["__fields", "__templateName", "__deprecatedFields"] {
        assert!(obj.contains_key(meta), "{meta} 应保留");
    }
    // 所有字符串值限长 200
    for v in obj.values() {
        if let Some(s) = v.as_str() {
            assert!(s.len() <= 200, "字符串值应限长 200");
        }
    }
}

#[test]
fn test_truncate_preview_properties_field_order_priority() {
    // P020 二次复核：提供模板 fieldOrder 时，截断优先按模板顺序选取字段——
    // 模板首位重要字段（字母序靠后）不再被截掉；不足 8 个时再按 Map 序补足。
    let props = serde_json::json!({
        "a_field": "a",
        "b_field": "b",
        "zz_top": "important", // 模板首位，字母序最后
        "c_field": "c",
        "__fields": {},
    });
    let order = vec![
        "zz_top".to_string(),
        "a_field".to_string(),
        "b_field".to_string(),
        "c_field".to_string(),
    ];
    let out = truncate_preview_properties(&props, Some(&order));
    let obj = out.as_object().unwrap();
    // 模板首位字段必须保留（即使字母序靠后）
    assert!(obj.contains_key("zz_top"), "模板首位字段应优先保留");
    assert_eq!(obj.get("zz_top").unwrap(), "important");
    // 全部非 __ 字段都应保留（4 个 < 8 上限）
    for k in ["a_field", "b_field", "c_field"] {
        assert!(obj.contains_key(k), "{k} 应保留");
    }
    assert!(obj.contains_key("__fields"));

    // 超限场景：order 优先填满 8 个，Map 序字段被挤掉
    let props2 = serde_json::json!({
        "f01": "1", "f02": "2", "f03": "3", "f04": "4", "f05": "5",
        "f06": "6", "f07": "7", "f08": "8", "f09": "9", "f10": "10",
    });
    let order2: Vec<String> = (1..=10).map(|i| format!("f{i:02}")).collect();
    let out2 = truncate_preview_properties(&props2, Some(&order2));
    let obj2 = out2.as_object().unwrap();
    let non_meta2: Vec<&String> = obj2.keys().filter(|k| !k.starts_with("__")).collect();
    assert_eq!(non_meta2.len(), 8);
    assert!(obj2.contains_key("f01"), "order 首位应保留");
    assert!(obj2.contains_key("f08"));
    assert!(!obj2.contains_key("f09") && !obj2.contains_key("f10"));
}

/// 字段推荐收集：同名字段跨对象匹配、敏感度解析、排除当前对象/页面/已删除对象、
/// 空值与 `__` 元数据跳过、标量类型转换、值限长。
#[test]
fn test_collect_field_suggestions() {
    let (vault, _dir) = setup_vault();

    // 身份证对象：字段 key 为 citizen_no，__fields 名称「身份证号码」，敏感度 critical（property_labels）
    let id_card = ObjectRecord {
        contract_type_id: None,
        id: "obj-idcard".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "identity".to_string(),
        section_type: "identity".to_string(),
        name: "我的身份证".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({
            "citizen_no": "110101199001011234",
            "birth": "1990-01-01",
            "__fields": {
                "citizen_no": { "name": "身份证号码", "type": "text", "sensitivityLevel": "critical" },
                "birth": { "name": "出生日期", "type": "date" },
            },
        }),
        property_labels: Some(serde_json::json!({ "citizen_no": "critical", "birth": "internal" })),
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: None,
        template_type: None,
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
        ..Default::default()
    };
    vault.save_object(&id_card).unwrap();

    // 身份信息对象：同名字段 key 不同（id_number），无 property_labels → 回退 __fields.sensitivityLevel
    let profile = ObjectRecord {
        contract_type_id: None,
        id: "obj-profile".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "identity".to_string(),
        section_type: "identity".to_string(),
        name: "身份信息".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({
            "id_number": "110101199001011234",
            "empty_field": "",
            "age": 33,
            "has_middle_name": false,
            "note_arr": ["a", "b"],
            "__fields": {
                "id_number": { "name": "身份证号码", "type": "text", "sensitivityLevel": "critical" },
                "empty_field": { "name": "空字段", "type": "text" },
                "age": { "name": "年龄", "type": "number" },
                "note_arr": { "name": "备注列表", "type": "multiselect" },
            },
        }),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: None,
        template_type: None,
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
        ..Default::default()
    };
    vault.save_object(&profile).unwrap();

    // 已删除对象：不应产出推荐
    let deleted = ObjectRecord {
        contract_type_id: None,
        id: "obj-deleted".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "identity".to_string(),
        section_type: "identity".to_string(),
        name: "已删除".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({
            "id_number": "999",
            "__fields": { "id_number": { "name": "身份证号码", "type": "text" } },
        }),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: true,
        deleted_at: Some("2024-01-01T00:00:00Z".to_string()),
        tags_json: vec![],
        template_id: None,
        template_type: None,
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
        ..Default::default()
    };
    vault.save_object(&deleted).unwrap();

    // 自定义页面（type_id = page）：不应产出推荐
    let page = ObjectRecord {
        contract_type_id: None,
        id: "obj-page".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "page".to_string(),
        section_type: "page".to_string(),
        name: "我的页面".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({ "note": "x" }),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: None,
        template_type: None,
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
        ..Default::default()
    };
    vault.save_object(&page).unwrap();

    // 无排除：身份证号码应有两条（身份证对象 + 身份信息对象），不来自已删除/页面对象
    let all = collect_field_suggestions(&vault, "acc-1", None).unwrap();
    let id_no: Vec<&FieldSuggestion> = all
        .iter()
        .filter(|s| s.field_name == "身份证号码")
        .collect();
    assert_eq!(id_no.len(), 2, "同名字段应跨对象聚合：{all:?}");
    assert!(id_no.iter().all(|s| s.value == "110101199001011234"));
    // 敏感度解析：property_labels 优先，回退 __fields.sensitivityLevel
    let idcard_s = id_no.iter().find(|s| s.object_id == "obj-idcard").unwrap();
    let profile_s = id_no.iter().find(|s| s.object_id == "obj-profile").unwrap();
    assert_eq!(idcard_s.sensitivity_level, "critical");
    assert_eq!(profile_s.sensitivity_level, "critical");
    assert_eq!(idcard_s.field_key, "citizen_no");
    assert_eq!(profile_s.field_key, "id_number");

    // 标量转换：number/bool 参与、数组跳过、空字符串跳过、`__` 元数据跳过
    let age = all.iter().find(|s| s.field_name == "年龄").unwrap();
    assert_eq!(age.value, "33");
    let middle = all
        .iter()
        .find(|s| s.field_key == "has_middle_name")
        .unwrap();
    assert_eq!(middle.value, "false");
    assert!(all.iter().all(|s| s.field_key != "empty_field"));
    assert!(all.iter().all(|s| s.field_key != "note_arr"));
    assert!(all.iter().all(|s| !s.field_key.starts_with("__")));

    // 排除当前编辑对象：身份证号码只剩身份信息对象
    let excluded = collect_field_suggestions(&vault, "acc-1", Some("obj-idcard")).unwrap();
    assert!(!excluded.iter().any(|s| s.object_id == "obj-idcard"));
    assert_eq!(
        excluded
            .iter()
            .filter(|s| s.field_name == "身份证号码")
            .count(),
        1
    );

    // 超长值限长
    let long = "x".repeat(5000);
    let long_obj = ObjectRecord {
        contract_type_id: None,
        id: "obj-long".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "note".to_string(),
        section_type: "note".to_string(),
        name: "长文本".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({ "body": long, "__fields": { "body": { "name": "正文", "type": "multiline" } } }),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: None,
        template_type: None,
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
        ..Default::default()
    };
    vault.save_object(&long_obj).unwrap();
    let with_long = collect_field_suggestions(&vault, "acc-1", None).unwrap();
    let body = with_long.iter().find(|s| s.field_name == "正文").unwrap();
    assert_eq!(body.value.len(), SUGGESTION_VALUE_LIMIT);

    // 无字段名回退：key 即字段名（字段定义缺失时）
    let no_def = ObjectRecord {
        contract_type_id: None,
        id: "obj-nodef".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "note".to_string(),
        section_type: "note".to_string(),
        name: "无定义".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({ "plain_key": "v" }),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: None,
        template_type: None,
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
        ..Default::default()
    };
    vault.save_object(&no_def).unwrap();
    let with_nodef = collect_field_suggestions(&vault, "acc-1", None).unwrap();
    let plain = with_nodef
        .iter()
        .find(|s| s.field_key == "plain_key")
        .unwrap();
    assert_eq!(plain.field_name, "plain_key");
    assert_eq!(plain.sensitivity_level, "internal");
}

/// N009: P026 对象输入校验函数边界单测。
#[test]
fn test_validate_object_input_boundaries() {
    // 空名称拒绝
    assert!(validate_object_input("", &serde_json::json!({})).is_err());
    assert!(validate_object_input("   ", &serde_json::json!({})).is_err());

    // 超长名称拒绝（> 200 字符）
    let long_name = "x".repeat(201);
    assert!(validate_object_input(&long_name, &serde_json::json!({})).is_err());
    // 恰好 200 字符放行
    assert!(validate_object_input(&"x".repeat(200), &serde_json::json!({})).is_ok());

    // 超限 properties 载荷拒绝（> 10 MiB）
    let big = "y".repeat(10 * 1024 * 1024 + 1);
    assert!(validate_object_input("obj", &serde_json::json!({ "big": big })).is_err());

    // 正常输入放行
    assert!(validate_object_input("对象名", &serde_json::json!({ "a": 1 })).is_ok());
}
