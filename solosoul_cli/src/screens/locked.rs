//! 已锁定状态主界面 —— 响应式 Logo + 可滚动垂直动作按钮。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Text};
use ratatui::widgets::{
    Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};

use crate::app::{ClickAction, ClickableRegion};
use crate::theme::Theme;

pub struct LockedAction {
    pub label: &'static str,
    pub command: &'static str,
}

pub const ACTIONS: &[LockedAction] = &[
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

pub const ACTION_COUNT: usize = ACTIONS.len();

/// 单个按钮高度：上边框 + 内容 + 下边框。
const BUTTON_HEIGHT: u16 = 3;

/// 渲染已锁定主界面。
pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    regions: &mut Vec<ClickableRegion>,
    mouse_pos: Option<(u16, u16)>,
    hover_pulse: bool,
    sheen_offset: u16,
    selected: usize,
) {
    let theme = Theme::load();
    let selected = selected.min(ACTIONS.len().saturating_sub(1));

    // 水平边距 4，垂直边距 0，最大化利用 24 行终端。
    let inner = area.inner(Margin::new(4, 0));

    // 内容区高度 >= 16 时才显示 banner，否则只显示可滚动按钮。
    let show_banner = inner.height >= 16;
    let options_area = if show_banner {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(8), Constraint::Min(0)])
            .split(inner);
        crate::screens::logo::render(frame, chunks[0], &theme, sheen_offset, " 已锁定 ");
        chunks[1]
    } else {
        inner
    };

    render_options(
        frame,
        options_area,
        &theme,
        regions,
        mouse_pos,
        hover_pulse,
        selected,
    );
}

fn render_options(
    frame: &mut ratatui::Frame,
    area: Rect,
    theme: &Theme,
    regions: &mut Vec<ClickableRegion>,
    mouse_pos: Option<(u16, u16)>,
    hover_pulse: bool,
    selected: usize,
) {
    let total_height = ACTIONS.len() as u16 * BUTTON_HEIGHT;

    // 能完整显示所有按钮时：在可用区域内垂直 + 水平居中。
    if area.height >= total_height {
        let button_width = area.width.min(50);
        let h_spacer = (area.width.saturating_sub(button_width)) / 2;
        let v_spacer = (area.height.saturating_sub(total_height)) / 2;
        let centered = Rect {
            x: area.x + h_spacer,
            y: area.y + v_spacer,
            width: button_width,
            height: total_height,
        };

        let items: Vec<Rect> = Layout::default()
            .direction(Direction::Vertical)
            .constraints(ACTIONS.iter().map(|_| Constraint::Length(BUTTON_HEIGHT)))
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
                idx == selected,
            );
        }
        return;
    }

    // 空间不足：显示可见部分并绘制滚动条。
    let visible = (area.height / BUTTON_HEIGHT).max(1) as usize;
    let max_offset = ACTIONS.len().saturating_sub(visible);
    let scroll_offset = selected
        .saturating_sub(visible.saturating_sub(1))
        .min(max_offset);

    let button_width = area.width.saturating_sub(2).min(50);
    let list_area = Rect {
        x: area.x,
        y: area.y,
        width: button_width,
        height: visible as u16 * BUTTON_HEIGHT,
    };

    let items: Vec<Rect> = Layout::default()
        .direction(Direction::Vertical)
        .constraints((0..visible).map(|_| Constraint::Length(BUTTON_HEIGHT)))
        .split(list_area)
        .to_vec();

    for (vis_idx, action) in ACTIONS.iter().skip(scroll_offset).take(visible).enumerate() {
        let actual_idx = scroll_offset + vis_idx;
        render_action_button(
            frame,
            items[vis_idx],
            theme,
            action,
            regions,
            mouse_pos,
            hover_pulse,
            actual_idx == selected,
        );
    }

    let scrollbar_area = Rect {
        x: area.x + button_width,
        y: area.y,
        width: area.width.saturating_sub(button_width),
        height: list_area.height,
    };
    let mut scrollbar_state = ScrollbarState::new(ACTIONS.len())
        .position(selected)
        .viewport_content_length(visible);
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None),
        scrollbar_area,
        &mut scrollbar_state,
    );
}

#[allow(clippy::too_many_arguments)]
fn render_action_button(
    frame: &mut ratatui::Frame,
    area: Rect,
    theme: &Theme,
    action: &LockedAction,
    regions: &mut Vec<ClickableRegion>,
    mouse_pos: Option<(u16, u16)>,
    hover_pulse: bool,
    is_selected: bool,
) {
    let hovered = is_hovered(area, mouse_pos);
    let focused = hovered || is_selected;
    let text = Text::from(vec![Line::from(format!(
        "> {}  {}",
        action.label, action.command
    ))
    .style(if focused {
        theme.style_brand()
    } else {
        theme.style_cream()
    })]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(card_border_style(theme, focused, hover_pulse));

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

fn card_border_style(theme: &Theme, focused: bool, pulse: bool) -> Style {
    if focused {
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
            .draw(|frame| render(frame, frame.area(), &mut regions, None, false, 0, 0))
            .unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf.content.iter().map(|cell| cell.symbol()).collect();
        assert!(content.contains('█'));
        assert!(content.contains('╗'));
        assert!(content.contains("/unlock"));
        assert_eq!(regions.len(), ACTIONS.len());
    }
}
