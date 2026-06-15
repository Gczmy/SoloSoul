//! 无账户时的欢迎界面 —— 大品牌名 + 可滚动垂直选项。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::text::{Line, Text};
use ratatui::widgets::Paragraph;

use crate::app::{ClickAction, ClickableRegion};
use crate::theme::Theme;
use crate::widgets::option_list::OptionItem;

pub const WELCOME_ACTIONS: &[OptionItem] = &[
    OptionItem {
        label: "开始创建账户",
        command: None,
        desc: "创建第一个本地账户并导入默认模板",
        action: ClickAction::StartOnboarding,
    },
    OptionItem {
        label: "退出 CLI",
        command: Some("/exit"),
        desc: "离开 SoloSoul 终端",
        action: ClickAction::Command("/exit"),
    },
];

/// 渲染欢迎界面。
pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    regions: &mut Vec<ClickableRegion>,
    mouse_pos: Option<(u16, u16)>,
    sheen_offset: u16,
    selected: usize,
) {
    let theme = Theme::load();

    let inner = area.inner(Margin::new(4, 1));
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // 6 行 Logo + 2 行边框
            Constraint::Length(2), // 副标语
            Constraint::Min(0),    // 可滚动选项
            Constraint::Length(1), // 底部提示
        ])
        .split(inner);

    crate::screens::logo::render(frame, chunks[0], &theme, sheen_offset, " 欢迎 ");
    render_taglines(frame, chunks[1], &theme);
    crate::widgets::option_list::render(
        frame,
        chunks[2],
        WELCOME_ACTIONS,
        selected,
        &theme,
        regions,
        mouse_pos,
    );
    render_hint(frame, chunks[3], &theme);
}

fn render_taglines(frame: &mut ratatui::Frame, area: Rect, theme: &Theme) {
    let text = Text::from(vec![
        Line::from("独奏生命数据，重塑数字原点")
            .style(theme.style_cream())
            .alignment(Alignment::Center),
        Line::from("本地优先 · 零知识 · 你的数据你做主")
            .style(theme.style_muted())
            .alignment(Alignment::Center),
    ]);
    frame.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .style(theme.style_text()),
        area,
    );
}

fn render_hint(frame: &mut ratatui::Frame, area: Rect, theme: &Theme) {
    let text = Text::from(vec![Line::from("↑/↓ 选择，Enter 确认，鼠标可直接点击选项")
        .style(theme.style_hint())
        .alignment(Alignment::Center)]);
    frame.render_widget(Paragraph::new(text), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    #[test]
    fn test_render_welcome_smoke() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut regions = Vec::new();
        terminal
            .draw(|frame| render(frame, frame.area(), &mut regions, None, 0, 0))
            .unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf.content.iter().map(|cell| cell.symbol()).collect();
        // 静态 Unicode Logo 包含方块与阴影框线字符
        assert!(content.contains('█'));
        assert!(content.contains('╗'));
        // CJK 在 TestBackend 中会被宽字符分隔，断言单个字符
        assert!(content.contains('开'));
        assert_eq!(regions.len(), WELCOME_ACTIONS.len());
    }
}
