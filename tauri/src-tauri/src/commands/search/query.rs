use super::*;

pub(crate) fn search_properties_for_matches(
    data: &serde_json::Value,
    query: &str,
    current_path: &str,
    matches: &mut Vec<FieldMatch>,
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
                    matches.push(FieldMatch {
                        field_path: field_path.clone(),
                        display_value: key.clone(),
                        match_type: FieldMatchType::FieldName,
                        score: SCORE_FIELD_NAME,
                    });
                }
                if let serde_json::Value::String(s) = value {
                    if s.to_lowercase().contains(query) {
                        let score = if s.len() == query.len() {
                            SCORE_EXACT_VALUE
                        } else {
                            SCORE_FIELD_VALUE
                        };
                        let truncated = if s.len() > MAX_DISPLAY_VALUE_CHARS {
                            let mut end = MAX_DISPLAY_VALUE_CHARS;
                            while !s.is_char_boundary(end) {
                                end -= 1;
                            }
                            format!("{}...", &s[..end])
                        } else {
                            s.clone()
                        };
                        matches.push(FieldMatch {
                            field_path: field_path.clone(),
                            display_value: truncated,
                            match_type: FieldMatchType::FieldValue,
                            score,
                        });
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
pub(crate) fn count_object_fields(properties: &serde_json::Value) -> usize {
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
pub(crate) fn collect_sensitivity_values(data: &serde_json::Value, out: &mut HashSet<String>) {
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
pub(crate) fn object_sensitivity_levels(
    rec: &ObjectRecord,
    templates: &std::collections::HashMap<String, solosoul_vault::UserTemplate>,
) -> Vec<String> {
    let mut levels = HashSet::new();
    levels.insert(rec.sensitivity_level.clone());
    if let Some(ref tid) = rec.template_id {
        if let Some(tpl) = templates.get(tid) {
            for prop in &tpl.properties {
                if let Some(ref sl) = prop.sensitivity_level {
                    levels.insert(sl.clone());
                }
            }
        }
    }
    collect_sensitivity_values(&rec.properties, &mut levels);
    levels.into_iter().collect()
}

/// Count non-deleted child objects for a page.
pub(crate) fn count_page_objects(vault: &VaultStore, account_id: &str, page_id: &str) -> usize {
    vault
        .list_objects(account_id, None, Some(page_id), None, false, false)
        .map(|v| v.len())
        .unwrap_or(0)
}

/// Count non-deleted objects that belong to a system section (identity/travel/etc.).
pub(crate) fn count_section_objects(vault: &VaultStore, account_id: &str, section: &str) -> usize {
    vault
        .list_objects(account_id, Some(section), None, None, false, false)
        .map(|v| v.len())
        .unwrap_or(0)
}

/// Search pages (system sections + custom pages) matching the query.
pub(crate) fn search_pages(
    vault: &VaultStore,
    account_id: &str,
    query: &str,
) -> Result<Vec<SearchResultItem>, String> {
    let q = query.to_lowercase();
    let mut items: Vec<SearchResultItem> = Vec::new();

    // Custom pages are stored as objects with type_id = "page"
    let custom_pages =
        vault.list_objects(account_id, Some("page"), None, Some(&q), false, false)?;
    for page in custom_pages {
        let score = if page.name.to_lowercase() == q {
            SCORE_EXACT_NAME
        } else {
            SCORE_PARTIAL_NAME
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
            match_type: None,
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
                match_type: None,
                relevance: SCORE_OBJECT_DEFAULT,
            });
        }
    }

    Ok(items)
}
