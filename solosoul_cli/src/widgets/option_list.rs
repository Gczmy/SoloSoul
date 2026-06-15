//! 可复用垂直选项列表。
//!
//! 支持在剩余空间中垂直 + 水平居中、文本左对齐、终端高度不足时显示滚动条，
//! 并为每个选项生成可点击区域。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Text};
use ratatui::widgets::{
    Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};

use crate::app::{ClickAction, ClickableRegion};
use crate::theme::Theme;

/// 单个按钮高度：上边框 + 内容行 + 下边框。
pub const BUTTON_HEIGHT: u16 = 3;

/// 列表中的一项。
#[derive(Debug, Clone)]
pub struct OptionItem {
    pub label: &'static str,
    /// 命令文本，None 表示无命令（如进入向导）。
    pub command: Option<&'static str>,
    /// 解释文本，显示在命令之后。
    pub desc: &'static str,
    pub action: ClickAction,
}

/// 简单估算字符串在终端中的显示宽度（ASCII 1，其它 2）。
fn display_width(s: &str) -> usize {
    s.chars().map(|c| if c.is_ascii() { 1 } else { 2 }).sum()
}

/// 渲染垂直选项列表。
///
/// - `items`：全部选项定义。
/// - `selected`：当前选中索引。
/// - `regions`：输出可点击区域，供鼠标事件使用。
pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    items: &[OptionItem],
    selected: usize,
    theme: &Theme,
    regions: &mut Vec<ClickableRegion>,
    mouse_pos: Option<(u16, u16)>,
) {
    let selected = selected.min(items.len().saturating_sub(1));
    let max_label_width = items
        .iter()
        .map(|i| display_width(i.label))
        .max()
        .unwrap_or(0);
    let total_height = items.len() as u16 * BUTTON_HEIGHT;

    // 能完整显示所有按钮时：在可用区域内垂直 + 水平居中。
    if area.height >= total_height {
        let button_width = area.width.min(50);
        let h_spacer = area.width.saturating_sub(button_width) / 2;
        let v_spacer = area.height.saturating_sub(total_height) / 2;
        let centered = Rect {
            x: area.x + h_spacer,
            y: area.y + v_spacer,
            width: button_width,
            height: total_height,
        };

        let item_areas: Vec<Rect> = Layout::default()
            .direction(Direction::Vertical)
            .constraints((0..items.len()).map(|_| Constraint::Length(BUTTON_HEIGHT)))
            .split(centered)
            .to_vec();

        for (idx, item) in items.iter().enumerate() {
            render_button(
                frame,
                item_areas[idx],
                item,
                theme,
                regions,
                mouse_pos,
                idx == selected,
                max_label_width,
            );
        }
        return;
    }

    // 空间不足：显示可见部分并绘制滚动条，整体仍保持水平居中。
    let visible = (area.height / BUTTON_HEIGHT).max(1) as usize;
    let max_offset = items.len().saturating_sub(visible);
    let scroll_offset = selected
        .saturating_sub(visible.saturating_sub(1))
        .min(max_offset);

    const SCROLLBAR_WIDTH: u16 = 2;
    let button_width = area.width.saturating_sub(SCROLLBAR_WIDTH).min(50);
    let content_width = button_width + SCROLLBAR_WIDTH;
    let h_spacer = area.width.saturating_sub(content_width) / 2;

    let list_area = Rect {
        x: area.x + h_spacer,
        y: area.y,
        width: button_width,
        height: visible as u16 * BUTTON_HEIGHT,
    };

    let item_areas: Vec<Rect> = Layout::default()
        .direction(Direction::Vertical)
        .constraints((0..visible).map(|_| Constraint::Length(BUTTON_HEIGHT)))
        .split(list_area)
        .to_vec();

    for (vis_idx, item) in items.iter().skip(scroll_offset).take(visible).enumerate() {
        let actual_idx = scroll_offset + vis_idx;
        render_button(
            frame,
            item_areas[vis_idx],
            item,
            theme,
            regions,
            mouse_pos,
            actual_idx == selected,
            max_label_width,
        );
    }

    let scrollbar_area = Rect {
        x: list_area.x + button_width,
        y: area.y,
        width: SCROLLBAR_WIDTH,
        height: list_area.height,
    };
    let mut scrollbar_state = ScrollbarState::new(items.len())
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
fn render_button(
    frame: &mut ratatui::Frame,
    area: Rect,
    item: &OptionItem,
    theme: &Theme,
    regions: &mut Vec<ClickableRegion>,
    mouse_pos: Option<(u16, u16)>,
    is_selected: bool,
    max_label_width: usize,
) {
    let hovered = is_hovered(area, mouse_pos);
    let marker = if is_selected { "> " } else { "  " };

    // 悬停时标题使用品牌色；选中但无悬停时显示普通标题色。
    let title_style = if hovered {
        theme.style_brand()
    } else {
        theme.style_cream()
    };
    // 选中时使用亮蓝色边框；仅悬停未选中时使用奶油色边框。
    let border_style = if is_selected {
        theme.style_border(true)
    } else if hovered {
        theme.style_cream()
    } else {
        theme.style_border(false)
    };

    const MIN_GAP: usize = 4;
    let label_pad = max_label_width.saturating_sub(display_width(item.label));
    let label_padded = format!("{}{}", item.label, " ".repeat(label_pad));
    let content = if let Some(cmd) = item.command {
        format!(
            "{}{}{} {} · {}",
            marker,
            label_padded,
            " ".repeat(MIN_GAP),
            cmd,
            item.desc
        )
    } else {
        format!(
            "{}{}{} {}",
            marker,
            label_padded,
            " ".repeat(MIN_GAP),
            item.desc
        )
    };

    let text = Text::from(vec![Line::from(content).style(title_style)]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style);

    frame.render_widget(
        Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Left)
            .style(theme.style_text()),
        area,
    );

    regions.push(ClickableRegion {
        rect: area,
        action: item.action.clone(),
    });
}

fn is_hovered(rect: Rect, mouse_pos: Option<(u16, u16)>) -> bool {
    mouse_pos.is_some_and(|pos| rect.contains(pos.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    #[test]
    fn test_render_option_list_smoke() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let items = &[
            OptionItem {
                label: "选项一",
                command: Some("/cmd1"),
                desc: "解释一",
                action: ClickAction::Command("/cmd1"),
            },
            OptionItem {
                label: "选项二",
                command: Some("/cmd2"),
                desc: "解释二",
                action: ClickAction::Command("/cmd2"),
            },
        ];
        let mut regions = Vec::new();
        terminal
            .draw(|frame| {
                render(
                    frame,
                    frame.area(),
                    items,
                    0,
                    &Theme::load(),
                    &mut regions,
                    None,
                )
            })
            .unwrap();
        assert_eq!(regions.len(), 2);
    }
}
