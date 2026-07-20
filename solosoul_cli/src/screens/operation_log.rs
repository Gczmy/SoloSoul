//! 审计日志列表屏幕。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};
use solosoul_core::AuditLogEntry;

use crate::i18n::I18n;
use crate::t;

pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    entries: &[AuditLogEntry],
    selected: usize,
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

    frame.render_widget(
        Paragraph::new(
            Line::from(t!(i18n, "log-title", count = entries.len().to_string()))
                .bold()
                .alignment(Alignment::Center),
        ),
        layout[0],
    );

    if entries.is_empty() {
        frame.render_widget(
            Paragraph::new(t!(i18n, "log-empty")).alignment(Alignment::Center),
            layout[1],
        );
    } else {
        let header = Row::new(vec!["时间", "操作", "实体", "名称/ID", "详情"])
            .style(Style::default().bold())
            .bottom_margin(1);
        let rows: Vec<Row> = entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let marker = if i == selected { "▸ " } else { "  " };
                let entity = if entry.entity_type.is_empty() {
                    "-".to_string()
                } else {
                    entry.entity_type.clone()
                };
                let name_or_id = entry
                    .entity_name
                    .clone()
                    .or_else(|| entry.entity_id.clone())
                    .unwrap_or_else(|| "-".to_string());
                let details = entry.details.clone().unwrap_or_default();
                let details = if details.len() > 40 {
                    format!("{}...", &details[..40])
                } else {
                    details
                };
                let cells = vec![
                    format!("{}{}", marker, entry.timestamp),
                    entry.action_type.clone(),
                    entity,
                    name_or_id,
                    details,
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
                Constraint::Percentage(20),
                Constraint::Percentage(15),
                Constraint::Percentage(12),
                Constraint::Percentage(20),
                Constraint::Percentage(33),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .title(" /operation_log ")
                .borders(Borders::ALL),
        );
        frame.render_widget(table, layout[1]);
    }

    frame.render_widget(
        Paragraph::new(Line::from(t!(i18n, "log-export-hint")).dark_gray())
            .alignment(Alignment::Center),
        layout[2],
    );
}
