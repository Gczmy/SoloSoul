use super::*;
use solosoul_core::{collect_protected_field_keys, is_protected_sensitivity};

/// 高级搜索实现。
///
/// 返回 `(SearchResult, 全量已解密 records)`：调用方（`search_unified`）可复用
/// 第二次全表扫描进行模板归属过滤，避免对全部对象做两次 AES 解密（P007）。
///
/// P114: 本函数无任何 `.await`（纯同步 rusqlite + AES 解密），因此降为普通 fn，
/// 由 `search_unified` 在 `spawn_blocking` 中调用，避免阻塞 tokio worker。
fn search_advanced_impl(
    vault: &solosoul_vault::VaultStore,
    account_id: &str,
    query: &str,
    collection_type: Option<String>,
    sensitivity_level: Option<String>,
    limit: Option<usize>,
) -> Result<(SearchResult, Vec<solosoul_vault::ObjectRecord>), String> {
    // 空查询时仅返回空结果，但保留全量 records 供调用方模板展开（行为与旧实现一致：
    // 旧代码在空查询时也会走 list_objects 全表扫描做模板归属过滤）。
    let q = query.to_lowercase();
    if q.is_empty() {
        let all_records = vault.list_object_records(account_id)?;
        return Ok((
            SearchResult {
                items: vec![],
                total: 0,
                has_more: false,
            },
            all_records,
        ));
    }

    // Pre-load user templates so we can aggregate field-level sensitivities
    // and resolve field labels without N+1 queries.
    let templates: std::collections::HashMap<String, solosoul_vault::UserTemplate> = vault
        .list_user_templates(account_id)?
        .into_iter()
        .map(|t| (t.id.clone(), t))
        .collect();

    // 单次全表解密扫描：内存过滤出查询命中项，同时把全量 records 交回调用方复用。
    let all_records = vault.list_object_records(account_id)?;

    let mut items: Vec<SearchResultItem> = Vec::new();

    // P210: 预筛 properties 用 json_contains_ignore_case 递归匹配，
    // 避免整值 to_string() 往返；精确字段匹配由下方 search_properties_for_matches 裁决。
    for rec in all_records.iter().filter(|r| {
        r.name.to_lowercase().contains(&q)
            || solosoul_vault::storage::json_contains_ignore_case(&r.properties, &q)
    }) {
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

        // 对象级敏感度为 sensitive/critical 时，跳过所有字段值匹配。
        let redact_all = is_protected_sensitivity(&rec.sensitivity_level);
        // 字段级敏感度过滤：property_labels 优先，缺失时回退到模板定义。
        let protected_keys = collect_protected_field_keys(
            rec.property_labels.as_ref(),
            rec.template_id.as_deref(),
            &templates,
        );

        // Collect field-level matches from properties
        let mut field_matches: Vec<FieldMatch> = Vec::new();
        // P021: 路径缓冲复用，避免热循环内每个 key 的 format! 分配
        let mut path_buf = String::new();
        search_properties_for_matches(
            &rec.properties,
            &q,
            &mut path_buf,
            &protected_keys,
            redact_all,
            &mut field_matches,
        );

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
                // P021: max_by 线性取最佳，替代全排序 O(n log n)
                let best = field_matches
                    .iter()
                    .max_by(|a, b| {
                        a.score
                            .partial_cmp(&b.score)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .unwrap();
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
            // 先尝试从模板缓存中获取名称，若模板已删除则回退到 properties 中存储的 __templateName
            let tpl_from_cache = rec
                .template_id
                .as_ref()
                .and_then(|tid| templates.get(tid))
                .map(|t| t.name.clone());
            let tpl_from_props = rec
                .properties
                .get("__templateName")
                .and_then(|v| v.as_str().map(String::from));
            let (tpl_name, tpl_deleted) = resolve_template_display(
                rec.template_id.as_deref(),
                tpl_from_cache,
                tpl_from_props,
            );
            items.push(SearchResultItem {
                object_id: rec.id.clone(),
                name: rec.name.clone(),
                collection_type: rec.type_id.clone(),
                template_name: tpl_name,
                template_deleted: tpl_deleted,
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
    Ok((
        SearchResult {
            items,
            total,
            has_more,
        },
        all_records,
    ))
}

/// P018: 模板显示名解析——模板缓存优先，已删除模板回退到 `__templateName` 属性，
/// 再无则退回 template_id（标记已删除）。`search_advanced_impl` 与页面筛选路径共用。
fn resolve_template_display(
    template_id: Option<&str>,
    tpl_from_cache: Option<String>,
    tpl_from_props: Option<String>,
) -> (Option<String>, bool) {
    match (template_id, tpl_from_cache, tpl_from_props) {
        (Some(_), Some(name), _) => (Some(name), false),
        (Some(_), None, Some(name)) => (Some(name), true),
        (Some(_), None, None) => (template_id.map(String::from), true),
        _ => (None, false),
    }
}

/// P018: 仅按页面/父对象筛选时的列表路径（无搜索关键词）——从 `search_unified` 拆出。
fn search_by_page_only(
    vault: &solosoul_vault::VaultStore,
    account_id: &str,
    collection_type: Option<&String>,
    parent_id: Option<&String>,
    limit: usize,
) -> Result<SearchResult, String> {
    let summaries = if let Some(ct) = collection_type {
        vault.list_objects(account_id, Some(ct), None, None, false, false)?
    } else if let Some(pid) = parent_id {
        vault.list_objects(account_id, None, Some(pid), None, false, false)?
    } else {
        vec![]
    };
    let templates: std::collections::HashMap<String, solosoul_vault::UserTemplate> = vault
        .list_user_templates(account_id)?
        .into_iter()
        .map(|t| (t.id.clone(), t))
        .collect();

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
            let tpl_from_cache = s
                .template_id
                .as_ref()
                .and_then(|tid| templates.get(tid))
                .map(|t| t.name.clone());
            let tpl_from_props = s
                .properties
                .get("__templateName")
                .and_then(|v| v.as_str().map(String::from));
            let (tpl_name, tpl_deleted) =
                resolve_template_display(s.template_id.as_deref(), tpl_from_cache, tpl_from_props);
            SearchResultItem {
                object_id: s.id,
                name: s.name,
                collection_type: s.collection_type,
                template_name: tpl_name,
                template_deleted: tpl_deleted,
                item_type: "object".to_string(),
                parent_id: parent_id.cloned(),
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
    Ok(SearchResult {
        items: items.into_iter().take(limit).collect(),
        total,
        has_more,
    })
}

/// P018: 模板命中时补充「使用该模板的对象」（复用 `search_advanced_impl` 已解密的全量
/// records，不再二次 list_objects）——从 `search_unified` 拆出。
fn expand_template_matches(
    items: &mut Vec<SearchResultItem>,
    all_records: &[solosoul_vault::ObjectRecord],
    matched_templates: &std::collections::HashMap<String, String>,
    collection_type: Option<&String>,
) {
    if matched_templates.is_empty() || all_records.is_empty() {
        return;
    }
    let existing_ids: std::collections::HashSet<String> =
        items.iter().map(|i| i.object_id.clone()).collect();

    for obj in all_records {
        if existing_ids.contains(&obj.id) {
            continue;
        }
        // 如果指定了 collectionType 过滤，仅添加属于该页面的对象
        if let Some(ct) = collection_type {
            if obj.type_id != *ct {
                continue;
            }
        }
        if let Some(ref tid) = obj.template_id {
            if let Some(tpl_name) = matched_templates.get(tid) {
                let field_count = count_object_fields(&obj.properties);
                items.push(SearchResultItem {
                    object_id: obj.id.clone(),
                    name: obj.name.clone(),
                    collection_type: obj.type_id.clone(),
                    template_name: Some(tpl_name.clone()),
                    template_deleted: false,
                    item_type: "object".to_string(),
                    parent_id: obj.parent_id.clone(),
                    field_count: Some(field_count),
                    sensitivity_levels: Some(vec![obj.sensitivity_level.clone()]),
                    object_count: None,
                    matched_field: Some("template".to_string()),
                    matched_value: Some(tpl_name.clone()),
                    match_type: Some("template".to_string()),
                    relevance: SCORE_PARTIAL_NAME,
                });
            }
        }
    }
}

#[tauri::command]
pub async fn search_unified(
    state: State<'_, AppState>,
    account_id: String,
    query: String,
    // P042: 参数名改 type_id——Tauri 参数名默认 camelCase 映射，前端传 typeId
    type_id: Option<String>,
    parent_id: Option<String>,
    limit: Option<usize>,
) -> Result<SearchResult, String> {
    let vault = vault_handle(&state)?;
    let collection_type = type_id;

    // P114: 全表 AES 解密 + 过滤/排序移入 spawn_blocking，避免阻塞 tokio worker。
    tokio::task::spawn_blocking(move || {
        let limit = limit.unwrap_or(DEFAULT_SEARCH_LIMIT);
        let trimmed = query.trim();

        // 仅按页面筛选时（无搜索关键词），列出该页面下全部对象
        if trimmed.is_empty() && (collection_type.is_some() || parent_id.is_some()) {
            return search_by_page_only(
                &vault,
                &account_id,
                collection_type.as_ref(),
                parent_id.as_ref(),
                limit,
            );
        }

        // 有关键词时走高级搜索（返回对象），再合并页面结果。
        // `all_records` 为已解密的全量对象记录，模板归属过滤直接复用它，
        // 避免对全部对象做第二次全表解密（P007）。
        let (mut object_result, all_records) = search_advanced_impl(
            &vault,
            &account_id,
            &query,
            collection_type.clone(),
            None,
            Some(limit),
        )?;

        // 搜索页面和模板（不受 collectionType 影响）
        if parent_id.is_none() {
            // 未按具体页面筛选时，额外搜索页面（系统分区 + 自定义页面）
            if collection_type.is_none() {
                if let Ok(pages) = search_pages(&vault, &account_id, &query) {
                    object_result.items.extend(pages);
                }
            }

            // 始终搜索模板 — 模板不归属于页面，不受 collectionType 影响
            let mut matched_templates: std::collections::HashMap<String, String> =
                std::collections::HashMap::new();
            if let Ok(templates) = search_templates(&vault, &account_id, &query) {
                for t in &templates {
                    matched_templates.insert(t.object_id.clone(), t.name.clone());
                }
                object_result.items.extend(templates);
            }

            // 如果模板匹配，查找使用这些模板的对象（即使名称/字段不包含查询词）。
            // 复用 search_advanced_impl 已解密的全量 records，不再二次 list_objects。
            expand_template_matches(
                &mut object_result.items,
                &all_records,
                &matched_templates,
                collection_type.as_ref(),
            );

            // 重新排序和截断
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
    })
    .await
    .map_err(|e| format!("search_unified task failed: {e}"))?
}
