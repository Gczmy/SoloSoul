//! 登录向导屏幕。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use solosoul_core::AccountSummary;

use crate::app::UnlockStep;
use crate::widgets::password_input::PasswordInput;

/// 渲染登录向导。
pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    step: &UnlockStep,
    password_input: &PasswordInput,
) {
    match step {
        UnlockStep::SelectAccount { accounts, selected } => {
            render_select_account(frame, area, accounts, *selected)
        }
        UnlockStep::EnterPassword { account_id } => {
            render_enter_password(frame, area, account_id, password_input)
        }
    }
}

fn render_select_account(
    frame: &mut ratatui::Frame,
    area: Rect,
    accounts: &[AccountSummary],
    selected: usize,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .margin(2)
        .split(area);

    let title = Paragraph::new(Text::from(vec![
        Line::from("选择账户登录").bold(),
        Line::from("使用 ↑/↓ 选择，Enter 确认，Esc 取消").dark_gray(),
    ]));
    frame.render_widget(title, layout[0]);

    let rows: Vec<Row> = accounts
        .iter()
        .enumerate()
        .map(|(idx, account)| {
            let marker = if idx == selected { "▶ " } else { "  " };
            let cells = vec![
                Cell::from(format!("{}{}", marker, account.name)),
                Cell::from(account.id.clone()).dark_gray(),
            ];
            let mut row = Row::new(cells);
            if idx == selected {
                row = row.style(Style::default().reversed());
            }
            row
        })
        .collect();

    let table = Table::new(
        rows,
        [Constraint::Percentage(50), Constraint::Percentage(50)],
    )
    .block(Block::default().title(" 账户 ").borders(Borders::ALL));
    frame.render_widget(table, layout[1]);

    let hint =
        Paragraph::new(Line::from("提示: 若未看到目标账户，请先用 GUI 客户端创建。").dark_gray())
            .alignment(Alignment::Center);
    frame.render_widget(hint, layout[2]);
}

fn render_enter_password(
    frame: &mut ratatui::Frame,
    area: Rect,
    account_id: &str,
    password_input: &PasswordInput,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .margin(2)
        .split(area);

    let title = Paragraph::new(Text::from(vec![
        Line::from("输入主密码").bold(),
        Line::from(format!("账户: {}", account_id)).dark_gray(),
        Line::from(""),
        Line::from("主密码不会被保存，无法找回。").yellow(),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(title, layout[0]);

    password_input.render(frame, layout[1]);

    let hint = Paragraph::new(Line::from("Enter 确认 · Esc 取消").dark_gray())
        .alignment(Alignment::Center);
    frame.render_widget(hint, layout[2]);
}
