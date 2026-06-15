//! 无账户时的欢迎界面 —— 暖色品牌引导页。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::theme::Theme;

/// 渲染欢迎界面。
pub fn render(frame: &mut ratatui::Frame, area: Rect) {
    let theme = Theme::load();

    let inner = area.inner(Margin::new(2, 2));
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),
            Constraint::Length(6),
            Constraint::Length(2),
        ])
        .split(inner);

    render_brand(frame, chunks[0], &theme);
    render_action_card(frame, chunks[1], &theme);
    render_hint(frame, chunks[2], &theme);
}

fn render_brand(frame: &mut ratatui::Frame, area: Rect, theme: &Theme) {
    let text = Text::from(vec![
        Line::from(""),
        Line::from("✦ SoloSoul ✦")
            .style(theme.style_brand())
            .alignment(Alignment::Center),
        Line::from("独奏生命数据，重塑数字原点")
            .style(theme.style_cream())
            .alignment(Alignment::Center),
        Line::from(""),
        Line::from("欢迎使用 SoloSoul CLI")
            .style(theme.style_text())
            .alignment(Alignment::Center),
        Line::from("本地优先 · 零知识 · 你的数据你做主")
            .style(theme.style_muted())
            .alignment(Alignment::Center),
        Line::from(""),
    ]);
    frame.render_widget(Paragraph::new(text), area);
}

fn render_action_card(frame: &mut ratatui::Frame, area: Rect, theme: &Theme) {
    let text = Text::from(vec![
        Line::from(""),
        Line::from("🚀 开始创建第一个账户")
            .style(theme.style_cream())
            .alignment(Alignment::Center),
        Line::from("按 Enter 启动向导")
            .style(theme.style_muted())
            .alignment(Alignment::Center),
        Line::from(""),
    ]);

    let block = Block::default()
        .title(" 开始 ")
        .title_style(theme.style_brand_dim())
        .borders(Borders::ALL)
        .border_style(theme.style_border(true));

    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center)
        .style(theme.style_text());

    frame.render_widget(paragraph, area);
}

fn render_hint(frame: &mut ratatui::Frame, area: Rect, theme: &Theme) {
    let text = Text::from(vec![Line::from("或输入 /exit 退出")
        .style(theme.style_hint())
        .alignment(Alignment::Center)]);
    frame.render_widget(Paragraph::new(text), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    #[test]
    fn test_render_welcome_smoke() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, frame.area())).unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf.content.iter().map(|cell| cell.symbol()).collect();
        assert!(content.contains("SoloSoul"));
        // CJK 字符在 TestBackend 中会占两格并在中间产生空格，因此不直接断言中文。
    }
}
