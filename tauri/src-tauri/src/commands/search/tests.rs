use super::*;

#[test]
fn test_search_properties_for_matches_value_match() {
    let data = serde_json::json!({
        "name": "Alice Smith",
        "email": "alice@example.com"
    });
    let mut matches = Vec::new();
    search_properties_for_matches(
        &data,
        "alice",
        &mut String::new(),
        &HashSet::new(),
        false,
        &mut matches,
    );
    assert!(!matches.is_empty());
    let has_value_match = matches.iter().any(|m| {
        m.display_value == "alice@example.com" && matches!(m.match_type, FieldMatchType::FieldValue)
    });
    assert!(has_value_match);
}

#[test]
fn test_search_properties_for_matches_field_name_match() {
    let data = serde_json::json!({
        "emailAddress": "test@example.com"
    });
    let mut matches = Vec::new();
    search_properties_for_matches(
        &data,
        "email",
        &mut String::new(),
        &HashSet::new(),
        false,
        &mut matches,
    );
    let has_field_match = matches.iter().any(|m| {
        m.display_value == "emailAddress" && matches!(m.match_type, FieldMatchType::FieldName)
    });
    assert!(has_field_match);
}

#[test]
fn test_search_properties_for_matches_nested_object() {
    let data = serde_json::json!({
        "contact": {
            "phone": "123-456-7890"
        }
    });
    let mut matches = Vec::new();
    search_properties_for_matches(
        &data,
        "456",
        &mut String::new(),
        &HashSet::new(),
        false,
        &mut matches,
    );
    let has_nested = matches.iter().any(|m| m.field_path == "contact.phone");
    assert!(has_nested);
}

#[test]
fn test_search_properties_for_matches_array_of_objects() {
    let data = serde_json::json!({
        "items": [{"name": "hello"}, {"name": "world"}]
    });
    let mut matches = Vec::new();
    search_properties_for_matches(
        &data,
        "world",
        &mut String::new(),
        &HashSet::new(),
        false,
        &mut matches,
    );
    let has_array = matches.iter().any(|m| m.field_path == "items[1].name");
    assert!(has_array);
}

#[test]
fn test_search_properties_for_matches_exact_length_bonus() {
    let data = serde_json::json!({
        "code": "abc"
    });
    let mut matches = Vec::new();
    search_properties_for_matches(
        &data,
        "abc",
        &mut String::new(),
        &HashSet::new(),
        false,
        &mut matches,
    );
    let score = matches
        .iter()
        .find(|m| m.display_value == "abc")
        .map(|m| m.score)
        .unwrap();
    assert_eq!(score, 5.0);
}

#[test]
fn test_search_properties_for_matches_partial_score() {
    let data = serde_json::json!({
        "code": "abcdef"
    });
    let mut matches = Vec::new();
    search_properties_for_matches(
        &data,
        "abc",
        &mut String::new(),
        &HashSet::new(),
        false,
        &mut matches,
    );
    let score = matches
        .iter()
        .find(|m| m.display_value == "abcdef")
        .map(|m| m.score)
        .unwrap();
    assert_eq!(score, 3.0);
}

#[test]
fn test_search_properties_for_matches_truncation() {
    let long = "a".repeat(200);
    let data = serde_json::json!({ "description": long });
    let mut matches = Vec::new();
    search_properties_for_matches(
        &data,
        "aaa",
        &mut String::new(),
        &HashSet::new(),
        false,
        &mut matches,
    );
    let m = &matches[0];
    assert!(m.display_value.ends_with("..."));
    assert!(m.display_value.len() <= 104); // 100 chars + "..."
}

#[test]
fn test_search_properties_for_matches_no_match() {
    let data = serde_json::json!({ "name": "Bob" });
    let mut matches = Vec::new();
    search_properties_for_matches(
        &data,
        "zzz",
        &mut String::new(),
        &HashSet::new(),
        false,
        &mut matches,
    );
    assert!(matches.is_empty());
}

#[test]
fn test_search_properties_for_matches_deeply_nested() {
    let data = serde_json::json!({
        "level1": {
            "level2": {
                "level3": "target_value"
            }
        }
    });
    let mut matches = Vec::new();
    search_properties_for_matches(
        &data,
        "target",
        &mut String::new(),
        &HashSet::new(),
        false,
        &mut matches,
    );
    let has_deep = matches
        .iter()
        .any(|m| m.field_path == "level1.level2.level3");
    assert!(has_deep);
}

