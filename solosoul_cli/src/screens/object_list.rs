//! 页面/对象列表屏幕。

use ratatui::layout::{Alignment, Constraint, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};
use solosoul_core::ObjectSummary;

use crate::i18n::I18n;
use crate::t;

/// 渲染对象列表。
pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    title: &str,
    items: &[ObjectSummary],
    truncated: bool,
    i18n: &I18n,
) {
    if items.is_empty() {
        let text = Text::from(
            Line::from(t!(i18n, "object-list-empty"))
                .dark_gray()
                .alignment(Alignment::Center),
        );
        let paragraph = Paragraph::new(text).alignment(Alignment::Center);
        frame.render_widget(paragraph, area);
        return;
    }

    let header = Row::new(vec![
        t!(i18n, "object-list-table-id"),
        t!(i18n, "object-list-table-name"),
        t!(i18n, "object-list-table-type"),
        t!(i18n, "object-list-table-sensitivity"),
    ])
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

    // R2-V7：截断时在标题附加提示（与 /search 的截断提示语义一致）
    let mut block_title = format!(" {} ", title);
    if truncated {
        block_title.push_str(&t!(i18n, "object-list-truncated"));
    }
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
    .block(Block::default().title(block_title).borders(Borders::ALL));

    frame.render_widget(table, area);
}
