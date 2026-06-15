//! 顶部状态栏组件。

use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{App, AppPhase};

/// 渲染顶部状态栏。
pub fn render(app: &App) -> Paragraph<'_> {
    let phase_text = match app.phase {
        AppPhase::Welcome => "未登录 · 无账户",
        AppPhase::Locked => "已锁定",
        AppPhase::AccountList { .. } => "账户列表",
        AppPhase::Doctor { .. } => "Doctor",
        AppPhase::Quit => "退出中",
    };

    let lock_text = if app.process_lock.is_some() {
        "🔒 独占"
    } else {
        "⚠ 未独占"
    };

    let spans = vec![
        Span::styled("SoloSoul CLI", Style::default().bold()),
        Span::raw(" | "),
        Span::raw(phase_text),
        Span::raw(" | "),
        Span::raw(lock_text),
    ];

    Paragraph::new(Line::from(spans)).dark_gray()
}
