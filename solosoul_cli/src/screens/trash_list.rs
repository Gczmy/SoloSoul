//! 回收站列表屏幕。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, HighlightSpacing, Paragraph, Row, Table};
use solosoul_core::TrashItemSummary;

pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    items: &[TrashItemSummary],
    selected: usize,
    selected_ids: &[String],
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let title = Paragraph::new(Line::from("回收站").bold()).alignment(Alignment::Center);
    frame.render_widget(title, layout[0]);

    if items.is_empty() {
        let hint = Paragraph::new("回收站为空。").alignment(Alignment::Center);
        frame.render_widget(hint, layout[1]);
    } else {
        let header = Row::new(vec![
            "",
            "类型",
            "名称",
            "删除时间",
            "原页面/分区",
            "trash_id",
        ])
        .style(Style::default().bold())
        .bottom_margin(1);
        let rows: Vec<Row> = items
            .iter()
            .enumerate()
            .map(|(idx, item)| {
                let deleted = chrono::DateTime::from_timestamp_millis(item.deleted_at)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| item.deleted_at.to_string());
                let checked = selected_ids.contains(&item.id);
                let checkbox = if checked { "☑" } else { "☐" };
                let style = if idx == selected {
                    Style::default().reversed()
                } else {
                    Style::default()
                };
                Row::new(vec![
                    checkbox.to_string(),
                    item.item_type.clone(),
                    item.name.clone(),
                    deleted,
                    item.original_section_type.clone().unwrap_or_default(),
                    item.id.clone(),
                ])
                .style(style)
            })
            .collect();
        let mut state = ratatui::widgets::TableState::default().with_selected(Some(selected));
        let table = Table::new(
            rows,
            [
                Constraint::Length(3),
                Constraint::Percentage(10),
                Constraint::Percentage(25),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(25),
            ],
        )
        .header(header)
        .highlight_spacing(HighlightSpacing::Always)
        .block(Block::default().title(" /trash ").borders(Borders::ALL));
        frame.render_stateful_widget(table, layout[1], &mut state);
    }

    frame.render_widget(
        Paragraph::new(
            Line::from("↑↓ 移动 · 空格 多选 · R 恢复 · P 彻底删除 · Esc 返回").dark_gray(),
        )
        .alignment(Alignment::Center),
        layout[2],
    );
}
