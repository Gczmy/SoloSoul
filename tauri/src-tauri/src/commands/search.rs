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
    limit: Option<usize>,
) -> Result<SearchResult, String> {
    search_advanced(state, account_id, query, None, None, limit).await
}