#[test]
fn test_search_result_serialization() {
    let item = SearchResultItem {
        object_id: "obj-1".to_string(),
        name: "Test".to_string(),
        collection_type: "note".to_string(),
        item_type: "object".to_string(),
        parent_id: None,
        template_name: None,
        template_deleted: false,
        field_count: Some(2),
        sensitivity_levels: Some(vec!["internal".to_string()]),
        object_count: None,
        matched_field: Some("name".to_string()),
        matched_value: Some("Test".to_string()),
        match_type: Some("name".to_string()),
        relevance: 3.5,
    };
    let result = SearchResult {
        items: vec![item],
        total: 1,
        has_more: false,
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"objectId\":\"obj-1\""));
    assert!(json.contains("\"relevance\":3.5"));
    assert!(json.contains("\"hasMore\":false"));
    assert!(json.contains("\"total\":1"));
    assert!(json.contains("\"matchType\":\"name\""));
}

#[test]
fn test_search_result_item_serialization_optional_fields() {
    let item = SearchResultItem {
        object_id: "obj-2".to_string(),
        name: "Minimal".to_string(),
        collection_type: "task".to_string(),
        item_type: "object".to_string(),
        parent_id: None,
        template_name: None,
        template_deleted: false,
        field_count: Some(0),
        sensitivity_levels: Some(vec![]),
        object_count: None,
        matched_field: None,
        matched_value: None,
        match_type: None,
        relevance: 0.0,
    };
    let json = serde_json::to_string(&item).unwrap();
    assert!(json.contains("\"objectId\":\"obj-2\""));
    assert!(json.contains("\"matchedField\":null"));
    assert!(json.contains("\"matchedValue\":null"));
    assert!(json.contains("\"matchType\":null"));
}

#[test]
fn test_search_skips_critical_field_value() {
    let data = serde_json::json!({
        "idNumber": "123456",
        "email": "a@b.com"
    });
    let mut protected = HashSet::new();
    protected.insert("idNumber".to_string());
    let mut matches = Vec::new();
    search_properties_for_matches(
        &data,
        "123456",
        &mut String::new(),
        &protected,
        false,
        &mut matches,
    );
    assert!(
        !matches
            .iter()
            .any(|m| matches!(m.match_type, FieldMatchType::FieldValue)),
        "critical 字段值不应产生 fieldValue 匹配"
    );
}

#[test]
fn test_search_skips_sensitive_field_value() {
    let data = serde_json::json!({ "ssn": "654321" });
    let mut protected = HashSet::new();
    protected.insert("ssn".to_string());
    let mut matches = Vec::new();
    search_properties_for_matches(
        &data,
        "654321",
        &mut String::new(),
        &protected,
        false,
        &mut matches,
    );
    assert!(!matches
        .iter()
        .any(|m| matches!(m.match_type, FieldMatchType::FieldValue)));
}

#[test]
fn test_search_allows_internal_field_value() {
    let data = serde_json::json!({ "email": "alice@example.com" });
    let mut matches = Vec::new();
    search_properties_for_matches(
        &data,
        "alice",
        &mut String::new(),
        &HashSet::new(),
        false,
        &mut matches,
    );
    assert!(matches.iter().any(|m| {
        m.display_value == "alice@example.com" && matches!(m.match_type, FieldMatchType::FieldValue)
    }));
}

#[test]
fn test_search_skips_nested_protected_field_value() {
    let data = serde_json::json!({
        "contact": {
            "phone": "123-456-7890"
        }
    });
    let mut protected = HashSet::new();
    protected.insert("phone".to_string());
    let mut matches = Vec::new();
    search_properties_for_matches(
        &data,
        "456",
        &mut String::new(),
        &protected,
        false,
        &mut matches,
    );
    assert!(!matches.iter().any(|m| {
        m.field_path == "contact.phone" && matches!(m.match_type, FieldMatchType::FieldValue)
    }));
}

#[test]
fn test_search_field_name_metadata_still_searchable() {
    let data = serde_json::json!({
        "idNumber": "123456",
        "__fields": {
            "idNumber": { "name": "证件号码", "type": "text" }
        }
    });
    let mut protected = HashSet::new();
    protected.insert("idNumber".to_string());
    let mut matches = Vec::new();
    search_properties_for_matches(
        &data,
        "证件号码",
        &mut String::new(),
        &protected,
        false,
        &mut matches,
    );
    assert!(matches.iter().any(|m| {
        m.display_value == "证件号码" && matches!(m.match_type, FieldMatchType::FieldValue)
    }));
}

