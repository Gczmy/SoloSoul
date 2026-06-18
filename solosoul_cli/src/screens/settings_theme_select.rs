//! 设置 → 主题选择屏幕。
//!
//! 列出可选主题（跟随系统 / 浅色 / 深色）。选中后写入
//! `ui_preferences.json` 并回退到 `SettingsMenu`，通过 toast 显示成功消息。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::app::{ClickAction, ClickableRegion};
use crate::theme::Theme;

/// 当前内置的可选主题列表。
pub const OPTIONS: &[(&str, &str)] = &[("system", "跟随系统"), ("light", "浅色"), ("dark", "深色")];

/// 渲染主题选择页。
///
/// - `selected`：键盘 ↑/↓ 选中的索引。
/// - `current`：当前生效的主题名（用于 "当前" 标记）。
/// - `regions`：每个候选项对应一个 `ClickAction::ApplyTheme(name)` 区域。
#[allow(clippy::too_many_arguments)]
pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    selected: usize,
    current: &str,
    regions: &mut Vec<ClickableRegion>,
    mouse_pos: Option<(u16, u16)>,
) {
    let theme = Theme::load();
    let selected = selected.min(OPTIONS.len().saturating_sub(1));

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let title = Paragraph::new(Line::from("选择主题").bold()).alignment(Alignment::Center);
    frame.render_widget(title, layout[0]);

    let row_height: u16 = 3;
    let total_height = OPTIONS.len() as u16 * row_height;
    let list_area = if layout[1].height >= total_height + 2 {
        let v_spacer = layout[1].height.saturating_sub(total_height + 2) / 2;
        Rect {
            x: layout[1].x,
            y: layout[1].y + v_spacer,
            width: layout[1].width,
            height: total_height + 2,
        }
    } else {
        layout[1]
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" /theme ")
        .border_style(theme.style_border(false));
    frame.render_widget(block.clone(), list_area);

    let inner = block.inner(list_area);
    let row_areas: Vec<Rect> = Layout::default()
        .direction(Direction::Vertical)
        .constraints((0..OPTIONS.len()).map(|_| Constraint::Length(row_height)))
        .split(inner)
        .to_vec();

    let mut list_state = ListState::default();
    list_state.select(Some(selected));

    let row_items: Vec<ListItem> = OPTIONS
        .iter()
        .enumerate()
        .map(|(i, (name, display))| {
            let is_current = *name == current;
            let marker = if i == selected { "▸ " } else { "  " };
            let current_marker = if is_current { "  · 当前" } else { "" };
            let style = if i == selected {
                Style::default()
                    .fg(theme.brand)
                    .add_modifier(Modifier::BOLD)
            } else if is_current {
                Style::default().fg(theme.success)
            } else {
                Style::default().fg(theme.cream)
            };
            ListItem::new(Line::from(vec![
                Span::styled(marker, style),
                Span::styled(format!("{:<10}", name), style),
                Span::raw("  "),
                Span::styled(*display, style),
                Span::styled(current_marker, Style::default().fg(theme.muted)),
            ]))
        })
        .collect();

    let list = List::new(row_items).highlight_style(Style::default().bg(theme.bg).fg(theme.brand));
    frame.render_stateful_widget(list, inner, &mut list_state);

    for (i, (name, _)) in OPTIONS.iter().enumerate() {
        let rect = if i < row_areas.len() {
            row_areas[i]
        } else {
            continue;
        };
        let _ = mouse_pos.is_some_and(|p| rect.contains(p.into()));
        regions.push(ClickableRegion {
            rect,
            action: ClickAction::ApplyTheme((*name).to_string()),
        });
    }

    frame.render_widget(
        Paragraph::new(Line::from(
            "↑/↓ 选择 · Enter 或点击应用 · Esc 取消".dark_gray(),
        ))
        .alignment(Alignment::Center),
        layout[2],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    #[test]
    fn test_render_theme_select_smoke() {
        let backend = TestBackend::new(80, 30);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut regions = Vec::new();
        terminal
            .draw(|frame| render(frame, frame.area(), 0, "system", &mut regions, None))
            .unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf.content.iter().map(|cell| cell.symbol()).collect();
        assert!(
            (content.chars().any(|c| c == '选')
                && content.chars().any(|c| c == '择')
                && content.chars().any(|c| c == '主')
                && content.chars().any(|c| c == '题'))
        );
        assert!(
            (content.chars().any(|c| c == '跟')
                && content.chars().any(|c| c == '随')
                && content.chars().any(|c| c == '系')
                && content.chars().any(|c| c == '统'))
        );
        assert!((content.chars().any(|c| c == '浅') && content.chars().any(|c| c == '色')));
        assert!((content.chars().any(|c| c == '深') && content.chars().any(|c| c == '色')));
        assert_eq!(regions.len(), OPTIONS.len());
    }

    #[test]
    fn test_render_theme_select_compact() {
        let backend = TestBackend::new(80, 18);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut regions = Vec::new();
        terminal
            .draw(|frame| render(frame, frame.area(), 2, "dark", &mut regions, None))
            .unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf.content.iter().map(|cell| cell.symbol()).collect();
        assert!(
            (content.chars().any(|c| c == '选')
                && content.chars().any(|c| c == '择')
                && content.chars().any(|c| c == '主')
                && content.chars().any(|c| c == '题'))
        );
        assert!(
            (content.chars().any(|c| c == '深') && content.chars().any(|c| c == '色'))
                || (content.chars().any(|c| c == '跟')
                    && content.chars().any(|c| c == '随')
                    && content.chars().any(|c| c == '系')
                    && content.chars().any(|c| c == '统'))
        );
        assert!(regions.len() <= OPTIONS.len());
    }
}
