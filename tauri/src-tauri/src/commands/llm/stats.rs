use crate::commands::vault_handle;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use solosoul_vault::VaultStore;
use std::collections::HashMap;
use tauri::State;

// =============================================================================
// Usage Statistics (§10)
// =============================================================================

use std::sync::Arc;
use tokio::sync::RwLock;

use once_cell::sync::Lazy;
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmUsageStats {
    pub usage_count: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub per_model_stats: Vec<ModelUsage>,
    pub daily_stats: Vec<DailyUsage>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsage {
    pub model: String,
    pub provider: String,
    pub count: u64,
    pub tokens: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub last_used_time: Option<String>, // ISO8601
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyUsage {
    pub date: String, // YYYY-MM-DD
    pub count: u64,
    pub tokens: u64,
    pub per_model_tokens: HashMap<String, u64>, // Key: "provider/model"
}

/// 内存中的使用统计（按账户隔离）
pub static STATS_MAP: Lazy<Arc<RwLock<HashMap<String, LlmUsageStats>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// 估算 Token 数（保守策略：所有字符按 1 token）
pub fn estimate_tokens(text: &str) -> u64 {
    text.chars().count() as u64
}

/// 记录一次 AI 调用（使用真实 token 数）
/// 四层聚合：account totals → per-model → daily (with per-model breakdown)
pub async fn record_usage(
    account_id: &str,
    model: &str,
    provider: &str,
    prompt_tokens: u64,
    completion_tokens: u64,
) {
    let total = prompt_tokens + completion_tokens;
    let now_iso = chrono::Utc::now().to_rfc3339();
    let model_key = format!("{}/{}", provider, model);
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    let mut map: tokio::sync::RwLockWriteGuard<'_, HashMap<String, LlmUsageStats>> =
        STATS_MAP.write().await;
    let stats: &mut LlmUsageStats = map.entry(account_id.to_string()).or_default();

    // 1. Account-level totals
    stats.usage_count += 1;
    stats.prompt_tokens += prompt_tokens;
    stats.completion_tokens += completion_tokens;
    stats.total_tokens += total;

    // 2. Per-model stats
    if let Some(m) = stats
        .per_model_stats
        .iter_mut()
        .find(|m| m.model == model && m.provider == provider)
    {
        m.count += 1;
        m.tokens += total;
        m.prompt_tokens += prompt_tokens;
        m.completion_tokens += completion_tokens;
        m.last_used_time = Some(now_iso.clone());
    } else {
        stats.per_model_stats.push(ModelUsage {
            model: model.to_string(),
            provider: provider.to_string(),
            count: 1,
            tokens: total,
            prompt_tokens,
            completion_tokens,
            last_used_time: Some(now_iso.clone()),
        });
    }

    // 3. Daily stats (with per-model breakdown)
    if let Some(d) = stats.daily_stats.iter_mut().find(|d| d.date == today) {
        d.count += 1;
        d.tokens += total;
        let prev = d.per_model_tokens.get(&model_key).copied().unwrap_or(0);
        d.per_model_tokens.insert(model_key, prev + total);
    } else {
        let mut per_model = HashMap::new();
        per_model.insert(model_key, total);
        stats.daily_stats.push(DailyUsage {
            date: today,
            count: 1,
            tokens: total,
            per_model_tokens: per_model,
        });
    }
}

/// 回退：当 API 未返回真实 token 时，使用估算值
pub async fn record_usage_fallback(
    account_id: &str,
    model: &str,
    provider: &str,
    prompt: &str,
    completion: &str,
) {
    let prompt_tokens = estimate_tokens(prompt);
    let completion_tokens = estimate_tokens(completion);
    record_usage(
        account_id,
        model,
        provider,
        prompt_tokens,
        completion_tokens,
    )
    .await;
}

pub fn save_stats_to_vault(
    vault: &VaultStore,
    account_id: &str,
    stats: &LlmUsageStats,
) -> Result<(), String> {
    let mut profile = match vault.load_profile(account_id) {
        Ok(Some(p)) => p,
        Ok(None) => solosoul_vault::Profile::new_with_id(account_id, account_id, Vec::new()),
        Err(e) => return Err(format!("Load: {}", e)),
    };
    let mut data: serde_json::Value = if profile.data.is_empty() {
        serde_json::Value::Object(serde_json::Map::new())
    } else {
        serde_json::from_slice(&profile.data).map_err(|e| format!("Parse: {}", e))?
    };
    let prefs = data
        .as_object_mut()
        .ok_or("Invalid")?
        .entry("preferences".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    prefs["llmUsageStats"] = serde_json::to_value(stats).map_err(|e| e.to_string())?;
    profile.data = serde_json::to_vec(&data).map_err(|e| e.to_string())?;
    profile.updated_at = chrono::Utc::now();
    profile.version += 1;
    vault.save_profile(&profile)
}

pub fn load_stats_from_vault(
    vault: &VaultStore,
    account_id: &str,
) -> Result<LlmUsageStats, String> {
    match vault.load_profile(account_id) {
        Ok(Some(profile)) => {
            let data: serde_json::Value =
                serde_json::from_slice(&profile.data).map_err(|e| format!("Parse: {}", e))?;
            Ok(data
                .get("preferences")
                .and_then(|p| p.get("llmUsageStats"))
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default())
        }
        _ => Ok(LlmUsageStats::default()),
    }
}

#[tauri::command]
pub async fn llm_get_stats(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<LlmUsageStats, String> {
    // 1. 尝试从内存读取
    {
        let map: tokio::sync::RwLockReadGuard<'_, HashMap<String, LlmUsageStats>> =
            STATS_MAP.read().await;
        if let Some(stats) = map.get(&account_id) {
            return Ok(stats.clone());
        }
    }
    // 2. 内存未命中，从 Vault 加载（严格限定作用域，确保 RwLockGuard 在 await 前 drop）
    let stats: LlmUsageStats = {
        let vault = vault_handle(&state)?;
        load_stats_from_vault(&vault, &account_id)?
    };
    // 3. 加载到内存
    {
        let mut map: tokio::sync::RwLockWriteGuard<'_, HashMap<String, LlmUsageStats>> =
            STATS_MAP.write().await;
        map.insert(account_id.clone(), stats.clone());
    }
    Ok(stats)
}

#[tauri::command]
pub async fn llm_reset_stats(state: State<'_, AppState>, account_id: String) -> Result<(), String> {
    {
        let mut map: tokio::sync::RwLockWriteGuard<'_, HashMap<String, LlmUsageStats>> =
            STATS_MAP.write().await;
        map.remove(&account_id);
    }
    let vault = vault_handle(&state)?;
    save_stats_to_vault(&vault, &account_id, &LlmUsageStats::default())
}

/// 将指定账户的统计持久化到 Vault（debounce 保存由调用方管理）
pub async fn persist_stats(account_id: &str, vault: &VaultStore) -> Result<(), String> {
    let stats: LlmUsageStats = {
        let map: tokio::sync::RwLockReadGuard<'_, HashMap<String, LlmUsageStats>> =
            STATS_MAP.read().await;
        map.get(account_id).cloned().unwrap_or_default()
    };
    save_stats_to_vault(vault, account_id, &stats)
}

#[tauri::command]
pub async fn llm_persist_stats(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<(), String> {
    let stats: LlmUsageStats = {
        let map: tokio::sync::RwLockReadGuard<'_, HashMap<String, LlmUsageStats>> =
            STATS_MAP.read().await;
        map.get(&account_id).cloned().unwrap_or_default()
    };
    let vault = vault_handle(&state)?;
    save_stats_to_vault(&vault, &account_id, &stats)
}
