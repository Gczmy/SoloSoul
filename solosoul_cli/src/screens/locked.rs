//! 已锁定状态主界面 —— 大品牌名 + 垂直动作按钮 + 悬停动画。

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

    // 默认 24 行终端：状态栏 1 + 内容区 20 + 命令行 3。
    // 为了放下 6 行 Logo（带边框 8）+ 4 个高度 3 的垂直按钮（12），垂直边距用 0。
    let inner = area.inner(Margin::new(4, 0));
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // 6 行 Logo + 2 行边框
            Constraint::Length(12), // 4 × 3 行垂直按钮
        ])
        .split(inner);

    crate::screens::logo::render(frame, chunks[0], &theme, sheen_offset, " 已锁定 ");
    render_actions(frame, chunks[1], &theme, regions, mouse_pos, hover_pulse);
}

fn render_actions(
    frame: &mut ratatui::Frame,
    area: Rect,
    theme: &Theme,
    regions: &mut Vec<ClickableRegion>,
    mouse_pos: Option<(u16, u16)>,
    hover_pulse: bool,
) {
    // 按钮整体水平居中
    let button_width = area.width.min(50);
    let h_spacer = (area.width.saturating_sub(button_width)) / 2;
    let centered = Rect {
        x: area.x + h_spacer,
        y: area.y,
        width: button_width,
        height: area.height,
    };

    let items: Vec<Rect> = Layout::default()
        .direction(Direction::Vertical)
        .constraints(ACTIONS.iter().map(|_| Constraint::Length(3)))
        .split(centered)
        .to_vec();

    for (idx, action) in ACTIONS.iter().enumerate() {
        render_action_button(
            frame,
            items[idx],
            theme,
            action,
            regions,
            mouse_pos,
            hover_pulse,
        );
    }
}

fn render_action_button(
    frame: &mut ratatui::Frame,
    area: Rect,
    theme: &Theme,
    action: &LockedAction,
    regions: &mut Vec<ClickableRegion>,
    mouse_pos: Option<(u16, u16)>,
    hover_pulse: bool,
) {
    let hovered = is_hovered(area, mouse_pos);
    let text = Text::from(vec![Line::from(format!(
        "> {}  {}",
        action.label, action.command
    ))
    .style(if hovered {
        theme.style_brand()
    } else {
        theme.style_cream()
    })]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(card_border_style(theme, hovered, hover_pulse));

    frame.render_widget(
        Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Left)
            .style(theme.style_text()),
        area,
    );

    regions.push(ClickableRegion {
        rect: area,
        action: ClickAction::Command(action.command),
    });
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
