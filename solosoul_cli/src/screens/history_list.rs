//! 对象历史快照列表屏幕。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};

use crate::i18n::I18n;
use crate::t;

pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    object_id: &str,
    snapshots: &[serde_json::Value],
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
            Line::from(t!(i18n, "history-title", id = object_id))
                .bold()
                .alignment(Alignment::Center),
        ),
        layout[0],
    );

    if snapshots.is_empty() {
        frame.render_widget(
            Paragraph::new(t!(i18n, "history-empty")).alignment(Alignment::Center),
            layout[1],
        );
    } else {
        let header = Row::new(vec!["时间", "触发者", "摘要", "快照ID"])
            .style(Style::default().bold())
            .bottom_margin(1);
        let rows: Vec<Row> = snapshots
            .iter()
            .enumerate()
            .map(|(i, snap)| {
                let ts = snap
                    .get("timestamp")
                    .and_then(|v| v.as_i64())
                    .and_then(chrono::DateTime::from_timestamp_millis)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "-".to_string());
                let triggered_by = snap
                    .get("triggeredBy")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string();
                let summary = snap
                    .get("diffSummary")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string();
                let id = snap
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string();
                let marker = if i == selected { "▸ " } else { "  " };
                let cells = vec![format!("{}{}", marker, ts), triggered_by, summary, id];
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
                Constraint::Percentage(40),
                Constraint::Percentage(25),
            ],
        )
        .header(header)
        .block(Block::default().title(" /history ").borders(Borders::ALL));
        frame.render_widget(table, layout[1]);
    }

    frame.render_widget(
        Paragraph::new(Line::from(t!(i18n, "log-export-hint")).dark_gray())
            // Note: rollback hint uses a generic fallback for now
            .alignment(Alignment::Center),
        layout[2],
    );
}
