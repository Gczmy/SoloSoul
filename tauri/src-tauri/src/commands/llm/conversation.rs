use crate::commands::vault_handle;
use crate::services::profile_prefs::update_profile_prefs;
use crate::state::AppState;
use solosoul_vault::VaultStore;
use tauri::State;

// ── Conversation storage ──────────────────────────────────

use super::*;

/// 单条对话的最大消息数量，超过此限时自动裁剪最早的消息。
const MAX_CONVERSATION_MESSAGES: usize = 500;
pub(crate) fn load_conversations(
    vault: &VaultStore,
    account_id: &str,
) -> Result<Vec<Conversation>, String> {
    match vault.load_profile(account_id) {
        Ok(Some(profile)) => {
            let data: serde_json::Value =
                serde_json::from_slice(&profile.data).map_err(|e| format!("Parse: {}", e))?;
            Ok(data
                .get("preferences")
                .and_then(|p| p.get("llmConversations"))
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default())
        }
        _ => Ok(vec![]),
    }
}

/// 裁剪单条对话的消息数量，防止 Profile 数据无限增长。
fn trim_conversation_messages(conv: &mut Conversation) {
    if conv.messages.len() > MAX_CONVERSATION_MESSAGES {
        let excess = conv.messages.len() - MAX_CONVERSATION_MESSAGES;
        conv.messages.drain(..excess);
    }
}

pub(crate) fn save_conversations(
    vault: &VaultStore,
    account_id: &str,
    conversations: &[Conversation],
) -> Result<(), String> {
    // 裁剪每条对话的消息数量，防止 Profile 数据无限增长
    let mut trimmed = conversations.to_vec();
    for conv in &mut trimmed {
        trim_conversation_messages(conv);
    }

    update_profile_prefs(vault, account_id, |prefs| {
        prefs.insert(
            "llmConversations".to_string(),
            serde_json::to_value(&trimmed).map_err(|e| e.to_string())?,
        );
        Ok(())
    })
}

pub(crate) fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

// ── Conversation IPC commands ─────────────────────────────

#[tauri::command]
pub async fn llm_list_conversations(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<Vec<ConversationSummary>, String> {
    let vault = vault_handle(&state)?;
    let convs = load_conversations(&vault, &account_id)?;
    let mut summaries: Vec<ConversationSummary> = convs
        .into_iter()
        .filter(|c| !c.is_temporary && c.deleted_at.is_none())
        .map(|c| ConversationSummary {
            id: c.id,
            name: c.name,
            updated_at: c.updated_at,
            message_count: c.messages.len(),
            deleted_at: None,
        })
        .collect();
    summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(summaries)
}

#[tauri::command]
pub async fn llm_list_trash(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<Vec<ConversationSummary>, String> {
    let vault = vault_handle(&state)?;
    let convs = load_conversations(&vault, &account_id)?;
    let mut summaries: Vec<ConversationSummary> = convs
        .into_iter()
        .filter(|c| c.deleted_at.is_some())
        .map(|c| ConversationSummary {
            id: c.id,
            name: c.name,
            updated_at: c.updated_at,
            message_count: c.messages.len(),
            deleted_at: c.deleted_at,
        })
        .collect();
    summaries.sort_by(|a, b| {
        b.deleted_at
            .as_deref()
            .unwrap_or(&a.updated_at)
            .cmp(a.deleted_at.as_deref().unwrap_or(&b.updated_at))
    });
    Ok(summaries)
}

#[tauri::command]
pub async fn llm_get_conversation(
    state: State<'_, AppState>,
    account_id: String,
    conversation_id: String,
) -> Result<Conversation, String> {
    let vault = vault_handle(&state)?;
    let convs = load_conversations(&vault, &account_id)?;
    convs
        .into_iter()
        .find(|c| c.id == conversation_id)
        .ok_or_else(|| "Not found".to_string())
}

#[tauri::command]
pub async fn llm_save_conversation(
    state: State<'_, AppState>,
    account_id: String,
    conversation: Conversation,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let mut convs = load_conversations(&vault, &account_id)?;
    let mut c = conversation;
    c.is_temporary = false;
    if let Some(existing) = convs.iter_mut().find(|e| e.id == c.id) {
        *existing = c;
    } else {
        convs.push(c);
    }
    save_conversations(&vault, &account_id, &convs)
}

#[tauri::command]
pub async fn llm_soft_delete_conversation(
    state: State<'_, AppState>,
    account_id: String,
    conversation_id: String,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let mut convs = load_conversations(&vault, &account_id)?;
    if let Some(c) = convs.iter_mut().find(|c| c.id == conversation_id) {
        c.deleted_at = Some(now_iso());
    }
    save_conversations(&vault, &account_id, &convs)
}

#[tauri::command]
pub async fn llm_restore_conversation(
    state: State<'_, AppState>,
    account_id: String,
    conversation_id: String,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let mut convs = load_conversations(&vault, &account_id)?;
    if let Some(c) = convs.iter_mut().find(|c| c.id == conversation_id) {
        c.deleted_at = None;
    }
    save_conversations(&vault, &account_id, &convs)
}

#[tauri::command]
pub async fn llm_permanent_delete(
    state: State<'_, AppState>,
    account_id: String,
    conversation_id: String,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let mut convs = load_conversations(&vault, &account_id)?;
    convs.retain(|c| c.id != conversation_id);
    save_conversations(&vault, &account_id, &convs)
}

#[tauri::command]
pub async fn llm_rename_conversation(
    state: State<'_, AppState>,
    account_id: String,
    conversation_id: String,
    name: String,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let mut convs = load_conversations(&vault, &account_id)?;
    if let Some(c) = convs.iter_mut().find(|c| c.id == conversation_id) {
        c.name = name;
        c.updated_at = now_iso();
    }
    save_conversations(&vault, &account_id, &convs)
}
