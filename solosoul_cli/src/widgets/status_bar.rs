//! 顶部状态栏组件。

use std::time::Instant;

use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{App, AppPhase};
use crate::t;
use crate::theme::Theme;

/// 渲染顶部状态栏。
pub fn render(app: &App) -> Paragraph<'_> {
    let theme = Theme::load();

    let phase_text = match &app.phase {
        AppPhase::Welcome => t!(app.i18n, "status-welcome"),
        AppPhase::Locked => t!(app.i18n, "status-locked"),
        AppPhase::AccountList { .. } => t!(app.i18n, "status-account-list"),
        AppPhase::UnlockWizard { .. } => t!(app.i18n, "status-unlock"),
        AppPhase::Home { account_id } => {
            if app.account_name.is_empty() || app.account_name == *account_id {
                t!(app.i18n, "status-unlocked", id = account_id)
            } else {
                t!(
                    app.i18n,
                    "status-unlocked-with-name",
                    name = &app.account_name,
                    id = account_id
                )
            }
        }
        AppPhase::ObjectList { title, .. } => title.clone(),
        AppPhase::ObjectDetail { object } => t!(app.i18n, "status-object", name = &object.name),
        AppPhase::Size { .. } => t!(app.i18n, "status-size"),
        AppPhase::Doctor { .. } => t!(app.i18n, "status-doctor"),
        AppPhase::NewObjectWizard { .. } => t!(app.i18n, "status-new-object"),
        AppPhase::EditObjectWizard { .. } => t!(app.i18n, "status-edit-object"),
        AppPhase::TrashList { .. } => t!(app.i18n, "status-trash"),
        AppPhase::Onboarding { .. } => t!(app.i18n, "status-onboarding"),
        AppPhase::SearchResults { .. } => t!(app.i18n, "status-search"),
        AppPhase::HistoryList { .. } => t!(app.i18n, "status-history"),
        AppPhase::OperationLog { .. } => t!(app.i18n, "status-operation-log"),
        AppPhase::About { .. } => t!(app.i18n, "status-about"),
        AppPhase::Help { .. } => t!(app.i18n, "status-help"),
        AppPhase::AttachmentList { .. } => t!(app.i18n, "status-attachment"),
        AppPhase::BackupList { .. } => t!(app.i18n, "status-backup"),
        AppPhase::Profile { .. } => t!(app.i18n, "status-profile"),
        AppPhase::TemplateList { .. } => t!(app.i18n, "status-template-list"),
        AppPhase::TemplateDetail { .. } => t!(app.i18n, "status-template-detail"),
        AppPhase::LlmConfig { .. } => t!(app.i18n, "status-llm-config"),
        AppPhase::LlmStats { .. } => t!(app.i18n, "status-llm-stats"),
        AppPhase::ConversationList { .. } => t!(app.i18n, "status-conversation"),
        AppPhase::LlmChat => t!(app.i18n, "status-llm-chat"),
        AppPhase::PluginList { .. } => t!(app.i18n, "status-plugin-list"),
        AppPhase::PluginDetail { .. } => t!(app.i18n, "status-plugin-detail"),
        AppPhase::SyncStatus { .. } => t!(app.i18n, "status-sync"),
        AppPhase::OcrResult { .. } => t!(app.i18n, "status-ocr"),
        AppPhase::EmbedModelList { .. } => t!(app.i18n, "status-embed"),
        AppPhase::SettingsMenu {
            current_language,
            current_theme,
            ..
        } => {
            t!(
                app.i18n,
                "status-settings",
                lang = current_language,
                theme = current_theme
            )
        }
        AppPhase::SettingsLanguageSelect { .. } => t!(app.i18n, "status-settings-language"),
        AppPhase::SettingsThemeSelect { .. } => t!(app.i18n, "status-settings-theme"),
        AppPhase::SettingsPreferenceEdit => t!(app.i18n, "status-settings-preference"),
        AppPhase::Quit => t!(app.i18n, "status-quit"),
    };

    let lock_text = if app.process_lock.is_some() {
        t!(app.i18n, "status-lock-held")
    } else {
        t!(app.i18n, "status-lock-not-exclusive")
    };

    let mut spans = vec![
        Span::styled("SoloSoul CLI", theme.style_status_brand()),
        Span::styled(" | ", theme.style_muted()),
        Span::styled(phase_text, theme.style_text()),
        Span::styled(" | ", theme.style_muted()),
        Span::styled(lock_text, theme.style_muted()),
    ];

    // 已登录时显示剩余锁定时间（<60 秒时橘红色强调提醒）
    if app.vault_service.is_unlocked() {
        let idle = Instant::now().duration_since(app.last_activity);
        let remaining = app.auto_lock_duration.saturating_sub(idle).as_secs();
        let countdown_style = if remaining < 60 {
            theme.style_error() // 橘红色强调
        } else {
            theme.style_muted()
        };
        spans.push(Span::styled(" | ", theme.style_muted()));
        spans.push(Span::styled(
            t!(
                app.i18n,
                "status-lock-countdown",
                sec = &remaining.to_string()
            ),
            countdown_style,
        ));
    }

    // 斜杠命令面板激活时追加操作提示
    if app.command_input.starts_with_slash() {
        spans.push(Span::styled(" | ", theme.style_muted()));
        spans.push(Span::styled(
            t!(app.i18n, "hint-up-down-enter-esc"),
            theme.style_cream(),
        ));
    }

    // 成功 toast（5 秒自动过期）。在 phase_text 之后追加，避免与锁定倒计时重叠。
    if let Some((text, ts)) = &app.success_message {
        if ts.elapsed() < std::time::Duration::from_secs(5) {
            spans.push(Span::styled(" | ", theme.style_muted()));
            spans.push(Span::styled(text.clone(), theme.style_success()));
        }
    }

    Paragraph::new(Line::from(spans)).style(theme.style_text())
}
