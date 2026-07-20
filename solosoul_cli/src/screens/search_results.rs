//! 搜索结果屏幕。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};

use crate::commands::search::SearchResultItem;
use crate::i18n::I18n;
use crate::t;

#[allow(clippy::too_many_arguments)]
pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    query: &str,
    items: &[SearchResultItem],
    selected: usize,
    truncated: bool,
    total_scanned: usize,
    i18n: &I18n,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let mut header_text = t!(
        i18n,
        "search-title",
        query = query,
        count = total_scanned.to_string()
    );
    if truncated {
        header_text.push_str(" · 结果已截断至前 200 条");
    }
    frame.render_widget(
        Paragraph::new(Line::from(header_text).bold()).alignment(Alignment::Center),
        layout[0],
    );

    if items.is_empty() {
        frame.render_widget(
            Paragraph::new(t!(i18n, "search-no-results")).alignment(Alignment::Center),
            layout[1],
        );
    } else {
        let header = Row::new(vec!["类型", "名称", "匹配", "ID"])
            .style(Style::default().bold())
            .bottom_margin(1);
        let rows: Vec<Row> = items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let matched = item
                    .matched_field
                    .as_ref()
                    .zip(item.matched_value.as_ref())
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .unwrap_or_else(|| "-".to_string());
                let marker = if i == selected { "▸ " } else { "  " };
                let cells = vec![
                    format!("{}{}", marker, item.item_type),
                    item.name.clone(),
                    matched,
                    item.object_id.clone(),
                ];
                if i == selected {
                    Row::new(cells).style(Style::default().reversed())
                } else {
                    Row::new(cells)
                }
            })
            .collect();
        let table = Table::new(
            rows,
            [
                Constraint::Percentage(10),
                Constraint::Percentage(25),
                Constraint::Percentage(40),
                Constraint::Percentage(25),
            ],
        )
        .header(header)
        .block(Block::default().title(" /search ").borders(Borders::ALL));
        frame.render_widget(table, layout[1]);
    }

    frame.render_widget(
        Paragraph::new(Line::from(t!(i18n, "hint-up-down-enter-esc")).dark_gray())
            .alignment(Alignment::Center),
        layout[2],
    );
}