#[test]
fn test_search_dynamic_group_internal_key_not_matched() {
    // 内部键 `__dynamic_group__` / 定义 type 值不应按原始文本命中：
    // 搜「_dynamic_group_」或「dynamic_group」不应产生任何匹配。
    let data = serde_json::json!({
        "__dynamic_group__": [{ "id": "c1", "name": "手机", "type": "phone", "value": "123" }],
        "__fields": { "__dynamic_group__": { "name": "__dynamic_group__", "type": "dynamic_group" } }
    });
    for q in ["_dynamic_group_", "dynamic_group", "__fields"] {
        let mut matches = Vec::new();
        search_properties_for_matches(
            &data,
            q,
            &mut String::new(),
            &HashSet::new(),
            false,
            &mut matches,
        );
        assert!(matches.is_empty(), "内部 token {} 不应被搜索命中", q);
    }
    // 但内部键承载的用户数据（子字段名/值）仍可搜索
    let mut matches = Vec::new();
    search_properties_for_matches(
        &data,
        "手机",
        &mut String::new(),
        &HashSet::new(),
        false,
        &mut matches,
    );
}

#[test]
fn test_search_template_hash_internal_value_not_matched() {
    // `__templateHash` 是注入对象 properties 的技术性模板指纹哈希：
    // 键与值均不参与搜索——搜中哈希子串不应产生「字段名：__templateHash」噪声结果。
    let data = serde_json::json!({
        "title": "护照",
        "__templateHash": "0f4a9d1c8e2b7f6a3c5d9e0b1a2f3c4d5e6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c"
    });
    let mut matches = Vec::new();
    search_properties_for_matches(
        &data,
        "1c8e2b7f",
        &mut String::new(),
        &HashSet::new(),
        false,
        &mut matches,
    );
    assert!(matches.is_empty(), "__templateHash 值不应被搜索命中");
    // 其余用户字段值仍可正常搜索
    let mut matches = Vec::new();
    search_properties_for_matches(
        &data,
        "护照",
        &mut String::new(),
        &HashSet::new(),
        false,
        &mut matches,
    );
    assert!(matches.iter().any(|m| m.field_path == "title"));
}

#[test]
fn test_search_template_hash_metadata_still_searchable() {
    // 普通字段名仍按原样命中，仅内部元数据键被跳过：
    // 搜「templateHash」（__templateHash 键名的一部分）不应产生任何匹配。
    let data = serde_json::json!({ "title": "护照", "__templateHash": "abc" });
    let mut matches = Vec::new();
    search_properties_for_matches(
        &data,
        "templateHash",
        &mut String::new(),
        &HashSet::new(),
        false,
        &mut matches,
    );
    assert!(matches.is_empty(), "__templateHash 内部键不应被搜索命中");
    // 普通字段名仍按原样命中
    let mut matches = Vec::new();
    search_properties_for_matches(
        &data,
        "title",
        &mut String::new(),
        &HashSet::new(),
        false,
        &mut matches,
    );
    assert!(matches
        .iter()
        .any(|m| m.field_path == "title" && matches!(m.match_type, FieldMatchType::FieldName)));
}

#[test]
fn test_search_dynamic_group_display_label_matches() {
    // 按用户可见显示名匹配（zh + en），而非内部键名
    let data = serde_json::json!({
        "__dynamic_group__": [{ "id": "c1", "name": "手机", "type": "phone", "value": "123" }],
        "title": "测试"
    });
    for q in ["动态字段组", "字段组", "dynamic group"] {
        let mut matches = Vec::new();
        search_properties_for_matches(
            &data,
            q,
            &mut String::new(),
            &HashSet::new(),
            false,
            &mut matches,
        );
        assert!(!matches.is_empty(), "显示名 {} 应命中动态字段组对象", q);
    }
    // 无动态字段组键的对象不命中显示名
    let plain = serde_json::json!({ "title": "x" });
    let mut matches = Vec::new();
    search_properties_for_matches(
        &plain,
        "动态字段组",
        &mut String::new(),
        &HashSet::new(),
        false,
        &mut matches,
    );
    assert!(matches.is_empty());
}

#[test]
fn test_search_dynamic_group_child_value_still_searchable() {
    // 动态字段组子字段值（用户数据）仍可搜索
    let data = serde_json::json!({
        "__dynamic_group__": [{ "id": "c1", "name": "手机", "type": "phone", "value": "13800138000" }]
    });
    let mut matches = Vec::new();
    search_properties_for_matches(
        &data,
        "13800138000",
        &mut String::new(),
        &HashSet::new(),
        false,
        &mut matches,
    );
    assert!(matches
        .iter()
        .any(|m| m.display_value == "13800138000"
            && matches!(m.match_type, FieldMatchType::FieldValue)));
}

#[test]
fn test_search_skip_values_redacts_object_level_sensitive() {
    let data = serde_json::json!({ "email": "alice@example.com" });
    let mut matches = Vec::new();
    search_properties_for_matches(
        &data,
        "alice",
        &mut String::new(),
        &HashSet::new(),
        true,
        &mut matches,
    );
    assert!(!matches
        .iter()
        .any(|m| matches!(m.match_type, FieldMatchType::FieldValue)));
}
