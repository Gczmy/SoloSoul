//! 设置 → 语言选择屏幕。
//!
//! 渲染委托给泛型 `settings_select::render_option_list`。

use ratatui::layout::Rect;

use crate::app::{ClickAction, ClickableRegion};
use crate::i18n::I18n;
use crate::t;

/// 当前内置的可选语言列表。
pub const OPTIONS: &[(&str, &str)] = &[
    ("zh-CN", "简体中文"),
    ("en-US", "English"),
    ("ja-JP", "日本語"),
];

/// 渲染语言选择页。
#[allow(clippy::too_many_arguments)]
pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    selected: usize,
    current: &str,
    regions: &mut Vec<ClickableRegion>,
    mouse_pos: Option<(u16, u16)>,
    i18n: &I18n,
) {
    super::settings_select::render_option_list(
        frame,
        area,
        super::settings_select::SelectOptionsConfig {
            options: OPTIONS,
            current,
            selected,
            title: t!(i18n, "settings-lang-title"),
            command: "/language",
            help: t!(i18n, "settings-select-hint"),
            click_action: |s| ClickAction::ApplyLanguage(s.to_string()),
        },
        regions,
        mouse_pos,
        i18n,
    );
}
