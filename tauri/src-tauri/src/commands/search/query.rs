use super::*;
use solosoul_core::is_searchable_field_value;
use solosoul_vault::storage::{dynamic_group_label_match, is_internal_metadata_key};

/// 递归遍历对象属性，收集与查询词匹配的字段名/字段值。
///
/// - `protected_keys`：字段 id 集合，这些字段的值不允许参与匹配。
/// - `skip_values`：为 true 时跳过所有字段值匹配（用于对象级敏感度为 sensitive/critical 时）。
///
/// 内部元数据约定（与 `storage::json_contains_ignore_case` 一致）：
/// - `__` 前缀的内部键不按原始键名匹配；`__dynamic_group__` 例外，按用户可见显示名
///   （动态字段组 / Dynamic Group，与前端 locale 同步）匹配；
/// - `__fields` 是字段定义元数据，仅定义中的 `name`（用户可见标签）参与匹配，
///   字段 id 键、`type`/`sensitivityLevel` 等技术值不参与；
/// - `__` 前缀的字符串值视为内部占位 token，不按原始文本匹配。
pub(crate) fn search_properties_for_matches(
    data: &serde_json::Value,
    query: &str,
    path_buf: &mut String,
    protected_keys: &std::collections::HashSet<String>,
    skip_values: bool,
    matches: &mut Vec<FieldMatch>,
) {
    match data {
        serde_json::Value::Object(obj) => {
            // P021: 路径缓冲 push/pop 复用——无命中场景不再为每个 key 做 format! 分配，
            // 仅命中时克隆路径；每轮迭代结束后 truncate 还原（递归自行维护其子路径）。
            let saved_len = path_buf.len();
            for (key, value) in obj {
                // 跳过系统元数据字段，避免搜索结果过多（lower_key 同时供下方字段名匹配复用）
                let lower_key = key.to_lowercase();
                if matches!(
                    lower_key.as_str(),
                    "createdat"
                        | "objectid"
                        | "id"
                        | "updatedat"
                        | "deletedat"
                        | "vaultpath"
                        | "__templatename"
                        | "__templatehash"
                ) {
                    continue;
                }
                if saved_len == 0 {
                    path_buf.push_str(key);
                } else {
                    path_buf.push('.');
                    path_buf.push_str(key);
                }
                let field_path: &str = path_buf.as_str();

                // `__fields` 是字段定义元数据：仅定义中的 name（用户可见标签）参与匹配；
                // 字段 id 键、type/sensitivityLevel 等技术值不参与——否则搜「dynamic_group」
                // 会命中定义中的 type 值、搜「internal」会命中敏感度值等内部 token。
                if key == "__fields" {
                    if let Some(defs) = value.as_object() {
                        for (field_id, def) in defs {
                            let Some(name) = def.get("name").and_then(|v| v.as_str()) else {
                                continue;
                            };
                            if is_internal_metadata_key(name) {
                                // 内部占位名（如 __dynamic_group__）：其搜索面由键路径的
                                // 显示名匹配覆盖（见下方 is_internal_key 分支），此处跳过。
                                continue;
                            }
                            let name_lower = name.to_lowercase();
                            if name_lower.contains(query) {
                                let score = if name_lower == query {
                                    SCORE_EXACT_VALUE
                                } else {
                                    SCORE_FIELD_VALUE
                                };
                                matches.push(FieldMatch {
                                    field_path: format!("{}.{}.name", field_path, field_id),
                                    display_value: name.to_string(),
                                    match_type: FieldMatchType::FieldValue,
                                    score,
                                });
                            }
                        }
                    }
                    path_buf.truncate(saved_len);
                    continue;
                }

                // 字段名匹配：内部元数据键（`__` 前缀）不按原始键名匹配——否则搜
                // 「_dynamic_group_」会命中内部键 `__dynamic_group__`。
                let is_internal_key = is_internal_metadata_key(key);
                if !is_internal_key && lower_key.contains(query) {
                    matches.push(FieldMatch {
                        field_path: field_path.to_string(),
                        display_value: key.clone(),
                        match_type: FieldMatchType::FieldName,
                        score: SCORE_FIELD_NAME,
                    });
                }
                // 内部元数据键按用户可见显示名匹配（当前仅 `__dynamic_group__`
                // → 动态字段组 / Dynamic Group，与前端 locale 同步）。
                if is_internal_key {
                    if let Some(label) = dynamic_group_label_match(query) {
                        matches.push(FieldMatch {
                            field_path: field_path.to_string(),
                            display_value: label.to_string(),
                            match_type: FieldMatchType::FieldName,
                            score: SCORE_FIELD_NAME,
                        });
                    }
                }
                if let serde_json::Value::String(s) = value {
                    // 内部占位 token（`__` 前缀，如 __fields 定义中的 name: "__dynamic_group__"）
                    // 不参与值匹配——其搜索面由键路径的显示名匹配覆盖。
                    // （值小写化保留完整 Unicode 大小写折叠语义，不做长度预检，正确性优先）
                    let value_match =
                        !is_internal_metadata_key(s) && s.to_lowercase().contains(query);
                    if value_match
                        && !skip_values
                        && is_searchable_field_value(field_path, protected_keys)
                    {
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
                            field_path: field_path.to_string(),
                            display_value: truncated,
                            match_type: FieldMatchType::FieldValue,
                            score,
                        });
                    }
                }
                search_properties_for_matches(
                    value,
                    query,
                    path_buf,
                    protected_keys,
                    skip_values,
                    matches,
                );
                path_buf.truncate(saved_len);
            }
        }
        serde_json::Value::Array(arr) => {
            for (i, item) in arr.iter().enumerate() {
                let saved_len = path_buf.len();
                path_buf.push('[');
                path_buf.push_str(&i.to_string());
                path_buf.push(']');
                search_properties_for_matches(
                    item,
                    query,
                    path_buf,
                    protected_keys,
                    skip_values,
                    matches,
                );
                path_buf.truncate(saved_len);
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

/// Count non-deleted child objects for a page (pure SQL COUNT, no decryption).
pub(crate) fn count_page_objects(vault: &VaultStore, account_id: &str, page_id: &str) -> usize {
    vault
        .count_objects(account_id, None, Some(page_id))
        .unwrap_or(0)
}

/// Count non-deleted objects that belong to a system section (identity/travel/etc.).
pub(crate) fn count_section_objects(vault: &VaultStore, account_id: &str, section: &str) -> usize {
    vault
        .count_objects(account_id, Some(section), None)
        .unwrap_or(0)
}

/// Search user templates matching the query.
pub(crate) fn search_templates(
    vault: &VaultStore,
    account_id: &str,
    query: &str,
) -> Result<Vec<SearchResultItem>, String> {
    let q = query.to_lowercase();
    let templates = vault.list_user_templates(account_id)?;
    let mut items: Vec<SearchResultItem> = Vec::new();

    for tpl in templates {
        let name_lower = tpl.name.to_lowercase();
        if name_lower.contains(&q) || q.contains(&name_lower) {
            let score = if name_lower == q {
                SCORE_EXACT_NAME
            } else {
                SCORE_PARTIAL_NAME
            };
            items.push(SearchResultItem {
                object_id: tpl.id,
                name: tpl.name,
                collection_type: "template".to_string(),
                template_name: None,
                template_deleted: false,
                item_type: "template".to_string(),
                parent_id: None,
                field_count: Some(tpl.properties.len()),
                sensitivity_levels: None,
                object_count: None,
                matched_field: None,
                matched_value: None,
                match_type: None,
                relevance: score,
            });
        }
    }

    Ok(items)
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
            template_name: None,
            template_deleted: false,
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
                template_name: None,
                template_deleted: false,
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
