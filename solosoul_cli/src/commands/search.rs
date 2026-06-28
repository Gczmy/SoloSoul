//! 全局搜索命令 /search。
//!
//! 由于对象属性在 SQLite 中已加密，搜索在解密后的内存数据上进行。
//! 命中 200 条结果后提前截断，避免大 Vault 卡顿。

use std::collections::HashSet;

use color_eyre::Result;
use solosoul_core::{
    collect_protected_field_keys, is_protected_sensitivity, is_searchable_field_value,
    ObjectRecord, UserTemplate, VaultStore,
};

use crate::app::{App, AppPhase};

/// 最大返回结果数。
const RESULT_LIMIT: usize = 200;

/// 搜索结果项。
#[derive(Debug, Clone)]
pub struct SearchResultItem {
    pub object_id: String,
    pub name: String,
    pub collection_type: String,
    /// "page" 或 "object"
    pub item_type: String,
    pub parent_id: Option<String>,
    /// 对象专属：填充字段数
    pub field_count: Option<usize>,
    /// 对象专属：敏感度分级
    pub sensitivity_levels: Option<Vec<String>>,
    /// 页面专属：子对象数量
    pub object_count: Option<usize>,
    /// 匹配的字段路径或 "name"
    pub matched_field: Option<String>,
    /// 匹配的高亮值
    pub matched_value: Option<String>,
    /// "name" | "fieldName" | "fieldValue"
    pub match_type: Option<String>,
    pub relevance: f64,
}

#[derive(Debug, Clone)]
enum FieldMatchType {
    FieldName,
    FieldValue,
}

#[derive(Debug, Clone)]
struct FieldMatch {
    field_path: String,
    display_value: String,
    match_type: FieldMatchType,
    score: f64,
}

fn map_err(e: String) -> color_eyre::Report {
    color_eyre::eyre::eyre!(e)
}

fn require_unlocked(app: &mut App) -> Result<String> {
    if !app.vault_service.is_unlocked() {
        app.error_message = Some("请先使用 /unlock 登录".to_string());
        return Err(color_eyre::eyre::eyre!("Vault is locked"));
    }
    app.vault_service
        .get_current_account()
        .ok_or_else(|| color_eyre::eyre::eyre!("No current account"))
}

fn vault(app: &mut App) -> Result<std::sync::Arc<VaultStore>> {
    app.vault_service
        .get_vault_store()
        .ok_or_else(|| color_eyre::eyre::eyre!("Vault 未打开"))
}

/// 从命令参数中提取搜索关键词。
/// 支持引号包裹多词，例如 `"project alpha"`。
pub fn extract_query(input: Option<&str>) -> Option<String> {
    let input = input?;
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(inner) = trimmed.strip_prefix('"') {
        match inner.find('"') {
            Some(idx) => Some(inner[..idx].to_string()),
            None => Some(inner.to_string()),
        }
    } else {
        Some(trimmed.split_whitespace().next()?.to_string())
    }
}

/// 执行 `/search <关键词>`。
pub fn search(app: &mut App, input: Option<&str>) -> Result<()> {
    let account_id = require_unlocked(app)?;
    let query = match extract_query(input) {
        Some(q) if !q.is_empty() => q,
        _ => {
            app.error_message = Some("请提供搜索关键词，例如 /search 护照".to_string());
            return Ok(());
        }
    };

    let vault = vault(app)?;
    let (items, truncated, total_scanned) = perform_search(&vault, &account_id, &query)?;

    app.previous_phase = Some(app.phase.clone());
    app.phase = AppPhase::SearchResults {
        query,
        items,
        selected: 0,
        truncated,
        total_scanned,
    };
    Ok(())
}

