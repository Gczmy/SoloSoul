//! 设置 → 主题选择屏幕。
//!
//! 渲染委托给泛型 `settings_select::render_option_list`。

use ratatui::layout::Rect;

use crate::app::{ClickAction, ClickableRegion};

/// 当前内置的可选主题列表。
pub const OPTIONS: &[(&str, &str)] = &[
    ("system", "跟随系统"),
    ("light", "浅色"),
    ("dark", "深色"),
];

/// 渲染主题选择页。
#[allow(clippy::too_many_arguments)]
pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    selected: usize,
    current: &str,
    regions: &mut Vec<ClickableRegion>,
    mouse_pos: Option<(u16, u16)>,
) {
    super::settings_select::render_option_list(
        frame,
        area,
        super::settings_select::SelectOptionsConfig {
            options: OPTIONS,
            current,
            selected,
            title: "选择主题",
            command: "/theme",
            help: "↑/↓ 选择 · Enter 或点击应用 · Esc 取消",
            click_action: |s| ClickAction::ApplyTheme(s.to_string()),
        },
        regions,
        mouse_pos,
    );
}
