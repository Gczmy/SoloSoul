//! 已锁定状态主界面 —— 暖色品牌页 + 核心动作卡片。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::theme::Theme;

struct LockedAction {
    icon: &'static str,
    fallback: &'static str,
    label: &'static str,
    command: &'static str,
    desc: &'static str,
}

const ACTIONS: &[LockedAction] = &[
    LockedAction {
        icon: "🔓",
        fallback: "[登录]",
        label: "登录",
        command: "/unlock",
        desc: "解锁 Vault",
    },
    LockedAction {
        icon: "👤",
        fallback: "[账户]",
        label: "账户",
        command: "/account_list",
        desc: "列出本地账户",
    },
    LockedAction {
        icon: "🩺",
        fallback: "[诊断]",
        label: "诊断",
        command: "/doctor",
        desc: "检查数据目录",
    },
    LockedAction {
        icon: "🚪",
        fallback: "[退出]",
        label: "退出",
        command: "/exit",
        desc: "安全退出 CLI",
    },
];

/// 渲染已锁定主界面。
pub fn render(frame: &mut ratatui::Frame, area: Rect) {
    let theme = Theme::load();

    let inner = area.inner(Margin::new(2, 2));
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(inner);

    render_brand(frame, chunks[0], &theme);
    render_actions(frame, chunks[1], &theme);
    render_hint(frame, chunks[2], &theme);
}

fn render_brand(frame: &mut ratatui::Frame, area: Rect, theme: &Theme) {
    let text = Text::from(vec![
        Line::from(""),
        Line::from("✦ SoloSoul ✦")
            .style(theme.style_brand())
            .alignment(Alignment::Center),
        Line::from("当前已锁定")
            .style(theme.style_cream())
            .alignment(Alignment::Center),
        Line::from(""),
        Line::from("本地优先 · 零知识 · 你的数据你做主")
            .style(theme.style_muted())
            .alignment(Alignment::Center),
        Line::from(""),
    ]);
    frame.render_widget(Paragraph::new(text), area);
}

fn render_actions(frame: &mut ratatui::Frame, area: Rect, theme: &Theme) {
    // 2 行 × 2 列
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .spacing(1)
        .split(area);

    for (row_idx, row_area) in rows.iter().enumerate() {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .spacing(1)
            .split(*row_area);

        for (col_idx, col_area) in cols.iter().enumerate() {
            let idx = row_idx * 2 + col_idx;
            if idx >= ACTIONS.len() {
                break;
            }
            render_action_card(frame, *col_area, theme, &ACTIONS[idx]);
        }
    }
}

fn render_action_card(
    frame: &mut ratatui::Frame,
    area: Rect,
    theme: &Theme,
    action: &LockedAction,
) {
    let icon = theme.icon_or_text(action.icon, action.fallback);
    let text = Text::from(vec![
        Line::from(""),
        Line::from(format!("{} {}", icon, action.label)).style(theme.style_cream()),
        Line::from(action.command).style(theme.style_muted()),
        Line::from(action.desc).style(theme.style_hint()),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.style_border(false));

    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center)
        .style(theme.style_text());

    frame.render_widget(paragraph, area);
}

fn render_hint(frame: &mut ratatui::Frame, area: Rect, theme: &Theme) {
    let text = Text::from(vec![Line::from("输入命令或按 Tab 补全，/help 查看全部")
        .style(theme.style_hint())
        .alignment(Alignment::Center)]);
    frame.render_widget(Paragraph::new(text), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    #[test]
    fn test_render_locked_smoke() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, frame.area())).unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf.content.iter().map(|cell| cell.symbol()).collect();
        assert!(content.contains("SoloSoul"));
        assert!(content.contains("/unlock"));
        // CJK 字符在 TestBackend 中会占两格并在中间产生空格，因此不直接断言中文。
    }
}
