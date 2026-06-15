//! 页面/对象列表屏幕。

use ratatui::layout::{Alignment, Constraint, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};
use solosoul_core::ObjectSummary;

/// 渲染对象列表。
pub fn render(frame: &mut ratatui::Frame, area: Rect, title: &str, items: &[ObjectSummary]) {
    if items.is_empty() {
        let text = Text::from(
            Line::from("暂无内容")
                .dark_gray()
                .alignment(Alignment::Center),
        );
        let paragraph = Paragraph::new(text).alignment(Alignment::Center);
        frame.render_widget(paragraph, area);
        return;
    }

    let header = Row::new(vec!["ID", "名称", "类型", "敏感度"])
        .style(Style::default().bold())
        .bottom_margin(1);

    let rows: Vec<Row> = items
        .iter()
        .map(|item| {
            Row::new(vec![
                item.id.clone(),
                item.name.clone(),
                item.collection_type.clone(),
                item.sensitivity_level.clone(),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(35),
            Constraint::Percentage(30),
            Constraint::Percentage(20),
            Constraint::Percentage(15),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title(format!(" {} ", title))
            .borders(Borders::ALL),
    );

    frame.render_widget(table, area);
}
