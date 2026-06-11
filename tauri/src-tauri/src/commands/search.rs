use crate::state::AppState;
use serde::Serialize;
use solosoul_vault::{ObjectRecord, VaultStore};
use std::collections::HashSet;
use tauri::State;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultItem {
    pub object_id: String,
    pub name: String,
    pub collection_type: String,
    /// "object" or "page"
    pub item_type: String,
    pub parent_id: Option<String>,
    /// Number of populated fields in the object (object results only)
    pub field_count: Option<usize>,
    /// Sensitivity levels present in the object (object results only)
    pub sensitivity_levels: Option<Vec<String>>,
    /// Number of objects inside this page (page results only)
    pub object_count: Option<usize>,
    pub matched_field: Option<String>,
    pub matched_value: Option<String>,
    pub relevance: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
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

/// Count meaningful top-level fields in an object's properties payload.
fn count_object_fields(properties: &serde_json::Value) -> usize {
    match properties {
        serde_json::Value::Object(map) => map
            .iter()
            .filter(|(k, v)| {
                !k.starts_with("__")
                    && !v.is_null()
                    && *v != &serde_json::Value::String(String::new())
            })
            .count(),
        _ => 0,
    }
}

/// Recursively collect string values whose key looks like a sensitivity marker.
fn collect_sensitivity_values(data: &serde_json::Value, out: &mut HashSet<String>) {
    match data {
        serde_json::Value::Object(obj) => {
            for (key, value) in obj {
                if key.to_lowercase().contains("sensitivity") {
                    if let serde_json::Value::String(s) = value {
                        out.insert(s.clone());
                    }
                }
                collect_sensitivity_values(value, out);
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                collect_sensitivity_values(item, out);
            }
        }
        _ => {}
    }
}

/// Build the set of sensitivity levels for an object result.
fn object_sensitivity_levels(rec: &ObjectRecord) -> Vec<String> {
    let mut levels = HashSet::new();
    levels.insert(rec.sensitivity_level.clone());
    collect_sensitivity_values(&rec.properties, &mut levels);
    levels.into_iter().collect()
}

/// Count non-deleted child objects for a page.
fn count_page_objects(vault: &VaultStore, account_id: &str, page_id: &str) -> usize {
    vault
        .list_objects(account_id, None, Some(page_id), None, false, false)
        .map(|v| v.len())
        .unwrap_or(0)
}

/// Count non-deleted objects that belong to a system section (identity/travel/etc.).
fn count_section_objects(vault: &VaultStore, account_id: &str, section: &str) -> usize {
    vault
        .list_objects(account_id, Some(section), None, None, false, false)
        .map(|v| v.len())
        .unwrap_or(0)
}

/// Search pages (system sections + custom pages) matching the query.
fn search_pages(
    vault: &VaultStore,
    account_id: &str,
    query: &str,
) -> Result<Vec<SearchResultItem>, String> {
    let q = query.to_lowercase();
    let mut items: Vec<SearchResultItem> = Vec::new();

    // Custom pages are stored as objects with type_id = "page"
    let custom_pages = vault.list_objects(account_id, Some("page"), None, Some(&q), false, false)?;
    for page in custom_pages {
        let score = if page.name.to_lowercase() == q {
            5.0
        } else {
            3.0
        };
        items.push(SearchResultItem {
            object_id: page.id.clone(),
            name: page.name,
            collection_type: "page".to_string(),
            item_type: "page".to_string(),
            parent_id: None,
            field_count: None,
            sensitivity_levels: None,
            object_count: Some(count_page_objects(vault, account_id, &page.id)),
            matched_field: None,
            matched_value: None,
            relevance: score,
        });
    }

    // System sections
    const SYSTEM_PAGES: &[&str] = &["identity", "travel", "financial", "professional"];
    for section in SYSTEM_PAGES {
        let section_lower = section.to_lowercase();
        if section_lower.contains(&q) || q.contains(&section_lower) {
            items.push(SearchResultItem {
                object_id: section.to_string(),
                name: section.to_string(),
                collection_type: section.to_string(),
                item_type: "page".to_string(),
                parent_id: None,
                field_count: None,
                sensitivity_levels: None,
                object_count: Some(count_section_objects(vault, account_id, section)),
                matched_field: None,
                matched_value: None,
                relevance: 3.0,
            });
        }
    }

    Ok(items)
}

async fn search_advanced_impl(
    state: &AppState,
    account_id: &str,
    query: &str,
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
    let records = vault.search_objects(account_id, &q)?;
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

        // Custom pages are stored as objects with type_id = "page".
        // They are surfaced as page results by search_pages, not as object results here.
        if rec.type_id == "page" {
            continue;
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
            let field_count = count_object_fields(&rec.properties);
            let sensitivity_levels = object_sensitivity_levels(rec);
            // 每个对象只返回一条最佳结果，避免同一对象因多个字段匹配而重复出现
            let (matched_field, matched_value, relevance) = if field_matches.is_empty() {
                (
                    Some("name".to_string()),
                    Some(rec.name.clone()),
                    name_score,
                )
            } else {
                field_matches
                    .sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
                let best = &field_matches[0];
                (
                    Some(best.0.clone()),
                    Some(best.1.clone()),
                    best.2 + name_score,
                )
            };
            items.push(SearchResultItem {
                object_id: rec.id.clone(),
                name: rec.name.clone(),
                collection_type: rec.type_id.clone(),
                item_type: "object".to_string(),
                parent_id: rec.parent_id.clone(),
                field_count: Some(field_count),
                sensitivity_levels: Some(sensitivity_levels),
                object_count: None,
                matched_field,
                matched_value,
                relevance,
            });
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
pub async fn search_advanced(
    state: State<'_, AppState>,
    account_id: String,
    query: String,
    collection_type: Option<String>,
    sensitivity_level: Option<String>,
    limit: Option<usize>,
) -> Result<SearchResult, String> {
    search_advanced_impl(&state, &account_id, &query, collection_type, sensitivity_level, limit).await
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
                item_type: "object".to_string(),
                parent_id: parent_id.clone(),
                field_count: Some(count_object_fields(&s.properties)),
                sensitivity_levels: Some(vec![s.sensitivity_level]),
                object_count: None,
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

    // 有关键词时走高级搜索（返回对象），再合并页面结果
    let mut object_result = search_advanced_impl(
        &state,
        &account_id,
        &query,
        collection_type.clone(),
        None,
        Some(limit),
    )
    .await?;

    // 未按具体页面筛选时，额外搜索页面（系统分区 + 自定义页面）
    if collection_type.is_none() && parent_id.is_none() {
        let svc = state.vault_service.read().await;
        let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
        let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
        match search_pages(vault, &account_id, &query) {
            Ok(pages) => object_result.items.extend(pages),
            Err(_) => {}
        }
        object_result.items.sort_by(|a, b| {
            b.relevance
                .partial_cmp(&a.relevance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let has_more = object_result.items.len() > limit;
        object_result.items.truncate(limit);
        object_result.has_more = has_more;
        object_result.total = object_result.items.len();
    }

    Ok(object_result)
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
            item_type: "object".to_string(),
            parent_id: None,
            field_count: Some(2),
            sensitivity_levels: Some(vec!["internal".to_string()]),
            object_count: None,
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
        assert!(json.contains("\"objectId\":\"obj-1\""));
        assert!(json.contains("\"relevance\":3.5"));
        assert!(json.contains("\"hasMore\":false"));
        assert!(json.contains("\"total\":1"));
    }

    #[test]
    fn test_search_result_item_serialization_optional_fields() {
        let item = SearchResultItem {
            object_id: "obj-2".to_string(),
            name: "Minimal".to_string(),
            collection_type: "task".to_string(),
            item_type: "object".to_string(),
            parent_id: None,
            field_count: Some(0),
            sensitivity_levels: Some(vec![]),
            object_count: None,
            matched_field: None,
            matched_value: None,
            relevance: 0.0,
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"objectId\":\"obj-2\""));
        assert!(json.contains("\"matchedField\":null"));
        assert!(json.contains("\"matchedValue\":null"));
    }
}
