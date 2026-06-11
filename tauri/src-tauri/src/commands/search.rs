use crate::state::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct SearchResultItem {
    pub object_id: String,
    pub name: String,
    pub collection_type: String,
    pub matched_field: Option<String>,
    pub matched_value: Option<String>,
    pub relevance: f64,
}

#[derive(Serialize)]
pub struct SearchResult {
    pub items: Vec<SearchResultItem>,
    pub total: usize,
    pub has_more: bool,
}

/// Search object properties for field-level query matches.
fn search_properties_for_matches(
    data: &serde_json::Value,
    query: &str,
    current_path: &str,
    matches: &mut Vec<(String, String, f64)>,
) {
    match data {
        serde_json::Value::Object(obj) => {
            for (key, value) in obj {
                let field_path = if current_path.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", current_path, key)
                };
                if key.to_lowercase().contains(query) {
                    matches.push((field_path.clone(), format!("field:{}", key), 2.5));
                }
                if let serde_json::Value::String(s) = value {
                    if s.to_lowercase().contains(query) {
                        let score = if s.len() == query.len() { 5.0 } else { 3.0 };
                        let truncated = if s.len() > 100 {
                            let mut end = 100;
                            while !s.is_char_boundary(end) {
                                end -= 1;
                            }
                            format!("{}...", &s[..end])
                        } else {
                            s.clone()
                        };
                        matches.push((field_path.clone(), truncated, score));
                    }
                }
                search_properties_for_matches(value, query, &field_path, matches);
            }
        }
        serde_json::Value::Array(arr) => {
            for (i, item) in arr.iter().enumerate() {
                search_properties_for_matches(
                    item,
                    query,
                    &format!("{}[{}]", current_path, i),
                    matches,
                );
            }
        }
        _ => {}
    }
}

