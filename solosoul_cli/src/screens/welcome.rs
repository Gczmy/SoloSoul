//! 无账户时的欢迎界面 —— 大品牌名 + 可点击选项 + 悬停动画。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{ClickAction, ClickableRegion};
use crate::theme::Theme;

/// Logo 扫光每 tick 前进的列数。
pub const SHEEN_STEP: u16 = 2;

/// 扫光高亮宽度（奇数便于取中心）。
const SHEEN_WIDTH: usize = 5;

/// 阴影/边框字符集合，扫光动画只影响这些字符。
const SHADOW_CHARS: [char; 11] = ['╚', '═', '╝', '║', '╔', '╗', '╠', '╣', '╦', '╩', '╬'];

/// SoloSoul 品牌 Logo（Codebuff 风格 Unicode 方块艺术字）， trimming 后宽度 55。
const LOGO_LINES: [&str; 5] = [
    "██████╗  █████╗ ██╗  █████╗ ██████╗  █████╗ ██╗  ██╗██╗",
    "██╔═══╝ ██╔══██╗██║ ██╔══██╗██╔═══╝ ██╔══██╗██║  ██║██║",
    "╚████╗  ██║  ██║██║ ██║  ██║╚████╗  ██║  ██║██║  ██║██║",
    " ╚══██║ ██║  ██║██║ ██║  ██║ ╚══██║ ██║  ██║██║  ██║██║",
    "██████╝ ╚█████╝ ╚█╝ ╚█████╝ ██████╝ ╚█████╝ ╚█████╝ ╚█╝",
];

const LOGO_WIDTH: usize = 55;

/// 渲染欢迎界面。
pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    regions: &mut Vec<ClickableRegion>,
    mouse_pos: Option<(u16, u16)>,
    hover_pulse: bool,
    sheen_offset: u16,
) {
    let theme = Theme::load();

    let inner = area.inner(Margin::new(4, 4));
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // Logo + 边框
            Constraint::Length(2), // 副标语
            Constraint::Length(6), // 可点击选项卡
            Constraint::Length(1), // 底部提示
        ])
        .split(inner);

    render_brand(frame, chunks[0], &theme, sheen_offset);
    render_taglines(frame, chunks[1], &theme);
    render_options(frame, chunks[2], &theme, regions, mouse_pos, hover_pulse);
    render_hint(frame, chunks[3], &theme);
}

fn render_brand(frame: &mut ratatui::Frame, area: Rect, theme: &Theme, sheen_offset: u16) {
    let center = (sheen_offset as usize) % (LOGO_WIDTH + SHEEN_WIDTH);
    let half = SHEEN_WIDTH / 2;

    let lines: Vec<Line> = LOGO_LINES
        .iter()
        .map(|raw| {
            let spans: Vec<Span> = raw
                .chars()
                .enumerate()
                .map(|(idx, c)| {
                    let is_shadow = SHADOW_CHARS.contains(&c);
                    let in_sheen = is_shadow && idx.abs_diff(center) <= half;
                    let style = if in_sheen {
                        theme.style_cream()
                    } else {
                        theme.style_brand()
                    };
                    Span::styled(c.to_string(), style)
                })
                .collect();
            Line::from(spans)
        })
        .collect();

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
            .draw(|frame| render(frame, frame.area(), &mut regions, None, false, 0))
            .unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf.content.iter().map(|cell| cell.symbol()).collect();
        // 静态 Unicode Logo 包含方块与阴影框线字符
        assert!(content.contains('█'));
        assert!(content.contains('╗'));
        assert_eq!(regions.len(), 2);
    }
}
