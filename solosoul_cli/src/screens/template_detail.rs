//! 模板详情屏幕。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::i18n::I18n;
use crate::t;

pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    template_id: &str,
    name: &str,
    source: &str,
    json: &str,
    i18n: &I18n,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(4),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let title = Paragraph::new(Line::from(t!(i18n, "template-detail-title")).bold())
        .alignment(Alignment::Center);
    frame.render_widget(title, layout[0]);

    let info = Paragraph::new(vec![
        Line::from(t!(i18n, "object-detail-name", name = name)),
        Line::from(t!(i18n, "doctor-source", source = source)),
        // Note: 'source' uses doctor-source as generic key
        Line::from(t!(i18n, "object-detail-id", id = template_id)),
    ])
    .block(Block::default().title(" 元数据 ").borders(Borders::ALL));
    frame.render_widget(info, layout[1]);

    let detail = Paragraph::new(json.to_string())
        .wrap(Wrap { trim: false })
        .block(Block::default().title(" 定义 ").borders(Borders::ALL));
    frame.render_widget(detail, layout[2]);

    frame.render_widget(
        Paragraph::new(Line::from(t!(i18n, "hint-esc-back")).dark_gray())
            .alignment(Alignment::Center),
        layout[3],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    #[test]
    fn test_render_template_detail_smoke() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let i18n = I18n::new("zh-CN");
        terminal
            .draw(|frame| render(frame, frame.area(), "note", "笔记", "系统", "{}", &i18n))
            .unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf.content.iter().map(|cell| cell.symbol()).collect();
        assert!(content.replace(' ', "").contains("模板详情"));
        assert!(content.replace(' ', "").contains("笔记"));
    }
}
