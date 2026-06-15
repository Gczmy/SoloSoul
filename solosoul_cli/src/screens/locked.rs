//! 已锁定状态主界面 —— 大品牌名 + 可点击动作卡片 + 悬停动画。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{ClickAction, ClickableRegion};
use crate::theme::Theme;

struct LockedAction {
    label: &'static str,
    command: &'static str,
}

const ACTIONS: &[LockedAction] = &[
    LockedAction {
        label: "登录",
        command: "/unlock",
    },
    LockedAction {
        label: "账户",
        command: "/account_list",
    },
    LockedAction {
        label: "诊断",
        command: "/doctor",
    },
    LockedAction {
        label: "退出",
        command: "/exit",
    },
];

/// 渲染已锁定主界面。
pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    regions: &mut Vec<ClickableRegion>,
    mouse_pos: Option<(u16, u16)>,
    hover_pulse: bool,
    sheen_offset: u16,
) {
    let theme = Theme::load();

    let inner = area.inner(Margin::new(4, 1));
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // 6 行 Logo + 2 行边框
            Constraint::Length(9), // 2×2 动作卡片
            Constraint::Length(1), // 底部提示
        ])
        .split(inner);

    crate::screens::logo::render(frame, chunks[0], &theme, sheen_offset, " 已锁定 ");
    render_actions(frame, chunks[1], &theme, regions, mouse_pos, hover_pulse);
    render_hint(frame, chunks[2], &theme);
}

fn render_actions(
    frame: &mut ratatui::Frame,
    area: Rect,
    theme: &Theme,
    regions: &mut Vec<ClickableRegion>,
    mouse_pos: Option<(u16, u16)>,
    hover_pulse: bool,
) {
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
            .spacing(2)
            .split(*row_area);

        for (col_idx, col_area) in cols.iter().enumerate() {
            let idx = row_idx * 2 + col_idx;
            if idx >= ACTIONS.len() {
                break;
            }
            render_action_card(
                frame,
                *col_area,
                theme,
                &ACTIONS[idx],
                regions,
                mouse_pos,
                hover_pulse,
            );
        }
    }
}

fn render_action_card(
    frame: &mut ratatui::Frame,
    area: Rect,
    theme: &Theme,
    action: &LockedAction,
    regions: &mut Vec<ClickableRegion>,
    mouse_pos: Option<(u16, u16)>,
    hover_pulse: bool,
) {
    let hovered = is_hovered(area, mouse_pos);
    let text = Text::from(vec![
        Line::from(format!("> {}", action.label))
            .style(if hovered {
                theme.style_brand()
            } else {
                theme.style_cream()
            })
            .alignment(Alignment::Center),
        Line::from(action.command)
            .style(theme.style_muted())
            .alignment(Alignment::Center),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(card_border_style(theme, hovered, hover_pulse));

    frame.render_widget(
        Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Center)
            .style(theme.style_text()),
        area,
    );

    regions.push(ClickableRegion {
        rect: area,
        action: ClickAction::Command(action.command),
    });
}

fn render_hint(frame: &mut ratatui::Frame, area: Rect, theme: &Theme) {
    let text = Text::from(vec![Line::from(
        "鼠标悬停卡片有动画效果，点击执行，或在下方输入命令",
    )
    .style(theme.style_hint())
    .alignment(Alignment::Center)]);
    frame.render_widget(Paragraph::new(text), area);
}

fn is_hovered(rect: Rect, mouse_pos: Option<(u16, u16)>) -> bool {
    mouse_pos.is_some_and(|pos| rect.contains(pos.into()))
}

fn card_border_style(theme: &Theme, hovered: bool, pulse: bool) -> Style {
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
    fn test_render_locked_smoke() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut regions = Vec::new();
        terminal
            .draw(|frame| render(frame, frame.area(), &mut regions, None, false, 0))
            .unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf.content.iter().map(|cell| cell.symbol()).collect();
        assert!(content.contains('█'));
        assert!(content.contains('╗'));
        assert!(content.contains("/unlock"));
        assert_eq!(regions.len(), ACTIONS.len());
    }
}