fn perform_search(
    vault: &VaultStore,
    account_id: &str,
    query: &str,
) -> Result<(Vec<SearchResultItem>, bool, usize)> {
    let q = query.to_lowercase();
    let mut items: Vec<SearchResultItem> = Vec::new();
    let mut total_scanned = 0usize;

    // 预加载模板，用于字段级敏感度兜底
    let templates: std::collections::HashMap<String, UserTemplate> = vault
        .list_user_templates(account_id)
        .map_err(map_err)?
        .into_iter()
        .map(|t| (t.id.clone(), t))
        .collect();

    // 1. 页面（系统分区 + 自定义页面）
    let mut page_items = search_pages(vault, account_id, &q)?;
    total_scanned += page_items.len();
    items.append(&mut page_items);

    // 2. 对象：通过 search_objects 获取候选，再细粒度匹配字段
    let records = vault.search_objects(account_id, &q).map_err(map_err)?;
    for rec in records {
        total_scanned += 1;
        if rec.type_id == "page" {
            // 自定义页面已在 search_pages 中通过 list_objects 处理
            continue;
        }
        if items.len() >= RESULT_LIMIT {
            break;
        }
        if let Some(item) = build_object_result(vault, account_id, &rec, &q, &templates) {
            items.push(item);
        }
    }

    let truncated = items.len() >= RESULT_LIMIT;
    items.sort_by(|a, b| {
        b.relevance
            .partial_cmp(&a.relevance)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    items.truncate(RESULT_LIMIT);
    Ok((items, truncated, total_scanned))
}

fn search_pages(
    vault: &VaultStore,
    account_id: &str,
    query: &str,
) -> Result<Vec<SearchResultItem>, color_eyre::Report> {
    let mut items = Vec::new();

    // 自定义页面
    let custom_pages = vault
        .list_objects(account_id, Some("page"), None, Some(query), false, false)
        .map_err(map_err)?;
    for page in custom_pages {
        let score = if page.name.to_lowercase() == query {
            5.0
        } else {
            3.0
        };
        let object_count = vault
            .list_objects(account_id, None, Some(&page.id), None, false, false)
            .map(|v| v.len())
            .unwrap_or(0);
        items.push(SearchResultItem {
            object_id: page.id.clone(),
            name: page.name,
            collection_type: "page".to_string(),
            item_type: "page".to_string(),
            parent_id: None,
            field_count: None,
            sensitivity_levels: None,
            object_count: Some(object_count),
            matched_field: None,
            matched_value: None,
            match_type: None,
            relevance: score,
        });
    }

    // 系统分区
    const SYSTEM_PAGES: &[&str] = &["identity", "travel", "financial", "professional"];
    for section in SYSTEM_PAGES {
        let section_lower = section.to_lowercase();
        if section_lower.contains(query) || query.contains(&section_lower) {
            let object_count = vault
                .list_objects(account_id, Some(section), None, None, false, false)
                .map(|v| v.len())
                .unwrap_or(0);
            items.push(SearchResultItem {
                object_id: section.to_string(),
                name: section.to_string(),
                collection_type: section.to_string(),
                item_type: "page".to_string(),
                parent_id: None,
                field_count: None,
                sensitivity_levels: None,
                object_count: Some(object_count),
                matched_field: None,
                matched_value: None,
                match_type: None,
                relevance: 3.0,
            });
        }
    }

    Ok(items)
}

fn build_object_result(
    vault: &VaultStore,
    _account_id: &str,
    rec: &ObjectRecord,
    query: &str,
    templates: &std::collections::HashMap<String, UserTemplate>,
) -> Option<SearchResultItem> {
    // 对象级敏感度为 sensitive/critical 时，整体跳过字段值匹配。
    let skip_values = is_protected_sensitivity(&rec.sensitivity_level);
    // 字段级敏感度过滤：property_labels 优先，缺失时回退到模板。
    let protected_keys = collect_protected_field_keys(
        rec.property_labels.as_ref(),
        rec.template_id.as_deref(),
        templates,
    );

    let mut field_matches: Vec<FieldMatch> = Vec::new();
    search_properties_for_matches(
        &rec.properties,
        query,
        "",
        &protected_keys,
        skip_values,
        &mut field_matches,
    );

    let name_score = if rec.name.to_lowercase().contains(query) {
        2.0
    } else {
        0.0
    };

    if field_matches.is_empty() && name_score == 0.0 {
        return None;
    }

    let field_count = count_object_fields(&rec.properties);
    let sensitivity_levels = object_sensitivity_levels(rec);

    let (matched_field, matched_value, match_type, relevance) = if field_matches.is_empty() {
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

    // 解析父页面名称（如果存在）
    let parent_name = rec
        .parent_id
        .as_ref()
        .and_then(|pid| vault.load_object(pid).ok().flatten())
        .map(|p| p.name);

    Some(SearchResultItem {
        object_id: rec.id.clone(),
        name: rec.name.clone(),
        collection_type: rec.type_id.clone(),
        item_type: "object".to_string(),
        parent_id: parent_name,
        field_count: Some(field_count),
        sensitivity_levels: Some(sensitivity_levels),
        object_count: None,
        matched_field,
        matched_value,
        match_type,
        relevance,
    })
}

fn search_properties_for_matches(
    data: &serde_json::Value,
    query: &str,
    current_path: &str,
    protected_keys: &HashSet<String>,
    skip_values: bool,
    matches: &mut Vec<FieldMatch>,
) {
    match data {
        serde_json::Value::Object(obj) => {
            for (key, value) in obj {
                // 跳过系统元数据字段，避免搜索结果过多
                let lower_key = key.to_lowercase();
                if lower_key == "createdat"
                    || lower_key == "objectid"
                    || lower_key == "id"
                    || lower_key == "updatedat"
                    || lower_key == "deletedat"
                    || lower_key == "vaultpath"
                    || lower_key == "__templatename"
                {
                    continue;
                }
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
                        score: 2.5,
                    });
                }
                if let serde_json::Value::String(s) = value {
                    if s.to_lowercase().contains(query)
                        && !skip_values
                        && is_searchable_field_value(&field_path, protected_keys)
                    {
                        let score = if s.len() == query.len() { 5.0 } else { 3.0 };
                        let truncated = truncate_value(s, 100);
                        matches.push(FieldMatch {
                            field_path: field_path.clone(),
                            display_value: truncated,
                            match_type: FieldMatchType::FieldValue,
                            score,
                        });
                    }
                }
                search_properties_for_matches(
                    value,
                    query,
                    &field_path,
                    protected_keys,
                    skip_values,
                    matches,
                );
            }
        }
        serde_json::Value::Array(arr) => {
            for (i, item) in arr.iter().enumerate() {
                search_properties_for_matches(
                    item,
                    query,
                    &format!("{}[{}]", current_path, i),
                    protected_keys,
                    skip_values,
                    matches,
                );
            }
        }
        _ => {}
    }
}

