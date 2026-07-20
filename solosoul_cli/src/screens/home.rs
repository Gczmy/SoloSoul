//! 已登录首页 —— 品牌蓝仪表盘 + 可导航垂直快捷入口。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::text::{Line, Text};
use ratatui::widgets::Paragraph;

use crate::app::{ClickAction, ClickableRegion};
use crate::i18n::I18n;
use crate::t;
use crate::theme::Theme;
use crate::widgets::option_list::OptionItem;

/// 首页快捷入口定义。
pub const SHORTCUTS: &[OptionItem] = &[
    OptionItem {
        label: "浏览",
        command: Some("/list"),
        desc: "列出页面与对象",
        action: ClickAction::Command("/list"),
    },
    OptionItem {
        label: "搜索",
        command: Some("/search"),
        desc: "全局关键词搜索",
        action: ClickAction::Command("/search"),
    },
    OptionItem {
        label: "创建",
        command: Some("/newobject"),
        desc: "新建对象",
        action: ClickAction::Command("/newobject"),
    },
    OptionItem {
        label: "回收站",
        command: Some("/trash"),
        desc: "查看已删除项目",
        action: ClickAction::Command("/trash"),
    },
    OptionItem {
        label: "设置",
        command: Some("/setting"),
        desc: "账户偏好设置",
        action: ClickAction::OpenSettingsMenu,
    },
    OptionItem {
        label: "帮助",
        command: Some("/help"),
        desc: "查看全部命令",
        action: ClickAction::Command("/help"),
    },
    OptionItem {
        label: "插件",
        command: Some("/plugin"),
        desc: "浏览插件市场",
        action: ClickAction::Command("/plugin"),
    },
];

/// 返回快捷入口总数。
pub fn shortcut_count() -> usize {
    SHORTCUTS.len()
}

/// 渲染已登录首页。
///
/// `selected_shortcut` 为当前获得焦点的选项索引（0..shortcut_count()）。
#[allow(clippy::too_many_arguments)]
pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    account_name: &str,
    account_id: &str,
    regions: &mut Vec<ClickableRegion>,
    selected_shortcut: usize,
    mouse_pos: Option<(u16, u16)>,
    sheen_offset: u16,
    i18n: &I18n,
) {
    let theme = Theme::load();

    // 整体内容区：留出边距。
    let inner = area.inner(Margin::new(2, 1));

    // 终端高度足够时显示品牌 Logo banner，否则显示紧凑文本标题。
    let show_banner = inner.height >= 26;
    let chunks = if show_banner {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),
                Constraint::Min(0),
                Constraint::Length(2),
            ])
            .split(inner)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Min(0),
                Constraint::Length(2),
            ])
            .split(inner)
    };

    if show_banner {
        crate::screens::logo::render(
            frame,
            chunks[0],
            &theme,
            sheen_offset,
            &t!(i18n, "locked-title"),
        );
    } else {
        let header_text = if account_name.is_empty() || account_name == account_id {
            t!(i18n, "welcome-back", name = account_id)
        } else {
            t!(
                i18n,
                "welcome-back-full",
                name = account_name,
                id = account_id
            )
        };
        render_header(frame, chunks[0], &theme, &header_text, i18n);
    }

    crate::widgets::option_list::render(
        frame,
        chunks[1],
        SHORTCUTS,
        selected_shortcut,
        &theme,
        regions,
        mouse_pos,
    );
    render_hint(frame, chunks[2], &theme, i18n);
}

fn render_header(frame: &mut ratatui::Frame, area: Rect, theme: &Theme, title: &str, i18n: &I18n) {
    let text = Text::from(vec![
        Line::from(""),
        Line::from(title).style(theme.style_brand()),
        Line::from(t!(i18n, "app-tagline")).style(theme.style_cream()),
        Line::from(""),
    ]);
    let paragraph = Paragraph::new(text).alignment(Alignment::Center);
    frame.render_widget(paragraph, area);
}

fn render_hint(frame: &mut ratatui::Frame, area: Rect, theme: &Theme, i18n: &I18n) {
    let text = Text::from(vec![Line::from(t!(i18n, "home-hint"))
        .style(theme.style_hint())
        .alignment(Alignment::Center)]);
    frame.render_widget(Paragraph::new(text), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::I18n;
    use ratatui::backend::TestBackend;

    #[test]
    fn test_shortcut_count_matches_const() {
        assert_eq!(shortcut_count(), SHORTCUTS.len());
        assert_eq!(shortcut_count(), 7);
    }

    #[test]
    fn test_render_home_smoke() {
        // 使用足够高度以完整显示全部 6 个选项
        let backend = TestBackend::new(80, 36);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut regions = Vec::new();
        let i18n = I18n::new("zh-CN");
        terminal
            .draw(|frame| {
                render(
                    frame,
                    frame.area(),
                    "Alice",
                    "alice-123",
                    &mut regions,
                    0,
                    None,
                    0,
                    &i18n,
                )
            })
            .unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf.content.iter().map(|cell| cell.symbol()).collect();
        assert!(content.contains('█') || content.contains("SoloSoul"));
        assert_eq!(regions.len(), SHORTCUTS.len());
        assert!(content.contains('浏'));
    }
}
