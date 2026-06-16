//! LLM commands: model, config, stats, conversation listing.

use color_eyre::Result;

use crate::app::{App, AppPhase};

/// /model — show current provider and model.
pub fn model(app: &mut App) -> Result<()> {
    let account_id = match app.vault_service.get_current_account() {
        Some(id) => id,
        None => {
            app.error_message = Some("未登录，无法查看 LLM 模型配置".to_string());
            return Ok(());
        }
    };
    let vault = match app.vault_service.get_vault_store() {
        Some(v) => v,
        None => {
            app.error_message = Some("Vault 未解锁".to_string());
            return Ok(());
        }
    };
    match app.llm_service.load_config(&vault, &account_id) {
        Ok(config) => {
            if let Some(provider) = config.active_provider() {
                let msg = format!(
                    "当前模型: {} — {}\n提供商: {}\nAPI 类型: {:?}",
                    provider.name, provider.model, provider.base_url, provider.api_type
                );
                app.error_message = Some(msg);
            } else {
                app.error_message =
                    Some("未设置活跃 LLM 提供商。使用 /llm_config 配置。".to_string());
            }
        }
        Err(e) => {
            app.error_message = Some(format!("加载 LLM 配置失败: {}", e));
        }
    }
    Ok(())
}

/// /llm_config — open LLM configuration screen.
pub fn config(app: &mut App) -> Result<()> {
    let account_id = match app.vault_service.get_current_account() {
        Some(id) => id,
        None => {
            app.error_message = Some("未登录，无法查看 LLM 配置".to_string());
            return Ok(());
        }
    };
    let vault = match app.vault_service.get_vault_store() {
        Some(v) => v,
        None => {
            app.error_message = Some("Vault 未解锁".to_string());
            return Ok(());
        }
    };
    match app.llm_service.load_config(&vault, &account_id) {
        Ok(config) => {
            app.phase = AppPhase::LlmConfig {
                config,
                account_id,
                selected: 0,
            };
        }
        Err(e) => {
            app.error_message = Some(format!("加载 LLM 配置失败: {}", e));
        }
    }
    Ok(())
}

/// /llm_stats — show LLM usage statistics.
pub fn stats(app: &mut App) -> Result<()> {
    let account_id = match app.vault_service.get_current_account() {
        Some(id) => id,
        None => {
            app.error_message = Some("未登录，无法查看 LLM 统计".to_string());
            return Ok(());
        }
    };
    let vault = match app.vault_service.get_vault_store() {
        Some(v) => v,
        None => {
            app.error_message = Some("Vault 未解锁".to_string());
            return Ok(());
        }
    };
    match app.llm_service.load_stats(&vault, &account_id) {
        Ok(stats) => {
            app.phase = AppPhase::LlmStats { stats, selected: 0 };
        }
        Err(e) => {
            app.error_message = Some(format!("加载 LLM 统计失败: {}", e));
        }
    }
    Ok(())
}

/// /llm_list_conversations or /llm_conversations — list conversation history.
pub fn list_conversations(app: &mut App) -> Result<()> {
    let account_id = match app.vault_service.get_current_account() {
        Some(id) => id,
        None => {
            app.error_message = Some("未登录，无法查看对话列表".to_string());
            return Ok(());
        }
    };
    let vault = match app.vault_service.get_vault_store() {
        Some(v) => v,
        None => {
            app.error_message = Some("Vault 未解锁".to_string());
            return Ok(());
        }
    };
    match app
        .llm_service
        .list_conversations(&vault, &account_id)
    {
        Ok(conversations) => {
            app.phase = AppPhase::ConversationList {
                conversations,
                selected: 0,
            };
        }
        Err(e) => {
            app.error_message = Some(format!("加载对话列表失败: {}", e));
        }
    }
    Ok(())
}

/// /llm_chat [conversation-id] — enter chat mode.
pub fn chat(app: &mut App, conversation_id: Option<&str>) -> Result<()> {
    let account_id = match app.vault_service.get_current_account() {
        Some(id) => id,
        None => {
            app.error_message = Some("未登录，无法使用 LLM 聊天".to_string());
            return Ok(());
        }
    };
    let _vault = match app.vault_service.get_vault_store() {
        Some(v) => v,
        None => {
            app.error_message = Some("Vault 未解锁".to_string());
            return Ok(());
        }
    };

    let mut state = crate::screens::llm_chat::LlmChatState::new();
    if let Some(conv_id) = conversation_id {
        // Load existing conversation if specified
        let vault = app.vault_service.get_vault_store().unwrap();
        if let Ok(Some(conv)) = app.llm_service.get_conversation(&vault, &account_id, conv_id) {
            state.messages.clear();
            state.messages.push(crate::screens::llm_chat::ChatLine::System(
                format!("已加载对话: {}", conv.name),
            ));
            for msg in &conv.messages {
                match msg.role.as_str() {
                    "user" => state.messages.push(crate::screens::llm_chat::ChatLine::User(msg.content.clone())),
                    "assistant" => state.messages.push(crate::screens::llm_chat::ChatLine::Assistant(msg.content.clone())),
                    _ => {}
                }
            }
        }
    }

    app.phase = AppPhase::LlmChat;
    app.chat_state = Some(state);
    Ok(())
}

