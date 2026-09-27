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
                let summary = match snap.get("diffSummary").and_then(|v| v.as_str()) {
                    Some("diff_rollback") => t!(i18n, "history-summary-rollback"),
                    other => other.unwrap_or("-").to_string(),
                };
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use serde_json::json;

    #[test]
    fn rf008_history_renders_localized_rollback_and_preserves_other_summaries() {
        let snapshots = vec![
            json!({"id": "rollback-snapshot", "timestamp": 1000, "triggeredBy": "rollback", "diffSummary": "diff_rollback"}),
            json!({"id": "legacy-snapshot", "timestamp": 2000, "triggeredBy": "user_edit", "diffSummary": "Legacy free text"}),
            json!({"id": "custom-snapshot", "timestamp": 3000, "triggeredBy": "user_edit", "diffSummary": "diff_custom"}),
        ];
        for (locale, expected) in [
            ("zh-CN", "已回滚到历史版本"),
            ("en-US", "Restored previous version"),
        ] {
            let i18n = I18n::new(locale);
            let mut terminal = Terminal::new(TestBackend::new(110, 16)).unwrap();
            terminal
                .draw(|frame| render(frame, frame.area(), "rf008-object", &snapshots, 0, &i18n))
                .unwrap();
            // 去除表格填充及中文宽字符的占位空格，检查实际终端单元格内容。
            let rendered: String = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect();
            let compact: String = rendered.chars().filter(|c| !c.is_whitespace()).collect();
            assert!(
                compact.contains(&expected.replace(' ', "")),
                "{locale}: {rendered}"
            );
            assert!(!rendered.contains("diff_rollback"), "{locale}: {rendered}");
            assert!(compact.contains("Legacyfreetext"), "{locale}: {rendered}");
            assert!(rendered.contains("diff_custom"), "{locale}: {rendered}");
            assert!(
                rendered.contains("rollback-snapshot"),
                "{locale}: {rendered}"
            );
        }
    }
}
