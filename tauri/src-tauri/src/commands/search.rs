use tauri::State;
use serde::Serialize;
use crate::state::AppState;

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

#[tauri::command]
pub async fn search_unified(
    state: State<'_, AppState>,
    _account_id: String,
    query: String,
    limit: Option<usize>,
) -> Result<SearchResult, String> {
    if query.trim().is_empty() {
        return Ok(SearchResult { items: vec![], total: 0, has_more: false });
    }

    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let profiles = vault.list_profiles().map_err(|e| e.to_string())?;
    let q = query.to_lowercase();
    let mut items: Vec<SearchResultItem> = Vec::new();

    for p in &profiles {
        let score = if p.name.to_lowercase().contains(&q) { 2.0 } else { 0.0 };
        if score > 0.0 {
            items.push(SearchResultItem {
                object_id: p.id.clone(),
                name: p.name.clone(),
                collection_type: "profile".to_string(),
                matched_field: Some("name".to_string()),
                matched_value: Some(p.name.clone()),
                relevance: score,
            });
        }
    }

    items.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap());
    let limit = limit.unwrap_or(50);
    let has_more = items.len() > limit;
    items.truncate(limit);

    let total = items.len();
    Ok(SearchResult { items, total, has_more })
}
