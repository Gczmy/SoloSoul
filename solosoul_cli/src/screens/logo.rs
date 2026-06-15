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

/// SoloSoul 品牌 Logo（每行右侧空格仅用于源代码对齐，运行时按最宽行补齐）。
const LOGO_LINES: [&str; 6] = [
    " █████╗ █████╗ ██╗      █████╗  █████╗ █████╗ ██╗  ██╗██╗  ",
    "██╔═══╝██╔══██╗██║     ██╔══██╗██╔═══╝██╔══██╗██║  ██║██║  ",
    "╚████╗ ██║  ██║██║     ██║  ██║╚████╗ ██║  ██║██║  ██║██║  ",
    " ╚══██╗██║  ██║██║     ██║  ██║ ╚══██╗██║  ██║██║  ██║██║  ",
    "█████╔╝╚█████╔╝╚██████╗╚█████╝ █████╔╝╚█████╝ ╚█████╝ ╚██████╗",
    "╚════╝  ╚════╝  ╚═════╝ ╚════╝ ╚════╝  ╚════╝  ╚════╝  ╚═════╝",
];

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
    let logo_width = LOGO_LINES
        .iter()
        .map(|line| line.trim_end().chars().count())
        .max()
        .unwrap_or(0);
    let center = (sheen_offset as usize) % (logo_width + SHEEN_WIDTH);
    let half = SHEEN_WIDTH / 2;

    let lines: Vec<Line> = LOGO_LINES
        .iter()
        .map(|raw| {
            let trimmed = raw.trim_end();
            let padded = format!("{: <width$}", trimmed, width = logo_width);
            let spans: Vec<Span> = padded
                .chars()
                .enumerate()
                .map(|(idx, c)| {
                    let is_shadow = SHADOW_CHARS.contains(&c);
                    let in_sheen = is_shadow && idx.abs_diff(center) <= half;
                    let style = if in_sheen {
                        theme.style_cream()
                    } else if is_shadow {
                        theme.style_brand()
                    } else {
                        theme.style_logo_fill()
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
