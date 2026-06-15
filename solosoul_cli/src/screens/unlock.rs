//! 登录向导屏幕 —— 暖色账户选择与密码输入。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use solosoul_core::AccountSummary;

use crate::app::UnlockStep;
use crate::theme::Theme;
use crate::widgets::password_input::PasswordInput;

/// 渲染登录向导。
pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    step: &UnlockStep,
    password_input: &PasswordInput,
) {
    let theme = Theme::load();
    match step {
        UnlockStep::SelectAccount { accounts, selected } => {
            render_select_account(frame, area, &theme, accounts, *selected)
        }
        UnlockStep::EnterPassword { account_id } => {
            render_enter_password(frame, area, &theme, account_id, password_input)
        }
    }
}

fn render_select_account(
    frame: &mut ratatui::Frame,
    area: Rect,
    theme: &Theme,
    accounts: &[AccountSummary],
    selected: usize,
) {
    let inner = area.inner(Margin::new(2, 2));
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(inner);

    render_brand_header(frame, chunks[0], theme, "选择账户登录");

    let rows: Vec<Row> = accounts
        .iter()
        .enumerate()
        .map(|(idx, account)| {
            let marker = if idx == selected { "▶ " } else { "  " };
            let cells = vec![
                Cell::from(format!("{}{}", marker, account.name)),
                Cell::from(account.id.clone()).style(theme.style_muted()),
            ];
            let mut row = Row::new(cells);
            if idx == selected {
                row = row.style(theme.style_card_focused());
            }
            row
        })
        .collect();

    let header = Row::new(vec!["账户名", "账户 ID"]).style(
        Style::default()
            .fg(theme.cream)
            .add_modifier(Modifier::BOLD),
    );
    let table = Table::new(
        rows,
        [Constraint::Percentage(50), Constraint::Percentage(50)],
    )
    .header(header)
    .block(
        Block::default()
            .title(" 账户 ")
            .title_style(theme.style_brand_dim())
            .borders(Borders::ALL)
            .border_style(theme.style_border(false)),
    );
    frame.render_widget(table, chunks[1]);

    let hint =
        Paragraph::new(Line::from("使用 ↑/↓ 选择，Enter 确认，Esc 取消").style(theme.style_hint()))
            .alignment(Alignment::Center);
    frame.render_widget(hint, chunks[2]);
}

fn render_enter_password(
    frame: &mut ratatui::Frame,
    area: Rect,
    theme: &Theme,
    account_id: &str,
    password_input: &PasswordInput,
) {
    let inner = area.inner(Margin::new(2, 2));
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .split(inner);

    render_brand_header(frame, chunks[0], theme, "输入主密码");

    let title = Paragraph::new(Text::from(vec![
        Line::from(format!("账户: {}", account_id)).style(theme.style_muted()),
        Line::from(""),
        Line::from("主密码不会被保存，无法找回。").style(theme.style_warning()),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(title, chunks[1]);

    password_input.render(frame, chunks[2]);

    let hint = Paragraph::new(Line::from("Enter 确认 · Esc 取消").style(theme.style_hint()))
        .alignment(Alignment::Center);
    frame.render_widget(hint, chunks[3]);
}

fn render_brand_header(frame: &mut ratatui::Frame, area: Rect, theme: &Theme, subtitle: &str) {
    let text = Text::from(vec![
        Line::from(""),
        Line::from("SoloSoul")
            .style(theme.style_brand())
            .alignment(Alignment::Center),
        Line::from(subtitle)
            .style(theme.style_cream())
            .alignment(Alignment::Center),
        Line::from(""),
    ]);
    frame.render_widget(Paragraph::new(text), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    #[test]
    fn test_render_unlock_select_smoke() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let accounts = vec![AccountSummary {
            id: "alice-123".to_string(),
            name: "Alice".to_string(),
            salt: None,
            verify_hash: None,
            password_hint: None,
            created_at: None,
        }];
        terminal
            .draw(|frame| {
                render(
                    frame,
                    frame.area(),
                    &UnlockStep::SelectAccount {
                        accounts,
                        selected: 0,
                    },
                    &PasswordInput::new(),
                )
            })
            .unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf.content.iter().map(|cell| cell.symbol()).collect();
        assert!(content.contains("SoloSoul"));
        assert!(content.contains("alice-123"));
        assert!(content.contains("Alice"));
    }
}