#[tauri::command]
pub async fn search_advanced(
    state: State<'_, AppState>,
    account_id: String,
    query: String,
    collection_type: Option<String>,
    sensitivity_level: Option<String>,
    limit: Option<usize>,
) -> Result<SearchResult, String> {
    if query.trim().is_empty() {
        return Ok(SearchResult {
            items: vec![],
            total: 0,
            has_more: false,
        });
    }

    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let q = query.to_lowercase();
    let records = vault.search_objects(&account_id, &q)?;
    let mut items: Vec<SearchResultItem> = Vec::new();

    for rec in &records {
        // Apply collection_type filter
        if let Some(ref filter_ct) = collection_type {
            if &rec.type_id != filter_ct {
                continue;
            }
        }

        // Apply sensitivity_level filter
        if let Some(ref filter_sl) = sensitivity_level {
            if &rec.sensitivity_level != filter_sl {
                continue;
            }
        }

        // Collect field-level matches from properties
        let mut field_matches: Vec<(String, String, f64)> = Vec::new();
        search_properties_for_matches(&rec.properties, &q, "", &mut field_matches);

        // Name match bonus
        let name_score = if rec.name.to_lowercase().contains(&q) {
            2.0
        } else {
            0.0
        };

        if !field_matches.is_empty() || name_score > 0.0 {
            if field_matches.is_empty() {
                items.push(SearchResultItem {
                    object_id: rec.id.clone(),
                    name: rec.name.clone(),
                    collection_type: rec.type_id.clone(),
                    matched_field: Some("name".to_string()),
                    matched_value: Some(rec.name.clone()),
                    relevance: name_score,
                });
            } else {
                field_matches
                    .sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
                let best = &field_matches[0];
                items.push(SearchResultItem {
                    object_id: rec.id.clone(),
                    name: rec.name.clone(),
                    collection_type: rec.type_id.clone(),
                    matched_field: Some(best.0.clone()),
                    matched_value: Some(best.1.clone()),
                    relevance: best.2 + name_score,
                });
                for m in field_matches.iter().skip(1).take(3) {
                    items.push(SearchResultItem {
                        object_id: rec.id.clone(),
                        name: rec.name.clone(),
                        collection_type: rec.type_id.clone(),
                        matched_field: Some(m.0.clone()),
                        matched_value: Some(m.1.clone()),
                        relevance: m.2 + name_score,
                    });
                }
            }
        }
    }

    items.sort_by(|a, b| {
        b.relevance
            .partial_cmp(&a.relevance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let limit = limit.unwrap_or(50);
    let has_more = items.len() > limit;
    items.truncate(limit);
    let total = items.len();
    Ok(SearchResult {
        items,
        total,
        has_more,
    })
}

#[tauri::command]
pub async fn search_unified(
    state: State<'_, AppState>,
    account_id: String,
    query: String,
    collection_type: Option<String>,
    parent_id: Option<String>,
    limit: Option<usize>,
) -> Result<SearchResult, String> {
    let limit = limit.unwrap_or(50);
    let trimmed = query.trim();

    // 仅按页面筛选时（无搜索关键词），列出该页面下全部对象
    if trimmed.is_empty() && (collection_type.is_some() || parent_id.is_some()) {
        let summaries = {
            let svc = state.vault_service.read().await;
            let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
            let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
            if let Some(ref ct) = collection_type {
                vault.list_objects(&account_id, Some(ct), None, None, false, false)?
            } else if let Some(ref pid) = parent_id {
                vault.list_objects(&account_id, None, Some(pid), None, false, false)?
            } else {
                vec![]
            }
        };

        let items: Vec<SearchResultItem> = summaries
            .into_iter()
            .map(|s| SearchResultItem {
                object_id: s.id,
                name: s.name,
                collection_type: s.collection_type,
                matched_field: None,
                matched_value: None,
                relevance: 0.0,
            })
            .collect();

        let total = items.len();
        let has_more = items.len() > limit;
        return Ok(SearchResult {
            items: items.into_iter().take(limit).collect(),
            total,
            has_more,
        });
    }

    // 有关键词时走高级搜索，同时应用页面筛选
    search_advanced(state, account_id, query, collection_type, None, Some(limit)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_properties_for_matches_value_match() {
        let data = serde_json::json!({
            "name": "Alice Smith",
            "email": "alice@example.com"
        });
        let mut matches = Vec::new();
        search_properties_for_matches(&data, "alice", "", &mut matches);
        assert!(!matches.is_empty());
        let has_value_match = matches.iter().any(|(_, val, _)| val == "alice@example.com");
        assert!(has_value_match);
    }

    #[test]
    fn test_search_properties_for_matches_field_name_match() {
        let data = serde_json::json!({
            "emailAddress": "test@example.com"
        });
        let mut matches = Vec::new();
        search_properties_for_matches(&data, "email", "", &mut matches);
        let has_field_match = matches
            .iter()
            .any(|(_, val, _)| val == "field:emailAddress");
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
        search_properties_for_matches(&data, "456", "", &mut matches);
        let has_nested = matches.iter().any(|(path, _, _)| path == "contact.phone");
        assert!(has_nested);
    }

    #[test]
    fn test_search_properties_for_matches_array_of_objects() {
        let data = serde_json::json!({
            "items": [{"name": "hello"}, {"name": "world"}]
        });
        let mut matches = Vec::new();
        search_properties_for_matches(&data, "world", "", &mut matches);
        let has_array = matches.iter().any(|(path, _, _)| path == "items[1].name");
        assert!(has_array);
    }

    #[test]
    fn test_search_properties_for_matches_exact_length_bonus() {
        let data = serde_json::json!({
            "code": "abc"
        });
        let mut matches = Vec::new();
        search_properties_for_matches(&data, "abc", "", &mut matches);
        let score = matches
            .iter()
            .find(|(_, val, _)| val == "abc")
            .map(|(_, _, s)| *s)
            .unwrap();
        assert_eq!(score, 5.0);
    }

    #[test]
    fn test_search_properties_for_matches_partial_score() {
        let data = serde_json::json!({
            "code": "abcdef"
        });
        let mut matches = Vec::new();
        search_properties_for_matches(&data, "abc", "", &mut matches);
        let score = matches
            .iter()
            .find(|(_, val, _)| val == "abcdef")
            .map(|(_, _, s)| *s)
            .unwrap();
        assert_eq!(score, 3.0);
    }

    #[test]
    fn test_search_properties_for_matches_truncation() {
        let long = "a".repeat(200);
        let data = serde_json::json!({ "description": long });
        let mut matches = Vec::new();
        search_properties_for_matches(&data, "aaa", "", &mut matches);
        let (_, val, _) = &matches[0];
        assert!(val.ends_with("..."));
        assert!(val.len() <= 104); // 100 chars + "..."
    }

    #[test]
    fn test_search_properties_for_matches_no_match() {
        let data = serde_json::json!({ "name": "Bob" });
        let mut matches = Vec::new();
        search_properties_for_matches(&data, "zzz", "", &mut matches);
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
        search_properties_for_matches(&data, "target", "", &mut matches);
        let has_deep = matches
            .iter()
            .any(|(path, _, _)| path == "level1.level2.level3");
        assert!(has_deep);
    }

    #[test]
    fn test_search_result_serialization() {
        let item = SearchResultItem {
            object_id: "obj-1".to_string(),
            name: "Test".to_string(),
            collection_type: "note".to_string(),
            matched_field: Some("name".to_string()),
            matched_value: Some("Test".to_string()),
            relevance: 3.5,
        };
        let result = SearchResult {
            items: vec![item],
            total: 1,
            has_more: false,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"object_id\":\"obj-1\""));
        assert!(json.contains("\"relevance\":3.5"));
        assert!(json.contains("\"has_more\":false"));
        assert!(json.contains("\"total\":1"));
    }

    #[test]
    fn test_search_result_item_serialization_optional_fields() {
        let item = SearchResultItem {
            object_id: "obj-2".to_string(),
            name: "Minimal".to_string(),
            collection_type: "task".to_string(),
            matched_field: None,
            matched_value: None,
            relevance: 0.0,
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"object_id\":\"obj-2\""));
        assert!(json.contains("\"matched_field\":null"));
        assert!(json.contains("\"matched_value\":null"));
    }
}
