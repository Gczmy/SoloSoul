//! 回收站列表屏幕。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};
use solosoul_core::TrashItemSummary;

pub fn render(frame: &mut ratatui::Frame, area: Rect, items: &[TrashItemSummary]) {
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
        let header = Row::new(vec!["类型", "名称", "删除时间", "原页面/分区", "trash_id"])
            .style(Style::default().bold())
            .bottom_margin(1);
        let rows: Vec<Row> = items
            .iter()
            .map(|item| {
                let deleted = chrono::DateTime::from_timestamp_millis(item.deleted_at)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| item.deleted_at.to_string());
                Row::new(vec![
                    item.item_type.clone(),
                    item.name.clone(),
                    deleted,
                    item.original_section_type.clone().unwrap_or_default(),
                    item.id.clone(),
                ])
            })
            .collect();
        let table = Table::new(
            rows,
            [
                Constraint::Percentage(10),
                Constraint::Percentage(25),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(25),
            ],
        )
        .header(header)
        .block(Block::default().title(" /trash ").borders(Borders::ALL));
        frame.render_widget(table, layout[1]);
    }

    frame.render_widget(
        Paragraph::new(
            Line::from("使用 /restore <trash_id> 恢复，/purge <trash_id> 彻底删除。").dark_gray(),
        )
        .alignment(Alignment::Center),
        layout[2],
    );
}
