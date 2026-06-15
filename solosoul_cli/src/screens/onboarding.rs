//! 首次启动创建账户向导屏幕。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, OnboardingStep};

/// 渲染创建账户向导。
pub fn render(frame: &mut ratatui::Frame, area: Rect, app: &App, step: &OnboardingStep) {
    match step {
        OnboardingStep::EnterName => render_enter_name(frame, area, app),
        OnboardingStep::EnterPassword { .. } => render_enter_password(frame, area, app),
        OnboardingStep::ConfirmPassword { .. } => render_confirm_password(frame, area, app),
        OnboardingStep::EnterHint { .. } => render_enter_hint(frame, area, app),
        OnboardingStep::Confirm { name, hint, .. } => {
            render_confirm(frame, area, name, hint.as_deref())
        }
    }
}

fn render_enter_name(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .margin(2)
        .split(area);

    let title = Paragraph::new(Text::from(vec![
        Line::from("创建账户").bold(),
        Line::from(""),
        Line::from("请输入账户名（用于本地标识，可自定义）").dark_gray(),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(title, layout[0]);

    app.command_input.render(frame, layout[1]);

    let hint = Paragraph::new(Line::from("Enter 下一步 · Esc 退出").dark_gray())
        .alignment(Alignment::Center);
    frame.render_widget(hint, layout[2]);
}

fn render_enter_password(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .margin(2)
        .split(area);

    let title = Paragraph::new(Text::from(vec![
        Line::from("设置主密码").bold(),
        Line::from(""),
        Line::from("主密码用于派生加密密钥，务必牢记且至少 8 位。").yellow(),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(title, layout[0]);

    app.password_input.render(frame, layout[1]);

    let hint = Paragraph::new(Line::from("Enter 下一步 · Esc 返回").dark_gray())
        .alignment(Alignment::Center);
    frame.render_widget(hint, layout[2]);
}

fn render_confirm_password(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .margin(2)
        .split(area);

    let title = Paragraph::new(Text::from(vec![
        Line::from("确认主密码").bold(),
        Line::from(""),
        Line::from("请再次输入主密码以确认无误。").dark_gray(),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(title, layout[0]);

    app.password_input.render(frame, layout[1]);

    let hint = Paragraph::new(Line::from("Enter 下一步 · Esc 返回").dark_gray())
        .alignment(Alignment::Center);
    frame.render_widget(hint, layout[2]);
}

fn render_enter_hint(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .margin(2)
        .split(area);

    let title = Paragraph::new(Text::from(vec![
        Line::from("密码提示词（可选）").bold(),
        Line::from(""),
        Line::from("当忘记主密码时，提示词可帮助你回忆。可直接留空。").dark_gray(),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(title, layout[0]);

    app.command_input.render(frame, layout[1]);

    let hint = Paragraph::new(Line::from("Enter 下一步 · Esc 返回").dark_gray())
        .alignment(Alignment::Center);
    frame.render_widget(hint, layout[2]);
}

fn render_confirm(frame: &mut ratatui::Frame, area: Rect, name: &str, hint: Option<&str>) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Length(2)])
        .margin(2)
        .split(area);

    let lines = vec![
        Line::from("确认创建账户").bold(),
        Line::from(""),
        Line::from(format!("账户名: {}", name)),
        Line::from("主密码: ******"),
        Line::from(format!("密码提示: {}", hint.unwrap_or("（无）"))),
        Line::from(""),
        Line::from("创建后将导入默认模板，并直接进入首页。").dark_gray(),
    ];
    let paragraph = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .title(" 创建账户确认 ")
                .borders(Borders::ALL)
                .border_style(Style::default().cyan()),
        )
        .alignment(Alignment::Center);
    frame.render_widget(paragraph, layout[0]);

    let hint = Paragraph::new(Line::from("Enter 创建账户 · Esc 返回修改").dark_gray())
        .alignment(Alignment::Center);
    frame.render_widget(hint, layout[1]);
}
