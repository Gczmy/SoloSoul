use super::*;

#[test]
fn test_search_properties_for_matches_value_match() {
    let data = serde_json::json!({
        "name": "Alice Smith",
        "email": "alice@example.com"
    });
    let mut matches = Vec::new();
    search_properties_for_matches(&data, "alice", "", &HashSet::new(), false, &mut matches);
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
    search_properties_for_matches(&data, "email", "", &HashSet::new(), false, &mut matches);
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
    search_properties_for_matches(&data, "456", "", &HashSet::new(), false, &mut matches);
    let has_nested = matches.iter().any(|m| m.field_path == "contact.phone");
    assert!(has_nested);
}

#[test]
fn test_search_properties_for_matches_array_of_objects() {
    let data = serde_json::json!({
        "items": [{"name": "hello"}, {"name": "world"}]
    });
    let mut matches = Vec::new();
    search_properties_for_matches(&data, "world", "", &HashSet::new(), false, &mut matches);
    let has_array = matches.iter().any(|m| m.field_path == "items[1].name");
    assert!(has_array);
}

#[test]
fn test_search_properties_for_matches_exact_length_bonus() {
    let data = serde_json::json!({
        "code": "abc"
    });
    let mut matches = Vec::new();
    search_properties_for_matches(&data, "abc", "", &HashSet::new(), false, &mut matches);
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
    search_properties_for_matches(&data, "abc", "", &HashSet::new(), false, &mut matches);
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
    search_properties_for_matches(&data, "aaa", "", &HashSet::new(), false, &mut matches);
    let m = &matches[0];
    assert!(m.display_value.ends_with("..."));
    assert!(m.display_value.len() <= 104); // 100 chars + "..."
}

#[test]
fn test_search_properties_for_matches_no_match() {
    let data = serde_json::json!({ "name": "Bob" });
    let mut matches = Vec::new();
    search_properties_for_matches(&data, "zzz", "", &HashSet::new(), false, &mut matches);
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
    search_properties_for_matches(&data, "target", "", &HashSet::new(), false, &mut matches);
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
    search_properties_for_matches(&data, "123456", "", &protected, false, &mut matches);
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
    search_properties_for_matches(&data, "654321", "", &protected, false, &mut matches);
    assert!(!matches
        .iter()
        .any(|m| matches!(m.match_type, FieldMatchType::FieldValue)));
}

#[test]
fn test_search_allows_internal_field_value() {
    let data = serde_json::json!({ "email": "alice@example.com" });
    let mut matches = Vec::new();
    search_properties_for_matches(&data, "alice", "", &HashSet::new(), false, &mut matches);
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
    search_properties_for_matches(&data, "456", "", &protected, false, &mut matches);
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
    search_properties_for_matches(&data, "证件号码", "", &protected, false, &mut matches);
    assert!(matches.iter().any(|m| {
        m.display_value == "证件号码" && matches!(m.match_type, FieldMatchType::FieldValue)
    }));
}

#[test]
fn test_search_skip_values_redacts_object_level_sensitive() {
    let data = serde_json::json!({ "email": "alice@example.com" });
    let mut matches = Vec::new();
    search_properties_for_matches(&data, "alice", "", &HashSet::new(), true, &mut matches);
    assert!(!matches
        .iter()
        .any(|m| matches!(m.match_type, FieldMatchType::FieldValue)));
}
