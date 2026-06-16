//! 登录向导屏幕 —— 品牌蓝账户选择与密码输入。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use solosoul_core::AccountSummary;

use crate::app::UnlockStep;
use crate::theme::Theme;

/// 渲染登录向导。
pub fn render(frame: &mut ratatui::Frame, area: Rect, step: &UnlockStep, sheen_offset: u16) {
    let theme = Theme::load();
    match step {
        UnlockStep::SelectAccount { accounts, selected } => {
            render_select_account(frame, area, &theme, accounts, *selected)
        }
        UnlockStep::EnterPassword {
            account_id,
            account_name,
            password_hint,
            biometric_configured,
            biometry_type,
        } => render_enter_password(
            frame,
            area,
            &theme,
            account_id,
            account_name,
            password_hint,
            *biometric_configured,
            biometry_type.as_deref(),
            sheen_offset,
        ),
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
            let marker = if idx == selected { "> " } else { "  " };
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

#[allow(clippy::too_many_arguments)]
fn render_enter_password(
    frame: &mut ratatui::Frame,
    area: Rect,
    theme: &Theme,
    account_id: &str,
    account_name: &str,
    password_hint: &Option<String>,
    biometric_configured: bool,
    biometry_type: Option<&str>,
    sheen_offset: u16,
) {
    let inner = area.inner(Margin::new(2, 2));

    // 终端高度足够时显示品牌 Logo banner，否则显示紧凑品牌头。
    let show_banner = inner.height >= 16;
    let chunks = if show_banner {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(inner)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(inner)
    };

    if show_banner {
        crate::screens::logo::render(frame, chunks[0], theme, sheen_offset, " 登录 ");
    } else {
        render_brand_header(frame, chunks[0], theme, "输入主密码");
    }

    let hint_display = password_hint.as_deref().unwrap_or("无");
    let info = Paragraph::new(Text::from(vec![
        Line::from(format!(
            "账户：{}·{}，密码提示词：{}",
            account_name, account_id, hint_display
        ))
        .style(theme.style_muted()),
        Line::from(""),
        Line::from("主密码不会被保存，无法找回。").style(theme.style_warning()),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(info, chunks[1]);

    let biometric_hint = if biometric_configured {
        let kind = biometry_type.unwrap_or("生物识别");
        format!(" · B 使用 {}", kind)
    } else {
        String::new()
    };
    let hint = Paragraph::new(
        Line::from(format!("Enter 确认 · Esc 取消{}", biometric_hint)).style(theme.style_hint()),
    )
    .alignment(Alignment::Center);
    frame.render_widget(hint, chunks[2]);
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
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.style_border(true))
        .title(" SoloSoul ")
        .title_style(theme.style_brand_dim());
    frame.render_widget(
        Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Center)
            .style(theme.style_text()),
        area,
    );
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
                    0,
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
