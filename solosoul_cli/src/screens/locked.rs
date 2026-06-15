//! 已锁定状态主界面 —— 响应式 Logo + 可滚动垂直动作按钮。

use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};

use crate::app::{ClickAction, ClickableRegion};
use crate::theme::Theme;
use crate::widgets::option_list::OptionItem;

pub const ACTIONS: &[OptionItem] = &[
    OptionItem {
        label: "登录",
        command: Some("/unlock"),
        desc: "解锁保险库",
        action: ClickAction::Command("/unlock"),
    },
    OptionItem {
        label: "账户",
        command: Some("/account_list"),
        desc: "列出本地账户",
        action: ClickAction::Command("/account_list"),
    },
    OptionItem {
        label: "诊断",
        command: Some("/doctor"),
        desc: "检查数据目录健康",
        action: ClickAction::Command("/doctor"),
    },
    OptionItem {
        label: "退出",
        command: Some("/exit"),
        desc: "离开 SoloSoul 终端",
        action: ClickAction::Command("/exit"),
    },
];

pub const ACTION_COUNT: usize = ACTIONS.len();

/// 渲染已锁定主界面。
pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    regions: &mut Vec<ClickableRegion>,
    mouse_pos: Option<(u16, u16)>,
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

    crate::widgets::option_list::render(
        frame,
        options_area,
        ACTIONS,
        selected,
        &theme,
        regions,
        mouse_pos,
    );
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
            .draw(|frame| render(frame, frame.area(), &mut regions, None, 0, 0))
            .unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf.content.iter().map(|cell| cell.symbol()).collect();
        assert!(content.contains('█'));
        assert!(content.contains('╗'));
        assert!(content.contains("/unlock"));
        assert_eq!(regions.len(), ACTIONS.len());
    }
}
