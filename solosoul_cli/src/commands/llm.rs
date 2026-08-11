//! LLM commands: model, config, stats, conversation listing.

use color_eyre::Result;

use crate::app::{App, AppPhase};
use crate::t;

/// /model — show current provider and model.
pub fn model(app: &mut App) -> Result<()> {
    let account_id = match app.vault_service.get_current_account() {
        Some(id) => id,
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-llm-need-login"));
            return Ok(());
        }
    };
    let vault = match app.vault_service.get_vault_store() {
        Some(v) => v,
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-llm-vault-locked"));
            return Ok(());
        }
    };
    match app.llm_service.load_config(&vault, &account_id) {
        Ok(config) => {
            if let Some(provider) = config.active_provider() {
                let msg = t!(
                    app.i18n,
                    "cmd-llm-current-model",
                    name = &provider.name,
                    model = &provider.model,
                    url = &provider.base_url,
                    api_type = &format!("{:?}", provider.api_type)
                );
                app.info_message = Some(msg);
            } else {
                app.info_message = Some(t!(app.i18n, "cmd-llm-no-active-provider"));
            }
        }
        Err(e) => {
            app.error_message = Some(t!(app.i18n, "cmd-llm-config-failed", err = e));
        }
    }
    Ok(())
}

/// /llm_config — open LLM configuration screen.
pub fn config(app: &mut App) -> Result<()> {
    let account_id = match app.vault_service.get_current_account() {
        Some(id) => id,
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-llm-need-login"));
            return Ok(());
        }
    };
    let vault = match app.vault_service.get_vault_store() {
        Some(v) => v,
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-llm-vault-locked"));
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
            app.error_message = Some(t!(app.i18n, "cmd-llm-config-failed", err = e));
        }
    }
    Ok(())
}

/// /llm_stats — show LLM usage statistics.
pub fn stats(app: &mut App) -> Result<()> {
    let account_id = match app.vault_service.get_current_account() {
        Some(id) => id,
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-llm-need-login"));
            return Ok(());
        }
    };
    let vault = match app.vault_service.get_vault_store() {
        Some(v) => v,
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-llm-vault-locked"));
            return Ok(());
        }
    };
    match app.llm_service.load_stats(&vault, &account_id) {
        Ok(stats) => {
            app.phase = AppPhase::LlmStats { stats, selected: 0 };
        }
        Err(e) => {
            app.error_message = Some(t!(app.i18n, "cmd-llm-stats-failed", err = e));
        }
    }
    Ok(())
}

/// /llm_list_conversations or /llm_conversations — list conversation history.
pub fn list_conversations(app: &mut App) -> Result<()> {
    let account_id = match app.vault_service.get_current_account() {
        Some(id) => id,
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-llm-need-login"));
            return Ok(());
        }
    };
    let vault = match app.vault_service.get_vault_store() {
        Some(v) => v,
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-llm-vault-locked"));
            return Ok(());
        }
    };
    // LlmService::list_conversations 内部自动懒迁移旧 blob 会话（与 GUI 共用实现，幂等）。
    match app.llm_service.list_conversations(&vault, &account_id) {
        Ok(conversations) => {
            app.phase = AppPhase::ConversationList {
                conversations,
                selected: 0,
            };
        }
        Err(e) => {
            app.error_message = Some(t!(app.i18n, "cmd-llm-list-failed", err = e));
        }
    }
    Ok(())
}

/// /llm_chat [conversation-id] — enter chat mode.
pub fn chat(app: &mut App, conversation_id: Option<&str>) -> Result<()> {
    let account_id = match app.vault_service.get_current_account() {
        Some(id) => id,
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-llm-need-login-chat"));
            return Ok(());
        }
    };
    if app.vault_service.get_vault_store().is_none() {
        app.error_message = Some(t!(app.i18n, "cmd-llm-vault-locked"));
        return Ok(());
    }

    let mut state = crate::screens::llm_chat::LlmChatState::new(&app.i18n);
    if let Some(conv_id) = conversation_id {
        // Load existing conversation if specified
        let vault = app
            .vault_service
            .get_vault_store()
            .expect("Vault 已校验解锁");
        if let Ok(Some(conv)) = app
            .llm_service
            .get_conversation(&vault, &account_id, conv_id)
        {
            state.messages.clear();
            state
                .messages
                .push(crate::screens::llm_chat::ChatLine::System(t!(
                    app.i18n,
                    "cmd-llm-loaded-conversation",
                    name = &conv.name
                )));
            for msg in &conv.messages {
                match msg.role.as_str() {
                    "user" => state
                        .messages
                        .push(crate::screens::llm_chat::ChatLine::User(
                            msg.content.clone(),
                        )),
                    "assistant" => {
                        state
                            .messages
                            .push(crate::screens::llm_chat::ChatLine::Assistant(
                                msg.content.clone(),
                            ))
                    }
                    _ => {}
                }
            }
        }
    }

    app.phase = AppPhase::LlmChat;
    app.chat_state = Some(state);
    Ok(())
}
