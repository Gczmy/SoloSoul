//! 模板详情屏幕。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    template_id: &str,
    name: &str,
    source: &str,
    json: &str,
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

    let title = Paragraph::new(Line::from("模板详情").bold()).alignment(Alignment::Center);
    frame.render_widget(title, layout[0]);

    let info = Paragraph::new(vec![
        Line::from(format!("名称: {}", name)),
        Line::from(format!("来源: {}", source)),
        Line::from(format!("ID: {}", template_id)),
    ])
    .block(Block::default().title(" 元数据 ").borders(Borders::ALL));
    frame.render_widget(info, layout[1]);

    let detail = Paragraph::new(json.to_string())
        .wrap(Wrap { trim: false })
        .block(Block::default().title(" 定义 ").borders(Borders::ALL));
    frame.render_widget(detail, layout[2]);

    frame.render_widget(
        Paragraph::new(Line::from("Esc 返回".dark_gray())).alignment(Alignment::Center),
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
        terminal
            .draw(|frame| render(frame, frame.area(), "note", "笔记", "系统", "{}"))
            .unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf.content.iter().map(|cell| cell.symbol()).collect();
        assert!(content.replace(' ', "").contains("模板详情"));
        assert!(content.replace(' ', "").contains("笔记"));
    }
}
