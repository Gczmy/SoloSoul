use crate::commands::vault_handle;
use crate::state::AppState;
use serde::Serialize;
use solosoul_vault::{ObjectRecord, VaultStore};
use std::collections::HashSet;
use tauri::State;

/// 默认返回的搜索结果条数。
pub(crate) const DEFAULT_SEARCH_LIMIT: usize = 50;
/// 字段名匹配基础分。
pub(crate) const SCORE_FIELD_NAME: f64 = 2.5;
/// 字段值匹配基础分。
pub(crate) const SCORE_FIELD_VALUE: f64 = 3.0;
/// 完全匹配（长度相等）字段值基础分。
pub(crate) const SCORE_EXACT_VALUE: f64 = 5.0;
/// 页面/对象名称完全匹配分。
pub(crate) const SCORE_EXACT_NAME: f64 = 5.0;
/// 页面/对象名称部分匹配分。
pub(crate) const SCORE_PARTIAL_NAME: f64 = 3.0;
/// 默认对象结果相关性分。
pub(crate) const SCORE_OBJECT_DEFAULT: f64 = 3.0;
/// 对象名称匹配加分。
pub(crate) const SCORE_NAME_BONUS: f64 = 2.0;
/// 字段值展示时的最大字符数。
pub(crate) const MAX_DISPLAY_VALUE_CHARS: usize = 100;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultItem {
    pub object_id: String,
    pub name: String,
    pub collection_type: String,
    /// "object" or "page"
    pub item_type: String,
    pub parent_id: Option<String>,
    /// Name of the template used by this object (object results only)
    pub template_name: Option<String>,
    /// Whether the template referenced by this object has been deleted
    pub template_deleted: bool,
    /// Number of populated fields in the object (object results only)
    pub field_count: Option<usize>,
    /// Sensitivity levels present in the object (object results only)
    pub sensitivity_levels: Option<Vec<String>>,
    /// Number of objects inside this page (page results only)
    pub object_count: Option<usize>,
    pub matched_field: Option<String>,
    pub matched_value: Option<String>,
    /// "fieldName" | "fieldValue" | "name"
    pub match_type: Option<String>,
    pub relevance: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub items: Vec<SearchResultItem>,
    pub total: usize,
    pub has_more: bool,
}

#[derive(Debug, Clone)]
pub(crate) enum FieldMatchType {
    FieldName,
    FieldValue,
}

#[derive(Debug, Clone)]
pub(crate) struct FieldMatch {
    pub(crate) field_path: String,
    pub(crate) display_value: String,
    pub(crate) match_type: FieldMatchType,
    pub(crate) score: f64,
}

// ── Sub-modules ───────────────────────────────────────────

pub mod commands;
pub mod query;
#[cfg(test)]
pub mod tests;

pub(crate) use commands::*;
pub(crate) use query::*;
