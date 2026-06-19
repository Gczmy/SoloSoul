//! 设置菜单屏幕（语言 / 主题 / 自定义偏好键值 / 导出调试包）。
//!
//! 与 `locked.rs` / `welcome.rs` 类似的 list 模式，但 push 的 `ClickAction`
//! 直接指向子动作（不再走 `command_input`）以避免点击设置再次触发
//! "用法: /setting <key> <value>" 错误。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{ClickAction, ClickableRegion};
use crate::theme::Theme;

/// 菜单项数。
pub const NUM_ITEMS: usize = 4;

/// 单个菜单项的展示数据（在 render 时根据当前 i18n 缓存动态填充）。
struct MenuItem {
    title: &'static str,
    desc: &'static str,
    value: String,
}

/// 渲染设置菜单。
///
/// - `selected`：当前通过键盘 ↑/↓ 选中的索引（0..`NUM_ITEMS`）。
/// - `current_language` / `current_theme` 来自父态进入时缓存的
///   `ui_preferences.json` 字段，避免每次进入都读盘。
/// - `mouse_pos` 用于悬停高亮，`regions` 收集可点击区域（每个项不等大）。
#[allow(clippy::too_many_arguments)]
pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    selected: usize,
    current_language: &str,
    current_theme: &str,
    regions: &mut Vec<ClickableRegion>,
    mouse_pos: Option<(u16, u16)>,
) {
    let theme = Theme::load();
    let items = build_menu_items(current_language, current_theme);
    let selected = selected.min(items.len().saturating_sub(1));

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let title = Paragraph::new(Line::from("设置").bold()).alignment(Alignment::Center);
    frame.render_widget(title, layout[0]);

    // 4 个菜单项，每张卡片固定 4 行（边框 + 标题 + 描述 + 当前值）。
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
        // 紧凑模式：占满可用高度，由框架裁剪。
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
        );
    }

    frame.render_widget(
        Paragraph::new(
            Line::from("↑/↓ 选择 · Enter 确认 · Esc 返回 home · 鼠标可点击").dark_gray(),
        )
        .alignment(Alignment::Center),
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
            Span::styled(item.title, title_style),
            Span::raw("  "),
            Span::styled(item.desc, Style::default().fg(theme.muted)),
        ]),
        Line::from(vec![
            Span::raw("    当前 · "),
            Span::styled(item.value.clone(), value_style),
        ]),
    ];

    frame.render_widget(Paragraph::new(lines), inner);

    regions.push(ClickableRegion {
        rect: area,
        action: ClickAction::SettingItem(index),
    });
}

fn build_menu_items<'a>(current_language: &'a str, current_theme: &'a str) -> Vec<MenuItem> {
    vec![
        MenuItem {
            title: "语言",
            desc: "切换界面语言（zh-CN / en-US / ja-JP）",
            value: display_lang_value(current_language),
        },
        MenuItem {
            title: "主题",
            desc: "切换界面主题（跟随系统 / 浅色 / 深色）",
            value: display_theme_value(current_theme),
        },
        MenuItem {
            title: "自定义偏好",
            desc: "写入加密 profile preferences 中的任意键值对",
            value: String::from("键值对模式"),
        },
        MenuItem {
            title: "导出调试包",
            desc: "导出审计日志 + 脱敏系统信息到 logs/",
            value: String::from("/debug_log"),
        },
    ]
}

fn display_lang_value(current: &str) -> String {
    if current.is_empty() {
        "未设置".to_string()
    } else {
        current.to_string()
    }
}

fn display_theme_value(current: &str) -> String {
    match current {
        "system" => "跟随系统".to_string(),
        "light" => "浅色".to_string(),
        "dark" => "深色".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    #[test]
    fn test_render_settings_menu_smoke_basic() {
        let backend = TestBackend::new(80, 36);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut regions = Vec::new();
        terminal
            .draw(|frame| render(frame, frame.area(), 0, "zh-CN", "dark", &mut regions, None))
            .unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf.content.iter().map(|cell| cell.symbol()).collect();
        assert!((content.chars().any(|c| c == '设') && content.chars().any(|c| c == '置')));
        assert!((content.chars().any(|c| c == '语') && content.chars().any(|c| c == '言')));
        assert!((content.chars().any(|c| c == '主') && content.chars().any(|c| c == '题')));
        assert!(
            (content.chars().any(|c| c == '自')
                && content.chars().any(|c| c == '定')
                && content.chars().any(|c| c == '义')
                && content.chars().any(|c| c == '偏')
                && content.chars().any(|c| c == '好'))
        );
        assert!(
            (content.chars().any(|c| c == '导')
                && content.chars().any(|c| c == '出')
                && content.chars().any(|c| c == '调')
                && content.chars().any(|c| c == '试')
                && content.chars().any(|c| c == '包'))
        );
        assert!(content.contains("zh-CN"));
        assert_eq!(regions.len(), NUM_ITEMS);
    }

    #[test]
    fn test_render_settings_menu_smoke_compact() {
        // 终端低高度（高度不足以整套显示）时仍能渲染且不 panic。
        let backend = TestBackend::new(80, 18);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut regions = Vec::new();
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
                )
            })
            .unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf.content.iter().map(|cell| cell.symbol()).collect();
        assert!((content.chars().any(|c| c == '设') && content.chars().any(|c| c == '置')));
        // 紧凑模式下未设置 / 跟随系统 文字应出现
        assert!(
            (content.chars().any(|c| c == '未')
                && content.chars().any(|c| c == '设')
                && content.chars().any(|c| c == '置'))
                || (content.chars().any(|c| c == '跟')
                    && content.chars().any(|c| c == '随')
                    && content.chars().any(|c| c == '系')
                    && content.chars().any(|c| c == '统'))
        );
        assert_eq!(regions.len(), NUM_ITEMS);
    }
}
