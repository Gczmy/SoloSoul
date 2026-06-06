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

#[tauri::command]
pub async fn search_advanced(
    state: State<'_, AppState>,
    _account_id: String,
    query: String,
    collection_type: Option<String>,
    _sensitivity_level: Option<String>,
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

    let profiles = vault.list_profiles().map_err(|e| e.to_string())?;
    let q = query.to_lowercase();
    let mut items: Vec<SearchResultItem> = Vec::new();

    for p in &profiles {
        // Apply collection_type filter
        if let Some(ref ct) = collection_type {
            if !p.name.to_lowercase().contains(&ct.to_lowercase()) {
                continue;
            }
        }

        let name_score = if p.name.to_lowercase().contains(&q) {
            3.0
        } else {
            0.0
        };

        // Also search in profile data (best-effort)
        let profile_full = vault.load_profile(&p.id).ok().flatten();
        let data_score = if let Some(ref full) = profile_full {
            if let Ok(data_json) = serde_json::from_slice::<serde_json::Value>(&full.data) {
                let data_str = serde_json::to_string(&data_json)
                    .unwrap_or_default()
                    .to_lowercase();
                if data_str.contains(&q) {
                    1.0
                } else {
                    0.0
                }
            } else {
                0.0
            }
        } else {
            0.0
        };

        let score = name_score + data_score;
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
    Ok(SearchResult {
        items,
        total,
        has_more,
    })
}

#[tauri::command]
pub async fn search_unified(
    state: State<'_, AppState>,
    _account_id: String,
    query: String,
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

    let profiles = vault.list_profiles().map_err(|e| e.to_string())?;
    let q = query.to_lowercase();
    let mut items: Vec<SearchResultItem> = Vec::new();

    for p in &profiles {
        let score = if p.name.to_lowercase().contains(&q) {
            2.0
        } else {
            0.0
        };
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
    Ok(SearchResult {
        items,
        total,
        has_more,
    })
}
