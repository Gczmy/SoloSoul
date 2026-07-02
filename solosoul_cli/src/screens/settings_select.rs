//! 泛型设置选择屏幕，供语言/主题/其他选项列表复用。
//!
//! 替代 `settings_language_select.rs` 和 `settings_theme_select.rs` 的重复实现。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::app::{ClickAction, ClickableRegion};
use crate::theme::Theme;

/// 调用者构造可点击动作的回调。参数：(code/name, rect, regions)
pub struct SelectOptionsConfig<'a> {
    /// 选项列表 `(code, display_name)`。
    pub options: &'a [(&'a str, &'a str)],
    /// 当前选中的码（用于 "当前" 标记）。
    pub current: &'a str,
    /// 键盘选中的索引。
    pub selected: usize,
    /// 页面标题，如 "选择语言"。
    pub title: &'a str,
    /// 命令名称，如 "/language"。
    pub command: &'a str,
    /// 底部帮助文字。
    pub help: &'a str,
    /// 回到译 action 构造器：参数为 code。
    pub click_action: fn(&'a str) -> ClickAction,
}

/// 渲染一个通用的选项列表屏幕。
///
/// 输出可点击区域到 `regions`；frame 被赋予 `frame`。
#[allow(clippy::too_many_arguments)]
pub fn render_option_list<'a>(
    frame: &mut ratatui::Frame,
    area: Rect,
    config: SelectOptionsConfig<'a>,
    regions: &mut Vec<ClickableRegion>,
    _mouse_pos: Option<(u16, u16)>,
) {
    let theme = Theme::load();
    let selected = config.selected.min(config.options.len().saturating_sub(1));

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let title = Paragraph::new(Line::from(config.title).bold()).alignment(Alignment::Center);
    frame.render_widget(title, layout[0]);

    let row_height: u16 = 3;
    let total_height = config.options.len() as u16 * row_height;
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
        .title(format!(" {} ", config.command))
        .border_style(theme.style_border(false));
    frame.render_widget(block.clone(), list_area);

    let inner = block.inner(list_area);
    let row_areas: Vec<Rect> = Layout::default()
        .direction(Direction::Vertical)
        .constraints((0..config.options.len()).map(|_| Constraint::Length(row_height)))
        .split(inner)
        .to_vec();

    let mut list_state = ListState::default();
    list_state.select(Some(selected));

    let row_items: Vec<ListItem> = config
        .options
        .iter()
        .enumerate()
        .map(|(i, (code, display))| {
            let is_current = *code == config.current;
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
                Span::styled(format!("{:<10}", code), style),
                Span::raw("  "),
                Span::styled(*display, style),
                Span::styled(current_marker, Style::default().fg(theme.muted)),
            ]))
        })
        .collect();

    let list = List::new(row_items).highlight_style(Style::default().bg(theme.bg).fg(theme.brand));
    frame.render_stateful_widget(list, inner, &mut list_state);

    for (i, (code, _)) in config.options.iter().enumerate() {
        let rect = if i < row_areas.len() {
            row_areas[i]
        } else {
            continue;
        };
        regions.push(ClickableRegion {
            rect,
            action: (config.click_action)(code),
        });
    }

    frame.render_widget(
        Paragraph::new(Line::from(config.help.dark_gray()))
            .alignment(Alignment::Center),
        layout[2],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    #[test]
    fn test_render_option_list_smoke() {
        let backend = TestBackend::new(80, 30);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut regions = Vec::new();
        let config = SelectOptionsConfig {
            options: &[("zh-CN", "简体中文"), ("en-US", "English")],
            current: "zh-CN",
            selected: 0,
            title: "测试选择",
            command: "/test",
            help: "↑/↓ 选择 · Enter 确认",
            click_action: |s| ClickAction::ApplyLanguage(s.to_string()),
        };
        terminal
            .draw(|frame| render_option_list(frame, frame.area(), config, &mut regions, None))
            .unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf.content.iter().map(|cell| cell.symbol()).collect();
        assert!(content.contains("zh-CN"));
        assert!(content.contains("en-US"));
        assert_eq!(regions.len(), 2);
    }

    #[test]
    fn test_render_option_list_compact() {
        let backend = TestBackend::new(80, 18);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut regions = Vec::new();
        let config = SelectOptionsConfig {
            options: &[("system", "跟随系统"), ("light", "浅色"), ("dark", "深色")],
            current: "system",
            selected: 1,
            title: "选择主题",
            command: "/theme",
            help: "↑/↓ 选择 · Enter 或点击应用",
            click_action: |s| ClickAction::ApplyTheme(s.to_string()),
        };
        terminal
            .draw(|frame| render_option_list(frame, frame.area(), config, &mut regions, None))
            .unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf.content.iter().map(|cell| cell.symbol()).collect();
        assert!(
            (content.chars().any(|c| c == '跟')
                && content.chars().any(|c| c == '随')
                && content.chars().any(|c| c == '系')
                && content.chars().any(|c| c == '统'))
        );
        assert!(regions.len() <= 3);
    }
}
