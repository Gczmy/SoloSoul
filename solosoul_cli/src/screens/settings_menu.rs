//! 设置菜单屏幕（语言 / 主题 / 自定义偏好键值 / 导出调试包）。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{ClickAction, ClickableRegion};
use crate::i18n::I18n;
use crate::t;
use crate::theme::Theme;

/// 菜单项数。
pub const NUM_ITEMS: usize = 4;

/// 单个菜单项的展示数据（在 render 时根据当前 i18n 缓存动态填充）。
struct MenuItem {
    title: String,
    desc: String,
    value: String,
}

/// 渲染设置菜单。
#[allow(clippy::too_many_arguments)]
pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    selected: usize,
    current_language: &str,
    current_theme: &str,
    regions: &mut Vec<ClickableRegion>,
    mouse_pos: Option<(u16, u16)>,
    i18n: &I18n,
) {
    let theme = Theme::load();
    let items = build_menu_items(current_language, current_theme, i18n);
    let selected = selected.min(items.len().saturating_sub(1));

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let title =
        Paragraph::new(Line::from(t!(i18n, "settings-title")).bold()).alignment(Alignment::Center);
    frame.render_widget(title, layout[0]);

    let card_height: u16 = 4;
    let total_height = items.len() as u16 * card_height;
    let card_area = if layout[1].height >= total_height {
        let v_spacer = layout[1].height.saturating_sub(total_height) / 2;
        let width = layout[1].width.min(60);
        let h_spacer = layout[1].width.saturating_sub(width) / 2;
        Rect {
            x: layout[1].x + h_spacer,
            y: layout[1].y + v_spacer,
            width,
            height: total_height,
        }
    } else {
        layout[1]
    };

    let item_areas: Vec<Rect> = Layout::default()
        .direction(Direction::Vertical)
        .constraints((0..items.len()).map(|_| Constraint::Length(card_height)))
        .split(card_area)
        .to_vec();

    for (idx, item) in items.iter().enumerate() {
        render_item(
            frame,
            item_areas[idx],
            idx,
            item,
            idx == selected,
            mouse_pos,
            &theme,
            regions,
            i18n,
        );
    }

    frame.render_widget(
        Paragraph::new(Line::from(t!(i18n, "hint-click")).dark_gray()).alignment(Alignment::Center),
        layout[2],
    );
}

#[allow(clippy::too_many_arguments)]
fn render_item(
    frame: &mut ratatui::Frame,
    area: Rect,
    index: usize,
    item: &MenuItem,
    is_selected: bool,
    mouse_pos: Option<(u16, u16)>,
    theme: &Theme,
    regions: &mut Vec<ClickableRegion>,
    i18n: &I18n,
) {
    let hovered = mouse_pos.is_some_and(|pos| area.contains(pos.into()));
    let marker = if is_selected || hovered { "▸ " } else { "  " };

    let title_style = if hovered || is_selected {
        Style::default()
            .fg(theme.brand)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.cream)
    };
    let value_style = if hovered || is_selected {
        Style::default().fg(theme.brand)
    } else {
        Style::default().fg(theme.muted)
    };
    let border_style = if is_selected {
        theme.style_border(true)
    } else if hovered {
        theme.style_brand_dim()
    } else {
        theme.style_border(false)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(vec![
            Span::styled(marker, title_style),
            Span::styled(item.title.clone(), title_style),
            Span::raw("  "),
            Span::styled(item.desc.clone(), Style::default().fg(theme.muted)),
        ]),
        Line::from(vec![
            Span::raw(format!("{} · ", t!(i18n, "settings-current"))),
            Span::styled(item.value.clone(), value_style),
        ]),
    ];

    frame.render_widget(Paragraph::new(lines), inner);

    regions.push(ClickableRegion {
        rect: area,
        action: ClickAction::SettingItem(index),
    });
}

fn build_menu_items(current_language: &str, current_theme: &str, i18n: &I18n) -> Vec<MenuItem> {
    vec![
        MenuItem {
            title: t!(i18n, "settings-language"),
            desc: t!(i18n, "settings-language-desc"),
            value: display_lang_value(current_language, i18n),
        },
        MenuItem {
            title: t!(i18n, "settings-theme"),
            desc: t!(i18n, "settings-theme-desc"),
            value: display_theme_value(current_theme, i18n),
        },
        MenuItem {
            title: t!(i18n, "settings-preference"),
            desc: t!(i18n, "settings-preference-desc"),
            value: String::from("key=value"),
        },
        MenuItem {
            title: t!(i18n, "settings-debug-log"),
            desc: t!(i18n, "settings-debug-log-desc"),
            value: String::from("/debug_log"),
        },
    ]
}

fn display_lang_value(current: &str, _i18n: &I18n) -> String {
    if current.is_empty() {
        String::new()
    } else {
        current.to_string()
    }
}

fn display_theme_value(current: &str, _i18n: &I18n) -> String {
    match current {
        "system" => "system".to_string(),
        "light" => "light".to_string(),
        "dark" => "dark".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::I18n;
    use ratatui::backend::TestBackend;

    #[test]
    fn test_render_settings_menu_smoke_basic() {
        let backend = TestBackend::new(80, 36);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut regions = Vec::new();
        let i18n = I18n::new("zh-CN");
        terminal
            .draw(|frame| {
                render(
                    frame,
                    frame.area(),
                    0,
                    "zh-CN",
                    "dark",
                    &mut regions,
                    None,
                    &i18n,
                )
            })
            .unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf.content.iter().map(|cell| cell.symbol()).collect();
        assert!((content.chars().any(|c| c == '设') && content.chars().any(|c| c == '置')));
        assert!((content.chars().any(|c| c == '语') && content.chars().any(|c| c == '言')));
        assert!((content.chars().any(|c| c == '主') && content.chars().any(|c| c == '题')));
        assert!(content.contains("zh-CN"));
        assert_eq!(regions.len(), NUM_ITEMS);
    }

    #[test]
    fn test_render_settings_menu_smoke_compact() {
        let backend = TestBackend::new(80, 18);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut regions = Vec::new();
        let i18n = I18n::new("zh-CN");
        terminal
            .draw(|frame| {
                render(
                    frame,
                    frame.area(),
                    2,
                    "",
                    "system",
                    &mut regions,
                    Some((2, 5)),
                    &i18n,
                )
            })
            .unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf.content.iter().map(|cell| cell.symbol()).collect();
        assert!((content.chars().any(|c| c == '设') && content.chars().any(|c| c == '置')));
        assert_eq!(regions.len(), NUM_ITEMS);
    }
}
