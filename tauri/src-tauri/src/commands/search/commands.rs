use super::*;

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

    let vault = vault_handle(state)?;

    // Pre-load user templates so we can aggregate field-level sensitivities
    // and resolve field labels without N+1 queries.
    let templates: std::collections::HashMap<String, solosoul_vault::UserTemplate> = vault
        .list_user_templates(account_id)?
        .into_iter()
        .map(|t| (t.id.clone(), t))
        .collect();

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
        let mut field_matches: Vec<FieldMatch> = Vec::new();
        search_properties_for_matches(&rec.properties, &q, "", &mut field_matches);

        // Name match bonus
        let name_score = if rec.name.to_lowercase().contains(&q) {
            SCORE_NAME_BONUS
        } else {
            0.0
        };

        if !field_matches.is_empty() || name_score > 0.0 {
            let field_count = count_object_fields(&rec.properties);
            let sensitivity_levels = object_sensitivity_levels(rec, &templates);
            // 每个对象只返回一条最佳结果，避免同一对象因多个字段匹配而重复出现
            let (matched_field, matched_value, match_type, relevance) = if field_matches.is_empty()
            {
                (
                    Some("name".to_string()),
                    Some(rec.name.clone()),
                    Some("name".to_string()),
                    name_score,
                )
            } else {
                field_matches.sort_by(|a, b| {
                    b.score
                        .partial_cmp(&a.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                let best = &field_matches[0];
                (
                    Some(best.field_path.clone()),
                    Some(best.display_value.clone()),
                    Some(match best.match_type {
                        FieldMatchType::FieldName => "fieldName".to_string(),
                        FieldMatchType::FieldValue => "fieldValue".to_string(),
                    }),
                    best.score + name_score,
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
                match_type,
                relevance,
            });
        }
    }

    items.sort_by(|a, b| {
        b.relevance
            .partial_cmp(&a.relevance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let limit = limit.unwrap_or(DEFAULT_SEARCH_LIMIT);
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
    search_advanced_impl(
        &state,
        &account_id,
        &query,
        collection_type,
        sensitivity_level,
        limit,
    )
    .await
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
    let limit = limit.unwrap_or(DEFAULT_SEARCH_LIMIT);
    let trimmed = query.trim();

    // 仅按页面筛选时（无搜索关键词），列出该页面下全部对象
    if trimmed.is_empty() && (collection_type.is_some() || parent_id.is_some()) {
        let (summaries, templates) = {
            let vault = vault_handle(&state)?;
            let summaries = if let Some(ref ct) = collection_type {
                vault.list_objects(&account_id, Some(ct), None, None, false, false)?
            } else if let Some(ref pid) = parent_id {
                vault.list_objects(&account_id, None, Some(pid), None, false, false)?
            } else {
                vec![]
            };
            let templates: std::collections::HashMap<String, solosoul_vault::UserTemplate> = vault
                .list_user_templates(&account_id)?
                .into_iter()
                .map(|t| (t.id.clone(), t))
                .collect();
            (summaries, templates)
        };

        let items: Vec<SearchResultItem> = summaries
            .into_iter()
            .map(|s| {
                let mut levels = HashSet::new();
                levels.insert(s.sensitivity_level.clone());
                if let Some(ref tid) = s.template_id {
                    if let Some(tpl) = templates.get(tid) {
                        for prop in &tpl.properties {
                            if let Some(ref sl) = prop.sensitivity_level {
                                levels.insert(sl.clone());
                            }
                        }
                    }
                }
                SearchResultItem {
                    object_id: s.id,
                    name: s.name,
                    collection_type: s.collection_type,
                    item_type: "object".to_string(),
                    parent_id: parent_id.clone(),
                    field_count: Some(count_object_fields(&s.properties)),
                    sensitivity_levels: Some(levels.into_iter().collect()),
                    object_count: None,
                    matched_field: None,
                    matched_value: None,
                    match_type: None,
                    relevance: 0.0,
                }
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

    // 未按具体页面筛选时，额外搜索页面（系统分区 + 自定义页面）和模板
    if collection_type.is_none() && parent_id.is_none() {
        let vault = vault_handle(&state)?;
        if let Ok(pages) = search_pages(&vault, &account_id, &query) {
            object_result.items.extend(pages);
        }
        if let Ok(templates) = search_templates(&vault, &account_id, &query) {
            object_result.items.extend(templates);
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
