//! 顶部状态栏组件。

use std::time::Instant;

use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{App, AppPhase};
use crate::theme::Theme;

/// 渲染顶部状态栏。
pub fn render(app: &App) -> Paragraph<'_> {
    let theme = Theme::load();

    let phase_text = match &app.phase {
        AppPhase::Welcome => "未登录 · 无账户".to_string(),
        AppPhase::Locked => "已锁定".to_string(),
        AppPhase::AccountList { .. } => "账户列表".to_string(),
        AppPhase::UnlockWizard { .. } => "登录".to_string(),
        AppPhase::Home { account_id } => {
            if app.account_name.is_empty() || app.account_name == *account_id {
                format!("已解锁 · {}", account_id)
            } else {
                format!("已解锁 · {} · {}", app.account_name, account_id)
            }
        }
        AppPhase::ObjectList { title, .. } => title.clone(),
        AppPhase::ObjectDetail { object } => format!("对象: {}", object.name),
        AppPhase::Size { .. } => "账户统计".to_string(),
        AppPhase::Doctor { .. } => "Doctor".to_string(),
        AppPhase::NewObjectWizard { .. } => "创建对象向导".to_string(),
        AppPhase::EditObjectWizard { .. } => "编辑对象向导".to_string(),
        AppPhase::TrashList { .. } => "回收站".to_string(),
        AppPhase::Onboarding { .. } => "创建账户".to_string(),
        AppPhase::SearchResults { .. } => "搜索结果".to_string(),
        AppPhase::HistoryList { .. } => "历史快照".to_string(),
        AppPhase::OperationLog { .. } => "审计日志".to_string(),
        AppPhase::About { .. } => "关于".to_string(),
        AppPhase::Help { .. } => "帮助".to_string(),
        AppPhase::AttachmentList { .. } => "附件列表".to_string(),
        AppPhase::BackupList { .. } => "备份列表".to_string(),
        AppPhase::Profile { .. } => "Profile".to_string(),
        AppPhase::TemplateList { .. } => "模板列表".to_string(),
        AppPhase::TemplateDetail { .. } => "模板详情".to_string(),
        AppPhase::LlmConfig { .. } => "LLM 配置".to_string(),
        AppPhase::LlmStats { .. } => "LLM 统计".to_string(),
        AppPhase::ConversationList { .. } => "对话历史".to_string(),
        AppPhase::LlmChat { .. } => "LLM 聊天".to_string(),
        AppPhase::Quit => "退出中".to_string(),
    };

    let lock_text = if app.process_lock.is_some() {
        "[L] 进程锁已持有 · GUI 不可用"
    } else {
        "[!] 未独占"
    };

    let mut spans = vec![
        Span::styled("SoloSoul CLI", theme.style_status_brand()),
        Span::styled(" | ", theme.style_muted()),
        Span::styled(phase_text, theme.style_text()),
        Span::styled(" | ", theme.style_muted()),
        Span::styled(lock_text, theme.style_muted()),
    ];

    // 已登录时显示剩余锁定时间
    if app.vault_service.is_unlocked() {
        let idle = Instant::now().duration_since(app.last_activity);
        let remaining = app.auto_lock_duration.saturating_sub(idle).as_secs();
        spans.push(Span::styled(" | ", theme.style_muted()));
        spans.push(Span::styled(
            format!("锁定倒计时: {}s", remaining),
            theme.style_muted(),
        ));
    }

    // 斜杠命令面板激活时追加操作提示
    if app.command_input.starts_with_slash() {
        spans.push(Span::styled(" | ", theme.style_muted()));
        spans.push(Span::styled(
            "↑↓ 选择 · Enter 确认 · Esc 取消",
            theme.style_cream(),
        ));
    }

    Paragraph::new(Line::from(spans)).style(theme.style_text())
}
