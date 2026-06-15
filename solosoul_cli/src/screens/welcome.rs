//! 无账户时的欢迎界面 —— 大品牌名 + 可点击选项 + 悬停动画。

use std::sync::OnceLock;

use figlet_rs::FIGlet;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{ClickAction, ClickableRegion};
use crate::theme::Theme;

fn standard_font() -> &'static FIGlet {
    static FONT: OnceLock<FIGlet> = OnceLock::new();
    FONT.get_or_init(|| FIGlet::standard().expect("内置 FIGlet 标准字体加载失败"))
}

/// 渲染欢迎界面。
pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    regions: &mut Vec<ClickableRegion>,
    mouse_pos: Option<(u16, u16)>,
    hover_pulse: bool,
) {
    let theme = Theme::load();

    let inner = area.inner(Margin::new(4, 4));
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // FIGlet banner + 边框
            Constraint::Length(2), // 副标语
            Constraint::Length(6), // 可点击选项卡
            Constraint::Length(1), // 底部提示
        ])
        .split(inner);

    render_brand(frame, chunks[0], &theme);
    render_taglines(frame, chunks[1], &theme);
    render_options(frame, chunks[2], &theme, regions, mouse_pos, hover_pulse);
    render_hint(frame, chunks[3], &theme);
}

fn render_brand(frame: &mut ratatui::Frame, area: Rect, theme: &Theme) {
    let mut lines: Vec<Line> = Vec::new();

    if let Some(figure) = standard_font().convert("SoloSoul") {
        let banner = figure.as_str();
        for raw in banner.lines() {
            let trimmed = raw.trim_end();
            if trimmed.is_empty() {
                continue;
            }
            lines.push(
                Line::from(trimmed.to_string())
                    .style(theme.style_brand())
                    .alignment(Alignment::Center),
            );
        }
    } else {
        lines.push(
            Line::from("SoloSoul")
                .style(theme.style_brand())
                .alignment(Alignment::Center),
        );
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.style_border(true))
        .title(" 欢迎 ")
        .title_style(theme.style_brand_dim());

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .alignment(Alignment::Center)
        .style(theme.style_text());
    frame.render_widget(paragraph, area);
}

fn render_taglines(frame: &mut ratatui::Frame, area: Rect, theme: &Theme) {
    let text = Text::from(vec![
        Line::from("独奏生命数据，重塑数字原点")
            .style(theme.style_cream())
            .alignment(Alignment::Center),
        Line::from("本地优先 · 零知识 · 你的数据你做主")
            .style(theme.style_muted())
            .alignment(Alignment::Center),
    ]);
    frame.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .style(theme.style_text()),
        area,
    );
}

fn render_options(
    frame: &mut ratatui::Frame,
    area: Rect,
    theme: &Theme,
    regions: &mut Vec<ClickableRegion>,
    mouse_pos: Option<(u16, u16)>,
    hover_pulse: bool,
) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .spacing(2)
        .split(area);

    let start_rect = cols[0];
    let start_hovered = is_hovered(start_rect, mouse_pos);
    let start_text = Text::from(vec![
        Line::from(""),
        Line::from("> 开始创建账户")
            .style(if start_hovered {
                theme.style_brand()
            } else {
                theme.style_cream()
            })
            .alignment(Alignment::Center),
        Line::from("创建第一个本地账户并导入默认模板")
            .style(theme.style_hint())
            .alignment(Alignment::Center),
        Line::from(""),
    ]);
    let start_block = Block::default()
        .borders(Borders::ALL)
        .border_style(card_border_style(theme, start_hovered, hover_pulse))
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
    let exit_hovered = is_hovered(exit_rect, mouse_pos);
    let exit_text = Text::from(vec![
        Line::from(""),
        Line::from("> 退出 CLI")
            .style(if exit_hovered {
                theme.style_brand()
            } else {
                theme.style_cream()
            })
            .alignment(Alignment::Center),
        Line::from("离开 SoloSoul 终端")
            .style(theme.style_hint())
            .alignment(Alignment::Center),
        Line::from(""),
    ]);
    let exit_block = Block::default()
        .borders(Borders::ALL)
        .border_style(card_border_style(theme, exit_hovered, hover_pulse));
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

fn is_hovered(rect: Rect, mouse_pos: Option<(u16, u16)>) -> bool {
    mouse_pos.is_some_and(|pos| rect.contains(pos.into()))
}

fn card_border_style(theme: &Theme, hovered: bool, pulse: bool) -> ratatui::style::Style {
    if hovered {
        if pulse {
            theme.style_brand_dim()
        } else {
            theme.style_cream()
        }
    } else {
        theme.style_border(false)
    }
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
            .draw(|frame| render(frame, frame.area(), &mut regions, None, false))
            .unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf.content.iter().map(|cell| cell.symbol()).collect();
        // FIGlet banner 会渲染为 ASCII 艺术，不再包含纯文本 "SoloSoul"
        assert!(content.contains(" ____ "));
        assert!(content.contains("/ ___|"));
        assert_eq!(regions.len(), 2);
    }
}
