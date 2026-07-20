//! LLM configuration screen — displays providers and their status.

use crate::i18n::I18n;
use crate::t;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;
use solosoul_core::llm::config::LlmConfig;

pub fn render(frame: &mut Frame, area: Rect, config: &LlmConfig, selected: usize, i18n: &I18n) {
    let layout = Layout::default()
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // Header
    let active_name = config
        .active_provider()
        .map(|p| p.name.as_str())
        .unwrap_or("未设置");
    let header = Paragraph::new(Text::from(vec![
        Line::from(vec![Span::styled(
            t!(i18n, "llm-config-title"),
            Style::default().bold().fg(Color::Cyan),
        )]),
        Line::from(vec![
            Span::raw(t!(i18n, "llm-active-label")),
            Span::styled(active_name, Style::default().fg(Color::Yellow).bold()),
        ]),
        Line::from(t!(i18n, "hint-up-down-esc-q")),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, layout[0]);

    // Provider list
    let items: Vec<ListItem> = config
        .providers
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let is_active = config.active_provider_id.as_deref() == Some(&p.id);
            let marker = if is_active { "★" } else { " " };
            let status = if p.is_enabled { "✓" } else { "✗" };
            let style = if i == selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };
            let status_color = if p.is_enabled {
                Color::Green
            } else {
                Color::Red
            };
            let name_style = if is_active {
                Style::default().fg(Color::Yellow).bold()
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{} [{}] ", marker, status), style.fg(status_color)),
                Span::styled(p.name.clone(), style.patch(name_style)),
                Span::styled(format!("  ─  {}", p.model), style.fg(Color::DarkGray)),
            ]))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(selected));
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(t!(i18n, "llm-providers")),
        )
        .highlight_style(Style::default().bg(Color::DarkGray));
    frame.render_stateful_widget(list, layout[1], &mut list_state);
}