fn truncate_value(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let mut end = max_len;
        while !s.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        format!("{}...", &s[..end])
    }
}

fn count_object_fields(properties: &serde_json::Value) -> usize {
    match properties {
        serde_json::Value::Object(map) => map
            .iter()
            .filter(|(k, v)| {
                !k.starts_with("__")
                    && !v.is_null()
                    && **v != serde_json::Value::String(String::new())
            })
            .count(),
        _ => 0,
    }
}

fn object_sensitivity_levels(rec: &ObjectRecord) -> Vec<String> {
    let mut levels = HashSet::new();
    levels.insert(rec.sensitivity_level.clone());
    collect_sensitivity_values(&rec.properties, &mut levels);
    levels.into_iter().collect()
}

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

/// 在搜索结果页中打开当前选中的项。
/// 若选中页面则进入对象列表；若选中对象则打开详情。
pub fn open_selected(app: &mut App) -> Result<()> {
    let account_id = app
        .vault_service
        .get_current_account()
        .ok_or_else(|| color_eyre::eyre::eyre!("未登录"))?;
    let vault = app
        .vault_service
        .get_vault_store()
        .ok_or_else(|| color_eyre::eyre::eyre!("Vault 未打开"))?;

    if let AppPhase::SearchResults {
        items, selected, ..
    } = &app.phase
    {
        let item = items
            .get(*selected)
            .ok_or_else(|| color_eyre::eyre::eyre!("选择项不存在"))?;
        if item.item_type == "page" {
            let objects = if item.object_id.starts_with("page_") {
                vault
                    .list_objects(&account_id, None, Some(&item.object_id), None, false, false)
                    .map_err(map_err)?
            } else {
                vault
                    .list_objects(&account_id, Some(&item.object_id), None, None, false, false)
                    .map_err(map_err)?
            };
            app.previous_phase = Some(app.phase.clone());
            app.phase = AppPhase::ObjectList {
                title: format!("页面: {}", item.name),
                items: objects,
            };
        } else {
            match vault.load_object(&item.object_id).map_err(map_err)? {
                Some(record) if record.account_id == account_id && !record.is_deleted => {
                    app.previous_phase = Some(app.phase.clone());
                    app.phase = AppPhase::ObjectDetail { object: record };
                }
                _ => {
                    app.error_message = Some(format!("对象 '{}' 不存在或已被删除", item.object_id));
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use solosoul_core::{ObjectRecord, VaultService};
    use std::sync::Arc;

    fn unlocked_app() -> (App, String, tempfile::TempDir) {
        let _guard = crate::VAULT_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::TempDir::new().unwrap();
        std::env::set_var("SOLOSOUL_DATA_DIR", dir.path());
        let vault = VaultService::new();
        let account = vault.create_account("Test", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap().to_string();
        let app = App::new(Arc::new(vault)).unwrap();
        (app, account_id, dir)
    }

    #[test]
    fn test_extract_query_plain() {
        assert_eq!(extract_query(Some("护照")), Some("护照".to_string()));
        assert_eq!(
            extract_query(Some("project alpha")),
            Some("project".to_string())
        );
    }

    #[test]
    fn test_extract_query_quoted() {
        assert_eq!(
            extract_query(Some("\"project alpha\"")),
            Some("project alpha".to_string())
        );
        assert_eq!(
            extract_query(Some("\"incomplete")),
            Some("incomplete".to_string())
        );
    }

    #[test]
    fn test_search_properties_for_matches() {
        let data = serde_json::json!({ "email": "alice@example.com" });
        let mut matches = Vec::new();
        search_properties_for_matches(&data, "alice", "", &HashSet::new(), false, &mut matches);
        assert!(matches
            .iter()
            .any(|m| m.display_value == "alice@example.com"));
    }

    #[test]
    fn test_search_properties_for_matches_skips_protected_value() {
        let data = serde_json::json!({
            "idNumber": "123456",
            "email": "alice@example.com"
        });
        let mut protected = HashSet::new();
        protected.insert("idNumber".to_string());

        let mut matches = Vec::new();
        search_properties_for_matches(&data, "123456", "", &protected, false, &mut matches);
        assert!(!matches.iter().any(|m| m.display_value == "123456"));

        let mut matches = Vec::new();
        search_properties_for_matches(&data, "alice", "", &protected, false, &mut matches);
        assert!(matches
            .iter()
            .any(|m| m.display_value == "alice@example.com"));
    }

    #[test]
    fn test_search_properties_for_matches_skip_values_flag() {
        let data = serde_json::json!({ "email": "alice@example.com" });
        let mut matches = Vec::new();
        search_properties_for_matches(&data, "alice", "", &HashSet::new(), true, &mut matches);
        assert!(!matches
            .iter()
            .any(|m| matches!(m.match_type, FieldMatchType::FieldValue)));
    }

    #[test]
    fn test_perform_search_hits_page_and_object() {
        let (mut app, account_id, _dir) = unlocked_app();
        let vault = app.vault_service.get_vault_store().unwrap();

        // 创建页面
        let page = ObjectRecord {
            id: format!("page_{}", uuid::Uuid::new_v4()),
            account_id: account_id.clone(),
            type_id: "page".to_string(),
            section_type: "page".to_string(),
            name: "旅行计划".to_string(),
            icon_name: "folder".to_string(),
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
            contract_type_id: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
        };
        vault.save_object(&page).unwrap();

        // 创建对象
        let obj = ObjectRecord {
            id: format!("obj_{}", uuid::Uuid::new_v4()),
            account_id: account_id.clone(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "护照信息".to_string(),
            icon_name: "document".to_string(),
            parent_id: Some(page.id.clone()),
            children_ids: vec![],
            properties: serde_json::json!({ "number": "E12345678" }),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            contract_type_id: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
        };
        vault.save_object(&obj).unwrap();

        super::search(&mut app, Some("护照")).unwrap();
        match &app.phase {
            AppPhase::SearchResults { items, .. } => {
                assert!(
                    items
                        .iter()
                        .any(|i| i.item_type == "object" && i.name == "护照信息"),
                    "应搜索到对象"
                );
            }
            _ => panic!("expected SearchResults"),
        }
    }

    #[test]
    fn test_search_empty_query() {
        let (mut app, _id, _dir) = unlocked_app();
        super::search(&mut app, None).unwrap();
        assert!(app.error_message.is_some());
    }
}
