//! 首次启动创建账户向导屏幕。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, OnboardingStep};
use crate::t;

/// 渲染创建账户向导。
pub fn render(frame: &mut ratatui::Frame, area: Rect, app: &App, step: &OnboardingStep) {
    match step {
        OnboardingStep::EnterName => render_enter_name(frame, area, app),
        OnboardingStep::EnterPassword { .. } => render_enter_password(frame, area, app),
        OnboardingStep::ConfirmPassword { .. } => render_confirm_password(frame, area, app),
        OnboardingStep::EnterHint { .. } => render_enter_hint(frame, area, app),
        OnboardingStep::Confirm { name, hint, .. } => {
            render_confirm(frame, area, name, hint.as_deref(), app)
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
        Line::from(t!(app.i18n, "onboarding-create-account")).bold(),
        Line::from(""),
        Line::from(t!(app.i18n, "onboarding-enter-name")).dark_gray(),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(title, layout[0]);

    app.command_input.render(frame, layout[1]);

    let hint = Paragraph::new(Line::from(t!(app.i18n, "hint-enter-esc-quit")).dark_gray())
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
        Line::from(t!(app.i18n, "onboarding-enter-password")).bold(),
        Line::from(""),
        Line::from(t!(app.i18n, "onboarding-password-length")).yellow(),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(title, layout[0]);

    app.password_input.render(frame, layout[1]);

    let hint = Paragraph::new(Line::from(t!(app.i18n, "hint-enter-esc")).dark_gray())
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
        Line::from(t!(app.i18n, "onboarding-confirm-password")).bold(),
        Line::from(""),
        Line::from(t!(app.i18n, "onboarding-confirm-desc")).dark_gray(),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(title, layout[0]);

    app.password_input.render(frame, layout[1]);

    let hint = Paragraph::new(Line::from(t!(app.i18n, "hint-enter-esc")).dark_gray())
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
        Line::from(t!(app.i18n, "onboarding-enter-hint")).bold(),
        Line::from(""),
        Line::from(t!(app.i18n, "onboarding-hint-desc")).dark_gray(),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(title, layout[0]);

    app.command_input.render(frame, layout[1]);

    let hint = Paragraph::new(Line::from(t!(app.i18n, "hint-enter-esc")).dark_gray())
        .alignment(Alignment::Center);
    frame.render_widget(hint, layout[2]);
}

fn render_confirm(
    frame: &mut ratatui::Frame,
    area: Rect,
    name: &str,
    hint: Option<&str>,
    app: &App,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Length(2)])
        .margin(2)
        .split(area);

    let hint_fallback = t!(app.i18n, "onboarding-confirm-hint-none");
    let hint_text = hint.unwrap_or(hint_fallback.as_str());
    let lines = vec![
        Line::from(t!(app.i18n, "onboarding-confirm-title")).bold(),
        Line::from(""),
        Line::from(t!(app.i18n, "onboarding-confirm-name", name = name)),
        Line::from(t!(app.i18n, "onboarding-confirm-pw-masked")),
        Line::from(t!(app.i18n, "onboarding-confirm-hint", hint = hint_text)),
        Line::from(""),
        Line::from(t!(app.i18n, "onboarding-confirm-import-desc")).dark_gray(),
    ];
    let paragraph = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .title(t!(app.i18n, "onboarding-confirm-title"))
                .borders(Borders::ALL)
                .border_style(Style::default().cyan()),
        )
        .alignment(Alignment::Center);
    frame.render_widget(paragraph, layout[0]);

    let hint = Paragraph::new(Line::from(t!(app.i18n, "hint-enter-esc")).dark_gray())
        .alignment(Alignment::Center);
    frame.render_widget(hint, layout[1]);
}
