//! 无账户时的欢迎界面 —— 大品牌名 + 可点击选项。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{ClickAction, ClickableRegion};
use crate::theme::Theme;

/// 渲染欢迎界面。
pub fn render(frame: &mut ratatui::Frame, area: Rect, regions: &mut Vec<ClickableRegion>) {
    let theme = Theme::load();

    let inner = area.inner(Margin::new(4, 4));
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Length(8),
            Constraint::Length(2),
        ])
        .split(inner);

    render_brand(frame, chunks[0], &theme);
    render_options(frame, chunks[1], &theme, regions);
    render_hint(frame, chunks[2], &theme);
}

fn render_brand(frame: &mut ratatui::Frame, area: Rect, theme: &Theme) {
    let text = Text::from(vec![
        Line::from(""),
        Line::from("SoloSoul")
            .style(theme.style_brand())
            .alignment(Alignment::Center),
        Line::from(""),
        Line::from("独奏生命数据，重塑数字原点")
            .style(theme.style_cream())
            .alignment(Alignment::Center),
        Line::from(""),
        Line::from("本地优先 · 零知识 · 你的数据你做主")
            .style(theme.style_muted())
            .alignment(Alignment::Center),
        Line::from(""),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.style_border(true))
        .title(" 欢迎 ")
        .title_style(theme.style_brand_dim());

    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center)
        .style(theme.style_text());
    frame.render_widget(paragraph, area);
}

fn render_options(
    frame: &mut ratatui::Frame,
    area: Rect,
    theme: &Theme,
    regions: &mut Vec<ClickableRegion>,
) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .spacing(2)
        .split(area);

    let start_rect = cols[0];
    let start_text = Text::from(vec![
        Line::from(""),
        Line::from("> 开始创建账户")
            .style(theme.style_brand())
            .alignment(Alignment::Center),
        Line::from("创建第一个本地账户并导入默认模板")
            .style(theme.style_hint())
            .alignment(Alignment::Center),
        Line::from(""),
    ]);
    let start_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.style_border(true))
        .title(" 开始 ")
        .title_style(theme.style_brand_dim());
    frame.render_widget(
        Paragraph::new(start_text)
            .block(start_block)
            .alignment(Alignment::Center)
            .style(theme.style_text()),
        start_rect,
    );
    regions.push(ClickableRegion {
        rect: start_rect,
        action: ClickAction::StartOnboarding,
    });

    let exit_rect = cols[1];
    let exit_text = Text::from(vec![
        Line::from(""),
        Line::from("> 退出 CLI")
            .style(theme.style_cream())
            .alignment(Alignment::Center),
        Line::from("离开 SoloSoul 终端")
            .style(theme.style_hint())
            .alignment(Alignment::Center),
        Line::from(""),
    ]);
    let exit_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.style_border(false));
    frame.render_widget(
        Paragraph::new(exit_text)
            .block(exit_block)
            .alignment(Alignment::Center)
            .style(theme.style_text()),
        exit_rect,
    );
    regions.push(ClickableRegion {
        rect: exit_rect,
        action: ClickAction::Command("/exit"),
    });
}

fn render_hint(frame: &mut ratatui::Frame, area: Rect, theme: &Theme) {
    let text = Text::from(vec![Line::from(
        "↑/↓/Tab 切换，Enter 确认，鼠标可直接点击选项",
    )
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
        let mut regions = Vec::new();
        terminal
            .draw(|frame| render(frame, frame.area(), &mut regions))
            .unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf.content.iter().map(|cell| cell.symbol()).collect();
        assert!(content.contains("SoloSoul"));
        assert_eq!(regions.len(), 2);
    }
}
