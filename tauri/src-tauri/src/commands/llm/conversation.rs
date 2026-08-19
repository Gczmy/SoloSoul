use crate::commands::vault_handle;
use crate::state::AppState;
use solosoul_core::llm::service::LlmService;
use solosoul_vault::VaultStore;
use tauri::State;

// ── Conversation storage ──────────────────────────────────

use super::*;

/// 单条对话的最大消息数量，超过此限时自动裁剪最早的消息。
const MAX_CONVERSATION_MESSAGES: usize = 500;

/// 从行级存储读取全部会话明文（P004：不再整 blob 解密；仅解密本账户行）。
/// 首次调用时触发旧 blob 数据懒迁移（见 `migrate_legacy_conversations`）。
pub(crate) fn load_conversations(
    vault: &VaultStore,
    account_id: &str,
) -> Result<Vec<Conversation>, String> {
    migrate_legacy_conversations(vault, account_id)?;
    let rows = vault.list_conversations(account_id)?;
    let mut convs = Vec::with_capacity(rows.len());
    for (_id, _updated, data) in rows {
        if let Ok(c) = serde_json::from_slice::<Conversation>(&data) {
            convs.push(c);
        }
    }
    Ok(convs)
}

/// 懒迁移：旧版本会话存于 profile preferences 的 `llmConversations` blob。
/// 首次进入时把 blob 中的全部会话写入行级表，并清除 blob 键（幂等）。
/// 委托 `LlmService::migrate_legacy_conversations` 共享实现（N005：与 CLI 同一
/// 迁移，且带 LWW 比较，避免无条件 upsert 覆盖 CLI 已写入的较新行）。
fn migrate_legacy_conversations(vault: &VaultStore, account_id: &str) -> Result<(), String> {
    LlmService::new().migrate_legacy_conversations(vault, account_id)
}

/// 裁剪单条对话的消息数量，防止数据无限增长（保存路径使用）。
fn trim_conversation_messages(conv: &mut Conversation) {
    if conv.messages.len() > MAX_CONVERSATION_MESSAGES {
        let excess = conv.messages.len() - MAX_CONVERSATION_MESSAGES;
        conv.messages.drain(..excess);
    }
}

/// 保存单条会话（行级 upsert，P004：不再整 blob 重写）。
/// 返回会话数据是否被裁剪（供调用方判断是否需要重新拉取摘要计数）。
pub(crate) fn save_conversation(
    vault: &VaultStore,
    account_id: &str,
    conversation: &Conversation,
) -> Result<(), String> {
    let mut c = conversation.clone();
    trim_conversation_messages(&mut c);
    let data = serde_json::to_vec(&c).map_err(|e| format!("Serialize: {e}"))?;
    vault.save_conversation(account_id, &c.id, &c.updated_at, &data)?;
    Ok(())
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
    // 先尝试行级单行读取（P004：避免整表加载只为取一条）。
    if let Some(data) = vault.load_conversation(&account_id, &conversation_id)? {
        if let Ok(c) = serde_json::from_slice::<Conversation>(&data) {
            return Ok(c);
        }
    }
    // 兼容旧 blob（迁移前）：回退全量读取查找。
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
    let mut c = conversation;
    c.is_temporary = false;
    save_conversation(&vault, &account_id, &c)
}

#[tauri::command]
pub async fn llm_soft_delete_conversation(
    state: State<'_, AppState>,
    account_id: String,
    conversation_id: String,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    // P008：单行读取，不再整表解密只为更新一条记录
    if let Some(data) = vault.load_conversation(&account_id, &conversation_id)? {
        if let Ok(mut c) = serde_json::from_slice::<Conversation>(&data) {
            c.deleted_at = Some(now_iso());
            save_conversation(&vault, &account_id, &c)?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn llm_restore_conversation(
    state: State<'_, AppState>,
    account_id: String,
    conversation_id: String,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    // P008：单行读取，不再整表解密只为更新一条记录
    if let Some(data) = vault.load_conversation(&account_id, &conversation_id)? {
        if let Ok(mut c) = serde_json::from_slice::<Conversation>(&data) {
            c.deleted_at = None;
            save_conversation(&vault, &account_id, &c)?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn llm_permanent_delete(
    state: State<'_, AppState>,
    account_id: String,
    conversation_id: String,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    // S001：委托服务层——行级删除 + 同步从本设备 blob 值移除该 id，防止下次
    // 懒迁移把已永久删除的会话从保留的 blob 键重新写回（purge 后复活）。
    LlmService::new().permanent_delete_conversation(&vault, &account_id, &conversation_id)
}

#[tauri::command]
pub async fn llm_rename_conversation(
    state: State<'_, AppState>,
    account_id: String,
    conversation_id: String,
    name: String,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    // P008：单行读取，不再整表解密只为更新一条记录
    if let Some(data) = vault.load_conversation(&account_id, &conversation_id)? {
        if let Ok(mut c) = serde_json::from_slice::<Conversation>(&data) {
            c.name = name;
            c.updated_at = now_iso();
            save_conversation(&vault, &account_id, &c)?;
        }
    }
    Ok(())
}
