//! 附件列表屏幕。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};

use crate::commands::attachment::AttachmentMeta;

/// 渲染附件列表。
pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    object_id: &str,
    items: &[AttachmentMeta],
    show_deleted: bool,
    selected: usize,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let title_text = if show_deleted {
        format!("附件列表（含已删除）- {}", object_id)
    } else {
        format!("附件列表 - {}", object_id)
    };
    let title = Paragraph::new(Line::from(title_text).bold()).alignment(Alignment::Center);
    frame.render_widget(title, layout[0]);

    if items.is_empty() {
        let hint = Paragraph::new("暂无附件。").alignment(Alignment::Center);
        frame.render_widget(hint, layout[1]);
    } else {
        let header = Row::new(vec!["#", "文件名", "大小", "状态"])
            .style(Style::default().bold())
            .bottom_margin(1);
        let rows: Vec<Row> = items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let size = format_size(item.size_bytes);
                let status = if item.deleted_at.is_some() {
                    "已删除"
                } else {
                    "正常"
                };
                let cells = vec![
                    (i + 1).to_string(),
                    item.file_name.clone(),
                    size,
                    status.to_string(),
                ];
                let mut row = Row::new(cells);
                if i == selected {
                    row = row.style(Style::default().reversed());
                }
                row
            })
            .collect();
        let table = Table::new(
            rows,
            [
                Constraint::Length(4),
                Constraint::Percentage(50),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .title(" /attach list ")
                .borders(Borders::ALL),
        );
        frame.render_widget(table, layout[1]);
    }

    frame.render_widget(
        Paragraph::new(
            Line::from("使用 /attach add <path> 添加，/attach delete <id> 删除，/attach purge <id> 彻底删除。").dark_gray(),
        )
        .alignment(Alignment::Center),
        layout[2],
    );
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
