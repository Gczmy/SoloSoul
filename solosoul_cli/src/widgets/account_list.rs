//! 账户列表渲染组件。

use ratatui::layout::{Constraint, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Cell, Row, Table};
use solosoul_core::AccountSummary;

/// 将账户列表渲染为 ratatui Table。
pub fn render_table<'a>(accounts: &'a [AccountSummary]) -> Table<'a> {
    let header = Row::new(vec!["账户名", "密码提示", "创建时间"])
        .style(Style::default().bold())
        .bottom_margin(1);

    let rows: Vec<Row> = accounts
        .iter()
        .map(|acc| {
            Row::new(vec![
                Cell::from(acc.name.clone()),
                Cell::from(acc.password_hint.clone().unwrap_or_else(|| "-".into())),
                Cell::from(acc.created_at.clone().unwrap_or_else(|| "-".into())),
            ])
        })
        .collect();

    Table::new(
        rows,
        [
            Constraint::Percentage(30),
            Constraint::Percentage(35),
            Constraint::Percentage(35),
        ],
    )
    .header(header)
    .row_highlight_style(Style::default().reversed())
}

/// 渲染空账户提示。
pub fn render_empty(_area: Rect) -> Text<'static> {
    Text::from(Line::from("未发现本地账户。请使用 GUI 客户端创建账户。").centered())
}
