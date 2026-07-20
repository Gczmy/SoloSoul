//! Profile 展示屏幕。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Stylize;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use serde_json::Value;
use solosoul_core::Profile;

use crate::i18n::I18n;
use crate::t;

pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    profile: &Profile,
    data: &Value,
    _selected: usize,
    i18n: &I18n,
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

    let title =
        Paragraph::new(Line::from(t!(i18n, "profile-title")).bold()).alignment(Alignment::Center);
    frame.render_widget(title, layout[0]);

    let updated = profile.updated_at.format("%Y-%m-%d %H:%M:%S").to_string();
    let info = Paragraph::new(Text::from(vec![
        Line::from(t!(i18n, "profile-id", id = &profile.id)),
        Line::from(t!(i18n, "profile-name", name = &profile.name)),
        Line::from(t!(
            i18n,
            "profile-version",
            ver = &profile.version.to_string()
        )),
        Line::from(t!(i18n, "profile-updated", time = &updated)),
    ]))
    .block(Block::default().title(" 元数据 ").borders(Borders::ALL));
    frame.render_widget(info, layout[1]);

    let preview = serde_json::to_string_pretty(data).unwrap_or_else(|_| "{}".to_string());
    let data_widget = Paragraph::new(preview)
        .wrap(Wrap { trim: false })
        .block(Block::default().title(" 数据预览 ").borders(Borders::ALL));
    frame.render_widget(data_widget, layout[2]);

    let hint = Paragraph::new(Line::from(t!(i18n, "template-hint").dark_gray()))
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
        let i18n = I18n::new("en-US");
        terminal
            .draw(|frame| {
                render(
                    frame,
                    frame.area(),
                    &profile,
                    &Value::Object(Default::default()),
                    0,
                    &i18n,
                )
            })
            .unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf.content.iter().map(|cell| cell.symbol()).collect();
        assert!(content.contains("Profile"));
        assert!(content.contains("acc-1"));
    }
}
