//! Profile 展示屏幕。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Stylize;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use serde_json::Value;
use solosoul_core::Profile;

pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    profile: &Profile,
    data: &Value,
    _selected: usize,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(6),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let title = Paragraph::new(Line::from("Profile").bold()).alignment(Alignment::Center);
    frame.render_widget(title, layout[0]);

    let updated = profile.updated_at.format("%Y-%m-%d %H:%M:%S").to_string();
    let info = Paragraph::new(Text::from(vec![
        Line::from(format!("ID: {}", profile.id)),
        Line::from(format!("名称: {}", profile.name)),
        Line::from(format!("版本: {}", profile.version)),
        Line::from(format!("更新时间: {}", updated)),
    ]))
    .block(Block::default().title(" 元数据 ").borders(Borders::ALL));
    frame.render_widget(info, layout[1]);

    let preview = serde_json::to_string_pretty(data).unwrap_or_else(|_| "{}".to_string());
    let data_widget = Paragraph::new(preview)
        .wrap(Wrap { trim: false })
        .block(Block::default().title(" 数据预览 ").borders(Borders::ALL));
    frame.render_widget(data_widget, layout[2]);

    let hint = Paragraph::new(Line::from(
        "Esc 返回 · /profile set <路径> <值> 编辑字段".dark_gray(),
    ))
    .alignment(Alignment::Center);
    frame.render_widget(hint, layout[3]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    #[test]
    fn test_render_profile_smoke() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let profile = Profile::new_with_id("acc-1", "Test", b"{}".to_vec());
        terminal
            .draw(|frame| {
                render(
                    frame,
                    frame.area(),
                    &profile,
                    &Value::Object(Default::default()),
                    0,
                )
            })
            .unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf.content.iter().map(|cell| cell.symbol()).collect();
        assert!(content.contains("Profile"));
        assert!(content.contains("acc-1"));
    }
}
