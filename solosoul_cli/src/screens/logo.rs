//! SoloSoul 品牌 Logo —— Codebuff 风格 Unicode 方块艺术字 + Sheen 扫光动画。

use ratatui::layout::{Alignment, Rect};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::theme::Theme;

/// Logo 扫光每 tick 前进的列数。
pub const SHEEN_STEP: u16 = 2;

/// 扫光高亮宽度（奇数便于取中心）。
const SHEEN_WIDTH: usize = 5;

/// 阴影/边框字符集合，扫光动画只影响这些字符。
const SHADOW_CHARS: [char; 11] = ['╚', '═', '╝', '║', '╔', '╗', '╠', '╣', '╦', '╩', '╬'];

/// SoloSoul 品牌 Logo， trimming 后宽度 58。
const LOGO_LINES: [&str; 6] = [
    "██████╗ █████╗ ██╗  █████╗ ██████╗ █████╗ ██╗  ██╗██╗     ",
    "██╔═══╝██╔══██╗██║ ██╔══██╗██╔═══╝██╔══██╗██║  ██║██║     ",
    "╚████╗ ██║  ██║██║ ██║  ██║╚████╗ ██║  ██║██║  ██║██║     ",
    " ╚══██╗██║  ██║██║ ██║  ██║ ╚══██╗██║  ██║██║  ██║██║     ",
    "█████╔╝╚█████╔╝██║ ╚█████╝ █████╔╝╚█████╝ ╚█████╝ ╚██████╗",
    "╚════╝ ╚═════╝ ╚═╝ ╚═════╝ ╚════╝ ╚═════╝ ╚═════╝  ╚═════╝",
];

const LOGO_WIDTH: usize = 58;

/// 渲染带边框与扫光动画的品牌 Logo。
///
/// `area` 高度至少应为 8（6 行 Logo + 2 行边框）。
pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    theme: &Theme,
    sheen_offset: u16,
    title: &str,
) {
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
        .title(title)
        .title_style(theme.style_brand_dim());

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .alignment(Alignment::Center)
        .style(theme.style_text());
    frame.render_widget(paragraph, area);
}
