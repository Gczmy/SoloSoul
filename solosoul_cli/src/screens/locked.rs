//! 已锁定状态主界面 —— 大品牌名 + 可点击动作卡片。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{ClickAction, ClickableRegion};
use crate::theme::Theme;

struct LockedAction {
    label: &'static str,
    command: &'static str,
}

const ACTIONS: &[LockedAction] = &[
    LockedAction {
        label: "登录",
        command: "/unlock",
    },
    LockedAction {
        label: "账户",
        command: "/account_list",
    },
    LockedAction {
        label: "诊断",
        command: "/doctor",
    },
    LockedAction {
        label: "退出",
        command: "/exit",
    },
];

/// 渲染已锁定主界面。
pub fn render(frame: &mut ratatui::Frame, area: Rect, regions: &mut Vec<ClickableRegion>) {
    let theme = Theme::load();

    let inner = area.inner(Margin::new(4, 4));
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(10),
            Constraint::Length(1),
        ])
        .split(inner);

    render_brand(frame, chunks[0], &theme);
    render_actions(frame, chunks[1], &theme, regions);
    render_hint(frame, chunks[2], &theme);
}

fn render_brand(frame: &mut ratatui::Frame, area: Rect, theme: &Theme) {
    let text = Text::from(vec![
        Line::from("SoloSoul")
            .style(theme.style_brand())
            .alignment(Alignment::Center),
        Line::from("当前已锁定")
            .style(theme.style_cream())
            .alignment(Alignment::Center),
        Line::from("本地优先 · 零知识 · 你的数据你做主")
            .style(theme.style_muted())
            .alignment(Alignment::Center),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.style_border(true))
        .title(" 已锁定 ")
        .title_style(theme.style_brand_dim());

    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center)
        .style(theme.style_text());
    frame.render_widget(paragraph, area);
}

fn render_actions(
    frame: &mut ratatui::Frame,
    area: Rect,
    theme: &Theme,
    regions: &mut Vec<ClickableRegion>,
) {
    // 2 行 × 2 列
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .spacing(1)
        .split(area);

    for (row_idx, row_area) in rows.iter().enumerate() {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .spacing(2)
            .split(*row_area);

        for (col_idx, col_area) in cols.iter().enumerate() {
            let idx = row_idx * 2 + col_idx;
            if idx >= ACTIONS.len() {
                break;
            }
            render_action_card(frame, *col_area, theme, &ACTIONS[idx], regions);
        }
    }
}

fn render_action_card(
    frame: &mut ratatui::Frame,
    area: Rect,
    theme: &Theme,
    action: &LockedAction,
    regions: &mut Vec<ClickableRegion>,
) {
    let text = Text::from(vec![
        Line::from(format!("> {}", action.label))
            .style(theme.style_cream())
            .alignment(Alignment::Center),
        Line::from(action.command)
            .style(theme.style_muted())
            .alignment(Alignment::Center),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.style_border(false));

    frame.render_widget(
        Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Center)
            .style(theme.style_text()),
        area,
    );

    regions.push(ClickableRegion {
        rect: area,
        action: ClickAction::Command(action.command),
    });
}

fn render_hint(frame: &mut ratatui::Frame, area: Rect, theme: &Theme) {
    let text = Text::from(vec![Line::from("鼠标可直接点击卡片，或在下方输入命令")
        .style(theme.style_hint())
        .alignment(Alignment::Center)]);
    frame.render_widget(Paragraph::new(text), area);
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
            .draw(|frame| render(frame, frame.area(), &mut regions))
            .unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf.content.iter().map(|cell| cell.symbol()).collect();
        assert!(content.contains("SoloSoul"));
        assert!(content.contains("/unlock"));
        assert_eq!(regions.len(), ACTIONS.len());
    }
}
