//! 备份列表屏幕。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};

use crate::commands::backup::BackupInfo;

/// 渲染备份列表。
pub fn render(frame: &mut ratatui::Frame, area: Rect, items: &[BackupInfo], selected: usize) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let title = Paragraph::new(Line::from("备份列表").bold()).alignment(Alignment::Center);
    frame.render_widget(title, layout[0]);

    if items.is_empty() {
        let hint = Paragraph::new("暂无备份。").alignment(Alignment::Center);
        frame.render_widget(hint, layout[1]);
    } else {
        let header = Row::new(vec!["ID / 名称", "创建时间", "大小", "Profile 数"])
            .style(Style::default().bold())
            .bottom_margin(1);

        let rows: Vec<Row> = items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let cells = vec![
                    item.id.clone(),
                    item.created_at.clone(),
                    format_size(item.size_bytes),
                    item.object_count.to_string(),
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
                Constraint::Percentage(35),
                Constraint::Percentage(30),
                Constraint::Percentage(20),
                Constraint::Percentage(15),
            ],
        )
        .header(header)
        .block(Block::default().title(" /backup ").borders(Borders::ALL));
        frame.render_widget(table, layout[1]);
    }

    frame.render_widget(
        Paragraph::new(
            Line::from("使用 /backup restore <id> 恢复，/backup delete <id> 删除。").dark_gray(),
        )
        .alignment(Alignment::Center),
        layout[2],
    );
}

/// 将字节数格式化为人类可读字符串。
fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx + 1 < UNITS.len() {
        size /= 1024.0;
        unit_idx += 1;
    }
    format!("{:.2} {}", size, UNITS[unit_idx])
}
