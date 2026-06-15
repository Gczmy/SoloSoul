//! 已登录首页 —— 暖色仪表盘 + 可导航快捷卡片。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::theme::Theme;

/// 首页快捷入口定义。
pub struct Shortcut {
    pub icon: &'static str,
    pub fallback: &'static str,
    pub label: &'static str,
    pub command: &'static str,
    pub desc: &'static str,
}

/// 首页全部快捷入口。
pub const SHORTCUTS: &[Shortcut] = &[
    Shortcut {
        icon: "📁",
        fallback: "[浏览]",
        label: "浏览",
        command: "/list",
        desc: "列出页面与对象",
    },
    Shortcut {
        icon: "🔍",
        fallback: "[搜索]",
        label: "搜索",
        command: "/search",
        desc: "全局关键词搜索",
    },
    Shortcut {
        icon: "➕",
        fallback: "[创建]",
        label: "创建",
        command: "/newobject",
        desc: "新建对象",
    },
    Shortcut {
        icon: "🗑",
        fallback: "[回收]",
        label: "回收站",
        command: "/trash",
        desc: "查看已删除项目",
    },
    Shortcut {
        icon: "⚙",
        fallback: "[设置]",
        label: "设置",
        command: "/setting",
        desc: "账户偏好设置",
    },
    Shortcut {
        icon: "❓",
        fallback: "[帮助]",
        label: "帮助",
        command: "/help",
        desc: "查看全部命令",
    },
];

/// 返回快捷入口总数。
pub fn shortcut_count() -> usize {
    SHORTCUTS.len()
}

/// 渲染已登录首页。
///
/// `selected_shortcut` 为当前获得焦点的卡片索引（0..SHORTCUTS.len()）。
pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    account_name: &str,
    account_id: &str,
    selected_shortcut: usize,
) {
    let theme = Theme::load();

    let header_text = if account_name.is_empty() || account_name == account_id {
        format!("SoloSoul · 欢迎回来，{}", account_id)
    } else {
        format!("SoloSoul · 欢迎回来，{} · {}", account_name, account_id)
    };

    // 整体内容区：留出边距，纵向分三份（标题、卡片网格、提示）。
    let inner = area.inner(Margin::new(2, 1));
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(inner);

    render_header(frame, chunks[0], &theme, &header_text);
    render_shortcut_grid(frame, chunks[1], &theme, selected_shortcut);
    render_hint(frame, chunks[2], &theme);
}

fn render_header(frame: &mut ratatui::Frame, area: Rect, theme: &Theme, title: &str) {
    let text = Text::from(vec![
        Line::from(""),
        Line::from(title).style(theme.style_brand()),
        Line::from("独奏生命数据，重塑数字原点").style(theme.style_cream()),
        Line::from(""),
    ]);
    let paragraph = Paragraph::new(text).alignment(Alignment::Center);
    frame.render_widget(paragraph, area);
}

fn render_shortcut_grid(frame: &mut ratatui::Frame, area: Rect, theme: &Theme, selected: usize) {
    // 2 行 × 3 列
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .spacing(1)
        .split(area);

    for (row_idx, row_area) in rows.iter().enumerate() {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .spacing(1)
            .split(*row_area);

        for (col_idx, col_area) in cols.iter().enumerate() {
            let idx = row_idx * 3 + col_idx;
            if idx >= SHORTCUTS.len() {
                break;
            }
            let focused = idx == selected;
            render_shortcut_card(frame, *col_area, theme, &SHORTCUTS[idx], focused);
        }
    }
}

fn render_shortcut_card(
    frame: &mut ratatui::Frame,
    area: Rect,
    theme: &Theme,
    shortcut: &Shortcut,
    focused: bool,
) {
    let marker = if focused { "▶ " } else { "" };
    let icon = theme.icon_or_text(shortcut.icon, shortcut.fallback);
    let title = format!("{}{} {}", marker, icon, shortcut.label);

    let text = Text::from(vec![
        Line::from(""),
        Line::from(title).style(theme.style_card_title(focused)),
        Line::from(shortcut.command).style(theme.style_muted()),
        Line::from(shortcut.desc).style(theme.style_hint()),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.style_border(focused));

    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center)
        .style(if focused {
            theme.style_card_focused()
        } else {
            theme.style_text()
        });

    frame.render_widget(paragraph, area);
}

fn render_hint(frame: &mut ratatui::Frame, area: Rect, theme: &Theme) {
    let text = Text::from(vec![
        Line::from("Tab / Shift+Tab 切换卡片，Enter 填入命令")
            .style(theme.style_hint())
            .alignment(Alignment::Center),
        Line::from("直接输入 /help 查看全部命令")
            .style(theme.style_muted())
            .alignment(Alignment::Center),
    ]);
    frame.render_widget(Paragraph::new(text), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ColorLevel;
    use ratatui::backend::TestBackend;

    #[test]
    fn test_shortcut_count_matches_const() {
        assert_eq!(shortcut_count(), SHORTCUTS.len());
        assert_eq!(shortcut_count(), 6);
    }

    #[test]
    fn test_render_home_smoke() {
        let _theme = Theme::with_level(ColorLevel::Indexed, true);
        let backend = TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render(frame, frame.area(), "Alice", "alice-123", 0))
            .unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf.content.iter().map(|cell| cell.symbol()).collect();
        assert!(content.contains("SoloSoul"));
        assert!(content.contains("/list"));
        // CJK 字符在 TestBackend 中会占两格并在中间产生空格，因此不直接断言中文。
    }
}
