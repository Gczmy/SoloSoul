//! /account_list 结果展示界面。

use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Clear, Paragraph, Wrap};
use solosoul_core::AccountSummary;

use crate::widgets::account_list::render_table;

/// 渲染账户列表。
pub fn render(frame: &mut ratatui::Frame, area: Rect, accounts: &[AccountSummary]) {
    if accounts.is_empty() {
        let text = Text::from(Line::from("未发现本地账户").centered());
        let paragraph = Paragraph::new(text).alignment(Alignment::Center);
        frame.render_widget(paragraph, area);
        return;
    }

    let table = render_table(accounts);
    frame.render_widget(table, area);
}

/// 渲染提示信息。
pub fn render_help(frame: &mut ratatui::Frame, area: Rect, message: &str) {
    let text = Text::from(Line::from(message));
    let paragraph = Paragraph::new(text)
        .wrap(Wrap { trim: true })
        .style(Style::default().yellow());
    frame.render_widget(Clear, area);
    frame.render_widget(paragraph, area);
}
